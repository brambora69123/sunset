//! Full world dump: terrain + carving + population over a chunk area, printing
//! one chunk in vanilla's save order so it can be diffed against a real world.
//!
//! Usage: world_dump <seed> <chunkX> <chunkZ> [populate_radius]
//! Generates (radius+1) of terrain around the populated area so features that
//! read across the border see real blocks.

use std::collections::HashMap;

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
    /// Out-of-world answers: air.
    empty_biome: i32,
}

fn chunk_index(x: i32, y: i32, z: i32) -> usize {
    (((x & 15) as usize) << 12) | (((z & 15) as usize) << 8) | (y as usize & 255)
}

impl World {
    fn chunk_at(&self, cx: i32, cz: i32) -> Option<&Chunk> {
        self.chunks.get(&(cx, cz))
    }
}

impl GenWorld for World {
    fn get_block(&self, x: i32, y: i32, z: i32) -> Block {
        if !(0..256).contains(&y) {
            return Block::Air;
        }
        match self.chunk_at(x >> 4, z >> 4) {
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
        _flags: u8,
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
        match self.chunk_at(x >> 4, z >> 4) {
            Some(chunk) => chunk.biomes[((z & 15) * 16 + (x & 15)) as usize] as i32,
            None => self.empty_biome,
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
    let radius: i32 = a.get(4).map_or(1, |v| v.parse().unwrap());

    // Terrain for the populated area plus one ring, so cross-border reads are real.
    let mut generator = OverworldChunkGenerator::new(seed, GeneratorType::Default);
    let mut world = World {
        chunks: HashMap::new(),
        empty_biome: 1,
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

    // Vanilla populates chunks as they load, which around spawn means the
    // centre chunk first and then ring by ring.
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

    let chunk = world.chunk_at(cx, cz).expect("centre chunk");
    let mut out = vec![0u16; 65536];
    for x in 0..16usize {
        for z in 0..16usize {
            for y in 0..256usize {
                out[(y << 8) | (z << 4) | x] = chunk.blocks[(x << 12) | (z << 8) | y];
            }
        }
    }
    println!(
        "{}",
        out.iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(",")
    );
}
