//! Stair geometry, ported from `net.minecraft.block.StairsBlock`.
//!
//! Vanilla mutates the singleton block's shape in place; these helpers return
//! the shape instead. `tesselate_stairs` renders base + inner-corner shape +
//! step shape from them, exactly like `RenderBlocks.tesselateStairs`.

use super::blocks::{BlockDef, Bounds, Kind};
use super::world::WorldView;

/// `StairsBlock.isStair`.
pub fn is_stair(block: &BlockDef) -> bool {
    block.kind == Kind::Stairs
}

fn is_stair_same_half_same_facing(
    world: &dyn WorldView,
    x: i32,
    y: i32,
    z: i32,
    metadata: i32,
) -> bool {
    is_stair(world.block(x, y, z)) && world.metadata(x, y, z) == metadata
}

/// `StairsBlock.setBaseShapeForCollisions`: the half slab the stair is made
/// of. Metadata bit 4 flips it to the top half.
pub fn base_shape(world: &dyn WorldView, x: i32, y: i32, z: i32) -> Bounds {
    if world.metadata(x, y, z) & 4 != 0 {
        Bounds::new(0.0, 0.5, 0.0, 1.0, 1.0, 1.0)
    } else {
        Bounds::new(0.0, 0.0, 0.0, 1.0, 0.5, 1.0)
    }
}

/// `StairsBlock.isInnerStair`: the corner or step shape of an inner corner,
/// plus whether the block is one (the return value vanilla's
/// `isInnerStair` gives; false means an inner corner was found).
pub fn inner_shape(world: &dyn WorldView, x: i32, y: i32, z: i32) -> (Bounds, bool) {
    let metadata = world.metadata(x, y, z);
    let facing = metadata & 3;
    let (mut min_y, mut max_y) = (0.5, 1.0);
    if metadata & 4 != 0 {
        min_y = 0.0;
        max_y = 0.5;
    }

    let (mut min_x, mut max_x, mut min_z, mut max_z) = (0.0, 1.0, 0.0, 0.5);
    let mut corner = true;
    if facing == 0 {
        min_x = 0.5;
        max_z = 1.0;
        let neighbour = world.block(x + 1, y, z);
        let neighbour_metadata = world.metadata(x + 1, y, z);
        if is_stair(neighbour) && metadata & 4 == neighbour_metadata & 4 {
            let neighbour_facing = neighbour_metadata & 3;
            if neighbour_facing == 3
                && !is_stair_same_half_same_facing(world, x, y, z + 1, metadata)
            {
                max_z = 0.5;
                corner = false;
            } else if neighbour_facing == 2
                && !is_stair_same_half_same_facing(world, x, y, z - 1, metadata)
            {
                min_z = 0.5;
                corner = false;
            }
        }
    } else if facing == 1 {
        max_x = 0.5;
        max_z = 1.0;
        let neighbour = world.block(x - 1, y, z);
        let neighbour_metadata = world.metadata(x - 1, y, z);
        if is_stair(neighbour) && metadata & 4 == neighbour_metadata & 4 {
            let neighbour_facing = neighbour_metadata & 3;
            if neighbour_facing == 3
                && !is_stair_same_half_same_facing(world, x, y, z + 1, metadata)
            {
                max_z = 0.5;
                corner = false;
            } else if neighbour_facing == 2
                && !is_stair_same_half_same_facing(world, x, y, z - 1, metadata)
            {
                min_z = 0.5;
                corner = false;
            }
        }
    } else if facing == 2 {
        min_z = 0.5;
        max_z = 1.0;
        let neighbour = world.block(x, y, z + 1);
        let neighbour_metadata = world.metadata(x, y, z + 1);
        if is_stair(neighbour) && metadata & 4 == neighbour_metadata & 4 {
            let neighbour_facing = neighbour_metadata & 3;
            if neighbour_facing == 1
                && !is_stair_same_half_same_facing(world, x + 1, y, z, metadata)
            {
                max_x = 0.5;
                corner = false;
            } else if neighbour_facing == 0
                && !is_stair_same_half_same_facing(world, x - 1, y, z, metadata)
            {
                min_x = 0.5;
                corner = false;
            }
        }
    } else if facing == 3 {
        let neighbour = world.block(x, y, z - 1);
        let neighbour_metadata = world.metadata(x, y, z - 1);
        if is_stair(neighbour) && metadata & 4 == neighbour_metadata & 4 {
            let neighbour_facing = neighbour_metadata & 3;
            if neighbour_facing == 1
                && !is_stair_same_half_same_facing(world, x + 1, y, z, metadata)
            {
                max_x = 0.5;
                corner = false;
            } else if neighbour_facing == 0
                && !is_stair_same_half_same_facing(world, x - 1, y, z, metadata)
            {
                min_x = 0.5;
                corner = false;
            }
        }
    }

    (
        Bounds::new(min_x, min_y, min_z, max_x, max_y, max_z),
        corner,
    )
}

/// `StairsBlock.setStepShapeForCollisions`: the missing quadrant of an inner
/// corner, when the stair meets a matching stair.
pub fn step_shape(world: &dyn WorldView, x: i32, y: i32, z: i32) -> Option<Bounds> {
    let metadata = world.metadata(x, y, z);
    let facing = metadata & 3;
    let (mut min_y, mut max_y) = (0.5, 1.0);
    if metadata & 4 != 0 {
        min_y = 0.0;
        max_y = 0.5;
    }

    let (mut min_x, mut max_x, mut min_z, mut max_z) = (0.0, 0.5, 0.5, 1.0);
    let mut has_step = false;
    if facing == 0 {
        let neighbour = world.block(x - 1, y, z);
        let neighbour_metadata = world.metadata(x - 1, y, z);
        if is_stair(neighbour) && metadata & 4 == neighbour_metadata & 4 {
            let neighbour_facing = neighbour_metadata & 3;
            if neighbour_facing == 3
                && !is_stair_same_half_same_facing(world, x, y, z - 1, metadata)
            {
                min_z = 0.0;
                max_z = 0.5;
                has_step = true;
            } else if neighbour_facing == 2
                && !is_stair_same_half_same_facing(world, x, y, z + 1, metadata)
            {
                min_z = 0.5;
                max_z = 1.0;
                has_step = true;
            }
        }
    } else if facing == 1 {
        let neighbour = world.block(x + 1, y, z);
        let neighbour_metadata = world.metadata(x + 1, y, z);
        if is_stair(neighbour) && metadata & 4 == neighbour_metadata & 4 {
            min_x = 0.5;
            max_x = 1.0;
            let neighbour_facing = neighbour_metadata & 3;
            if neighbour_facing == 3
                && !is_stair_same_half_same_facing(world, x, y, z - 1, metadata)
            {
                min_z = 0.0;
                max_z = 0.5;
                has_step = true;
            } else if neighbour_facing == 2
                && !is_stair_same_half_same_facing(world, x, y, z + 1, metadata)
            {
                min_z = 0.5;
                max_z = 1.0;
                has_step = true;
            }
        }
    } else if facing == 2 {
        let neighbour = world.block(x, y, z - 1);
        let neighbour_metadata = world.metadata(x, y, z - 1);
        if is_stair(neighbour) && metadata & 4 == neighbour_metadata & 4 {
            min_z = 0.0;
            max_z = 0.5;
            let neighbour_facing = neighbour_metadata & 3;
            if neighbour_facing == 1
                && !is_stair_same_half_same_facing(world, x - 1, y, z, metadata)
            {
                has_step = true;
            } else if neighbour_facing == 0
                && !is_stair_same_half_same_facing(world, x + 1, y, z, metadata)
            {
                min_x = 0.5;
                max_x = 1.0;
                has_step = true;
            }
        }
    } else if facing == 3 {
        let neighbour = world.block(x, y, z + 1);
        let neighbour_metadata = world.metadata(x, y, z + 1);
        if is_stair(neighbour) && metadata & 4 == neighbour_metadata & 4 {
            let neighbour_facing = neighbour_metadata & 3;
            if neighbour_facing == 1
                && !is_stair_same_half_same_facing(world, x - 1, y, z, metadata)
            {
                has_step = true;
            } else if neighbour_facing == 0
                && !is_stair_same_half_same_facing(world, x + 1, y, z, metadata)
            {
                min_x = 0.5;
                max_x = 1.0;
                has_step = true;
            }
        }
    }

    if has_step {
        Some(Bounds::new(min_x, min_y, min_z, max_x, max_y, max_z))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_id(name: &str) -> u16 {
        super::super::fixture::id(name)
    }

    #[test]
    fn base_shape_flips_with_metadata_bit_4() {
        let mut world = super::super::fixture::TestWorld::new();
        world.set(0, 0, 0, fixture_id("oak_stairs") as u8, 0);
        assert_eq!(base_shape(&world, 0, 0, 0).max_y, 0.5);
        world.set(0, 0, 0, fixture_id("oak_stairs") as u8, 4);
        assert_eq!(base_shape(&world, 0, 0, 0).min_y, 0.5);
        assert_eq!(base_shape(&world, 0, 0, 0).max_y, 1.0);
    }

    #[test]
    fn facing_zero_keeps_the_east_half() {
        let mut world = super::super::fixture::TestWorld::new();
        // A plain stair (facing 0) with no neighbours: the corner shape is
        // the east half.
        world.set(0, 0, 0, fixture_id("stone") as u8, 0);
        let (shape, corner) = inner_shape(&world, 0, 0, 0);
        assert_eq!(shape.min_x, 0.5);
        assert_eq!(shape.max_x, 1.0);
        assert_eq!(shape.min_z, 0.0);
        assert_eq!(shape.max_z, 1.0);
        assert!(corner);
    }

    #[test]
    fn two_stairs_facing_each_other_form_an_inner_corner() {
        let mut world = super::super::fixture::TestWorld::new();
        // Stair at (3,0,0) facing east (1), west neighbour facing south (2):
        // facing 1 reads the west neighbour, facing 2 shrinks z.
        world.set(3, 0, 0, fixture_id("oak_stairs") as u8, 1);
        world.set(2, 0, 0, fixture_id("oak_stairs") as u8, 2);
        let (shape, corner) = inner_shape(&world, 3, 0, 0);
        assert!(!corner);
        assert_eq!(shape.max_x, 0.5);
        assert_eq!(shape.min_x, 0.0);
        assert_eq!(shape.min_z, 0.5);
        assert_eq!(shape.max_z, 1.0);
    }

    #[test]
    fn step_shape_only_when_a_matching_stair_meets() {
        let mut world = super::super::fixture::TestWorld::new();
        world.set(0, 0, 0, fixture_id("oak_stairs") as u8, 1); // facing east
        world.set(1, 0, 0, fixture_id("oak_stairs") as u8, 2); // east neighbour facing south
        let step = step_shape(&world, 0, 0, 0);
        assert!(step.is_some());

        world.set(1, 0, 0, fixture_id("oak_stairs") as u8, 1); // same facing, no longer a corner
        assert!(step_shape(&world, 0, 0, 0).is_none());
    }
}
