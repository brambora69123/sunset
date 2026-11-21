//! `HopperBlockEntity` and `HopperBlock`: the transfer tick, push into the
//! target inventory, pull from above, and the redstone switch.

use crate::block::{Block, directions};
use crate::world::{BlockEntity, Pos, World};

/// `HopperBlock.isEnabled`: metadata bit 8 clear.
fn is_enabled(meta: i32) -> bool {
    (meta & 8) != 8
}

/// `HopperBlockEntity.tick`: the cooldown counts down, then a transfer runs.
pub fn tick_hopper(world: &mut World, pos: Pos) {
    let Some(BlockEntity::Inventory(inventory)) = world.block_entities.get_mut(&pos) else {
        return;
    };
    inventory.transfer_cooldown -= 1;
    if !inventory.is_on_cooldown() {
        inventory.set_cooldown(0);
        transfer_items(world, pos);
    }
}

/// `HopperBlockEntity.transferItems`.
fn transfer_items(world: &mut World, pos: Pos) {
    if !is_enabled(world.get_block_metadata(pos.x, pos.y, pos.z)) {
        return;
    }
    let on_cooldown = match world.block_entity(pos) {
        Some(BlockEntity::Inventory(inventory)) => inventory.is_on_cooldown(),
        _ => return,
    };
    if on_cooldown {
        return;
    }

    let mut moved = false;
    if !is_inventory_empty(world, pos) {
        moved = push_item_out(world, pos);
    }
    if !is_inventory_full(world, pos) {
        moved = pull_item(world, pos) || moved;
    }
    if moved && let Some(BlockEntity::Inventory(inventory)) = world.block_entities.get_mut(&pos) {
        inventory.set_cooldown(8);
    }
}

fn is_inventory_empty(world: &World, pos: Pos) -> bool {
    match world.block_entity(pos) {
        Some(BlockEntity::Inventory(inventory)) => inventory.is_empty(),
        _ => true,
    }
}

fn is_inventory_full(world: &World, pos: Pos) -> bool {
    match world.block_entity(pos) {
        Some(BlockEntity::Inventory(inventory)) => inventory.is_full(),
        _ => false,
    }
}

/// `HopperBlockEntity.pushItem`: one item into the inventory the hopper faces.
fn push_item_out(world: &mut World, pos: Pos) -> bool {
    let facing = (world.get_block_metadata(pos.x, pos.y, pos.z) & 7) as usize;
    let target_pos = pos.offset(facing as i32);
    let side = directions::OPPOSITE[facing];

    // HopperBlockEntity.isFullInventory over the target.
    let target_full = match world.block_entity(target_pos) {
        Some(BlockEntity::Inventory(inventory)) => inventory.is_full_inventory(),
        _ => return false,
    };
    if target_full {
        return false;
    }

    let size = match world.block_entity(pos) {
        Some(BlockEntity::Inventory(inventory)) => inventory.size(),
        _ => return false,
    };
    for slot in 0..size {
        let original = match world.block_entity(pos) {
            Some(BlockEntity::Inventory(inventory)) => inventory.get_item(slot).cloned(),
            _ => return false,
        };
        let Some(original) = original else { continue };
        let removed = {
            let Some(BlockEntity::Inventory(inventory)) = world.block_entities.get_mut(&pos) else {
                return false;
            };
            inventory.remove_item(slot, 1).expect("item was present")
        };
        let rest = {
            let Some(target) = world.get_inventory_at(target_pos) else {
                return false;
            };
            target.push_item(removed, side)
        };
        match rest {
            None => return true,
            Some(rest) if rest.is_empty() => return true,
            _ => {
                if let Some(BlockEntity::Inventory(inventory)) = world.block_entities.get_mut(&pos)
                {
                    inventory.set_item(slot, Some(original));
                }
            }
        }
    }
    false
}

/// `HopperBlockEntity.pullItem`: one item from the inventory above.
fn pull_item(world: &mut World, pos: Pos) -> bool {
    let above = pos.offset(directions::UP);
    if !matches!(world.block_entity(above), Some(BlockEntity::Inventory(_))) {
        return false;
    }
    let empty = match world.block_entity(above) {
        Some(BlockEntity::Inventory(inventory)) => inventory.is_empty(),
        _ => return false,
    };
    if empty {
        return false;
    }

    let size = match world.block_entity(above) {
        Some(BlockEntity::Inventory(inventory)) => inventory.size(),
        _ => return false,
    };
    for slot in 0..size {
        let original = match world.block_entity(above) {
            Some(BlockEntity::Inventory(inventory)) => inventory.get_item(slot).cloned(),
            _ => return false,
        };
        let Some(original) = original else { continue };
        let removed = {
            let Some(BlockEntity::Inventory(inventory)) = world.block_entities.get_mut(&above)
            else {
                return false;
            };
            inventory.remove_item(slot, 1).expect("item was present")
        };
        let rest = {
            let Some(inventory) = world.get_inventory_at(pos) else {
                return false;
            };
            inventory.push_item(removed, -1)
        };
        match rest {
            None => return true,
            Some(rest) if rest.is_empty() => return true,
            _ => {
                if let Some(BlockEntity::Inventory(inventory)) =
                    world.block_entities.get_mut(&above)
                {
                    inventory.set_item(slot, Some(original));
                }
            }
        }
    }
    false
}

/// `HopperBlock.neighborChanged` / `updateEnabled`.
pub fn neighbor_changed(world: &mut World, x: i32, y: i32, z: i32) {
    update_enabled(world, x, y, z);
}

fn update_enabled(world: &mut World, x: i32, y: i32, z: i32) {
    let meta = world.get_block_metadata(x, y, z);
    let facing = meta & 7;
    let should = !world.has_neighbor_signal(x, y, z);
    let was = is_enabled(meta);
    if should != was {
        world.set_block_metadata(x, y, z, facing | if should { 0 } else { 8 }, 4);
    }
}

/// `HopperBlock.onAdded`.
pub fn on_added(world: &mut World, x: i32, y: i32, z: i32) {
    update_enabled(world, x, y, z);
}

/// `HopperBlock.onRemoved`: the comparators around the hopper react, then the
/// entity is removed (`BlockWithBlockEntity.onRemoved`).
pub fn on_removed(world: &mut World, x: i32, y: i32, z: i32) {
    world.update_neighbor_comparators(x, y, z, Block::Hopper);
    world.remove_block_entity(Pos::new(x, y, z));
}
