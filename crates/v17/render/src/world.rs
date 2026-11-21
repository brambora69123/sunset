//! The render-side world view, ported from `net.minecraft.world.WorldView`
//! and the light sampling of `net.minecraft.world.World`: the raw
//! `getLight` levels, `getBrightness`'s `usesNeighborLight` smoothing and
//! `getLightColor`'s sky/block packing.

use crate::block::BlockDef;

/// `WorldView`: what the block renderer reads from the world.
pub trait WorldView {
    /// `getBlock`; outside the world this is air, like vanilla.
    fn block(&self, x: i32, y: i32, z: i32) -> &BlockDef;

    /// `getBlockMetadata`.
    fn metadata(&self, x: i32, y: i32, z: i32) -> i32;

    /// `World.getLight(LightType.SKY, x, y, z)`: the raw 0..15 level, with
    /// `y < 0` clamped to 0. `getBrightness`'s smoothing lives in
    /// [`WorldView::light_color`].
    fn sky_light(&self, x: i32, y: i32, z: i32) -> i32;

    /// `World.getLight(LightType.BLOCK, x, y, z)`: the raw 0..15 level.
    fn block_light(&self, x: i32, y: i32, z: i32) -> i32;

    /// `getBiome(x, z).getGrassColor(x, z)` for grass tinting.
    fn biome_grass_tint(&self, x: i32, z: i32) -> i32;

    /// `World.getLightColor(x, y, z, blockLight)`: `sky << 20 | max(block,
    /// blockLight) << 4`, with `World.getBrightness`'s neighbour-light
    /// smoothing for blocks that `usesNeighborLight` (stairs, slabs, plants,
    /// liquids and anything with opacity 0).
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

/// One 3D byte column lookup shared by the sample world.
pub fn sample(grid: &[u8], size: i32, x: i32, y: i32, z: i32) -> u8 {
    if x < 0 || y < 0 || z < 0 || x >= size || y >= size || z >= size {
        return 0;
    }
    grid[((y * size + z) * size + x) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_color_packs_sky_and_block() {
        let mut world = crate::fixture::TestWorld::new();
        world.set_sky_light(0, 0, 0, 15);
        world.set_block_light(0, 0, 0, 5);
        // sky 15 << 20 | block 5 << 4
        assert_eq!(world.light_color(0, 0, 0, 0), 15 << 20 | 5 << 4);
        // blockLight raises the block part when the stored light is darker
        assert_eq!(world.light_color(0, 0, 0, 9), 15 << 20 | 9 << 4);
    }

    #[test]
    fn sample_out_of_grid_is_zero() {
        let grid = vec![1u8; 27];
        assert_eq!(sample(&grid, 3, 0, 0, 0), 1);
        assert_eq!(sample(&grid, 3, -1, 0, 0), 0);
        assert_eq!(sample(&grid, 3, 3, 0, 0), 0);
    }
}
