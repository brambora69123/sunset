//! Deterministic probe for later parity checks: generates a few overworld
//! chunks and prints checksums, the biome map and surface samples.

use sunset_1_7_chunkgen::{GeneratorType, OverworldChunkGenerator, STONE, WATER};

fn main() {
    for seed in [0i64, 123_456_789] {
        for (chunk_x, chunk_z, kind) in [
            (0, 0, GeneratorType::Default),
            (3, -2, GeneratorType::Default),
            (-17, 41, GeneratorType::Default),
            (11, -35, GeneratorType::Amplified),
        ] {
            let mut generator = OverworldChunkGenerator::new(seed, kind);
            let chunk = generator.get_chunk(chunk_x, chunk_z);

            let checksum: u64 = chunk.blocks.iter().map(|&id| id as u64).sum();
            let stone = chunk.blocks.iter().filter(|&&id| id == STONE).count();
            let water = chunk.blocks.iter().filter(|&&id| id == WATER).count();
            println!(
                "RUST seed={seed} kind={kind:?} chunk=({chunk_x},{chunk_z}) checksum={checksum} stone={stone} water={water}"
            );

            let biomes: Vec<String> = chunk.biomes.iter().map(|&id| id.to_string()).collect();
            println!(
                "RUST seed={seed} chunk=({chunk_x},{chunk_z}) biomes={}",
                biomes.join(",")
            );

            for (x, z) in [(8i32, 8i32), (0, 15), (15, 0)] {
                let index = ((x << 12) | (z << 8)) as usize;
                let mut y = 255;
                while y > 0 && chunk.blocks[index + y as usize] == 0 {
                    y -= 1;
                }
                println!(
                    "RUST seed={seed} kind={kind:?} chunk=({chunk_x},{chunk_z}) surface=({x},{z}) y={y} block={}",
                    chunk.blocks[index + y as usize]
                );
            }
        }
    }
}
