//! `RedstoneWireBlock`: power levels, connection geometry and the change
//! propagation, ported one to one from `net.minecraft.block.RedstoneWireBlock`.

use crate::block::{Block, directions, facings};
use crate::world::{Pos, World};

/// `RedstoneWireBlock.canBePlaced`.
pub fn can_be_placed(world: &World, x: i32, y: i32, z: i32) -> bool {
    world.has_solid_top(x, y - 1, z) || world.get_block(x, y - 1, z) == Block::Glowstone
}

/// `RedstoneWireBlock.updatePower`.
pub fn update_power(world: &mut World, x: i32, y: i32, z: i32) {
    do_update_power(world, x, y, z, x, y, z);
    // Java collects the neighbors in a HashSet and walks the copy; this port
    // keeps them in first-seen order, which the final state does not depend on.
    let neighbors = std::mem::take(&mut world.wire_neighbors_to_update);
    for neighbor in &neighbors {
        world.update_neighbors(neighbor.x, neighbor.y, neighbor.z, Block::RedstoneWire);
    }
}

/// `RedstoneWireBlock.doUpdatePower`.
fn do_update_power(
    world: &mut World,
    x: i32,
    y: i32,
    z: i32,
    source_x: i32,
    source_y: i32,
    source_z: i32,
) {
    let meta = world.get_block_metadata(x, y, z);
    let mut power = 0;
    power = get_highest_wire_power(world, source_x, source_y, source_z, power);
    world.wire_should_signal = false;
    let neighbor = world.get_neighbor_signal(x, y, z);
    world.wire_should_signal = true;
    if neighbor > 0 && neighbor > power - 1 {
        power = neighbor;
    }

    let mut highest = 0;
    for var12 in 0..4 {
        let (mut var13, mut var14) = (x, z);
        match var12 {
            0 => var13 -= 1,
            1 => var13 += 1,
            2 => var14 -= 1,
            3 => var14 += 1,
            _ => {}
        }

        if var13 != source_x || var14 != source_z {
            highest = get_highest_wire_power(world, var13, y, var14, highest);
        }

        if world.get_block(var13, y, var14).is_solid() && !world.get_block(x, y + 1, z).is_solid() {
            if (var13 != source_x || var14 != source_z) && y >= source_y {
                highest = get_highest_wire_power(world, var13, y + 1, var14, highest);
            }
        } else if !world.get_block(var13, y, var14).is_solid()
            && (var13 != source_x || var14 != source_z)
            && y <= source_y
        {
            highest = get_highest_wire_power(world, var13, y - 1, var14, highest);
        }
    }

    if highest > power {
        power = highest - 1;
    } else if power > 0 {
        power -= 1;
    } else {
        power = 0;
    }

    if neighbor > power - 1 {
        power = neighbor;
    }

    if meta != power {
        world.set_block_metadata(x, y, z, power, 2);
        world.wire_neighbors_to_update.push(Pos::new(x, y, z));
        world.wire_neighbors_to_update.push(Pos::new(x - 1, y, z));
        world.wire_neighbors_to_update.push(Pos::new(x + 1, y, z));
        world.wire_neighbors_to_update.push(Pos::new(x, y - 1, z));
        world.wire_neighbors_to_update.push(Pos::new(x, y + 1, z));
        world.wire_neighbors_to_update.push(Pos::new(x, y, z - 1));
        world.wire_neighbors_to_update.push(Pos::new(x, y, z + 1));
    }
}

/// `RedstoneWireBlock.getHighestWirePower`.
fn get_highest_wire_power(world: &World, x: i32, y: i32, z: i32, wire_power: i32) -> i32 {
    if world.get_block(x, y, z) != Block::RedstoneWire {
        return wire_power;
    }
    let meta = world.get_block_metadata(x, y, z);
    if meta > wire_power { meta } else { wire_power }
}

/// `RedstoneWireBlock.onAdded`.
pub fn on_added(world: &mut World, x: i32, y: i32, z: i32) {
    update_power(world, x, y, z);
    world.update_neighbors(x, y + 1, z, Block::RedstoneWire);
    world.update_neighbors(x, y - 1, z, Block::RedstoneWire);
    update_neighbors_of_wire(world, x - 1, y, z);
    update_neighbors_of_wire(world, x + 1, y, z);
    update_neighbors_of_wire(world, x, y, z - 1);
    update_neighbors_of_wire(world, x, y, z + 1);
    if world.get_block(x - 1, y, z).is_solid() {
        update_neighbors_of_wire(world, x - 1, y + 1, z);
    } else {
        update_neighbors_of_wire(world, x - 1, y - 1, z);
    }
    if world.get_block(x + 1, y, z).is_solid() {
        update_neighbors_of_wire(world, x + 1, y + 1, z);
    } else {
        update_neighbors_of_wire(world, x + 1, y - 1, z);
    }
    if world.get_block(x, y, z - 1).is_solid() {
        update_neighbors_of_wire(world, x, y + 1, z - 1);
    } else {
        update_neighbors_of_wire(world, x, y - 1, z - 1);
    }
    if world.get_block(x, y, z + 1).is_solid() {
        update_neighbors_of_wire(world, x, y + 1, z + 1);
    } else {
        update_neighbors_of_wire(world, x, y - 1, z + 1);
    }
}

/// `RedstoneWireBlock.onRemoved`.
pub fn on_removed(world: &mut World, x: i32, y: i32, z: i32) {
    world.update_neighbors(x, y + 1, z, Block::RedstoneWire);
    world.update_neighbors(x, y - 1, z, Block::RedstoneWire);
    world.update_neighbors(x + 1, y, z, Block::RedstoneWire);
    world.update_neighbors(x - 1, y, z, Block::RedstoneWire);
    world.update_neighbors(x, y, z + 1, Block::RedstoneWire);
    world.update_neighbors(x, y, z - 1, Block::RedstoneWire);
    update_power(world, x, y, z);
    update_neighbors_of_wire(world, x - 1, y, z);
    update_neighbors_of_wire(world, x + 1, y, z);
    update_neighbors_of_wire(world, x, y, z - 1);
    update_neighbors_of_wire(world, x, y, z + 1);
    if world.get_block(x - 1, y, z).is_solid() {
        update_neighbors_of_wire(world, x - 1, y + 1, z);
    } else {
        update_neighbors_of_wire(world, x - 1, y - 1, z);
    }
    if world.get_block(x + 1, y, z).is_solid() {
        update_neighbors_of_wire(world, x + 1, y + 1, z);
    } else {
        update_neighbors_of_wire(world, x + 1, y - 1, z);
    }
    if world.get_block(x, y, z - 1).is_solid() {
        update_neighbors_of_wire(world, x, y + 1, z - 1);
    } else {
        update_neighbors_of_wire(world, x, y - 1, z - 1);
    }
    if world.get_block(x, y, z + 1).is_solid() {
        update_neighbors_of_wire(world, x, y + 1, z + 1);
    } else {
        update_neighbors_of_wire(world, x, y - 1, z + 1);
    }
}

/// `RedstoneWireBlock.updateNeighborsOfWire`.
fn update_neighbors_of_wire(world: &mut World, x: i32, y: i32, z: i32) {
    if world.get_block(x, y, z) == Block::RedstoneWire {
        world.update_neighbors(x, y, z, Block::RedstoneWire);
        world.update_neighbors(x - 1, y, z, Block::RedstoneWire);
        world.update_neighbors(x + 1, y, z, Block::RedstoneWire);
        world.update_neighbors(x, y, z - 1, Block::RedstoneWire);
        world.update_neighbors(x, y, z + 1, Block::RedstoneWire);
        world.update_neighbors(x, y - 1, z, Block::RedstoneWire);
        world.update_neighbors(x, y + 1, z, Block::RedstoneWire);
    }
}

/// `RedstoneWireBlock.neighborChanged`.
pub fn neighbor_changed(world: &mut World, x: i32, y: i32, z: i32) {
    if can_be_placed(world, x, y, z) {
        update_power(world, x, y, z);
    } else {
        world.drop_items(x, y, z, 0, 0);
        world.remove_block(x, y, z);
    }
}

/// `RedstoneWireBlock.getDirectSignal`.
pub fn get_direct_signal(world: &World, x: i32, y: i32, z: i32, dir: i32) -> i32 {
    if !world.wire_should_signal {
        0
    } else {
        get_signal(world, x, y, z, dir)
    }
}

/// `RedstoneWireBlock.getSignal`: the weak power the wire emits, from its
/// connection geometry. `dir` is `World.getSignal`'s direction from the asking
/// block toward the wire.
pub fn get_signal(world: &World, x: i32, y: i32, z: i32, dir: i32) -> i32 {
    if !world.wire_should_signal {
        return 0;
    }
    let meta = world.get_block_metadata(x, y, z);
    if meta == 0 {
        return 0;
    }
    if dir == directions::UP {
        return meta;
    }

    let mut west = connects_to(world, x - 1, y, z, 1)
        || !world.get_block(x - 1, y, z).is_solid() && connects_to(world, x - 1, y - 1, z, -1);
    let mut east = connects_to(world, x + 1, y, z, 3)
        || !world.get_block(x + 1, y, z).is_solid() && connects_to(world, x + 1, y - 1, z, -1);
    let mut north = connects_to(world, x, y, z - 1, 2)
        || !world.get_block(x, y, z - 1).is_solid() && connects_to(world, x, y - 1, z - 1, -1);
    let mut south = connects_to(world, x, y, z + 1, 0)
        || !world.get_block(x, y, z + 1).is_solid() && connects_to(world, x, y - 1, z + 1, -1);
    if !world.get_block(x, y + 1, z).is_solid() {
        if world.get_block(x - 1, y, z).is_solid() && connects_to(world, x - 1, y + 1, z, -1) {
            west = true;
        }
        if world.get_block(x + 1, y, z).is_solid() && connects_to(world, x + 1, y + 1, z, -1) {
            east = true;
        }
        if world.get_block(x, y, z - 1).is_solid() && connects_to(world, x, y + 1, z - 1, -1) {
            north = true;
        }
        if world.get_block(x, y, z + 1).is_solid() && connects_to(world, x, y + 1, z + 1, -1) {
            south = true;
        }
    }

    if !north && !east && !west && !south {
        // A wire with no connection at all points every horizontal way.
        return if (directions::NORTH..=directions::EAST).contains(&dir) {
            meta
        } else {
            0
        };
    }
    let points = match dir {
        directions::NORTH => north && !west && !east,
        directions::SOUTH => south && !west && !east,
        directions::WEST => west && !north && !south,
        directions::EAST => east && !north && !south,
        _ => false,
    };
    if points { meta } else { 0 }
}

/// `RedstoneWireBlock.shouldConnectTo`.
pub fn should_connect_to(world: &World, x: i32, y: i32, z: i32, side: i32) -> bool {
    let block = world.get_block(x, y, z);
    if block == Block::RedstoneWire {
        return true;
    }
    if !crate::block::is_repeater(block) {
        return block.is_signal_source() && side != -1;
    }
    let meta = world.get_block_metadata(x, y, z);
    side == (meta & 3) || side == facings::OPPOSITE[(meta & 3) as usize]
}

/// `RedstoneWireBlock.connectsTo`.
pub fn connects_to(world: &World, x: i32, y: i32, z: i32, side: i32) -> bool {
    if should_connect_to(world, x, y, z, side) {
        return true;
    }
    if world.get_block(x, y, z) == Block::PoweredRepeater {
        let meta = world.get_block_metadata(x, y, z);
        return side == (meta & 3);
    }
    false
}
