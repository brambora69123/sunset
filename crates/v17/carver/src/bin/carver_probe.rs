//! Deterministic carver probe: caves and ravines over fixed chunks for fixed
//! world seeds, printed for later parity against a Java oracle.
//!
//! Run: `cargo +1.99.1 run -p sunset-1-7-carver --bin carver_probe`

use sunset_1_7_carver::{BiomeSurface, CaveWorldCarver, RavineWorldCarver, block_id};

/// Flat-plains stand-in for `World.getBiome(x, z).surfaceBlock`.
struct Plains;
impl BiomeSurface for Plains {
    fn surface_block(&self, _x: i32, _z: i32) -> u16 {
        block_id::GRASS
    }
}

const SIZE: usize = 16 * 16 * 256;
// Seeds 9 and 232 both carve caves and ravines at chunk (0, 0).
const WORLD_SEEDS: [i64; 3] = [0x5EED_1710, 9, 232];
const CHUNKS: [(i32, i32); 3] = [(0, 0), (3, -2), (-7, 11)];

fn checksum(blocks: &[u16]) -> u64 {
    blocks.iter().fold(0u64, |acc, &b| {
        acc.wrapping_mul(31).wrapping_add(u64::from(b))
    })
}

fn counts(blocks: &[u16]) -> (usize, usize, usize) {
    let mut air = 0;
    let mut lava = 0;
    let mut flowing_lava = 0;
    for &b in blocks {
        match b {
            block_id::AIR => air += 1,
            block_id::LAVA => lava += 1,
            block_id::FLOWING_LAVA => flowing_lava += 1,
            _ => {}
        }
    }
    (air, lava, flowing_lava)
}

fn main() {
    for world_seed in WORLD_SEEDS {
        for (chunk_x, chunk_z) in CHUNKS {
            let mut caves = vec![block_id::STONE; SIZE];
            CaveWorldCarver::new().place(&Plains, world_seed, chunk_x, chunk_z, &mut caves);
            let (air, lava, flowing) = counts(&caves);
            println!(
                "caves seed {world_seed} chunk ({chunk_x}, {chunk_z}): air={air} lava={lava} flowing_lava={flowing} checksum={:#018x}",
                checksum(&caves)
            );

            let mut ravines = vec![block_id::STONE; SIZE];
            RavineWorldCarver::new().place(&Plains, world_seed, chunk_x, chunk_z, &mut ravines);
            let (air, lava, flowing) = counts(&ravines);
            println!(
                "ravines seed {world_seed} chunk ({chunk_x}, {chunk_z}): air={air} lava={lava} flowing_lava={flowing} checksum={:#018x}",
                checksum(&ravines)
            );
        }
    }
}
