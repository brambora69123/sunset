//! `NoiseGeneratorImproved`, 1.7.10: Ken Perlin's improved noise on a 256-entry
//! permutation table, with the 2D (single-y-layer) fast path `add` takes when
//! `sizeY == 1`.

use crate::{Noise, init_permutation};
use sunset_1_7_core::LegacyRandom;

/// `ImprovedNoise`: one improved-noise level with its permutation table and
/// the three `nextDouble() * 256.0` offsets.
pub struct ImprovedNoise {
    permutations: [i32; 512],
    pub offset_x: f64,
    pub offset_y: f64,
    pub offset_z: f64,
}

impl Noise for ImprovedNoise {}

/// `f_56556676` — identical to `f_57766507`, the 2D dot's x table.
const GRADIENT_X: [f64; 16] = [
    1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, -1.0, 0.0,
];
/// `f_44859570`.
const GRADIENT_Y: [f64; 16] = [
    1.0, 1.0, -1.0, -1.0, 0.0, 0.0, 0.0, 0.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0,
];
/// `f_78261736` — identical to `f_05584602`, the 2D dot's z table.
const GRADIENT_Z: [f64; 16] = [
    0.0, 0.0, 0.0, 0.0, 1.0, 1.0, -1.0, -1.0, 1.0, 1.0, -1.0, -1.0, 0.0, 1.0, 0.0, -1.0,
];

impl ImprovedNoise {
    pub fn new(random: &mut LegacyRandom) -> Self {
        let (offsets, permutations) = init_permutation(random);
        Self {
            permutations,
            offset_x: offsets[0],
            offset_y: offsets[1],
            offset_z: offsets[2],
        }
    }

    /// `lerp`: linear interpolation toward the max.
    pub fn lerp(delta: f64, min: f64, max: f64) -> f64 {
        min + delta * (max - min)
    }

    /// The `x * 6 - 15` fade curve applied to each fractional coordinate.
    fn fade(fraction: f64) -> f64 {
        fraction * fraction * fraction * (fraction * (fraction * 6.0 - 15.0) + 10.0)
    }

    /// `gradientDot(hash, x, z)`: the 2D dot, through `f_57766507`/`f_05584602`.
    pub fn gradient_dot_2d(hash: i32, x: f64, z: f64) -> f64 {
        let index = (hash & 15) as usize;
        GRADIENT_X[index] * x + GRADIENT_Z[index] * z
    }

    /// `gradientDot(hash, x, y, z)`.
    pub fn gradient_dot_3d(hash: i32, x: f64, y: f64, z: f64) -> f64 {
        let index = (hash & 15) as usize;
        GRADIENT_X[index] * x + GRADIENT_Y[index] * y + GRADIENT_Z[index] * z
    }

    /// `add`: accumulate this level's noise into `values`, one entry per cell,
    /// scaled by `1 / noiseScale` as in 1.7.10. The `(int)` cast truncates
    /// toward zero and the `& 0xFF` wraps negatives, both as Java does. The
    /// 3D path's corner hashes and dots are declared dead until their first
    /// recompute, as the Java locals are.
    #[allow(unused_assignments)]
    pub fn add(
        &self,
        values: &mut [f64],
        x: f64,
        y: f64,
        z: f64,
        size_x: i32,
        size_y: i32,
        size_z: i32,
        scale_x: f64,
        scale_y: f64,
        scale_z: f64,
        noise_scale: f64,
    ) {
        let reciprocal = 1.0 / noise_scale;
        if size_y == 1 {
            // The 2D path: one y layer, hashed through y = 0. The mixed 2D/3D
            // gradient dots read the same tables, so both give one dot.
            let mut index = 0;
            for cell_x in 0..size_x {
                let mut local_x = x + cell_x as f64 * scale_x + self.offset_x;
                let mut cell = local_x as i32;
                if local_x < cell as f64 {
                    cell -= 1;
                }
                let hash_x = (cell & 0xFF) as usize;
                local_x -= cell as f64;
                let fade_x = Self::fade(local_x);

                for cell_z in 0..size_z {
                    let mut local_z = z + cell_z as f64 * scale_z + self.offset_z;
                    let mut cell = local_z as i32;
                    if local_z < cell as f64 {
                        cell -= 1;
                    }
                    let hash_z = (cell & 0xFF) as usize;
                    local_z -= cell as f64;
                    let fade_z = Self::fade(local_z);

                    let corner_y = self.permutations[hash_x]; // hashed with y = 0
                    let hash_a = self.permutations[corner_y as usize] + hash_z as i32;
                    let corner_y = self.permutations[hash_x + 1];
                    let hash_c = self.permutations[corner_y as usize] + hash_z as i32;
                    let low = Self::lerp(
                        fade_x,
                        Self::gradient_dot_2d(self.permutations[hash_a as usize], local_x, local_z),
                        Self::gradient_dot_3d(
                            self.permutations[hash_c as usize],
                            local_x - 1.0,
                            0.0,
                            local_z,
                        ),
                    );
                    let high = Self::lerp(
                        fade_x,
                        Self::gradient_dot_3d(
                            self.permutations[(hash_a + 1) as usize],
                            local_x,
                            0.0,
                            local_z - 1.0,
                        ),
                        Self::gradient_dot_3d(
                            self.permutations[(hash_c + 1) as usize],
                            local_x - 1.0,
                            0.0,
                            local_z - 1.0,
                        ),
                    );
                    values[index] += Self::lerp(fade_z, low, high) * reciprocal;
                    index += 1;
                }
            }
        } else {
            // Corner hashes are per-x-column; the y layer only moves them, so
            // 1.7.10 recomputes them when the layer changes.
            let mut index = 0;
            for cell_x in 0..size_x {
                let mut local_x = x + cell_x as f64 * scale_x + self.offset_x;
                let mut cell = local_x as i32;
                if local_x < cell as f64 {
                    cell -= 1;
                }
                let hash_x = (cell & 0xFF) as usize;
                local_x -= cell as f64;
                let fade_x = Self::fade(local_x);

                for cell_z in 0..size_z {
                    let mut local_z = z + cell_z as f64 * scale_z + self.offset_z;
                    let mut cell = local_z as i32;
                    if local_z < cell as f64 {
                        cell -= 1;
                    }
                    let hash_z = (cell & 0xFF) as usize;
                    local_z -= cell as f64;
                    let fade_z = Self::fade(local_z);

                    let mut cached_y = -1;
                    let mut hash_a = 0;
                    let mut hash_b = 0;
                    let mut hash_c = 0;
                    let mut hash_d = 0;
                    let mut dot_x = 0.0;
                    let mut dot_xy = 0.0;
                    let mut dot_xz = 0.0;
                    let mut dot_xyz = 0.0;

                    for cell_y in 0..size_y {
                        let mut local_y = y + cell_y as f64 * scale_y + self.offset_y;
                        let mut cell = local_y as i32;
                        if local_y < cell as f64 {
                            cell -= 1;
                        }
                        let hash_y = (cell & 0xFF) as usize;
                        local_y -= cell as f64;
                        let fade_y = Self::fade(local_y);

                        if cell_y == 0 || hash_y as i32 != cached_y {
                            cached_y = hash_y as i32;
                            let corner_low_x = self.permutations[hash_x] + hash_y as i32;
                            let corner_high_x = self.permutations[hash_x + 1] + hash_y as i32;
                            hash_a = self.permutations[corner_low_x as usize] + hash_z as i32;
                            hash_b = self.permutations[(corner_low_x + 1) as usize] + hash_z as i32;
                            hash_c = self.permutations[corner_high_x as usize] + hash_z as i32;
                            hash_d =
                                self.permutations[(corner_high_x + 1) as usize] + hash_z as i32;
                            dot_x = Self::lerp(
                                fade_x,
                                Self::gradient_dot_3d(
                                    self.permutations[hash_a as usize],
                                    local_x,
                                    local_y,
                                    local_z,
                                ),
                                Self::gradient_dot_3d(
                                    self.permutations[hash_c as usize],
                                    local_x - 1.0,
                                    local_y,
                                    local_z,
                                ),
                            );
                            dot_xy = Self::lerp(
                                fade_x,
                                Self::gradient_dot_3d(
                                    self.permutations[hash_b as usize],
                                    local_x,
                                    local_y - 1.0,
                                    local_z,
                                ),
                                Self::gradient_dot_3d(
                                    self.permutations[hash_d as usize],
                                    local_x - 1.0,
                                    local_y - 1.0,
                                    local_z,
                                ),
                            );
                            dot_xz = Self::lerp(
                                fade_x,
                                Self::gradient_dot_3d(
                                    self.permutations[(hash_a + 1) as usize],
                                    local_x,
                                    local_y,
                                    local_z - 1.0,
                                ),
                                Self::gradient_dot_3d(
                                    self.permutations[(hash_c + 1) as usize],
                                    local_x - 1.0,
                                    local_y,
                                    local_z - 1.0,
                                ),
                            );
                            dot_xyz = Self::lerp(
                                fade_x,
                                Self::gradient_dot_3d(
                                    self.permutations[(hash_b + 1) as usize],
                                    local_x,
                                    local_y - 1.0,
                                    local_z - 1.0,
                                ),
                                Self::gradient_dot_3d(
                                    self.permutations[(hash_d + 1) as usize],
                                    local_x - 1.0,
                                    local_y - 1.0,
                                    local_z - 1.0,
                                ),
                            );
                        }

                        let low_y = Self::lerp(fade_y, dot_x, dot_xy);
                        let high_y = Self::lerp(fade_y, dot_xz, dot_xyz);
                        values[index] += Self::lerp(fade_z, low_y, high_y) * reciprocal;
                        index += 1;
                    }
                }
            }
        }
    }
}
