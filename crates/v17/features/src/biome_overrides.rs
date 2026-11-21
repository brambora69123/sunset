//! The per-biome `decorate` overrides: `PlainsBiome`, `ForestBiome`,
//! `SavannaBiome`, `DesertBiome`, `ExtremeHillsBiome`, `IceBiome`,
//! `JungleBiome` and `TaigaBiome`, each around the base decorator body in
//! vanilla's order with vanilla's draws.

use crate::biome::{BiomeDecorator, BiomeDef, DecoratorConfig, foliage};
use crate::block::Block;
use crate::decorator::FeatureDecorator;
use crate::features::desert_well::DesertWellFeature;
use crate::features::double_plant::DoublePlantFeature;
use crate::features::huge_mushroom::HugeMushroomFeature;
use crate::features::ice_patch::IcePatchFeature;
use crate::features::ice_spike::IceSpikeFeature;
use crate::features::melon_patch::MelonPatchFeature;
use crate::features::vein::{BlockVeinFeature, VeinFeature};
use crate::features::vine::VineFeature;
use crate::world::{FLAG_NO_NEIGHBOR_NOTIFY, GenWorld, next_int};
use sunset_1_7_core::LegacyRandom;

/// `super.decorate(world, random, x, z)`: the base body, run with whatever
/// attempt counts the override set.
pub type BaseBody<'a> = &'a mut dyn FnMut(
    &mut FeatureDecorator,
    &mut dyn GenWorld,
    &mut LegacyRandom,
    &DecoratorConfig,
);

/// `Biome.decorate`: run the biome's own override around the base body.
pub fn decorate(
    decorator: &mut FeatureDecorator,
    world: &mut dyn GenWorld,
    random: &mut LegacyRandom,
    biome: &BiomeDef,
    x: i32,
    z: i32,
    base: BaseBody,
) {
    match biome.decorator {
        // `MutatedBiome` and the biomes without an override of their own.
        BiomeDecorator::Base => base(decorator, world, random, &biome.config),
        BiomeDecorator::Plains { mutated } => {
            let mut config = biome.config;
            // The plains read the foliage noise at their own corner, before
            // anything else in the chunk draws.
            if foliage((f64::from(x) + 8.0) / 200.0, (f64::from(z) + 8.0) / 200.0) < -0.8 {
                config.flower_attempts = 15;
                config.grass_attempts = 5;
            } else {
                config.flower_attempts = 4;
                config.grass_attempts = 10;
                plant_double(world, random, x, z, 2, 7);
            }
            if mutated {
                plant_double(world, random, x, z, 0, 10);
            }
            base(decorator, world, random, &config);
        }
        BiomeDecorator::Forest { variant } => {
            if variant == 3 {
                // The roofed forest's own 4x4 grid, huge mushroom branch and
                // all, replaces the base body's tree loop.
                for dx in 0..4 {
                    for dz in 0..4 {
                        let px = x + dx * 4 + 1 + 8 + next_int(random, 3);
                        let pz = z + dz * 4 + 1 + 8 + next_int(random, 3);
                        let py = world.get_height(px, pz);
                        if next_int(random, 20) == 0 {
                            HugeMushroomFeature::new().place(world, random, px, py, pz);
                        } else {
                            decorator.place_picked_tree(world, random, biome, px, py, pz);
                        }
                    }
                }
            }

            // `nextInt(5) - 3` can go negative: no attempts, draw kept.
            let mut attempts = next_int(random, 5) - 3;
            if variant == 1 {
                attempts += 2;
            }
            let mut double_plant = DoublePlantFeature::new();
            for _ in 0..attempts {
                double_plant.set_variant(match next_int(random, 3) {
                    0 => 1,
                    1 => 4,
                    _ => 5,
                });
                for _ in 0..5 {
                    let px = x + next_int(random, 16) + 8;
                    let pz = z + next_int(random, 16) + 8;
                    let py = next_int(random, world.get_height(px, pz) + 32);
                    if double_plant.place(world, random, px, py, pz) {
                        break;
                    }
                }
            }

            base(decorator, world, random, &biome.config);
        }
        BiomeDecorator::Savanna => {
            plant_double(world, random, x, z, 2, 7);
            base(decorator, world, random, &biome.config);
        }
        BiomeDecorator::Desert => {
            base(decorator, world, random, &biome.config);
            if next_int(random, 1000) == 0 {
                let px = x + next_int(random, 16) + 8;
                let pz = z + next_int(random, 16) + 8;
                let py = world.get_height(px, pz) + 1;
                DesertWellFeature.place(world, px, py, pz);
            }
        }
        BiomeDecorator::ExtremeHills => {
            base(decorator, world, random, &biome.config);

            let attempts = 3 + next_int(random, 6);
            for _ in 0..attempts {
                let px = x + next_int(random, 16);
                let py = next_int(random, 28) + 4;
                let pz = z + next_int(random, 16);
                if world.get_block(px, py, pz) == Block::Stone {
                    world.set_block_with_metadata(
                        px,
                        py,
                        pz,
                        Block::EmeraldOre,
                        0,
                        FLAG_NO_NEIGHBOR_NOTIFY,
                    );
                }
            }

            let monster_egg = VeinFeature::new(Block::MonsterEgg, 8);
            for _ in 0..7 {
                let px = x + next_int(random, 16);
                let py = next_int(random, 64);
                let pz = z + next_int(random, 16);
                monster_egg.place(world, random, px, py, pz);
            }
        }
        BiomeDecorator::IceSpikes => {
            for _ in 0..3 {
                let px = x + next_int(random, 16) + 8;
                let pz = z + next_int(random, 16) + 8;
                let py = world.get_height(px, pz);
                IceSpikeFeature.place(world, random, px, py, pz);
            }

            for _ in 0..2 {
                let px = x + next_int(random, 16) + 8;
                let pz = z + next_int(random, 16) + 8;
                let py = world.get_height(px, pz);
                IcePatchFeature::new(4).place(world, random, px, py, pz);
            }

            base(decorator, world, random, &biome.config);
        }
        BiomeDecorator::Jungle => {
            base(decorator, world, random, &biome.config);

            let px = x + next_int(random, 16) + 8;
            let pz = z + next_int(random, 16) + 8;
            let py = next_int(random, world.get_height(px, pz) * 2);
            MelonPatchFeature.place(world, random, px, py, pz);

            // Vanilla asks for vines at y 128, where the feature's own loop
            // never runs: this only draws the two coordinates.
            for _ in 0..50 {
                let vx = x + next_int(random, 16) + 8;
                let vz = z + next_int(random, 16) + 8;
                VineFeature.place(world, random, vx, 128, vz);
            }
        }
        BiomeDecorator::Taiga { variant } => {
            if variant == 1 || variant == 2 {
                let mossy_boulder = BlockVeinFeature::new(Block::MossyCobblestone, 0);
                let attempts = next_int(random, 3);
                for _ in 0..attempts {
                    let px = x + next_int(random, 16) + 8;
                    let pz = z + next_int(random, 16) + 8;
                    let py = world.get_height(px, pz);
                    mossy_boulder.place(world, random, px, py, pz);
                }
            }

            plant_double(world, random, x, z, 3, 7);
            base(decorator, world, random, &biome.config);
        }
    }
}

/// The seven-attempt double-plant scatter the plains, forest, savanna and
/// taiga overrides share: a fresh site and a `nextInt(height + 32)` each.
fn plant_double(
    world: &mut dyn GenWorld,
    random: &mut LegacyRandom,
    x: i32,
    z: i32,
    variant: u8,
    attempts: i32,
) {
    let mut double_plant = DoublePlantFeature::new();
    double_plant.set_variant(variant);
    for _ in 0..attempts {
        let px = x + next_int(random, 16) + 8;
        let pz = z + next_int(random, 16) + 8;
        let py = next_int(random, world.get_height(px, pz) + 32);
        double_plant.place(world, random, px, py, pz);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk_world::ChunkWorld;
    use crate::decorator::STAGE_ALL;

    fn grass_platform(origin_x: i32, origin_z: i32) -> ChunkWorld {
        let mut world = ChunkWorld::new(origin_x, origin_z);
        world.fill(0, 40, Block::Grass);
        world
    }

    #[test]
    fn plains_draws_its_double_plants_before_the_base_body() {
        let biome = crate::biome::biome(1);
        // Chunk (0, 0) sits above the -0.8 foliage threshold, so the plains
        // run the seven double-plant sites and keep 4 flowers / 10 grass.
        assert!(foliage(8.0 / 200.0, 8.0 / 200.0) >= -0.8);
        assert_eq!(biome.config.flower_attempts, 4);
        assert_eq!(biome.config.grass_attempts, 10);

        let mut world = grass_platform(0, 0);
        let mut random = LegacyRandom::new(9);
        crate::decorator::FeatureDecorator::new().decorate(&mut world, &mut random, &biome, 0, 0);
        assert!(world.block_count(Block::DoublePlant) > 0);

        // The same chunk replayed: seven double-plant sites, then the base
        // body. Both worlds see the same terrain as the pass writes.
        let mut reference_world = grass_platform(0, 0);
        let mut reference = LegacyRandom::new(9);
        let mut scratch = grass_platform(0, 0);
        let mut double_plant = DoublePlantFeature::new();
        double_plant.set_variant(2);
        for _ in 0..7 {
            let px = next_int(&mut reference, 16) + 8;
            let pz = next_int(&mut reference, 16) + 8;
            let py = next_int(&mut reference, scratch.get_height(px, pz) + 32);
            double_plant.place(&mut scratch, &mut reference, px, py, pz);
        }
        crate::decorator::FeatureDecorator::new().decorate_upto(
            &mut reference_world,
            &mut reference,
            &biome,
            0,
            0,
            STAGE_ALL,
        );
        assert_eq!(random.state(), reference.state());
    }

    #[test]
    fn the_dry_plains_skip_the_double_plants() {
        let biome = crate::biome::biome(1);
        // Chunk (-784, -784) is under the -0.8 foliage threshold.
        let (x, z) = (-784, -784);
        assert!(foliage((f64::from(x) + 8.0) / 200.0, (f64::from(z) + 8.0) / 200.0) < -0.8);

        let mut wet_plants = 0;
        let mut dry_plants = 0;
        for seed in 0..16 {
            let mut wet = grass_platform(0, 0);
            crate::decorator::FeatureDecorator::new().decorate(
                &mut wet,
                &mut LegacyRandom::new(seed),
                &biome,
                0,
                0,
            );
            wet_plants += wet.block_count(Block::DoublePlant);

            let mut dry = grass_platform(x, z);
            crate::decorator::FeatureDecorator::new().decorate(
                &mut dry,
                &mut LegacyRandom::new(seed),
                &biome,
                x,
                z,
            );
            dry_plants += dry.block_count(Block::DoublePlant);
        }
        assert!(wet_plants > 0);
        assert_eq!(dry_plants, 0);
    }

    #[test]
    fn desert_places_a_well_on_the_one_in_a_thousand_draw() {
        // Seed 2847 takes the `nextInt(1000) == 0` branch of the desert.
        let mut world = ChunkWorld::new(0, 0);
        world.fill(0, 40, Block::Sand);
        crate::decorator::FeatureDecorator::new().decorate(
            &mut world,
            &mut LegacyRandom::new(2847),
            &crate::biome::biome(2),
            0,
            0,
        );
        assert!(world.block_count(Block::StoneSlab) > 0);
        assert!(world.block_count(Block::FlowingWater) > 0);
    }

    #[test]
    fn ice_spikes_build_packed_ice_before_the_base_body() {
        // Ice plains spikes (id 140) never come out of the biome layers, so
        // this is the only end-to-end pass over the override.
        let mut world = ChunkWorld::new(0, 0);
        world.fill(0, 63, Block::SnowBlock);
        crate::decorator::FeatureDecorator::new().decorate(
            &mut world,
            &mut LegacyRandom::new(7),
            &crate::biome::biome(140),
            0,
            0,
        );
        assert!(world.block_count(Block::PackedIce) > 0);
    }

    #[test]
    fn mega_taiga_scatters_mossy_boulders() {
        let mut mega = 0;
        for seed in 0..16 {
            let mut world = grass_platform(0, 0);
            crate::decorator::FeatureDecorator::new().decorate(
                &mut world,
                &mut LegacyRandom::new(seed),
                &crate::biome::biome(32),
                0,
                0,
            );
            mega += world.block_count(Block::MossyCobblestone);
        }
        assert!(mega > 0);

        // The plain taiga (variant 0) has no boulder pass at all.
        let mut plain = 0;
        for seed in 0..16 {
            let mut world = grass_platform(0, 0);
            crate::decorator::FeatureDecorator::new().decorate(
                &mut world,
                &mut LegacyRandom::new(seed),
                &crate::biome::biome(5),
                0,
                0,
            );
            plain += world.block_count(Block::MossyCobblestone);
        }
        assert_eq!(plain, 0);
    }
}
