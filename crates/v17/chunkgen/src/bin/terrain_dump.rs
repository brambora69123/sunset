use sunset_1_7_chunkgen::{GeneratorType, OverworldChunkGenerator};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let seed: i64 = a[1].parse().unwrap();
    let cx: i32 = a[2].parse().unwrap();
    let cz: i32 = a[3].parse().unwrap();
    let mut g = OverworldChunkGenerator::new(seed, GeneratorType::Default);
    let chunk = g.get_chunk(cx, cz);
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
    eprintln!(
        "biomes={}",
        chunk
            .biomes
            .iter()
            .map(|b| b.to_string())
            .collect::<Vec<_>>()
            .join(",")
    );
}
