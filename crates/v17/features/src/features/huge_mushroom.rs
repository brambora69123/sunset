//! `HugeMushroomFeature`: the big red and brown mushrooms.

use crate::block::Block;
use crate::world::{FLAG_NO_NEIGHBOR_NOTIFY, GenWorld, next_int};

pub(crate) struct HugeMushroomFeature {
    /// Fixed cap variant, or -1 to draw (`nextInt(2)`).
    pub variant: i32,
    pub notify_neighbors: bool,
}

impl HugeMushroomFeature {
    pub fn new() -> HugeMushroomFeature {
        HugeMushroomFeature {
            variant: -1,
            notify_neighbors: false,
        }
    }

    fn flag(&self) -> u8 {
        if self.notify_neighbors {
            3
        } else {
            FLAG_NO_NEIGHBOR_NOTIFY
        }
    }

    /// RNG order: `nextInt(2)` (cap kind, unless fixed), `nextInt(3) + 4`
    /// (stem height); then the cap placement draws nothing.
    pub fn place(
        &self,
        world: &mut dyn GenWorld,
        random: &mut sunset_1_7_core::LegacyRandom,
        x: i32,
        y: i32,
        z: i32,
    ) -> bool {
        let mut kind = next_int(random, 2);
        if self.variant >= 0 {
            kind = self.variant;
        }
        let height = next_int(random, 3) + 4;

        if !(y >= 1 && y + height + 1 < 256) {
            return false;
        }

        // Clearance: air or leaves only, radius 3 above the stem base.
        for layer in y..=(y + 1 + height) {
            let radius = if layer <= y + 3 { 0 } else { 3 };
            for bx in (x - radius)..=(x + radius) {
                for bz in (z - radius)..=(z + radius) {
                    if !(0..256).contains(&layer) {
                        return false;
                    }
                    let block = world.get_block(bx, layer, bz);
                    if !(block.is_air_material() || matches!(block, Block::Leaves | Block::Leaves2))
                    {
                        return false;
                    }
                }
            }
        }

        let below = world.get_block(x, y - 1, z);
        if !matches!(below, Block::Dirt | Block::Grass | Block::Mycelium) {
            return false;
        }

        let cap_block = if kind == 0 {
            Block::BrownMushroomBlock
        } else {
            Block::RedMushroomBlock
        };
        let cap_y = if kind == 1 {
            y + height - 3
        } else {
            y + height
        };

        for by in cap_y..=(y + height) {
            let mut radius = 1;
            if by < y + height {
                radius += 1;
            }
            if kind == 0 {
                radius = 3;
            }

            for bx in (x - radius)..=(x + radius) {
                for bz in (z - radius)..=(z + radius) {
                    let mut metadata = 5_u8;
                    if bx == x - radius {
                        metadata -= 1;
                    }
                    if bx == x + radius {
                        metadata += 1;
                    }
                    if bz == z - radius {
                        metadata -= 3;
                    }
                    if bz == z + radius {
                        metadata += 3;
                    }

                    if kind == 0 || by < y + height {
                        let corner_x = bx == x - radius || bx == x + radius;
                        let corner_z = bz == z - radius || bz == z + radius;
                        if corner_x && corner_z {
                            continue;
                        }

                        if bx == x - (radius - 1) && bz == z - radius {
                            metadata = 1;
                        }
                        if bx == x - radius && bz == z - (radius - 1) {
                            metadata = 1;
                        }
                        if bx == x + (radius - 1) && bz == z - radius {
                            metadata = 3;
                        }
                        if bx == x + radius && bz == z - (radius - 1) {
                            metadata = 3;
                        }
                        if bx == x - (radius - 1) && bz == z + radius {
                            metadata = 7;
                        }
                        if bx == x - radius && bz == z + (radius - 1) {
                            metadata = 7;
                        }
                        if bx == x + (radius - 1) && bz == z + radius {
                            metadata = 9;
                        }
                        if bx == x + radius && bz == z + (radius - 1) {
                            metadata = 9;
                        }
                    }

                    if metadata == 5 && by < y + height {
                        metadata = 0;
                    }

                    if (metadata != 0 || y >= y + height - 1)
                        && !world.get_block(bx, by, bz).is_opaque()
                    {
                        world.set_block_with_metadata(bx, by, bz, cap_block, metadata, self.flag());
                    }
                }
            }
        }

        for step in 0..height {
            if !world.get_block(x, y + step, z).is_opaque() {
                world.set_block_with_metadata(x, y + step, z, cap_block, 10, self.flag());
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
    fn mushroom_grows_on_grass() {
        let mut world = ChunkWorld::new(0, 0);
        world.fill(0, 40, Block::Grass);
        let mut rng = sunset_1_7_core::LegacyRandom::new(51);
        let mushroom = HugeMushroomFeature::new();
        assert!(mushroom.place(&mut world, &mut rng, 8, 41, 8));
        assert!(
            world.block_count(Block::BrownMushroomBlock) > 0
                || world.block_count(Block::RedMushroomBlock) > 0
        );
    }

    #[test]
    fn fixed_variant_skips_the_kind_draw() {
        let mut world = ChunkWorld::new(0, 0);
        world.fill(0, 40, Block::Grass);
        let mut rng = sunset_1_7_core::LegacyRandom::new(2);
        let mushroom = HugeMushroomFeature {
            variant: 1,
            notify_neighbors: false,
        };
        assert!(mushroom.place(&mut world, &mut rng, 8, 41, 8));
        assert!(world.block_count(Block::RedMushroomBlock) > 0);
        assert_eq!(world.block_count(Block::BrownMushroomBlock), 0);
    }
}
