//! `BushFeature` (WorldGenShrub): the jungle's two-block shrub, built like
//! `TreeFeature` but rooted at the first ground below the asked height.

use crate::block::Block;
use crate::world::{FLAG_NO_NEIGHBOR_NOTIFY, GenWorld, next_int};
use sunset_1_7_core::LegacyRandom;

/// `BushFeature(logVariant, leavesVariant)`; `JungleBiome.pickTree` uses
/// `BushFeature(3, 0)`.
pub struct BushFeature {
    log_variant: u8,
    leaves_variant: u8,
}

impl BushFeature {
    pub fn new(log_variant: u8, leaves_variant: u8) -> BushFeature {
        BushFeature {
            log_variant,
            leaves_variant,
        }
    }

    pub fn place(
        &self,
        world: &mut dyn GenWorld,
        random: &mut LegacyRandom,
        x: i32,
        y: i32,
        z: i32,
    ) -> bool {
        let mut y = y;
        loop {
            let block = world.get_block(x, y, z);
            if !(block.is_air_material() || block.is_leaves()) || y <= 0 {
                break;
            }
            y -= 1;
        }

        let ground = world.get_block(x, y, z);
        if matches!(ground, Block::Dirt | Block::Grass) {
            y += 1;
            world.set_block_with_metadata(
                x,
                y,
                z,
                Block::Log,
                self.log_variant,
                FLAG_NO_NEIGHBOR_NOTIFY,
            );

            for layer in y..=(y + 2) {
                let step = layer - y;
                let radius = 2 - step;
                for bx in (x - radius)..=(x + radius) {
                    let dx = bx - x;
                    for bz in (z - radius)..=(z + radius) {
                        let dz = bz - z;
                        if (dx.abs() != radius || dz.abs() != radius || next_int(random, 2) != 0)
                            && !world.get_block(bx, layer, bz).is_opaque()
                        {
                            world.set_block_with_metadata(
                                bx,
                                layer,
                                bz,
                                Block::Leaves,
                                self.leaves_variant,
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
