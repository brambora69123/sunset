//! `GiantTreeFeature`: the height, clearance, soil and canopy shared by the
//! 2x2 giants (`GiantJungleTreeFeature`, `GiantSpruceTreeFeature`).

use crate::block::Block;
use crate::features::tree::TreeBase;
use crate::world::{FLAG_NO_NEIGHBOR_NOTIFY, GenWorld, next_int};
use sunset_1_7_core::LegacyRandom;

/// The `GiantTreeFeature` state its subclasses read.
pub(crate) struct GiantTreeBase {
    pub(crate) base_height: i32,
    bonus_height: i32,
    log_variant: u8,
    leaves_variant: u8,
}

impl GiantTreeBase {
    pub(crate) fn new(
        base_height: i32,
        bonus_height: i32,
        log_variant: u8,
        leaves_variant: u8,
    ) -> GiantTreeBase {
        GiantTreeBase {
            base_height,
            bonus_height,
            log_variant,
            leaves_variant,
        }
    }

    pub(crate) fn log_variant(&self) -> u8 {
        self.log_variant
    }

    /// `getRandomHeight`: one `nextInt(3)` plus, for a real bonus range, one
    /// `nextInt(bonusHeight)`.
    pub(crate) fn random_height(&self, random: &mut LegacyRandom) -> i32 {
        let mut height = next_int(random, 3) + self.base_height;
        if self.bonus_height > 1 {
            height += next_int(random, self.bonus_height);
        }
        height
    }

    /// `canGrow`: the 2-wide clearance (1 at the base layer) and the four
    /// dirt blocks underneath.
    pub(crate) fn can_grow(
        &self,
        world: &mut dyn GenWorld,
        x: i32,
        y: i32,
        z: i32,
        height: i32,
    ) -> bool {
        let base = TreeBase::new(false);
        if !base.can_place(
            world,
            x,
            y,
            z,
            height,
            |layer| {
                if layer == y { 1 } else { 2 }
            },
        ) {
            return false;
        }

        let below = world.get_block(x, y - 1, z);
        if !matches!(below, Block::Grass | Block::Dirt) || y < 2 {
            return false;
        }
        for (dx, dz) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
            world.set_block_with_metadata(
                x + dx,
                y - 1,
                z + dz,
                Block::Dirt,
                0,
                FLAG_NO_NEIGHBOR_NOTIFY,
            );
        }
        true
    }

    /// `placeLeavesRingStrict`: the four overlapping circles' union.
    pub(crate) fn place_leaves_ring_strict(
        &self,
        world: &mut dyn GenWorld,
        x: i32,
        y: i32,
        z: i32,
        radius: i32,
    ) {
        let limit = radius * radius;
        for bx in (x - radius)..=(x + radius + 1) {
            let dx = bx - x;
            let dx_prev = dx - 1;
            for bz in (z - radius)..=(z + radius + 1) {
                let dz = bz - z;
                let dz_prev = dz - 1;
                if dx * dx + dz * dz <= limit
                    || dx_prev * dx_prev + dz_prev * dz_prev <= limit
                    || dx * dx + dz_prev * dz_prev <= limit
                    || dx_prev * dx_prev + dz * dz <= limit
                {
                    self.place_leaf(world, bx, y, bz);
                }
            }
        }
    }

    /// `placeLeavesRing`: one circle around the branch tip.
    pub(crate) fn place_leaves_ring(
        &self,
        world: &mut dyn GenWorld,
        x: i32,
        y: i32,
        z: i32,
        radius: i32,
    ) {
        let limit = radius * radius;
        for bx in (x - radius)..=(x + radius) {
            let dx = bx - x;
            for bz in (z - radius)..=(z + radius) {
                let dz = bz - z;
                if dx * dx + dz * dz <= limit {
                    self.place_leaf(world, bx, y, bz);
                }
            }
        }
    }

    fn place_leaf(&self, world: &mut dyn GenWorld, x: i32, y: i32, z: i32) {
        let block = world.get_block(x, y, z);
        if block.is_air_material() || block.is_leaves() {
            world.set_block_with_metadata(
                x,
                y,
                z,
                Block::Leaves,
                self.leaves_variant,
                FLAG_NO_NEIGHBOR_NOTIFY,
            );
        }
    }

    /// One trunk log, placed only through air or leaves.
    pub(crate) fn place_log(&self, world: &mut dyn GenWorld, x: i32, y: i32, z: i32) {
        let block = world.get_block(x, y, z);
        if block.is_air_material() || block.is_leaves() {
            world.set_block_with_metadata(
                x,
                y,
                z,
                Block::Log,
                self.log_variant,
                FLAG_NO_NEIGHBOR_NOTIFY,
            );
        }
    }
}
