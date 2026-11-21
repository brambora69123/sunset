//! `AcaciaTreeFeature` (WorldGenSavannaTree): the savanna acacia, a leaning
//! trunk with an optional second branch.

use crate::block::Block;
use crate::features::tree::TreeBase;
use crate::features::trees::{X_OFFSET, Z_OFFSET};
use crate::world::{FLAG_NO_NEIGHBOR_NOTIFY, GenWorld, next_int};
use sunset_1_7_core::LegacyRandom;

/// `AcaciaTreeFeature(false)`.
pub struct AcaciaTreeFeature;

impl Default for AcaciaTreeFeature {
    fn default() -> AcaciaTreeFeature {
        AcaciaTreeFeature::new()
    }
}

impl AcaciaTreeFeature {
    pub fn new() -> AcaciaTreeFeature {
        AcaciaTreeFeature
    }

    pub fn place(
        &self,
        world: &mut dyn GenWorld,
        random: &mut LegacyRandom,
        x: i32,
        y: i32,
        z: i32,
    ) -> bool {
        let height = next_int(random, 3) + next_int(random, 3) + 5;
        let base = TreeBase::new(false);
        if !base.can_place(world, x, y, z, height, |layer| {
            if layer == y {
                0
            } else if layer >= y + 1 + height - 2 {
                2
            } else {
                1
            }
        }) {
            return false;
        }

        let below = world.get_block(x, y - 1, z);
        if !matches!(below, Block::Grass | Block::Dirt) || y >= 256 - height - 1 {
            return false;
        }
        world.set_block_with_metadata(x, y - 1, z, Block::Dirt, 0, base.flag());

        let direction = next_int(random, 4);
        let lean_start = height - next_int(random, 4) - 1;
        let mut lean_length = 3 - next_int(random, 3);
        let mut trunk_x = x;
        let mut trunk_z = z;
        let mut top = 0;

        for step in 0..height {
            let layer = y + step;
            if step >= lean_start && lean_length > 0 {
                trunk_x += X_OFFSET[direction as usize];
                trunk_z += Z_OFFSET[direction as usize];
                lean_length -= 1;
            }

            let block = world.get_block(trunk_x, layer, trunk_z);
            if block.is_air_material() || block.is_leaves() {
                world.set_block_with_metadata(
                    trunk_x,
                    layer,
                    trunk_z,
                    Block::Log2,
                    0,
                    FLAG_NO_NEIGHBOR_NOTIFY,
                );
                top = layer;
            }
        }

        for dx in -1_i32..=1 {
            for dz in -1_i32..=1 {
                self.place_leaves(world, trunk_x + dx, top + 1, trunk_z + dz);
            }
        }
        self.place_leaves(world, trunk_x + 2, top + 1, trunk_z);
        self.place_leaves(world, trunk_x - 2, top + 1, trunk_z);
        self.place_leaves(world, trunk_x, top + 1, trunk_z + 2);
        self.place_leaves(world, trunk_x, top + 1, trunk_z - 2);

        for dx in -3_i32..=3 {
            for dz in -3_i32..=3 {
                if dx.abs() != 3 || dz.abs() != 3 {
                    self.place_leaves(world, trunk_x + dx, top, trunk_z + dz);
                }
            }
        }

        trunk_x = x;
        trunk_z = z;
        let branch_direction = next_int(random, 4);
        if branch_direction != direction {
            let mut branch_step = lean_start - next_int(random, 2) - 1;
            let mut branch_length = 1 + next_int(random, 3);
            top = 0;

            while branch_step < height && branch_length > 0 {
                if branch_step >= 1 {
                    let layer = y + branch_step;
                    trunk_x += X_OFFSET[branch_direction as usize];
                    trunk_z += Z_OFFSET[branch_direction as usize];
                    let block = world.get_block(trunk_x, layer, trunk_z);
                    if block.is_air_material() || block.is_leaves() {
                        world.set_block_with_metadata(
                            trunk_x,
                            layer,
                            trunk_z,
                            Block::Log2,
                            0,
                            FLAG_NO_NEIGHBOR_NOTIFY,
                        );
                        top = layer;
                    }
                }
                branch_step += 1;
                branch_length -= 1;
            }

            if top > 0 {
                for dx in -1_i32..=1 {
                    for dz in -1_i32..=1 {
                        self.place_leaves(world, trunk_x + dx, top + 1, trunk_z + dz);
                    }
                }
                for dx in -2_i32..=2 {
                    for dz in -2_i32..=2 {
                        if dx.abs() != 2 || dz.abs() != 2 {
                            self.place_leaves(world, trunk_x + dx, top, trunk_z + dz);
                        }
                    }
                }
            }
        }

        true
    }

    /// `placeLeaves`: only through air or leaves, always acacia leaves.
    fn place_leaves(&self, world: &mut dyn GenWorld, x: i32, y: i32, z: i32) {
        let block = world.get_block(x, y, z);
        if block.is_air_material() || block.is_leaves() {
            world.set_block_with_metadata(x, y, z, Block::Leaves2, 0, FLAG_NO_NEIGHBOR_NOTIFY);
        }
    }
}
