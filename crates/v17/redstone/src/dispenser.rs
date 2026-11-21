//! `DispenserBlock` and `DropperBlock`: the powered tick, the default dispense
//! behavior and the dropper's insert-into-inventory path.

use crate::block::{Block, directions};
use crate::world::{BlockEntity, Pos, World};
use sunset_1_7_core::RandomSource;

/// `DispenserBlock.getTickRate`.
fn tick_rate() -> i32 {
    4
}

/// `DispenserBlock.onAdded` / `updateFacing`: point away from opaque
/// neighbors.
pub fn on_added(world: &mut World, x: i32, y: i32, z: i32) {
    let north = world.get_block(x, y, z - 1);
    let south = world.get_block(x, y, z + 1);
    let west = world.get_block(x - 1, y, z);
    let east = world.get_block(x + 1, y, z);
    let mut facing = 3;
    if north.is_opaque() && !south.is_opaque() {
        facing = 3;
    }
    if south.is_opaque() && !north.is_opaque() {
        facing = 2;
    }
    if west.is_opaque() && !east.is_opaque() {
        facing = 5;
    }
    if east.is_opaque() && !west.is_opaque() {
        facing = 4;
    }
    world.set_block_metadata(x, y, z, facing, 2);
}

/// `DispenserBlock.neighborChanged`: the block above counts too
/// (`hasNeighborSignal(x, y + 1, z)` is vanilla behavior).
pub fn neighbor_changed(world: &mut World, x: i32, y: i32, z: i32) {
    let powered = world.has_neighbor_signal(x, y, z) || world.has_neighbor_signal(x, y + 1, z);
    let meta = world.get_block_metadata(x, y, z);
    let was_on = (meta & 8) != 0;
    if powered && !was_on {
        let block = world.get_block(x, y, z);
        world.schedule_tick(x, y, z, block, tick_rate(), 0);
        world.set_block_metadata(x, y, z, meta | 8, 4);
    } else if !powered && was_on {
        world.set_block_metadata(x, y, z, meta & !8, 4);
    }
}

/// `DispenserBlock.tick`: dispense.
pub fn tick(world: &mut World, pos: Pos) {
    dispense(world, pos.x, pos.y, pos.z);
}

/// `DispenserBlock.dispense` with the `DropperBlock.dispense` override.
fn dispense(world: &mut World, x: i32, y: i32, z: i32) {
    let dropper = world.get_block(x, y, z) == Block::Dropper;
    let pos = Pos::new(x, y, z);
    let slot = match world.block_entities.get_mut(&pos) {
        Some(BlockEntity::Inventory(inventory)) => inventory.pick_non_empty_slot(),
        _ => return,
    };
    if slot < 0 {
        world.do_event(1001, x, y, z, 0);
        return;
    }

    let stack = match world.block_entities.get(&pos) {
        Some(BlockEntity::Inventory(inventory)) => inventory.get_item(slot as usize).cloned(),
        _ => return,
    };
    let Some(stack) = stack else { return };

    if dropper {
        let dir = (world.get_block_metadata(x, y, z) & 7) as usize;
        let target_pos = Pos::new(
            x + directions::X_OFFSET[dir],
            y + directions::Y_OFFSET[dir],
            z + directions::Z_OFFSET[dir],
        );
        let side = directions::OPPOSITE[dir];
        if world.get_inventory_at(target_pos).is_some() {
            // HopperBlockEntity.pushItem(var10, var8.copy().split(1), OPPOSITE)
            let mut copy = stack.clone();
            let one = copy.split(1);
            let rest = {
                let target = world.get_inventory_at(target_pos).expect("checked above");
                target.push_item(one, side)
            };
            let result = if rest.is_none() {
                let mut original = stack.clone();
                original.size -= 1;
                if original.size == 0 {
                    None
                } else {
                    Some(original)
                }
            } else {
                Some(stack.clone())
            };
            if let Some(BlockEntity::Inventory(inventory)) = world.block_entities.get_mut(&pos) {
                inventory.set_item(slot as usize, result);
            }
            return;
        }
    }

    // The default dispense behavior: split one item and record the spawn.
    let meta = world.get_block_metadata(x, y, z);
    let dir = (meta & 7) as usize;
    let mut remaining = stack;
    let dropped = remaining.split(1);

    // DispenserBlock.getDispensePos.
    let px = x as f64 + 0.7 * directions::X_OFFSET[dir] as f64;
    let py = y as f64 + 0.7 * directions::Y_OFFSET[dir] as f64;
    let pz = z as f64 + 0.7 * directions::Z_OFFSET[dir] as f64;

    // DispenseItemBehavior.spawnItem: the velocity draws happen in vanilla
    // order even though entities are not simulated.
    let speed = world.rng.next_double() * 0.1 + 0.2;
    let mut vx = directions::X_OFFSET[dir] as f64 * speed;
    let vy = 0.2f64;
    let mut vz = directions::Z_OFFSET[dir] as f64 * speed;
    vx += world.rng.next_gaussian() * 0.0075 * 6.0;
    let vy = vy + world.rng.next_gaussian() * 0.0075 * 6.0;
    vz += world.rng.next_gaussian() * 0.0075 * 6.0;
    world.item_drops.push((px, py - 0.3, pz, dropped));
    let _ = (vx, vy, vz);

    world.do_event(1000, x, y, z, 0);
    // DispenseItemBehavior.doWorldEvent: the direction packed for particles.
    let packed = directions::X_OFFSET[dir] + 1 + (directions::Z_OFFSET[dir] + 1) * 3;
    world.do_event(2000, x, y, z, packed);

    if let Some(BlockEntity::Inventory(inventory)) = world.block_entities.get_mut(&pos) {
        inventory.set_item(
            slot as usize,
            if remaining.size == 0 {
                None
            } else {
                Some(remaining)
            },
        );
    }
}

/// `DispenserBlock.onRemoved`: the comparators around the container react,
/// then the entity is removed (`BlockWithBlockEntity.onRemoved`).
pub fn on_removed(world: &mut World, x: i32, y: i32, z: i32, block: Block) {
    world.update_neighbor_comparators(x, y, z, block);
    world.remove_block_entity(Pos::new(x, y, z));
}
