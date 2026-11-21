//! `VeinFeature` (WorldGenMinable): ore and dirt/gravel veins.

use crate::block::Block;
use crate::math;
use crate::world::{FLAG_NO_NEIGHBOR_NOTIFY, FLAG_SILENT, GenWorld, next_int};
use sunset_1_7_core::RandomSource;

pub struct VeinFeature {
    pub block: Block,
    pub size: i32,
    pub replaceable: Block,
}

impl VeinFeature {
    pub fn new(block: Block, size: i32) -> VeinFeature {
        VeinFeature {
            block,
            size,
            replaceable: Block::Stone,
        }
    }

    /// RNG order: one `nextFloat` (angle), two `nextInt(3)` (y ends), then one
    /// `nextDouble` per vein step.
    pub fn place(
        &self,
        world: &mut dyn GenWorld,
        random: &mut sunset_1_7_core::LegacyRandom,
        x: i32,
        y: i32,
        z: i32,
    ) -> bool {
        let angle = random.next_float() * std::f32::consts::PI;
        let x_start = f64::from(x) + 8.0 + f64::from(math::sin(angle)) * f64::from(self.size) / 8.0;
        let x_end = f64::from(x) + 8.0 - f64::from(math::sin(angle)) * f64::from(self.size) / 8.0;
        let z_start = f64::from(z) + 8.0 + f64::from(math::cos(angle)) * f64::from(self.size) / 8.0;
        let z_end = f64::from(z) + 8.0 - f64::from(math::cos(angle)) * f64::from(self.size) / 8.0;
        let y_start = f64::from(y + next_int(random, 3) - 2);
        let y_end = f64::from(y + next_int(random, 3) - 2);

        for step in 0..=self.size {
            let px = x_start + (x_end - x_start) * f64::from(step) / f64::from(self.size);
            let py = y_start + (y_end - y_start) * f64::from(step) / f64::from(self.size);
            let pz = z_start + (z_end - z_start) * f64::from(step) / f64::from(self.size);
            let radius = random.next_double() * f64::from(self.size) / 16.0;
            let wave = math::sin((step as f32) * (std::f32::consts::PI / self.size as f32));
            let width = f64::from(wave + 1.0) * radius + 1.0;
            let height = width;

            let min_x = math::floor(px - width / 2.0);
            let min_y = math::floor(py - height / 2.0);
            let min_z = math::floor(pz - width / 2.0);
            let max_x = math::floor(px + width / 2.0);
            let max_y = math::floor(py + height / 2.0);
            let max_z = math::floor(pz + width / 2.0);

            for bx in min_x..=max_x {
                let dx = (f64::from(bx) + 0.5 - px) / (width / 2.0);
                if dx * dx >= 1.0 {
                    continue;
                }
                for by in min_y..=max_y {
                    let dy = (f64::from(by) + 0.5 - py) / (height / 2.0);
                    if dx * dx + dy * dy >= 1.0 {
                        continue;
                    }
                    for bz in min_z..=max_z {
                        let dz = (f64::from(bz) + 0.5 - pz) / (width / 2.0);
                        if dx * dx + dy * dy + dz * dz < 1.0
                            && world.get_block(bx, by, bz) == self.replaceable
                        {
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

/// `BlockVeinFeature` (WorldGenBlockBlob): the taiga's mossy boulders. Three
/// steps of `nextInt(2)` sizes, `nextInt(2)` offsets and a `nextInt(2)` drop.
pub struct BlockVeinFeature {
    pub block: Block,
    pub size: i32,
}

impl BlockVeinFeature {
    pub fn new(block: Block, size: i32) -> BlockVeinFeature {
        BlockVeinFeature { block, size }
    }

    pub fn place(
        &self,
        world: &mut dyn GenWorld,
        random: &mut sunset_1_7_core::LegacyRandom,
        x: i32,
        y: i32,
        z: i32,
    ) -> bool {
        let mut x = x;
        let mut y = y;
        let mut z = z;

        while y > 3 {
            if !world.is_air(x, y - 1, z)
                && matches!(
                    world.get_block(x, y - 1, z),
                    Block::Grass | Block::Dirt | Block::Stone
                )
            {
                break;
            }
            y -= 1;
        }

        if y <= 3 {
            return false;
        }

        let size = self.size;
        // Vanilla's bound is `size >= 0 && i < 3` and `size` never changes,
        // so a non-negative size always runs three steps.
        for _ in 0..3 {
            if size < 0 {
                break;
            }
            let sx = size + next_int(random, 2);
            let sy = size + next_int(random, 2);
            let sz = size + next_int(random, 2);
            let radius = (sx + sy + sz) as f32 * 0.333_f32 + 0.5;

            for bx in (x - sx)..=(x + sx) {
                for bz in (z - sz)..=(z + sz) {
                    for by in (y - sy)..=(y + sy) {
                        let dx = (bx - x) as f32;
                        let dz = (bz - z) as f32;
                        let dy = (by - y) as f32;
                        if dx * dx + dz * dz + dy * dy <= radius * radius {
                            world.set_block_with_metadata(bx, by, bz, self.block, 0, FLAG_SILENT);
                        }
                    }
                }
            }

            x += -(size + 1) + next_int(random, 2 + size * 2);
            z += -(size + 1) + next_int(random, 2 + size * 2);
            y -= next_int(random, 2);
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk_world::ChunkWorld;

    #[test]
    fn replaces_only_stone_and_keeps_draw_count() {
        let mut world = ChunkWorld::new(0, 0);
        world.fill(0, 100, Block::Stone);
        let mut rng = sunset_1_7_core::LegacyRandom::new(42);
        let state = rng.state();
        let vein = VeinFeature::new(Block::CoalOre, 16);
        vein.place(&mut world, &mut rng, 8, 40, 8);
        // Draws: 1 float + 2 nextInt(3) + 17 nextDoubles.
        assert!(world.block_count(Block::CoalOre) > 0);
        assert!(world.block_count(Block::Dirt) == 0);
        // Feed the same draws again: 1 float + 2 ints + 17 doubles.
        let mut reference = sunset_1_7_core::LegacyRandom::new(42);
        let _ = reference.next_float();
        let _ = reference.next_int_bound(3);
        let _ = reference.next_int_bound(3);
        for _ in 0..17 {
            let _ = reference.next_double();
        }
        assert_eq!(rng.state(), reference.state());
        let _ = state;
    }

    #[test]
    fn nothing_outside_replaceable() {
        let mut world = ChunkWorld::new(0, 0);
        world.fill(0, 100, Block::Dirt);
        let mut rng = sunset_1_7_core::LegacyRandom::new(1);
        let vein = VeinFeature::new(Block::DiamondOre, 7);
        vein.place(&mut world, &mut rng, 8, 40, 8);
        assert_eq!(world.block_count(Block::DiamondOre), 0);
    }

    #[test]
    fn the_mossy_boulder_runs_three_steps_and_keeps_its_draws() {
        let mut world = ChunkWorld::new(0, 0);
        world.fill(0, 100, Block::Stone);
        let mut rng = sunset_1_7_core::LegacyRandom::new(17);
        let boulder = BlockVeinFeature::new(Block::MossyCobblestone, 0);
        assert!(boulder.place(&mut world, &mut rng, 8, 60, 8));
        assert!(world.block_count(Block::MossyCobblestone) > 0);

        // Three steps of three sizes, two offsets and one drop.
        let mut reference = sunset_1_7_core::LegacyRandom::new(17);
        for _ in 0..3 {
            for _ in 0..6 {
                let _ = reference.next_int_bound(2);
            }
        }
        assert_eq!(rng.state(), reference.state());
    }
}
