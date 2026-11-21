//! Dumps a populated chunk region, one file per chunk in save order, for
//! `tools/terrain_parity_1_7_10.py --region`.
//!
//! Usage: region_dump <seed> <centreChunkX> <centreChunkZ> <radius> <outDir>

use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;

use sunset_1_7_carver::{BiomeSurface, CaveWorldCarver, RavineWorldCarver};
use sunset_1_7_chunkgen::{GeneratorType, OverworldChunkGenerator, biome_facts};
use sunset_1_7_features::block::Block;
use sunset_1_7_features::populate::populate_chunk;
use sunset_1_7_features::world::GenWorld;

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

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let seed: i64 = a[1].parse().unwrap();
    let cx: i32 = a[2].parse().unwrap();
    let cz: i32 = a[3].parse().unwrap();
    let radius: i32 = a[4].parse().unwrap();
    let out_dir = PathBuf::from(&a[5]);
    std::fs::create_dir_all(&out_dir).unwrap();

    let mut generator = OverworldChunkGenerator::new(seed, GeneratorType::Default);
    let mut world = World {
        chunks: HashMap::new(),
    };
    // One ring of terrain beyond the populated area for cross-border reads.
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

    let mut order: Vec<(i32, i32)> = Vec::new();
    for dz in -radius..=radius {
        for dx in -radius..=radius {
            order.push((dx, dz));
        }
    }
    order.sort_by_key(|&(dx, dz)| (dx.abs().max(dz.abs()), dz, dx));
    for (dx, dz) in order {
        populate_chunk(&mut world, seed, cx + dx, cz + dz, None);
    }

    for dz in -radius..=radius {
        for dx in -radius..=radius {
            let chunk = &world.chunks[&(cx + dx, cz + dz)];
            let mut out = String::with_capacity(65536 * 3);
            for y in 0..256usize {
                for z in 0..16usize {
                    for x in 0..16usize {
                        if !out.is_empty() {
                            out.push(',');
                        }
                        let i = (x << 12) | (z << 8) | y;
                        out.push_str(&format!("{}:{}", chunk.blocks[i], chunk.metadata[i]));
                    }
                }
            }
            let path = out_dir.join(format!("{}_{}.txt", cx + dx, cz + dz));
            let mut f = std::fs::File::create(path).unwrap();
            f.write_all(out.as_bytes()).unwrap();
        }
    }
    println!(
        "dumped {} chunks to {}",
        (2 * radius + 1) * (2 * radius + 1),
        out_dir.display()
    );
}
