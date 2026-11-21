//! `DiodeBlock`, `RepeaterBlock` and `ComparatorBlock`: delay ticks, locking,
//! comparator modes and analog output, ported one to one.

use crate::block::{Block, facings};
use crate::world::{Pos, World};

/// `RepeaterBlock.DELAY_SETTINGS`.
const DELAY_SETTINGS: [i32; 4] = [1, 2, 3, 4];

fn get_facing(meta: i32) -> i32 {
    meta & 3
}

/// `RepeaterBlock.getDelay`: the delay setting times 2.
pub fn get_delay(meta: i32) -> i32 {
    DELAY_SETTINGS[((meta & 12) >> 2) as usize] * 2
}

/// `ComparatorBlock.getDelay`: always 2.
fn comparator_delay() -> i32 {
    2
}

/// `DiodeBlock.isPowered` with the comparator override.
pub fn is_powered(world: &World, x: i32, y: i32, z: i32, block: Block) -> bool {
    match block {
        Block::PoweredRepeater | Block::PoweredComparator => true,
        Block::Comparator => (world.get_block_metadata(x, y, z) & 8) != 0,
        _ => false,
    }
}

/// `DiodeBlock.getOutputSignal`: 15 for repeaters, the block entity's output
/// for comparators.
fn output_signal(world: &World, x: i32, y: i32, z: i32, block: Block) -> i32 {
    if crate::block::is_comparator(block) {
        world.comparator_output(Pos::new(x, y, z))
    } else {
        15
    }
}

/// `DiodeBlock.getSignal`: 15 toward the block the diode faces when powered.
pub fn get_signal(world: &World, x: i32, y: i32, z: i32, block: Block, dir: i32) -> i32 {
    if !is_powered(world, x, y, z, block) {
        return 0;
    }
    let facing = get_facing(world.get_block_metadata(x, y, z));
    match (facing, dir) {
        (0, 3) => output_signal(world, x, y, z, block),
        (1, 4) => output_signal(world, x, y, z, block),
        (2, 2) => output_signal(world, x, y, z, block),
        (3, 5) => output_signal(world, x, y, z, block),
        _ => 0,
    }
}

/// `RepeaterBlock.isValidSideInput`: any diode; the comparator keeps the base
/// `isSignalSource`.
fn is_valid_side_input(block: Block, for_block: Block) -> bool {
    if for_block == Block::Comparator || for_block == Block::PoweredComparator {
        block.is_signal_source()
    } else {
        block.is_diode()
    }
}

/// `DiodeBlock.getInputSignalFromSide`.
fn get_input_signal_from_side(
    world: &World,
    x: i32,
    y: i32,
    z: i32,
    dir: i32,
    for_block: Block,
) -> i32 {
    let block = world.get_block(x, y, z);
    if is_valid_side_input(block, for_block) {
        if block == Block::RedstoneWire {
            world.get_block_metadata(x, y, z)
        } else {
            world.get_direct_signal(x, y, z, dir)
        }
    } else {
        0
    }
}

/// `DiodeBlock.getInputSignalFromSides`.
pub fn get_input_signal_from_sides(
    world: &World,
    x: i32,
    y: i32,
    z: i32,
    meta: i32,
    for_block: Block,
) -> i32 {
    match get_facing(meta) {
        0 | 2 => get_input_signal_from_side(world, x - 1, y, z, 4, for_block)
            .max(get_input_signal_from_side(world, x + 1, y, z, 5, for_block)),
        1 | 3 => get_input_signal_from_side(world, x, y, z + 1, 3, for_block)
            .max(get_input_signal_from_side(world, x, y, z - 1, 2, for_block)),
        _ => 0,
    }
}

/// `DiodeBlock.getInputSignal`: the wire or source in front of the facing.
pub fn get_input_signal(world: &World, x: i32, y: i32, z: i32, meta: i32) -> i32 {
    let facing = get_facing(meta) as usize;
    let bx = x + facings::X_OFFSET[facing];
    let bz = z + facings::Z_OFFSET[facing];
    let signal = world.get_signal(bx, y, bz, facings::TO_DIR[facing]);
    if signal >= 15 {
        signal
    } else {
        let front = world.get_block(bx, y, bz);
        let wire = if front == Block::RedstoneWire {
            world.get_block_metadata(bx, y, bz)
        } else {
            0
        };
        signal.max(wire)
    }
}

/// `ComparatorBlock.getInputSignal`: the base input, then analog sources in
/// front of and, through a solid block, two blocks in front of the facing.
pub fn comparator_get_input_signal(world: &World, x: i32, y: i32, z: i32, meta: i32) -> i32 {
    let mut signal = get_input_signal(world, x, y, z, meta);
    let facing = get_facing(meta) as usize;
    let mut bx = x + facings::X_OFFSET[facing];
    let mut bz = z + facings::Z_OFFSET[facing];
    let block = world.get_block(bx, y, bz);
    if block.is_analog_signal_source() {
        signal = world.inventory_analog_signal(Pos::new(bx, y, bz));
    } else if signal < 15 && block.is_solid() {
        bx += facings::X_OFFSET[facing];
        bz += facings::Z_OFFSET[facing];
        let behind = world.get_block(bx, y, bz);
        if behind.is_analog_signal_source() {
            signal = world.inventory_analog_signal(Pos::new(bx, y, bz));
        }
    }
    signal
}

/// `RepeaterBlock.isLocked`: a powered side input locks the repeater.
pub fn is_locked(world: &World, x: i32, y: i32, z: i32, meta: i32) -> bool {
    get_input_signal_from_sides(world, x, y, z, meta, Block::Repeater) > 0
}

/// `RepeaterBlock.shouldBePowered` (`DiodeBlock.shouldBePowered`).
fn repeater_should_be_powered(world: &World, x: i32, y: i32, z: i32, meta: i32) -> bool {
    get_input_signal(world, x, y, z, meta) > 0
}

/// `ComparatorBlock.shouldBePowered`.
fn comparator_should_be_powered(world: &World, x: i32, y: i32, z: i32, meta: i32) -> bool {
    let front = comparator_get_input_signal(world, x, y, z, meta);
    if front >= 15 {
        return true;
    }
    if front == 0 {
        return false;
    }
    let side = get_input_signal_from_sides(world, x, y, z, meta, Block::Comparator);
    if side == 0 { true } else { front >= side }
}

/// `DiodeBlock.shouldPrioritize`: a diode pointing at this one from behind.
fn should_prioritize(world: &World, x: i32, y: i32, z: i32, meta: i32) -> bool {
    let facing = get_facing(meta) as usize;
    let bx = x - facings::X_OFFSET[facing];
    let bz = z - facings::Z_OFFSET[facing];
    if world.get_block(bx, y, bz).is_diode() {
        let other_facing = get_facing(world.get_block_metadata(bx, y, bz));
        other_facing != facing as i32
    } else {
        false
    }
}

/// `DiodeBlock.tick` for repeaters.
pub fn tick(world: &mut World, x: i32, y: i32, z: i32, block: Block) {
    let meta = world.get_block_metadata(x, y, z);
    if !is_locked(world, x, y, z, meta) {
        let should = repeater_should_be_powered(world, x, y, z, meta);
        if block == Block::PoweredRepeater && !should {
            world.set_block_with_metadata(x, y, z, Block::Repeater, meta, 2);
        } else if block == Block::Repeater {
            world.set_block_with_metadata(x, y, z, Block::PoweredRepeater, meta, 2);
            if !should {
                world.schedule_tick(x, y, z, Block::PoweredRepeater, get_delay(meta), -1);
            }
        }
    }
}

/// `ComparatorBlock.tick` (only the powered comparator's tick swaps the block).
pub fn comparator_tick(world: &mut World, x: i32, y: i32, z: i32, block: Block) {
    if block == Block::PoweredComparator {
        let meta = world.get_block_metadata(x, y, z);
        world.set_block_with_metadata(x, y, z, Block::Comparator, meta | 8, 4);
    }
    update_output_state(world, x, y, z);
}

/// `ComparatorBlock.updateOutputState`.
fn update_output_state(world: &mut World, x: i32, y: i32, z: i32) {
    let meta = world.get_block_metadata(x, y, z);
    let new_output = calculate_output_signal(world, x, y, z, meta);
    let old_output = world.comparator_output(Pos::new(x, y, z));
    world.set_comparator_output(Pos::new(x, y, z), new_output);
    if old_output != new_output || !is_in_compare_mode(meta) {
        let should = comparator_should_be_powered(world, x, y, z, meta);
        let block = world.get_block(x, y, z);
        let is_on = block == Block::PoweredComparator || (meta & 8) != 0;
        if is_on && !should {
            world.set_block_metadata(x, y, z, meta & !8, 2);
        } else if !is_on && should {
            world.set_block_metadata(x, y, z, meta | 8, 2);
        }
        update_neighbors(world, x, y, z);
    }
}

/// `ComparatorBlock.calculateOutputSignal`.
fn calculate_output_signal(world: &World, x: i32, y: i32, z: i32, meta: i32) -> i32 {
    if !is_in_compare_mode(meta) {
        comparator_get_input_signal(world, x, y, z, meta)
    } else {
        (comparator_get_input_signal(world, x, y, z, meta)
            - get_input_signal_from_sides(world, x, y, z, meta, Block::Comparator))
        .max(0)
    }
}

/// `ComparatorBlock.isInCompareMode`: the decompiler's name for metadata bit 4,
/// which selects the subtracting output (front - side).
pub fn is_in_compare_mode(meta: i32) -> bool {
    (meta & 4) == 4
}

/// `ComparatorBlock.checkOutputState`.
fn comparator_check_output_state(world: &mut World, x: i32, y: i32, z: i32) {
    let block = world.get_block(x, y, z);
    if !world.will_tick_this_tick(x, y, z, block) {
        let meta = world.get_block_metadata(x, y, z);
        let new_output = calculate_output_signal(world, x, y, z, meta);
        let old_output = world.comparator_output(Pos::new(x, y, z));
        if new_output != old_output
            || is_powered(world, x, y, z, block)
                != comparator_should_be_powered(world, x, y, z, meta)
        {
            if should_prioritize(world, x, y, z, meta) {
                world.schedule_tick(x, y, z, block, comparator_delay(), -1);
            } else {
                world.schedule_tick(x, y, z, block, comparator_delay(), 0);
            }
        }
    }
}

/// `DiodeBlock.checkOutputState` for repeaters.
fn repeater_check_output_state(world: &mut World, x: i32, y: i32, z: i32) {
    let meta = world.get_block_metadata(x, y, z);
    if !is_locked(world, x, y, z, meta) {
        let block = world.get_block(x, y, z);
        let should = repeater_should_be_powered(world, x, y, z, meta);
        let powered = block == Block::PoweredRepeater;
        if (powered && !should || !powered && should) && !world.will_tick_this_tick(x, y, z, block)
        {
            let mut priority = -1;
            if should_prioritize(world, x, y, z, meta) {
                priority = -3;
            } else if powered {
                priority = -2;
            }
            world.schedule_tick(x, y, z, block, get_delay(meta), priority);
        }
    }
}

/// `DiodeBlock.neighborChanged`: break without support, otherwise schedule the
/// output change.
pub fn neighbor_changed(world: &mut World, x: i32, y: i32, z: i32, block: Block) {
    if !can_survive(world, x, y, z) {
        world.drop_items(x, y, z, world.get_block_metadata(x, y, z), 0);
        world.remove_block(x, y, z);
        world.update_neighbors(x + 1, y, z, block);
        world.update_neighbors(x - 1, y, z, block);
        world.update_neighbors(x, y, z + 1, block);
        world.update_neighbors(x, y, z - 1, block);
        world.update_neighbors(x, y - 1, z, block);
        world.update_neighbors(x, y + 1, z, block);
        return;
    }
    if crate::block::is_comparator(block) {
        comparator_check_output_state(world, x, y, z);
    } else {
        repeater_check_output_state(world, x, y, z);
    }
}

/// `DiodeBlock.canSurvive`: a solid top below.
fn can_survive(world: &World, x: i32, y: i32, z: i32) -> bool {
    world.has_solid_top(x, y - 1, z)
}

/// `DiodeBlock.updateNeighbors`: the block in front of the facing (and its
/// neighbors, except the side the diode read it through) reacts.
pub fn update_neighbors(world: &mut World, x: i32, y: i32, z: i32) {
    let facing = get_facing(world.get_block_metadata(x, y, z));
    match facing {
        1 => {
            world.neighbor_changed(x + 1, y, z, Block::Repeater);
            world.update_neighbors_except(x + 1, y, z, Block::Repeater, 4);
        }
        3 => {
            world.neighbor_changed(x - 1, y, z, Block::Repeater);
            world.update_neighbors_except(x - 1, y, z, Block::Repeater, 5);
        }
        2 => {
            world.neighbor_changed(x, y, z + 1, Block::Repeater);
            world.update_neighbors_except(x, y, z + 1, Block::Repeater, 2);
        }
        0 => {
            world.neighbor_changed(x, y, z - 1, Block::Repeater);
            world.update_neighbors_except(x, y, z - 1, Block::Repeater, 3);
        }
        _ => {}
    }
}

/// `DiodeBlock.onAdded`.
pub fn on_added(world: &mut World, x: i32, y: i32, z: i32, _block: Block) {
    update_neighbors(world, x, y, z);
    // `DiodeBlock.onPlaced` schedules a tick when placed powered; placement by
    // entity is not ported.
}

/// `DiodeBlock.onRemoved` (`RepeaterBlock.onRemoved`,
/// `ComparatorBlock.onRemoved`).
pub fn on_removed(world: &mut World, x: i32, y: i32, z: i32, block: Block) {
    if crate::block::is_comparator(block) {
        world.remove_block_entity(Pos::new(x, y, z));
    }
    update_neighbors(world, x, y, z);
}

/// `RepeaterBlock.use`: cycle the delay setting 1 -> 2 -> 3 -> 4.
pub fn use_repeater(world: &mut World, x: i32, y: i32, z: i32) -> bool {
    let meta = world.get_block_metadata(x, y, z);
    let mut delay = (meta & 12) >> 2;
    delay = ((delay + 1) << 2) & 12;
    world.set_block_metadata(x, y, z, delay | (meta & 3), 3);
    true
}

/// `ComparatorBlock.use`: switch between compare and subtract mode.
pub fn use_comparator(world: &mut World, x: i32, y: i32, z: i32) -> bool {
    let meta = world.get_block_metadata(x, y, z);
    let block = world.get_block(x, y, z);
    let powered_bit = block == Block::PoweredComparator || (meta & 8) != 0;
    let to_compare = !is_in_compare_mode(meta);
    let mut new_meta = if to_compare { 4 } else { 0 };
    if powered_bit {
        new_meta |= 8;
    }
    // the click sound is not ported
    world.set_block_metadata(x, y, z, new_meta | (meta & 3), 2);
    update_output_state(world, x, y, z);
    true
}
