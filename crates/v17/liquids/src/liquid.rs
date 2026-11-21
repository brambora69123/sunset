//! `net.minecraft.block.LiquidBlock`: the pieces both liquid classes share.

use sunset_1_7_core::{LegacyRandom, RandomSource};

use crate::block::{
    AIR, Block, COBBLESTONE, FLOWING_LAVA_ID, FLOWING_WATER_ID, IRON_DOOR, LADDER, LAVA_ID,
    Material, OBSIDIAN, REEDS, STANDING_SIGN, WATER_ID, WOODEN_DOOR, flowing_of, source_of,
};
use crate::world::{LiquidDimension, LiquidWorld, NOTIFY_CLIENTS, NOTIFY_NEIGHBORS};

/// `LiquidBlock.getTickRate`: water 5; lava 30 above ground, 10 in a
/// sky-less dimension.
pub fn tick_rate(dimension: LiquidDimension, material: Material) -> i32 {
    match material {
        Material::Water => 5,
        Material::Lava if dimension.no_sky => 10,
        Material::Lava => 30,
        _ => 0,
    }
}

/// `LiquidBlock.getLiquidState`: the metadata when the block has the liquid's
/// material, else -1.
pub fn get_liquid_state(
    world: &dyn LiquidWorld,
    x: i32,
    y: i32,
    z: i32,
    material: Material,
) -> i32 {
    if world.get_block(x, y, z).material == material {
        world.get_block_metadata(x, y, z)
    } else {
        -1
    }
}

/// `LiquidBlock.getLiquidDepth`: -1 for another material, and the falling bit
/// reads as level 0.
pub fn get_liquid_depth(
    world: &dyn LiquidWorld,
    x: i32,
    y: i32,
    z: i32,
    material: Material,
) -> i32 {
    let mut depth = get_liquid_state(world, x, y, z, material);
    if depth < 0 {
        return -1;
    }
    if depth >= 8 {
        depth = 0;
    }
    depth
}

/// `FlowingLiquidBlock.isLiquidBlocking`: doors, signs, ladders, reeds and
/// portals block liquid even where their material does not block movement.
pub fn is_liquid_blocking(world: &dyn LiquidWorld, x: i32, y: i32, z: i32) -> bool {
    let block = world.get_block(x, y, z);
    matches!(
        block.id,
        WOODEN_DOOR | IRON_DOOR | STANDING_SIGN | LADDER | REEDS
    ) || block.material == Material::Portal
        || block.blocks_movement
}

/// `FlowingLiquidBlock.canSpreadTo`.
pub(crate) fn can_spread_to(
    world: &dyn LiquidWorld,
    x: i32,
    y: i32,
    z: i32,
    material: Material,
) -> bool {
    let target = world.get_block(x, y, z).material;
    target != material && target != Material::Lava && !is_liquid_blocking(world, x, y, z)
}

/// `World.setBlockWithMetadata` with the liquid-visible effects of the block
/// callbacks: the new block's `onAdded`, then, for flag bit 1, the six
/// `neighborChanged` calls in `World.updateNeighbors` order.
///
/// The storage side must be plain ([`LiquidWorld::set_block_with_metadata`]);
/// this function is the whole of the liquid world mutation. Use it for every
/// block a liquid places or replaces, including the probe's initial setup, so
/// the flags and callback order match vanilla.
pub fn place_block(
    world: &mut dyn LiquidWorld,
    x: i32,
    y: i32,
    z: i32,
    block: Block,
    metadata: i32,
    flags: i32,
    random: &mut LegacyRandom,
) -> bool {
    if !world.set_block_with_metadata(x, y, z, block, metadata, flags) {
        return false;
    }
    block_on_added(world, x, y, z, block, random);
    if flags & NOTIFY_NEIGHBORS != 0 {
        update_neighbors(world, x, y, z, random);
    }
    true
}

/// `Block.onAdded` for the block just placed, dispatched by its id.
pub(crate) fn block_on_added(
    world: &mut dyn LiquidWorld,
    x: i32,
    y: i32,
    z: i32,
    block: Block,
    random: &mut LegacyRandom,
) {
    match block.id {
        FLOWING_WATER_ID | FLOWING_LAVA_ID => {
            check_block_collisions(world, x, y, z, block, random);
            if world.get_block(x, y, z) == block {
                let delay = tick_rate(world.dimension(), block.material);
                world.schedule_tick(x, y, z, block, delay);
            }
        }
        WATER_ID | LAVA_ID => check_block_collisions(world, x, y, z, block, random),
        _ => {}
    }
}

/// `Block.neighborChanged` at a position, dispatched by the block there.
pub(crate) fn block_neighbor_changed(
    world: &mut dyn LiquidWorld,
    x: i32,
    y: i32,
    z: i32,
    random: &mut LegacyRandom,
) {
    let block = world.get_block(x, y, z);
    match block.id {
        WATER_ID | LAVA_ID => {
            check_block_collisions(world, x, y, z, block, random);
            if world.get_block(x, y, z) == block {
                let metadata = world.get_block_metadata(x, y, z);
                let flowing = flowing_of(block);
                let delay = tick_rate(world.dimension(), block.material);
                place_block(world, x, y, z, flowing, metadata, NOTIFY_CLIENTS, random);
                world.schedule_tick(x, y, z, flowing, delay);
            }
        }
        FLOWING_WATER_ID | FLOWING_LAVA_ID => check_block_collisions(world, x, y, z, block, random),
        _ => {}
    }
}

/// `World.updateNeighbors`: the six neighbors in vanilla's order.
pub(crate) fn update_neighbors(
    world: &mut dyn LiquidWorld,
    x: i32,
    y: i32,
    z: i32,
    random: &mut LegacyRandom,
) {
    block_neighbor_changed(world, x - 1, y, z, random);
    block_neighbor_changed(world, x + 1, y, z, random);
    block_neighbor_changed(world, x, y - 1, z, random);
    block_neighbor_changed(world, x, y + 1, z, random);
    block_neighbor_changed(world, x, y, z - 1, random);
    block_neighbor_changed(world, x, y, z + 1, random);
}

/// `LiquidBlock.checkBlockCollisions`: lava next to water becomes obsidian at
/// level 0, cobblestone at levels 1..4, and merely fizzes above that.
pub(crate) fn check_block_collisions(
    world: &mut dyn LiquidWorld,
    x: i32,
    y: i32,
    z: i32,
    this: Block,
    random: &mut LegacyRandom,
) {
    if world.get_block(x, y, z) != this || this.material != Material::Lava {
        return;
    }

    // The short-circuit order is vanilla's, though only the answer matters.
    let water = world.get_block(x, y, z - 1).material == Material::Water
        || world.get_block(x, y, z + 1).material == Material::Water
        || world.get_block(x - 1, y, z).material == Material::Water
        || world.get_block(x + 1, y, z).material == Material::Water
        || world.get_block(x, y + 1, z).material == Material::Water;
    if !water {
        return;
    }

    let metadata = world.get_block_metadata(x, y, z);
    if metadata == 0 {
        place_block(world, x, y, z, OBSIDIAN, 0, NOTIFY_NEIGHBORS, random);
    } else if metadata <= 4 {
        place_block(world, x, y, z, COBBLESTONE, 0, NOTIFY_NEIGHBORS, random);
    }
    fizz(world, x, y, z, random);
}

/// `LiquidBlock.fizz`: two `nextFloat` draws for the pitch, the sound, then
/// eight smoke particles.
pub(crate) fn fizz(world: &mut dyn LiquidWorld, x: i32, y: i32, z: i32, random: &mut LegacyRandom) {
    let pitch = 2.6f32 + (random.next_float() - random.next_float()) * 0.8f32;
    world.play_sound(
        x as f32 + 0.5,
        y as f32 + 0.5,
        z as f32 + 0.5,
        "random.fizz",
        0.5,
        pitch,
    );
    for _ in 0..8 {
        world.fizz_particles(x, y, z);
    }
}

/// `FlowingLiquidBlock.convertToSource`: the still block of the same liquid,
/// keeping the metadata, with flags 2.
pub(crate) fn convert_to_source(
    world: &mut dyn LiquidWorld,
    x: i32,
    y: i32,
    z: i32,
    this: Block,
    random: &mut LegacyRandom,
) {
    let metadata = world.get_block_metadata(x, y, z);
    place_block(
        world,
        x,
        y,
        z,
        source_of(this),
        metadata,
        NOTIFY_CLIENTS,
        random,
    );
}

/// `World.setBlock(x, y, z, block)`: flags 3.
pub(crate) fn set_block(
    world: &mut dyn LiquidWorld,
    x: i32,
    y: i32,
    z: i32,
    block: Block,
    random: &mut LegacyRandom,
) -> bool {
    place_block(
        world,
        x,
        y,
        z,
        block,
        0,
        NOTIFY_NEIGHBORS | NOTIFY_CLIENTS,
        random,
    )
}

/// `World.removeBlock`: air with flags 3.
pub(crate) fn remove_block(
    world: &mut dyn LiquidWorld,
    x: i32,
    y: i32,
    z: i32,
    random: &mut LegacyRandom,
) -> bool {
    set_block(world, x, y, z, AIR, random)
}
