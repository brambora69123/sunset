//! Probe printing deterministic feature output for later parity checks.

use sunset_1_7_core::{LegacyRandom, RandomSource};
use sunset_1_7_features::Block;
use sunset_1_7_features::chunk_world::ChunkWorld;
use sunset_1_7_features::decorator::FeatureDecorator;
use sunset_1_7_features::populate::populate_chunk;
use sunset_1_7_features::world::{FLAG_NO_NEIGHBOR_NOTIFY, GenWorld};

fn main() {
    // Single large oak: leaf and log counts for a fixed seed.
    let (placed, logs, leaves, height) = place_large_oak(97);
    println!("large_oak placed={placed} logs={logs} leaves={leaves} height={height}");

    // Populated chunks over seeds and biomes.
    for seed in [12345_i64, 42, -987654321] {
        for biome_id in [1_i32, 4, 5, 6, 10, 14] {
            let mut world = ChunkWorld::new(0, 0);
            world.fill(0, 62, Block::Stone);
            for x in 0..16_i32 {
                for z in 0..16_i32 {
                    world.set_block_with_metadata(
                        x,
                        63,
                        z,
                        Block::Water,
                        0,
                        FLAG_NO_NEIGHBOR_NOTIFY,
                    );
                    world.set_biome(x, z, biome_id);
                }
            }
            populate_chunk(&mut world, seed, 0, 0, None);
            let rows = world.rows().count();
            let ores: usize = [
                Block::CoalOre,
                Block::IronOre,
                Block::GoldOre,
                Block::RedstoneOre,
                Block::DiamondOre,
                Block::LapisOre,
                Block::Dirt,
                Block::Gravel,
            ]
            .iter()
            .map(|b| world.block_count(*b))
            .sum();
            let mut random = LegacyRandom::new(seed);
            let half = random.next_long() / 2 * 2 + 1;
            println!("seed={seed} biome={biome_id} rows={rows} ores={ores} half={half:x}");
        }
    }

    // Pure decorator pass on a stone world with a grass surface.
    let mut world = ChunkWorld::new(0, 0);
    world.fill(0, 98, Block::Stone);
    world.fill(99, 100, Block::Grass);
    let mut rng = LegacyRandom::new(7);
    let mut decorator = FeatureDecorator::new();
    decorator.decorate(
        &mut world,
        &mut rng,
        &sunset_1_7_features::biome::biome(4),
        0,
        0,
    );
    println!(
        "decorator forest logs={} leaves={} coal={} iron={}",
        world.block_count(Block::Log),
        world.block_count(Block::Leaves),
        world.block_count(Block::CoalOre),
        world.block_count(Block::IronOre)
    );
}

fn place_large_oak(seed: i64) -> (bool, usize, usize, i32) {
    let mut world = ChunkWorld::new(0, 0);
    world.fill(0, 40, Block::Grass);
    let mut rng = LegacyRandom::new(seed);
    let mut decorator = FeatureDecorator::new();
    let placed = decorator.place_large_oak(&mut world, &mut rng, 8, 41, 8);
    (
        placed,
        world.block_count(Block::Log),
        world.block_count(Block::Leaves),
        decorator.large_oak_height(),
    )
}
