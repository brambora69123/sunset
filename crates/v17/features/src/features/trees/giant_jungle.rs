//! `GiantJungleTreeFeature`: the 2x2 jungle giant, with its leaning branches
//! and trunk vines.

use crate::block::Block;
use crate::features::trees::giant::GiantTreeBase;
use crate::math;
use crate::world::{FLAG_NO_NEIGHBOR_NOTIFY, GenWorld, next_int};
use sunset_1_7_core::{LegacyRandom, RandomSource};

/// `GiantJungleTreeFeature(false, 10, 20, 3, 3)`.
pub struct GiantJungleTreeFeature {
    base: GiantTreeBase,
}

impl Default for GiantJungleTreeFeature {
    fn default() -> GiantJungleTreeFeature {
        GiantJungleTreeFeature::new()
    }
}

impl GiantJungleTreeFeature {
    pub fn new() -> GiantJungleTreeFeature {
        GiantJungleTreeFeature {
            base: GiantTreeBase::new(10, 20, 3, 3),
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

        // `placeLeaves(x, y + height, z, 2)`.
        for layer in (y + height - 2)..=(y + height) {
            let corner = layer - (y + height);
            self.base
                .place_leaves_ring_strict(world, x, layer, z, 3 - corner);
        }

        let mut branch_y = y + height - 2 - next_int(random, 4);
        while branch_y > y + height / 2 {
            let angle = random.next_float() * std::f32::consts::PI * 2.0;
            let cos = math::cos(angle);
            let sin = math::sin(angle);
            let mut branch_x = x + (0.5_f32 + cos * 4.0) as i32;
            let mut branch_z = z + (0.5_f32 + sin * 4.0) as i32;

            for step in 0..5 {
                branch_x = x + (1.5_f32 + cos * step as f32) as i32;
                branch_z = z + (1.5_f32 + sin * step as f32) as i32;
                world.set_block_with_metadata(
                    branch_x,
                    branch_y - 3 + step / 2,
                    branch_z,
                    Block::Log,
                    self.base.log_variant(),
                    FLAG_NO_NEIGHBOR_NOTIFY,
                );
            }

            let cluster = 1 + next_int(random, 2);
            for layer in (branch_y - cluster)..=branch_y {
                let corner = layer - branch_y;
                self.base
                    .place_leaves_ring(world, branch_x, layer, branch_z, 1 - corner);
            }

            branch_y -= 2 + next_int(random, 4);
        }

        for step in 0..height {
            let layer = y + step;
            self.trunk_column(world, random, x, layer, z, step > 0, (-1, 0, 8), (0, -1, 1));
            if step < height - 1 {
                self.trunk_column(
                    world,
                    random,
                    x + 1,
                    layer,
                    z,
                    step > 0,
                    (1, 0, 2),
                    (0, -1, 1),
                );
                self.trunk_column(
                    world,
                    random,
                    x + 1,
                    layer,
                    z + 1,
                    step > 0,
                    (1, 0, 2),
                    (0, 1, 4),
                );
                self.trunk_column(
                    world,
                    random,
                    x,
                    layer,
                    z + 1,
                    step > 0,
                    (-1, 0, 8),
                    (0, 1, 4),
                );
            }
        }

        true
    }

    /// One of the four trunk columns: the log, then its two vines, each behind
    /// its own `nextInt(3)` draw.
    fn trunk_column(
        &self,
        world: &mut dyn GenWorld,
        random: &mut LegacyRandom,
        x: i32,
        y: i32,
        z: i32,
        vine_step: bool,
        vine_a: (i32, i32, u8),
        vine_b: (i32, i32, u8),
    ) {
        let block = world.get_block(x, y, z);
        if !(block.is_air_material() || block.is_leaves()) {
            return;
        }
        world.set_block_with_metadata(
            x,
            y,
            z,
            Block::Log,
            self.base.log_variant(),
            FLAG_NO_NEIGHBOR_NOTIFY,
        );
        if !vine_step {
            return;
        }

        if next_int(random, 3) > 0 && world.is_air(x + vine_a.0, y, z + vine_a.1) {
            world.set_block_with_metadata(
                x + vine_a.0,
                y,
                z + vine_a.1,
                Block::Vine,
                vine_a.2,
                FLAG_NO_NEIGHBOR_NOTIFY,
            );
        }
        if next_int(random, 3) > 0 && world.is_air(x + vine_b.0, y, z + vine_b.1) {
            world.set_block_with_metadata(
                x + vine_b.0,
                y,
                z + vine_b.1,
                Block::Vine,
                vine_b.2,
                FLAG_NO_NEIGHBOR_NOTIFY,
            );
        }
    }
}
