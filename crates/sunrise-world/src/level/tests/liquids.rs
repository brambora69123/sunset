//! 1.7.10 liquid parity: the same scenarios `tools/javaoracle_1_7_10/
//! FluidOracle.java` lays out, run through the engine and compared with the
//! real game's block dumps.
//!
//! The fixture is a 1.7.10-shaped registry built in memory (every named block,
//! the four liquid blocks with metadata `0..=15`), so the test needs no
//! prepared resources.
use super::{Fixture, World};
use crate::{
    level::{data, flags},
    registry::BlockStateId,
};
use sunrise_core::position::BlockPos;

const HEIGHT: i32 = 16;

pub(super) fn liquid_fixture() -> Fixture {
    let (registry, behaviours) = data::liquid_1_7_10_fixture();
    Fixture {
        registry,
        behaviours,
    }
}

pub(super) fn fill(
    w: &mut World<'_>,
    x0: i32,
    y0: i32,
    z0: i32,
    x1: i32,
    y1: i32,
    z1: i32,
    state: BlockStateId,
) {
    for x in x0..=x1 {
        for z in z0..=z1 {
            for y in y0..=y1 {
                w.put(BlockPos::new(x, y, z), state);
            }
        }
    }
}

pub(super) fn at(x: i32, y: i32, z: i32) -> BlockPos {
    BlockPos::new(x, y, z)
}

/// `World.setBlockWithMetadata(x, y, z, state, 0, 3)`.
pub(super) fn place(w: &mut World<'_>, x: i32, y: i32, z: i32, state: BlockStateId) {
    w.set(at(x, y, z), state);
}

/// A neighbour change on the placed liquid, as the block beside it does.
pub(super) fn kick(w: &mut World<'_>, x: i32, y: i32, z: i32) {
    let stone = w.id("stone", &[]);
    let changed = w.b().block(stone);
    let state = w.get(at(x, y, z));
    w.b()
        .sim(&mut w.level, &mut w.ticks)
        .neighbor_changed(state, at(x, y, z), changed, false);
}

/// The same lines the oracle prints: every non-air block, `x y z id meta`.
fn dump(w: &World<'_>) -> String {
    let b = w.b();
    let mut out = String::new();
    for y in 0..=HEIGHT {
        for z in 0..16 {
            for x in 0..16 {
                let state = w.get(at(x, y, z));
                if b.is_air(state) {
                    continue;
                }
                let name = &w.f.registry.state(state).unwrap().name;
                let meta = b.table.int(state, b.props.meta);
                out.push_str(&format!("{x} {y} {z} {name} {meta}\n"));
            }
        }
    }
    out
}

/// One scenario: the layout, the placements and the ticks the oracle runs.
fn run(name: &str) -> String {
    let f = liquid_fixture();
    let mut w = World::new(&f);
    // A 1.7.10 world is 0..256 high; the flow's writes outside it fail as
    // `World.setBlockWithMetadata` does.
    w.level.min_y = 0;
    w.level.max_y = 256;
    let stone = w.id("stone", &[]);
    let water = w.id("water", &[]);
    let flowing_water = w.id("flowing_water", &[]);
    let lava = w.id("lava", &[]);
    let flowing_lava = w.id("flowing_lava", &[]);
    let sand = w.id("sand", &[]);
    let gravel = w.id("gravel", &[]);
    let air = w.b().air();
    let ticks: i32 = match name {
        "flat" => {
            fill(&mut w, 4, 3, 4, 12, 3, 12, stone);
            place(&mut w, 8, 4, 8, water);
            kick(&mut w, 8, 4, 8);
            80
        }
        "hole" => {
            fill(&mut w, 4, 3, 4, 12, 3, 12, stone);
            fill(&mut w, 8, 3, 8, 8, 3, 8, air);
            place(&mut w, 8, 4, 8, water);
            kick(&mut w, 8, 4, 8);
            80
        }
        "run15" => {
            fill(&mut w, 0, 3, 7, 15, 3, 9, stone);
            fill(&mut w, 0, 4, 7, 15, 4, 7, stone);
            fill(&mut w, 0, 4, 9, 15, 4, 9, stone);
            fill(&mut w, 0, 4, 8, 0, 4, 8, stone);
            fill(&mut w, 15, 4, 8, 15, 4, 8, stone);
            place(&mut w, 8, 4, 8, water);
            kick(&mut w, 8, 4, 8);
            100
        }
        "two_sources" => {
            fill(&mut w, 2, 3, 6, 14, 3, 10, stone);
            place(&mut w, 4, 4, 8, water);
            place(&mut w, 12, 4, 8, water);
            kick(&mut w, 4, 4, 8);
            kick(&mut w, 12, 4, 8);
            120
        }
        "water_lava" => {
            fill(&mut w, 0, 3, 0, 15, 3, 15, stone);
            place(&mut w, 3, 4, 3, lava);
            place(&mut w, 3, 4, 4, water);
            let depth_two = w.b().table.set_int(flowing_lava, w.b().props.meta, 2);
            place(&mut w, 12, 4, 3, depth_two);
            place(&mut w, 12, 4, 4, water);
            place(&mut w, 3, 4, 12, water);
            place(&mut w, 3, 5, 12, lava);
            kick(&mut w, 3, 5, 12);
            60
        }
        "lava_over_water" => {
            fill(&mut w, 4, 3, 4, 12, 3, 12, stone);
            place(&mut w, 8, 4, 8, water);
            place(&mut w, 8, 6, 8, lava);
            kick(&mut w, 8, 6, 8);
            80
        }
        "cave" => {
            fill(&mut w, 3, 0, 3, 13, 4, 13, stone);
            fill(&mut w, 5, 1, 5, 11, 3, 11, air);
            fill(&mut w, 8, 4, 8, 8, 4, 8, air);
            place(&mut w, 8, 5, 8, water);
            kick(&mut w, 8, 5, 8);
            120
        }
        "lava_spring_under_water" => {
            fill(&mut w, 4, 3, 4, 12, 3, 12, stone);
            place(&mut w, 8, 4, 8, lava);
            place(&mut w, 8, 5, 8, water);
            20
        }
        "lava_nether" => {
            fill(&mut w, 4, 3, 4, 12, 3, 12, stone);
            w.level.dimension = sunrise_core::dimension::Dimension::Nether;
            place(&mut w, 8, 4, 8, lava);
            kick(&mut w, 8, 4, 8);
            40
        }
        "sand_fall" => {
            fill(&mut w, 4, 3, 4, 12, 3, 12, stone);
            w.ticks.fall_immediately = true;
            let sand_meta_one = w.b().table.set_int(sand, w.b().props.meta, 1);
            place(&mut w, 8, 6, 8, sand);
            place(&mut w, 9, 6, 8, gravel);
            place(&mut w, 10, 6, 8, sand_meta_one);
            10
        }
        "spring_gen" => {
            fill(&mut w, 4, 3, 4, 12, 3, 12, stone);
            w.ticks.do_ticks_immediately = true;
            // LiquidFallFeature: flags 2, then the immediate tick itself.
            w.b().set_block(
                &mut w.level,
                &mut w.ticks,
                at(8, 4, 8),
                flowing_water,
                flags::UPDATE_CLIENTS,
            );
            w.ticks.do_ticks_immediately = false;
            20
        }
        other => panic!("unknown scenario {other}"),
    };
    for _ in 0..ticks {
        w.tick();
    }
    dump(&w)
}

macro_rules! parity {
    ($name:ident) => {
        #[test]
        fn $name() {
            assert_eq!(
                run(stringify!($name)),
                include_str!(concat!("liquids/", stringify!($name), ".txt"))
            );
        }
    };
}

parity!(flat);
parity!(hole);
parity!(run15);
parity!(two_sources);
parity!(water_lava);
parity!(lava_over_water);
parity!(cave);
parity!(lava_spring_under_water);
parity!(sand_fall);
parity!(spring_gen);
parity!(lava_nether);
