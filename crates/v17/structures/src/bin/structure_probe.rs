//! Deterministic structure-start dump for parity checks: the starts the four
//! 1.7.10 features place around a world seed.
//!
//! Usage: `structure_probe [seed] [half-extent-chunks]`
//!
//! The neighborhood scan makes each start chunk's check deterministic no
//! matter which target chunk triggers it, so the probe seeds the RNG the
//! MapGenBase way for every chunk and checks it once — the net effect of
//! vanilla's `Generator.place` over every target chunk.

use sunset_1_7_core::{LegacyRandom, RandomSource};
use sunset_1_7_structures::{
    MineshaftStructure, StrongholdStructure, StructureFeature, StructureGen, TempleStructure,
    VillageStructure, chunk_pos_to_long,
};

fn dump(name: &str, structure: &mut impl StructureFeature, world: &mut StructureGen, half: i32) {
    let mut random = LegacyRandom::new(0);
    random.set_seed(world.seed);
    let a = random.next_long();
    let b = random.next_long();
    let mut starts = Vec::new();
    for x in -half..=half {
        for z in -half..=half {
            random.set_seed((x as i64).wrapping_mul(a) ^ (z as i64).wrapping_mul(b) ^ world.seed);
            if structure.prepare_start(world, &mut random, x, z) {
                let key = chunk_pos_to_long(x, z);
                let start = &structure.structures()[&key];
                starts.push((x, z, start.bounds));
            }
        }
    }
    println!("{name} {} starts", starts.len());
    for (x, z, bounds) in starts {
        println!(
            "  chunk ({x}, {z}) bb ({}, {}, {}; {}, {}, {})",
            bounds.min_x, bounds.min_y, bounds.min_z, bounds.max_x, bounds.max_y, bounds.max_z
        );
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let seed: i64 = args
        .get(1)
        .map(|s| s.parse().expect("seed"))
        .unwrap_or(-9069452278850244);
    let half: i32 = args
        .get(2)
        .map(|s| s.parse().expect("half-extent"))
        .unwrap_or(96);
    println!("seed {seed} scan -{half}..={half}");
    let mut world = StructureGen::new(seed, false);

    let mut village = VillageStructure::new();
    dump("Village", &mut village, &mut world, half);
    let mut stronghold = StrongholdStructure::new();
    dump("Stronghold", &mut stronghold, &mut world, half);
    println!("  Stronghold ring positions: {:?}", stronghold.positions);
    println!(
        "  Stronghold potential positions: {:?}",
        stronghold.potential_positions()
    );
    let mut mineshaft = MineshaftStructure::new();
    dump("Mineshaft", &mut mineshaft, &mut world, half);
    let mut temple = TempleStructure::new();
    dump("Temple", &mut temple, &mut world, half);
}
