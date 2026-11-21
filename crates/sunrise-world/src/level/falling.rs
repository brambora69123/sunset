//! 1.7.10 gravity blocks: `FallingBlock.onAdded`/`neighborChanged` (a
//! scheduled tick two ticks on), `FallingBlock.tick` and its `tryFall`, and
//! the landing branch of `FallingBlockEntity.tick`. The falling entity itself
//! (motion, collision, entity damage) is simulated by the caller.
use super::{
    Behaviours, LevelAccess, LevelEvent, Sim, Ticks, classes::Class, flags, update::below,
};
use crate::registry::BlockStateId;
use sunrise_core::position::BlockPos;

/// `FallingBlock.canFallThrough`: air, fire, water and lava.
fn can_fall_through(b: &Behaviours, state: BlockStateId) -> bool {
    b.is_air(state) || b.data(state).is(Class::Fire) || b.liquid_of(state).is_some()
}

/// Outcome of [`Behaviours::falling_block_landed`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Landing {
    /// The block was placed at the position; discard the entity.
    Placed(BlockStateId),
    /// The block could not be placed: discard the entity and drop its item.
    Broken,
    /// The landing position holds a moving piston: keep falling.
    Continue,
}

impl<L: LevelAccess + ?Sized> Sim<'_, L> {
    /// `FallingBlock.tryFall`'s entity branch: the block is removed and the
    /// caller spawns a falling-block entity carrying `state`.
    pub(crate) fn falling_block_spawn(
        &mut self,
        pos: BlockPos,
        state: BlockStateId,
        hurt_per_distance: Option<f32>,
    ) {
        self.set_block_and_update(pos, self.b.air());
        self.emit(LevelEvent::FallingBlock {
            pos,
            state,
            hurt_per_distance,
        });
    }
    /// `FallingBlock.tryFall`'s immediate branch: the block moves down inside
    /// this call, without an entity. The landing write uses metadata 0, as
    /// `World.setBlock` does in 1.7.10.
    fn falling_block_move(&mut self, pos: BlockPos, state: BlockStateId) {
        self.set_block(pos, self.b.air(), flags::UPDATE_ALL);
        let mut y = pos.y;
        while y > 0 && can_fall_through(self.b, self.state(BlockPos::new(pos.x, y - 1, pos.z))) {
            y -= 1;
        }
        if y > 0 {
            let placed = self.b.default_of(self.b.block(state));
            self.set_block(BlockPos::new(pos.x, y, pos.z), placed, flags::UPDATE_ALL);
        }
    }
    /// `FallingBlock.tick`.
    pub(crate) fn falling_block_tick(&mut self, pos: BlockPos, state: BlockStateId) {
        if !can_fall_through(self.b, self.below_state(pos)) || pos.y < 0 {
            return;
        }
        if self.ticks.fall_immediately || !self.area_loaded(pos, 32) {
            self.falling_block_move(pos, state);
        } else {
            self.falling_block_spawn(pos, state, None);
        }
    }
    /// `SpeleothemBlock.spawnFallingStalactite` (a 26.3 block, kept for its
    /// caller): each stalactite piece becomes a falling entity from the top
    /// down, the tip carrying the damage.
    pub(crate) fn spawn_falling_stalactite(&mut self, pos: BlockPos, state: BlockStateId) {
        let b = self.b;
        let p = b.props;
        let is_stalactite = |s: BlockStateId| {
            b.tag("minecraft:speleothems")
                .is_some_and(|tag| b.in_set(s, tag))
                && b.table.dir(s, p.vertical_direction)
                    == Some(sunrise_core::geometry::Direction::Down)
        };
        let mut fall_pos = pos;
        let mut fall_state = state;
        while is_stalactite(fall_state) {
            let thickness = b.table.text(fall_state, p.thickness);
            let tip = thickness == "tip" || thickness == "tip_merge";
            let damage = tip.then_some((1 + pos.y - fall_pos.y).max(6) as f32);
            self.falling_block_spawn(fall_pos, fall_state, damage);
            if tip {
                break;
            }
            fall_pos = below(fall_pos);
            fall_state = self.state(fall_pos);
        }
    }
}

impl Behaviours {
    /// The landing branch of `FallingBlockEntity.tick`, called by the caller's
    /// falling-block entity once it is on the ground. `state` is the carried
    /// block state (its metadata is the entity's).
    pub fn falling_block_landed<L: LevelAccess + ?Sized>(
        &self,
        level: &mut L,
        ticks: &mut Ticks,
        pos: BlockPos,
        state: BlockStateId,
        _stuck_in_water: bool,
    ) -> Landing {
        let mut sim = self.sim(level, ticks);
        let current = sim.state(pos);
        if self.is_block(current, self.named.moving_piston) {
            return Landing::Continue;
        }
        // `World.canPlace(state, pos, true, 1, null, null)`: the target's
        // material is replaceable (the falling entity only lands in a cell it
        // could pass through), and `!canFallThrough(below)`.
        if !self.is_replaceable(current) || can_fall_through(self, sim.below_state(pos)) {
            return Landing::Broken;
        }
        if !sim.set_block_and_update(pos, state) {
            return Landing::Broken;
        }
        Landing::Placed(sim.state(pos))
    }
}
