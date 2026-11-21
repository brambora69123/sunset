//! The chunk-array scan `CaveWorldCarver` and `RavineWorldCarver` share:
//! the index math and the pre-carve water check (`var61`/`var63` in the
//! decompile). In 1.7.10 it is near-duplicate code in each carver; the one
//! behaviour that differs is the cell-shape test inside the carve loop,
//! which stays per-carver in [`crate::cave`] and [`crate::ravine`].

use crate::block_id;

/// `(x * 16 + z) * 256 + y`, the 1.7.10 chunk array index over
/// `[u16; 16 * 16 * 256]` with chunk-local `x`, `z` in `0..16`.
#[inline]
pub fn index(x: i32, z: i32, y: i32) -> usize {
    ((x * 16 + z) * 256 + y) as usize
}

/// The water guard the carve-fill scan runs before any carve of a step: does
/// the box (walls and the y-band it spans) touch water? A hit skips the whole
/// carve for this step; 1.7.10 has no dry-run undo, the tunnel just moves on.
///
/// The decompile's `if (y != yMin - 1 && x != xMin && x != xMax - 1 &&
/// z != zMin && z != zMax - 1) y = yMin;` jump is preserved: a column inside
/// the walls only checks the top of the band, then jumps to `yMin` (whose
/// next iteration is `yMin - 1`, ending the loop). So water in the middle of
/// the band under an interior column is *not* seen — vanilla behaviour.
pub fn touches_water(
    blocks: &[u16],
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
                if (0..256).contains(&y) {
                    let block = blocks[index(x, z, y)];
                    if block == block_id::FLOWING_WATER || block == block_id::WATER {
                        found = true;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn stone() -> Vec<u16> {
        vec![block_id::STONE; 16 * 16 * 256]
    }

    #[test]
    fn index_is_column_major() {
        assert_eq!(index(0, 0, 0), 0);
        assert_eq!(index(0, 0, 1), 1);
        assert_eq!(index(0, 1, 0), 256);
        assert_eq!(index(1, 0, 0), 4096);
    }

    #[test]
    fn water_in_band_is_found() {
        // A wall column scans every y of the band.
        let mut blocks = stone();
        blocks[index(5, 7, 64)] = block_id::WATER;
        assert!(touches_water(&blocks, 5, 12, 1, 100, 5, 12));
        let mut flowing = stone();
        flowing[index(5, 7, 64)] = block_id::FLOWING_WATER;
        assert!(touches_water(&flowing, 5, 12, 1, 100, 5, 12));
    }

    #[test]
    fn water_outside_the_box_is_not() {
        let mut blocks = stone();
        blocks[index(0, 0, 200)] = block_id::WATER;
        assert!(!touches_water(&blocks, 5, 12, 1, 100, 5, 12));
    }

    #[test]
    fn interior_columns_skip_the_middle_of_the_band() {
        // The shell jump: an interior column checks the top and `yMin - 1`
        // only, so water mid-band under it is missed. Vanilla behaviour.
        let mut blocks = stone();
        blocks[index(7, 7, 5)] = block_id::WATER;
        assert!(!touches_water(&blocks, 5, 12, 1, 10, 5, 12));
        // The bottom of the band (`yMin - 1`) is still checked.
        let mut bottom = stone();
        bottom[index(7, 7, 0)] = block_id::WATER;
        assert!(touches_water(&bottom, 5, 12, 1, 10, 5, 12));
        // And a wall column scans every y.
        let mut wall = stone();
        wall[index(5, 7, 5)] = block_id::WATER;
        assert!(touches_water(&wall, 5, 12, 1, 10, 5, 12));
    }
}
