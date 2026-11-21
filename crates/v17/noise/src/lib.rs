//! Minecraft 1.7.10 noise, ported for sunset: the generators under
//! `world/gen/noise` and their `getRegion` sampling APIs as
//! `OverworldChunkGenerator` calls them.

mod improved_noise;
mod perlin_noise;
mod perlin_simplex_noise;
mod simplex_noise;

pub use improved_noise::ImprovedNoise;
pub use perlin_noise::PerlinNoise;
pub use perlin_simplex_noise::PerlinSimplexNoise;
pub use simplex_noise::SimplexNoise;

use sunset_1_7_core::{LegacyRandom, RandomSource};

#[cfg(test)]
mod tests;

/// `Noise`, 1.7.10's empty abstract base: no behavior of its own, kept as a
/// marker so the generator hierarchy reads like the Java side.
pub trait Noise {}

/// `MathHelper.lfloor`: floor as a long, truncation then a decrement below
/// zero. The `(long)` cast saturates on overflow in both Java and Rust.
pub fn lfloor(value: f64) -> i64 {
    let truncated = value as i64;
    if value >= truncated as f64 {
        truncated
    } else {
        truncated - 1
    }
}

/// The table both `ImprovedNoise` and `SimplexNoise` build from a
/// `java.util.Random`: three `nextDouble() * 256.0` offsets, then a
/// Fisher-Yates shuffle of 0..256, duplicated into 512 entries.
pub(crate) fn init_permutation(random: &mut LegacyRandom) -> ([f64; 3], [i32; 512]) {
    let offsets = [
        random.next_double() * 256.0,
        random.next_double() * 256.0,
        random.next_double() * 256.0,
    ];
    let mut permutations = [0i32; 512];
    for (i, slot) in permutations.iter_mut().enumerate().take(256) {
        *slot = i as i32;
    }
    for i in 0..256i32 {
        let swap = random.next_int_bound(256 - i).expect("256 - i is positive") + i;
        permutations.swap(i as usize, swap as usize);
        permutations[i as usize + 256] = permutations[i as usize];
    }
    (offsets, permutations)
}

/// Region buffer as the Java callers keep one: grown to hold `needed` values,
/// then zeroed whole, so reused buffers never leak a previous region.
pub(crate) fn prepare_region(values: &mut Vec<f64>, needed: usize) {
    if values.len() < needed {
        values.resize(needed, 0.0);
    }
    for value in values.iter_mut() {
        *value = 0.0;
    }
}
