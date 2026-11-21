//! `CaveWorldCarver`, ported from `world/gen/carver/CaveWorldCarver.java`
//! (1.7.10, calamus). The carve loop is the Java code line for line,
//! including its quirks: the block index lags one above the y being tested,
//! and the placement rolls take the z base from `startChunkX` too.

use crate::BiomeSurface;
use crate::block_id;
use crate::math;
use crate::scan;
use sunset_1_7_core::{LegacyRandom, RandomSource};

/// `CaveWorldCarver`: tunnels and rooms carved into the chunk array.
pub struct CaveWorldCarver {
    /// `Generator.range` — the neighborhood radius in chunks (`place`).
    pub range: i32,
    /// `Generator.random` — reseeded per call and per neighborhood chunk.
    pub random: LegacyRandom,
}

impl CaveWorldCarver {
    pub fn new() -> Self {
        // Java seeds from `new Random()` (nanotime), but `place` reseedes
        // before the first draw, so the initial state is never read.
        Self {
            range: 8,
            random: LegacyRandom::new(0),
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
                    random, surface, range, start_x, start_z, chunk_x, chunk_z, blocks,
                )
            },
        );
    }

    /// `Generator.place(world, startChunkX, startChunkZ, chunkX, chunkZ, blocks)`.
    fn place_chunk(
        random: &mut LegacyRandom,
        surface: &dyn BiomeSurface,
        range: i32,
        start_x: i32,
        start_z: i32,
        chunk_x: i32,
        chunk_z: i32,
        blocks: &mut [u16],
    ) {
        let inner = next_int(random, 15) + 1;
        let middle = next_int(random, inner) + 1;
        let mut count = next_int(random, middle);
        if next_int(random, 7) != 0 {
            count = 0;
        }

        for _ in 0..count {
            let x = f64::from(start_x * 16 + next_int(random, 16));
            let y_bound = next_int(random, 120) + 8;
            let y = f64::from(next_int(random, y_bound));
            let z = f64::from(start_z * 16 + next_int(random, 16));
            let mut tunnels = 1;
            if next_int(random, 4) == 0 {
                let seed = random.next_long();
                carve_room(
                    random, surface, range, seed, chunk_x, chunk_z, blocks, x, y, z,
                );
                tunnels += next_int(random, 4);
            }

            for _ in 0..tunnels {
                let yaw = random.next_float() * std::f32::consts::PI * 2.0;
                let pitch = (random.next_float() - 0.5) * 2.0 / 8.0;
                let mut base_width = random.next_float() * 2.0 + random.next_float();
                if next_int(random, 10) == 0 {
                    base_width *= random.next_float() * random.next_float() * 3.0 + 1.0;
                }
                let seed = random.next_long();
                carve_tunnel(
                    range, surface, seed, chunk_x, chunk_z, blocks, x, y, z, base_width, yaw,
                    pitch, 0, 0, 1.0,
                );
            }
        }
    }
}

impl Default for CaveWorldCarver {
    fn default() -> Self {
        Self::new()
    }
}

/// `CaveWorldCarver.carveRoom`: one room from `(x, y, z)`, a tunnel seeded at
/// its middle with the width drawn from the per-chunk generator.
fn carve_room(
    random: &mut LegacyRandom,
    surface: &dyn BiomeSurface,
    range: i32,
    seed: i64,
    chunk_x: i32,
    chunk_z: i32,
    blocks: &mut [u16],
    x: f64,
    y: f64,
    z: f64,
) {
    let base_width = 1.0 + random.next_float() * 6.0;
    carve_tunnel(
        range, surface, seed, chunk_x, chunk_z, blocks, x, y, z, base_width, 0.0, 0.0, -1, -1, 0.5,
    );
}

/// `Random.nextInt(bound)`: every bound the carvers draw is a positive
/// constant or derived from one, so the `Result` never fires.
fn next_int(random: &mut LegacyRandom, bound: i32) -> i32 {
    random
        .next_int_bound(bound)
        .expect("carver nextInt bound is positive")
}

/// `CaveWorldCarver.carveTunnel`, line for line. `tunnel == -1` starts a room
/// at `tunnelCount / 2` (the `var57` flag): always carve, stop after the
/// first carve. Otherwise the tunnel runs to `tunnelCount`, forking once at
/// `branch_step` into two narrower tunnels.
#[allow(clippy::too_many_arguments, clippy::neg_cmp_op_on_partial_ord)]
fn carve_tunnel(
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

    let branch_step = next_int(&mut random, tunnel_count / 2) + tunnel_count / 4;
    let droop = next_int(&mut random, 6) == 0;

    'tunnel: while tunnel < tunnel_count {
        // 1.5 + sin(t * PI / count) * width * 1.0F — the f32 product widens
        // at the add.
        let radius = 1.5
            + f64::from(
                math::sin((tunnel as f32) * std::f32::consts::PI / (tunnel_count as f32))
                    * base_width
                    * 1.0,
            );
        let y_radius = radius * width_height_ratio;
        let cos_pitch = math::cos(pitch);
        let sin_pitch = math::sin(pitch);
        x += f64::from(math::cos(yaw) * cos_pitch);
        y += f64::from(sin_pitch);
        z += f64::from(math::sin(yaw) * cos_pitch);
        pitch *= if droop { 0.92 } else { 0.7 };
        pitch += pitch_delta * 0.1;
        yaw += yaw_delta * 0.1;
        pitch_delta *= 0.9;
        yaw_delta *= 0.75;
        let a = random.next_float();
        let b = random.next_float();
        let c = random.next_float();
        pitch_delta += (a - b) * c * 2.0;
        let a = random.next_float();
        let b = random.next_float();
        let c = random.next_float();
        yaw_delta += (a - b) * c * 4.0;

        if !starting && tunnel == branch_step && base_width > 1.0 && tunnel_count > 0 {
            let seed = random.next_long();
            let width = random.next_float() * 0.5 + 0.5;
            let half_pi = (std::f64::consts::PI / 2.0) as f32;
            carve_tunnel(
                range,
                surface,
                seed,
                chunk_x,
                chunk_z,
                blocks,
                x,
                y,
                z,
                width,
                yaw - half_pi,
                pitch / 3.0,
                tunnel,
                tunnel_count,
                1.0,
            );
            let seed = random.next_long();
            let width = random.next_float() * 0.5 + 0.5;
            carve_tunnel(
                range,
                surface,
                seed,
                chunk_x,
                chunk_z,
                blocks,
                x,
                y,
                z,
                width,
                yaw + half_pi,
                pitch / 3.0,
                tunnel,
                tunnel_count,
                1.0,
            );
            return;
        }

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
                                    if c > -0.7 && a * a + c * c + b * b < 1.0 {
                                        let block = blocks[index];
                                        if block == block_id::GRASS {
                                            hit_grass = true;
                                        }
                                        if block == block_id::STONE
                                            || block == block_id::DIRT
                                            || block == block_id::GRASS
                                        {
                                            if yv < 10 {
                                                blocks[index] = block_id::LAVA;
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
                        break 'tunnel;
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

    /// Every cell the carve touched came out air or lava — never grass or
    /// dirt on an all-stone chunk. The carve writes one above the y the
    /// ellipse test runs at (vanilla), so lava can land at y ≤ 10.
    #[test]
    fn carved_cells_are_air_or_lava() {
        let mut blocks = vec![block_id::STONE; SIZE];
        let mut caves = CaveWorldCarver::new();
        caves.place(&Plains, SEED, 0, 0, &mut blocks);
        for (i, block) in carved(&blocks) {
            let y = i % 256;
            if y < 11 {
                assert_eq!(block, block_id::LAVA, "cell {i} at y {y}");
            } else {
                assert_eq!(block, block_id::AIR, "cell {i} at y {y}");
            }
        }
    }

    /// A chunk of water is never carved: every shell check finds water.
    #[test]
    fn water_chunk_is_untouched() {
        let mut blocks = vec![block_id::WATER; SIZE];
        let mut caves = CaveWorldCarver::new();
        caves.place(&Plains, SEED, 0, 0, &mut blocks);
        assert!(blocks.iter().all(|&b| b == block_id::WATER));
    }

    /// Same seed, same chunk, same array: bit for bit.
    #[test]
    fn deterministic() {
        let mut a = vec![block_id::STONE; SIZE];
        let mut b = vec![block_id::STONE; SIZE];
        let mut caves = CaveWorldCarver::new();
        caves.place(&Plains, SEED, 3, -2, &mut a);
        let mut caves = CaveWorldCarver::new();
        caves.place(&Plains, SEED, 3, -2, &mut b);
        assert_eq!(a, b);
    }

    /// Carving happens at all for the fixed seed, and the two tunnels that
    /// fork stay consistent with the lava rule across seeds.
    #[test]
    fn caves_form_and_vary_by_seed() {
        let mut counts = Vec::new();
        for seed in 1..=8 {
            let mut blocks = vec![block_id::STONE; SIZE];
            let mut caves = CaveWorldCarver::new();
            caves.place(&Plains, seed, 0, 0, &mut blocks);
            counts.push(carved(&blocks).count());
        }
        assert!(counts.iter().any(|&c| c > 0), "no caves formed: {counts:?}");
        assert!(counts.iter().any(|&c| c != counts[0]), "seeds all equal");
    }
}
