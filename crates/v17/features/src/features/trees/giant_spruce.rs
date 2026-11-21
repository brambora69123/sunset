//! `GiantSpruceTreeFeature`: the 2x2 taiga giant (`sparseLeaves` selects the
//! long spruce crown over the short pine one), with its podzol soil.

use crate::block::Block;
use crate::features::trees::giant::GiantTreeBase;
use crate::math;
use crate::world::{FLAG_NO_NEIGHBOR_NOTIFY, GenWorld, next_int};
use sunset_1_7_core::LegacyRandom;

/// `GiantSpruceTreeFeature(false, sparseLeaves)`; `super(false, 13, 15, 1, 1)`.
pub struct GiantSpruceTreeFeature {
    base: GiantTreeBase,
    sparse_leaves: bool,
}

impl GiantSpruceTreeFeature {
    pub fn new(sparse_leaves: bool) -> GiantSpruceTreeFeature {
        GiantSpruceTreeFeature {
            base: GiantTreeBase::new(13, 15, 1, 1),
            sparse_leaves,
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
        let height = self.base.random_height(random);
        if !self.base.can_grow(world, x, y, z, height) {
            return false;
        }

        self.place_leaves(world, random, x, y + height, z);

        for step in 0..height {
            let layer = y + step;
            self.base.place_log(world, x, layer, z);
            if step < height - 1 {
                self.base.place_log(world, x + 1, layer, z);
                self.base.place_log(world, x + 1, layer, z + 1);
                self.base.place_log(world, x, layer, z + 1);
            }
        }

        true
    }

    /// `placeLeaves`: the crown's `nextInt(5)` and its strict rings.
    fn place_leaves(
        &self,
        world: &mut dyn GenWorld,
        random: &mut LegacyRandom,
        x: i32,
        y: i32,
        z: i32,
    ) {
        let mut crown = next_int(random, 5);
        if self.sparse_leaves {
            crown += self.base.base_height;
        } else {
            crown += 3;
        }

        let mut previous = 0;
        for layer in (y - crown)..=y {
            let depth = y - layer;
            let radius = math::floor(f64::from(depth as f32 / crown as f32 * 3.5));
            let extra = if depth > 0 && radius == previous && layer % 2 == 0 {
                1
            } else {
                0
            };
            self.base
                .place_leaves_ring_strict(world, x, layer, z, radius + extra);
            previous = radius;
        }
    }

    /// `placeSoil`: the four corners and five scattered podzol patches.
    pub fn place_soil(
        &self,
        world: &mut dyn GenWorld,
        random: &mut LegacyRandom,
        x: i32,
        y: i32,
        z: i32,
    ) {
        self.place_podzol_patch(world, x - 1, y, z - 1);
        self.place_podzol_patch(world, x + 2, y, z - 1);
        self.place_podzol_patch(world, x - 1, y, z + 2);
        self.place_podzol_patch(world, x + 2, y, z + 2);

        for _ in 0..5 {
            let value = next_int(random, 64);
            let dx = value % 8;
            let dz = value / 8;
            if dx == 0 || dx == 7 || dz == 0 || dz == 7 {
                self.place_podzol_patch(world, x - 3 + dx, y, z - 3 + dz);
            }
        }
    }

    fn place_podzol_patch(&self, world: &mut dyn GenWorld, x: i32, y: i32, z: i32) {
        for dx in -2_i32..=2 {
            for dz in -2_i32..=2 {
                if dx.abs() != 2 || dz.abs() != 2 {
                    self.place_podzol(world, x + dx, y, z + dz);
                }
            }
        }
    }

    /// `placePodzol`: the topmost grass or dirt in the five-block column.
    fn place_podzol(&self, world: &mut dyn GenWorld, x: i32, y: i32, z: i32) {
        for layer in ((y - 3)..=(y + 2)).rev() {
            let block = world.get_block(x, layer, z);
            if matches!(block, Block::Grass | Block::Dirt) {
                world.set_block_with_metadata(x, layer, z, Block::Dirt, 2, FLAG_NO_NEIGHBOR_NOTIFY);
                break;
            }
            if !block.is_air_material() && layer < y {
                break;
            }
        }
    }
}
