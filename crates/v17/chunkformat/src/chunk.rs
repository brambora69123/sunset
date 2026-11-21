//! `net.minecraft.world.chunk.WorldChunk`'s save form and
//! `AnvilChunkStorage.writeChunkToNbt` / `createChunkFromNbt`: one root
//! compound with a `Level` compound holding the sections, biomes, height map,
//! entities, tile entities and tile ticks.

use crate::nbt::{Nbt, NbtError, tag};
use crate::nibble::NibbleStorage;
use crate::section::{SECTION_BLOCKS, Section};

/// One serialized chunk. Entities and tile entities stay as opaque NBT
/// compounds: their fields belong to the entity and block-entity layers.
#[derive(Clone, Debug, PartialEq)]
pub struct Chunk {
    pub x_pos: i32,
    pub z_pos: i32,
    /// Sections 0..16, absent where the height is empty.
    pub sections: [Option<Section>; 16],
    /// `Biomes`, one byte per column, 0xFF where the biome is unset.
    pub biomes: [u8; 256],
    /// `HeightMap`, one int per column.
    pub height_map: [i32; 256],
    /// `Entities` list compounds, in write order.
    pub entities: Vec<Nbt>,
    /// `TileEntities` list compounds, in write order.
    pub tile_entities: Vec<Nbt>,
    /// `TileTicks` list compounds; written only when non-empty, as vanilla does.
    pub tile_ticks: Vec<Nbt>,
    pub terrain_populated: bool,
    pub light_populated: bool,
    pub inhabited_time: i64,
    /// `LastUpdate`, written but not read back (vanilla leaves it on the world).
    pub last_update: i64,
}

impl Chunk {
    /// `new WorldChunk(world, x, z)`: unset biomes (0xFF), zeroed height map.
    pub fn new(x_pos: i32, z_pos: i32) -> Self {
        Self {
            x_pos,
            z_pos,
            sections: Default::default(),
            biomes: [0xFF; 256],
            height_map: [0; 256],
            entities: Vec::new(),
            tile_entities: Vec::new(),
            tile_ticks: Vec::new(),
            terrain_populated: false,
            light_populated: false,
            inhabited_time: 0,
            last_update: 0,
        }
    }

    /// `writeChunkToNbt`: the `{"Level": {...}}` root of one chunk file.
    ///
    /// `has_sky_light` is the dimension's "has sky" (vanilla's `!noSky`); a
    /// no-sky chunk still writes an all-zero `SkyLight` array per section.
    pub fn to_nbt(&self, has_sky_light: bool) -> Nbt {
        let mut level = Nbt::Compound(Vec::new());
        level.put_byte("V", 1);
        level.put_int("xPos", self.x_pos);
        level.put_int("zPos", self.z_pos);
        level.put_long("LastUpdate", self.last_update);
        level.put_int_array("HeightMap", self.height_map.to_vec());
        level.put_boolean("TerrainPopulated", self.terrain_populated);
        level.put_boolean("LightPopulated", self.light_populated);
        level.put_long("InhabitedTime", self.inhabited_time);

        let mut sections = Vec::new();
        for section in &self.sections {
            let Some(s) = section else { continue };
            let mut compound = Nbt::Compound(Vec::new());
            compound.put_byte("Y", (s.offset_y >> 4) as i8);
            compound.put_byte_array("Blocks", s.blocks.to_vec());
            if let Some(add) = &s.add {
                compound.put_byte_array("Add", add.data.clone());
            }
            compound.put_byte_array("Data", s.metadata.data.clone());
            compound.put_byte_array("BlockLight", s.block_light.data.clone());
            if has_sky_light {
                compound.put_byte_array(
                    "SkyLight",
                    s.sky_light
                        .as_ref()
                        .map_or_else(Vec::new, |s| s.data.clone()),
                );
            } else {
                compound.put_byte_array("SkyLight", vec![0; SECTION_BLOCKS / 2]);
            }
            sections.push(compound);
        }
        level.put(
            "Sections",
            Nbt::List {
                element: tag::COMPOUND,
                items: sections,
            },
        );

        level.put_byte_array("Biomes", self.biomes.to_vec());
        level.put(
            "Entities",
            Nbt::List {
                element: tag::COMPOUND,
                items: self.entities.clone(),
            },
        );
        level.put(
            "TileEntities",
            Nbt::List {
                element: tag::COMPOUND,
                items: self.tile_entities.clone(),
            },
        );
        if !self.tile_ticks.is_empty() {
            level.put(
                "TileTicks",
                Nbt::List {
                    element: tag::COMPOUND,
                    items: self.tile_ticks.clone(),
                },
            );
        }

        Nbt::Compound(vec![("Level".to_string(), level)])
    }

    /// `createChunkFromNbt`: the chunk from one chunk file's root compound.
    ///
    /// Vanilla replaces the height map and biome arrays wholesale and reads
    /// each section's arrays as they come; a short array keeps the chunk's
    /// zeroed/0xFF default here rather than crashing later, as Java would.
    pub fn from_nbt(root: &Nbt) -> Result<Self, ChunkError> {
        if !root.contains_typed("Level", tag::COMPOUND) {
            return Err(ChunkError::MissingLevel);
        }
        let level = root.get_compound("Level");
        if !level.contains_typed("Sections", tag::LIST) {
            return Err(ChunkError::MissingSections);
        }

        let mut chunk = Chunk::new(level.get_int("xPos"), level.get_int("zPos"));
        let height_map = level.get_int_array("HeightMap");
        for (slot, &value) in chunk.height_map.iter_mut().zip(height_map) {
            *slot = value;
        }
        chunk.terrain_populated = level.get_boolean("TerrainPopulated");
        chunk.light_populated = level.get_boolean("LightPopulated");
        chunk.inhabited_time = level.get_long("InhabitedTime");

        for section in level.get_list("Sections", tag::COMPOUND) {
            let y = section.get_byte("Y") as u8;
            if y >= 16 {
                return Err(ChunkError::SectionY(y));
            }
            let mut s = Section::new((y as i32) << 4, true);
            let blocks = section.get_byte_array("Blocks");
            for (slot, &b) in s.blocks.iter_mut().zip(blocks) {
                *slot = b;
            }
            if section.contains_typed("Add", tag::BYTE_ARRAY) {
                s.add = Some(copy_nibble(section.get_byte_array("Add")));
            }
            s.metadata = copy_nibble(section.get_byte_array("Data"));
            s.block_light = copy_nibble(section.get_byte_array("BlockLight"));
            if section.contains_typed("SkyLight", tag::BYTE_ARRAY) {
                s.sky_light = Some(copy_nibble(section.get_byte_array("SkyLight")));
            }
            s.validate_block_counters();
            chunk.sections[y as usize] = Some(s);
        }

        if level.contains_typed("Biomes", tag::BYTE_ARRAY) {
            for (slot, &b) in chunk.biomes.iter_mut().zip(level.get_byte_array("Biomes")) {
                *slot = b;
            }
        }

        chunk.entities = level
            .get_list("Entities", tag::COMPOUND)
            .iter()
            .filter(|e| matches!(e, Nbt::Compound(_)))
            .cloned()
            .collect();
        chunk.tile_entities = level
            .get_list("TileEntities", tag::COMPOUND)
            .iter()
            .filter(|e| matches!(e, Nbt::Compound(_)))
            .cloned()
            .collect();
        chunk.tile_ticks = level
            .get_list("TileTicks", tag::COMPOUND)
            .iter()
            .filter(|e| matches!(e, Nbt::Compound(_)))
            .cloned()
            .collect();
        Ok(chunk)
    }
}

/// Copies stored bytes into a full section nibble array, zero-filling the rest
/// (a short array keeps the all-zero tail instead of Java's later crash).
fn copy_nibble(bytes: &[u8]) -> NibbleStorage {
    let mut data = vec![0; SECTION_BLOCKS / 2];
    for (slot, &b) in data.iter_mut().zip(bytes) {
        *slot = b;
    }
    NibbleStorage::from_data(data, 4)
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChunkError {
    /// No `Level` compound in the file.
    MissingLevel,
    /// No `Sections` list in the level data.
    MissingSections,
    /// A section's `Y` byte outside 0..16.
    SectionY(u8),
    /// The root compound failed to parse.
    Nbt(NbtError),
}

impl From<NbtError> for ChunkError {
    fn from(e: NbtError) -> Self {
        ChunkError::Nbt(e)
    }
}

impl std::fmt::Display for ChunkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChunkError::MissingLevel => write!(f, "chunk file is missing level data"),
            ChunkError::MissingSections => write!(f, "chunk file is missing block data"),
            ChunkError::SectionY(y) => write!(f, "chunk section Y {y} out of range"),
            ChunkError::Nbt(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ChunkError {}

/// `AnvilChunkStorage.loadChunk`: read the bytes, then the chunk from them.
pub fn read_chunk(bytes: &[u8]) -> Result<Chunk, ChunkError> {
    Chunk::from_nbt(&crate::nbt::read(bytes)?)
}

/// `AnvilChunkStorage.saveChunk`'s NBT half: the uncompressed chunk file bytes.
pub fn write_chunk(chunk: &Chunk, has_sky_light: bool) -> Result<Vec<u8>, NbtError> {
    crate::nbt::write(&chunk.to_nbt(has_sky_light))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_chunk() -> Chunk {
        let mut chunk = Chunk::new(-3, 7);
        chunk.last_update = 12345;
        chunk.inhabited_time = 678;
        chunk.terrain_populated = true;
        chunk.light_populated = false;
        // Two sections, one high enough for the Add nibble.
        let mut low = Section::new(0, true);
        low.set_block_id(1, 2, 3, 4);
        low.set_metadata(1, 2, 3, 9);
        low.set_sky_light(1, 2, 3, 15);
        let mut high = Section::new(80, true);
        high.set_block_id(15, 15, 15, 600);
        high.set_block_light(0, 0, 0, 14);
        chunk.sections[0] = Some(low);
        chunk.sections[5] = Some(high);
        for (i, biome) in chunk.biomes.iter_mut().enumerate() {
            *biome = (i % 254) as u8;
        }
        for (i, height) in chunk.height_map.iter_mut().enumerate() {
            *height = (i % 256) as i32;
        }
        chunk.entities.push(Nbt::Compound(vec![
            ("id".to_string(), Nbt::String("Zombie".to_string())),
            (
                "Pos".to_string(),
                Nbt::List {
                    element: tag::DOUBLE,
                    items: vec![Nbt::Double(1.5), Nbt::Double(64.0), Nbt::Double(2.5)],
                },
            ),
        ]));
        chunk.tile_entities.push(Nbt::Compound(vec![
            ("id".to_string(), Nbt::String("Furnace".to_string())),
            ("x".to_string(), Nbt::Int(1)),
            ("y".to_string(), Nbt::Int(2)),
            ("z".to_string(), Nbt::Int(3)),
        ]));
        chunk.tile_ticks.push(Nbt::Compound(vec![
            ("i".to_string(), Nbt::Int(8)),
            ("t".to_string(), Nbt::Int(5)),
        ]));
        chunk
    }

    #[test]
    fn chunk_round_trips_through_nbt() {
        let chunk = sample_chunk();
        let bytes = write_chunk(&chunk, true).unwrap();
        let back = read_chunk(&bytes).unwrap();
        // LastUpdate stays on the world in vanilla: written, not read back.
        let mut expected = chunk.clone();
        expected.last_update = 0;
        assert_eq!(back, expected);
        // The written file carries the level's LastUpdate.
        let root = crate::nbt::read(&bytes).unwrap();
        assert_eq!(root.get_compound("Level").get_long("LastUpdate"), 12345);
        // Re-serializing the read-back chunk is stable: the only difference
        // from the first pass is LastUpdate, which vanilla does not read back.
        assert_eq!(
            write_chunk(&back, true).unwrap(),
            write_chunk(&expected, true).unwrap()
        );
    }

    #[test]
    fn no_sky_chunks_write_zeroed_skylight() {
        let chunk = sample_chunk();
        let bytes = write_chunk(&chunk, false).unwrap();
        let root = crate::nbt::read(&bytes).unwrap();
        let level = root.get_compound("Level");
        for section in level.get_list("Sections", tag::COMPOUND) {
            assert_eq!(section.get_byte_array("SkyLight"), &[0u8; 2048]);
        }
        // A no-sky chunk reads back with no sky light storage.
        let back = read_chunk(&bytes).unwrap();
        assert!(
            back.sections.iter().flatten().all(|s| s.sky_light.is_none()
                || s.sky_light.as_ref().unwrap().data.iter().all(|&b| b == 0))
        );
    }

    #[test]
    fn missing_level_and_sections_rejected() {
        let root = Nbt::Compound(vec![]);
        assert_eq!(
            Chunk::from_nbt(&root).unwrap_err(),
            ChunkError::MissingLevel
        );
        let root = Nbt::Compound(vec![("Level".to_string(), Nbt::Compound(vec![]))]);
        assert_eq!(
            Chunk::from_nbt(&root).unwrap_err(),
            ChunkError::MissingSections
        );
    }

    #[test]
    fn unset_biomes_default_to_ff() {
        let mut chunk = sample_chunk();
        // Drop Biomes from the written level: read back keeps the 0xFF default.
        let mut root = chunk.to_nbt(true);
        if let Nbt::Compound(entries) = &mut root
            && let Some((_, Nbt::Compound(level))) = entries.first_mut()
        {
            level.retain(|(k, _)| k != "Biomes");
        }
        let back = Chunk::from_nbt(&root).unwrap();
        assert!(chunk.biomes.iter().any(|&b| b != 0xFF));
        assert!(back.biomes.iter().all(|&b| b == 0xFF));
        chunk.biomes = [0xFF; 256];
        chunk.last_update = 0; // not read back, as in vanilla
        assert_eq!(back, chunk);
    }

    #[test]
    fn add_nibble_survives_the_file() {
        let chunk = sample_chunk();
        let back = read_chunk(&write_chunk(&chunk, true).unwrap()).unwrap();
        let high = back.sections[5].as_ref().unwrap();
        assert_eq!(high.block_id(15, 15, 15), 600);
        assert_eq!(high.non_air_block_count, 1);
        assert_eq!(back.sections[0].as_ref().unwrap().block_id(1, 2, 3), 4);
        assert_eq!(back.sections[0].as_ref().unwrap().metadata_at(1, 2, 3), 9);
    }
}
