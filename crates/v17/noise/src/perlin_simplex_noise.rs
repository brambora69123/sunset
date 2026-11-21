//! `NoiseGenerator3`/`PerlinSimplexNoise`, 1.7.10: a stack of `SimplexNoise`
//! levels summed octave by octave with the 1/d weight the game places on each.

use crate::{Noise, SimplexNoise, prepare_region};
use sunset_1_7_core::LegacyRandom;

/// `PerlinSimplexNoise`: `levels` simplex-noise levels drawn from one stream.
pub struct PerlinSimplexNoise {
    noise_levels: Vec<SimplexNoise>,
}

impl Noise for PerlinSimplexNoise {}

impl PerlinSimplexNoise {
    pub fn new(random: &mut LegacyRandom, levels: i32) -> Self {
        let noise_levels = (0..levels).map(|_| SimplexNoise::new(random)).collect();
        Self { noise_levels }
    }

    /// `getValue`: the fBm sum, each octave weighted `1 / frequency`.
    pub fn get_value(&self, x: f64, y: f64) -> f64 {
        let mut total = 0.0;
        let mut frequency = 1.0;
        for level in &self.noise_levels {
            total += level.get_value(x * frequency, y * frequency) / frequency;
            frequency /= 2.0;
        }
        total
    }

    /// `getRegion` (the 8-arg form): the 9-arg form with the y exponent
    /// fixed at 0.5.
    pub fn get_region(
        &self,
        values: &mut Vec<f64>,
        x: f64,
        y: f64,
        size_x: i32,
        size_y: i32,
        scale_x: f64,
        scale_y: f64,
        scale_exponent: f64,
    ) {
        self.get_region_exponents(
            values,
            x,
            y,
            size_x,
            size_y,
            scale_x,
            scale_y,
            scale_exponent,
            0.5,
        );
    }

    /// `getRegion` (the 9-arg form): the x and y exponents scale each
    /// octave's frequency separately; the noise scale follows `0.55 / frequency`.
    pub fn get_region_exponents(
        &self,
        values: &mut Vec<f64>,
        x: f64,
        y: f64,
        size_x: i32,
        size_y: i32,
        scale_x: f64,
        scale_y: f64,
        scale_exponent_x: f64,
        scale_exponent_y: f64,
    ) {
        prepare_region(values, (size_x * size_y) as usize);
        let mut frequency = 1.0;
        let mut amplitude = 1.0;
        for level in &self.noise_levels {
            level.add(
                values,
                x,
                y,
                size_x,
                size_y,
                scale_x * amplitude * frequency,
                scale_y * amplitude * frequency,
                0.55 / frequency,
            );
            amplitude *= scale_exponent_x;
            frequency *= scale_exponent_y;
        }
    }
}
