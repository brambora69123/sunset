//! Liquid parity probe: builds one of the 1.7.10 layouts, places the liquids,
//! ticks the scheduled queue and prints the blocks that changed (plus every
//! liquid), in save order, and the world random's state.
//!
//! The Java counterpart is `tools/javaoracle_1_7_10/LiquidProbe.java`; run both
//! with the layout name and diff the output (`tools/liquids_parity_1_7_10.py`).
//!
//! Layouts: `flat`, `hole`, `two_sources`, `water_lava`, `lava_over_water`,
//! `cave`, `run15`.

use std::process::ExitCode;

use sunset_1_7_liquids::fixture::TestWorld;
use sunset_1_7_liquids::{AIR, LAVA, LiquidWorld, STONE, WATER};

const SEED: i64 = 1234;
const SPAN: i32 = 16;
const HEIGHT: i32 = 32;

fn main() -> ExitCode {
    let name = match std::env::args().nth(1) {
        Some(name) => name,
        None => {
            eprintln!(
                "usage: liquid_probe <flat|hole|two_sources|water_lava|lava_over_water|cave|run15>"
            );
            return ExitCode::FAILURE;
        }
    };

    let (mut world, kicks, ticks) = setup(&name);
    let before = snapshot(&world);

    for (x, y, z) in kicks {
        world.kick(x, y, z);
    }
    world.tick_n(ticks);

    for y in 0..HEIGHT {
        for z in 0..SPAN {
            for x in 0..SPAN {
                let slot = slot(x, y, z);
                let after = (world.get_block(x, y, z), world.get_block_metadata(x, y, z));
                let initial = before[slot];
                if after != initial || is_liquid(after.0) {
                    println!("{} {} {}", index(x, y, z), id(after.0), after.1);
                }
            }
        }
    }
    println!("rng {}", world.random_state());
    ExitCode::SUCCESS
}

fn slot(x: i32, y: i32, z: i32) -> usize {
    ((y * SPAN + z) * SPAN + x) as usize
}

fn index(x: i32, y: i32, z: i32) -> i32 {
    y << 8 | z << 4 | x
}

fn id(block: sunset_1_7_liquids::Block) -> u16 {
    block.id
}

fn is_liquid(block: sunset_1_7_liquids::Block) -> bool {
    matches!(block.id, 8 | 9 | 10 | 11)
}

fn snapshot(world: &TestWorld) -> Vec<(sunset_1_7_liquids::Block, i32)> {
    let mut cells = Vec::with_capacity((SPAN * SPAN * HEIGHT) as usize);
    for y in 0..HEIGHT {
        for z in 0..SPAN {
            for x in 0..SPAN {
                cells.push((world.get_block(x, y, z), world.get_block_metadata(x, y, z)));
            }
        }
    }
    cells
}

fn setup(name: &str) -> (TestWorld, Vec<(i32, i32, i32)>, usize) {
    let mut world = TestWorld::new(SEED);
    match name {
        "flat" => {
            world.fill(0, 0, 0, 15, 4, 15, STONE, 0);
            world.place(8, 5, 8, WATER, 0, 3);
            (world, vec![(8, 5, 8)], 80)
        }
        "hole" => {
            world.fill(0, 0, 0, 15, 4, 15, STONE, 0);
            world.fill(8, 1, 8, 8, 4, 8, AIR, 0);
            world.place(8, 6, 8, WATER, 0, 3);
            (world, vec![(8, 6, 8)], 80)
        }
        "two_sources" => {
            world.fill(0, 0, 0, 15, 4, 15, STONE, 0);
            world.place(4, 5, 8, WATER, 0, 3);
            world.place(12, 5, 8, WATER, 0, 3);
            (world, vec![(4, 5, 8), (12, 5, 8)], 100)
        }
        "water_lava" => {
            // Three sealed chambers over one stone floor.
            world.fill(0, 0, 0, 15, 9, 15, STONE, 0);
            world.fill(1, 5, 1, 5, 9, 5, AIR, 0); // obsidian station
            world.fill(10, 5, 1, 14, 9, 5, AIR, 0); // cobblestone station
            world.fill(1, 5, 10, 5, 9, 14, AIR, 0); // stone station
            world.set_direct(2, 4, 11, AIR, 0);
            world.place(2, 5, 2, LAVA, 0, 3);
            world.place(2, 5, 3, WATER, 0, 3);
            world.place(11, 5, 2, LAVA, 2, 3);
            world.place(11, 5, 3, WATER, 0, 3);
            world.place(2, 4, 11, WATER, 0, 3);
            world.place(2, 5, 11, LAVA, 0, 3);
            (world, vec![(2, 5, 11)], 120)
        }
        "lava_over_water" => {
            world.fill(0, 0, 0, 15, 4, 15, STONE, 0);
            world.place(8, 5, 8, WATER, 0, 3);
            world.place(8, 6, 8, LAVA, 0, 3);
            (world, vec![(8, 6, 8)], 80)
        }
        "cave" => {
            world.fill(0, 0, 0, 15, 4, 15, STONE, 0);
            world.fill(4, 1, 4, 11, 3, 11, AIR, 0);
            world.set_direct(8, 4, 8, AIR, 0);
            world.place(8, 6, 8, WATER, 0, 3);
            (world, vec![(8, 6, 8)], 120)
        }
        "run15" => {
            world.fill(0, 0, 0, 15, 4, 15, STONE, 0);
            for x in 0..=15 {
                world.set_direct(x, 5, 7, STONE, 0);
                world.set_direct(x, 5, 9, STONE, 0);
            }
            world.set_direct(0, 5, 8, STONE, 0);
            world.place(1, 5, 8, WATER, 0, 3);
            (world, vec![(1, 5, 8)], 80)
        }
        other => {
            panic!("unknown layout {other}");
        }
    }
}
