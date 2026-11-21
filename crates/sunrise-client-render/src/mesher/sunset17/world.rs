//! The render-side world view, ported from `net.minecraft.world.WorldView`
//! and the light sampling of `net.minecraft.world.World`: the raw
//! `getLight` levels, `getBrightness`'s `usesNeighborLight` smoothing and
//! `getLightColor`'s sky/block packing.

use super::State17;
use super::blocks::{BlockDef, Tint};

/// `WorldView`: what the block renderer reads from the world.
pub trait WorldView {
    /// `getBlock` and `getBlockMetadata`: the 1.7.10 state at a position. Out
    /// of the world this is the air state (state 0).
    fn state(&self, x: i32, y: i32, z: i32) -> &State17;

    /// `getBlock`: the block facts of the state at a position.
    fn block(&self, x: i32, y: i32, z: i32) -> &BlockDef {
        &self.state(x, y, z).block
    }

    /// `getBlockMetadata`.
    fn metadata(&self, x: i32, y: i32, z: i32) -> i32 {
        i32::from(self.state(x, y, z).meta)
    }

    /// `World.getLight(LightType.SKY, x, y, z)`: the raw 0..15 level. 1.7.10
    /// clamps `y` into the column (`y < 0` is the bottom section, `y >= 256`
    /// the top one), which the caller's world data already does.
    fn sky_light(&self, x: i32, y: i32, z: i32) -> i32;

    /// `World.getLight(LightType.BLOCK, x, y, z)`: the raw 0..15 level.
    fn block_light(&self, x: i32, y: i32, z: i32) -> i32;

    /// The era's `Biome.getGrassColor`/`getFoliageColor` at a block: the 3x3
    /// average of the biome colours of the nine columns around it.
    fn biome_tint(&self, tint: Tint, x: i32, y: i32, z: i32) -> i32;

    /// `World.getLightColor(x, y, z, blockLight)`: `sky << 20 | max(block,
    /// blockLight) << 4`, with `World.getBrightness`'s neighbour-light
    /// smoothing for blocks that `usesNeighborLight` (stairs, slabs, plants,
    /// liquids and anything of opacity 0).
    fn light_color(&self, x: i32, y: i32, z: i32, block_light: i32) -> i32 {
        let sky = self.brightness(true, x, y, z);
        let mut block = self.brightness(false, x, y, z);
        if block < block_light {
            block = block_light;
        }
        sky << 20 | block << 4
    }

    /// `World.getBrightness(LightType, x, y, z)`, including the
    /// `usesNeighborLight` maximum over the block above and the four
    /// horizontal neighbours.
    fn brightness(&self, sky: bool, x: i32, y: i32, z: i32) -> i32 {
        let y = if y < 0 { 0 } else { y };
        if y >= 256 {
            return if sky { 15 } else { 0 };
        }
        if !self.block(x, y, z).use_neighbor_light {
            return if sky {
                self.sky_light(x, y, z)
            } else {
                self.block_light(x, y, z)
            };
        }
        let sample = |x: i32, y: i32, z: i32| {
            if sky {
                self.sky_light(x, y, z)
            } else {
                self.block_light(x, y, z)
            }
        };
        let mut level = sample(x, y + 1, z);
        for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let neighbor = sample(x + dx, y, z + dz);
            if neighbor > level {
                level = neighbor;
            }
        }
        level
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesher::sunset17::fixture::TestWorld;

    #[test]
    fn light_color_packs_sky_and_block() {
        let mut world = TestWorld::new();
        world.set_sky_light(0, 0, 0, 15);
        world.set_block_light(0, 0, 0, 5);
        // sky 15 << 20 | block 5 << 4
        assert_eq!(world.light_color(0, 0, 0, 0), 15 << 20 | 5 << 4);
        // blockLight raises the block part when the stored light is darker
        assert_eq!(world.light_color(0, 0, 0, 9), 15 << 20 | 9 << 4);
    }

    #[test]
    fn the_brightness_smoother_reads_the_block_above_and_the_sides() {
        let mut world = TestWorld::new();
        for y in 0..16 {
            for z in 0..16 {
                for x in 0..16 {
                    world.set_sky_light(x, y, z, 0);
                }
            }
        }
        // Tall grass smooths its light: the brightest of the block above and
        // the four horizontal neighbours wins.
        world.set(
            1,
            2,
            2,
            crate::mesher::sunset17::fixture::id("tallgrass") as u8,
            1,
        );
        world.set_sky_light(1, 3, 2, 4);
        world.set_sky_light(2, 2, 2, 9);
        assert_eq!(world.brightness(true, 1, 2, 2), 9);
        // Stone does not: it reads its own cell.
        world.set(
            3,
            2,
            2,
            crate::mesher::sunset17::fixture::id("stone") as u8,
            0,
        );
        world.set_sky_light(3, 2, 2, 3);
        world.set_sky_light(4, 2, 2, 12);
        assert_eq!(world.brightness(true, 3, 2, 2), 3);
    }
}
