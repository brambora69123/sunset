//! `BlockPatchFeature` (sand/gravel patches) and `ClayPatchFeature`.

use crate::block::Block;
use crate::world::{FLAG_NO_NEIGHBOR_NOTIFY, GenWorld, next_int};

/// `BlockPatchFeature` (WorldGenSand): a disc of `block` where dirt or grass
/// sits, centred on a water block.
pub(crate) struct BlockPatchFeature {
    pub block: Block,
    pub size: i32,
}

impl BlockPatchFeature {
    /// RNG order: the water check runs first (no draw when it fails), then
    /// one `nextInt(size - 2) + 2`.
    pub fn place(
        &self,
        world: &mut dyn GenWorld,
        random: &mut sunset_1_7_core::LegacyRandom,
        x: i32,
        y: i32,
        z: i32,
    ) -> bool {
        if world.get_block(x, y, z) != Block::Water {
            return false;
        }

        let radius = next_int(random, self.size - 2) + 2;

        for bx in (x - radius)..=(x + radius) {
            for bz in (z - radius)..=(z + radius) {
                let dx = bx - x;
                let dz = bz - z;
                if dx * dx + dz * dz <= radius * radius {
                    for by in (y - 2)..=(y + 2) {
                        let block = world.get_block(bx, by, bz);
                        if block == Block::Dirt || block == Block::Grass {
                            world.set_block_with_metadata(
                                bx,
                                by,
                                bz,
                                self.block,
                                0,
                                FLAG_NO_NEIGHBOR_NOTIFY,
                            );
                        }
                    }
                }
            }
        }

        true
    }
}

/// `ClayPatchFeature`: a shallow clay disc under water.
pub(crate) struct ClayPatchFeature {
    pub size: i32,
}

impl ClayPatchFeature {
    pub fn place(
        &self,
        world: &mut dyn GenWorld,
        random: &mut sunset_1_7_core::LegacyRandom,
        x: i32,
        y: i32,
        z: i32,
    ) -> bool {
        if world.get_block(x, y, z) != Block::Water {
            return false;
        }

        let radius = next_int(random, self.size - 2) + 2;

        for bx in (x - radius)..=(x + radius) {
            for bz in (z - radius)..=(z + radius) {
                let dx = bx - x;
                let dz = bz - z;
                if dx * dx + dz * dz <= radius * radius {
                    for by in (y - 1)..=(y + 1) {
                        let block = world.get_block(bx, by, bz);
                        if block == Block::Dirt || block == Block::Clay {
                            world.set_block_with_metadata(
                                bx,
                                by,
                                bz,
                                Block::Clay,
                                0,
                                FLAG_NO_NEIGHBOR_NOTIFY,
                            );
                        }
                    }
                }
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk_world::ChunkWorld;

    #[test]
    fn patch_fails_without_water_at_the_centre() {
        let mut world = ChunkWorld::new(0, 0);
        world.fill(0, 40, Block::Stone);
        let mut rng = sunset_1_7_core::LegacyRandom::new(3);
        let state = rng.state();
        let patch = BlockPatchFeature {
            block: Block::Sand,
            size: 7,
        };
        assert!(!patch.place(&mut world, &mut rng, 8, 20, 8));
        // No draw consumed when the centre is not water.
        assert_eq!(rng.state(), state);
    }

    #[test]
    fn clay_replaces_dirt_under_water() {
        let mut world = ChunkWorld::new(0, 0);
        world.fill(0, 40, Block::Dirt);
        for x in 0..16 {
            for z in 0..16 {
                world.set_block_with_metadata(x, 41, z, Block::Water, 0, FLAG_NO_NEIGHBOR_NOTIFY);
            }
        }
        let mut rng = sunset_1_7_core::LegacyRandom::new(5);
        let clay = ClayPatchFeature { size: 4 };
        assert!(clay.place(&mut world, &mut rng, 8, 41, 8));
        let before = 16 * 16 * 41;
        assert!(world.block_count(Block::Clay) > 0);
        assert!(world.block_count(Block::Dirt) < before);
    }
}
