//! `LakeFeature` (WorldGenLakes): underground water and lava lakes.

use crate::block::Block;
use crate::world::{FLAG_NO_NEIGHBOR_NOTIFY, GenWorld, next_int};
use sunset_1_7_core::RandomSource;

fn blob_index(lx: i32, lz: i32, ly: i32) -> usize {
    ((lx * 16 + lz) * 8 + ly) as usize
}

pub struct LakeFeature {
    liquid: Block,
}

impl LakeFeature {
    pub fn new(liquid: Block) -> LakeFeature {
        LakeFeature { liquid }
    }

    pub fn water() -> LakeFeature {
        LakeFeature::new(Block::Water)
    }

    pub fn lava() -> LakeFeature {
        LakeFeature::new(Block::Lava)
    }

    /// RNG order: `nextInt(4) + 4` blobs, six `nextDouble`s per blob; the
    /// lava border then draws `nextInt(2)` per exposed solid block.
    pub fn place(
        &self,
        world: &mut dyn GenWorld,
        random: &mut sunset_1_7_core::LegacyRandom,
        x: i32,
        y: i32,
        z: i32,
    ) -> bool {
        let x = x - 8;
        let z = z - 8;

        let mut y = y;
        while y > 5 && world.is_air(x, y, z) {
            y -= 1;
        }

        if y <= 4 {
            return false;
        }

        y -= 4;

        // `boolean[2048]`: `(localX * 16 + localZ) * 8 + localY`.
        let mut blob = [false; 2048];
        let blobs = next_int(random, 4) + 4;

        for _ in 0..blobs {
            let rx = random.next_double() * 6.0 + 3.0;
            let ry = random.next_double() * 4.0 + 2.0;
            let rz = random.next_double() * 6.0 + 3.0;
            let cx = random.next_double() * (16.0 - rx - 2.0) + 1.0 + rx / 2.0;
            let cy = random.next_double() * (8.0 - ry - 4.0) + 2.0 + ry / 2.0;
            let cz = random.next_double() * (16.0 - rz - 2.0) + 1.0 + rz / 2.0;

            for lx in 1..15_i32 {
                for lz in 1..15_i32 {
                    for ly in 1..7_i32 {
                        let dx = (lx as f64 - cx) / (rx / 2.0);
                        let dy = (ly as f64 - cy) / (ry / 2.0);
                        let dz = (lz as f64 - cz) / (rz / 2.0);
                        if dx * dx + dy * dy + dz * dz < 1.0 {
                            blob[blob_index(lx, lz, ly)] = true;
                        }
                    }
                }
            }
        }

        // The shape must border only solid blocks below the surface and
        // liquid (or the lake's own block) above it.
        for lx in 0..16_i32 {
            for lz in 0..16_i32 {
                for ly in 0..8_i32 {
                    let border = !blob[blob_index(lx, lz, ly)]
                        && ((lx < 15 && blob[blob_index(lx + 1, lz, ly)])
                            || (lx > 0 && blob[blob_index(lx - 1, lz, ly)])
                            || (lz < 15 && blob[blob_index(lx, lz + 1, ly)])
                            || (lz > 0 && blob[blob_index(lx, lz - 1, ly)])
                            || (ly < 7 && blob[blob_index(lx, lz, ly + 1)])
                            || (ly > 0 && blob[blob_index(lx, lz, ly - 1)]));
                    if border {
                        let block = world.get_block(x + lx, y + ly, z + lz);
                        if ly >= 4 && block.is_liquid() {
                            return false;
                        }
                        if ly < 4 && !block.is_solid() && block != self.liquid {
                            return false;
                        }
                    }
                }
            }
        }

        for lx in 0..16_i32 {
            for lz in 0..16_i32 {
                for ly in 0..8_i32 {
                    if blob[blob_index(lx, lz, ly)] {
                        world.set_block_with_metadata(
                            x + lx,
                            y + ly,
                            z + lz,
                            if ly >= 4 { Block::Air } else { self.liquid },
                            0,
                            FLAG_NO_NEIGHBOR_NOTIFY,
                        );
                    }
                }
            }
        }

        // Grass back over the dirt floor, sky-lit.
        for lx in 0..16_i32 {
            for lz in 0..16_i32 {
                for ly in 4..8_i32 {
                    if blob[blob_index(lx, lz, ly)]
                        && world.get_block(x + lx, y + ly - 1, z + lz) == Block::Dirt
                    {
                        let surface = if world.get_biome_id(x + lx, z + lz) == 14
                            || world.get_biome_id(x + lx, z + lz) == 15
                        {
                            Block::Mycelium
                        } else {
                            Block::Grass
                        };
                        world.set_block_with_metadata(
                            x + lx,
                            y + ly - 1,
                            z + lz,
                            surface,
                            0,
                            FLAG_NO_NEIGHBOR_NOTIFY,
                        );
                    }
                }
            }
        }

        // Lava lakes stone over their exposed faces; water lakes freeze over.
        if self.liquid == Block::Lava {
            for lx in 0..16_i32 {
                for lz in 0..16_i32 {
                    for ly in 0..8_i32 {
                        let border = !blob[blob_index(lx, lz, ly)]
                            && ((lx < 15 && blob[blob_index(lx + 1, lz, ly)])
                                || (lx > 0 && blob[blob_index(lx - 1, lz, ly)])
                                || (lz < 15 && blob[blob_index(lx, lz + 1, ly)])
                                || (lz > 0 && blob[blob_index(lx, lz - 1, ly)])
                                || (ly < 7 && blob[blob_index(lx, lz, ly + 1)])
                                || (ly > 0 && blob[blob_index(lx, lz, ly - 1)]));
                        if border
                            && (ly < 4 || next_int(random, 2) != 0)
                            && world.get_block(x + lx, y + ly, z + lz).is_solid()
                        {
                            world.set_block_with_metadata(
                                x + lx,
                                y + ly,
                                z + lz,
                                Block::Stone,
                                0,
                                FLAG_NO_NEIGHBOR_NOTIFY,
                            );
                        }
                    }
                }
            }
        } else {
            for lx in 0..16_i32 {
                for lz in 0..16_i32 {
                    if world.can_freeze(x + lx, y + 4, z + lz) {
                        world.set_block_with_metadata(
                            x + lx,
                            y + 4,
                            z + lz,
                            Block::Ice,
                            0,
                            FLAG_NO_NEIGHBOR_NOTIFY,
                        );
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
    fn water_lake_forms_under_solid_ground() {
        let mut world = ChunkWorld::new(0, 0);
        world.fill(0, 60, Block::Stone);
        let mut rng = sunset_1_7_core::LegacyRandom::new(11);
        let lake = LakeFeature::water();
        assert!(lake.place(&mut world, &mut rng, 8, 40, 8));
        assert!(world.block_count(Block::Water) > 0);
    }

    #[test]
    fn lake_fails_in_open_air() {
        let mut world = ChunkWorld::new(0, 0);
        // One floating block: the walk stops on it and the lake would sit in
        // open air, which the border check rejects.
        world.set_block_with_metadata(0, 20, 0, Block::Stone, 0, FLAG_NO_NEIGHBOR_NOTIFY);
        let mut rng = sunset_1_7_core::LegacyRandom::new(2);
        let lake = LakeFeature::water();
        assert!(!lake.place(&mut world, &mut rng, 8, 30, 8));
        assert_eq!(world.block_count(Block::Water), 0);
    }
}
