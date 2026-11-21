//! 1.7.10 liquid and gravity behaviour: the metadata ladder, the tick delays,
//! the lava-water reaction and the falling block that moves inside its tick.
//! The full scenarios are diffed against the real game in [`super::liquids`];
//! these pin the rules one at a time.
use super::World;
use super::liquids::{at, fill, kick, liquid_fixture, place};
use crate::level::{LevelEvent, flags};
use sunrise_core::position::BlockPos;

fn new_world(f: &super::Fixture) -> World<'_> {
    let mut w = World::new(f);
    w.level.min_y = 0;
    w.level.max_y = 256;
    w
}

fn meta(w: &World<'_>, pos: BlockPos) -> i32 {
    let state = w.get(pos);
    w.b().table.int(state, w.b().props.meta)
}

fn name(w: &World<'_>, pos: BlockPos) -> String {
    w.f.registry
        .state(w.get(pos))
        .unwrap()
        .name
        .trim_start_matches("minecraft:")
        .to_owned()
}

/// `FlowingLiquidBlock.tick`: the source stays a source at level 0 and the
/// neighbours become source water one metadata level further out.
#[test]
fn a_water_source_spreads_by_metadata_levels() {
    let f = liquid_fixture();
    let mut w = new_world(&f);
    let stone = w.id("stone", &[]);
    fill(&mut w, 0, 3, 0, 15, 3, 15, stone);
    let water = w.id("water", &[]);
    place(&mut w, 8, 4, 8, water);
    kick(&mut w, 8, 4, 8);
    for _ in 0..80 {
        w.tick();
    }
    assert_eq!(
        (name(&w, at(8, 4, 8)), meta(&w, at(8, 4, 8))),
        ("water".into(), 0)
    );
    for (x, z, level) in [(9, 8, 1), (10, 8, 2), (8, 9, 1), (8, 10, 2)] {
        assert_eq!(
            (name(&w, at(x, 4, z)), meta(&w, at(x, 4, z))),
            ("water".into(), level),
            "distance cell ({x}, 4, {z})"
        );
    }
}

/// `LiquidBlock.getTickRate`: a flowing liquid's first tick comes five ticks
/// on for water and thirty for lava in the overworld.
#[test]
fn a_flowing_liquid_schedules_the_vanilla_delay() {
    let f = liquid_fixture();
    let mut w = new_world(&f);
    let stone = w.id("stone", &[]);
    fill(&mut w, 0, 3, 0, 15, 3, 15, stone);
    let flowing_water = w.id("flowing_water", &[]);
    let flowing_lava = w.id("flowing_lava", &[]);
    place(&mut w, 4, 4, 4, flowing_water);
    place(&mut w, 6, 4, 4, flowing_lava);
    let delay = |pos: BlockPos| {
        w.ticks
            .blocks
            .all_queued()
            .find(|tick| tick.pos == pos)
            .unwrap()
            .trigger_tick
    };
    assert_eq!(delay(at(4, 4, 4)), 5, "water");
    assert_eq!(delay(at(6, 4, 4)), 30, "lava");
}

/// `LiquidBlock.checkBlockCollisions`: a source lava beside water becomes
/// obsidian, a flow of level 1..4 becomes cobblestone, and every reaction
/// hisses.
#[test]
fn lava_beside_water_turns_to_obsidian_or_cobblestone() {
    let f = liquid_fixture();
    let mut w = new_world(&f);
    let stone = w.id("stone", &[]);
    fill(&mut w, 0, 3, 0, 15, 3, 15, stone);
    let lava = w.id("lava", &[]);
    let water = w.id("water", &[]);
    place(&mut w, 3, 4, 3, lava);
    place(&mut w, 3, 4, 4, water);
    let depth_two = w
        .b()
        .table
        .set_int(w.id("flowing_lava", &[]), w.b().props.meta, 2);
    place(&mut w, 10, 4, 3, depth_two);
    place(&mut w, 10, 4, 4, water);
    assert_eq!(
        (name(&w, at(3, 4, 3)), meta(&w, at(3, 4, 3))),
        ("obsidian".into(), 0)
    );
    assert_eq!(
        (name(&w, at(10, 4, 3)), meta(&w, at(10, 4, 3))),
        ("cobblestone".into(), 0)
    );
    let hisses = w
        .level
        .events
        .iter()
        .filter(|event| matches!(event, LevelEvent::LevelEvent { id: 1501, .. }))
        .count();
    assert_eq!(hisses, 2, "one hiss for each reaction");
}

/// `FlowingLiquidBlock.tick`s lava branch: lava falling onto water turns the
/// water to stone.
#[test]
fn a_flowing_lava_above_water_turns_it_to_stone() {
    let f = liquid_fixture();
    let mut w = new_world(&f);
    let stone = w.id("stone", &[]);
    fill(&mut w, 4, 3, 4, 12, 3, 12, stone);
    let water = w.id("water", &[]);
    let lava = w.id("lava", &[]);
    place(&mut w, 8, 4, 8, water);
    place(&mut w, 8, 6, 8, lava);
    kick(&mut w, 8, 6, 8);
    for _ in 0..80 {
        w.tick();
    }
    assert_eq!(
        (name(&w, at(8, 4, 8)), meta(&w, at(8, 4, 8))),
        ("stone".into(), 0)
    );
}

/// `FallingBlock.fallImmediately`: chunk population moves the block down in
/// the tick itself, with the metadata `World.setBlock` writes (0).
#[test]
fn fall_immediately_moves_sand_in_its_tick() {
    let f = liquid_fixture();
    let mut w = new_world(&f);
    let stone = w.id("stone", &[]);
    fill(&mut w, 4, 3, 4, 12, 3, 12, stone);
    w.ticks.fall_immediately = true;
    let sand_meta_one = w.b().table.set_int(w.id("sand", &[]), w.b().props.meta, 1);
    place(&mut w, 8, 6, 8, sand_meta_one);
    for _ in 0..2 {
        w.tick();
    }
    assert_eq!(
        (name(&w, at(8, 4, 8)), meta(&w, at(8, 4, 8))),
        ("sand".into(), 0)
    );
    assert!(
        !w.level
            .events
            .iter()
            .any(|event| matches!(event, LevelEvent::FallingBlock { .. })),
        "no falling entity is spawned"
    );
}

/// `FallingBlockEntity`'s landing keeps the carried metadata (1.7.10 has no
/// concrete powder and no waterlogged landing) and the entity is spawned as a
/// level event when the block is not falling inside world generation.
#[test]
fn a_falling_block_spawns_an_entity_and_lands_with_its_metadata() {
    let f = liquid_fixture();
    let mut w = new_world(&f);
    let stone = w.id("stone", &[]);
    fill(&mut w, 4, 3, 4, 12, 3, 12, stone);
    let sand_meta_one = w.b().table.set_int(w.id("sand", &[]), w.b().props.meta, 1);
    place(&mut w, 8, 6, 8, sand_meta_one);
    for _ in 0..2 {
        w.tick();
    }
    assert_eq!(w.get(at(8, 6, 8)), w.b().air());
    assert!(
        w.level
            .events
            .iter()
            .any(|event| matches!(event, LevelEvent::FallingBlock { state, .. } if *state == sand_meta_one)),
        "the falling entity is spawned"
    );
    let landing = w.b().falling_block_landed(
        &mut w.level,
        &mut w.ticks,
        at(8, 4, 8),
        sand_meta_one,
        false,
    );
    assert_eq!(landing, crate::level::Landing::Placed(sand_meta_one));
    assert_eq!(
        (name(&w, at(8, 4, 8)), meta(&w, at(8, 4, 8))),
        ("sand".into(), 1)
    );
}

/// A world generation spring placed with `doTicksImmediately` settles inside
/// the placing write, as `LiquidFallFeature`'s tick does.
#[test]
fn a_generated_spring_ticks_immediately() {
    let f = liquid_fixture();
    let mut w = new_world(&f);
    let stone = w.id("stone", &[]);
    fill(&mut w, 4, 3, 4, 12, 3, 12, stone);
    w.ticks.do_ticks_immediately = true;
    let flowing_water = w.id("flowing_water", &[]);
    w.b().set_block(
        &mut w.level,
        &mut w.ticks,
        at(8, 4, 8),
        flowing_water,
        flags::UPDATE_CLIENTS,
    );
    w.ticks.do_ticks_immediately = false;
    // The immediate tick already spread level 1 to the four neighbours.
    for (x, z) in [(7, 8), (9, 8), (8, 7), (8, 9)] {
        assert_eq!(meta(&w, at(x, 4, z)), 1, "({x}, 4, {z})");
    }
}
