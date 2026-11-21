//! `NoiseGeneratorOctaves`, 1.7.10: a stack of `ImprovedNoise` levels summed
//! octave by octave, with the coordinate wrap that keeps the fractional part
//! and a 2^24-wrapped integer in the same double.

use crate::{ImprovedNoise, Noise, lfloor, prepare_region};
use sunset_1_7_core::LegacyRandom;

/// `PerlinNoise`: `levels` improved-noise levels drawn from one stream.
pub struct PerlinNoise {
    noise_levels: Vec<ImprovedNoise>,
}

impl Noise for PerlinNoise {}

impl PerlinNoise {
    pub fn new(random: &mut LegacyRandom, levels: i32) -> Self {
        let noise_levels = (0..levels).map(|_| ImprovedNoise::new(random)).collect();
        Self { noise_levels }
    }

    /// `getRegion` (the 10-arg form): sum every octave into `values`, one
    /// entry per cell, in x-then-z-then-y order.
    pub fn get_region(
        &self,
        values: &mut Vec<f64>,
        x: i32,
        y: i32,
        z: i32,
        size_x: i32,
        size_y: i32,
        size_z: i32,
        scale_x: f64,
        scale_y: f64,
        scale_z: f64,
    ) {
        prepare_region(values, (size_x * size_y * size_z) as usize);
        let mut frequency = 1.0;
        for level in &self.noise_levels {
            let mut octave_x = x as f64 * frequency * scale_x;
            let octave_y = y as f64 * frequency * scale_y;
            let mut octave_z = z as f64 * frequency * scale_z;
            let mut wrapped_x = lfloor(octave_x);
            let mut wrapped_z = lfloor(octave_z);
            octave_x -= wrapped_x as f64;
            octave_z -= wrapped_z as f64;
            wrapped_x %= 16777216;
            wrapped_z %= 16777216;
            octave_x += wrapped_x as f64;
            octave_z += wrapped_z as f64;
            level.add(
                values,
                octave_x,
                octave_y,
                octave_z,
                size_x,
                size_y,
                size_z,
                scale_x * frequency,
                scale_y * frequency,
                scale_z * frequency,
                frequency,
            );
            frequency /= 2.0;
        }
    }

    /// `getRegion` (the 8-arg 2D form): samples through y = 10; the last
    /// parameter is dropped, as in 1.7.10.
    pub fn get_region_2d(
        &self,
        values: &mut Vec<f64>,
        x: i32,
        z: i32,
        size_x: i32,
        size_z: i32,
        scale_x: f64,
        scale_z: f64,
        _unused: f64,
    ) {
        self.get_region(values, x, 10, z, size_x, 1, size_z, scale_x, 1.0, scale_z);
    }
}
