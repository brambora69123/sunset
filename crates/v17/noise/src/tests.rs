//! Invariant tests for the 1.7.10 noise port: the 2D/3D fast paths agreeing,
//! buffer reuse, octave ranges, and golden values run through the real
//! decompiled 1.7.10 classes (see the oracle harness when it lands).

use crate::{ImprovedNoise, PerlinNoise, PerlinSimplexNoise, SimplexNoise, lfloor};
use sunset_1_7_core::LegacyRandom;

#[test]
fn lfloor_matches_java_floor() {
    assert_eq!(lfloor(-0.5), -1);
    assert_eq!(lfloor(-1.0), -1);
    assert_eq!(lfloor(-1.25), -2);
    assert_eq!(lfloor(0.0), 0);
    assert_eq!(lfloor(3.75), 3);
    assert_eq!(lfloor(-1.0e18), -1_000_000_000_000_000_000);
}

#[test]
fn improved_2d_matches_3d_single_layer() {
    // The sizeY == 1 path only skips the y interpolation; with one layer both
    // must agree to the last bit.
    let noise = ImprovedNoise::new(&mut LegacyRandom::new(7));
    let mut fast = vec![0.0; 12];
    noise.add(
        &mut fast,
        -25.0,
        0.0,
        75.0,
        4,
        1,
        3,
        1.0 / 6.0,
        1.0,
        1.0 / 6.0,
        0.5,
    );
    let mut slow = vec![0.0; 12];
    noise.add(
        &mut slow,
        -25.0,
        5.0,
        75.0,
        4,
        1,
        3,
        1.0 / 6.0,
        1.0,
        1.0 / 6.0,
        0.5,
    );
    assert_eq!(fast, slow);
}

#[test]
fn perlin_zeroes_reused_buffer() {
    let noise = PerlinNoise::new(&mut LegacyRandom::new(9), 2);
    let mut buffer = vec![1.5f64; 64];
    noise.get_region(&mut buffer, 0, 0, 0, 4, 4, 4, 1.0, 1.0, 1.0);
    // The region is additive; a stale buffer must not leak its old values in.
    let mut fresh = vec![1.5f64; 64];
    noise.get_region(&mut fresh, 0, 0, 0, 4, 4, 4, 1.0, 1.0, 1.0);
    assert_eq!(buffer, fresh);
    // A smaller region reuses the same buffer, zeroed whole as 1.7.10 does.
    noise.get_region(&mut buffer, 0, 0, 0, 2, 2, 2, 1.0, 1.0, 1.0);
    let mut fresh = vec![1.5f64; 64];
    noise.get_region(&mut fresh, 0, 0, 0, 2, 2, 2, 1.0, 1.0, 1.0);
    assert_eq!(buffer, fresh);
    assert_eq!(buffer.len(), 64);
    assert!(buffer[8..].iter().all(|value| *value == 0.0));
}

#[test]
fn simplex_add_cell_matches_point_sample() {
    // `add` at unit scale is the point sample of each cell plus the offsets,
    // so the two paths must agree to the last bit.
    let noise = SimplexNoise::new(&mut LegacyRandom::new(41));
    let mut region = vec![0.0; 16];
    noise.add(&mut region, 2.0, -1.0, 4, 4, 1.0, 1.0, 1.0);
    for cell_y in 0..4i32 {
        for cell_x in 0..4i32 {
            let sample_x = (2.0 + cell_x as f64) * 1.0 + noise.offset_x;
            let sample_y = (-1.0 + cell_y as f64) * 1.0 + noise.offset_y;
            assert_eq!(
                region[(cell_x + cell_y * 4) as usize],
                noise.get_value(sample_x, sample_y),
                "cell ({cell_x}, {cell_y})"
            );
        }
    }
}

#[test]
fn perlin_same_seed_same_region() {
    let mut buffer = Vec::new();
    PerlinNoise::new(&mut LegacyRandom::new(63), 16).get_region(
        &mut buffer,
        -2,
        0,
        3,
        3,
        4,
        3,
        684.412,
        684.412,
        684.412,
    );
    let mut again = Vec::new();
    PerlinNoise::new(&mut LegacyRandom::new(63), 16)
        .get_region(&mut again, -2, 0, 3, 3, 4, 3, 684.412, 684.412, 684.412);
    assert_eq!(buffer, again);
}

#[test]
fn one_octave_region_equals_improved_add() {
    // getRegion with one octave must reach the same ImprovedNoise add the
    // 1.7.10 loop makes: the y layer sits at 10 * 1.0 and the octave amplitude
    // starts at 1.0.
    let mut region = Vec::new();
    PerlinNoise::new(&mut LegacyRandom::new(31337), 1).get_region_2d(
        &mut region,
        -2,
        3,
        4,
        5,
        200.0,
        200.0,
        0.5,
    );
    let mut expected = vec![0.0; 20];
    ImprovedNoise::new(&mut LegacyRandom::new(31337)).add(
        &mut expected,
        -2.0 * 200.0,
        10.0,
        3.0 * 200.0,
        4,
        1,
        5,
        200.0,
        1.0,
        200.0,
        1.0,
    );
    assert_eq!(region, expected);
}

#[test]
fn simplex_stays_in_unit_range() {
    let noise = SimplexNoise::new(&mut LegacyRandom::new(41));
    let mut region = vec![0.0; 4096];
    noise.add(&mut region, -32.0, -32.0, 64, 64, 1.0, 1.0, 1.0);
    assert!(region.iter().all(|value| *value <= 1.0), "range");
}

#[test]
fn perlin_simplex_exponents_match_point_sample() {
    // The 8-arg getRegion delegates with the y exponent fixed at 0.5, so both
    // forms must agree at the region's first cell.
    let noise = PerlinSimplexNoise::new(&mut LegacyRandom::new(64), 3);
    let mut region = Vec::new();
    noise.get_region(&mut region, 0.5, 0.5, 1, 1, 0.1, 0.1, 0.8);
    let mut with_exponents = Vec::new();
    noise.get_region_exponents(&mut with_exponents, 0.5, 0.5, 1, 1, 0.1, 0.1, 0.8, 0.5);
    assert_eq!(region, with_exponents);
}

#[test]
fn matches_java_oracle_regions() {
    // The regions of `bin/noise.rs`, sampled through the real decompiled
    // 1.7.10 classes under a JVM (seed 12345). The init draws, lerp/fade math
    // and the `1 / noiseScale` accumulate mirror that code line for line.
    let oracle_perlin16_2d = [
        7323.424576702614,
        7570.585664771817,
        7962.33636733343,
        8066.98719492514,
        7298.253441282997,
        7792.0150381447,
        7990.306421258932,
        7799.974407054204,
        7356.503180821423,
        7719.660214991356,
        7769.188975596466,
        7745.163348309081,
        7548.295817894295,
        7762.593470181612,
        7706.667193508742,
        8058.0894887367285,
    ];
    let oracle_simplex_value = [
        -0.23526496123584156,
        0.259710970557841,
        0.3963783832125781,
        0.5866325940336317,
        0.24985548633967206,
        0.07459370030816952,
        -0.09188865665137026,
        -0.45272920871574424,
        -0.14171715988489475,
        0.8331706078775927,
        0.13637559176421743,
        0.2303730809140015,
        0.42600806661461443,
        0.3273900609397604,
        0.8771871472092005,
        0.8169938527475432,
    ];
    let oracle_perlin_simplex_value = [
        3.1150217598015177,
        1.5070964263663327,
        0.7407506306539955,
        -0.08822247761215207,
        -0.4294179662667741,
        0.2699055942503539,
        0.9789369602684076,
        1.3809058539854315,
        0.22832386102653357,
        -0.9990653071667113,
        -1.564602825390696,
        -2.138326688161266,
        -2.581965610206103,
        -3.6694168582615054,
        -3.8832356154005283,
        -3.0708571405479503,
    ];

    let mut region = Vec::new();
    PerlinNoise::new(&mut LegacyRandom::new(12345), 16).get_region_2d(
        &mut region,
        0,
        0,
        4,
        4,
        200.0,
        200.0,
        0.5,
    );
    for (got, want) in region.iter().zip(oracle_perlin16_2d) {
        assert_eq!(*got, want, "depth noise sample");
    }

    // Every generator rides one stream in the oracle's construction order:
    // both PerlinNoise stacks, then the PerlinSimplexNoise, then the
    // SimplexNoise, each drawing its own table from what is left.
    let mut random = LegacyRandom::new(12345);
    let _ = PerlinNoise::new(&mut random, 16);
    let _ = PerlinNoise::new(&mut random, 8);
    let perlin_simplex = PerlinSimplexNoise::new(&mut random, 4);
    let simplex = SimplexNoise::new(&mut random);
    for (step, want) in oracle_simplex_value.iter().enumerate() {
        let x = step as f64 * 0.137 - 1.0;
        assert_eq!(simplex.get_value(x, x * 1.7 - 0.3), *want);
    }

    for (step, want) in oracle_perlin_simplex_value.iter().enumerate() {
        let x = step as f64 * 0.137 - 1.0;
        assert_eq!(perlin_simplex.get_value(x, x * 1.7 - 0.3), *want);
    }
}
