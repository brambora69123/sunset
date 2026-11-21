//! `DarkOakTreeFeature`: the roofed forest's 2x2 dark oak, with its leaning
//! trunk, low canopy and root logs.

use crate::block::Block;
use crate::features::tree::TreeBase;
use crate::features::trees::{X_OFFSET, Z_OFFSET};
use crate::world::{FLAG_NO_NEIGHBOR_NOTIFY, GenWorld, next_int};
use sunset_1_7_core::{LegacyRandom, RandomSource};

/// `DarkOakTreeFeature(false)`.
pub struct DarkOakTreeFeature;

impl Default for DarkOakTreeFeature {
    fn default() -> DarkOakTreeFeature {
        DarkOakTreeFeature::new()
    }
}

impl DarkOakTreeFeature {
    pub fn new() -> DarkOakTreeFeature {
        DarkOakTreeFeature
    }

    pub fn place(
        &self,
        world: &mut dyn GenWorld,
        random: &mut LegacyRandom,
        x: i32,
        y: i32,
        z: i32,
    ) -> bool {
        let height = next_int(random, 3) + next_int(random, 2) + 6;
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
        for (dx, dz) in [(0, 0), (1, 0), (1, 1), (0, 1)] {
            world.set_block_with_metadata(
                x + dx,
                y - 1,
                z + dz,
                Block::Dirt,
                0,
                FLAG_NO_NEIGHBOR_NOTIFY,
            );
        }

        let direction = next_int(random, 4);
        let lean_start = height - next_int(random, 4);
        let mut lean_length = 2 - next_int(random, 3);
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
                for (dx, dz) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    world.set_block_with_metadata(
                        trunk_x + dx,
                        layer,
                        trunk_z + dz,
                        Block::Log2,
                        1,
                        FLAG_NO_NEIGHBOR_NOTIFY,
                    );
                }
                top = layer;
            }
        }

        for dx in -2..=0 {
            for dz in -2..=0 {
                self.place_leaves(world, trunk_x + dx, top - 1, trunk_z + dz);
                self.place_leaves(world, 1 + trunk_x - dx, top - 1, trunk_z + dz);
                self.place_leaves(world, trunk_x + dx, top - 1, 1 + trunk_z - dz);
                self.place_leaves(world, 1 + trunk_x - dx, top - 1, 1 + trunk_z - dz);
                if (dx > -2 || dz > -1) && (dx != -1 || dz != -2) {
                    self.place_leaves(world, trunk_x + dx, top + 1, trunk_z + dz);
                    self.place_leaves(world, 1 + trunk_x - dx, top + 1, trunk_z + dz);
                    self.place_leaves(world, trunk_x + dx, top + 1, 1 + trunk_z - dz);
                    self.place_leaves(world, 1 + trunk_x - dx, top + 1, 1 + trunk_z - dz);
                }
            }
        }

        if random.next_boolean() {
            self.place_leaves(world, trunk_x, top + 2, trunk_z);
            self.place_leaves(world, trunk_x + 1, top + 2, trunk_z);
            self.place_leaves(world, trunk_x + 1, top + 2, trunk_z + 1);
            self.place_leaves(world, trunk_x, top + 2, trunk_z + 1);
        }

        for dx in -3_i32..=4 {
            for dz in -3_i32..=4 {
                if (dx != -3 || dz != -3)
                    && (dx != -3 || dz != 4)
                    && (dx != 4 || dz != -3)
                    && (dx != 4 || dz != 4)
                    && (dx.abs() < 3 || dz.abs() < 3)
                {
                    self.place_leaves(world, trunk_x + dx, top, trunk_z + dz);
                }
            }
        }

        for dx in -1_i32..=2 {
            for dz in -1_i32..=2 {
                if (!(0..=1).contains(&dx) || !(0..=1).contains(&dz)) && next_int(random, 3) <= 0 {
                    let length = next_int(random, 3) + 2;
                    for step in 0..length {
                        world.set_block_with_metadata(
                            x + dx,
                            top - step - 1,
                            z + dz,
                            Block::Log2,
                            1,
                            FLAG_NO_NEIGHBOR_NOTIFY,
                        );
                    }
                    for ax in -1_i32..=1 {
                        for az in -1_i32..=1 {
                            self.place_leaves(world, trunk_x + dx + ax, top, trunk_z + dz + az);
                        }
                    }
                    for ax in -2_i32..=2 {
                        for az in -2_i32..=2 {
                            if ax.abs() != 2 || az.abs() != 2 {
                                self.place_leaves(
                                    world,
                                    trunk_x + dx + ax,
                                    top - 1,
                                    trunk_z + dz + az,
                                );
                            }
                        }
                    }
                }
            }
        }

        true
    }

    /// `placeLeaves`: only through air.
    fn place_leaves(&self, world: &mut dyn GenWorld, x: i32, y: i32, z: i32) {
        if world.get_block(x, y, z).is_air_material() {
            world.set_block_with_metadata(x, y, z, Block::Leaves2, 1, FLAG_NO_NEIGHBOR_NOTIFY);
        }
    }
}
