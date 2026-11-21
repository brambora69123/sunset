//! `RavineWorldCarver`, ported from `world/gen/carver/RavineWorldCarver.java`
//! (1.7.10, calamus). Same carve loop as the cave carver, with a per-y
//! thickness buffer weighting the shape test and flowing lava at the bottom.
//! The block index lags one above the y being tested, as in vanilla.

use crate::BiomeSurface;
use crate::block_id;
use crate::math;
use crate::scan;
use sunset_1_7_core::{LegacyRandom, RandomSource};

/// `RavineWorldCarver`: ravines carved into the chunk array.
pub struct RavineWorldCarver {
    /// `Generator.range` — the neighborhood radius in chunks (`place`).
    pub range: i32,
    /// `Generator.random` — reseeded per call and per neighborhood chunk.
    pub random: LegacyRandom,
    /// `RavineWorldCarver.buffer` — per-y thickness squares, reused per carve.
    buffer: [f32; 1024],
}

impl RavineWorldCarver {
    pub fn new() -> Self {
        // Java seeds from `new Random()` (nanotime), but `place` reseedes
        // before the first draw, so the initial state is never read.
        Self {
            range: 8,
            random: LegacyRandom::new(0),
            buffer: [0.0; 1024],
        }
    }

    /// `Generator.place(source, world, chunkX, chunkZ, blocks)`: the
    /// chunk-neighborhood recursion.
    pub fn place(
        &mut self,
        surface: &dyn BiomeSurface,
        world_seed: i64,
        chunk_x: i32,
        chunk_z: i32,
        blocks: &mut [u16],
    ) {
        let random = &mut self.random;
        let range = self.range;
        crate::carve_neighborhood(
            random,
            range,
            surface,
            world_seed,
            chunk_x,
            chunk_z,
            blocks,
            |random, surface, start_x, start_z, chunk_x, chunk_z, blocks| {
                Self::place_chunk(
                    random,
                    surface,
                    &mut self.buffer,
                    range,
                    start_x,
                    start_z,
                    chunk_x,
                    chunk_z,
                    blocks,
                )
            },
        );
    }

    /// `Generator.place(world, startChunkX, startChunkZ, chunkX, chunkZ, blocks)`.
    fn place_chunk(
        random: &mut LegacyRandom,
        surface: &dyn BiomeSurface,
        buffer: &mut [f32; 1024],
        range: i32,
        start_x: i32,
        start_z: i32,
        chunk_x: i32,
        chunk_z: i32,
        blocks: &mut [u16],
    ) {
        if next_int(random, 50) != 0 {
            return;
        }

        let x = f64::from(start_x * 16 + next_int(random, 16));
        let y_bound = next_int(random, 40) + 8;
        let y = f64::from(next_int(random, y_bound) + 20);
        let z = f64::from(start_z * 16 + next_int(random, 16));

        // `byte var13 = 1` — the loop runs once.
        let yaw = random.next_float() * std::f32::consts::PI * 2.0;
        let pitch = (random.next_float() - 0.5) * 2.0 / 8.0;
        let base_width = (random.next_float() * 2.0 + random.next_float()) * 2.0;
        let seed = random.next_long();
        carve_ravine(
            buffer, range, surface, seed, chunk_x, chunk_z, blocks, x, y, z, base_width, yaw,
            pitch, 0, 0, 3.0,
        );
    }
}

impl Default for RavineWorldCarver {
    fn default() -> Self {
        Self::new()
    }
}

/// `Random.nextInt(bound)`: every bound the carvers draw is a positive
/// constant or derived from one, so the `Result` never fires.
fn next_int(random: &mut LegacyRandom, bound: i32) -> i32 {
    random
        .next_int_bound(bound)
        .expect("carver nextInt bound is positive")
}

/// `RavineWorldCarver.carveRavine`, line for line. `tunnel == -1` starts at
/// `tunnelCount / 2` (the `var56` flag): always carve, stop after the first
/// carve. There is no fork branch.
#[allow(clippy::too_many_arguments, clippy::neg_cmp_op_on_partial_ord)]
fn carve_ravine(
    buffer: &mut [f32; 1024],
    range: i32,
    surface: &dyn BiomeSurface,
    seed: i64,
    chunk_x: i32,
    chunk_z: i32,
    blocks: &mut [u16],
    mut x: f64,
    mut y: f64,
    mut z: f64,
    base_width: f32,
    mut yaw: f32,
    mut pitch: f32,
    mut tunnel: i32,
    mut tunnel_count: i32,
    width_height_ratio: f64,
) {
    let mut random = LegacyRandom::new(seed);
    let center_x = f64::from(chunk_x * 16 + 8);
    let center_z = f64::from(chunk_z * 16 + 8);
    let mut yaw_delta = 0.0f32;
    let mut pitch_delta = 0.0f32;
    if tunnel_count <= 0 {
        let span = range * 16 - 16;
        tunnel_count = span - next_int(&mut random, span / 4);
    }

    let starting = tunnel == -1;
    if starting {
        tunnel = tunnel_count / 2;
    }

    // The thickness squares: redrawn at step 0 and whenever `nextInt(3)` hits.
    let mut thickness = 1.0f32;
    for step in 0usize..256 {
        if step == 0 || next_int(&mut random, 3) == 0 {
            thickness = 1.0 + random.next_float() * random.next_float() * 1.0;
        }
        buffer[step] = thickness * thickness;
    }

    'carve: while tunnel < tunnel_count {
        // 1.5 + sin(t * PI / count) * width * 1.0F — the f32 product widens
        // at the add; then both radii shrink by a fresh `nextFloat * 0.25 +
        // 0.75`.
        let mut radius = 1.5
            + f64::from(
                math::sin((tunnel as f32) * std::f32::consts::PI / (tunnel_count as f32))
                    * base_width
                    * 1.0,
            );
        let mut y_radius = radius * width_height_ratio;
        radius *= f64::from(random.next_float() * 0.25 + 0.75);
        y_radius *= f64::from(random.next_float() * 0.25 + 0.75);
        let cos_pitch = math::cos(pitch);
        let sin_pitch = math::sin(pitch);
        x += f64::from(math::cos(yaw) * cos_pitch);
        y += f64::from(sin_pitch);
        z += f64::from(math::sin(yaw) * cos_pitch);
        pitch *= 0.7;
        pitch += pitch_delta * 0.05;
        yaw += yaw_delta * 0.05;
        pitch_delta *= 0.8;
        yaw_delta *= 0.5;
        let a = random.next_float();
        let b = random.next_float();
        let c = random.next_float();
        pitch_delta += (a - b) * c * 2.0;
        let a = random.next_float();
        let b = random.next_float();
        let c = random.next_float();
        yaw_delta += (a - b) * c * 4.0;

        if starting || next_int(&mut random, 4) != 0 {
            let dx = x - center_x;
            let dz = z - center_z;
            let remaining = f64::from(tunnel_count - tunnel);
            let reach = f64::from(base_width + 2.0 + 16.0);
            if dx * dx + dz * dz - remaining * remaining > reach * reach {
                return;
            }

            if !(x < center_x - 16.0 - radius * 2.0)
                && !(z < center_z - 16.0 - radius * 2.0)
                && !(x > center_x + 16.0 + radius * 2.0)
                && !(z > center_z + 16.0 + radius * 2.0)
            {
                let x_min = (math::floor(x - radius) - chunk_x * 16 - 1).max(0);
                let x_max = (math::floor(x + radius) - chunk_x * 16 + 1).min(16);
                let y_min = (math::floor(y - y_radius) - 1).max(1);
                let y_max = (math::floor(y + y_radius) + 1).min(248);
                let z_min = (math::floor(z - radius) - chunk_z * 16 - 1).max(0);
                let z_max = (math::floor(z + radius) - chunk_z * 16 + 1).min(16);

                if !scan::touches_water(blocks, x_min, x_max, y_min, y_max, z_min, z_max) {
                    for xi in x_min..x_max {
                        let a = (f64::from(xi + chunk_x * 16) + 0.5 - x) / radius;
                        for zi in z_min..z_max {
                            let b = (f64::from(zi + chunk_z * 16) + 0.5 - z) / radius;
                            // The index lags one above the y being tested:
                            // vanilla reads and writes at `y_max` first.
                            let mut index = scan::index(xi, zi, y_max);
                            let mut hit_grass = false;
                            if a * a + b * b < 1.0 {
                                for yv in (y_min..y_max).rev() {
                                    let c = (f64::from(yv) + 0.5 - y) / y_radius;
                                    if (a * a + b * b) * f64::from(buffer[yv as usize])
                                        + c * c / 6.0
                                        < 1.0
                                    {
                                        let block = blocks[index];
                                        if block == block_id::GRASS {
                                            hit_grass = true;
                                        }
                                        if block == block_id::STONE
                                            || block == block_id::DIRT
                                            || block == block_id::GRASS
                                        {
                                            if yv < 10 {
                                                blocks[index] = block_id::FLOWING_LAVA;
                                            } else {
                                                blocks[index] = block_id::AIR;
                                                if hit_grass && blocks[index - 1] == block_id::DIRT
                                                {
                                                    blocks[index - 1] = surface.surface_block(
                                                        xi + chunk_x * 16,
                                                        zi + chunk_z * 16,
                                                    );
                                                }
                                            }
                                        }
                                    }
                                    index -= 1;
                                }
                            }
                        }
                    }

                    if starting {
                        break 'carve;
                    }
                }
            }
        }

        tunnel += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_id;

    struct Plains;
    impl BiomeSurface for Plains {
        fn surface_block(&self, _x: i32, _z: i32) -> u16 {
            block_id::GRASS
        }
    }

    const SEED: i64 = 0x5EED_1710;
    const SIZE: usize = 16 * 16 * 256;

    fn carved(blocks: &[u16]) -> impl Iterator<Item = (usize, u16)> + '_ {
        blocks
            .iter()
            .enumerate()
            .filter(|&(_, &b)| b != block_id::STONE)
            .map(|(i, &b)| (i, b))
    }

    /// Every cell the carve touched came out air or flowing lava — never
    /// grass or dirt on an all-stone chunk. The carve writes one above the y
    /// the shape test runs at (vanilla), so lava can land at y ≤ 10.
    #[test]
    fn carved_cells_are_air_or_flowing_lava() {
        let mut blocks = vec![block_id::STONE; SIZE];
        let mut ravines = RavineWorldCarver::new();
        ravines.place(&Plains, SEED, 0, 0, &mut blocks);
        for (i, block) in carved(&blocks) {
            let y = i % 256;
            if y < 11 {
                assert_eq!(block, block_id::FLOWING_LAVA, "cell {i} at y {y}");
            } else {
                assert_eq!(block, block_id::AIR, "cell {i} at y {y}");
            }
        }
    }

    /// A chunk of water is never carved: every shell check finds water.
    #[test]
    fn water_chunk_is_untouched() {
        let mut blocks = vec![block_id::WATER; SIZE];
        let mut ravines = RavineWorldCarver::new();
        ravines.place(&Plains, SEED, 0, 0, &mut blocks);
        assert!(blocks.iter().all(|&b| b == block_id::WATER));
    }

    /// Same seed, same chunk, same array: bit for bit.
    #[test]
    fn deterministic() {
        let mut a = vec![block_id::STONE; SIZE];
        let mut b = vec![block_id::STONE; SIZE];
        let mut ravines = RavineWorldCarver::new();
        ravines.place(&Plains, SEED, 3, -2, &mut a);
        let mut ravines = RavineWorldCarver::new();
        ravines.place(&Plains, SEED, 3, -2, &mut b);
        assert_eq!(a, b);
    }

    /// One placement roll in 50 carves, so over 100 seeds most chunks are
    /// untouched but some ravines form.
    #[test]
    fn ravines_are_rare_and_vary_by_seed() {
        let mut counts = Vec::new();
        for seed in 0..100 {
            let mut blocks = vec![block_id::STONE; SIZE];
            let mut ravines = RavineWorldCarver::new();
            ravines.place(&Plains, seed, 0, 0, &mut blocks);
            counts.push(carved(&blocks).count());
        }
        let formed = counts.iter().filter(|&&c| c > 0).count();
        assert!(formed > 0, "no ravine formed in 100 seeds");
        assert!(formed < 50, "ravines too common: {formed} of 100");
    }
}
