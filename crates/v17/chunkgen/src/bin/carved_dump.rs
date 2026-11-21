use sunset_1_7_carver::{BiomeSurface, CaveWorldCarver, RavineWorldCarver};
use sunset_1_7_chunkgen::{GeneratorType, OverworldChunkGenerator, biome_facts};

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
    let mut g = OverworldChunkGenerator::new(seed, GeneratorType::Default);
    let chunk = g.get_chunk(cx, cz);
    let mut blocks = chunk.blocks;
    let surface = Surface {
        biomes: chunk.biomes,
    };
    CaveWorldCarver::default().place(&surface, seed, cx, cz, &mut blocks);
    RavineWorldCarver::default().place(&surface, seed, cx, cz, &mut blocks);
    let mut out = vec![0u16; 65536];
    for x in 0..16usize {
        for z in 0..16usize {
            for y in 0..256usize {
                out[(y << 8) | (z << 4) | x] = blocks[(x << 12) | (z << 8) | y];
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
