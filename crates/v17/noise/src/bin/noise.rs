//! Deterministic noise probe: a fixed seed drives every generator and the
//! sampled regions print in the `bin/noise.rs` oracle format — one name line,
//! then the values space-separated — for later parity diffs against the
//! 1.7.10 Java side.

use sunset_1_7_core::LegacyRandom;
use sunset_1_7_noise::{PerlinNoise, PerlinSimplexNoise, SimplexNoise};

fn print_region(name: &str, values: &[f64]) {
    println!("{name}");
    let line: Vec<String> = values.iter().map(|value| format!("{value}")).collect();
    println!("{}", line.join(" "));
}

fn main() {
    let seed = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(12345);
    let mut random = LegacyRandom::new(seed);
    let perlin16 = PerlinNoise::new(&mut random, 16);
    let perlin8 = PerlinNoise::new(&mut random, 8);
    let perlin_simplex = PerlinSimplexNoise::new(&mut random, 4);
    let simplex = SimplexNoise::new(&mut random);

    println!(
        "offsets {} {} {}",
        simplex.offset_x, simplex.offset_y, simplex.offset_z
    );

    // The OverworldChunkGenerator's depth noise, through the 8-arg 2D call.
    let mut region = Vec::new();
    perlin16.get_region_2d(&mut region, 0, 0, 4, 4, 200.0, 200.0, 0.5);
    print_region("perlin16-2d", &region);

    // The min/max limit region, the full 3D call.
    let mut region = Vec::new();
    perlin16.get_region(&mut region, 0, 0, 0, 2, 3, 2, 684.412, 684.412, 684.412);
    print_region("perlin16-3d", &region);

    // The surface noise, the full 3D call with the chunk generator's scales.
    let mut region = Vec::new();
    perlin8.get_region(
        &mut region,
        -3,
        0,
        5,
        2,
        3,
        2,
        8.555150000000001,
        4.277575000000001,
        8.555150000000001,
    );
    print_region("perlin8-3d", &region);

    // The perlinNoise3 decoration noise, through the 8-arg 2D call.
    let mut region = vec![0.0; 64];
    simplex.add(&mut region, 0.0, 0.0, 8, 8, 1.0, 1.0, 1.0);
    print_region("simplex-add", &region);

    let mut region = Vec::new();
    perlin_simplex.get_region(&mut region, 0.0, 0.0, 8, 8, 0.03125, 0.03125, 1.0);
    print_region("perlin-simplex", &region);

    let line: Vec<String> = (0..16)
        .map(|step| {
            let x = step as f64 * 0.137 - 1.0;
            format!("{}", simplex.get_value(x, x * 1.7 - 0.3))
        })
        .collect();
    println!("simplex-value");
    println!("{}", line.join(" "));

    let line: Vec<String> = (0..16)
        .map(|step| {
            let x = step as f64 * 0.137 - 1.0;
            format!("{}", perlin_simplex.get_value(x, x * 1.7 - 0.3))
        })
        .collect();
    println!("perlin-simplex-value");
    println!("{}", line.join(" "));
}
