//! Binds the 1.7.10 port's verified world generator (terrain, carvers and the
//! population pass in `crates/v17`) to chunk streaming.
//!
//! The generator is off unless the launch asks for it (`--1-7-10-terrain`):
//! the 26.3 generator in [`crate::vanillagen`] stays the default. The
//! 1.7.10 content export is the content root the game loaded, so the state
//! ids [`sunset_1_7_content::state_id`] names are this registry's rows.
use crate::{
    biome::Biomes,
    chunk::{BIOME_STRATEGY, Chunk, ColumnLayout, Section},
    columns::Surface,
    streaming::{ChunkGenerator, Proto},
    vanillagen::TerrainResources,
};
use std::sync::Arc;
use sunrise_core::paletted::PalettedContainer;
use sunrise_world::registry::{BlockRegistry, BlockStateId};
use sunset_1_7_carver::{BiomeSurface, CaveWorldCarver, RavineWorldCarver};
use sunset_1_7_chunkgen::{GeneratorType, OverworldChunkGenerator, SEA_LEVEL, biome_facts};
use sunset_1_7_features::{block::Block, populate::populate_chunk, world::GenWorld};

/// Block ids in a 1.7.10 chunk fit in a byte, and metadata has four bits.
const MAX_BLOCK_ID: usize = 255;
const METADATA_VALUES: usize = 16;

/// A 1.7.10 world generator bound to the engine's chunk streaming.
pub struct Sunset17Terrain {
    seed: i64,
    /// `(id << 4) | meta` to the registry state id: the content export's row,
    /// or the block's `meta = 0` row for metadata the block does not have.
    states: Vec<u32>,
    /// 1.7.10 biome id to the client biome table's index.
    biome_map: [u16; 256],
    /// The column the engine's level holds; 1.7.10's 0..256 sits inside it.
    layout: ColumnLayout,
}

/// The generator state one streaming worker owns.
pub struct Sunset17Worker {
    generator: OverworldChunkGenerator,
    /// The chunk `lod_surface` last sampled, so a whole far-terrain tile does
    /// not generate one chunk per column.
    lod: Option<Sunset17Proto>,
}

/// A chunk after terrain, carving and (once `decorated`) population.
///
/// Blocks are `(id, metadata)` pairs in 1.7.10's own layout,
/// `x << 12 | z << 8 | y`; `0` is air. Population reads and writes them
/// through [`RegionWorld`].
pub struct Sunset17Proto {
    chunk_x: i32,
    chunk_z: i32,
    blocks: Vec<(u16, u8)>,
    /// 16x16 biome ids, `x | z << 4`.
    biomes: [u8; 256],
    /// `World.getHeight` per column after carving: one above the highest
    /// non-air block. Only far terrain reads it; population rescans the
    /// blocks it writes.
    heights: [u16; 256],
    decorated: bool,
}

impl Sunset17Proto {
    fn new(
        chunk_x: i32,
        chunk_z: i32,
        blocks: Vec<u16>,
        metadata: Vec<u8>,
        biomes: [u8; 256],
    ) -> Self {
        let blocks: Vec<(u16, u8)> = blocks.into_iter().zip(metadata).collect();
        let mut heights = [0u16; 256];
        for z in 0..16usize {
            for x in 0..16usize {
                let column = z * 16 + x;
                heights[column] = (0..256u16)
                    .rev()
                    .find(|&y| blocks[(x << 12) | (z << 8) | usize::from(y)].0 != 0)
                    .map_or(0, |y| y + 1);
            }
        }
        Self {
            chunk_x,
            chunk_z,
            blocks,
            biomes,
            heights,
            decorated: false,
        }
    }

    /// The chunk's blocks in 1.7.10's layout, `(id, metadata)`.
    pub fn blocks(&self) -> &[(u16, u8)] {
        &self.blocks
    }

    fn index(x: i32, y: i32, z: i32) -> usize {
        ((x & 15) as usize) << 12 | ((z & 15) as usize) << 8 | (y as usize & 255)
    }

    fn block(&self, x: i32, y: i32, z: i32) -> (u16, u8) {
        self.blocks[Self::index(x, y, z)]
    }

    fn set_block(&mut self, x: i32, y: i32, z: i32, id: u16, metadata: u8) {
        self.blocks[Self::index(x, y, z)] = (id, metadata);
    }

    /// The column's current highest non-air block + 1, as `World.getHeight`.
    fn height(&self, x: i32, z: i32) -> i32 {
        (0..256)
            .rev()
            .find(|&y| self.block(x, y, z).0 != 0)
            .map_or(0, |y| y + 1)
    }
}

impl Sunset17Terrain {
    /// The generator for the world whose content root `resources` names.
    pub fn new(
        resources: TerrainResources<'_>,
        seed: i64,
        layout: ColumnLayout,
    ) -> crate::Result<Self> {
        let root = resources.dir.parent().unwrap_or(resources.dir);
        if !root.join("facts.json").is_file() {
            return Err(format!(
                "the 1.7.10 content export is missing from {}: run \
                 `cargo run -p sunset-1-7-content --bin export` or open the world with \
                 `--resources {}`",
                root.display(),
                sunset_1_7_content::DEFAULT_DIR
            )
            .into());
        }
        Self::from_registry(
            seed,
            resources.registry,
            biome_ids(resources.biomes),
            layout,
        )
    }

    /// The generator over an already built registry and biome table.
    pub fn from_registry(
        seed: i64,
        registry: &'static BlockRegistry,
        biome_map: [u16; 256],
        layout: ColumnLayout,
    ) -> crate::Result<Self> {
        Ok(Self {
            seed,
            states: state_table(registry).map_err(std::io::Error::other)?,
            biome_map,
            layout,
        })
    }

    /// `World.SEA_LEVEL`: the level rules and mods read it.
    pub fn sea_level(&self) -> i32 {
        SEA_LEVEL
    }

    /// The registry state id the 1.7.10 content export gives `(id, metadata)`.
    pub fn state_id(&self, id: u16, meta: u8) -> u32 {
        self.states
            .get(usize::from(id) * METADATA_VALUES + usize::from(meta & 15))
            .copied()
            .unwrap_or(0)
    }

    /// Terrain and carving for one chunk, in the order `world_dump` runs them.
    fn generate(&self, worker: &mut Sunset17Worker, chunk_x: i32, chunk_z: i32) -> Sunset17Proto {
        let generated = worker.generator.get_chunk(chunk_x, chunk_z);
        let surface = BiomeSurfaceBlocks {
            biomes: generated.biomes,
        };
        let mut blocks = generated.blocks;
        CaveWorldCarver::default().place(&surface, self.seed, chunk_x, chunk_z, &mut blocks);
        RavineWorldCarver::default().place(&surface, self.seed, chunk_x, chunk_z, &mut blocks);
        Sunset17Proto::new(
            chunk_x,
            chunk_z,
            blocks,
            generated.metadata,
            generated.biomes,
        )
    }

    /// A proto chunk's blocks, biomes and light as the engine's column.
    fn column(&self, proto: &Sunset17Proto) -> Chunk {
        let mut out = Chunk::with_layout(proto.chunk_x, proto.chunk_z, self.layout);
        for (index, slot) in out.sections.iter_mut().enumerate() {
            let base_y = self.layout.section_y(index);
            // Every section carries the biome of its columns: 1.7.10 biomes
            // vary by column alone, so all four cells down a 4x4 column match.
            let mut cells = [0u32; 64];
            for (cell, value) in cells.iter_mut().enumerate() {
                let (qx, qz) = (cell as i32 & 3, (cell as i32 >> 2) & 3);
                let biome = proto.biomes[(qz * 4 * 16 + qx * 4) as usize];
                *value = u32::from(self.biome_map[usize::from(biome)]);
            }
            let biomes = PalettedContainer::from_values(BIOME_STRATEGY, &cells);
            if base_y >= 256 || base_y + 16 <= 0 {
                Arc::make_mut(slot).biomes = biomes;
                continue;
            }
            let mut values = [0u32; 4096];
            for y in 0..16 {
                let world_y = base_y + y;
                if !(0..256).contains(&world_y) {
                    continue;
                }
                for z in 0..16 {
                    for x in 0..16 {
                        let (id, meta) = proto.block(x, world_y, z);
                        values[(y << 8 | z << 4 | x) as usize] = self.state_id(id, meta);
                    }
                }
            }
            if values.iter().any(|&value| value != 0) {
                *slot = Arc::new(Section {
                    blocks: Section::from_blocks(&values),
                    biomes,
                    ..Default::default()
                });
            } else {
                Arc::make_mut(slot).biomes = biomes;
            }
        }
        out
    }
}

impl ChunkGenerator for Sunset17Terrain {
    type Worker = Sunset17Worker;

    fn worker(&self) -> Sunset17Worker {
        Sunset17Worker {
            generator: OverworldChunkGenerator::new(self.seed, GeneratorType::Default),
            lod: None,
        }
    }

    fn proto(&self, worker: &mut Sunset17Worker, x: i32, z: i32) -> Proto {
        Box::new(self.generate(worker, x, z))
    }

    fn decorates(&self) -> bool {
        true
    }

    /// `populate_chunk` for the centre proto, over the nine protos as one
    /// world. The population pass writes into its neighbours too; those writes
    /// stay in their protos.
    fn decorate(&self, _worker: &mut Sunset17Worker, region: &mut [Proto; 9]) {
        if !region.iter().all(|proto| proto.is::<Sunset17Proto>()) {
            return;
        }
        let mut chunks: [Sunset17Proto; 9] = std::array::from_fn(|index| {
            let proto = std::mem::replace(&mut region[index], Box::new(()));
            *proto
                .downcast::<Sunset17Proto>()
                .expect("every region slot is a 1.7.10 proto")
        });
        {
            let (chunk_x, chunk_z) = (chunks[4].chunk_x, chunks[4].chunk_z);
            let mut world = RegionWorld {
                chunks: &mut chunks,
            };
            populate_chunk(&mut world, self.seed, chunk_x, chunk_z, None);
        }
        chunks[4].decorated = true;
        for (slot, chunk) in region.iter_mut().zip(chunks) {
            *slot = Box::new(chunk);
        }
    }

    fn finish(&self, proto: Proto) -> Chunk {
        let proto = *proto
            .downcast::<Sunset17Proto>()
            .expect("1.7.10 terrain proto");
        self.column(&proto)
    }

    /// A decorated proto as a column, so a light job can read a neighbour past
    /// the kept area (`light_region` asks for it).
    fn light_view(&self, proto: &Proto) -> Option<Chunk> {
        Some(self.column(proto.downcast_ref::<Sunset17Proto>()?))
    }

    /// The engine sections of a decorated proto that hold a non-air block,
    /// which its neighbours' light layers count.
    fn non_air_sections(&self, proto: &Proto) -> Option<crate::section_mask::SectionMask> {
        let proto = proto.downcast_ref::<Sunset17Proto>()?;
        let below = ((0 - self.layout.min_y()).max(0) >> 4) as usize;
        let mut mask = crate::section_mask::SectionMask::EMPTY;
        for section in 0..16usize {
            let base = section as i32 * 16;
            let filled = (0..16)
                .any(|y| (0..16).any(|z| (0..16).any(|x| proto.block(x, base + y, z).0 != 0)));
            if filled && below + section < self.layout.sections() {
                mask.insert(below + section);
            }
        }
        Some(mask)
    }

    /// The undecorated surface of the column, sampled from the proto the far
    /// terrain worker generated for its chunk.
    fn lod_surface(&self, worker: &mut Sunset17Worker, x: i32, z: i32) -> Option<Surface> {
        let (chunk_x, chunk_z) = (x >> 4, z >> 4);
        if worker
            .lod
            .as_ref()
            .is_none_or(|proto| proto.chunk_x != chunk_x || proto.chunk_z != chunk_z)
        {
            worker.lod = Some(self.generate(worker, chunk_x, chunk_z));
        }
        let proto = worker.lod.as_ref()?;
        let (local_x, local_z) = (x & 15, z & 15);
        let biome = self.biome_map[usize::from(proto.biomes[(local_z * 16 + local_x) as usize])];
        let mut water = None;
        let top = i32::from(proto.heights[(local_z * 16 + local_x) as usize]);
        for y in (0..top).rev() {
            let (id, meta) = proto.block(local_x, y, local_z);
            if id == 0 {
                continue;
            }
            let state = self.state_id(id, meta);
            if Block::from_id(id as u8).is_liquid() {
                water.get_or_insert((y + 1, BlockStateId(state)));
            } else {
                return Some(Surface {
                    height: y + 1,
                    state: BlockStateId(state),
                    water,
                    biome,
                });
            }
        }
        Some(Surface {
            height: 0,
            state: BlockStateId(self.state_id(0, 0)),
            water,
            biome,
        })
    }
}

/// The nine protos of a decoration region as `World`: global coordinates,
/// reads outside answer air, writes outside are dropped.
struct RegionWorld<'a> {
    chunks: &'a mut [Sunset17Proto; 9],
}

impl RegionWorld<'_> {
    fn slot(&self, chunk_x: i32, chunk_z: i32) -> Option<usize> {
        let (centre_x, centre_z) = (self.chunks[4].chunk_x, self.chunks[4].chunk_z);
        let (dx, dz) = (chunk_x - centre_x, chunk_z - centre_z);
        (dx.abs() <= 1 && dz.abs() <= 1).then_some(((dz + 1) * 3 + (dx + 1)) as usize)
    }

    fn chunk(&self, x: i32, z: i32) -> Option<&Sunset17Proto> {
        self.slot(x >> 4, z >> 4).map(|slot| &self.chunks[slot])
    }
}

impl GenWorld for RegionWorld<'_> {
    fn get_block(&self, x: i32, y: i32, z: i32) -> Block {
        if !(0..256).contains(&y) {
            return Block::Air;
        }
        self.chunk(x, z).map_or(Block::Air, |chunk| {
            Block::from_id(chunk.block(x, y, z).0 as u8)
        })
    }

    fn get_block_metadata(&self, x: i32, y: i32, z: i32) -> u8 {
        if !(0..256).contains(&y) {
            return 0;
        }
        self.chunk(x, z).map_or(0, |chunk| chunk.block(x, y, z).1)
    }

    fn set_block_with_metadata(
        &mut self,
        x: i32,
        y: i32,
        z: i32,
        block: Block,
        metadata: u8,
        _flags: u8,
    ) {
        if !(0..256).contains(&y) {
            return;
        }
        let Some(slot) = self.slot(x >> 4, z >> 4) else {
            return;
        };
        self.chunks[slot].set_block(x, y, z, u16::from(block.id()), metadata);
    }

    fn get_biome_id(&self, x: i32, z: i32) -> i32 {
        self.chunk(x, z).map_or(1, |chunk| {
            i32::from(chunk.biomes[((z & 15) * 16 + (x & 15)) as usize])
        })
    }

    fn get_height(&self, x: i32, z: i32) -> i32 {
        self.chunk(x, z).map_or(0, |chunk| chunk.height(x, z))
    }

    fn get_surface_height(&self, x: i32, z: i32) -> i32 {
        (0..256)
            .rev()
            .find(|&y| {
                let block = self.get_block(x, y, z);
                block.blocks_movement() && block != Block::Leaves
            })
            .map_or(0, |y| y + 1)
    }

    fn get_precipitation_height(&self, x: i32, z: i32) -> i32 {
        (0..256)
            .rev()
            .find(|&y| {
                let block = self.get_block(x, y, z);
                block.blocks_movement() || block.is_liquid()
            })
            .map_or(0, |y| y + 1)
    }
}

/// `World.getBiome(x, z).surfaceBlock` for the carvers' dirt restore.
struct BiomeSurfaceBlocks {
    biomes: [u8; 256],
}

impl BiomeSurface for BiomeSurfaceBlocks {
    fn surface_block(&self, x: i32, z: i32) -> u16 {
        let id = i32::from(self.biomes[((z & 15) * 16 + (x & 15)) as usize]);
        biome_facts::surface_blocks(id).0
    }
}

/// The 1.7.10 content export's rows as this registry's state ids.
///
/// Rows are the export's state order, so a registry built from another
/// content root would name other blocks there; the two checks refuse it.
fn state_table(registry: &BlockRegistry) -> Result<Vec<u32>, String> {
    let row = |id: u16, meta: u8| {
        sunset_1_7_content::state_id(id, meta).or_else(|| sunset_1_7_content::state_id(id, 0))
    };
    let name = |id, meta| {
        row(id, meta)
            .and_then(|row| registry.state(BlockStateId(row)))
            .map(|state| state.name.as_str())
    };
    if name(0, 0) != Some("minecraft:air") || name(2, 0) != Some("minecraft:grass") {
        return Err(
            "the block registry is not the 1.7.10 content export, so its state rows do not \
             name 1.7.10 blocks; open the world with `--resources .local/sunrise-client/1.7.10`"
                .into(),
        );
    }
    let mut states = vec![0u32; (MAX_BLOCK_ID + 1) * METADATA_VALUES];
    for id in 0..=MAX_BLOCK_ID as u16 {
        for meta in 0..METADATA_VALUES as u8 {
            states[usize::from(id) * METADATA_VALUES + usize::from(meta)] = row(id, meta)
                .filter(|&row| (row as usize) < registry.len())
                .unwrap_or(0);
        }
    }
    Ok(states)
}

/// The client biome table's index for each 1.7.10 biome id.
fn biome_ids(biomes: &Biomes) -> [u16; 256] {
    std::array::from_fn(|id| biomes.id(biome_name(id as u8)))
}

/// The 26.3 name a 1.7.10 biome id plays as.
///
/// 1.7.10's hills and mutated biomes were folded into their base biomes by
/// later versions; they keep the base's name here. `minecraft:plains` is the
/// fallback for the ids vanilla never generated.
fn biome_name(id: u8) -> &'static str {
    match id {
        0 => "minecraft:ocean",
        1 => "minecraft:plains",
        2 => "minecraft:desert",
        3 => "minecraft:windswept_hills",
        4 => "minecraft:forest",
        5 => "minecraft:taiga",
        6 => "minecraft:swamp",
        7 => "minecraft:river",
        8 => "minecraft:nether_wastes",
        9 => "minecraft:the_end",
        10 => "minecraft:frozen_ocean",
        11 => "minecraft:frozen_river",
        12 => "minecraft:snowy_plains",
        13 => "minecraft:snowy_taiga",
        14 | 15 => "minecraft:mushroom_fields",
        16 => "minecraft:beach",
        17 => "minecraft:desert",
        18 => "minecraft:forest",
        19 => "minecraft:taiga",
        20 => "minecraft:windswept_hills",
        21 | 22 => "minecraft:jungle",
        23 => "minecraft:sparse_jungle",
        24 => "minecraft:deep_ocean",
        25 => "minecraft:stony_shore",
        26 => "minecraft:snowy_beach",
        27 | 28 => "minecraft:birch_forest",
        29 => "minecraft:dark_forest",
        30 | 31 => "minecraft:snowy_taiga",
        32 | 33 => "minecraft:old_growth_spruce_taiga",
        34 => "minecraft:windswept_gravelly_hills",
        35 => "minecraft:savanna",
        36 => "minecraft:savanna_plateau",
        37 | 39 => "minecraft:badlands",
        38 => "minecraft:wooded_badlands",
        127 => "minecraft:the_void",
        129 => "minecraft:sunflower_plains",
        130 => "minecraft:desert",
        131 => "minecraft:windswept_hills",
        132 => "minecraft:flower_forest",
        133 => "minecraft:taiga",
        134 => "minecraft:swamp",
        140 => "minecraft:ice_spikes",
        149 => "minecraft:jungle",
        151 => "minecraft:sparse_jungle",
        155 | 156 => "minecraft:birch_forest",
        157 => "minecraft:dark_forest",
        158 => "minecraft:snowy_taiga",
        160 | 161 => "minecraft:old_growth_spruce_taiga",
        162 => "minecraft:windswept_gravelly_hills",
        163 | 164 => "minecraft:windswept_savanna",
        165 => "minecraft:eroded_badlands",
        166 => "minecraft:wooded_badlands",
        167 => "minecraft:badlands",
        _ => "minecraft:plains",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> (tempfile::TempDir, &'static BlockRegistry) {
        let dir = tempfile::tempdir().unwrap();
        sunset_1_7_content::export(dir.path()).unwrap();
        let registry = BlockRegistry::from_facts_json(
            std::fs::File::open(dir.path().join("facts.json")).unwrap(),
        )
        .unwrap();
        (dir, Box::leak(Box::new(registry)))
    }

    /// `finish` puts each `(id, meta)` pair at its block position as the
    /// export's state, and unused metadata takes the block's `meta = 0` row.
    #[test]
    fn finish_places_export_states_and_falls_back_to_metadata_zero() {
        let (_dir, registry) = registry();
        let terrain = Sunset17Terrain::from_registry(
            1234,
            registry,
            [0; 256],
            ColumnLayout::of_level(0, 256),
        )
        .unwrap();

        let mut blocks = vec![0u16; 65536];
        let mut metadata = vec![0u8; 65536];
        let index = |x: usize, z: usize, y: usize| (x << 12) | (z << 8) | y;
        blocks[index(0, 0, 64)] = 2; // grass
        blocks[index(1, 1, 64)] = 1; // stone
        blocks[index(2, 2, 64)] = 12; // sand
        blocks[index(3, 3, 64)] = 1; // stone
        metadata[index(3, 3, 64)] = 15; // stone has metas 0..=6

        let proto = Sunset17Proto::new(0, 0, blocks, metadata, [0; 256]);
        let chunk = terrain.finish(Box::new(proto));
        let default = |name: &str| registry.default_state(name).unwrap().0;
        assert_eq!(chunk.get(0, 64, 0), default("minecraft:grass"));
        assert_eq!(chunk.get(1, 64, 1), default("minecraft:stone"));
        assert_eq!(chunk.get(2, 64, 2), default("minecraft:sand"));
        assert_eq!(chunk.get(3, 64, 3), default("minecraft:stone"));
        assert_eq!(chunk.get(0, 63, 0), 0);
        assert_eq!(chunk.get(0, 65, 0), 0);
    }
}
