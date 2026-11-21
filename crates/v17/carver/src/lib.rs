//! Minecraft 1.7.10 carver: caves and ravines, ported from
//! `world/gen/carver/*.java` (`Generator` base, `CaveWorldCarver`,
//! `RavineWorldCarver`). See docs/ for the port contract.

pub mod cave;
pub mod math;
pub mod ravine;
pub mod scan;

pub use cave::CaveWorldCarver;
pub use ravine::RavineWorldCarver;

/// The 1.7.10 block registry IDs the carvers read and write (`Blocks.java`).
/// A chunk array is `[u16; 16 * 16 * 256]`, indexed
/// [`scan::index`](`scan::index`) style, with `0` for the generated air a
/// carve leaves behind (Java's `null` in the decompiled `Block[]`).
pub mod block_id {
    pub const AIR: u16 = 0;
    pub const STONE: u16 = 1;
    pub const GRASS: u16 = 2;
    pub const DIRT: u16 = 3;
    pub const FLOWING_WATER: u16 = 8;
    pub const WATER: u16 = 9;
    pub const FLOWING_LAVA: u16 = 10;
    pub const LAVA: u16 = 11;
}

/// `World.getBiome(x, z).surfaceBlock`: the top block of the biome at world
/// `(x, z)`, as a 1.7.10 block ID (grass for most biomes, sand for deserts).
/// The carvers use it to restore the dirt under a carved-out grass surface.
pub trait BiomeSurface {
    fn surface_block(&self, x: i32, z: i32) -> u16;
}

/// `Generator.place(source, world, chunkX, chunkZ, blocks)`: the
/// chunk-neighborhood recursion both carvers inherit. Seeds the generator
/// from the world seed, draws the two scramble multipliers, then calls
/// `place_chunk` once per neighborhood chunk (`chunk ± range` on both axes)
/// with the generator reseeded to `startX * a ^ startZ * b ^ worldSeed`.
pub fn carve_neighborhood(
    random: &mut sunset_1_7_core::LegacyRandom,
    range: i32,
    surface: &dyn BiomeSurface,
    world_seed: i64,
    chunk_x: i32,
    chunk_z: i32,
    blocks: &mut [u16],
    mut place_chunk: impl FnMut(
        &mut sunset_1_7_core::LegacyRandom,
        &dyn BiomeSurface,
        i32,
        i32,
        i32,
        i32,
        &mut [u16],
    ),
) {
    use sunset_1_7_core::RandomSource;
    random.set_seed(world_seed);
    let a = random.next_long();
    let b = random.next_long();
    for start_x in (chunk_x - range)..=(chunk_x + range) {
        for start_z in (chunk_z - range)..=(chunk_z + range) {
            random.set_seed(
                i64::from(start_x).wrapping_mul(a)
                    ^ i64::from(start_z).wrapping_mul(b)
                    ^ world_seed,
            );
            place_chunk(random, surface, start_x, start_z, chunk_x, chunk_z, blocks);
        }
    }
}
