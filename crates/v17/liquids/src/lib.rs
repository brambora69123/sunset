//! Minecraft 1.7.10 water and lava spreading.
//!
//! Ported from the 1.7.10 decompiled sources: `net.minecraft.block.LiquidBlock`
//! (tick delay, `onAdded`, `neighborChanged`, the water-meets-lava fizz),
//! `net.minecraft.block.FlowingLiquidBlock` (the level ladder, the downward
//! fall, the nearest-drop search and the two-source reconstruction) and
//! `net.minecraft.block.LiquidSourceBlock` (the still block: lava's fire
//! spreading and the conversion to flowing on a neighbor update).
//!
//! The flow rules are standalone: [`LiquidWorld`] is the small world contract
//! they read and write through, and the crate depends only on the 1.7.10 core
//! primitives. The world stores blocks; the liquid module runs every
//! liquid-visible effect of `World.setBlockWithMetadata` itself, so a plain
//! block grid is enough ([`fixture::TestWorld`] is one).
//!
//! Entry points:
//! - [`tick`] — one block tick (`Block.tick`), scheduled or direct.
//! - [`on_added`] — `Block.onAdded` for a block the world has just placed.
//! - [`neighbor_changed`] — `Block.neighborChanged`.
//! - [`place_block`] — `World.setBlockWithMetadata` plus the liquid reactions
//!   of `onAdded` and the flag-1 neighbor updates.
//!
//! A chunk generator that places a flowing spring and settles it immediately
//! calls [`tick`] with the world's random, like `LiquidFallFeature` does after
//! setting `World.doTicksImmediately`; the world's `schedule_tick` decides
//! whether the follow-up ticks run immediately or from its queue.

pub mod block;
pub mod fixture;
mod flowing;
mod liquid;
mod source;
pub mod world;

pub use block::{
    AIR, Block, COBBLESTONE, FIRE, FLOWING_LAVA, FLOWING_WATER, LAVA, Material, OBSIDIAN, STONE,
    WATER, flowing_of, height_loss, liquid_depth, source_of,
};
pub use liquid::{get_liquid_depth, get_liquid_state, is_liquid_blocking, place_block, tick_rate};
pub use world::{LiquidDimension, LiquidWorld, NOTIFY_CLIENTS, NOTIFY_NEIGHBORS};

use sunset_1_7_core::LegacyRandom;

/// `Block.tick` for a 1.7.10 liquid block.
///
/// Dispatches by the block currently in the world, like the block instance
/// vanilla's `ServerWorld` looks up before calling `tick`: flowing water and
/// lava run the flow rules, the still blocks run the source rules (lava's fire
/// spreading; water does nothing).
pub fn tick(world: &mut dyn LiquidWorld, x: i32, y: i32, z: i32, random: &mut LegacyRandom) {
    let block = world.get_block(x, y, z);
    match block.id {
        block::FLOWING_WATER_ID | block::FLOWING_LAVA_ID => {
            flowing::tick(world, x, y, z, block, random)
        }
        block::WATER_ID | block::LAVA_ID => source::tick(world, x, y, z, block, random),
        _ => {}
    }
}

/// `Block.onAdded` for a 1.7.10 liquid block.
///
/// Runs with the world's random, as vanilla's `LiquidBlock.onAdded` reaches
/// `World.random` through `fizz`. A world that already runs `onAdded` for
/// ordinary blocks must not call this as well: [`place_block`] does it.
pub fn on_added(world: &mut dyn LiquidWorld, x: i32, y: i32, z: i32) {
    let mut random = world.random().clone();
    let block = world.get_block(x, y, z);
    liquid::block_on_added(world, x, y, z, block, &mut random);
    *world.random() = random;
}

/// `Block.neighborChanged` for a 1.7.10 liquid block.
///
/// A source converts to flowing and schedules the follow-up tick; a flowing
/// block only runs the water-meets-lava check. As with [`on_added`], a world
/// that dispatches ordinary neighbor updates must not call this twice.
pub fn neighbor_changed(world: &mut dyn LiquidWorld, x: i32, y: i32, z: i32) {
    let mut random = world.random().clone();
    liquid::block_neighbor_changed(world, x, y, z, &mut random);
    *world.random() = random;
}

/// Whether the block is one of the four 1.7.10 liquid blocks.
pub fn is_liquid(block: Block) -> bool {
    matches!(
        block.id,
        block::FLOWING_WATER_ID | block::WATER_ID | block::FLOWING_LAVA_ID | block::LAVA_ID
    )
}
