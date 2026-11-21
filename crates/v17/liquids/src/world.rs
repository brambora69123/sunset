//! The world seam the liquid rules read and write through.
//!
//! Vanilla's liquid classes sit on the full `World`; this crate keeps only the
//! operations they use, so a server, a probe or a chunk generator can host the
//! rules without pulling the rest of the 1.7.10 world in.

use sunset_1_7_core::LegacyRandom;

use crate::block::Block;

/// `World.dimension`'s two flags the liquid rules read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LiquidDimension {
    /// `Dimension.noSky`: lava's `getTickRate` is 10 instead of 30.
    pub no_sky: bool,
    /// `Dimension.yeetsWater`: in the Nether lava decays by 1 per block
    /// instead of 2.
    pub yeets_water: bool,
}

impl LiquidDimension {
    /// `net.minecraft.world.dimension.OverworldDimension`.
    pub const OVERWORLD: Self = LiquidDimension {
        no_sky: false,
        yeets_water: false,
    };

    /// `net.minecraft.world.dimension.NetherDimension`.
    pub const NETHER: Self = LiquidDimension {
        no_sky: true,
        yeets_water: true,
    };

    /// `net.minecraft.world.dimension.TheEndDimension`: no sky, but water is
    /// not yeeted.
    pub const END: Self = LiquidDimension {
        no_sky: true,
        yeets_water: false,
    };
}

/// `World.setBlockWithMetadata` flag bit 1: `onBlockChanged` runs the six
/// neighbor updates (`World.setBlock`, `World.removeBlock`, and the flow's
/// `spreadTo`).
pub const NOTIFY_NEIGHBORS: i32 = 1;

/// `World.setBlockWithMetadata` flag bit 2: the block change reaches clients
/// and light. The liquid rules pass the flag through; the storage side decides
/// what it means.
pub const NOTIFY_CLIENTS: i32 = 2;

/// The 1.7.10 world operations the liquid classes use.
///
/// `set_block_with_metadata` and `set_block_metadata` are **storage only**:
/// they must not call back into this crate. The liquid module applies the
/// liquid-visible `onAdded` and flag-1 `neighborChanged` effects itself (see
/// [`crate::place_block`]), which is what lets a plain block grid host the
/// rules. `schedule_tick` must mirror `ServerWorld.scheduleTick`: dedupe by
/// position and block (an existing tick keeps its earlier time), order by
/// time, priority and scheduling order, and drop the tick when the area is not
/// loaded.
pub trait LiquidWorld {
    /// `World.getBlock`: air outside the world.
    fn get_block(&self, x: i32, y: i32, z: i32) -> Block;

    /// `World.getBlockMetadata`: 0 outside the world.
    fn get_block_metadata(&self, x: i32, y: i32, z: i32) -> i32;

    /// `World.setBlockWithMetadata` without the block callbacks. Returns
    /// false when the block and metadata are already there, or the position
    /// is outside the world.
    fn set_block_with_metadata(
        &mut self,
        x: i32,
        y: i32,
        z: i32,
        block: Block,
        metadata: i32,
        flags: i32,
    ) -> bool;

    /// `World.setBlockMetadata` without the block callbacks.
    fn set_block_metadata(&mut self, x: i32, y: i32, z: i32, metadata: i32, flags: i32) -> bool;

    /// `World.scheduleTick(x, y, z, block, delay)` with priority 0.
    fn schedule_tick(&mut self, x: i32, y: i32, z: i32, block: Block, delay: i32);

    /// `World.dimension`.
    fn dimension(&self) -> LiquidDimension;

    /// `World.random`. [`crate::tick`] is also given the stream explicitly;
    /// the two must be the same generator.
    fn random(&mut self) -> &mut LegacyRandom;

    /// `World.playSound` for the fizz. Defaults to no output.
    fn play_sound(
        &mut self,
        _x: f32,
        _y: f32,
        _z: f32,
        _name: &'static str,
        _volume: f32,
        _pitch: f32,
    ) {
    }

    /// The eight `largesmoke` particles of the fizz. Vanilla draws their
    /// offsets from `Math.random`, which is not the world's stream; the
    /// presentation layer owns that jitter.
    fn fizz_particles(&mut self, _x: i32, _y: i32, _z: i32) {}

    /// `Block.dropItems` of the block water spreads into, before it is
    /// replaced. The world owns the item entities and their random draws.
    fn drop_block(&mut self, _x: i32, _y: i32, _z: i32, _block: Block, _metadata: i32) {}

    /// `World.isBlockIndirectlyGettingPowered` and friends. The 1.7.10 liquid
    /// classes never read a redstone signal; this is part of the world
    /// contract for embedding worlds, and the flow rules do not call it.
    fn has_neighbor_signal(&self, _x: i32, _y: i32, _z: i32) -> bool {
        false
    }
}
