//! `LeverBlock`: the toggle switch, its strong-power faces and survival.

use crate::block::Block;
use crate::world::World;

/// `LeverBlock.getSignal`: 15 on every face while on.
pub fn get_signal(_world: &World, x: i32, y: i32, z: i32) -> i32 {
    if (world_metadata(_world, x, y, z) & 8) > 0 {
        15
    } else {
        0
    }
}

fn world_metadata(world: &World, x: i32, y: i32, z: i32) -> i32 {
    world.get_block_metadata(x, y, z)
}

/// `LeverBlock.getDirectSignal`: strong power through the face the lever is
/// mounted on.
pub fn get_direct_signal(world: &World, x: i32, y: i32, z: i32, dir: i32) -> i32 {
    let meta = world.get_block_metadata(x, y, z);
    if (meta & 8) == 0 {
        return 0;
    }
    match meta & 7 {
        0 if dir == 0 => 15,
        7 if dir == 0 => 15,
        6 if dir == 1 => 15,
        5 if dir == 1 => 15,
        4 if dir == 2 => 15,
        3 if dir == 3 => 15,
        2 if dir == 4 => 15,
        1 if dir == 5 => 15,
        _ => 0,
    }
}

/// `LeverBlock.canBePlaced`.
fn can_be_placed(world: &World, x: i32, y: i32, z: i32) -> bool {
    if world.get_block(x - 1, y, z).is_solid() {
        return true;
    }
    if world.get_block(x + 1, y, z).is_solid() {
        return true;
    }
    if world.get_block(x, y, z - 1).is_solid() {
        return true;
    }
    if world.get_block(x, y, z + 1).is_solid() {
        return true;
    }
    world.has_solid_top(x, y - 1, z) || world.get_block(x, y + 1, z).is_solid()
}

/// `LeverBlock.use`: flip the state, then update the mount face's neighbors.
pub fn use_lever(world: &mut World, x: i32, y: i32, z: i32) -> bool {
    let meta = world.get_block_metadata(x, y, z);
    let facing = meta & 7;
    let new_state = 8 - (meta & 8);
    world.set_block_metadata(x, y, z, facing + new_state, 3);
    world.update_neighbors(x, y, z, Block::Lever);
    match facing {
        1 => world.update_neighbors(x - 1, y, z, Block::Lever),
        2 => world.update_neighbors(x + 1, y, z, Block::Lever),
        3 => world.update_neighbors(x, y, z - 1, Block::Lever),
        4 => world.update_neighbors(x, y, z + 1, Block::Lever),
        5 | 6 => world.update_neighbors(x, y - 1, z, Block::Lever),
        0 | 7 => world.update_neighbors(x, y + 1, z, Block::Lever),
        _ => {}
    }
    true
}

/// `LeverBlock.neighborChanged`: break when the mount is gone.
pub fn neighbor_changed(world: &mut World, x: i32, y: i32, z: i32) {
    if !can_be_placed(world, x, y, z) {
        world.drop_items(x, y, z, world.get_block_metadata(x, y, z), 0);
        world.remove_block(x, y, z);
        return;
    }
    let facing = world.get_block_metadata(x, y, z) & 7;
    let mut break_ = false;
    if !world.get_block(x - 1, y, z).is_solid() && facing == 1 {
        break_ = true;
    }
    if !world.get_block(x + 1, y, z).is_solid() && facing == 2 {
        break_ = true;
    }
    if !world.get_block(x, y, z - 1).is_solid() && facing == 3 {
        break_ = true;
    }
    if !world.get_block(x, y, z + 1).is_solid() && facing == 4 {
        break_ = true;
    }
    if !world.has_solid_top(x, y - 1, z) && (facing == 5 || facing == 6) {
        break_ = true;
    }
    if !world.get_block(x, y + 1, z).is_solid() && (facing == 0 || facing == 7) {
        break_ = true;
    }
    if break_ {
        world.drop_items(x, y, z, world.get_block_metadata(x, y, z), 0);
        world.remove_block(x, y, z);
    }
}

/// `LeverBlock.onRemoved`: an on lever updates its neighbors.
pub fn on_removed(world: &mut World, x: i32, y: i32, z: i32, meta: i32) {
    if (meta & 8) > 0 {
        world.update_neighbors(x, y, z, Block::Lever);
        match meta & 7 {
            1 => world.update_neighbors(x - 1, y, z, Block::Lever),
            2 => world.update_neighbors(x + 1, y, z, Block::Lever),
            3 => world.update_neighbors(x, y, z - 1, Block::Lever),
            4 => world.update_neighbors(x, y, z + 1, Block::Lever),
            5 | 6 => world.update_neighbors(x, y - 1, z, Block::Lever),
            0 | 7 => world.update_neighbors(x, y + 1, z, Block::Lever),
            _ => {}
        }
    }
}
