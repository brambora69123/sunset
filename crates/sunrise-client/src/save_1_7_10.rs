//! Minecraft 1.7.10 worlds: the era's chunk, `level.dat` and player forms.
//!
//! A save is 1.7.10's when its `level.dat` carries the era's field set
//! (`RandomSeed`, `generatorName`, `version` 19133 and no `DataVersion`).
//! A world not saved yet is 1.7.10's when the content root is the 1.7.10
//! export (`sunset_1_7_content`), because the block ids a chunk stores are
//! that export's rows: the chunk format and the content root travel together.
//! The 26.3 code paths in `save.rs` stay as they are; this module is the
//! second format they select.
//!
//! What comes from the reference port rather than from re-reading vanilla:
//!
//! - `sunset_1_7_chunkformat`'s [`era_chunk::Chunk`] is 1.7.10's
//!   `AnvilChunkStorage.writeChunkToNbt` / `createChunkFromNbt`: a `Level`
//!   compound with `V`, the positions, `HeightMap`, the population flags,
//!   `InhabitedTime`, `Sections` (one `Y`, `Blocks`, `Add`, `Data`,
//!   `BlockLight` and `SkyLight` each), `Biomes` and the three lists. This
//!   module converts that structure to and from the engine's [`Chunk`], and
//!   converts the era `Nbt` to [`sunrise_core::nbt`]'s `Tag` — whose
//!   compound keeps Java's `HashMap` order, so the bytes written are the
//!   bytes 1.7.10 writes for the same values.
//! - `sunset_1_7_save::level_dat` is 1.7.10's `WorldData`: the field set,
//!   its read coercions and defaults, and the version stamp 19133.
//!   `sunset_1_7_save::player` is `PlayerEntity.writeNbtWithoutId`'s set.
//! - `sunset_1_7_content::state_id` is the content export's `(id, metadata)`
//!   to registry state row; [`era_state`] inverts it for saving.
//!
//! Fields the engine does not model — the exact `Biomes` bytes, `HeightMap`,
//! the population flags, raw entity lists, tile entities and tile ticks —
//! stay in [`ChunkExtras::root`] under their era names, so a chunk read and
//! written back keeps them.

use std::sync::{Arc, OnceLock};

use crate::chunk::{BIOME_STRATEGY, Chunk, ChunkExtras, ColumnLayout, Light, Section};
use sunrise_core::{
    nbt::{Compound, JavaString, NamedTag, Tag},
    paletted::PalettedContainer,
};
use sunset_1_7_chunkformat::{
    chunk as era_chunk, nbt as era_nbt, nibble::NibbleStorage, section::Section as EraSection,
};
use sunset_1_7_save::{
    level_dat::{self, GameMode, WorldData},
    player::PlayerData,
};

/// One 16³ section holds this many blocks.
const SECTION_BLOCKS: usize = 4096;
/// 1.7.10's chunk height, one byte per column.
const CHUNK_HEIGHT: usize = 256;
/// Every 1.7.10 metadata value a block can carry.
const METADATA_VALUES: usize = 16;
/// The highest 1.7.10 block id a chunk byte can name.
const MAX_BLOCK_ID: usize = 255;

// ---------------------------------------------------------------------------
// Which format a save uses
// ---------------------------------------------------------------------------

/// Whether a `level.dat` `Data` compound is 1.7.10's: no `DataVersion`
/// (that tag starts in 1.8) and a seed plus a version, as `WorldData` writes.
pub fn is_1_7_10_level_data(data: &Tag) -> bool {
    data.get("DataVersion").is_none()
        && data.get("RandomSeed").is_some()
        && data.get("version").is_some()
}

/// Whether a chunk root is 1.7.10's `{"Level": {...}}`.
pub use sunrise_world::region::is_1_7_10_chunk;

/// Whether the block registry was built from the 1.7.10 content export, so
/// the row [`sunset_1_7_content::state_id`] names is this registry's state.
///
/// The two probes are [`crate::sunset17gen`]'s: air must be row 0, and block
/// id 2 metadata 0 must be `minecraft:grass`, which no 26.3 registry has.
pub fn registry_is_1_7_10(registry: &sunrise_world::registry::BlockRegistry) -> bool {
    use sunrise_world::registry::BlockStateId;
    let name = |id: u16, meta: u8| {
        era_row(id, meta)
            .and_then(|row| registry.state(BlockStateId(row)))
            .map(|state| state.name.as_str())
    };
    name(0, 0) == Some("minecraft:air") && name(2, 0) == Some("minecraft:grass")
}

/// Whether an item catalog was built from the 1.7.10 content export.
///
/// `minecraft:reeds` is the era item; 26.3 renamed it `minecraft:sugar_cane`,
/// so a table holding it was loaded from the era's `items.json`.
pub fn is_1_7_10_content(items: &crate::items::Items) -> bool {
    items.items.contains_key("minecraft:reeds")
}

/// The dimension a `dimensions/<namespace>/<path>` folder belongs to: the
/// world folder and the dimension id, for a folder [`crate::save::dimension_dir`]
/// made.
pub fn split_dimension_root(root: &std::path::Path) -> Option<(&std::path::Path, String)> {
    let dimensions = root.parent()?.parent()?;
    if dimensions.file_name()? != "dimensions" {
        return None;
    }
    let namespace = root.parent()?.file_name()?.to_str()?;
    let path = root.file_name()?.to_str()?;
    Some((dimensions.parent()?, format!("{namespace}:{path}")))
}

/// The folder a 1.7.10 world keeps a dimension's region files in, when `root`
/// is that world's dimension folder and the world is 1.7.10's.
///
/// 1.7.10 has no `dimensions/` tree: the Overworld is the world folder itself
/// and the Nether and End are `DIM-1` and `DIM1`, with no metadata that could
/// move them (`AnvilChunkStorage` and `AnvilWorldStorage`). A custom dimension
/// has no era folder and keeps the 26.3 one.
pub fn era_dimension_root(
    root: &std::path::Path,
    items: Option<&crate::items::Items>,
) -> Option<std::path::PathBuf> {
    let (world, dimension) = split_dimension_root(root)?;
    // A saved world's own level.dat decides its format; the content root only
    // chooses one for a world not saved yet.
    let era = match crate::save::read_dat(&world.join("level.dat")) {
        Some(root) => root.get("Data").is_some_and(is_1_7_10_level_data),
        None => items.is_some_and(is_1_7_10_content),
    };
    if !era {
        return None;
    }
    match dimension.as_str() {
        "minecraft:overworld" => Some(world.to_owned()),
        "minecraft:the_nether" => Some(world.join("DIM-1")),
        "minecraft:the_end" => Some(world.join("DIM1")),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Block ids and metadata
// ---------------------------------------------------------------------------

/// The content export's row for a 1.7.10 `(id, metadata)`, with vanilla's
/// fallback: metadata the block does not have reads as its metadata 0 state.
fn era_row(id: u16, meta: u8) -> Option<u32> {
    sunset_1_7_content::state_id(id, meta).or_else(|| sunset_1_7_content::state_id(id, 0))
}

/// Registry state row → the 1.7.10 `(id, metadata)` the content export gave
/// it. The export writes rows in ascending `(id, metadata)` order, so this is
/// the inverse of [`sunset_1_7_content::state_id`].
fn state_rows() -> &'static [(u16, u8)] {
    static ROWS: OnceLock<Vec<(u16, u8)>> = OnceLock::new();
    ROWS.get_or_init(|| {
        let mut rows = Vec::new();
        for id in 0..=MAX_BLOCK_ID as u16 {
            for meta in 0..METADATA_VALUES as u8 {
                if let Some(row) = sunset_1_7_content::state_id(id, meta) {
                    let row = row as usize;
                    if row >= rows.len() {
                        rows.resize(row + 1, (0, 0));
                    }
                    rows[row] = (id, meta);
                }
            }
        }
        rows
    })
}

/// A registry state row as the 1.7.10 `(id, metadata)` a chunk stores. A
/// state the export does not have (a 26.3-only block) is air.
fn era_state(state: u32) -> (u16, u8) {
    state_rows().get(state as usize).copied().unwrap_or((0, 0))
}

/// The registry state a chunk cell holds.
fn state_of(id: u16, meta: u8) -> u32 {
    era_row(id, meta).unwrap_or(0)
}

// ---------------------------------------------------------------------------
// The era `Nbt` and `sunrise_core::nbt::Tag`
// ---------------------------------------------------------------------------

/// The era NBT as this workspace's `Tag`. `sunset_1_7_chunkformat` carries
/// its own Nbt because it writes `NbtCompound`'s insertion order; the shared
/// `Tag` keeps the Java `HashMap` order 1.7.10 actually writes.
fn era_tag(tag: &Tag) -> era_nbt::Nbt {
    match tag {
        Tag::End => era_nbt::Nbt::Byte(0),
        Tag::Byte(v) => era_nbt::Nbt::Byte(*v),
        Tag::Short(v) => era_nbt::Nbt::Short(*v),
        Tag::Int(v) => era_nbt::Nbt::Int(*v),
        Tag::Long(v) => era_nbt::Nbt::Long(*v),
        Tag::Float(v) => era_nbt::Nbt::Float(f32::from_bits(*v)),
        Tag::Double(v) => era_nbt::Nbt::Double(f64::from_bits(*v)),
        Tag::ByteArray(v) => era_nbt::Nbt::ByteArray(v.clone()),
        Tag::String(v) => era_nbt::Nbt::String(v.to_string_lossy()),
        Tag::List(items) => era_nbt::Nbt::List {
            // `NbtList.write` writes its element type; a list built from
            // items takes the first one's.
            element: items.first().map_or(0, Tag::id),
            items: items.iter().map(era_tag).collect(),
        },
        Tag::Compound(map) => era_nbt::Nbt::Compound(
            map.iter()
                .map(|(key, value)| (key.to_string_lossy(), era_tag(value)))
                .collect(),
        ),
        Tag::IntArray(v) => era_nbt::Nbt::IntArray(v.clone()),
        // 1.7.10 has no TAG_Long_Array; a chunk never carries one.
        Tag::LongArray(_) => era_nbt::Nbt::IntArray(Vec::new()),
    }
}

/// The reverse of [`era_tag`], into a compound whose entries keep Java's
/// `HashMap` order.
fn core_tag(nbt: &era_nbt::Nbt) -> Tag {
    match nbt {
        era_nbt::Nbt::Byte(v) => Tag::Byte(*v),
        era_nbt::Nbt::Short(v) => Tag::Short(*v),
        era_nbt::Nbt::Int(v) => Tag::Int(*v),
        era_nbt::Nbt::Long(v) => Tag::Long(*v),
        era_nbt::Nbt::Float(v) => Tag::Float(v.to_bits()),
        era_nbt::Nbt::Double(v) => Tag::Double(v.to_bits()),
        era_nbt::Nbt::ByteArray(v) => Tag::ByteArray(v.clone()),
        era_nbt::Nbt::String(v) => Tag::String(JavaString::from(v.as_str())),
        era_nbt::Nbt::List { items, .. } => Tag::List(items.iter().map(core_tag).collect()),
        era_nbt::Nbt::Compound(entries) => Tag::compound(
            entries
                .iter()
                .map(|(key, value)| (JavaString::from(key.as_str()), core_tag(value))),
        ),
        era_nbt::Nbt::IntArray(v) => Tag::IntArray(v.clone()),
    }
}

// ---------------------------------------------------------------------------
// Biomes
// ---------------------------------------------------------------------------

/// The 26.3 name each 1.7.10 biome id plays as, for a table that is not the era's
/// own (`Biomes::era_id`): 1.7.10's hills and mutated biomes were folded into their
/// base biome by later versions, so several ids name one modern biome. The era file
/// keeps the exact id it read, and a chunk written back stores that id wherever the
/// registry's biome still matches it.
pub fn biome_map(biomes: &crate::biome::Biomes) -> [u16; 256] {
    std::array::from_fn(|id| biomes.era_id(id as u8))
}

/// The engine biome id of every 4×4 cell in an era column's `Biomes`.
fn biome_cells(map: &[u16; 256], biomes: &[u8; 256], out: &mut [u32; 64]) {
    for (cell, value) in out.iter_mut().enumerate() {
        // Cell `(y << 2 | z) << 2 | x` at the column's bottom row: 1.7.10
        // biomes vary by column, so all four cells down it match.
        let (qx, qz) = (cell & 3, (cell >> 2) & 3);
        let column = qz * 4 * 16 + qx * 4;
        *value = u32::from(map[usize::from(biomes[column])]);
    }
}

// ---------------------------------------------------------------------------
// Chunks
// ---------------------------------------------------------------------------

/// The engine section index of the era section at `index` (0..16), if the
/// column holds a section at that Y.
fn engine_section(chunk: &Chunk, index: usize) -> Option<usize> {
    chunk.section_index(index as i32 * 16)
}

/// The engine section's 4×4×4 biome cell holding a chunk column, at the
/// section's bottom row: `(z >> 2) << 2 | x >> 2`, the inverse of the
/// `qz * 4 + qx` the era `Biomes` array's north-west column sits at.
fn column_cell(column: usize) -> usize {
    ((column >> 6) & 3) << 2 | ((column >> 2) & 3)
}

/// `AnvilChunkStorage.createChunkFromNbt`: the era chunk as the engine's column.
///
/// `map` is [`biome_map`]; `sky_light` is the dimension's. Fields the engine
/// does not model stay in [`ChunkExtras::root`] under their era names, so a
/// chunk read and written back keeps them byte for byte.
pub fn decode(
    root: &NamedTag,
    layout: ColumnLayout,
    sky_light: bool,
    map: &[u16; 256],
) -> Option<Chunk> {
    if !is_1_7_10_chunk(&root.tag) {
        return None;
    }
    let era = era_chunk::Chunk::from_nbt(&era_tag(&root.tag)).ok()?;
    let mut chunk = Chunk::with_layout(era.x_pos, era.z_pos, layout);
    let mut cells = [0u32; 64];
    biome_cells(map, &era.biomes, &mut cells);
    let biomes = PalettedContainer::from_values(BIOME_STRATEGY, &cells);
    // 1.7.10 biomes vary by column, so every section of the column carries
    // them, including the engine's sections outside 1.7.10's 0..256.
    for section in 0..chunk.sections.len() {
        Arc::make_mut(&mut chunk.sections[section]).biomes = biomes.clone();
    }
    for (index, section) in era.sections.iter().enumerate() {
        let Some(target) = engine_section(&chunk, index) else {
            continue;
        };
        let Some(section) = section else {
            continue;
        };
        let target = Arc::make_mut(&mut chunk.sections[target]);
        let mut values = [0u32; SECTION_BLOCKS];
        for (i, value) in values.iter_mut().enumerate() {
            let (x, y, z) = section_pos(i);
            *value = state_of(section.block_id(x, y, z), section.metadata_at(x, y, z));
        }
        target.blocks = Section::from_blocks(&values);
    }
    // The stored light layers, in the engine light loader's indexing: one
    // pad below the column, the sections, then one pad above.
    let first = (chunk.min_y() >> 4) - 1;
    let mut layers: Vec<(Option<&[u8]>, Option<&[u8]>)> =
        vec![(None, None); chunk.sections.len() + 2];
    for (index, section) in era.sections.iter().enumerate() {
        let Some(section) = section else {
            continue;
        };
        let Some(slot) = (index as i64 - i64::from(first))
            .try_into()
            .ok()
            .and_then(|slot: usize| layers.get_mut(slot))
        else {
            continue;
        };
        *slot = (
            section
                .sky_light
                .as_ref()
                .filter(|_| sky_light)
                .map(|sky| sky.data.as_slice()),
            Some(section.block_light.data.as_slice()),
        );
    }
    crate::light::light_from_layers(&mut chunk, &layers, sky_light);
    let mut extras = ChunkExtras {
        // `createChunkFromNbt` leaves `LastUpdate` on the world, so the
        // chunk-format reader does not return it; it is kept here so a chunk
        // written back carries the time it was saved at.
        last_update: match root
            .tag
            .get("Level")
            .and_then(|level| level.get("LastUpdate"))
        {
            Some(Tag::Long(value)) => *value,
            _ => 0,
        },
        ..ChunkExtras::default()
    };
    let mut keep = |key: &str, value: Tag| {
        extras.root.insert(JavaString::from(key), value);
    };
    keep("HeightMap", Tag::IntArray(era.height_map.to_vec()));
    keep(
        "TerrainPopulated",
        Tag::Byte(i8::from(era.terrain_populated)),
    );
    keep("LightPopulated", Tag::Byte(i8::from(era.light_populated)));
    keep("InhabitedTime", Tag::Long(era.inhabited_time));
    keep("Biomes", Tag::ByteArray(era.biomes.to_vec()));
    // The section list as stored: vanilla keeps a section an edit emptied,
    // so the list is not always `0..=top` of the blocks that are left.
    keep(
        "EraSections",
        Tag::ByteArray(
            era.sections
                .iter()
                .enumerate()
                .filter_map(|(index, section)| section.as_ref().map(|_| index as u8))
                .collect(),
        ),
    );
    keep(
        "Entities",
        Tag::List(era.entities.iter().map(core_tag).collect()),
    );
    keep(
        "TileEntities",
        Tag::List(era.tile_entities.iter().map(core_tag).collect()),
    );
    keep(
        "TileTicks",
        Tag::List(era.tile_ticks.iter().map(core_tag).collect()),
    );
    chunk.saved = Some(Arc::new(extras));
    Some(chunk)
}

/// Section-local position of `index` in 1.7.10's order, `y << 8 | z << 4 | x`
/// (the era `NibbleStorage` and the engine's `chunk::index` agree on it).
fn section_pos(index: usize) -> (u8, u8, u8) {
    (
        (index & 15) as u8,
        ((index >> 8) & 15) as u8,
        ((index >> 4) & 15) as u8,
    )
}

/// The era extra `key` a decoded chunk kept, if any.
fn saved_extra<'a>(extras: Option<&'a ChunkExtras>, key: &str) -> Option<&'a Tag> {
    extras?.root.get(&JavaString::from(key))
}

fn bool_extra(extras: Option<&ChunkExtras>, key: &str) -> Option<bool> {
    saved_extra(extras, key).map(|tag| crate::save::number(Some(tag)).unwrap_or(0) != 0)
}

/// The era `Entities`/`TileEntities`/`TileTicks` list, as loaded.
fn raw_list(extras: Option<&ChunkExtras>, key: &str) -> Vec<era_nbt::Nbt> {
    match saved_extra(extras, key) {
        Some(Tag::List(items)) => items.iter().map(era_tag).collect(),
        _ => Vec::new(),
    }
}

/// `AnvilChunkStorage.writeChunkToNbt`: the era store form of `chunk`.
///
/// The sections written are 1.7.10's: every era Y from 0 to the highest one
/// holding a block, with no gaps — what vanilla's section array holds after a
/// chunk has been generated.
pub fn encode(chunk: &Chunk, sky_light: bool, map: &[u16; 256], biome_len: usize) -> NamedTag {
    encode_terrain(chunk, sky_light, map, biome_len, None)
}

/// [`encode`] for a partially generated chunk: 1.7.10's `TerrainPopulated`
/// false makes the next load decorate it.
pub fn encode_proto(
    chunk: &Chunk,
    sky_light: bool,
    map: &[u16; 256],
    biome_len: usize,
) -> NamedTag {
    encode_terrain(chunk, sky_light, map, biome_len, Some(false))
}

fn encode_terrain(
    chunk: &Chunk,
    sky_light: bool,
    map: &[u16; 256],
    biome_len: usize,
    terrain_populated: Option<bool>,
) -> NamedTag {
    let extras = chunk.saved.as_deref();
    let mut era = era_chunk::Chunk::new(chunk.x, chunk.z);
    era.last_update = extras.map_or(0, |e| e.last_update);
    era.terrain_populated = terrain_populated
        .or_else(|| bool_extra(extras, "TerrainPopulated"))
        .unwrap_or(true);
    era.light_populated = bool_extra(extras, "LightPopulated").unwrap_or(chunk.lit);
    era.inhabited_time = saved_extra(extras, "InhabitedTime")
        .and_then(|tag| match tag {
            Tag::Long(value) => Some(*value),
            _ => None,
        })
        .unwrap_or(0);
    era.height_map = match saved_extra(extras, "HeightMap") {
        Some(Tag::IntArray(map)) if map.len() == CHUNK_HEIGHT => {
            let mut out = [0; CHUNK_HEIGHT];
            for (slot, &value) in out.iter_mut().zip(map) {
                *slot = value;
            }
            out
        }
        _ => height_map(chunk),
    };
    era.biomes = era_biomes(chunk, map, biome_len, extras);

    // Vanilla's section array is dense from 0 to the highest section with a
    // block, plus any section an earlier save still had (one its last block
    // was removed from): a decoded chunk keeps that list in its extras.
    let air = state_of(0, 0);
    let filled = |index: usize| {
        engine_section(chunk, index).is_some_and(|si| !chunk.sections[si].is_uniform(air))
    };
    let mut stored = [false; 16];
    match saved_extra(extras, "EraSections") {
        Some(Tag::ByteArray(indices)) => {
            for &index in indices {
                if let Some(slot) = stored.get_mut(usize::from(index)) {
                    *slot = true;
                }
            }
        }
        _ => {
            let top = (0..16).rev().find(|&index| filled(index));
            for slot in stored.iter_mut().take(top.map_or(0, |top| top + 1)) {
                *slot = true;
            }
        }
    }
    for (index, store) in stored.iter().enumerate() {
        if !store {
            continue;
        }
        let Some(si) = engine_section(chunk, index) else {
            continue;
        };
        let mut values = [0u32; SECTION_BLOCKS];
        chunk.sections[si].unpack(&mut values);
        era.sections[index] = Some(era_section(
            &chunk.sections[si].light,
            &values,
            index,
            sky_light,
        ));
    }

    era.entities = raw_list(extras, "Entities");
    era.tile_entities = raw_list(extras, "TileEntities");
    era.tile_ticks = raw_list(extras, "TileTicks");
    NamedTag {
        name: JavaString::default(),
        tag: core_tag(&era.to_nbt(sky_light)),
    }
}

/// One era section from an engine section's blocks and light.
fn era_section(
    light: &Light,
    values: &[u32; SECTION_BLOCKS],
    index: usize,
    sky_light: bool,
) -> EraSection {
    let mut era = EraSection::new(index as i32 * 16, sky_light);
    let mut blocks = [0u8; SECTION_BLOCKS];
    let mut metadata = vec![0u8; SECTION_BLOCKS / 2];
    for (i, &value) in values.iter().enumerate() {
        let (id, meta) = era_state(value);
        blocks[i] = id as u8;
        let slot = &mut metadata[i >> 1];
        *slot = if i & 1 == 0 {
            *slot & 0xf0 | (meta & 15)
        } else {
            *slot & 0x0f | (meta & 15) << 4
        };
    }
    era.blocks = blocks;
    era.metadata = NibbleStorage::from_data(metadata, 4);
    era.block_light = NibbleStorage::from_data(crate::save::nibbles(light, false), 4);
    if sky_light {
        era.sky_light = Some(NibbleStorage::from_data(
            crate::save::nibbles(light, true),
            4,
        ));
    }
    era.validate_block_counters();
    era
}

/// `WorldChunk.populateHeightMap`: one above the highest block with light
/// opacity per column, and 0 for a column with none.
///
/// 1.7.10 asks `Block.getOpacity()`, a per-block constant; the reference
/// port's `BlockDef.opacity` is that field.
fn height_map(chunk: &Chunk) -> [i32; CHUNK_HEIGHT] {
    let opaque = |state: u32| {
        let (id, _) = era_state(state);
        sunset_1_7_blocks::by_id(id).is_some_and(|block| block.opacity != 0)
    };
    let mut out = [0; CHUNK_HEIGHT];
    for (column, slot) in out.iter_mut().enumerate() {
        let (x, z) = ((column & 15) as i32, (column >> 4) as i32);
        for y in (1..CHUNK_HEIGHT as i32).rev() {
            if opaque(chunk.get(x, y - 1, z)) {
                *slot = y;
                break;
            }
        }
    }
    out
}

/// The 16×16 biome bytes a chunk stores: the era id it was loaded with where
/// the column's biome still reads as that id, and the modern biome's era id
/// otherwise.
fn era_biomes(
    chunk: &Chunk,
    map: &[u16; 256],
    biome_len: usize,
    extras: Option<&ChunkExtras>,
) -> [u8; 256] {
    // Modern biome id -> an era id naming it; the first era id wins for the
    // ids later versions folded together.
    let mut inverse = vec![0u8; biome_len];
    let mut seen = vec![false; biome_len];
    for id in 0..=u8::MAX {
        let engine = usize::from(map[usize::from(id)]);
        if let Some(slot) = seen.get_mut(engine)
            && !*slot
        {
            *slot = true;
            inverse[engine] = id;
        }
    }
    let saved = match saved_extra(extras, "Biomes") {
        Some(Tag::ByteArray(bytes)) if bytes.len() == CHUNK_HEIGHT => Some(bytes),
        _ => None,
    };
    // Every section of a decoded chunk carries the column's biome; the
    // engine section holding 1.7.10's y=0 is the column's own.
    let column_section = chunk
        .section_index(0)
        .and_then(|index| chunk.sections.get(index));
    let mut out = [0u8; CHUNK_HEIGHT];
    for (column, slot) in out.iter_mut().enumerate() {
        let engine = column_section.map_or(0, |section| {
            section.biomes.get(column_cell(column)) as usize
        });
        // The engine keeps one biome per 4×4×4 cell, so only the group's
        // north-west column read its saved id back. The group's saved ids are
        // kept while that representative still names the engine's biome.
        let representative = (column >> 4 & !3) * 16 + (column & 15 & !3);
        let kept = saved
            .filter(|bytes| usize::from(map[usize::from(bytes[representative])]) == engine)
            .map(|bytes| bytes[column]);
        *slot = kept.unwrap_or_else(|| inverse.get(engine).copied().unwrap_or(1));
    }
    out
}

// ---------------------------------------------------------------------------
// level.dat
// ---------------------------------------------------------------------------

/// The era game rule each 26.3 rule carries the value of, by their names.
///
/// 1.7.10's nine booleans were split and renamed by later versions; the era
/// file keeps the era names and values, and the engine's own rule map follows
/// them so gameplay sees the saved setting.
const GAME_RULES: [(&str, &str); 9] = [
    ("doFireTick", "fire_spread_radius_around_player"),
    ("mobGriefing", "mob_griefing"),
    ("keepInventory", "keep_inventory"),
    ("doMobSpawning", "spawn_mobs"),
    ("doMobLoot", "mob_drops"),
    ("doTileDrops", "block_drops"),
    ("commandBlockOutput", "command_block_output"),
    ("naturalRegeneration", "natural_health_regeneration"),
    ("doDaylightCycle", "advance_time"),
];

/// The engine dimension a 1.7.10 `Dimension` int names.
fn era_dimension(id: i32) -> &'static str {
    match id {
        -1 => "minecraft:the_nether",
        1 => "minecraft:the_end",
        _ => "minecraft:overworld",
    }
}

/// The 1.7.10 `Dimension` int for an engine dimension id.
fn dimension_era(id: &str) -> i32 {
    match id {
        "minecraft:the_nether" => -1,
        "minecraft:the_end" => 1,
        _ => 0,
    }
}

/// The era generator row a saved `generatorName` names.
fn generator_row(name: &str) -> usize {
    level_dat::GENERATOR_TYPES
        .iter()
        .position(|generator| generator.key.eq_ignore_ascii_case(name))
        .unwrap_or(0)
}

/// The 26.3 world preset an era generator name plays as. `flat`'s layers are
/// 1.7.10's own string and are not translated; the engine generates the
/// preset's 26.3 layers instead.
fn preset_of_generator(name: &str) -> crate::save::WorldPreset {
    use crate::save::WorldPreset;
    match generator_row(name) {
        1 => WorldPreset::Flat,
        2 => WorldPreset::LargeBiomes,
        3 => WorldPreset::Amplified,
        _ => WorldPreset::Normal,
    }
}

/// `new WorldData(nbt)`: the era `level.dat` `Data` compound read into the
/// engine's [`crate::save::Level`].
pub fn read_level(level: &mut crate::save::Level, data: &Tag) {
    let n = |key: &str| crate::save::number(data.get(key));
    level.name = crate::save::text(data.get("LevelName")).unwrap_or_else(|| "World".into());
    level.seed = n("RandomSeed").unwrap_or(0);
    let mode = n("GameType").unwrap_or(0) as i32;
    level.game_mode = mode;
    level.creative = mode == 1;
    level.time = n("Time").unwrap_or(0);
    level.day_time = n("DayTime").unwrap_or(level.time);
    level.world_spawn = (data.get("SpawnX").is_some()
        || data.get("SpawnY").is_some()
        || data.get("SpawnZ").is_some())
    .then(|| {
        [
            n("SpawnX").unwrap_or(0) as i32,
            n("SpawnY").unwrap_or(64) as i32,
            n("SpawnZ").unwrap_or(0) as i32,
        ]
    });
    level.spawn = level
        .world_spawn
        .map(|s| [f64::from(s[0]), f64::from(s[1]), f64::from(s[2])]);
    level.hardcore = n("hardcore").unwrap_or(0) != 0;
    level.allow_commands = Some(n("allowCommands").unwrap_or(0) != 0);
    let raining = n("raining").unwrap_or(0) != 0;
    let thundering = n("thundering").unwrap_or(0) != 0;
    level.weather = crate::world::Weather {
        clear_time: 0,
        rain_time: n("rainTime").unwrap_or(0) as i32,
        thunder_time: n("thunderTime").unwrap_or(0) as i32,
        raining,
        thundering,
        rain_level: if raining { (1.0, 1.0) } else { (0.0, 0.0) },
        thunder_level: if thundering { (1.0, 1.0) } else { (0.0, 0.0) },
    };
    // `GameRules.readNbt`: every rule a string. The nine era rules map to the
    // engine's own names so gameplay follows the file.
    if let Some(Tag::Compound(rules)) = data.get("GameRules") {
        for (name, value) in rules {
            let era_name = name.to_string_lossy();
            let Some(engine) = GAME_RULES
                .iter()
                .find(|(era, _)| *era == era_name)
                .map(|(_, engine)| *engine)
            else {
                continue;
            };
            let enabled = match value {
                Tag::String(value) => value.to_string_lossy() == "true",
                value => crate::save::number(Some(value)).unwrap_or(0) != 0,
            };
            let value = if engine == "fire_spread_radius_around_player" {
                if enabled { 128 } else { -1 }
            } else {
                i32::from(enabled)
            };
            level.game_rules.insert(engine.to_owned(), value);
        }
    }
}

/// The era `Data` compound for `level`, keeping the era-only fields the file
/// already had (its generator row and options, and `MapFeatures`).
///
/// `player` is the era player compound to embed (1.7.10's single-player
/// `level.dat` holds the player); `None` writes no `Player`.
pub fn level_data(
    level: &crate::save::Level,
    on_disk: Option<&Tag>,
    player: Option<Compound>,
    now_millis: i64,
) -> Compound {
    let n = |key: &str| on_disk.and_then(|data| crate::save::number(data.get(key)));
    let name = on_disk
        .and_then(|data| crate::save::text(data.get("generatorName")))
        .unwrap_or_else(|| {
            level.generator.as_ref().map_or_else(
                || "default".to_owned(),
                |generator| {
                    match crate::save::WorldGen::from_settings(generator.settings()).preset {
                        crate::save::WorldPreset::Flat => "flat",
                        crate::save::WorldPreset::LargeBiomes => "largeBiomes",
                        crate::save::WorldPreset::Amplified => "amplified",
                        _ => "default",
                    }
                    .to_owned()
                },
            )
        });
    let spawn = level.world_spawn.unwrap_or([0, 64, 0]);
    let mut world = WorldData {
        seed: level.seed,
        generator: generator_row(&name),
        generator_options: on_disk
            .and_then(|data| crate::save::text(data.get("generatorOptions")))
            .unwrap_or_default(),
        spawn_x: spawn[0],
        spawn_y: spawn[1],
        spawn_z: spawn[2],
        time: level.time,
        time_of_day: level.day_time,
        last_played: now_millis,
        // 1.7.10's `WorldData` keeps a size field no code sets; a saved world holds 0.
        size_on_disk: 0,
        player: None,
        dimension: 0,
        name: level.name.clone(),
        version: level_dat::ANVIL_VERSION,
        rain_time: level.weather.rain_time,
        raining: level.weather.raining,
        thunder_time: level.weather.thunder_time,
        thundering: level.weather.thundering,
        game_mode: GameMode::by_id(level.mode()),
        map_features: n("MapFeatures").is_none_or(|value| value != 0),
        hardcore: level.hardcore,
        allow_commands: level.allow_commands.unwrap_or(false),
        initialized: n("initialized").is_none_or(|value| value != 0),
        game_rules: era_game_rules(level, on_disk),
    };
    if let Some(player) = player {
        world.dimension =
            crate::save::number(player.get(&JavaString::from("Dimension"))).unwrap_or(0) as i32;
        world.player = Some(player);
    }
    world.to_nbt(now_millis)
}

/// The era `GameRules` list: the engine's values for the nine era rules, and
/// every other rule the file had, unchanged.
fn era_game_rules(level: &crate::save::Level, on_disk: Option<&Tag>) -> Vec<(String, String)> {
    let mut rules: Vec<(String, String)> = Vec::new();
    if let Some(Tag::Compound(saved)) = on_disk.and_then(|data| data.get("GameRules")) {
        for (name, value) in saved {
            let name = name.to_string_lossy();
            let value = match value {
                Tag::String(value) => value.to_string_lossy(),
                value => crate::save::number(Some(value)).unwrap_or(0).to_string(),
            };
            rules.push((name, value));
        }
    }
    for (era, engine) in GAME_RULES {
        let Some(&value) = level.game_rules.get(engine) else {
            continue;
        };
        let value = if engine == "fire_spread_radius_around_player" {
            (value >= 0).to_string()
        } else {
            (value != 0).to_string()
        };
        match rules.iter_mut().find(|(name, _)| name == era) {
            Some(slot) => slot.1 = value,
            None => rules.push((era.to_owned(), value)),
        }
    }
    rules
}

/// The `world_gen_settings` a 1.7.10 world's `level.dat` implies: its seed,
/// its preset and `MapFeatures` as structure generation.
pub fn world_gen(on_disk: Option<&Tag>) -> crate::save::WorldGen {
    let name = on_disk
        .and_then(|data| crate::save::text(data.get("generatorName")))
        .unwrap_or_else(|| "default".to_owned());
    crate::save::WorldGen {
        preset: preset_of_generator(&name),
        structures: on_disk
            .and_then(|data| crate::save::number(data.get("MapFeatures")))
            .unwrap_or(1)
            != 0,
        bonus_chest: false,
    }
}

// ---------------------------------------------------------------------------
// The player
// ---------------------------------------------------------------------------

/// The era `Player` compound of a 1.7.10 `level.dat` or `playerdata` file
/// read into the engine's [`crate::save::Level`].
pub fn read_player(level: &mut crate::save::Level, player: &Tag) {
    level.read_player(player);
    let n = |key: &str| crate::save::number(player.get(key));
    if let Some(heal) = crate::save::float(player.get("HealF")) {
        level.health = heal as f32;
    }
    if let Some(fall) = crate::save::float(player.get("FallDistance")) {
        level.fall_distance = fall;
    }
    if let Some(Tag::List(values)) = player.get("Motion") {
        for (axis, value) in values.iter().take(3).enumerate() {
            if let Tag::Double(bits) = value {
                level.motion[axis] = f64::from_bits(*bits);
            }
        }
    }
    if let (Some(Tag::Long(most)), Some(Tag::Long(least))) =
        (player.get("UUIDMost"), player.get("UUIDLeast"))
    {
        level.player_uuid = Some(integers_of_uuid([*most, *least]));
    }
    level.dimension = era_dimension(n("Dimension").unwrap_or(0) as i32).to_owned();
    if let (Some(x), Some(y), Some(z)) = (n("SpawnX"), n("SpawnY"), n("SpawnZ")) {
        level.respawn_elsewhere = Some((
            level.dimension.clone(),
            [x as i32, y as i32, z as i32],
            level.rotation[0],
            level.rotation[1],
            n("SpawnForced").unwrap_or(0) != 0,
        ));
    }
    let mut inventory = Vec::new();
    if let Some(Tag::List(stacks)) = player.get("Inventory") {
        for stack in stacks {
            if let Some((slot, item)) = stack_from_era(stack) {
                inventory.push((slot, item.item.to_string(), item.count, item.damage));
            }
        }
    }
    level.inventory = inventory;
    level.ender_items = match player.get("EnderItems") {
        Some(Tag::List(stacks)) => {
            let mut items = vec![None; 27];
            for (index, stack) in stacks.iter().enumerate() {
                let Some(item) = stack_from_era(stack).map(|(_, item)| item) else {
                    continue;
                };
                if let Some(slot) = items.get_mut(index) {
                    *slot = Some(item);
                }
            }
            items
        }
        _ => Vec::new(),
    };
}

/// The era stack compound's engine form: its `Slot` and the stack.
fn stack_from_era(tag: &Tag) -> Option<(u8, crate::items::ItemStack)> {
    let id = crate::save::number(tag.get("id"))? as i32;
    let def = sunset_1_7_items::by_id(id)?;
    let slot = crate::save::number(tag.get("Slot")).map_or(0, |slot| slot as i8 as u8);
    Some((
        slot,
        crate::items::ItemStack {
            item: Arc::from(format!("minecraft:{}", def.name)),
            count: crate::save::number(tag.get("Count")).unwrap_or(0) as u32,
            damage: crate::save::number(tag.get("Damage")).unwrap_or(0) as u32,
            // 1.7.10's item `tag` is not a component set; the stack's raw NBT
            // stays on the player compound and is written back unchanged.
            extra: None,
        },
    ))
}

/// The era `Player` compound for `level`.
///
/// The loaded compound is the base, so fields the engine does not model
/// (attributes, effects and each stack's `tag`) ride along; the fields the
/// engine tracks are written over it.
pub fn player_tag(level: &crate::save::Level) -> Compound {
    let mut player = match level.player_extra.as_ref() {
        Some(Tag::Compound(compound)) => compound.clone(),
        _ => PlayerData::default().to_nbt(),
    };
    for key in [
        "DataVersion",
        "equipment",
        "respawn",
        "recipeBook",
        "LastDeathLocation",
        "previousPlayerGameType",
        "post_effects",
        "active_effects",
        "fall_distance",
        "FallFlying",
        "seenCredits",
        "XpSeed",
        "playerGameType",
    ] {
        player.remove(&JavaString::from(key));
    }
    let mut put = |key: &str, value: Tag| {
        player.insert(JavaString::from(key), value);
    };
    if let Some(position) = level.position {
        put("Pos", list_of_doubles(&position));
    }
    put("Motion", list_of_doubles(&level.motion));
    put(
        "Rotation",
        Tag::List(
            level
                .rotation
                .iter()
                .map(|value| Tag::Float(value.to_bits()))
                .collect(),
        ),
    );
    put(
        "FallDistance",
        Tag::Float((level.fall_distance as f32).to_bits()),
    );
    put("OnGround", Tag::Byte(i8::from(level.on_ground)));
    put("Dimension", Tag::Int(dimension_era(&level.dimension)));
    put("HealF", Tag::Float(level.health.to_bits()));
    put("Health", Tag::Short(level.health.ceil() as i16));
    put("AbsorptionAmount", Tag::Float(level.absorption.to_bits()));
    put("foodLevel", Tag::Int(level.food));
    put("foodTickTimer", Tag::Int(level.food_tick_timer));
    put(
        "foodSaturationLevel",
        Tag::Float(level.saturation.to_bits()),
    );
    put(
        "foodExhaustionLevel",
        Tag::Float(level.food_exhaustion.to_bits()),
    );
    put("XpP", Tag::Float(level.xp_progress.to_bits()));
    put("XpLevel", Tag::Int(level.xp_level));
    put("XpTotal", Tag::Int(level.xp_total));
    put("SelectedItemSlot", Tag::Int(level.selected_slot));
    if let Some([most, least]) = level.player_uuid.map(uuid_longs) {
        put("UUIDMost", Tag::Long(most));
        put("UUIDLeast", Tag::Long(least));
    }
    let abilities = match level.abilities {
        Some(abilities) => sunset_1_7_save::player::Abilities {
            invulnerable: abilities.invulnerable,
            flying: abilities.flying,
            may_fly: abilities.may_fly,
            instabuild: abilities.instabuild,
            may_build: abilities.may_build,
            fly_speed: abilities.flying_speed,
            walk_speed: abilities.walking_speed,
        },
        None => {
            let mode = level.mode();
            let (creative, spectator) = (mode == 1, mode == 3);
            sunset_1_7_save::player::Abilities {
                invulnerable: creative || spectator,
                flying: level.flying && (creative || spectator),
                may_fly: creative || spectator,
                instabuild: creative,
                may_build: mode < 2,
                ..Default::default()
            }
        }
    };
    put("abilities", Tag::Compound(abilities.to_nbt()));
    // The respawn point is a bed or a forced point; 1.7.10 keeps one.
    if let Some((_, [x, y, z], _, _, forced)) = &level.respawn_elsewhere {
        put("SpawnX", Tag::Int(*x));
        put("SpawnY", Tag::Int(*y));
        put("SpawnZ", Tag::Int(*z));
        put("SpawnForced", Tag::Byte(i8::from(*forced)));
    }
    let saved = saved_stacks(level.player_extra.as_ref(), "Inventory");
    let inventory: Vec<Tag> = level
        .inventory
        .iter()
        .filter(|(slot, ..)| *slot <= 35 || (100..=103).contains(slot))
        .filter_map(|(slot, item, count, damage)| {
            let saved = saved.iter().find(|stack| {
                crate::save::number(stack.get("Slot")).map(|s| s as i8 as u8) == Some(*slot)
            });
            stack_to_era(*slot, item, *count, *damage, saved, true)
        })
        .collect();
    put("Inventory", Tag::List(inventory));
    let saved = saved_stacks(level.player_extra.as_ref(), "EnderItems");
    let ender: Vec<Tag> = level
        .ender_items
        .iter()
        .enumerate()
        .filter_map(|(slot, stack)| {
            let stack = stack.as_ref()?;
            let saved = saved.get(slot);
            stack_to_era(
                slot as u8,
                &stack.item,
                stack.count,
                stack.damage,
                saved,
                false,
            )
        })
        .collect();
    put("EnderItems", Tag::List(ender));
    player
}

/// The raw `Inventory`/`EnderItems` list a loaded player compound carried.
fn saved_stacks(player: Option<&Tag>, key: &str) -> Vec<Tag> {
    match player.and_then(|p| p.get(key)) {
        Some(Tag::List(stacks)) => stacks.clone(),
        _ => Vec::new(),
    }
}

/// One era stack compound, from the engine's stack. `saved` is the raw
/// compound the same slot loaded with: when it still names this stack, it is
/// written back so its `tag` (enchantments, names) survives.
fn stack_to_era(
    slot: u8,
    item: &str,
    count: u32,
    damage: u32,
    saved: Option<&Tag>,
    named_slot: bool,
) -> Option<Tag> {
    let name = item.strip_prefix("minecraft:")?;
    let def = sunset_1_7_items::id_of(name)?;
    let matches = saved.is_some_and(|saved| {
        crate::save::number(saved.get("id")) == Some(i64::from(def.id))
            && crate::save::number(saved.get("Count")) == Some(i64::from(count))
            && crate::save::number(saved.get("Damage")) == Some(i64::from(damage))
    });
    if matches {
        return saved.cloned();
    }
    let mut entries = Vec::new();
    if named_slot {
        entries.push(("Slot", Tag::Byte(slot as i8)));
    }
    entries.push(("id", Tag::Short(def.id as i16)));
    entries.push(("Count", Tag::Byte(count as i8)));
    entries.push(("Damage", Tag::Short(damage as i16)));
    Some(Tag::compound(entries))
}

/// `UUIDMost`/`UUIDLeast` for the engine's four-int UUID.
fn uuid_longs(uuid: [i32; 4]) -> [i64; 2] {
    let most = (u64::from(uuid[0] as u32) << 32) | u64::from(uuid[1] as u32);
    let least = (u64::from(uuid[2] as u32) << 32) | u64::from(uuid[3] as u32);
    [most as i64, least as i64]
}

/// The engine's four-int UUID for `UUIDMost`/`UUIDLeast`.
fn integers_of_uuid([most, least]: [i64; 2]) -> [i32; 4] {
    let (most, least) = (most as u64, least as u64);
    [
        (most >> 32) as u32 as i32,
        most as u32 as i32,
        (least >> 32) as u32 as i32,
        least as u32 as i32,
    ]
}

fn list_of_doubles(values: &[f64; 3]) -> Tag {
    Tag::List(values.iter().map(|v| Tag::Double(v.to_bits())).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The identity biome map: era biome id `i` reads as engine biome `i`.
    fn identity_biomes() -> [u16; 256] {
        std::array::from_fn(|id| id as u16)
    }

    /// The era content export's row for a block, for a chunk built without a
    /// registry.
    fn state(id: u16, meta: u8) -> u32 {
        sunset_1_7_content::state_id(id, meta).expect("an exported row")
    }

    fn era_bytes(tag: &NamedTag) -> Vec<u8> {
        sunrise_core::nbt::write_named(tag, sunrise_core::nbt::Limits::saved()).unwrap()
    }

    /// A tag's value for a diff line, long arrays summarised.
    fn short(tag: &Tag) -> String {
        match tag {
            Tag::ByteArray(v) => format!("ByteArray({} bytes)", v.len()),
            Tag::IntArray(v) => format!("IntArray({} values)", v.len()),
            Tag::LongArray(v) => format!("LongArray({} values)", v.len()),
            Tag::List(v) => format!("List({} items)", v.len()),
            Tag::Compound(v) => format!("Compound({} keys)", v.len()),
            tag => format!("{tag:?}").chars().take(80).collect(),
        }
    }

    fn differing<T: PartialEq + std::fmt::Display>(
        a: &[T],
        b: &[T],
        path: &str,
        out: &mut Vec<String>,
    ) {
        if a.len() != b.len() {
            out.push(format!("{path}.len: {} != {}", a.len(), b.len()));
        }
        let mut at = Vec::new();
        for (i, (x, y)) in a.iter().zip(b).enumerate() {
            if x != y {
                at.push(format!("[{i}] {x} != {y}"));
                if at.len() >= 6 {
                    break;
                }
            }
        }
        if !at.is_empty() {
            out.push(format!("{path}: {}", at.join(", ")));
        }
    }

    /// Every difference between two tags, as `path: left != right`, stopping
    /// after `limit` — what a chunk comparison reports field by field.
    fn diff(left: &Tag, right: &Tag, path: &str, out: &mut Vec<String>, limit: usize) {
        if out.len() >= limit {
            return;
        }
        match (left, right) {
            (Tag::Compound(a), Tag::Compound(b)) => {
                for (key, value) in a {
                    let key = key.to_string_lossy();
                    let child = format!("{path}.{key}");
                    match b.get(&JavaString::from(key.as_str())) {
                        Some(other) => diff(value, other, &child, out, limit),
                        None => out.push(format!("{child}: {} != <absent>", short(value))),
                    }
                }
                for (key, value) in b {
                    let key = key.to_string_lossy();
                    if a.get(&JavaString::from(key.as_str())).is_none() {
                        out.push(format!("{path}.{key}: <absent> != {}", short(value)));
                    }
                }
            }
            (Tag::List(a), Tag::List(b)) => {
                if a.len() != b.len() {
                    out.push(format!("{path}.len: {} != {}", a.len(), b.len()));
                }
                for (i, (x, y)) in a.iter().zip(b).enumerate() {
                    diff(x, y, &format!("{path}[{i}]"), out, limit);
                }
            }
            (Tag::ByteArray(a), Tag::ByteArray(b)) => differing(a, b, path, out),
            (Tag::IntArray(a), Tag::IntArray(b)) => differing(a, b, path, out),
            (Tag::LongArray(a), Tag::LongArray(b)) => differing(a, b, path, out),
            (Tag::Float(a), Tag::Float(b)) if f32::from_bits(*a) == f32::from_bits(*b) => {}
            (Tag::Double(a), Tag::Double(b)) if f64::from_bits(*a) == f64::from_bits(*b) => {}
            (a, b) if a == b => {}
            (a, b) => out.push(format!("{path}: {} != {}", short(a), short(b))),
        }
    }

    fn level_tag(level: &crate::save::Level) -> Tag {
        Tag::compound([(
            "Data",
            Tag::Compound(level_data(level, None, None, 1_700_000_000_000)),
        )])
    }

    #[test]
    fn state_rows_invert_the_content_export() {
        for id in [0u16, 1, 2, 3, 5, 9, 12, 35, 95, 159] {
            for meta in 0..METADATA_VALUES as u8 {
                if let Some(row) = sunset_1_7_content::state_id(id, meta) {
                    assert_eq!(era_state(row), (id, meta), "row {row} of {id}:{meta}");
                }
            }
        }
        // A state the export does not have reads as air.
        assert_eq!(era_state(u32::MAX), (0, 0));
    }

    /// 1.7.10's `BiomeGenBase` ids, under the names 26.3 gives the biomes
    /// they became (an independent oracle: the era's own registry order).
    #[test]
    fn the_biome_table_names_1_7_10_ids() {
        for (id, name) in [
            (0, "minecraft:ocean"),
            (1, "minecraft:plains"),
            (2, "minecraft:desert"),
            (3, "minecraft:windswept_hills"),
            (6, "minecraft:swamp"),
            (8, "minecraft:nether_wastes"),
            (9, "minecraft:the_end"),
            (12, "minecraft:snowy_plains"),
            (14, "minecraft:mushroom_fields"),
            (16, "minecraft:beach"),
            (23, "minecraft:sparse_jungle"),
            (24, "minecraft:deep_ocean"),
            (25, "minecraft:stony_shore"),
            (26, "minecraft:snowy_beach"),
            (27, "minecraft:birch_forest"),
            (29, "minecraft:dark_forest"),
            (32, "minecraft:old_growth_spruce_taiga"),
            (35, "minecraft:savanna"),
            (36, "minecraft:savanna_plateau"),
            (37, "minecraft:badlands"),
            (38, "minecraft:wooded_badlands"),
            // 1.7.10's mesa plateau (39) plays as badlands here, as the era
            // generator's own table has it.
            (39, "minecraft:badlands"),
            (127, "minecraft:the_void"),
            (129, "minecraft:sunflower_plains"),
            (132, "minecraft:flower_forest"),
            (140, "minecraft:ice_spikes"),
            (165, "minecraft:eroded_badlands"),
            (167, "minecraft:badlands"),
            // Ids 1.7.10 never generated play as plains.
            (40, "minecraft:plains"),
            (126, "minecraft:plains"),
        ] {
            assert_eq!(
                crate::biome::era_biome_name(id),
                name,
                "1.7.10 biome id {id}"
            );
        }
    }

    /// The engine keeps 4×4×4 biome cells; a chunk's per-column ids are kept
    /// where the group's representative still names the engine's biome, and
    /// a changed group reads back as the modern biome's first era id.
    #[test]
    fn per_column_biomes_survive_where_the_cell_still_matches() {
        // Three engine biomes: 0 = plains (era 1), 1 = desert (era 2, then
        // 17), 2 = every other era id.
        let map: [u16; 256] = std::array::from_fn(|id| match id {
            1 => 0,
            2 | 17 => 1,
            _ => 2,
        });
        let layout = ColumnLayout::new(0, 256).unwrap();
        // Columns 0..8 desert (ids 2, 17, 2, 17, ...), 8..16 plains.
        let biomes: [u8; 256] = std::array::from_fn(|c| {
            if c & 15 < 8 {
                if c & 1 == 0 { 2 } else { 17 }
            } else {
                1
            }
        });
        let mut chunk = Chunk::with_layout(0, 0, layout);
        // The engine biomes the decoder would have made from those columns.
        for section in 0..chunk.sections.len() {
            let section = Arc::make_mut(&mut chunk.sections[section]);
            let mut cells = [0u32; 64];
            for (cell, value) in cells.iter_mut().enumerate() {
                let (qx, qz) = (cell & 3, (cell >> 2) & 3);
                let column = qz * 4 * 16 + qx * 4;
                *value = u32::from(map[usize::from(biomes[column])]);
            }
            section.biomes = PalettedContainer::from_values(BIOME_STRATEGY, &cells);
        }
        chunk.set(0, 0, 0, state(1, 0));
        let mut saved = ChunkExtras::default();
        saved
            .root
            .insert(JavaString::from("Biomes"), Tag::ByteArray(biomes.to_vec()));
        chunk.saved = Some(Arc::new(saved));
        let encoded = encode(&chunk, true, &map, 3);
        let level = encoded.tag.get("Level").unwrap();
        let Some(Tag::ByteArray(written)) = level.get("Biomes") else {
            panic!("no Biomes")
        };
        assert_eq!(written[..], biomes[..], "the era ids are kept");
        // A changed cell reads as the modern biome's first era id in every
        // column of its group.
        let mut edited = decode(&encoded, layout, true, &map).unwrap();
        edit_biome(&mut edited, 0);
        let encoded = encode(&edited, true, &map, 3);
        let Some(Tag::ByteArray(written)) = encoded.tag.get("Level").unwrap().get("Biomes") else {
            panic!("no Biomes")
        };
        // The first group's engine biome is now plains (era 1).
        assert_eq!(written[0], 1, "an edited group reads as the modern biome");
        assert_eq!(written[3], 1);
        assert_eq!(written[8], 1, "the untouched groups are kept");
    }

    /// Set the bottom-row biome of a chunk's 4×4 cell holding column `0`.
    fn edit_biome(chunk: &mut Chunk, engine: u32) {
        let section = Arc::make_mut(&mut chunk.sections[0]);
        let mut cells = [0u32; 64];
        for (i, cell) in cells.iter_mut().enumerate() {
            *cell = section.biomes.get(i);
        }
        cells[column_cell(0)] = engine;
        section.biomes = PalettedContainer::from_values(BIOME_STRATEGY, &cells);
    }

    #[test]
    fn a_chunk_round_trips_through_the_era_bytes() {
        let layout = ColumnLayout::new(0, 256).unwrap();
        let mut chunk = Chunk::with_layout(7, -3, layout);
        for z in 0..16 {
            for x in 0..16 {
                chunk.set(x, 0, z, state(1, 0));
                chunk.set(x, 1, z, state(1, 0));
                chunk.set(x, 2, z, state(35, (x % 16) as u8));
            }
        }
        chunk.set(3, 10, 4, state(50, 0));
        for si in 0..chunk.sections.len() {
            let section = Arc::make_mut(&mut chunk.sections[si]);
            section.light = Light::from_slice(&std::array::from_fn(|i| {
                let y = (si as i32 * 16 + ((i >> 8) & 15) as i32) as u8;
                // Open sky above the surface, darker below it.
                if y > 4 { 0xf0 } else { 0x05 }
            }));
        }
        let first = encode(&chunk, true, &identity_biomes(), 256);
        let back = decode(&first, layout, true, &identity_biomes()).expect("the era chunk");
        assert_eq!((back.x, back.z), (7, -3));
        for y in [0, 1, 2, 10, 11] {
            for (x, z) in [(0, 0), (3, 4), (15, 15)] {
                assert_eq!(back.get(x, y, z), chunk.get(x, y, z), "block {x} {y} {z}");
            }
        }
        assert_eq!(back.light(3, 10, 4), chunk.light(3, 10, 4));
        // The bytes are stable: a decoded chunk writes the same record again.
        let second = encode(&back, true, &identity_biomes(), 256);
        let mut out = Vec::new();
        diff(&first.tag, &second.tag, "chunk", &mut out, 20);
        assert!(out.is_empty(), "re-encode differs:\n{}", out.join("\n"));
        assert_eq!(era_bytes(&first), era_bytes(&second));
    }

    #[test]
    fn a_1_7_10_level_dat_round_trips_through_the_engine() {
        let dir = std::env::temp_dir().join(format!("sunset-era-level-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut level = crate::save::Level {
            name: "Era".into(),
            seed: 4242,
            time: 2,
            day_time: 2,
            world_spawn: Some([200, 64, 256]),
            spawn: Some([200.0, 64.0, 256.0]),
            hardcore: false,
            allow_commands: Some(false),
            weather: crate::world::Weather {
                clear_time: 0,
                rain_time: 106_514,
                thunder_time: 74_337,
                raining: false,
                thundering: false,
                rain_level: (0.0, 0.0),
                thunder_level: (0.0, 0.0),
            },
            ..crate::save::Level::default()
        };
        level.game_rules.insert("keep_inventory".into(), 1);
        let mut player = PlayerData::default().to_nbt();
        player.insert("UUIDMost".into(), Tag::Long(0x0000_0001_0000_0002));
        player.insert("UUIDLeast".into(), Tag::Long(0x0000_0003_0000_0004));
        player.insert(
            "Pos".into(),
            Tag::List(vec![
                Tag::Double(0.0f64.to_bits()),
                Tag::Double(64.5f64.to_bits()),
                Tag::Double(0.0f64.to_bits()),
            ]),
        );
        let data = level_data(&level, None, Some(player), 1_700_000_000_000);
        assert!(is_1_7_10_level_data(&Tag::Compound(data.clone())));
        crate::save::write_dat(
            &dir.join("level.dat"),
            Tag::compound([("Data", Tag::Compound(data.clone()))]),
        )
        .unwrap();

        // Through the engine's own read: the era fields land in `Level`.
        let mut back = crate::save::Level::read(&dir).expect("the era level");
        assert_eq!(back.name, "Era");
        assert_eq!(back.seed, 4242);
        assert_eq!(back.time, 2);
        assert_eq!(back.day_time, 2);
        assert_eq!(back.world_spawn, Some([200, 64, 256]));
        assert_eq!(back.game_rule("keep_inventory"), 1);
        assert_eq!(back.weather.rain_time, 106_514);
        assert!(!back.weather.raining);
        assert_eq!(back.position, Some([0.0, 64.5, 0.0]));

        // And through the engine's own write: still the era's field set, with
        // the values it read.
        assert_eq!(
            back.player_uuid,
            Some([1, 2, 3, 4]),
            "the player's UUIDMost/UUIDLeast are its identity"
        );
        back.xp_level = 7;
        back.write(&dir, &crate::items::Items::default()).unwrap();
        let root = crate::save::read_dat(&dir.join("level.dat")).unwrap();
        let written = match root.get("Data") {
            Some(Tag::Compound(data)) => data.clone(),
            _ => panic!("no Data"),
        };
        assert!(is_1_7_10_level_data(&Tag::Compound(written.clone())));
        for key in [
            "RandomSeed",
            "Time",
            "DayTime",
            "SpawnX",
            "SpawnY",
            "SpawnZ",
            "rainTime",
            "MapFeatures",
            "GameRules",
        ] {
            assert_eq!(
                written.get(&JavaString::from(key)),
                data.get(&JavaString::from(key)),
                "{key}"
            );
        }
        let player = match written.get(&JavaString::from("Player")) {
            Some(Tag::Compound(player)) => player.clone(),
            _ => panic!("the player is in level.dat"),
        };
        assert_eq!(
            player.get(&JavaString::from("XpLevel")),
            Some(&Tag::Int(7)),
            "the player is saved with the level"
        );
        assert!(
            written.get(&JavaString::from("DataVersion")).is_none(),
            "no 26.3 version stamp"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_era_world_keeps_its_regions_where_1_7_10_puts_them() {
        let world = std::env::temp_dir().join(format!("sunset-era-folders-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&world);
        std::fs::create_dir_all(&world).unwrap();
        let level = crate::save::Level {
            name: "Era".into(),
            ..crate::save::Level::default()
        };
        crate::save::write_dat(&world.join("level.dat"), level_tag(&level)).unwrap();
        let dimension = |id: &str| crate::save::dimension_dir(&world, id);
        let save = crate::save::SaveDir::new(
            dimension("minecraft:overworld"),
            crate::chunk::LevelShape::OVERWORLD,
            None,
        );
        assert_eq!(save.root, world, "the Overworld is the world folder");
        let nether = crate::save::SaveDir::new(
            dimension("minecraft:the_nether"),
            crate::chunk::LevelShape::NETHER,
            None,
        );
        assert_eq!(nether.root, world.join("DIM-1"));
        let end = crate::save::SaveDir::new(
            dimension("minecraft:the_end"),
            crate::chunk::LevelShape::END,
            None,
        );
        assert_eq!(end.root, world.join("DIM1"));
        // A custom dimension has no 1.7.10 folder and keeps the 26.3 one.
        let custom = crate::save::SaveDir::new(
            dimension("test:pocket"),
            crate::chunk::LevelShape::OVERWORLD,
            None,
        );
        assert_eq!(custom.root, dimension("test:pocket"));
        std::fs::remove_dir_all(&world).unwrap();
    }

    /// The saved world's region file, when this machine has one: its chunks
    /// decode through the engine and re-encode to the bytes vanilla wrote.
    ///
    /// `SUNSET_1_7_10_WORLD` names another save. The sample covers the chunks
    /// with entities, tile entities and tile ticks and every Nth other one;
    /// `SUNSET_1_7_10_WORLD_ALL=1` sweeps the whole region.
    #[test]
    fn a_vanilla_1_7_10_chunk_round_trips_byte_for_byte() {
        let world = std::env::var_os("SUNSET_1_7_10_WORLD")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from("/tmp/opencode/vanilla4242/world"));
        let path = world.join("region/r.0.0.mca");
        if !path.is_file() {
            return;
        }
        // The world's `level.dat` re-emitted from its own values, with the
        // clock it was last played at, is the file vanilla wrote.
        let level_root = crate::save::read_dat(&world.join("level.dat")).expect("the era level");
        let original = match level_root.get("Data") {
            Some(Tag::Compound(data)) => data.clone(),
            _ => panic!("no Data"),
        };
        let level = crate::save::Level::read(&world).expect("the era level reads");
        let last_played =
            crate::save::number(original.get(&JavaString::from("LastPlayed"))).unwrap_or(0);
        let again = level_data(
            &level,
            Some(&Tag::Compound(original.clone())),
            None,
            last_played,
        );
        let mut out = Vec::new();
        diff(
            &Tag::Compound(original),
            &Tag::Compound(again),
            "level.dat",
            &mut out,
            12,
        );
        assert!(
            out.is_empty(),
            "the era level.dat differs:\n{}",
            out.join("\n")
        );

        let layout = ColumnLayout::VANILLA;
        let mut region = sunrise_world::region::RegionFile::open(&path).unwrap();
        let all = region.chunks().map(|(pos, _)| pos).collect::<Vec<_>>();
        // The chunks this world has entities, tile entities and tile ticks in.
        let interesting = [[0, 4], [2, 4], [8, 4], [12, 18], [17, 5]];
        let sweep = std::env::var_os("SUNSET_1_7_10_WORLD_ALL").is_some();
        let mut checked = 0;
        let mut compared = 0;
        for (i, pos) in all.iter().enumerate() {
            let sampled =
                sweep || i % (all.len() / 24).max(1) == 0 || interesting.contains(&[pos.x, pos.z]);
            if !sampled {
                continue;
            }
            let Some(record) = region.read(*pos).unwrap() else {
                continue;
            };
            let original = record.nbt(sunrise_core::nbt::Limits::saved()).unwrap();
            let decoded = decode(&original, layout, true, &identity_biomes())
                .unwrap_or_else(|| panic!("chunk {pos:?} decodes"));
            let encoded = encode(&decoded, true, &identity_biomes(), 256);
            let mut out = Vec::new();
            diff(&original.tag, &encoded.tag, "chunk", &mut out, 12);
            assert!(
                out.is_empty(),
                "chunk {pos:?} differs after the round trip:\n{}",
                out.join("\n")
            );
            // The payload the region would store: the same NBT, and a zlib
            // stream that reads back to it. (The stream is not the bytes the
            // JVM wrote: deflate leaves the encoder free choices, and this
            // save's came from another zlib than sunrise-deflate's.)
            let bytes = era_bytes(&encoded);
            let compressed = sunrise_world::compression::Compression::Zlib
                .encode(&bytes)
                .unwrap();
            let stored = sunrise_world::compression::Compression::Zlib
                .decode(&compressed, 1 << 24)
                .unwrap();
            assert_eq!(stored, bytes, "chunk {pos:?}: the stored payload");
            compared += era_bytes(&original).len();
            checked += 1;
        }
        assert!(checked > 0, "the region holds no chunks");
        if sweep {
            assert_eq!(checked, all.len(), "every chunk of the region");
        }
        println!(
            "{checked} chunks of {} re-encoded byte for byte ({compared} uncompressed bytes)",
            path.display()
        );
    }

    /// The engine's 1.7.10 generator's chunk saves and reads back cell for
    /// cell. Skips when this machine has no 1.7.10 content export.
    #[test]
    fn a_generated_chunk_saves_and_reads_back() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../.local/sunrise-client/1.7.10");
        let facts = root.join("facts.json");
        if !facts.is_file() {
            return;
        }
        let registry: &'static sunrise_world::registry::BlockRegistry = Box::leak(Box::new(
            sunrise_world::registry::BlockRegistry::from_facts_json(
                std::fs::File::open(&facts).unwrap(),
            )
            .unwrap(),
        ));
        // The engine's Overworld column, which the generator is bound to.
        let layout = ColumnLayout::VANILLA;
        let terrain = crate::sunset17gen::Sunset17Terrain::from_registry(
            4242,
            registry,
            identity_biomes(),
            layout,
        )
        .expect("the 1.7.10 generator");
        use crate::streaming::ChunkGenerator;
        let mut worker = terrain.worker();
        let proto = terrain.proto(&mut worker, 3, -2);
        let chunk = terrain.finish(proto);
        assert!(
            (0..256).any(|y| (0..16).any(|z| (0..16).any(|x| chunk.get(x, y, z) != 0))),
            "the generated chunk has blocks"
        );
        let encoded = encode(&chunk, true, &identity_biomes(), 256);
        let decoded = decode(&encoded, layout, true, &identity_biomes()).expect("the saved chunk");
        let mut out = Vec::new();
        for y in 0..256 {
            for z in 0..16 {
                for x in 0..16 {
                    let (a, b) = (chunk.get(x, y, z), decoded.get(x, y, z));
                    if a != b && out.len() < 12 {
                        out.push(format!("[{x} {y} {z}] {a} != {b}"));
                    }
                }
            }
        }
        assert!(
            out.is_empty(),
            "the generator's chunk changed:\n{}",
            out.join("\n")
        );
        // Biomes survive the save as the era's ids.
        assert_eq!(
            era_biomes(&decoded, &identity_biomes(), 256, decoded.saved.as_deref()),
            era_biomes(&chunk, &identity_biomes(), 256, chunk.saved.as_deref()),
        );

        // Through a region file, as the save path writes it: zlib, epoch
        // timestamp, the era bytes back.
        let dir = std::env::temp_dir().join(format!("sunset-era-region-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("r.0.0.mca");
        let mut region = sunrise_world::region::RegionFile::open_writable(&path, true).unwrap();
        let bytes = era_bytes(&encoded);
        let record = sunrise_world::region::ChunkRecord::from_nbt(
            &encoded,
            sunrise_world::compression::Compression::Lz4,
            1_700_000_000,
            sunrise_core::nbt::Limits::saved(),
        )
        .unwrap();
        assert_eq!(
            record.compression,
            sunrise_world::compression::Compression::Zlib,
            "1.7.10 reads zlib and gzip only"
        );
        region
            .write(sunrise_core::position::ChunkPos::new(3, 2), &record)
            .unwrap();
        region.sync().unwrap();
        let back = region
            .read(sunrise_core::position::ChunkPos::new(3, 2))
            .unwrap()
            .expect("the saved chunk");
        assert_eq!(
            back.compression,
            sunrise_world::compression::Compression::Zlib
        );
        assert_eq!(back.timestamp, 1_700_000_000);
        assert_eq!(
            era_bytes(&back.nbt(sunrise_core::nbt::Limits::saved()).unwrap()),
            bytes
        );
        let read = decode(
            &back.nbt(sunrise_core::nbt::Limits::saved()).unwrap(),
            layout,
            true,
            &identity_biomes(),
        )
        .expect("the region's chunk");
        assert_eq!(read.get(0, 0, 0), chunk.get(0, 0, 0));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
