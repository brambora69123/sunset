//! One 1.7.10 tree on a flat platform, for diffing against the Java oracle.
//!
//! `tree_parity <tree> <seed> <mode>`; the Java side is
//! `tools/javaoracle_1_7_10/run.sh TreeDump`, same arguments.
//!
//! Modes: `normal` (grass ground), `ground` (stone under the trunk, so the
//! placement fails after its height draw), `canopy` (a stone block inside the
//! clearance volume, so the clearance loop fails).
//!
//! Output is the whole chunk in save order, `index:id:metadata` with
//! `index = y << 8 | z << 4 | x`, after the decorator's `place` and
//! `placeSoil` calls.

use sunset_1_7_core::{LegacyRandom, RandomSource};
use sunset_1_7_features::block::Block;
use sunset_1_7_features::chunk_world::ChunkWorld;
use sunset_1_7_features::features::trees::acacia::AcaciaTreeFeature;
use sunset_1_7_features::features::trees::bush::BushFeature;
use sunset_1_7_features::features::trees::dark_oak::DarkOakTreeFeature;
use sunset_1_7_features::features::trees::giant_jungle::GiantJungleTreeFeature;
use sunset_1_7_features::features::trees::giant_pine::GiantPineTreeFeature;
use sunset_1_7_features::features::trees::giant_spruce::GiantSpruceTreeFeature;
use sunset_1_7_features::features::trees::jungle::JungleTreeFeature;
use sunset_1_7_features::features::trees::tall_birch::TallBirchTreeFeature;
use sunset_1_7_features::world::{FLAG_NO_NEIGHBOR_NOTIFY, GenWorld};

/// The platform's top grass layer; the tree starts at `TOP + 1`.
const TOP: i32 = 63;

fn main() {
    let tree = std::env::args().nth(1).expect("tree name");
    let seed: i64 = std::env::args()
        .nth(2)
        .expect("seed")
        .parse()
        .expect("seed is an i64");
    let mode = std::env::args().nth(3).expect("mode");

    // The giant jungle's branches reach eight blocks out, so it starts further
    // from the chunk edge than the others.
    let origin = if tree == "giant_jungle" { 5 } else { 8 };
    let (x, y, z) = (origin, TOP + 1, origin);

    let mut world = ChunkWorld::new(0, 0);
    world.fill(0, TOP - 1, Block::Dirt);
    world.fill(TOP, TOP, Block::Grass);
    match mode.as_str() {
        "normal" => {}
        "ground" => {
            world.set_block_with_metadata(x, y - 1, z, Block::Stone, 0, FLAG_NO_NEIGHBOR_NOTIFY);
        }
        "canopy" => {
            world.set_block_with_metadata(
                x + 1,
                y + 3,
                z,
                Block::Stone,
                0,
                FLAG_NO_NEIGHBOR_NOTIFY,
            );
        }
        other => panic!("unknown mode {other}"),
    }

    let mut random = LegacyRandom::new(seed);
    let placed = match tree.as_str() {
        "acacia" => AcaciaTreeFeature::new().place(&mut world, &mut random, x, y, z),
        "bush" => BushFeature::new(3, 0).place(&mut world, &mut random, x, y, z),
        "dark_oak" => DarkOakTreeFeature::new().place(&mut world, &mut random, x, y, z),
        "tall_birch" => TallBirchTreeFeature::new().place(&mut world, &mut random, x, y, z),
        "jungle" => {
            // `JungleBiome.pickTree` draws the base height before `place`.
            let base_height = 4 + random.next_int_bound(7).expect("positive bound");
            JungleTreeFeature::new(base_height).place(&mut world, &mut random, x, y, z)
        }
        "giant_jungle" => GiantJungleTreeFeature::new().place(&mut world, &mut random, x, y, z),
        "giant_pine" => {
            let feature = GiantPineTreeFeature::new();
            let placed = feature.place(&mut world, &mut random, x, y, z);
            if placed {
                feature.place_soil(&mut world, &mut random, x, y, z);
            }
            placed
        }
        "giant_spruce" => {
            let feature = GiantSpruceTreeFeature::new(true);
            let placed = feature.place(&mut world, &mut random, x, y, z);
            if placed {
                feature.place_soil(&mut world, &mut random, x, y, z);
            }
            placed
        }
        other => panic!("unknown tree {other}"),
    };

    println!("placed {placed}");
    let mut out = String::new();
    for layer in 0..256 {
        for bz in 0..16 {
            for bx in 0..16 {
                let block = world.get_block(bx, layer, bz);
                if block != Block::Air {
                    out.push_str(&format!(
                        "{}:{}:{}\n",
                        layer * 256 + bz * 16 + bx,
                        block.id(),
                        world.get_block_metadata(bx, layer, bz)
                    ));
                }
            }
        }
    }
    print!("{out}");
}
