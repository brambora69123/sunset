//! Stage-by-stage hashes of the population pass, matching `StageDump.java`.

use std::collections::HashMap;

use sunset_1_7_carver::{BiomeSurface, CaveWorldCarver, RavineWorldCarver};
use sunset_1_7_chunkgen::{GeneratorType, OverworldChunkGenerator, biome_facts};
use sunset_1_7_core::{LegacyRandom, RandomSource};
use sunset_1_7_features::biome;
use sunset_1_7_features::block::Block;
use sunset_1_7_features::decorator::FeatureDecorator;
use sunset_1_7_features::features::dungeon::DungeonFeature;
use sunset_1_7_features::features::lake::LakeFeature;
use sunset_1_7_features::world::{FLAG_NO_NEIGHBOR_NOTIFY, GenWorld};

struct Chunk {
    blocks: Vec<u16>,
    metadata: Vec<u8>,
    biomes: [u8; 256],
}

struct World {
    chunks: HashMap<(i32, i32), Chunk>,
}

fn chunk_index(x: i32, y: i32, z: i32) -> usize {
    (((x & 15) as usize) << 12) | (((z & 15) as usize) << 8) | (y as usize & 255)
}

impl GenWorld for World {
    fn get_block(&self, x: i32, y: i32, z: i32) -> Block {
        if !(0..256).contains(&y) {
            return Block::Air;
        }
        match self.chunks.get(&(x >> 4, z >> 4)) {
            Some(chunk) => Block::from_id(chunk.blocks[chunk_index(x, y, z)] as u8),
            None => Block::Air,
        }
    }
    fn get_block_metadata(&self, x: i32, y: i32, z: i32) -> u8 {
        if !(0..256).contains(&y) {
            return 0;
        }
        match self.chunks.get(&(x >> 4, z >> 4)) {
            Some(chunk) => chunk.metadata[chunk_index(x, y, z)],
            None => 0,
        }
    }
    fn set_block_with_metadata(
        &mut self,
        x: i32,
        y: i32,
        z: i32,
        block: Block,
        metadata: u8,
        _f: u8,
    ) {
        if !(0..256).contains(&y) {
            return;
        }
        if let Some(chunk) = self.chunks.get_mut(&(x >> 4, z >> 4)) {
            let i = chunk_index(x, y, z);
            chunk.blocks[i] = block.id() as u16;
            chunk.metadata[i] = metadata;
        }
    }
    fn get_biome_id(&self, x: i32, z: i32) -> i32 {
        match self.chunks.get(&(x >> 4, z >> 4)) {
            Some(chunk) => chunk.biomes[((z & 15) * 16 + (x & 15)) as usize] as i32,
            None => 1,
        }
    }
    fn get_height(&self, x: i32, z: i32) -> i32 {
        (0..256)
            .rev()
            .find(|&y| self.get_block(x, y, z) != Block::Air)
            .map_or(0, |y| y + 1)
    }
    fn get_surface_height(&self, x: i32, z: i32) -> i32 {
        (0..256)
            .rev()
            .find(|&y| {
                let b = self.get_block(x, y, z);
                b.blocks_movement() && b != Block::Leaves
            })
            .map_or(0, |y| y + 1)
    }
    fn get_precipitation_height(&self, x: i32, z: i32) -> i32 {
        (0..256)
            .rev()
            .find(|&y| {
                let b = self.get_block(x, y, z);
                b.blocks_movement() || b.is_liquid()
            })
            .map_or(0, |y| y + 1)
    }
}

struct Surface {
    biomes: [u8; 256],
}

impl BiomeSurface for Surface {
    fn surface_block(&self, x: i32, z: i32) -> u16 {
        let id = self.biomes[((z & 15) * 16 + (x & 15)) as usize] as i32;
        biome_facts::surface_blocks(id).0
    }
}

fn hash(world: &World, cx: i32, cz: i32) -> String {
    let chunk = world.chunks.get(&(cx, cz)).unwrap();
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut non_air = 0u32;
    for x in 0..16i32 {
        for z in 0..16i32 {
            for y in 0..256i32 {
                let id = chunk.blocks[chunk_index(x, y, z)] as u64;
                if id != 0 {
                    non_air += 1;
                }
                h ^= id;
                h = h.wrapping_mul(0x100_0000_01b3);
            }
        }
    }
    format!("{h:x}/{non_air}")
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let seed: i64 = a[1].parse().unwrap();
    let cx: i32 = a[2].parse().unwrap();
    let cz: i32 = a[3].parse().unwrap();
    let radius = 1;

    let mut generator = OverworldChunkGenerator::new(seed, GeneratorType::Default);
    let mut world = World {
        chunks: HashMap::new(),
    };
    for dz in -(radius + 1)..=radius + 1 {
        for dx in -(radius + 1)..=radius + 1 {
            let generated = generator.get_chunk(cx + dx, cz + dz);
            let surface = Surface {
                biomes: generated.biomes,
            };
            let mut blocks = generated.blocks;
            let metadata = generated.metadata;
            CaveWorldCarver::default().place(&surface, seed, cx + dx, cz + dz, &mut blocks);
            RavineWorldCarver::default().place(&surface, seed, cx + dx, cz + dz, &mut blocks);
            world.chunks.insert(
                (cx + dx, cz + dz),
                Chunk {
                    blocks,
                    metadata,
                    biomes: generated.biomes,
                },
            );
        }
    }

    let block_x = cx * 16;
    let block_z = cz * 16;
    let def = biome::biome(world.get_biome_id(block_x + 16, block_z + 16));
    let mut random = LegacyRandom::new(seed);
    random.set_seed(seed);
    let half_a = random.next_long() / 2 * 2 + 1;
    let half_b = random.next_long() / 2 * 2 + 1;
    random.set_seed(
        i64::from(cx)
            .wrapping_mul(half_a)
            .wrapping_add(i64::from(cz).wrapping_mul(half_b))
            ^ seed,
    );

    if a.get(4).is_some_and(|v| v == "heights") {
        let mut out = Vec::with_capacity(256);
        for dz in 8..24 {
            for dx in 8..24 {
                out.push(world.get_height(block_x + dx, block_z + dz).to_string());
            }
        }
        println!("HEIGHTS {}", out.join(","));
        return;
    }

    println!("start {}", hash(&world, cx, cz));

    if def.id != 2 && def.id != 17 && random.next_int_bound(4).unwrap() == 0 {
        let x = block_x + random.next_int_bound(16).unwrap() + 8;
        let y = random.next_int_bound(256).unwrap();
        let z = block_z + random.next_int_bound(16).unwrap() + 8;
        LakeFeature::water().place(&mut world, &mut random, x, y, z);
    }
    println!("water {}", hash(&world, cx, cz));

    if random.next_int_bound(8).unwrap() == 0 {
        let x = block_x + random.next_int_bound(16).unwrap() + 8;
        let outer = random.next_int_bound(248).unwrap() + 8;
        let y = random.next_int_bound(outer).unwrap();
        let z = block_z + random.next_int_bound(16).unwrap() + 8;
        if y < 63 || random.next_int_bound(10).unwrap() == 0 {
            LakeFeature::lava().place(&mut world, &mut random, x, y, z);
        }
    }
    println!("lava {}", hash(&world, cx, cz));

    let mut dungeons = DungeonFeature::new();
    for _ in 0..8 {
        let x = block_x + random.next_int_bound(16).unwrap() + 8;
        let y = random.next_int_bound(256).unwrap();
        let z = block_z + random.next_int_bound(16).unwrap() + 8;
        dungeons.place(&mut world, &mut random, x, y, z);
    }
    println!("dungeon {}", hash(&world, cx, cz));

    if a.get(4).is_some_and(|v| v == "rng") {
        println!("predecorate {} {}", random.next_long(), random.next_long());
        return;
    }
    if let Some(rest) = a.get(4).and_then(|v| v.strip_prefix("stage:")) {
        let stage: usize = rest.parse().unwrap();
        eprintln!(
            "biome {} trees={} grass={} flowers={} sand={} trees_policy={:?}",
            def.id,
            def.config.tree_attempts,
            def.config.grass_attempts,
            def.config.flower_attempts,
            def.config.sand_patch_attempts,
            def.tree_policy
        );
        // Java marks 11 as "both sugarcane loops"; from 12 on the Rust pass
        // numbering has one more entry.
        let passes = match stage {
            0..=10 => stage + 1,
            11 => 13,
            _ => stage + 2,
        };
        let mut decorator = FeatureDecorator::new();
        decorator.decorate_upto(&mut world, &mut random, &def, block_x, block_z, passes);
        if stage == 15 {
            for lx in 0..16 {
                for lz in 0..16 {
                    let x = block_x + 8 + lx;
                    let z = block_z + 8 + lz;
                    let surface = world.get_precipitation_height(x, z);
                    if world.can_freeze(x, surface - 1, z) {
                        world.set_block_with_metadata(
                            x,
                            surface - 1,
                            z,
                            Block::Ice,
                            0,
                            FLAG_NO_NEIGHBOR_NOTIFY,
                        );
                    }
                    if world.can_snow(x, surface, z) {
                        world.set_block_with_metadata(
                            x,
                            surface,
                            z,
                            Block::SnowLayer,
                            0,
                            FLAG_NO_NEIGHBOR_NOTIFY,
                        );
                    }
                }
            }
        }
        println!("{} {} {}", stage, random.next_long(), random.next_long());
        return;
    }
    if let Some(rest) = a.get(4).and_then(|v| v.strip_prefix("blocks:")) {
        let upto: usize = rest.parse().unwrap();
        let mut decorator = FeatureDecorator::new();
        decorator.decorate_upto(
            &mut world,
            &mut random,
            &def,
            block_x,
            block_z,
            match upto {
                0..=10 => upto + 1,
                11 => 13,
                _ => upto + 2,
            },
        );
        let chunk = world.chunks.get(&(cx, cz)).unwrap();
        let mut out = Vec::with_capacity(65536);
        for y in 0..256i32 {
            for z in 0..16i32 {
                for x in 0..16i32 {
                    let i = chunk_index(x, y, z);
                    out.push(format!("{}:{}", chunk.blocks[i], chunk.metadata[i]));
                }
            }
        }
        println!("BLOCKS {}", out.join(","));
        return;
    }
    FeatureDecorator::new().decorate(&mut world, &mut random, &def, block_x, block_z);
    println!("decorate {}", hash(&world, cx, cz));
    println!("rngafter {} {}", random.next_long(), random.next_long());
    if a.get(4).is_some_and(|v| v == "blocks") {
        let chunk = world.chunks.get(&(cx, cz)).unwrap();
        let mut out = Vec::with_capacity(65536);
        for y in 0..256i32 {
            for z in 0..16i32 {
                for x in 0..16i32 {
                    let i = chunk_index(x, y, z);
                    out.push(format!("{}:{}", chunk.blocks[i], chunk.metadata[i]));
                }
            }
        }
        println!("BLOCKS {}", out.join(","));
        return;
    }

    for lx in 0..16 {
        for lz in 0..16 {
            let x = block_x + 8 + lx;
            let z = block_z + 8 + lz;
            let surface = world.get_precipitation_height(x, z);
            if world.can_freeze(x, surface - 1, z) {
                world.set_block_with_metadata(
                    x,
                    surface - 1,
                    z,
                    Block::Ice,
                    0,
                    FLAG_NO_NEIGHBOR_NOTIFY,
                );
            }
            if world.can_snow(x, surface, z) {
                world.set_block_with_metadata(
                    x,
                    surface,
                    z,
                    Block::SnowLayer,
                    0,
                    FLAG_NO_NEIGHBOR_NOTIFY,
                );
            }
        }
    }
    println!("snow {}", hash(&world, cx, cz));
}
