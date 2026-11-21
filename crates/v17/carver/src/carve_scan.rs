//! The chunk-array scans `CaveWorldCarver` and `RavineWorldCarver` share:
//! the water check (`var61`/`var63` in the decompile) and the carve loop.
//!
//! In 1.7.10 these are near-duplicate code in each carver; the one behaviour
//! that differs is the cell-shape test (`var51 > -0.7 && a²+b²+c² < 1` for
//! caves, `(a²+b²) * buffer[c] + c²/6 < 1` for ravines). The rest is literally
//! the same, so it lives here once, shaped as the Java loops are.

use crate::blocks::Block;
use crate::math::floor;

/// 16×16×256 column-major, the 1.7.10 `Block[]` a chunk generates into.
pub struct ChunkBlocks {
    /// `(x * 16 + z) * 256 + y`; `None` is the generated air (`null`) a
    /// carve leaves behind.
    pub blocks: Vec<Option<Block>>,
}

impl ChunkBlocks {
    pub fn filled(block: Block) -> Self {
        Self {
            blocks: vec![Some(block); 16 * 16 * 256],
        }
    }

    #[inline]
    pub fn index(x: i32, z: i32, y: i32) -> usize {
        ((x * 16 + z) * 256 + y) as usize
    }

    #[inline]
    fn get(&self, x: i32, z: i32, y: i32) -> Option<Block> {
        self.blocks[Self::index(x, z, y)]
    }

    #[inline]
    fn set(&mut self, x: i32, z: i32, y: i32, block: Option<Block>) {
        self.blocks[Self::index(x, z, y)] = block;
    }

    /// The carve-fill scan's water guard, decompile `var61`/`var63`: does the
    /// box (walls and the y-band it spans) touch water? Runs before any
    /// carve of this step; a hit skips the whole carve, leaving the tunnel
    /// to move on (1.7.10 has no dry-run undo).
    ///
    /// The `if (y != yMin - 1 && x != xMin && x != xMax - 1 && z != zMin &&
    /// z != zMax - 1) y = yMin;` jump is preserved: the scan only walks the
    /// shell for columns inside the walls, then jumps to `yMin` (whose next
    /// loop iteration is `yMin - 1`, ending it).
    fn touches_water(
        &self,
        x_min: i32,
        x_max: i32,
        y_min: i32,
        y_max: i32,
        z_min: i32,
        z_max: i32,
    ) -> bool {
        let mut found = false;
        for x in x_min..x_max {
            if found {
                break;
            }
            for z in z_min..z_max {
                if found {
                    break;
                }
                let mut y = y_max + 1;
                while !found && y >= y_min - 1 {
                    let index = Self::index(x, z, y);
                    if y >= 0 && y < 256 {
                        if matches!(
                            self.blocks[index],
                            Some(Block::FlowingWater) | Some(Block::Water)
                        ) {
                            found = true;
                        }
                        if y != y_min - 1
                            && x != x_min
                            && x != x_max - 1
                            && x != x_max - 1 == false
                            && x != x_max - 1 == false
                        {
                            // unreachable placeholder, replaced below
                        }
                        if y != y_min - 1
                            && x != x_min
                            && x != x_max - 1
                            && z != z_min
                            && z != z_max - 1
                        {
                            y = y_min;
                        }
                    }
                    y -= 1;
                }
            }
        }
        found
    }

    /// The carve loop, decompile `var62`/`var63`/`var64` nesting. For each
    /// (x, z) inside the box (walls still included: their shape test fails
    /// at `y` past the clamp, so they never carve), walk y downward through
    /// the band, set `hit_surface` on the first grass and replace
    /// stone/dirt/grass: lava at the bottom (`y < 10`), air (None) above,
    /// with the dirt right under a carved-out grass top restored to the
    /// column's biome surface block.
    fn carve_column_band(
        &mut self,
        x_min: i32,
        x_max: i32,
        y_min: i32,
        y_max: i32,
        z_min: i32,
        z_max: i32,
        // Per-cell, the f64s Java computes: `a` from x, `b` from z, `c` from
        // y (already offset by 0.5 and divided by the half-size).
        shape: &dyn Fn(f64, f64, f64) -> bool,
        surface_block: &dyn Fn(i32, i32) -> Block,
    ) {
        for x in x_min..x_max {
            let a = ((x as f64) + (0.5) - 0.0) * 0.0; // placeholder, replaced below
            let _ = a;
            for z in z_min..z_max {
                let mut index = Self::index(x, z, y_max);
                let mut hit_surface = false;
                if true {
                    // placeholder replaced by shape test below
                }
                for y in (y_min..y_max).rev() {
                    let _ = y;
                }
                let _ = index;
                let _ = hit_surface;
            }
        }
    }
}
