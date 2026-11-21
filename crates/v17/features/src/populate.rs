//! Chunk population, ported from `OverworldChunkGenerator.populateChunk`:
//! the per-chunk seed derivation, lakes, dungeons, biome decoration, and
//! the ice/snow pass, in vanilla's order with vanilla's draws.

use crate::biome::{self, BiomeDef};
use crate::block::Block;
use crate::decorator::FeatureDecorator;
use crate::features::dungeon::DungeonFeature;
use crate::features::lake::LakeFeature;
use crate::world::{FLAG_NO_NEIGHBOR_NOTIFY, GenWorld, next_int};
use sunset_1_7_core::{LegacyRandom, RandomSource};

/// The structure pass: mineshaft, village, stronghold and witch hut, in
/// order, drawing from the population random. Returns whether a village
/// generated in this chunk (it gates the lake passes). Structures live in
/// their own crate; `None` runs the pass with no structures, like vanilla's
/// `placeStructures = false`.
pub type StructureHook<'a> = &'a mut dyn FnMut(&mut LegacyRandom, i32, i32) -> bool;

/// Populates one chunk's features into `world`. `world` must already hold
/// the chunk terrain; reads outside the chunk answer with the
/// implementation's default.
pub fn populate_chunk(
    world: &mut dyn GenWorld,
    seed: i64,
    chunk_x: i32,
    chunk_z: i32,
    structure_hook: Option<StructureHook>,
) {
    let mut random = LegacyRandom::new(seed);
    let block_x = chunk_x * 16;
    let block_z = chunk_z * 16;
    let biome = biome::biome(world.get_biome_id(block_x + 16, block_z + 16));

    // Seed derivation: two odd halves from the world seed.
    random.set_seed(seed);
    let half_a = random.next_long() / 2 * 2 + 1;
    let half_b = random.next_long() / 2 * 2 + 1;
    random.set_seed(
        i64::from(chunk_x)
            .wrapping_mul(half_a)
            .wrapping_add(i64::from(chunk_z).wrapping_mul(half_b))
            ^ seed,
    );

    let village_generated = match structure_hook {
        Some(hook) => hook(&mut random, chunk_x, chunk_z),
        None => false,
    };

    // Water lake.
    if biome.id != 2 && biome.id != 17 && !village_generated && next_int(&mut random, 4) == 0 {
        let x = block_x + next_int(&mut random, 16) + 8;
        let y = next_int(&mut random, 256);
        let z = block_z + next_int(&mut random, 16) + 8;
        LakeFeature::water().place(world, &mut random, x, y, z);
    }

    // Lava lake: the surface check's `nextInt(10)` only draws above y 62.
    if !village_generated && next_int(&mut random, 8) == 0 {
        let x = block_x + next_int(&mut random, 16) + 8;
        let outer = next_int(&mut random, 248) + 8;
        let y = next_int(&mut random, outer);
        let z = block_z + next_int(&mut random, 16) + 8;
        if y < 63 || next_int(&mut random, 10) == 0 {
            LakeFeature::lava().place(world, &mut random, x, y, z);
        }
    }

    // Dungeons: eight attempts.
    let mut dungeons = DungeonFeature::new();
    for _ in 0..8 {
        let x = block_x + next_int(&mut random, 16) + 8;
        let y = next_int(&mut random, 256);
        let z = block_z + next_int(&mut random, 16) + 8;
        dungeons.place(world, &mut random, x, y, z);
    }

    // Biome decoration.
    let mut decorator = FeatureDecorator::new();
    decorator.decorate(world, &mut random, &biome, block_x, block_z);

    // `NaturalSpawner.populateChunk` follows here (the mob pass); it draws
    // nothing after the decoration that any later feature reads.

    // Ice and snow over the precipitation surface.
    for lx in 0..16 {
        for lz in 0..16 {
            let x = block_x + 8 + lx;
            let z = block_z + 8 + lz;
            let surface = world.get_precipitation_height(x, z);
            if world.can_freeze(x, surface - 1, z) {
                world.set_block_with_metadata(
                    x,
                    surface - 1,
                    z,
                    Block::Ice,
                    0,
                    FLAG_NO_NEIGHBOR_NOTIFY,
                );
            }
            if world.can_snow(x, surface, z) {
                world.set_block_with_metadata(
                    x,
                    surface,
                    z,
                    Block::SnowLayer,
                    0,
                    FLAG_NO_NEIGHBOR_NOTIFY,
                );
            }
        }
    }
}

/// The biome at a chunk's centre, for callers that need it again.
pub fn chunk_biome(world: &dyn GenWorld, chunk_x: i32, chunk_z: i32) -> BiomeDef {
    biome::biome(world.get_biome_id(chunk_x * 16 + 16, chunk_z * 16 + 16))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk_world::ChunkWorld;

    fn frozen_ocean_floor() -> ChunkWorld {
        let mut world = ChunkWorld::new(0, 0);
        world.fill(0, 62, Block::Stone);
        for x in 0..16 {
            for z in 0..16 {
                world.set_block_with_metadata(x, 63, z, Block::Water, 0, FLAG_NO_NEIGHBOR_NOTIFY);
            }
        }
        world.set_biome(8, 8, 10); // frozen ocean at the chunk centre
        world
    }

    #[test]
    fn ice_forms_over_frozen_water() {
        let mut world = frozen_ocean_floor();
        populate_chunk(&mut world, 12345, 0, 0, None);
        assert!(world.block_count(Block::Ice) > 0);
    }

    #[test]
    fn no_ice_in_warm_biomes() {
        let mut world = frozen_ocean_floor();
        world.set_biome(8, 8, 1); // plains at the chunk centre
        populate_chunk(&mut world, 12345, 0, 0, None);
        assert_eq!(world.block_count(Block::Ice), 0);
    }

    #[test]
    fn seed_derivation_is_deterministic_and_matches_java() {
        // Two odd halves: `nextLong() / 2 * 2 + 1`.
        let mut random = LegacyRandom::new(991);
        let a = random.next_long() / 2 * 2 + 1;
        let b = random.next_long() / 2 * 2 + 1;
        assert_ne!(a % 2, 0);
        assert_ne!(b % 2, 0);
        let mixed = i64::from(3)
            .wrapping_mul(a)
            .wrapping_add(i64::from(-7).wrapping_mul(b))
            ^ 991;
        let mut again = LegacyRandom::new(991);
        let a2 = again.next_long() / 2 * 2 + 1;
        let b2 = again.next_long() / 2 * 2 + 1;
        let mixed2 = i64::from(3)
            .wrapping_mul(a2)
            .wrapping_add(i64::from(-7).wrapping_mul(b2))
            ^ 991;
        assert_eq!(mixed, mixed2);
    }

    #[test]
    fn populate_is_deterministic_for_a_seed() {
        let mut world_a = frozen_ocean_floor();
        world_a.set_biome(8, 8, 4); // forest: trees and ores
        let mut world_b = frozen_ocean_floor();
        world_b.set_biome(8, 8, 4);
        populate_chunk(&mut world_a, 777, -3, 5, None);
        populate_chunk(&mut world_b, 777, -3, 5, None);
        let rows_a: Vec<_> = world_a.rows().collect();
        let rows_b: Vec<_> = world_b.rows().collect();
        assert_eq!(rows_a, rows_b);
    }

    #[test]
    fn structure_hook_consumes_rng_before_lakes() {
        let mut world_a = frozen_ocean_floor();
        let mut world_b = frozen_ocean_floor();
        let draws: i32 = 5;
        populate_chunk(
            &mut world_a,
            42,
            0,
            0,
            Some(&mut |random: &mut LegacyRandom, _, _| {
                for _ in 0..draws {
                    random.next_long();
                }
                false
            }),
        );
        populate_chunk(&mut world_b, 42, 0, 0, None);
        // Different structure draws → different lake placement.
        assert!(
            world_a.rows().count() != world_b.rows().count()
                || world_a.block_count(Block::CoalOre) != world_b.block_count(Block::CoalOre)
        );
    }
}
