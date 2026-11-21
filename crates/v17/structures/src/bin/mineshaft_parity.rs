use sunset_1_7_core::{LegacyRandom, RandomSource};
use sunset_1_7_structures::mineshaft::MineshaftStructure;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let seed: i64 = a[1].parse().unwrap();
    let r: i32 = a[2].parse().unwrap();
    let longs: bool = a.len() > 3;
    let s = MineshaftStructure::new();
    let mut rand = LegacyRandom::new(seed);
    if longs {
        let _ = rand.next_long();
        let _ = rand.next_long();
    }
    let mut out = String::new();
    for x in -r..=r {
        for z in -r..=r {
            if s.is_feature_chunk(&mut rand, x, z) {
                out.push_str(&format!("{x},{z};"));
            }
        }
    }
    println!("{out}");
}
