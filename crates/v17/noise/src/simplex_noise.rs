//! `NoiseGeneratorSimplex`, 1.7.10: Stefan Gustavson's 2D simplex noise on a
//! 256-entry permutation table and the 12 2D gradients.

use crate::init_permutation;
use sunset_1_7_core::LegacyRandom;

/// `SimplexNoise`: one simplex-noise level with its permutation table and
/// offsets.
pub struct SimplexNoise {
    permutations: [i32; 512],
    pub offset_x: f64,
    pub offset_y: f64,
    pub offset_z: f64,
}

/// The 12 gradients, each a 3D vector with one zero component.
const GRADIENTS: [[i32; 3]; 12] = [
    [1, 1, 0],
    [-1, 1, 0],
    [1, -1, 0],
    [-1, -1, 0],
    [1, 0, 1],
    [-1, 0, 1],
    [1, 0, -1],
    [-1, 0, -1],
    [0, 1, 1],
    [0, -1, 1],
    [0, 1, -1],
    [0, -1, -1],
];

/// `Math.sqrt(3.0)`, correctly rounded; `f64::sqrt` is not const.
const SQRT_THREE: f64 = 1.7320508075688772;
/// `SKEW_FACTOR_2D`.
const SKEW_FACTOR_2D: f64 = 0.5 * (SQRT_THREE - 1.0);
/// `UNSKEW_FACTOR_2D`.
const UNSKEW_FACTOR_2D: f64 = (3.0 - SQRT_THREE) / 6.0;

/// `fastFloor`: truncation, then a decrement for non-positive values —
/// note the 0.0 case yields -1, as in 1.7.10.
fn fast_floor(value: f64) -> i32 {
    if value > 0.0 {
        value as i32
    } else {
        value as i32 - 1
    }
}

/// The gradient dot against the gradient's x and y components.
fn dot(gradient: &[i32; 3], x: f64, y: f64) -> f64 {
    gradient[0] as f64 * x + gradient[1] as f64 * y
}

impl SimplexNoise {
    pub fn new(random: &mut LegacyRandom) -> Self {
        let (offsets, permutations) = init_permutation(random);
        Self {
            permutations,
            offset_x: offsets[0],
            offset_y: offsets[1],
            offset_z: offsets[2],
        }
    }

    /// `getValue`: one 2D simplex sample in -1..1.
    pub fn get_value(&self, x: f64, y: f64) -> f64 {
        let skew_offset = (x + y) * SKEW_FACTOR_2D;
        let cell_x = fast_floor(x + skew_offset);
        let cell_y = fast_floor(y + skew_offset);
        let unskew_offset = (cell_x + cell_y) as f64 * UNSKEW_FACTOR_2D;
        let corner_x = x - (cell_x as f64 - unskew_offset);
        let corner_y = y - (cell_y as f64 - unskew_offset);
        let (offset_x, offset_y): (i32, i32) = if corner_x > corner_y { (1, 0) } else { (0, 1) };
        let far_x = corner_x - offset_x as f64 + UNSKEW_FACTOR_2D;
        let far_y = corner_y - offset_y as f64 + UNSKEW_FACTOR_2D;
        let last_x = corner_x - 1.0 + 2.0 * UNSKEW_FACTOR_2D;
        let last_y = corner_y - 1.0 + 2.0 * UNSKEW_FACTOR_2D;
        let index_x = (cell_x & 0xFF) as usize;
        let index_y = (cell_y & 0xFF) as usize;
        let gradient_0 =
            (self.permutations[index_x + self.permutations[index_y] as usize] % 12) as usize;
        let gradient_1 = (self.permutations
            [index_x + offset_x as usize + self.permutations[index_y + offset_y as usize] as usize]
            % 12) as usize;
        let gradient_2 = (self.permutations[index_x + 1 + self.permutations[index_y + 1] as usize]
            % 12) as usize;

        let mut weight_0 = 0.5 - corner_x * corner_x - corner_y * corner_y;
        let corner_0 = if weight_0 < 0.0 {
            0.0
        } else {
            weight_0 *= weight_0;
            weight_0 * weight_0 * dot(&GRADIENTS[gradient_0], corner_x, corner_y)
        };
        let mut weight_1 = 0.5 - far_x * far_x - far_y * far_y;
        let corner_1 = if weight_1 < 0.0 {
            0.0
        } else {
            weight_1 *= weight_1;
            weight_1 * weight_1 * dot(&GRADIENTS[gradient_1], far_x, far_y)
        };
        let mut weight_2 = 0.5 - last_x * last_x - last_y * last_y;
        let corner_2 = if weight_2 < 0.0 {
            0.0
        } else {
            weight_2 *= weight_2;
            weight_2 * weight_2 * dot(&GRADIENTS[gradient_2], last_x, last_y)
        };
        70.0 * (corner_0 + corner_1 + corner_2)
    }

    /// `add`: accumulate into `values`, one entry per cell, multiplied by
    /// `noiseScale`.
    pub fn add(
        &self,
        values: &mut [f64],
        x: f64,
        y: f64,
        size_x: i32,
        size_y: i32,
        scale_x: f64,
        scale_y: f64,
        noise_scale: f64,
    ) {
        let mut index = 0;
        for cell_y in 0..size_y {
            let sample_y = (y + cell_y as f64) * scale_y + self.offset_y;
            for cell_x in 0..size_x {
                let sample_x = (x + cell_x as f64) * scale_x + self.offset_x;
                let skew_offset = (sample_x + sample_y) * SKEW_FACTOR_2D;
                let grid_x = fast_floor(sample_x + skew_offset);
                let grid_y = fast_floor(sample_y + skew_offset);
                let unskew_offset = (grid_x + grid_y) as f64 * UNSKEW_FACTOR_2D;
                let corner_x = sample_x - (grid_x as f64 - unskew_offset);
                let corner_y = sample_y - (grid_y as f64 - unskew_offset);
                let (offset_x, offset_y): (i32, i32) =
                    if corner_x > corner_y { (1, 0) } else { (0, 1) };
                let far_x = corner_x - offset_x as f64 + UNSKEW_FACTOR_2D;
                let far_y = corner_y - offset_y as f64 + UNSKEW_FACTOR_2D;
                let last_x = corner_x - 1.0 + 2.0 * UNSKEW_FACTOR_2D;
                let last_y = corner_y - 1.0 + 2.0 * UNSKEW_FACTOR_2D;
                let index_x = (grid_x & 0xFF) as usize;
                let index_y = (grid_y & 0xFF) as usize;
                let gradient_0 = (self.permutations[index_x + self.permutations[index_y] as usize]
                    % 12) as usize;
                let gradient_1 = (self.permutations[index_x
                    + offset_x as usize
                    + self.permutations[index_y + offset_y as usize] as usize]
                    % 12) as usize;
                let gradient_2 = (self.permutations
                    [index_x + 1 + self.permutations[index_y + 1] as usize]
                    % 12) as usize;

                let mut weight_0 = 0.5 - corner_x * corner_x - corner_y * corner_y;
                let corner_0 = if weight_0 < 0.0 {
                    0.0
                } else {
                    weight_0 *= weight_0;
                    weight_0 * weight_0 * dot(&GRADIENTS[gradient_0], corner_x, corner_y)
                };
                let mut weight_1 = 0.5 - far_x * far_x - far_y * far_y;
                let corner_1 = if weight_1 < 0.0 {
                    0.0
                } else {
                    weight_1 *= weight_1;
                    weight_1 * weight_1 * dot(&GRADIENTS[gradient_1], far_x, far_y)
                };
                let mut weight_2 = 0.5 - last_x * last_x - last_y * last_y;
                let corner_2 = if weight_2 < 0.0 {
                    0.0
                } else {
                    weight_2 *= weight_2;
                    weight_2 * weight_2 * dot(&GRADIENTS[gradient_2], last_x, last_y)
                };
                values[index] += 70.0 * (corner_0 + corner_1 + corner_2) * noise_scale;
                index += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SimplexNoise;
    use crate::init_permutation;
    use sunset_1_7_core::{LegacyRandom, RandomSource};

    #[test]
    fn permutation_is_a_permutation_of_256() {
        let noise = SimplexNoise::new(&mut LegacyRandom::new(987654321));
        let mut seen = [false; 256];
        for entry in &noise.permutations[..256] {
            assert!((0..256).contains(entry), "entry {entry} out of range");
            assert!(!seen[*entry as usize], "entry {entry} drawn twice");
            seen[*entry as usize] = true;
        }
        // The doubled tail follows the head on every entry.
        for i in 0..256 {
            assert_eq!(
                noise.permutations[i],
                noise.permutations[i + 256],
                "entry {i}"
            );
        }
    }

    #[test]
    fn permutation_draws_match_java() {
        // ImprovedNoise(12345): three nextDouble() * 256 offsets, then the
        // Fisher-Yates shuffle with nextIntBound(256 - i) + i.
        let mut random = LegacyRandom::new(12345);
        let offsets: Vec<f64> = (0..3).map(|_| random.next_double() * 256.0).collect();
        let mut expected = [0i32; 256];
        for (i, slot) in expected.iter_mut().enumerate() {
            *slot = i as i32;
        }
        for i in 0..256i32 {
            let swap = random.next_int_bound(256 - i).expect("positive") + i;
            expected.swap(i as usize, swap as usize);
        }
        let (noise_offsets, permutations) = init_permutation(&mut LegacyRandom::new(12345));
        assert_eq!(noise_offsets, [offsets[0], offsets[1], offsets[2]]);
        for i in 0..256 {
            assert_eq!(permutations[i], expected[i], "entry {i}");
            assert_eq!(permutations[i + 256], expected[i], "tail of entry {i}");
        }
    }
}
