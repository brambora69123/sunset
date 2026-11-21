use sunset_1_7_core::{LegacyRandom, RandomSource};
use sunset_1_7_features::block::Block;
use sunset_1_7_features::features::vein::VeinFeature;
use sunset_1_7_features::world::GenWorld;

struct World {
    blocks: [u16; 65536],
}

impl World {
    fn idx(x: i32, y: i32, z: i32) -> usize {
        (((x & 15) as usize) << 12) | (((z & 15) as usize) << 8) | (y as usize & 255)
    }
}

impl GenWorld for World {
    fn get_block(&self, x: i32, y: i32, z: i32) -> Block {
        if !(0..256).contains(&y) {
            return Block::Air;
        }
        Block::from_id(self.blocks[Self::idx(x, y, z)] as u8)
    }
    fn get_block_metadata(&self, _x: i32, _y: i32, _z: i32) -> u8 {
        0
    }
    fn set_block_with_metadata(&mut self, x: i32, y: i32, z: i32, block: Block, _m: u8, _f: u8) {
        if (0..256).contains(&y) {
            let i = Self::idx(x, y, z);
            self.blocks[i] = block.id() as u16;
        }
    }
    fn get_biome_id(&self, _x: i32, _z: i32) -> i32 {
        12
    }
    fn get_height(&self, _x: i32, _z: i32) -> i32 {
        0
    }
    fn get_surface_height(&self, _x: i32, _z: i32) -> i32 {
        0
    }
    fn get_precipitation_height(&self, _x: i32, _z: i32) -> i32 {
        0
    }
}

fn main() {
    let seed: i64 = std::env::args().nth(1).unwrap().parse().unwrap();
    let mut world = World { blocks: [1; 65536] };
    let mut random = LegacyRandom::new(seed);
    let vein = VeinFeature::new(Block::Dirt, 32);
    for _ in 0..6 {
        let x = random.next_int_bound(16).unwrap();
        let y = random.next_int_bound(256).unwrap();
        let z = random.next_int_bound(16).unwrap();
        vein.place(&mut world, &mut random, x, y, z);
    }
    let mut hist = std::collections::BTreeMap::new();
    for b in world.blocks.iter() {
        *hist.entry(*b).or_insert(0) += 1;
    }
    eprintln!("hist={:?}", hist);
    let mut out = Vec::new();
    for y in 0..256i32 {
        for z in 0..16i32 {
            for x in 0..16i32 {
                if world.blocks[World::idx(x, y, z)] == 3 {
                    out.push((y * 256 + z * 16 + x).to_string());
                }
            }
        }
    }
    println!("{}", out.join(","));
}
