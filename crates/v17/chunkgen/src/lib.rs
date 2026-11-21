//! Minecraft 1.7.10 chunkgen, ported from
//! `world/gen/chunk/OverworldChunkGenerator.java`: the biome-weight table, the
//! heightmap noise stack, the trilinear terrain fill into a flat 65536 block
//! array indexed `x << 12 | z << 8 | y` (`0` = air), and the surface pass, in
//! the order `getChunk` runs them.

pub mod biome_facts;
mod generator;

/// Vanilla 1.7.10 numeric block ids, `0` = air.
pub const AIR: u16 = 0;
pub const STONE: u16 = 1;
pub const GRASS: u16 = 2;
pub const DIRT: u16 = 3;
pub const BEDROCK: u16 = 7;
pub const WATER: u16 = 9;
pub const SAND: u16 = 12;
pub const GRAVEL: u16 = 13;
pub const SANDSTONE: u16 = 24;
pub const ICE: u16 = 79;
pub const MYCELIUM: u16 = 110;
pub const STAINED_HARDENED_CLAY: u16 = 159;

pub use biome_facts::{base_height, height_variation, surface_blocks, temperature};
pub use generator::{GeneratedChunk, GeneratorType, OverworldChunkGenerator, SEA_LEVEL};
