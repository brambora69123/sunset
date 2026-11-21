//! `net.minecraft.block.FlowingLiquidBlock`: the liquid that moves.

use sunset_1_7_core::{LegacyRandom, RandomSource};

use crate::block::{Block, Material, STONE};
use crate::liquid::{
    can_spread_to, convert_to_source, fizz, get_liquid_state, is_liquid_blocking, place_block,
    remove_block, set_block, tick_rate, update_neighbors,
};
use crate::world::{LiquidWorld, NOTIFY_CLIENTS, NOTIFY_NEIGHBORS};

const DIRECTIONS: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

/// `FlowingLiquidBlock.tick`.
pub(crate) fn tick(
    world: &mut dyn LiquidWorld,
    x: i32,
    y: i32,
    z: i32,
    this: Block,
    random: &mut LegacyRandom,
) {
    let mut state = get_liquid_state(world, x, y, z, this.material);
    let decay = if this.material == Material::Lava && !world.dimension().yeets_water {
        2
    } else {
        1
    };
    let mut delay = tick_rate(world.dimension(), this.material);

    if state > 0 {
        let mut lowest = -100;
        let mut adjacent_sources = 0;
        lowest = get_lowest_depth(
            world,
            x - 1,
            y,
            z,
            lowest,
            &mut adjacent_sources,
            this.material,
        );
        lowest = get_lowest_depth(
            world,
            x + 1,
            y,
            z,
            lowest,
            &mut adjacent_sources,
            this.material,
        );
        lowest = get_lowest_depth(
            world,
            x,
            y,
            z - 1,
            lowest,
            &mut adjacent_sources,
            this.material,
        );
        lowest = get_lowest_depth(
            world,
            x,
            y,
            z + 1,
            lowest,
            &mut adjacent_sources,
            this.material,
        );

        let mut candidate = lowest + decay;
        if candidate >= 8 || lowest < 0 {
            candidate = -1;
        }

        let above = get_liquid_state(world, x, y + 1, z, this.material);
        if above >= 0 {
            candidate = if above >= 8 { above } else { above + 8 };
        }

        if adjacent_sources >= 2 && this.material == Material::Water {
            let below = world.get_block(x, y - 1, z);
            if below.solid {
                candidate = 0;
            } else if below.material == this.material && world.get_block_metadata(x, y - 1, z) == 0
            {
                candidate = 0;
            }
        }

        if this.material == Material::Lava
            && state < 8
            && candidate < 8
            && candidate > state
            && random.next_int_bound(4).unwrap_or(0) != 0
        {
            delay *= 4;
        }

        if candidate == state {
            // The decompiled `var8` is a constant true in 1.7.10.
            convert_to_source(world, x, y, z, this, random);
        } else {
            state = candidate;
            if state < 0 {
                remove_block(world, x, y, z, random);
            } else {
                world.set_block_metadata(x, y, z, state, NOTIFY_CLIENTS);
                world.schedule_tick(x, y, z, this, delay);
                update_neighbors(world, x, y, z, random);
            }
        }
    } else {
        convert_to_source(world, x, y, z, this, random);
    }

    if can_spread_to(world, x, y - 1, z, this.material) {
        if this.material == Material::Lava
            && world.get_block(x, y - 1, z).material == Material::Water
        {
            set_block(world, x, y - 1, z, STONE, random);
            fizz(world, x, y - 1, z, random);
            return;
        }

        if state >= 8 {
            spread_to(world, x, y - 1, z, state, this, random);
        } else {
            spread_to(world, x, y - 1, z, state + 8, this, random);
        }
    } else if state >= 0 && (state == 0 || is_liquid_blocking(world, x, y - 1, z)) {
        let spread = get_spread(world, x, y, z, this.material);
        let mut depth = state + decay;
        if state >= 8 {
            depth = 1;
        }
        if depth >= 8 {
            return;
        }

        if spread[0] {
            spread_to(world, x - 1, y, z, depth, this, random);
        }
        if spread[1] {
            spread_to(world, x + 1, y, z, depth, this, random);
        }
        if spread[2] {
            spread_to(world, x, y, z - 1, depth, this, random);
        }
        if spread[3] {
            spread_to(world, x, y, z + 1, depth, this, random);
        }
    }
}

/// `FlowingLiquidBlock.spreadTo`: fizz (lava) or the replaced block's drops
/// (water), then the liquid with flags 3.
fn spread_to(
    world: &mut dyn LiquidWorld,
    x: i32,
    y: i32,
    z: i32,
    depth: i32,
    this: Block,
    random: &mut LegacyRandom,
) {
    if !can_spread_to(world, x, y, z, this.material) {
        return;
    }
    let target = world.get_block(x, y, z);
    if this.material == Material::Lava {
        fizz(world, x, y, z, random);
    } else {
        let metadata = world.get_block_metadata(x, y, z);
        world.drop_block(x, y, z, target, metadata);
    }
    place_block(
        world,
        x,
        y,
        z,
        this,
        depth,
        NOTIFY_NEIGHBORS | NOTIFY_CLIENTS,
        random,
    );
}

/// `FlowingLiquidBlock.getLowestDepth`; `adjacent_sources` is the class field
/// `adjacentSources`.
fn get_lowest_depth(
    world: &dyn LiquidWorld,
    x: i32,
    y: i32,
    z: i32,
    depth: i32,
    adjacent_sources: &mut i32,
    material: Material,
) -> i32 {
    let mut state = get_liquid_state(world, x, y, z, material);
    if state < 0 {
        return depth;
    }
    if state == 0 {
        *adjacent_sources += 1;
    }
    if state >= 8 {
        state = 0;
    }
    if depth >= 0 && state >= depth {
        depth
    } else {
        state
    }
}

/// `FlowingLiquidBlock.getSpread`: the four directions whose nearest drop is
/// the closest, using the `distanceToGap` class field as a local array.
fn get_spread(world: &dyn LiquidWorld, x: i32, y: i32, z: i32, material: Material) -> [bool; 4] {
    let mut distance = [1000; 4];
    for dir in 0..4 {
        let (dx, dz) = DIRECTIONS[dir];
        let nx = x + dx;
        let nz = z + dz;
        if !is_liquid_blocking(world, nx, y, nz)
            && (world.get_block(nx, y, nz).material != material
                || world.get_block_metadata(nx, y, nz) != 0)
        {
            distance[dir] = if is_liquid_blocking(world, nx, y - 1, nz) {
                get_distance_to_gap(world, nx, y, nz, 1, dir, material)
            } else {
                0
            };
        }
    }

    let mut closest = distance[0];
    for entry in &distance[1..] {
        if *entry < closest {
            closest = *entry;
        }
    }

    let mut spread = [false; 4];
    for dir in 0..4 {
        spread[dir] = distance[dir] == closest;
    }
    spread
}

/// `FlowingLiquidBlock.getDistanceToGap`: walk at most four blocks looking for
/// a neighbour whose floor is open.
fn get_distance_to_gap(
    world: &dyn LiquidWorld,
    x: i32,
    y: i32,
    z: i32,
    distance: i32,
    from_dir: usize,
    material: Material,
) -> i32 {
    let mut closest = 1000;
    for dir in 0..4 {
        if (dir == 0 && from_dir == 1)
            || (dir == 1 && from_dir == 0)
            || (dir == 2 && from_dir == 3)
            || (dir == 3 && from_dir == 2)
        {
            continue;
        }
        let (dx, dz) = DIRECTIONS[dir];
        let nx = x + dx;
        let nz = z + dz;
        if !is_liquid_blocking(world, nx, y, nz)
            && (world.get_block(nx, y, nz).material != material
                || world.get_block_metadata(nx, y, nz) != 0)
        {
            if !is_liquid_blocking(world, nx, y - 1, nz) {
                return distance;
            }
            if distance < 4 {
                let candidate = get_distance_to_gap(world, nx, y, nz, distance + 1, dir, material);
                if candidate < closest {
                    closest = candidate;
                }
            }
        }
    }
    closest
}
