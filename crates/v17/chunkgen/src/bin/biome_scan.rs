//! Finds chunks of a given biome without generating terrain.
//!
//! Usage: biome_scan <seed> <centreX> <centreZ> <radiusChunks> [--biome ID] [--list N]
//!
//! Prints, for every chunk in the square, `bx bz biome` when `--biome` is
//! given (at most `--list` lines, default 20), else a histogram of biome ids
//! with the first chunk seen for each.

use sunset_1_7::biome;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let seed: i64 = a[1].parse().unwrap();
    let cx: i32 = a[2].parse().unwrap();
    let cz: i32 = a[3].parse().unwrap();
    let radius: i32 = a[4].parse().unwrap();
    let want: Option<i32> = a
        .iter()
        .position(|v| v == "--biome")
        .and_then(|i| a.get(i + 1)?.parse().ok());
    let list: usize = a
        .iter()
        .position(|v| v == "--list")
        .and_then(|i| a.get(i + 1)?.parse().ok())
        .unwrap_or(20);

    let [_, mut biomes, _] = biome::init(seed, false);
    let mut found = 0usize;
    let mut seen: std::collections::BTreeMap<i32, (i32, i32, u32)> =
        std::collections::BTreeMap::new();
    for bz in (cz - radius)..=(cz + radius) {
        for bx in (cx - radius)..=(cx + radius) {
            // The decorator reads the biome at the chunk's centre.
            let id = biomes.area(bx * 16 + 16, bz * 16 + 16, 1, 1)[0];
            if let Some(want) = want {
                if id == want {
                    println!("{bx} {bz} {id}");
                    found += 1;
                    if found >= list {
                        return;
                    }
                }
            } else {
                let entry = seen.entry(id).or_insert((bx, bz, 0));
                entry.2 += 1;
            }
        }
    }
    if want.is_none() {
        for (id, (bx, bz, count)) in seen {
            if id >= 128 {
                println!("{id} {count} {bx} {bz}");
            }
        }
    }
}
