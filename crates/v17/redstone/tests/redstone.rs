//! Unit tests for the 1.7.10 redstone invariants.

use sunset_1_7_redstone::block::{Block, directions};
use sunset_1_7_redstone::item::ItemStack;
use sunset_1_7_redstone::lever;
use sunset_1_7_redstone::piston;
use sunset_1_7_redstone::wire;
use sunset_1_7_redstone::world::{BlockEntity, Pos, World};

fn wire_at(world: &mut World, x: i32, y: i32, z: i32) {
    world.set_block_with_metadata(x, y, z, Block::RedstoneWire, 0, 2);
}

fn solid_at(world: &mut World, x: i32, y: i32, z: i32) {
    world.set_block_with_metadata(x, y, z, Block::Solid, 0, 2);
}

fn count(world: &World, x: i32, y: i32, z: i32) -> i32 {
    match world.block_entity(Pos::new(x, y, z)) {
        Some(BlockEntity::Inventory(inv)) => inv.get_item(0).map(|s| s.size).unwrap_or(0),
        _ => -1,
    }
}

/// A floor lever powers the wire next to it.
#[test]
fn lever_powers_wire() {
    let mut world = World::new(0);
    solid_at(&mut world, 0, 1, 0); // lever mount
    solid_at(&mut world, 1, 1, 0); // wire support
    world.set_block_with_metadata(0, 2, 0, Block::Lever, 5, 2);
    wire_at(&mut world, 1, 2, 0);
    lever::use_lever(&mut world, 0, 2, 0);
    assert_eq!(
        world.get_block_metadata(1, 2, 0),
        15,
        "wire next to an on lever"
    );
    // `RedstoneWireBlock.getSignal` returns the wire's power for `dir == 1`
    // only: the wire weakly powers the block it sits on, not the one above it.
    assert!(world.has_neighbor_signal(1, 1, 0));
}

/// Power decays by one per wire block after the first, which the source powers
/// to 15 (`RedstoneWireBlock.doUpdatePower`).
#[test]
fn wire_decay() {
    let mut world = World::new(0);
    solid_at(&mut world, 0, 1, 0);
    world.set_block_with_metadata(0, 2, 0, Block::Lever, 5, 2);
    for x in 1..=6 {
        solid_at(&mut world, x, 1, 0);
        wire_at(&mut world, x, 2, 0);
    }
    lever::use_lever(&mut world, 0, 2, 0);
    for (i, x) in (1..=6).enumerate() {
        let expected = (15 - i as i32).max(0);
        assert_eq!(world.get_block_metadata(x, 2, 0), expected, "wire at x={x}");
    }
}

/// Turning the lever off drains the wire back to zero.
#[test]
fn wire_drains_when_source_off() {
    let mut world = World::new(0);
    solid_at(&mut world, 0, 1, 0);
    world.set_block_with_metadata(0, 2, 0, Block::Lever, 5, 2);
    for x in 1..=4 {
        solid_at(&mut world, x, 1, 0);
        wire_at(&mut world, x, 2, 0);
    }
    lever::use_lever(&mut world, 0, 2, 0);
    lever::use_lever(&mut world, 0, 2, 0);
    for x in 1..=4 {
        assert_eq!(
            world.get_block_metadata(x, 2, 0),
            0,
            "wire drained at x={x}"
        );
    }
}

/// A lit torch strongly powers through a solid block.
#[test]
fn torch_strong_powers_through_block() {
    let mut world = World::new(0);
    solid_at(&mut world, 0, 0, 0); // floor
    solid_at(&mut world, 0, 2, 0); // the block above the torch
    solid_at(&mut world, 1, 1, 0); // wire support
    world.set_block_with_metadata(0, 1, 0, Block::RedstoneTorch, 0, 2);
    wire_at(&mut world, 1, 2, 0); // wire last: it settles against the torch
    let level = world.get_block_metadata(1, 2, 0);
    assert_eq!(level, 15, "wire beside the strongly powered block: {level}");
}

/// A powered attachment turns the torch off.
#[test]
fn torch_turns_off_when_attachment_powered() {
    let mut world = World::new(0);
    solid_at(&mut world, 0, 0, 0); // the attachment
    world.set_block_with_metadata(1, 0, 0, Block::Lever, 1, 2); // side lever
    world.set_block_with_metadata(0, 1, 0, Block::RedstoneTorch, 0, 2);
    lever::use_lever(&mut world, 1, 0, 0);
    world.tick();
    world.tick();
    assert_eq!(
        world.get_block(0, 1, 0),
        Block::UnlitRedstoneTorch,
        "torch turns off"
    );
}

/// A repeater turns on after its delay and powers the wire it faces.
#[test]
fn repeater_delays_and_faces() {
    let mut world = World::new(0);
    solid_at(&mut world, 0, 0, 0); // repeater support
    // redstone block east of the repeater: metadata 3 reads its input from the east
    world.set_block_with_metadata(1, 1, 0, Block::RedstoneBlock, 0, 3);
    solid_at(&mut world, -1, 0, 0); // wire support
    world.set_block_with_metadata(0, 1, 0, Block::Repeater, 3, 2);
    wire_at(&mut world, -1, 1, 0);
    assert_eq!(
        world.get_block(0, 1, 0),
        Block::Repeater,
        "repeater idle before the delay"
    );
    world.neighbor_changed(0, 1, 0, Block::Repeater);
    world.tick();
    assert_eq!(
        world.get_block(0, 1, 0),
        Block::Repeater,
        "still waiting after one tick"
    );
    world.tick();
    assert_eq!(
        world.get_block(0, 1, 0),
        Block::PoweredRepeater,
        "repeater on after 2 ticks"
    );
    assert_eq!(
        world.get_block_metadata(-1, 1, 0),
        15,
        "repeater output powers the wire"
    );
}

/// A repeater turns off after its delay when the input drops.
#[test]
fn repeater_turns_off() {
    let mut world = World::new(0);
    solid_at(&mut world, 0, 0, 0); // repeater support
    // redstone block east, repeater reading its input from the east
    world.set_block_with_metadata(1, 1, 0, Block::RedstoneBlock, 0, 3);
    world.set_block_with_metadata(0, 1, 0, Block::Repeater, 3, 2);
    world.neighbor_changed(0, 1, 0, Block::Repeater);
    world.tick();
    world.tick();
    assert_eq!(world.get_block(0, 1, 0), Block::PoweredRepeater);
    world.remove_block(1, 1, 0);
    world.neighbor_changed(0, 1, 0, Block::Air);
    world.tick();
    assert_eq!(
        world.get_block(0, 1, 0),
        Block::PoweredRepeater,
        "still on after one tick"
    );
    world.tick();
    assert_eq!(
        world.get_block(0, 1, 0),
        Block::Repeater,
        "repeater off after the delay"
    );
}

/// A comparator with metadata bit 4 clear passes its front signal when no side
/// signal competes (`ComparatorBlock.calculateOutputSignal`); with no side input
/// compare and subtract give the same 15.
#[test]
fn comparator_subtract() {
    let mut world = World::new(0);
    solid_at(&mut world, 0, 0, 0); // comparator support
    world.set_block_with_metadata(1, 1, 0, Block::RedstoneBlock, 0, 3);
    world.set_block_with_metadata(0, 1, 0, Block::Comparator, 3, 2);
    world.neighbor_changed(0, 1, 0, Block::Comparator);
    for _ in 0..6 {
        world.tick();
    }
    assert_eq!(
        world.comparator_output(Pos::new(0, 1, 0)),
        15,
        "front 15, side 0"
    );
}

/// The comparator's subtract output is front - side.
#[test]
fn comparator_subtracts_side_input() {
    let mut world = World::new(0);
    solid_at(&mut world, 0, 0, 0); // comparator support
    solid_at(&mut world, 0, 0, 1); // side wire support
    world.set_block_with_metadata(1, 1, 0, Block::RedstoneBlock, 0, 3); // front, east
    world.set_block_with_metadata(0, 1, 1, Block::RedstoneWire, 0, 2); // side, south
    world.set_block_metadata(0, 1, 1, 5, 2);
    world.set_block_with_metadata(0, 1, 0, Block::Comparator, 7, 2); // facing east, subtract
    world.neighbor_changed(0, 1, 0, Block::Comparator);
    for _ in 0..6 {
        world.tick();
    }
    assert_eq!(world.comparator_output(Pos::new(0, 1, 0)), 10, "15 - 5");
}

/// A comparator keeps its output through a state swap.
#[test]
fn comparator_keeps_output_through_swap() {
    let mut world = World::new(0);
    solid_at(&mut world, 0, 0, 0); // comparator support
    world.set_block_with_metadata(1, 1, 0, Block::RedstoneBlock, 0, 3);
    world.set_block_with_metadata(0, 1, 0, Block::Comparator, 3, 2);
    world.neighbor_changed(0, 1, 0, Block::Comparator);
    world.tick();
    world.tick();
    let output = world.comparator_output(Pos::new(0, 1, 0));
    assert_eq!(output, 15, "the comparator reads the redstone block");
    // the powered comparator's tick swaps back to the idle block, keeping the
    // output signal in the fresh block entity
    world.schedule_tick(0, 1, 0, Block::PoweredComparator, 1, 0);
    world.tick();
    assert_eq!(
        world.get_block(0, 1, 0),
        Block::Comparator,
        "swapped to the idle block"
    );
    assert_eq!(
        world.comparator_output(Pos::new(0, 1, 0)),
        output,
        "output kept"
    );
}

/// A comparator reads a container's fill level.
#[test]
fn comparator_reads_container() {
    let mut world = World::new(0);
    world.set_block_with_metadata(1, 1, 0, Block::Hopper, 3, 2);
    if let Some(inv) = world.inventory_at(Pos::new(1, 1, 0)) {
        inv.set_item(0, Some(ItemStack::new(1, 64)));
        inv.set_item(1, Some(ItemStack::new(2, 64)));
    }
    let signal = world.inventory_analog_signal(Pos::new(1, 1, 0));
    assert_eq!(signal, 6, "2/5 slots full: 14 * 0.4 = 5.6 -> 5 + 1");
    let _ = Block::Dropper;
}

/// A piston extends at the end of the tick the update arrives, and retracts
/// when the power drops.
#[test]
fn piston_extends_and_retracts() {
    let mut world = World::new(0);
    world.set_block_with_metadata(0, 1, 0, Block::Piston, 5, 2);
    world.set_block_with_metadata(0, 1, 1, Block::RedstoneBlock, 0, 3); // direct neighbor
    solid_at(&mut world, 1, 1, 0); // the block to push
    world.tick();
    assert_eq!(
        world.get_block(0, 1, 0),
        Block::Piston,
        "piston base stays the base"
    );
    assert!(
        piston::is_extended(world.get_block_metadata(0, 1, 0)),
        "piston extended"
    );
    assert_eq!(
        world.get_block(2, 1, 0),
        Block::MovingPiston,
        "pushed block moving"
    );
    // the moving blocks finish over the next entity ticks
    world.tick();
    world.tick();
    world.tick();
    assert_eq!(
        world.get_block(0, 1, 0),
        Block::Piston,
        "piston base landed"
    );
    assert!(
        piston::is_extended(world.get_block_metadata(0, 1, 0)),
        "piston extended"
    );
    assert_eq!(
        world.get_block(1, 1, 0),
        Block::PistonHead,
        "head landed in front"
    );
    assert_eq!(
        world.get_block(2, 1, 0),
        Block::Solid,
        "pushed block landed"
    );

    // retract
    world.remove_block(0, 1, 1);
    world.tick();
    assert_eq!(
        world.get_block(0, 1, 0),
        Block::MovingPiston,
        "piston retracting"
    );
    world.tick();
    world.tick();
    world.tick();
    assert_eq!(
        world.get_block(0, 1, 0),
        Block::Piston,
        "piston base landed back"
    );
    assert!(!piston::is_extended(world.get_block_metadata(0, 1, 0)));
    assert_eq!(world.get_block(1, 1, 0), Block::Air, "head gone");
    assert_eq!(
        world.get_block(2, 1, 0),
        Block::Solid,
        "pushed block stayed"
    );
}

/// A sticky piston pulls its block back.
#[test]
fn sticky_piston_pulls() {
    let mut world = World::new(0);
    world.set_block_with_metadata(0, 1, 0, Block::StickyPiston, 5, 2);
    world.set_block_with_metadata(0, 1, 1, Block::RedstoneBlock, 0, 3);
    solid_at(&mut world, 1, 1, 0); // the block to pull
    world.tick();
    world.tick();
    world.tick();
    world.tick();
    assert_eq!(world.get_block(1, 1, 0), Block::PistonHead, "extended");
    assert_eq!(world.get_block(2, 1, 0), Block::Solid, "pushed out");
    world.remove_block(0, 1, 1);
    world.tick();
    world.tick();
    world.tick();
    world.tick();
    assert_eq!(world.get_block(0, 1, 0), Block::StickyPiston, "retracted");
    assert_eq!(world.get_block(1, 1, 0), Block::Solid, "block pulled back");
    assert_eq!(
        world.get_block(2, 1, 0),
        Block::Air,
        "nothing left in front"
    );
}

/// A piston with a power source only in a block above it (quasi-connectivity)
/// extends: placing the piston is the update that reads the block above.
#[test]
fn piston_quasi_connectivity() {
    let mut world = World::new(0);
    world.set_block_with_metadata(0, 3, 0, Block::RedstoneBlock, 0, 2);
    world.set_block_with_metadata(0, 1, 0, Block::StickyPiston, 1, 2);
    world.tick();
    assert_eq!(
        world.get_block(0, 1, 0),
        Block::StickyPiston,
        "the base stays the base"
    );
    assert!(
        piston::is_extended(world.get_block_metadata(0, 1, 0)),
        "BUD power extends the piston"
    );
    assert_eq!(
        world.get_block(0, 2, 0),
        Block::MovingPiston,
        "the head is on its way"
    );
}

/// Obsidian blocks a piston's push.
#[test]
fn piston_blocked_by_obsidian() {
    let mut world = World::new(0);
    world.set_block_with_metadata(0, 1, 0, Block::Piston, 5, 2);
    world.set_block_with_metadata(0, 1, 1, Block::RedstoneBlock, 0, 3);
    world.set_block_with_metadata(1, 1, 0, Block::Obsidian, 0, 2);
    world.tick();
    assert_eq!(world.get_block(0, 1, 0), Block::Piston, "piston stays put");
    assert!(!piston::is_extended(world.get_block_metadata(0, 1, 0)));
}

/// The 12-block push limit: 13 and 14 blocks stop extension.
#[test]
fn piston_push_limit() {
    for count in [13, 14] {
        let mut world = World::new(0);
        world.set_block_with_metadata(0, 1, 0, Block::Piston, 5, 2);
        world.set_block_with_metadata(0, 1, 1, Block::RedstoneBlock, 0, 3);
        for x in 1..=count {
            solid_at(&mut world, x, 1, 0);
        }
        world.tick();
        assert!(
            !piston::is_extended(world.get_block_metadata(0, 1, 0)),
            "{count} blocks: too far"
        );
    }
}

/// 12 blocks are the most a piston pushes.
#[test]
fn piston_pushes_twelve() {
    let mut world = World::new(0);
    world.set_block_with_metadata(0, 1, 0, Block::Piston, 5, 2);
    world.set_block_with_metadata(0, 1, 1, Block::RedstoneBlock, 0, 3);
    for x in 1..=12 {
        solid_at(&mut world, x, 1, 0);
    }
    world.tick();
    assert!(
        piston::is_extended(world.get_block_metadata(0, 1, 0)),
        "12 blocks push"
    );
}

/// Hoppers pass items down a chain, one per 8-tick cooldown.
#[test]
fn hopper_chain() {
    let mut world = World::new(0);
    world.set_block_with_metadata(0, 2, 0, Block::Hopper, 0, 2);
    world.set_block_with_metadata(0, 1, 0, Block::Hopper, 0, 2);
    if let Some(inv) = world.inventory_at(Pos::new(0, 2, 0)) {
        inv.set_item(0, Some(ItemStack::new(1, 3)));
    }
    for _ in 0..12 {
        world.tick();
    }
    assert_eq!(count(&world, 0, 1, 0), 3, "all items moved down the chain");
    assert_eq!(count(&world, 0, 2, 0), 0, "nothing left above");
}

/// A block entity made again at the same position ticks once per tick, not once
/// per time the position was ever given an entity.
#[test]
fn replaced_hopper_ticks_once() {
    let mut world = World::new(0);
    world.set_block_with_metadata(0, 2, 0, Block::Hopper, 0, 2);
    world.set_block_with_metadata(0, 1, 0, Block::Hopper, 0, 2);
    world.remove_block(0, 1, 0);
    world.set_block_with_metadata(0, 1, 0, Block::Hopper, 0, 2);
    if let Some(inv) = world.inventory_at(Pos::new(0, 2, 0)) {
        inv.set_item(0, Some(ItemStack::new(1, 3)));
    }
    for _ in 0..4 {
        world.tick();
    }
    assert_eq!(
        count(&world, 0, 1, 0),
        1,
        "one item after the first transfer"
    );
    assert_eq!(count(&world, 0, 2, 0), 2, "the 8-tick cooldown still holds");
}

/// A hopper under redstone power does not transfer, and the hopper below it
/// cannot pull the locked hopper's item either.
#[test]
fn hopper_locked_by_signal() {
    let mut world = World::new(0);
    world.set_block_with_metadata(0, 2, 0, Block::Hopper, 0, 2);
    world.set_block_with_metadata(0, 1, 0, Block::Hopper, 0, 2);
    world.set_block_with_metadata(1, 2, 0, Block::RedstoneBlock, 0, 3);
    world.set_block_with_metadata(1, 1, 0, Block::RedstoneBlock, 0, 3);
    if let Some(inv) = world.inventory_at(Pos::new(0, 2, 0)) {
        inv.set_item(0, Some(ItemStack::new(1, 1)));
    }
    for _ in 0..12 {
        world.tick();
    }
    assert_eq!(world.get_block_metadata(0, 2, 0) & 8, 8, "hopper disabled");
    assert_eq!(
        count(&world, 0, 2, 0),
        1,
        "the item stays in the locked hopper"
    );
    assert_eq!(
        count(&world, 0, 1, 0),
        0,
        "powered hopper does not transfer"
    );
}

/// A dropper facing a hopper drops one item into it.
#[test]
fn dropper_into_hopper() {
    let mut world = World::new(0);
    world.set_block_with_metadata(0, 1, 0, Block::Dropper, 5, 2);
    // `DispenserBlock.onAdded` re-derives the facing from the opaque neighbors;
    // set it after placement so east is the output.
    world.set_block_metadata(0, 1, 0, 5, 2);
    world.set_block_with_metadata(1, 1, 0, Block::Hopper, 1, 2);
    if let Some(inv) = world.inventory_at(Pos::new(0, 1, 0)) {
        inv.set_item(0, Some(ItemStack::new(7, 2)));
    }
    world.set_block_with_metadata(0, 2, 0, Block::RedstoneBlock, 0, 3);
    for _ in 0..4 {
        world.tick();
    }
    assert_eq!(
        count(&world, 1, 1, 0),
        1,
        "one item dropped into the hopper"
    );
    assert_eq!(count(&world, 0, 1, 0), 1, "one item left in the dropper");
}

/// Wire connects to both ends of a repeater, and the repeater's output powers it
/// once the delay has passed.
#[test]
fn wire_connects_to_repeater() {
    let mut world = World::new(0);
    solid_at(&mut world, 0, 0, 0); // wire support
    solid_at(&mut world, 1, 0, 0); // repeater support
    world.set_block_with_metadata(1, 1, 0, Block::Repeater, 3, 2);
    world.set_block_with_metadata(2, 1, 0, Block::RedstoneBlock, 0, 2);
    wire_at(&mut world, 0, 1, 0);
    // `RedstoneWireBlock.connectsTo` asks its neighbours with the direction from
    // the wire, so the repeater west of it is side 3 (east).
    assert!(
        wire::should_connect_to(&world, 1, 1, 0, 3),
        "the repeater's output end"
    );
    assert!(
        wire::should_connect_to(&world, 1, 1, 0, 1),
        "the repeater's input end"
    );
    world.neighbor_changed(1, 1, 0, Block::Repeater);
    world.tick();
    world.tick();
    assert_eq!(world.get_block(1, 1, 0), Block::PoweredRepeater);
    assert_eq!(
        world.get_block_metadata(0, 1, 0),
        15,
        "the powered repeater connects"
    );
    assert!(
        wire::connects_to(&world, 1, 1, 0, 3),
        "powered repeater connects"
    );
}

/// The scheduled-tick dedupe: a diode scheduled twice ticks once.
#[test]
fn scheduled_tick_dedupe() {
    let mut world = World::new(0);
    world.set_block_with_metadata(0, 1, 0, Block::Repeater, 0, 2);
    world.schedule_tick(0, 1, 0, Block::Repeater, 1, 0);
    world.schedule_tick(0, 1, 0, Block::Repeater, 2, 0);
    world.tick();
    assert_eq!(
        world.scheduled_tick_count(),
        1,
        "one turn-off left, deduped to one tick"
    );
}

/// An isolated powered wire powers all four horizontal neighbors.
#[test]
fn unconnected_wire_points_all_directions() {
    let mut world = World::new(0);
    solid_at(&mut world, 0, 0, 0);
    wire_at(&mut world, 0, 1, 0);
    // `RedstoneWireBlock.onAdded` recomputes the power, so a freshly placed lone
    // wire is 0; set the state a source below its support would settle on.
    world.set_block_metadata(0, 1, 0, 15, 2);
    assert_eq!(world.get_block_metadata(0, 1, 0), 15);
    assert!(world.has_neighbor_signal(1, 1, 0), "east powered");
    assert!(world.has_neighbor_signal(-1, 1, 0), "west powered");
    assert!(world.has_neighbor_signal(0, 1, 1), "south powered");
    assert!(world.has_neighbor_signal(0, 1, -1), "north powered");
    let _ = directions::DOWN;
}

/// A north-south wire line powers along the line, not to the sides.
#[test]
fn connected_wire_points_one_direction() {
    let mut world = World::new(0);
    for (x, z) in [(0, 0), (0, 1), (0, -1)] {
        solid_at(&mut world, x, 0, z);
        wire_at(&mut world, x, 1, z);
    }
    // Placement recomputes the power; set the settled levels explicitly.
    world.set_block_metadata(0, 1, 0, 15, 2);
    world.set_block_metadata(0, 1, 1, 14, 2);
    world.set_block_metadata(0, 1, -1, 14, 2);
    assert!(!world.has_neighbor_signal(1, 1, 0), "east not powered");
    assert!(!world.has_neighbor_signal(-1, 1, 0), "west not powered");
    assert!(world.has_neighbor_signal(0, 1, 1), "south powered");
    assert!(world.has_neighbor_signal(0, 1, -1), "north powered");
}

/// Wire pops off without support.
#[test]
fn wire_pops_without_support() {
    let mut world = World::new(0);
    world.set_block_with_metadata(0, 1, 0, Block::RedstoneWire, 0, 2);
    world.neighbor_changed(0, 1, 0, Block::Air);
    assert_eq!(world.get_block(0, 1, 0), Block::Air, "floating wire gone");
}
