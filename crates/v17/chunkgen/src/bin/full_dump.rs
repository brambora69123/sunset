use sunset_1_7_carver::{BiomeSurface, CaveWorldCarver, RavineWorldCarver};
use sunset_1_7_chunkgen::{GeneratorType, OverworldChunkGenerator, biome_facts};
use sunset_1_7_features::block::Block;
use sunset_1_7_features::populate::populate_chunk;
use sunset_1_7_features::world::GenWorld;

struct Surface {
    biomes: [u8; 256],
}

impl BiomeSurface for Surface {
    fn surface_block(&self, x: i32, z: i32) -> u16 {
        let id = self.biomes[((z & 15) * 16 + (x & 15)) as usize] as i32;
        biome_facts::surface_blocks(id).0
    }
}

struct ChunkWorld {
    blocks: Vec<u16>,
    biomes: [u8; 256],
}

impl ChunkWorld {
    fn index(x: i32, y: i32, z: i32) -> usize {
        (((x & 15) as usize) << 12) | (((z & 15) as usize) << 8) | (y as usize & 255)
    }
}

impl GenWorld for ChunkWorld {
    fn get_block(&self, x: i32, y: i32, z: i32) -> Block {
        if !(0..256).contains(&y) {
            return Block::Air;
        }
        Block::from_id(self.blocks[Self::index(x, y, z)] as u8)
    }

    fn get_block_metadata(&self, _x: i32, _y: i32, _z: i32) -> u8 {
        0
    }

    fn set_block_with_metadata(
        &mut self,
        x: i32,
        y: i32,
        z: i32,
        block: Block,
        _metadata: u8,
        _flags: u8,
    ) {
        if (0..256).contains(&y) {
            let i = Self::index(x, y, z);
            self.blocks[i] = block.id() as u16;
        }
    }

    fn get_biome_id(&self, x: i32, z: i32) -> i32 {
        self.biomes[((z & 15) * 16 + (x & 15)) as usize] as i32
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

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let seed: i64 = a[1].parse().unwrap();
    let cx: i32 = a[2].parse().unwrap();
    let cz: i32 = a[3].parse().unwrap();
    let mut g = OverworldChunkGenerator::new(seed, GeneratorType::Default);
    let chunk = g.get_chunk(cx, cz);
    let mut blocks = chunk.blocks;
    let surface = Surface {
        biomes: chunk.biomes,
    };
    CaveWorldCarver::default().place(&surface, seed, cx, cz, &mut blocks);
    RavineWorldCarver::default().place(&surface, seed, cx, cz, &mut blocks);
    let mut world = ChunkWorld {
        blocks,
        biomes: chunk.biomes,
    };
    populate_chunk(&mut world, seed, cx, cz, None);
    let mut out = vec![0u16; 65536];
    for x in 0..16usize {
        for z in 0..16usize {
            for y in 0..256usize {
                out[(y << 8) | (z << 4) | x] = world.blocks[(x << 12) | (z << 8) | y];
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
