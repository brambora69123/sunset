//! `OverworldChunkGenerator`, 1.7.10: the heightmap noise stack, the trilinear
//! terrain fill and the biome surface pass, in the order `getChunk` runs them.
//! Blocks are vanilla 1.7.10 numeric ids in a flat array indexed
//! `x << 12 | z << 8 | y`, `0` = air, as `WorldChunk` reads them.

use sunset_1_7::biome::{self, Layer};
use sunset_1_7_core::{LegacyRandom, RandomSource};
use sunset_1_7_noise::{PerlinNoise, PerlinSimplexNoise};

use crate::biome_facts;
use crate::{AIR, BEDROCK, DIRT, GRASS, GRAVEL, ICE, SAND, SANDSTONE, STONE, WATER};

/// Vanilla sea level, the `byte var4 = 63` both passes read.
pub const SEA_LEVEL: i32 = 63;

/// `WorldGeneratorType`, the part the overworld generator reads.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GeneratorType {
    Default,
    Flat,
    LargeBiomes,
    Amplified,
    Default11,
}

/// The chunk `getChunk` produces, before carving and population.
pub struct GeneratedChunk {
    pub chunk_x: i32,
    pub chunk_z: i32,
    /// Flat `x << 12 | z << 8 | y`, `0` = air.
    pub blocks: Vec<u16>,
    /// Flat block metadata, same layout.
    pub metadata: Vec<u8>,
    /// 16x16 biome ids, `x | z << 4`.
    pub biomes: [u8; 256],
}

pub struct OverworldChunkGenerator {
    random: LegacyRandom,
    min_limit: PerlinNoise,
    max_limit: PerlinNoise,
    perlin1: PerlinNoise,
    simplex3: PerlinSimplexNoise,
    /// Constructed for the seed stream like vanilla; 1.7.10 never samples it.
    pub scale_noise: PerlinNoise,
    pub depth_noise: PerlinNoise,
    pub forest_noise: PerlinNoise,
    /// `Biome.TEMPERATURE_NOISE`: a fixed-seed shared noise the surface pass
    /// reads for the ice line. Rebuilt per generator; identical every time.
    temperature_noise: PerlinSimplexNoise,
    height_map: Vec<f64>,
    biome_weights: [f32; 25],
    depth_noise_buffer: Vec<f64>,
    perlin_buffer: Vec<f64>,
    min_limit_buffer: Vec<f64>,
    max_limit_buffer: Vec<f64>,
    depth_buffer: Vec<f64>,
    generator_type: GeneratorType,
    gen_biomes: Layer,
    biome_index: Layer,
    noise_biomes: Vec<i32>,
}

impl OverworldChunkGenerator {
    pub fn new(seed: i64, generator_type: GeneratorType) -> Self {
        let mut random = LegacyRandom::new(seed);
        // The constructor draw order is vanilla: every noise level draws from
        // this one stream, so a swap would move every later offset.
        let min_limit = PerlinNoise::new(&mut random, 16);
        let max_limit = PerlinNoise::new(&mut random, 16);
        let perlin1 = PerlinNoise::new(&mut random, 8);
        let simplex3 = PerlinSimplexNoise::new(&mut random, 4);
        let scale_noise = PerlinNoise::new(&mut random, 10);
        let depth_noise = PerlinNoise::new(&mut random, 16);
        let forest_noise = PerlinNoise::new(&mut random, 8);
        let mut temperature_random = LegacyRandom::new(1234);
        let temperature_noise = PerlinSimplexNoise::new(&mut temperature_random, 1);

        let mut biome_weights = [0.0f32; 25];
        for dx in -2i32..=2 {
            for dz in -2i32..=2 {
                let weight = 10.0f32 / ((dx * dx + dz * dz) as f32 + 0.2f32).sqrt();
                biome_weights[(dx + 2 + (dz + 2) * 5) as usize] = weight;
            }
        }

        let [gen_biomes, biome_index, _] =
            biome::init(seed, generator_type == GeneratorType::LargeBiomes);
        Self {
            random,
            min_limit,
            max_limit,
            perlin1,
            simplex3,
            scale_noise,
            depth_noise,
            forest_noise,
            temperature_noise,
            height_map: vec![0.0; 825],
            biome_weights,
            depth_noise_buffer: Vec::new(),
            perlin_buffer: Vec::new(),
            min_limit_buffer: Vec::new(),
            max_limit_buffer: Vec::new(),
            depth_buffer: Vec::new(),
            generator_type,
            gen_biomes,
            biome_index,
            noise_biomes: Vec::new(),
        }
    }

    /// `getChunk`, in vanilla order: the chunk seed, `buildTerrain`, the 16x16
    /// biome map, `buildSurfaces`. Carving (the carver crate) and structure
    /// placement run right after `build_surfaces`; population follows the
    /// finished chunk.
    pub fn get_chunk(&mut self, chunk_x: i32, chunk_z: i32) -> GeneratedChunk {
        self.random.set_seed(
            (chunk_x as i64)
                .wrapping_mul(341_873_128_712)
                .wrapping_add((chunk_z as i64).wrapping_mul(132_897_987_541)),
        );
        let mut blocks = vec![AIR; 65536];
        let mut metadata = vec![0u8; 65536];
        self.build_terrain(chunk_x, chunk_z, &mut blocks);
        let area = self.biome_index.area(chunk_x * 16, chunk_z * 16, 16, 16);
        let mut biomes = [0u8; 256];
        for (slot, id) in biomes.iter_mut().zip(area) {
            *slot = id as u8;
        }
        self.build_surfaces(chunk_x, chunk_z, &mut blocks, &mut metadata, &biomes);
        GeneratedChunk {
            chunk_x,
            chunk_z,
            blocks,
            metadata,
            biomes,
        }
    }

    /// `buildTerrain`: the noise biomes (10x10 at 4-block scale, offset -2),
    /// the heightmap, then the trilinear fill. Density above zero is stone,
    /// below it water under sea level and air above.
    pub fn build_terrain(&mut self, chunk_x: i32, chunk_z: i32, blocks: &mut [u16]) {
        self.noise_biomes = self
            .gen_biomes
            .area(chunk_x * 4 - 2, chunk_z * 4 - 2, 10, 10);
        self.generate_height_map(chunk_x * 4, 0, chunk_z * 4);

        for node_x in 0..4i32 {
            let left = node_x * 5;
            let right = (node_x + 1) * 5;
            for node_z in 0..4i32 {
                let base00 = (left + node_z) * 33;
                let base10 = (left + node_z + 1) * 33;
                let base01 = (right + node_z) * 33;
                let base11 = (right + node_z + 1) * 33;
                for grid_y in 0..32i32 {
                    let mut c00 = self.height_map[(base00 + grid_y) as usize];
                    let mut c10 = self.height_map[(base10 + grid_y) as usize];
                    let mut c01 = self.height_map[(base01 + grid_y) as usize];
                    let mut c11 = self.height_map[(base11 + grid_y) as usize];
                    let step_y00 = (self.height_map[(base00 + grid_y + 1) as usize] - c00) * 0.125;
                    let step_y10 = (self.height_map[(base10 + grid_y + 1) as usize] - c10) * 0.125;
                    let step_y01 = (self.height_map[(base01 + grid_y + 1) as usize] - c01) * 0.125;
                    let step_y11 = (self.height_map[(base11 + grid_y + 1) as usize] - c11) * 0.125;
                    for sub_y in 0..8i32 {
                        let y = grid_y * 8 + sub_y;
                        let mut x00 = c00;
                        let mut x10 = c10;
                        let step_x00 = (c01 - c00) * 0.25;
                        let step_x10 = (c11 - c10) * 0.25;
                        for sub_x in 0..4i32 {
                            let mut index =
                                ((((sub_x + node_x * 4) << 12) | ((node_z * 4) << 8) | y) - 256)
                                    as isize;
                            let step_z = (x10 - x00) * 0.25;
                            let mut value = x00 - step_z;
                            for _ in 0..4 {
                                value += step_z;
                                index += 256;
                                blocks[index as usize] = if value > 0.0 {
                                    STONE
                                } else if y < SEA_LEVEL {
                                    WATER
                                } else {
                                    AIR
                                };
                            }
                            x00 += step_x00;
                            x10 += step_x10;
                        }
                        c00 += step_y00;
                        c10 += step_y10;
                        c01 += step_y01;
                        c11 += step_y11;
                    }
                }
            }
        }
    }

    /// `generateHeightMap`: the 5x5x33 density field the fill samples. Each
    /// node blends the 25 surrounding biomes by weight, shapes the depth noise
    /// into the base height, then lerps the min/max limit noises.
    fn generate_height_map(&mut self, x: i32, y: i32, z: i32) {
        self.depth_noise
            .get_region_2d(&mut self.depth_noise_buffer, x, z, 5, 5, 200.0, 200.0, 0.5);
        self.perlin1.get_region(
            &mut self.perlin_buffer,
            x,
            y,
            z,
            5,
            33,
            5,
            8.555150000000001,
            4.277575000000001,
            8.555150000000001,
        );
        self.min_limit.get_region(
            &mut self.min_limit_buffer,
            x,
            y,
            z,
            5,
            33,
            5,
            684.412,
            684.412,
            684.412,
        );
        self.max_limit.get_region(
            &mut self.max_limit_buffer,
            x,
            y,
            z,
            5,
            33,
            5,
            684.412,
            684.412,
            684.412,
        );

        let mut node = 0usize;
        let mut depth_index = 0usize;
        for grid_x in 0..5i32 {
            for grid_z in 0..5i32 {
                let mut variation = 0.0f32;
                let mut height = 0.0f32;
                let mut weight_total = 0.0f32;
                let center = self.noise_biomes[(grid_x + 2 + (grid_z + 2) * 10) as usize];
                for dx in -2i32..=2 {
                    for dz in -2i32..=2 {
                        let neighbor =
                            self.noise_biomes[(grid_x + dx + 2 + (grid_z + dz + 2) * 10) as usize];
                        let mut base = biome_facts::base_height(neighbor);
                        let mut variance = biome_facts::height_variation(neighbor);
                        if self.generator_type == GeneratorType::Amplified && base > 0.0 {
                            base = 1.0 + base * 2.0;
                            variance = 1.0 + variance * 4.0;
                        }
                        let mut weight =
                            self.biome_weights[(dx + 2 + (dz + 2) * 5) as usize] / (base + 2.0);
                        if biome_facts::base_height(neighbor) > biome_facts::base_height(center) {
                            weight /= 2.0;
                        }
                        variation += variance * weight;
                        height += base * weight;
                        weight_total += weight;
                    }
                }
                variation /= weight_total;
                height /= weight_total;
                variation = variation * 0.9 + 0.1;
                height = (height * 4.0 - 1.0) / 8.0;

                let mut shaped = self.depth_noise_buffer[depth_index] / 8000.0;
                if shaped < 0.0 {
                    shaped = -shaped * 0.3;
                }
                shaped = shaped * 3.0 - 2.0;
                if shaped < 0.0 {
                    shaped /= 2.0;
                    if shaped < -1.0 {
                        shaped = -1.0;
                    }
                    shaped /= 1.4;
                    shaped /= 2.0;
                } else {
                    if shaped > 1.0 {
                        shaped = 1.0;
                    }
                    shaped /= 8.0;
                }
                depth_index += 1;

                let mut depth_base = height as f64;
                let vertical = variation as f64;
                depth_base += shaped * 0.2;
                depth_base = depth_base * 8.5 / 8.0;
                let center_height = 8.5 + depth_base * 4.0;

                for grid_y in 0..33i32 {
                    let mut taper =
                        (grid_y as f64 - center_height) * 12.0 * 128.0 / 256.0 / vertical;
                    if taper < 0.0 {
                        taper *= 4.0;
                    }
                    let min = self.min_limit_buffer[node] / 512.0;
                    let max = self.max_limit_buffer[node] / 512.0;
                    let blend = (self.perlin_buffer[node] / 10.0 + 1.0) / 2.0;
                    let mut density = clamped_lerp(min, max, blend) - taper;
                    if grid_y > 29 {
                        let fall = ((grid_y - 29) as f32 / 3.0f32) as f64;
                        density = density * (1.0 - fall) + -10.0 * fall;
                    }
                    self.height_map[node] = density;
                    node += 1;
                }
            }
        }
    }

    /// `buildSurfaces`: the stone-noise depth per column, then the per-biome
    /// surface builder. Vanilla reads both the biome and the noise at the
    /// transposed index `z + x * 16`; kept verbatim.
    pub fn build_surfaces(
        &mut self,
        chunk_x: i32,
        chunk_z: i32,
        blocks: &mut [u16],
        metadata: &mut [u8],
        biomes: &[u8; 256],
    ) {
        self.simplex3.get_region(
            &mut self.depth_buffer,
            (chunk_x * 16) as f64,
            (chunk_z * 16) as f64,
            16,
            16,
            0.0625,
            0.0625,
            1.0,
        );
        for offset_x in 0..16i32 {
            for offset_z in 0..16i32 {
                let index = (offset_z + offset_x * 16) as usize;
                let biome_id = biomes[index] as i32;
                self.prepare_and_build_surfaces(
                    blocks,
                    metadata,
                    chunk_x * 16 + offset_x,
                    chunk_z * 16 + offset_z,
                    biome_id,
                    self.depth_buffer[index],
                );
            }
        }
    }

    /// The depth-based surface override `ExtremeHillsBiome
    /// .prepareAndBuildSurfaces` applies (id 3 only; the edge and + variants
    /// always keep grass), then the generic per-column builder.
    fn prepare_and_build_surfaces(
        &mut self,
        blocks: &mut [u16],
        metadata: &mut [u8],
        world_x: i32,
        world_z: i32,
        biome_id: i32,
        depth: f64,
    ) {
        let (mut surface, mut surface_md, mut subsurface) = match biome_id {
            3 => {
                if depth > 1.0 {
                    (STONE, 0, STONE)
                } else {
                    (GRASS, 0, DIRT)
                }
            }
            _ => biome_facts::surface_blocks(biome_id),
        };
        let mut depth_left = -1i32;
        let surface_depth = (depth / 3.0 + 3.0 + self.random.next_double() * 0.25) as i32;
        let local_x = world_x & 15;
        let local_z = world_z & 15;
        let column_base = (local_z * 16 + local_x) * 256;

        for y in (0..256i32).rev() {
            let index = (column_base + y) as usize;
            if y <= self.random.next_int_bound(5).expect("bound is 5") {
                blocks[index] = BEDROCK;
            } else {
                let current = blocks[index];
                if current == AIR {
                    depth_left = -1;
                } else if current == STONE {
                    if depth_left == -1 {
                        if surface_depth <= 0 {
                            surface = AIR;
                            surface_md = 0;
                            subsurface = STONE;
                        } else if (59..=64).contains(&y) {
                            (surface, surface_md, subsurface) =
                                biome_facts::surface_blocks(biome_id);
                        }
                        if y < SEA_LEVEL && surface == AIR {
                            if self.get_temperature(world_x, y, world_z, biome_id) < 0.15 {
                                surface = ICE;
                                surface_md = 0;
                            } else {
                                surface = WATER;
                                surface_md = 0;
                            }
                        }

                        depth_left = surface_depth;
                        if y >= 62 {
                            blocks[index] = surface;
                            metadata[index] = surface_md;
                        } else if y < 56 - surface_depth {
                            surface = AIR;
                            subsurface = STONE;
                            blocks[index] = GRAVEL;
                        } else {
                            blocks[index] = subsurface;
                        }
                    } else if depth_left > 0 {
                        depth_left -= 1;
                        blocks[index] = subsurface;
                        if depth_left == 0 && subsurface == SAND {
                            depth_left = self.random.next_int_bound(4).expect("bound is 4")
                                + (y - 63).max(0);
                            subsurface = SANDSTONE;
                        }
                    }
                }
            }
        }
    }

    /// `Biome.getTemperature`: the biome temperature, thinned by height and the
    /// shared temperature noise above y 64.
    fn get_temperature(&self, world_x: i32, y: i32, world_z: i32, biome_id: i32) -> f32 {
        if y > 64 {
            let noise =
                self.temperature_noise
                    .get_value(world_x as f64 / 8.0, world_z as f64 / 8.0) as f32
                    * 4.0;
            biome_facts::temperature(biome_id) - (noise + y as f32 - 64.0) * 0.05 / 30.0
        } else {
            biome_facts::temperature(biome_id)
        }
    }
}

/// `MathHelper.clampedLerp`.
fn clamped_lerp(min: f64, max: f64, delta: f64) -> f64 {
    if delta < 0.0 {
        min
    } else if delta > 1.0 {
        max
    } else {
        min + (max - min) * delta
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AIR, BEDROCK, STONE, WATER};

    #[test]
    fn biome_weight_table() {
        let generator = OverworldChunkGenerator::new(1, GeneratorType::Default);
        // index = dx + 2 + (dz + 2) * 5: (−2,−2) → 10/√8.2, (0,0) → 10/√0.2,
        // (0,2) → 10/√4.2, computed in f64.
        assert!((generator.biome_weights[0] - 3.492_152).abs() < 1e-5);
        assert!((generator.biome_weights[12] - 22.36068).abs() < 1e-4);
        assert!((generator.biome_weights[22] - 4.8795).abs() < 1e-5);
    }

    #[test]
    fn terrain_fill_invariants() {
        let mut generator = OverworldChunkGenerator::new(42, GeneratorType::Default);
        let mut blocks = vec![AIR; 65536];
        generator.build_terrain(0, 0, &mut blocks);
        for (index, &id) in blocks.iter().enumerate() {
            let y = (index & 255) as i32;
            if y < SEA_LEVEL {
                assert!(id == STONE || id == WATER, "air below sea level at {index}");
            } else {
                assert!(id == STONE || id == AIR, "water above sea level at {index}");
            }
        }
    }

    #[test]
    fn surface_bedrock_floor() {
        let mut generator = OverworldChunkGenerator::new(7, GeneratorType::Default);
        let chunk = generator.get_chunk(0, 0);
        for x in 0..16i32 {
            for z in 0..16i32 {
                let index = ((x << 12) | (z << 8)) as usize;
                assert_eq!(chunk.blocks[index], BEDROCK, "y 0 not bedrock at ({x},{z})");
            }
        }
    }

    #[test]
    fn get_chunk_is_deterministic() {
        let mut a = OverworldChunkGenerator::new(999, GeneratorType::Default);
        let mut b = OverworldChunkGenerator::new(999, GeneratorType::Default);
        let chunk_a = a.get_chunk(3, -2);
        let chunk_b = b.get_chunk(3, -2);
        assert_eq!(chunk_a.blocks, chunk_b.blocks);
        assert_eq!(chunk_a.metadata, chunk_b.metadata);
        assert_eq!(chunk_a.biomes, chunk_b.biomes);

        let mut c = OverworldChunkGenerator::new(1000, GeneratorType::Default);
        let chunk_c = c.get_chunk(3, -2);
        assert_ne!(chunk_a.blocks, chunk_c.blocks);
    }

    #[test]
    fn clamped_lerp_bounds() {
        assert_eq!(clamped_lerp(1.0, 5.0, -0.5), 1.0);
        assert_eq!(clamped_lerp(1.0, 5.0, 1.5), 5.0);
        assert_eq!(clamped_lerp(1.0, 5.0, 0.25), 2.0);
    }
}
