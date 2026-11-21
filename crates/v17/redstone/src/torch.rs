//! `RedstoneTorchBlock` and `UnlitRedstoneTorchBlock`: the torch toggle, the
//! 8-toggle burnout check and the attachment semantics.

use crate::block::Block;
use crate::world::World;

/// `TorchBlock.canSitOnTop` with the fence and wall variants not represented:
/// only a solid top supports a torch here.
fn can_sit_on_top(world: &World, x: i32, y: i32, z: i32) -> bool {
    world.has_solid_top(x, y, z)
}

/// `TorchBlock.canBePlaced`.
fn can_be_placed(world: &World, x: i32, y: i32, z: i32) -> bool {
    if world.is_solid_blocking_cube(x - 1, y, z) {
        return true;
    }
    if world.is_solid_blocking_cube(x + 1, y, z) {
        return true;
    }
    if world.is_solid_blocking_cube(x, y, z - 1) {
        return true;
    }
    if world.is_solid_blocking_cube(x, y, z + 1) {
        return true;
    }
    can_sit_on_top(world, x, y - 1, z)
}

/// `TorchBlock.canSurviveOrBreak`.
fn can_survive_or_break(world: &mut World, x: i32, y: i32, z: i32) -> bool {
    if !can_be_placed(world, x, y, z) {
        world.drop_items(x, y, z, world.get_block_metadata(x, y, z), 0);
        world.remove_block(x, y, z);
        return false;
    }
    true
}

/// `TorchBlock.onAdded`: orientation metadata for a standing torch, then the
/// survival check.
pub fn place(world: &mut World, x: i32, y: i32, z: i32) {
    if world.get_block_metadata(x, y, z) == 0 {
        if world.is_solid_blocking_cube(x - 1, y, z) {
            world.set_block_metadata(x, y, z, 1, 2);
        } else if world.is_solid_blocking_cube(x + 1, y, z) {
            world.set_block_metadata(x, y, z, 2, 2);
        } else if world.is_solid_blocking_cube(x, y, z - 1) {
            world.set_block_metadata(x, y, z, 3, 2);
        } else if world.is_solid_blocking_cube(x, y, z + 1) {
            world.set_block_metadata(x, y, z, 4, 2);
        } else if can_sit_on_top(world, x, y - 1, z) {
            world.set_block_metadata(x, y, z, 5, 2);
        }
    }
    can_survive_or_break(world, x, y, z);
}

/// `RedstoneTorchBlock.getSignal`: the lit torch emits 15 toward every
/// neighbor but the one it hangs on (`dir` is the side from that neighbor to
/// the torch).
pub fn get_signal(world: &World, x: i32, y: i32, z: i32, dir: i32) -> i32 {
    if world.get_block(x, y, z) != Block::RedstoneTorch {
        return 0;
    }
    let meta = world.get_block_metadata(x, y, z);
    if meta == 5 && dir == 1 {
        return 0;
    }
    if meta == 3 && dir == 3 {
        return 0;
    }
    if meta == 4 && dir == 2 {
        return 0;
    }
    if meta == 1 && dir == 5 {
        return 0;
    }
    if meta == 2 && dir == 4 {
        return 0;
    }
    15
}

/// `RedstoneTorchBlock.getDirectSignal`: strong power through the face the
/// block above touches (dir 0).
pub fn get_direct_signal(world: &World, x: i32, y: i32, z: i32, dir: i32) -> i32 {
    if dir == 0 {
        get_signal(world, x, y, z, dir)
    } else {
        0
    }
}

/// `RedstoneTorchBlock.hasNeighborSignal`: the block the torch hangs on is
/// powered.
fn attachment_is_powered(world: &World, x: i32, y: i32, z: i32) -> bool {
    let meta = world.get_block_metadata(x, y, z);
    if meta == 5 && world.has_signal(x, y - 1, z, 0) {
        return true;
    }
    if meta == 3 && world.has_signal(x, y, z - 1, 2) {
        return true;
    }
    if meta == 4 && world.has_signal(x, y, z + 1, 3) {
        return true;
    }
    if meta == 1 && world.has_signal(x - 1, y, z, 4) {
        return true;
    }
    meta == 2 && world.has_signal(x + 1, y, z, 5)
}

/// `RedstoneTorchBlock.shouldBurnOut`: the world's recent-toggle list, pruned
/// to 60 ticks by `tick`.
fn should_burn_out(world: &mut World, x: i32, y: i32, z: i32, log_toggle: bool) -> bool {
    if log_toggle {
        world.torch_toggles.push(crate::world::ToggleEntry {
            x,
            y,
            z,
            time: world.time,
        });
    }
    let mut count = 0;
    for entry in &world.torch_toggles {
        if entry.x == x && entry.y == y && entry.z == z {
            count += 1;
            if count >= 8 {
                return true;
            }
        }
    }
    false
}

/// `RedstoneTorchBlock.tick` (tick rate 2): swap the block between lit and
/// unlit, logging toggles for the burnout check.
pub fn tick(world: &mut World, x: i32, y: i32, z: i32) {
    let lit = world.get_block(x, y, z) == Block::RedstoneTorch;
    let powered = attachment_is_powered(world, x, y, z);
    world.torch_toggles.retain(|e| world.time - e.time <= 60);
    if lit {
        if powered {
            let meta = world.get_block_metadata(x, y, z);
            world.set_block_with_metadata(x, y, z, Block::UnlitRedstoneTorch, meta, 3);
            if should_burn_out(world, x, y, z, true) {
                // fizz sound and smoke particles are not ported
            }
        }
    } else if !powered && !should_burn_out(world, x, y, z, false) {
        let meta = world.get_block_metadata(x, y, z);
        world.set_block_with_metadata(x, y, z, Block::RedstoneTorch, meta, 3);
    }
}

/// `RedstoneTorchBlock.neighborChanged`: schedule the toggle when the torch's
/// state no longer matches its attachment.
pub fn neighbor_changed(world: &mut World, x: i32, y: i32, z: i32) {
    if !can_survive_or_break(world, x, y, z) {
        return;
    }
    let lit = world.get_block(x, y, z) == Block::RedstoneTorch;
    let powered = attachment_is_powered(world, x, y, z);
    if (lit && powered) || (!lit && !powered) {
        let block = world.get_block(x, y, z);
        world.schedule_tick(x, y, z, block, 2, 0);
    }
}

/// `RedstoneTorchBlock.onAdded`: orientation, then the lit torch updates its
/// neighbors.
pub fn on_added(world: &mut World, x: i32, y: i32, z: i32, block: Block) {
    if world.get_block_metadata(x, y, z) == 0 {
        place(world, x, y, z);
    }
    if block == Block::RedstoneTorch {
        world.update_neighbors(x, y - 1, z, block);
        world.update_neighbors(x, y + 1, z, block);
        world.update_neighbors(x - 1, y, z, block);
        world.update_neighbors(x + 1, y, z, block);
        world.update_neighbors(x, y, z - 1, block);
        world.update_neighbors(x, y, z + 1, block);
    }
}

/// `RedstoneTorchBlock.onRemoved`: the lit torch updates its neighbors.
pub fn on_removed(world: &mut World, x: i32, y: i32, z: i32, block: Block) {
    if block == Block::RedstoneTorch {
        world.update_neighbors(x, y - 1, z, block);
        world.update_neighbors(x, y + 1, z, block);
        world.update_neighbors(x - 1, y, z, block);
        world.update_neighbors(x + 1, y, z, block);
        world.update_neighbors(x, y, z - 1, block);
        world.update_neighbors(x, y, z + 1, block);
    }
}
