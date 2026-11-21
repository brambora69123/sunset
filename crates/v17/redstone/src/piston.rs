//! `PistonBaseBlock`, `PistonHeadBlock` and `MovingBlock`: extension checks
//! with quasi-connectivity, the push list, and the block event that moves
//! blocks, ported one to one.

use crate::block::{Block, directions};
use crate::world::{BlockEntity, MovingBlockEntity, Pos, World};

/// `PistonBaseBlock.getFacing`.
pub fn get_facing(meta: i32) -> i32 {
    meta & 7
}

/// `PistonBaseBlock.isExtended`.
pub fn is_extended(meta: i32) -> bool {
    (meta & 8) != 0
}

fn is_piston(block: Block) -> bool {
    block == Block::Piston || block == Block::StickyPiston
}

/// `PistonBaseBlock.neighborChanged`.
pub fn neighbor_changed(world: &mut World, x: i32, y: i32, z: i32) {
    check_extended(world, x, y, z);
}

/// `PistonBaseBlock.onAdded`.
pub fn on_added(world: &mut World, x: i32, y: i32, z: i32) {
    if world.block_entity(Pos::new(x, y, z)).is_none() {
        check_extended(world, x, y, z);
    }
}

/// `PistonBaseBlock.checkExtended`.
fn check_extended(world: &mut World, x: i32, y: i32, z: i32) {
    let meta = world.get_block_metadata(x, y, z);
    let facing = get_facing(meta);
    if facing != 7 {
        let should = should_extend(world, x, y, z, facing);
        if should && !is_extended(meta) {
            if can_extend(world, x, y, z, facing) {
                let block = world.get_block(x, y, z);
                world.add_block_event(x, y, z, block, 0, facing);
            }
        } else if !should && is_extended(meta) {
            world.set_block_metadata(x, y, z, facing, 2);
            let block = world.get_block(x, y, z);
            world.add_block_event(x, y, z, block, 1, facing);
        }
    }
}

/// `PistonBaseBlock.shouldExtend`: the six neighbors except the facing itself,
/// then the quasi-connectivity check over the five blocks above the piston.
pub fn should_extend(world: &World, x: i32, y: i32, z: i32, facing: i32) -> bool {
    if facing != 0 && world.has_signal(x, y - 1, z, 0) {
        return true;
    }
    if facing != 1 && world.has_signal(x, y + 1, z, 1) {
        return true;
    }
    if facing != 2 && world.has_signal(x, y, z - 1, 2) {
        return true;
    }
    if facing != 3 && world.has_signal(x, y, z + 1, 3) {
        return true;
    }
    if facing != 5 && world.has_signal(x + 1, y, z, 5) {
        return true;
    }
    if facing != 4 && world.has_signal(x - 1, y, z, 4) {
        return true;
    }
    if world.has_signal(x, y, z, 0) {
        return true;
    }
    if world.has_signal(x, y + 2, z, 1) {
        return true;
    }
    if world.has_signal(x, y + 1, z - 1, 2) {
        return true;
    }
    if world.has_signal(x, y + 1, z + 1, 3) {
        return true;
    }
    if world.has_signal(x - 1, y + 1, z, 4) {
        return true;
    }
    world.has_signal(x + 1, y + 1, z, 5)
}

/// `PistonBaseBlock.doEvent` (block events 0 extend, 1 retract).
pub fn do_event(world: &mut World, x: i32, y: i32, z: i32, kind: i32, data: i32) -> bool {
    let sticky = world.get_block(x, y, z) == Block::StickyPiston;
    let should = should_extend(world, x, y, z, data);
    if should && kind == 1 {
        world.set_block_metadata(x, y, z, data | 8, 2);
        return false;
    }
    if !should && kind == 0 {
        return false;
    }

    if kind == 0 {
        if !push(world, x, y, z, data, sticky) {
            return false;
        }
        world.set_block_metadata(x, y, z, data | 8, 2);
        // the tile.piston.out sound is not ported
    } else if kind == 1 {
        let head = Pos::new(x, y, z).offset(data);

        // Finish a moving block still mid-move in front of the piston.
        let mut finish = None;
        if let Some(BlockEntity::MovingPiston(moving)) = world.block_entity(head)
            && moving.last_progress < 1.0
        {
            finish = Some((moving.moved_block, moving.moved_metadata));
        }
        if let Some((moved_block, moved_metadata)) = finish {
            world.remove_block_entity(head);
            if world.get_block(head.x, head.y, head.z) == Block::MovingPiston {
                world.set_block_with_metadata(
                    head.x,
                    head.y,
                    head.z,
                    moved_block,
                    moved_metadata,
                    3,
                );
                let moved = moved_block;
                world.neighbor_changed(head.x, head.y, head.z, moved);
            }
        }

        let base = world.get_block(x, y, z);
        world.set_block_with_metadata(x, y, z, Block::MovingPiston, data, 3);
        world.set_block_entity(
            Pos::new(x, y, z),
            BlockEntity::MovingPiston(MovingBlockEntity::new(base, data, data, false, true)),
        );

        if sticky {
            let far = head.offset(data);
            let mut moved_block = world.get_block(far.x, far.y, far.z);
            let mut moved_meta = world.get_block_metadata(far.x, far.y, far.z);
            let mut pulled = false;
            if moved_block == Block::MovingPiston
                && let Some(BlockEntity::MovingPiston(moving)) = world.block_entity(far)
                && moving.facing == data
                && moving.extending
            {
                pulled = true;
                moved_block = moving.moved_block;
                moved_meta = moving.moved_metadata;
                finish_far_moving(world, far, moved_block, moved_meta);
            }

            if pulled
                || moved_block == Block::Air
                || !can_move_block(world, moved_block, far.x, far.y, far.z, false)
                || moved_block.piston_move_behavior() != 0
                    && moved_block != Block::Piston
                    && moved_block != Block::StickyPiston
            {
                if !pulled {
                    world.remove_block(head.x, head.y, head.z);
                }
            } else {
                world.set_block_with_metadata(
                    head.x,
                    head.y,
                    head.z,
                    Block::MovingPiston,
                    moved_meta,
                    3,
                );
                world.set_block_entity(
                    head,
                    BlockEntity::MovingPiston(MovingBlockEntity::new(
                        moved_block,
                        moved_meta,
                        data,
                        false,
                        false,
                    )),
                );
                world.remove_block(far.x, far.y, far.z);
            }
        } else {
            world.remove_block(head.x, head.y, head.z);
        }
        // the tile.piston.in sound is not ported
    }

    true
}

/// `MovingBlockEntity.finish` for the far moving block a sticky piston pulls.
fn finish_far_moving(world: &mut World, pos: Pos, moved_block: Block, moved_meta: i32) {
    world.remove_block_entity(pos);
    if world.get_block(pos.x, pos.y, pos.z) == Block::MovingPiston {
        world.set_block_with_metadata(pos.x, pos.y, pos.z, moved_block, moved_meta, 3);
        let moved = moved_block;
        world.neighbor_changed(pos.x, pos.y, pos.z, moved);
    }
}

/// `PistonBaseBlock.canMoveBlock`.
pub fn can_move_block(
    world: &World,
    block: Block,
    x: i32,
    y: i32,
    z: i32,
    allow_breaking: bool,
) -> bool {
    if block == Block::Obsidian {
        return false;
    }
    if !is_piston(block) {
        if block.is_unbreakable() {
            return false;
        }
        if block.piston_move_behavior() == 2 {
            return false;
        }
        if block.piston_move_behavior() == 1 {
            if !allow_breaking {
                return false;
            }
            return true;
        }
    } else if is_extended(world.get_block_metadata(x, y, z)) {
        return false;
    }
    !block.has_block_entity()
}

/// `PistonBaseBlock.canExtend`.
fn can_extend(world: &World, x: i32, y: i32, z: i32, dir: i32) -> bool {
    let dir = dir as usize;
    let (mut bx, mut by, mut bz) = (
        x + directions::X_OFFSET[dir],
        y + directions::Y_OFFSET[dir],
        z + directions::Z_OFFSET[dir],
    );
    for var8 in 0..13 {
        if by <= 0 || by >= 255 {
            return false;
        }
        let block = world.get_block(bx, by, bz);
        if block != Block::Air {
            if !can_move_block(world, block, bx, by, bz, true) {
                return false;
            }
            if block.piston_move_behavior() != 1 {
                if var8 == 12 {
                    return false;
                }
                bx += directions::X_OFFSET[dir];
                by += directions::Y_OFFSET[dir];
                bz += directions::Z_OFFSET[dir];
                continue;
            }
        }
        break;
    }
    true
}

/// `PistonBaseBlock.push`: sweep the push list into moving blocks, then
/// update the neighbors of every position the list passed through.
fn push(world: &mut World, x: i32, y: i32, z: i32, dir: i32, sticky: bool) -> bool {
    let dir_u = dir as usize;
    let (mut bx, mut by, mut bz) = (
        x + directions::X_OFFSET[dir_u],
        y + directions::Y_OFFSET[dir_u],
        z + directions::Z_OFFSET[dir_u],
    );
    let mut count = 0;

    while count < 13 {
        if by <= 0 || by >= 255 {
            return false;
        }
        let block = world.get_block(bx, by, bz);
        if block == Block::Air {
            break;
        }
        if !can_move_block(world, block, bx, by, bz, true) {
            return false;
        }
        if block.piston_move_behavior() != 1 {
            if count == 12 {
                return false;
            }
            bx += directions::X_OFFSET[dir_u];
            by += directions::Y_OFFSET[dir_u];
            bz += directions::Z_OFFSET[dir_u];
            count += 1;
        } else {
            world.drop_items(bx, by, bz, world.get_block_metadata(bx, by, bz), 0);
            world.remove_block(bx, by, bz);
            break;
        }
    }

    let (end_x, end_y, end_z) = (bx, by, bz);
    let base = world.get_block(x, y, z);
    let mut blocks: Vec<Block> = Vec::new();
    let (mut cx, mut cy, mut cz) = (bx, by, bz);

    while cx != x || cy != y || cz != z {
        let (px, py, pz) = (
            cx - directions::X_OFFSET[dir_u],
            cy - directions::Y_OFFSET[dir_u],
            cz - directions::Z_OFFSET[dir_u],
        );
        let block = world.get_block(px, py, pz);
        let meta = world.get_block_metadata(px, py, pz);
        if block == base && px == x && py == y && pz == z {
            world.set_block_with_metadata(
                cx,
                cy,
                cz,
                Block::MovingPiston,
                dir | if sticky { 8 } else { 0 },
                4,
            );
            world.set_block_entity(
                Pos::new(cx, cy, cz),
                BlockEntity::MovingPiston(MovingBlockEntity::new(
                    Block::PistonHead,
                    dir | if sticky { 8 } else { 0 },
                    dir,
                    true,
                    false,
                )),
            );
        } else {
            world.set_block_with_metadata(cx, cy, cz, Block::MovingPiston, meta, 4);
            world.set_block_entity(
                Pos::new(cx, cy, cz),
                BlockEntity::MovingPiston(MovingBlockEntity::new(block, meta, dir, true, false)),
            );
        }
        blocks.push(block);
        cx = px;
        cy = py;
        cz = pz;
    }

    let (mut cx, mut cy, mut cz) = (end_x, end_y, end_z);
    let mut index = 0;
    while cx != x || cy != y || cz != z {
        let (px, py, pz) = (
            cx - directions::X_OFFSET[dir_u],
            cy - directions::Y_OFFSET[dir_u],
            cz - directions::Z_OFFSET[dir_u],
        );
        world.update_neighbors(px, py, pz, blocks[index]);
        index += 1;
        cx = px;
        cy = py;
        cz = pz;
    }

    true
}

/// `PistonHeadBlock.neighborChanged`: break without a piston behind, otherwise
/// the piston reacts.
pub fn head_neighbor_changed(world: &mut World, x: i32, y: i32, z: i32, source: Block) {
    let facing = get_facing(world.get_block_metadata(x, y, z)) as usize;
    let base = world.get_block(
        x - directions::X_OFFSET[facing],
        y - directions::Y_OFFSET[facing],
        z - directions::Z_OFFSET[facing],
    );
    if !is_piston(base) {
        world.remove_block(x, y, z);
    } else {
        world.block_neighbor_changed(
            x - directions::X_OFFSET[facing],
            y - directions::Y_OFFSET[facing],
            z - directions::Z_OFFSET[facing],
            base,
            source,
        );
    }
}

/// `PistonHeadBlock.onRemoved`: drop the extended piston behind the head.
pub fn head_on_removed(world: &mut World, x: i32, y: i32, z: i32, meta: i32) {
    let behind = directions::OPPOSITE[(meta & 7) as usize] as usize;
    let (bx, by, bz) = (
        x + directions::X_OFFSET[behind],
        y + directions::Y_OFFSET[behind],
        z + directions::Z_OFFSET[behind],
    );
    let block = world.get_block(bx, by, bz);
    if is_piston(block) {
        let m = world.get_block_metadata(bx, by, bz);
        if is_extended(m) {
            world.drop_items(bx, by, bz, m, 0);
            world.remove_block(bx, by, bz);
        }
    }
}

/// `MovingBlock.onRemoved`: finish the moving block entity, if any.
pub fn moving_on_removed(world: &mut World, x: i32, y: i32, z: i32) {
    let pos = Pos::new(x, y, z);
    if let Some(BlockEntity::MovingPiston(moving)) = world.block_entity(pos)
        && moving.last_progress < 1.0
    {
        let (moved_block, moved_meta) = (moving.moved_block, moving.moved_metadata);
        finish_far_moving(world, pos, moved_block, moved_meta);
    }
}
