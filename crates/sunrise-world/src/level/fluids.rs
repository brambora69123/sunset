//! 1.7.10 liquids: `LiquidBlock` (the pieces both liquid classes share),
//! `FlowingLiquidBlock` (the level ladder, the fall, the nearest-drop search
//! and the two-source reconstruction) and `LiquidSourceBlock` (the still
//! block: lava's fire and the conversion to flowing on a neighbour update),
//! plus liquid containers (a bucket emptying into a cauldron) and bucket
//! pickup.
//!
//! The level of a liquid is its block metadata: `0` is a source, `1..=7` a
//! flowing distance and `>= 8` the falling bit. Source and flowing are
//! separate block ids (`water`/`flowing_water`, `lava`/`flowing_lava`).
use super::{
    Behaviours, LevelAccess, Sim,
    buckets::{Douse, Drain, PlacedLiquid},
    classes::Class,
    data::state_flags,
    update::{above, below, rel},
};
use crate::{
    fluid::{FluidKind, FluidState, ModFluidRules},
    registry::BlockStateId,
};
use sunrise_core::{position::BlockPos, random::RandomSource};

/// `FlowingLiquidBlock`'s four horizontal directions, in its own order: `0`
/// is west (`x - 1`), `1` east, `2` north (`z - 1`), `3` south.
const LIQUID_DIRECTIONS: [sunrise_core::geometry::Direction; 4] = [
    sunrise_core::geometry::Direction::West,
    sunrise_core::geometry::Direction::East,
    sunrise_core::geometry::Direction::North,
    sunrise_core::geometry::Direction::South,
];

/// `LiquidBlock`'s material: water and lava are the only liquids 1.7.10 has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Liquid {
    Water,
    Lava,
}
impl Liquid {
    fn name(self) -> &'static str {
        match self {
            Self::Water => "water",
            Self::Lava => "lava",
        }
    }
    fn is_water(self) -> bool {
        self == Self::Water
    }
    fn is_lava(self) -> bool {
        self == Self::Lava
    }
    /// The still block: `Blocks.WATER` / `Blocks.LAVA`.
    fn source_block(self, b: &Behaviours) -> super::states::BlockId {
        match self {
            Self::Water => b.named.water,
            Self::Lava => b.named.lava,
        }
    }
    /// The moving block: `Blocks.FLOWING_WATER` / `Blocks.FLOWING_LAVA`.
    fn flowing_block(self, b: &Behaviours) -> super::states::BlockId {
        match self {
            Self::Water => b.named.flowing_water,
            Self::Lava => b.named.flowing_lava,
        }
    }
}

fn is_water(kind: FluidKind) -> bool {
    matches!(kind, FluidKind::Water | FluidKind::FlowingWater)
}
pub(super) fn source_of(kind: FluidKind) -> FluidKind {
    match kind {
        FluidKind::Mod(fluid) | FluidKind::ModFlowing(fluid) => FluidKind::Mod(fluid),
        _ if is_water(kind) => FluidKind::Water,
        _ => FluidKind::Lava,
    }
}

/// `Fluid.getTickDelay(level)`: water 5, lava 30 (10 where lava flows fast,
/// the Nether), the empty fluid 0. A mod's fluid keeps its registered delay.
pub fn fluid_tick_delay(
    kind: FluidKind,
    fast_lava: bool,
    mod_rules: impl FnOnce() -> Option<ModFluidRules>,
) -> i32 {
    match kind {
        FluidKind::Empty => 0,
        FluidKind::Water | FluidKind::FlowingWater => 5,
        FluidKind::Mod(fluid) | FluidKind::ModFlowing(fluid) => {
            mod_rules().unwrap_or(fluid.get().rules).tick_delay
        }
        FluidKind::Lava | FluidKind::FlowingLava => {
            if fast_lava {
                10
            } else {
                30
            }
        }
    }
}

/// `FluidState.canBeReplacedWith(level, pos, other, direction)` for the mod
/// API's read. 1.7.10's own flow decides replacement inside
/// `FlowingLiquidBlock.tick` (`canSpreadTo` plus the lava-water branch) and
/// never asks this; the read stays for a mod that queries it.
/// `height` is `fluid`'s height at the position, asked only for lava.
pub fn fluid_can_be_replaced_with(
    fluid: FluidState,
    other: FluidKind,
    direction: sunrise_core::geometry::Direction,
    height: impl FnOnce() -> f32,
) -> bool {
    match fluid.kind() {
        FluidKind::Empty => true,
        FluidKind::Water | FluidKind::FlowingWater => {
            direction == sunrise_core::geometry::Direction::Down && !is_water(other)
        }
        FluidKind::Mod(_) | FluidKind::ModFlowing(_) => false,
        FluidKind::Lava | FluidKind::FlowingLava => height() >= 0.444_444_45 && is_water(other),
    }
}

/// `Fluid.getExplosionResistance` of `kind`: 100 for water, lava and a mod's
/// fluid, 0 for the empty fluid. Ray-cast explosions read the same 100 for
/// any block that holds a fluid.
pub fn fluid_explosion_resistance(kind: FluidKind) -> f32 {
    match kind {
        FluidKind::Empty => 0.0,
        _ => 100.0,
    }
}

/// `Fluid.isRandomlyTicking`: only lava's state ticks randomly. In 1.7.10 a
/// flowing water block also carries `setTicksRandomly(true)`, but its random
/// tick is the no-op `Block.randomTick`; the exported block flag says so.
pub fn fluid_randomly_ticks(kind: FluidKind) -> bool {
    matches!(kind, FluidKind::Lava | FluidKind::FlowingLava)
}

/// `Fluid.getDripParticle`: the particle a block of `kind` drips beneath it.
pub fn fluid_drip_particle(kind: FluidKind) -> Option<&'static str> {
    match kind {
        FluidKind::Water | FluidKind::FlowingWater => Some("minecraft:dripping_water"),
        FluidKind::Lava | FluidKind::FlowingLava => Some("minecraft:dripping_lava"),
        FluidKind::Empty | FluidKind::Mod(_) | FluidKind::ModFlowing(_) => None,
    }
}

impl Behaviours {
    /// The 1.7.10 liquid material of a state, if it is one of the four liquid
    /// blocks.
    pub(crate) fn liquid_of(&self, state: BlockStateId) -> Option<Liquid> {
        let block = self.table.block(state);
        if block == self.named.water || block == self.named.flowing_water {
            Some(Liquid::Water)
        } else if block == self.named.lava || block == self.named.flowing_lava {
            Some(Liquid::Lava)
        } else {
            None
        }
    }
    /// Whether `state` is `Blocks.FLOWING_WATER` or `Blocks.FLOWING_LAVA`.
    pub(crate) fn is_liquid_flowing(&self, state: BlockStateId) -> bool {
        let block = self.table.block(state);
        block == self.named.flowing_water || block == self.named.flowing_lava
    }
    /// `state.getBlock() instanceof LiquidBlockContainer`: a block that keeps
    /// a fluid inside its own state (`SimpleWaterloggedBlock` extends the
    /// interface, so stairs, slabs and every other waterloggable block count).
    pub fn is_liquid_container(&self, state: BlockStateId) -> bool {
        self.implements(state, super::classes::Interface::LiquidBlockContainer)
            || self.implements(state, super::classes::Interface::SimpleWaterlogged)
    }
    /// `LiquidBlockContainer.canPlaceLiquid(user, level, pos, state, fluid)`.
    pub fn can_place_liquid(&self, state: BlockStateId, fluid: FluidKind, creative: bool) -> bool {
        let data = self.data(state);
        if data.is(Class::Kelp)
            || data.is(Class::KelpPlant)
            || data.is(Class::Seagrass)
            || data.is(Class::TallSeagrass)
        {
            return false;
        }
        if data.is(Class::Barrier) && !creative {
            return false;
        }
        if data.is(Class::Slab) && self.table.text(state, self.props.kind) == "double" {
            return false;
        }
        fluid == FluidKind::Water
    }
    /// `Block.getMaterial().isSolid()`: false for air, fire, liquids and the
    /// portal material, true for everything else 1.7.10 has.
    fn liquid_solid_material(&self, state: BlockStateId) -> bool {
        !self.is_air(state)
            && self.liquid_of(state).is_none()
            && !self.data(state).is(Class::Fire)
            && self.table.name(state) != "minecraft:portal"
    }
    /// `Block.getMaterial().blocksMovement()`: air, fire, liquids, plants and
    /// decorated blocks let a liquid through. The exported collision flag
    /// holds the first four and the flat decorations; a replaceable state (a
    /// snow layer) is a plant material too.
    fn liquid_material_blocks_movement(&self, state: BlockStateId) -> bool {
        !self.has_flag(state, state_flags::COLLISION_EMPTY)
            && !self.is_replaceable(state)
            && !self.data(state).is(Class::Carpet)
            && !self.data(state).is(Class::SnowLayer)
    }
    /// `FlowingLiquidBlock.isLiquidBlocking`: doors, signs, ladders, reeds and
    /// portals block a liquid even where their material lets movement through.
    fn liquid_blocking(&self, state: BlockStateId) -> bool {
        matches!(
            self.table.name(state),
            "minecraft:wooden_door"
                | "minecraft:iron_door"
                | "minecraft:standing_sign"
                | "minecraft:ladder"
                | "minecraft:reeds"
                | "minecraft:portal"
        ) || self.liquid_material_blocks_movement(state)
    }
}

impl<L: LevelAccess + ?Sized> Sim<'_, L> {
    fn liquid_meta(&self, state: BlockStateId) -> i32 {
        self.b.table.int(state, self.b.props.meta)
    }
    fn liquid_state(&self, block: super::states::BlockId, meta: i32) -> BlockStateId {
        self.b
            .table
            .set_int(self.b.default_of(block), self.b.props.meta, meta)
    }
    /// `LiquidBlock.getLiquidState`: the metadata when the block has the
    /// liquid's material, else -1.
    fn liquid_state_at(&self, pos: BlockPos, liquid: Liquid) -> i32 {
        let state = self.state(pos);
        if self.b.liquid_of(state) == Some(liquid) {
            self.liquid_meta(state)
        } else {
            -1
        }
    }
    /// `FlowingLiquidBlock.getLowestDepth`, one neighbour.
    fn liquid_lowest_depth(
        &self,
        pos: BlockPos,
        depth: i32,
        liquid: Liquid,
        adjacent_sources: &mut i32,
    ) -> i32 {
        let mut value = self.liquid_state_at(pos, liquid);
        if value < 0 {
            return depth;
        }
        if value == 0 {
            *adjacent_sources += 1;
        }
        if value >= 8 {
            value = 0;
        }
        if depth >= 0 && value >= depth {
            depth
        } else {
            value
        }
    }
    /// `LiquidBlock.getTickRate`.
    fn liquid_tick_rate(&self, liquid: Liquid) -> i32 {
        match liquid {
            Liquid::Water => 5,
            Liquid::Lava => {
                if self.level.dimension_type().fast_lava {
                    10
                } else {
                    30
                }
            }
        }
    }
    /// `FlowingLiquidBlock.canSpreadTo`.
    fn liquid_can_spread_to(&self, state: BlockStateId, liquid: Liquid) -> bool {
        let other = self.b.liquid_of(state);
        other != Some(liquid) && other != Some(Liquid::Lava) && !self.b.liquid_blocking(state)
    }
    /// `FlowingLiquidBlock.getSpread`: which of the four sides get the flow,
    /// those whose nearest drop is the closest.
    fn liquid_spread_map(&self, pos: BlockPos, liquid: Liquid) -> [bool; 4] {
        let mut distance = [1000; 4];
        for (index, direction) in LIQUID_DIRECTIONS.into_iter().enumerate() {
            let test = rel(pos, direction);
            let state = self.state(test);
            if !self.b.liquid_blocking(state)
                && !(self.b.liquid_of(state) == Some(liquid) && self.liquid_meta(state) == 0)
            {
                if self.b.liquid_blocking(self.state(below(test))) {
                    distance[index] = self.liquid_distance_to_gap(test, 1, index, liquid);
                } else {
                    distance[index] = 0;
                }
            }
        }
        let lowest = distance.into_iter().min().unwrap_or(1000);
        [
            distance[0] == lowest,
            distance[1] == lowest,
            distance[2] == lowest,
            distance[3] == lowest,
        ]
    }
    /// `FlowingLiquidBlock.getDistanceToGap`: the nearest cell with nothing
    /// solid below it, walking `distance` steps into the four sides without
    /// turning back.
    fn liquid_distance_to_gap(
        &self,
        pos: BlockPos,
        distance: i32,
        from: usize,
        liquid: Liquid,
    ) -> i32 {
        let mut lowest = 1000;
        for (index, direction) in LIQUID_DIRECTIONS.into_iter().enumerate() {
            // The opposite of `from` would turn back.
            if index == [1, 0, 3, 2][from] {
                continue;
            }
            let test = rel(pos, direction);
            let state = self.state(test);
            if !self.b.liquid_blocking(state)
                && !(self.b.liquid_of(state) == Some(liquid) && self.liquid_meta(state) == 0)
            {
                if !self.b.liquid_blocking(self.state(below(test))) {
                    return distance;
                }
                if distance < 4 {
                    let found = self.liquid_distance_to_gap(test, distance + 1, index, liquid);
                    lowest = lowest.min(found);
                }
            }
        }
        lowest
    }
    /// `LiquidBlock.fizz`: the hiss and its smoke. The pitch draws the level
    /// random twice, in vanilla's order.
    fn liquid_fizz(&mut self, pos: BlockPos) {
        let first = self.random().next_float();
        let second = self.random().next_float();
        let pitch = 2.6 + (first - second) * 0.8;
        self.sound(pos, &"minecraft:random.fizz".into(), 0.5, pitch);
        // 1.7.10 adds eight `largesmoke` particles with `Math.random`; the
        // level event draws the same puff on the client.
        self.level_event(1501, pos, 0);
    }
    /// `LiquidBlock.checkBlockCollisions`: lava beside or above water turns to
    /// obsidian (a source) or cobblestone (a flow of level 1..4) and hisses.
    fn liquid_check_collisions(&mut self, pos: BlockPos, state: BlockStateId) {
        if self.b.liquid_of(state) != Some(Liquid::Lava) {
            return;
        }
        if self.b.block(self.state(pos)) != self.b.block(state) {
            return;
        }
        let water = [
            sunrise_core::geometry::Direction::North,
            sunrise_core::geometry::Direction::South,
            sunrise_core::geometry::Direction::West,
            sunrise_core::geometry::Direction::East,
            sunrise_core::geometry::Direction::Up,
        ]
        .into_iter()
        .any(|direction| self.b.liquid_of(self.state(rel(pos, direction))) == Some(Liquid::Water));
        if !water {
            return;
        }
        let meta = self.liquid_meta(state);
        if meta == 0 {
            self.set_block_and_update(pos, self.b.default_of(self.b.named.obsidian));
        } else if meta <= 4 {
            self.set_block_and_update(pos, self.b.default_of(self.b.named.cobblestone));
        }
        self.liquid_fizz(pos);
    }
    /// `FlowingLiquidBlock.spreadTo`: the replaced block's drops (water) or a
    /// hiss (lava), then the flowing block at `depth`.
    fn liquid_spread_to(&mut self, pos: BlockPos, liquid: Liquid, depth: i32) {
        let state = self.state(pos);
        if !self.liquid_can_spread_to(state, liquid) {
            return;
        }
        if liquid.is_lava() {
            self.liquid_fizz(pos);
        } else if !self.b.is_air(state) {
            self.drop_resources(state, pos);
        }
        let block = liquid.flowing_block(self.b);
        let target = self.liquid_state(block, depth);
        self.set_block(
            pos,
            target,
            super::flags::UPDATE_ALL | super::flags::UPDATE_KNOWN_SHAPE,
        );
    }
    /// `FlowingLiquidBlock.convertToSource`: the still block with the same
    /// metadata, sent to clients only.
    fn liquid_convert_to_source(&mut self, pos: BlockPos, liquid: Liquid, meta: i32) {
        let state = self.state(pos);
        if !self.b.is_block(state, liquid.flowing_block(self.b)) {
            return;
        }
        let source = liquid.source_block(self.b);
        let target = self.liquid_state(source, meta);
        self.set_block(
            pos,
            target,
            super::flags::UPDATE_CLIENTS | super::flags::UPDATE_KNOWN_SHAPE,
        );
    }
    /// `LiquidSourceBlock.convertToFlowing`: the moving block with the same
    /// metadata, then its first tick.
    fn liquid_convert_to_flowing(&mut self, pos: BlockPos, liquid: Liquid, meta: i32) {
        let state = self.state(pos);
        if !self.b.is_block(state, liquid.source_block(self.b)) {
            return;
        }
        let flowing = liquid.flowing_block(self.b);
        let target = self.liquid_state(flowing, meta);
        self.set_block(
            pos,
            target,
            super::flags::UPDATE_CLIENTS | super::flags::UPDATE_KNOWN_SHAPE,
        );
        let delay = self.liquid_tick_rate(liquid);
        self.schedule_tick(pos, flowing, delay);
    }
    /// `FlowingLiquidBlock.tick`.
    fn flowing_liquid_tick(&mut self, pos: BlockPos, liquid: Liquid) {
        let mut state = self.liquid_state_at(pos, liquid);
        // `var7`: lava loses two levels per block where water does not fall.
        let decay = if liquid.is_lava() && !self.level.dimension_type().water_evaporates {
            2
        } else {
            1
        };
        let mut delay = self.liquid_tick_rate(liquid);

        if state > 0 {
            let mut lowest = -100;
            let mut adjacent_sources = 0;
            for direction in LIQUID_DIRECTIONS {
                lowest = self.liquid_lowest_depth(
                    rel(pos, direction),
                    lowest,
                    liquid,
                    &mut adjacent_sources,
                );
            }
            let mut candidate = lowest + decay;
            if candidate >= 8 || lowest < 0 {
                candidate = -1;
            }

            let above = self.liquid_state_at(above(pos), liquid);
            if above >= 0 {
                candidate = if above >= 8 { above } else { above + 8 };
            }

            if adjacent_sources >= 2 && liquid.is_water() {
                let below_state = self.state(below(pos));
                if self.b.liquid_solid_material(below_state)
                    || (self.b.liquid_of(below_state) == Some(liquid)
                        && self.liquid_meta(below_state) == 0)
                {
                    candidate = 0;
                }
            }

            if liquid.is_lava()
                && state < 8
                && candidate < 8
                && candidate > state
                && self.random().next_int_bound(4).unwrap_or(0) != 0
            {
                delay *= 4;
            }

            if candidate == state {
                self.liquid_convert_to_source(pos, liquid, state);
            } else {
                state = candidate;
                if state < 0 {
                    self.remove_block(pos, false);
                } else {
                    let block = liquid.flowing_block(self.b);
                    let target = self.liquid_state(block, state);
                    // `setBlockMetadata(..., 2)`: clients only, no `onAdded`
                    // and no neighbour notice of its own.
                    self.set_block(
                        pos,
                        target,
                        super::flags::UPDATE_CLIENTS
                            | super::flags::UPDATE_SKIP_ON_PLACE
                            | super::flags::UPDATE_KNOWN_SHAPE,
                    );
                    self.schedule_tick(pos, block, delay);
                    self.update_neighbors_at(pos, block);
                }
            }
        } else {
            self.liquid_convert_to_source(pos, liquid, state);
        }

        let below_pos = below(pos);
        let below_state = self.state(below_pos);
        if self.liquid_can_spread_to(below_state, liquid) {
            if liquid.is_lava() && self.b.liquid_of(below_state) == Some(Liquid::Water) {
                self.set_block_and_update(below_pos, self.b.default_of(self.b.named.stone));
                self.liquid_fizz(below_pos);
                return;
            }
            let depth = if state >= 8 { state } else { state + 8 };
            self.liquid_spread_to(below_pos, liquid, depth);
        } else if state >= 0 && (state == 0 || self.b.liquid_blocking(below_state)) {
            let spread = self.liquid_spread_map(pos, liquid);
            let mut depth = state + decay;
            if state >= 8 {
                depth = 1;
            }
            if depth >= 8 {
                return;
            }
            for (index, direction) in LIQUID_DIRECTIONS.into_iter().enumerate() {
                if spread[index] {
                    self.liquid_spread_to(rel(pos, direction), liquid, depth);
                }
            }
        }
    }
    /// `LiquidBlock.isFlammable` of the block at `pos`, asked from `face`.
    fn liquid_lava_flammable(
        &self,
        pos: BlockPos,
        face: sunrise_core::geometry::Direction,
    ) -> bool {
        if self.is_inside_build_height(pos.y) && !self.level.is_loaded(pos) {
            return false;
        }
        let state = self.state(pos);
        let mut ignites = None;
        self.level
            .before_ignited_by_lava(state, pos, face, &mut ignites);
        ignites.unwrap_or_else(|| self.b.has_flag(state, state_flags::IGNITED_BY_LAVA))
    }
    /// `LiquidSourceBlock.tick`: lava alone, the three tries that light a
    /// neighbour on fire and the three under it. Source water does nothing.
    pub(crate) fn source_liquid_tick(&mut self, pos: BlockPos, liquid: Liquid) {
        if !liquid.is_lava() {
            return;
        }
        let mut x = pos.x;
        let mut y = pos.y;
        let mut z = pos.z;
        let passes = self.random().next_int_bound(3).unwrap_or(0);
        if passes > 0 {
            for _ in 0..passes {
                x += self.random().next_int_bound(3).unwrap_or(0) - 1;
                y += 1;
                z += self.random().next_int_bound(3).unwrap_or(0) - 1;
                let test = BlockPos::new(x, y, z);
                if !self.level.is_loaded(test) {
                    return;
                }
                let state = self.state(test);
                if self.b.is_air(state) {
                    let flammable = [
                        sunrise_core::geometry::Direction::West,
                        sunrise_core::geometry::Direction::East,
                        sunrise_core::geometry::Direction::North,
                        sunrise_core::geometry::Direction::South,
                        sunrise_core::geometry::Direction::Down,
                        sunrise_core::geometry::Direction::Up,
                    ]
                    .into_iter()
                    .any(|direction| self.liquid_lava_flammable(rel(test, direction), direction));
                    if flammable {
                        if !self.level.allow_block_ignite(test, "lava") {
                            return;
                        }
                        let fire = self.b.default_of(self.b.named.fire);
                        if let Some(state) = self.level.fluid_place_block(test, pos, fire) {
                            self.set_block_and_update(test, state);
                        }
                        return;
                    }
                } else if self.b.liquid_material_blocks_movement(state) {
                    return;
                }
            }
        } else {
            for _ in 0..3 {
                let dx = self.random().next_int_bound(3).unwrap_or(0) - 1;
                let dz = self.random().next_int_bound(3).unwrap_or(0) - 1;
                let test = pos.offset(dx, 0, dz);
                if !self.level.is_loaded(test) {
                    return;
                }
                let fire_pos = above(test);
                if self.b.is_air(self.state(fire_pos))
                    && self.liquid_lava_flammable(test, sunrise_core::geometry::Direction::Up)
                {
                    if !self.level.allow_block_ignite(fire_pos, "lava") {
                        continue;
                    }
                    let fire = self.b.default_of(self.b.named.fire);
                    if let Some(state) = self.level.fluid_place_block(fire_pos, pos, fire) {
                        self.set_block_and_update(fire_pos, state);
                    }
                }
            }
        }
    }
    /// `Block.tick` of the liquid at `pos`: the flowing rules, the source's
    /// lava fire, or nothing.
    pub(crate) fn fluid_tick(&mut self, pos: BlockPos) {
        let state = self.state(pos);
        let Some(liquid) = self.b.liquid_of(state) else {
            return;
        };
        let name = liquid.name();
        if !self.level.allow_fluid_tick(pos, name) {
            return;
        }
        if self.b.is_liquid_flowing(state) {
            self.flowing_liquid_tick(pos, liquid);
        } else {
            self.source_liquid_tick(pos, liquid);
        }
    }
    /// `Block.onAdded` of a liquid: the lava-water check, then a flowing
    /// block's first tick.
    pub(crate) fn liquid_block_added(&mut self, pos: BlockPos, state: BlockStateId) {
        self.liquid_check_collisions(pos, state);
        let now = self.state(pos);
        if let Some(liquid) = self.b.liquid_of(now)
            && self.b.is_liquid_flowing(now)
        {
            let delay = self.liquid_tick_rate(liquid);
            self.schedule_tick(pos, self.b.block(now), delay);
        }
    }
    /// `Block.neighborChanged` of a liquid: the lava-water check, then a
    /// source's conversion to its flowing form.
    pub(crate) fn liquid_block_neighbor_changed(&mut self, pos: BlockPos, state: BlockStateId) {
        self.liquid_check_collisions(pos, state);
        if let Some(liquid) = self.b.liquid_of(state)
            && !self.b.is_liquid_flowing(state)
        {
            let meta = self.liquid_meta(state);
            self.liquid_convert_to_flowing(pos, liquid, meta);
        }
    }

    // ---- liquid containers and bucket pickup ----

    /// `Fluid.getTickDelay`.
    fn fluid_tick_delay(&self, kind: FluidKind, pos: BlockPos) -> i32 {
        fluid_tick_delay(kind, self.level.dimension_type().fast_lava, || {
            kind.mod_fluid()
                .map(|fluid| self.level.mod_fluid_rules(pos, fluid, fluid.get().rules))
        })
    }
    /// `LiquidBlockContainer.canPlaceLiquid` for a liquid container.
    pub(crate) fn is_liquid_container(&self, state: BlockStateId) -> bool {
        self.b.is_liquid_container(state)
    }
    pub(crate) fn can_place_liquid(
        &self,
        state: BlockStateId,
        fluid: FluidKind,
        creative: bool,
    ) -> bool {
        self.b.can_place_liquid(state, fluid, creative)
    }
    /// What `placeLiquid` does once decided.
    pub(crate) fn apply_placed_liquid(&mut self, pos: BlockPos, placed: PlacedLiquid) {
        self.douse(pos, placed.douse);
        self.set_block_and_update(pos, placed.to);
        self.schedule_fluid_tick(pos, placed.fluid, placed.delay);
        self.placed_in_water(pos, placed);
    }
    /// What `LiquidBlockContainer.placeLiquid` of `fluid` into `state` at `pos`
    /// does, decided against the level as it is.
    pub(crate) fn plan_place_liquid(
        &self,
        pos: BlockPos,
        state: BlockStateId,
        fluid: FluidState,
        creative: bool,
    ) -> Option<PlacedLiquid> {
        let b = self.b;
        if !self.can_place_liquid(state, fluid.kind(), creative)
            || fluid.kind() != FluidKind::Water
            || !b.table.has(state, b.props.waterlogged)
            || b.is_waterlogged(state)
        {
            return None;
        }
        let mut to = b.table.set_bool(state, b.props.waterlogged, true);
        let data = b.data(state);
        let douse = if (data.is(Class::Candle) || data.is(Class::Campfire))
            && b.table.bool(state, b.props.lit)
        {
            to = b.table.set_bool(to, b.props.lit, false);
            Some(match data.is(Class::Campfire) {
                true => Douse::Campfire,
                false => Douse::Candle,
            })
        } else {
            None
        };
        Some(PlacedLiquid {
            to,
            fluid: fluid.kind(),
            delay: self.fluid_tick_delay(fluid.kind(), pos),
            douse,
            dried_ghast: data.is(Class::DriedGhast),
        })
    }
    /// `CampfireBlock.douse` and `AbstractCandleBlock.extinguish`.
    pub(crate) fn douse(&mut self, pos: BlockPos, douse: Option<Douse>) {
        let Some(douse) = douse else {
            return;
        };
        let sound = match douse {
            Douse::Campfire => "minecraft:entity.generic.extinguish_fire",
            Douse::Candle => "minecraft:block.candle.extinguish",
        };
        self.emit(super::LevelEvent::Sound {
            pos,
            sound: sound.into(),
            source: None,
            volume: 1.0,
            pitch: 1.0,
        });
        self.game_event(pos, "minecraft:block_change");
    }
    /// `DriedGhastBlock.placeLiquid`: its own place-in-water sound.
    pub(crate) fn placed_in_water(&mut self, pos: BlockPos, placed: PlacedLiquid) {
        if placed.dried_ghast {
            self.emit(super::LevelEvent::Sound {
                pos,
                sound: "minecraft:block.dried_ghast.place_in_water".into(),
                source: None,
                volume: 1.0,
                pitch: 1.0,
            });
        }
    }
    /// `BucketPickup.pickupBlock`.
    pub(crate) fn pickup_block(
        &mut self,
        pos: BlockPos,
        state: BlockStateId,
        creative: bool,
    ) -> bool {
        let Some(drain) = self.plan_pickup_block(pos, state, creative) else {
            return false;
        };
        self.set_block(pos, drain.to, drain.flags);
        if drain.burst {
            self.level_event(2001, pos, state.0 as i32);
        }
        if drain.drained && !self.can_survive(state, pos) {
            self.destroy_block(pos, true, 512);
        }
        true
    }
    /// What `BucketPickup.pickupBlock` does to `state` at `pos`, decided
    /// against the level as it is: None where the block gives nothing.
    pub(crate) fn plan_pickup_block(
        &self,
        pos: BlockPos,
        state: BlockStateId,
        creative: bool,
    ) -> Option<Drain> {
        let b = self.b;
        let removed = |burst| Drain {
            to: b.air(),
            flags: 11,
            drained: false,
            breaks: false,
            burst,
        };
        if b.liquid_of(state).is_some() {
            // `BucketItem`: a liquid at metadata 0 fills a bucket.
            return (b.table.int(state, b.props.meta) == 0).then(|| removed(false));
        }
        if b.implements(state, super::classes::Interface::SimpleWaterlogged)
            && b.table.has(state, b.props.waterlogged)
        {
            if (b.instance_of(state, Class::Barrier) && !creative) || !b.is_waterlogged(state) {
                return None;
            }
            return Some(Drain {
                to: b.table.set_bool(state, b.props.waterlogged, false),
                flags: super::flags::UPDATE_ALL,
                drained: true,
                breaks: !self.can_survive(state, pos),
                burst: false,
            });
        }
        if b.is_block(state, b.named.bubble_column) {
            return Some(removed(false));
        }
        b.is_block(state, b.named.powder_snow)
            .then(|| removed(true))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fluid::{ModFluidType, register_mod_fluid};

    fn mod_fluid(id: &str, tick_delay: i32) -> FluidKind {
        FluidKind::Mod(
            register_mod_fluid(ModFluidType {
                id: format!("fluid_facts:{id}"),
                flowing_id: format!("fluid_facts:flowing_{id}"),
                bucket: None,
                rules: ModFluidRules {
                    tick_delay,
                    drop_off: 1,
                    slope_find_distance: 4,
                    converts_to_source: false,
                },
            })
            .unwrap(),
        )
    }

    /// `Fluid.getTickDelay`: water 5 in every dimension, lava 30 and 10 where
    /// it flows fast (the Nether), the empty fluid nothing.
    #[test]
    fn tick_delay_follows_the_fluid_and_the_dimension() {
        let delay = |kind, fast| fluid_tick_delay(kind, fast, || None);
        for fast in [false, true] {
            assert_eq!(delay(FluidKind::Water, fast), 5);
            assert_eq!(delay(FluidKind::FlowingWater, fast), 5);
            assert_eq!(delay(FluidKind::Empty, fast), 0);
        }
        assert_eq!(delay(FluidKind::Lava, false), 30);
        assert_eq!(delay(FluidKind::FlowingLava, false), 30);
        assert_eq!(delay(FluidKind::Lava, true), 10);
        assert_eq!(delay(FluidKind::FlowingLava, true), 10);
    }

    #[test]
    fn a_mod_fluid_delays_as_registered_or_as_its_parts_say() {
        let oil = mod_fluid("tick_delay", 20);
        for fast in [false, true] {
            assert_eq!(fluid_tick_delay(oil, fast, || None), 20);
        }
        let faster = ModFluidRules {
            tick_delay: 7,
            ..oil.mod_fluid().unwrap().get().rules
        };
        assert_eq!(fluid_tick_delay(oil, false, || Some(faster)), 7);
        assert_eq!(
            fluid_tick_delay(FluidKind::Lava, false, || unreachable!()),
            30
        );
    }

    /// Vanilla's fluid constants: 100 for a fluid, nothing for the empty one;
    /// only lava ticks randomly.
    #[test]
    fn fluid_constants_match_vanilla() {
        let oil = mod_fluid("constants", 5);
        let flowing_oil = FluidKind::ModFlowing(oil.mod_fluid().unwrap());
        for (kind, resistance, random) in [
            (FluidKind::Empty, 0.0, false),
            (FluidKind::Water, 100.0, false),
            (FluidKind::FlowingWater, 100.0, false),
            (FluidKind::Lava, 100.0, true),
            (FluidKind::FlowingLava, 100.0, true),
            (oil, 100.0, false),
            (flowing_oil, 100.0, false),
        ] {
            assert_eq!(fluid_explosion_resistance(kind), resistance, "{kind:?}");
            assert_eq!(fluid_randomly_ticks(kind), random, "{kind:?}");
        }
    }
}
