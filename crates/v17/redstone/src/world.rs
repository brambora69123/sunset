//! The redstone world: block grid, block entities, scheduled ticks, block
//! events and the power queries, ported from `net.minecraft.world.World`,
//! `net.minecraft.server.world.ServerWorld` and `WorldChunk`.

use std::collections::{BTreeSet, HashMap, HashSet};

use sunset_1_7_core::LegacyRandom;

use crate::block::{Block, directions, facings};
use crate::item::{InvKind, Inventory, ItemStack};
use crate::{diode, dispenser, hopper, lever, piston, torch, wire};

/// An integer block position (`BlockPos`).
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct Pos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Pos {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Pos { x, y, z }
    }

    pub fn offset(self, dir: i32) -> Self {
        Pos {
            x: self.x + directions::X_OFFSET[dir as usize],
            y: self.y + directions::Y_OFFSET[dir as usize],
            z: self.z + directions::Z_OFFSET[dir as usize],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub block: Block,
    pub meta: i32,
}

/// Block entities the redstone subsystem keeps: comparator output signals,
/// container inventories and moving piston blocks.
pub enum BlockEntity {
    Comparator { output: i32 },
    Inventory(Inventory),
    MovingPiston(MovingBlockEntity),
}

/// `MovingBlockEntity`: a block mid-move between the piston and its target.
pub struct MovingBlockEntity {
    pub moved_block: Block,
    pub moved_metadata: i32,
    pub facing: i32,
    pub extending: bool,
    pub source: bool,
    pub progress: f32,
    pub last_progress: f32,
}

impl MovingBlockEntity {
    pub fn new(
        moved_block: Block,
        moved_metadata: i32,
        facing: i32,
        extending: bool,
        source: bool,
    ) -> Self {
        MovingBlockEntity {
            moved_block,
            moved_metadata,
            facing,
            extending,
            source,
            progress: 0.0,
            last_progress: 0.0,
        }
    }

    /// `MovingBlockEntity.tick`: the move progresses half a block per tick;
    /// entities riding the block are not simulated.
    pub fn tick(&mut self) {
        self.last_progress = self.progress;
        if self.last_progress >= 1.0 {
            return;
        }
        self.progress += 0.5;
        if self.progress >= 1.0 {
            self.progress = 1.0;
        }
    }
}

/// `ScheduledTick`: deduped by position and block, ordered by time, priority,
/// then the order it was scheduled (`ScheduledTick.compareTo`).
#[derive(Clone, Debug)]
pub struct ScheduledTick {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub block: Block,
    pub time: i64,
    pub priority: i32,
    pub id: u64,
}

impl PartialEq for ScheduledTick {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y && self.z == other.z && self.block == other.block
    }
}
impl Eq for ScheduledTick {}

impl std::hash::Hash for ScheduledTick {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.x.hash(state);
        self.y.hash(state);
        self.z.hash(state);
        self.block.hash(state);
    }
}

impl Ord for ScheduledTick {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.time
            .cmp(&other.time)
            .then(self.priority.cmp(&other.priority))
            .then(self.id.cmp(&other.id))
    }
}

impl PartialOrd for ScheduledTick {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// `ServerWorld$BlockEvent` / `net.minecraft.world.BlockEvent`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockEvent {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub block: Block,
    pub kind: i32,
    pub data: i32,
}

/// `RedstoneTorchBlock$ToggleEntry` for the torch burnout check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToggleEntry {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub time: i64,
}

/// A fully loaded single-player world: `isMultiplayer` is false, every chunk
/// is loaded (`isAreaLoaded` true), and random chunk ticks are not ported.
pub struct World {
    cells: HashMap<Pos, Cell>,
    pub(crate) block_entities: HashMap<Pos, BlockEntity>,
    block_entity_order: Vec<Pos>,
    /// `data.getTime()`.
    pub time: i64,
    /// `world.random`.
    pub rng: LegacyRandom,
    next_tick_id: u64,
    scheduled_ticks: HashSet<ScheduledTick>,
    scheduled_ticks_in_order: BTreeSet<ScheduledTick>,
    /// `scheduledTicksThisTick`.
    pub this_tick: Vec<ScheduledTick>,
    /// The two `ServerWorld$BlockEventQueue` buffers.
    block_events: [Vec<BlockEvent>; 2],
    next_block_event_queue: usize,
    /// `RedstoneTorchBlock.RECENT_TOGGLES` for this world.
    pub torch_toggles: Vec<ToggleEntry>,
    /// `RedstoneWireBlock.neighborsToUpdate`.
    pub wire_neighbors_to_update: Vec<Pos>,
    /// `RedstoneWireBlock.shouldSignal`, false while power propagates.
    pub wire_should_signal: bool,
    /// Every `notifyBlockChanged` (render listeners in vanilla).
    pub changed: Vec<Pos>,
    /// Item spawns recorded instead of simulated (no entities in this crate).
    pub item_drops: Vec<(f64, f64, f64, ItemStack)>,
    /// `World.doEvent` records: (type, x, y, z, data).
    pub events: Vec<(i32, i32, i32, i32, i32)>,
}

impl World {
    pub fn new(seed: i64) -> Self {
        World {
            cells: HashMap::new(),
            block_entities: HashMap::new(),
            block_entity_order: Vec::new(),
            time: 0,
            rng: LegacyRandom::new(seed),
            next_tick_id: 0,
            scheduled_ticks: HashSet::new(),
            scheduled_ticks_in_order: BTreeSet::new(),
            this_tick: Vec::new(),
            block_events: [Vec::new(), Vec::new()],
            next_block_event_queue: 0,
            torch_toggles: Vec::new(),
            wire_neighbors_to_update: Vec::new(),
            wire_should_signal: true,
            changed: Vec::new(),
            item_drops: Vec::new(),
            events: Vec::new(),
        }
    }

    /// One server tick: `data.time++` -> scheduled ticks -> block events ->
    /// block entities (`ServerWorld.tick` + `tickEntities`, entities aside).
    pub fn tick(&mut self) {
        self.time += 1;
        self.do_scheduled_ticks(false);
        self.do_block_events();
        self.tick_block_entities();
    }

    // ------------------------------------------------------------------
    // Blocks
    // ------------------------------------------------------------------

    /// `World.getBlock`: out of bounds and unloaded positions are Air.
    pub fn get_block(&self, x: i32, y: i32, z: i32) -> Block {
        self.cells
            .get(&Pos::new(x, y, z))
            .map(|c| c.block)
            .unwrap_or(Block::Air)
    }

    pub fn get_block_at(&self, pos: Pos) -> Block {
        self.get_block(pos.x, pos.y, pos.z)
    }

    /// `World.getBlockMetadata`.
    pub fn get_block_metadata(&self, x: i32, y: i32, z: i32) -> i32 {
        self.cells
            .get(&Pos::new(x, y, z))
            .map(|c| c.meta)
            .unwrap_or(0)
    }

    /// `World.isSolidBlockingCube(x, y, z, true)` with every chunk loaded.
    pub fn is_solid_blocking_cube(&self, x: i32, y: i32, z: i32) -> bool {
        let block = self.get_block(x, y, z);
        block.is_solid_blocking() && block.is_cube()
    }

    /// `World.hasSolidTop`: stairs, slabs and snow layers are not represented
    /// in this port, so only solid cubes and hoppers carry a solid top.
    pub fn has_solid_top(&self, x: i32, y: i32, z: i32) -> bool {
        let block = self.get_block(x, y, z);
        (block.material().solid_blocking && block.is_cube()) || block == Block::Hopper
    }

    /// `World.setBlockWithMetadata`: `onRemoved` of the old block, then
    /// `onAdded` of the new (`WorldChunk.setBlockWithMetadataAt` order), then
    /// the flag 2 notify and the flag 1 neighbor cascade.
    pub fn set_block_with_metadata(
        &mut self,
        x: i32,
        y: i32,
        z: i32,
        block: Block,
        metadata: i32,
        flags: i32,
    ) -> bool {
        if x < -30_000_000 || z < -30_000_000 || x >= 30_000_000 || z >= 30_000_000 {
            return false;
        }
        if !(0..256).contains(&y) {
            return false;
        }

        let pos = Pos::new(x, y, z);
        let old = self.get_block(x, y, z);
        let old_meta = self.get_block_metadata(x, y, z);
        if old == block && old_meta == metadata {
            return false;
        }

        self.cells.insert(
            pos,
            Cell {
                block,
                meta: metadata,
            },
        );

        // oldBlock.beforeRemoved (no block has state to clear) and onRemoved.
        self.block_on_removed(x, y, z, old, old_meta);

        // block.onAdded.
        self.block_on_added(x, y, z, block);

        // Block entity creation for BlockEntityProvider blocks (chunk step 9):
        // an existing block entity is kept, as on a comparator state swap.
        self.create_block_entity_for(block, pos);

        if flags & 2 != 0 {
            self.notify_block_changed(x, y, z);
        }
        if flags & 1 != 0 {
            self.update_neighbors(x, y, z, old);
            if block.is_analog_signal_source() {
                self.update_neighbor_comparators(x, y, z, block);
            }
        }

        true
    }

    /// `World.setBlockMetadata`.
    pub fn set_block_metadata(
        &mut self,
        x: i32,
        y: i32,
        z: i32,
        metadata: i32,
        flags: i32,
    ) -> bool {
        if x < -30_000_000 || z < -30_000_000 || x >= 30_000_000 || z >= 30_000_000 {
            return false;
        }
        if !(0..256).contains(&y) {
            return false;
        }

        let pos = Pos::new(x, y, z);
        match self.cells.get_mut(&pos) {
            Some(cell) => {
                if cell.meta == metadata {
                    return false;
                }
                cell.meta = metadata;
            }
            None => return false,
        }

        if flags & 2 != 0 {
            self.notify_block_changed(x, y, z);
        }
        if flags & 1 != 0 {
            let block = self.get_block(x, y, z);
            self.update_neighbors(x, y, z, block);
            if block.is_analog_signal_source() {
                self.update_neighbor_comparators(x, y, z, block);
            }
        }

        true
    }

    /// `World.setBlock`.
    pub fn set_block(&mut self, x: i32, y: i32, z: i32, block: Block) -> bool {
        self.set_block_with_metadata(x, y, z, block, 0, 3)
    }

    /// `World.removeBlock`.
    pub fn remove_block(&mut self, x: i32, y: i32, z: i32) -> bool {
        self.set_block_with_metadata(x, y, z, Block::Air, 0, 3)
    }

    /// `World.notifyBlockChanged`.
    pub fn notify_block_changed(&mut self, x: i32, y: i32, z: i32) {
        self.changed.push(Pos::new(x, y, z));
    }

    fn create_block_entity_for(&mut self, block: Block, pos: Pos) {
        if self.block_entities.contains_key(&pos) {
            return;
        }
        let entity = match block {
            Block::Hopper => Some(BlockEntity::Inventory(Inventory::new(InvKind::Hopper, 0))),
            Block::Dispenser => Some(BlockEntity::Inventory(Inventory::new(
                InvKind::Dispenser,
                0,
            ))),
            Block::Dropper => Some(BlockEntity::Inventory(Inventory::new(InvKind::Dropper, 0))),
            Block::Comparator | Block::PoweredComparator => {
                Some(BlockEntity::Comparator { output: 0 })
            }
            _ => None,
        };
        if let Some(entity) = entity {
            self.set_block_entity(pos, entity);
        }
    }

    // ------------------------------------------------------------------
    // Block entities
    // ------------------------------------------------------------------

    /// `World.setBlockEntity`: a fresh entity joins the end of the tick list. A
    /// position whose old entity was removed moves to the end instead of
    /// keeping a stale entry that would tick it twice.
    pub fn set_block_entity(&mut self, pos: Pos, entity: BlockEntity) {
        if !self.block_entities.contains_key(&pos) {
            if let Some(index) = self.block_entity_order.iter().position(|p| *p == pos) {
                self.block_entity_order.remove(index);
            }
            self.block_entity_order.push(pos);
        }
        self.block_entities.insert(pos, entity);
    }

    /// `World.removeBlockEntity`.
    pub fn remove_block_entity(&mut self, pos: Pos) {
        self.block_entities.remove(&pos);
    }

    pub fn block_entity(&self, pos: Pos) -> Option<&BlockEntity> {
        self.block_entities.get(&pos)
    }

    /// The comparator output signal at a position (`ComparatorBlockEntity`).
    pub fn comparator_output(&self, pos: Pos) -> i32 {
        match self.block_entities.get(&pos) {
            Some(BlockEntity::Comparator { output }) => *output,
            _ => 0,
        }
    }

    /// `InventoryMenu.getAnalogSignal` for the inventory at a position, or 0.
    pub fn inventory_analog_signal(&self, pos: Pos) -> i32 {
        match self.block_entities.get(&pos) {
            Some(BlockEntity::Inventory(inventory)) => inventory.get_analog_signal(),
            _ => 0,
        }
    }

    pub fn set_comparator_output(&mut self, pos: Pos, output: i32) {
        if let Some(BlockEntity::Comparator { output: slot }) = self.block_entities.get_mut(&pos) {
            *slot = output;
        }
    }

    /// The inventory at a position, for the test and probe surface.
    pub fn inventory_at(&mut self, pos: Pos) -> Option<&mut Inventory> {
        self.block_entities.get_mut(&pos).and_then(|e| match e {
            BlockEntity::Inventory(inventory) => Some(inventory),
            _ => None,
        })
    }

    /// The size of the ordered scheduled-tick list.
    pub fn scheduled_tick_count(&self) -> usize {
        self.scheduled_ticks_in_order.len()
    }

    /// The size of the current block-event queue.
    pub fn block_event_count(&self) -> usize {
        self.block_events[self.next_block_event_queue].len()
    }

    /// `HopperBlockEntity.getInventoryAt`: the block entity at a position when
    /// it is an inventory; item-entity inventories are not simulated.
    pub fn get_inventory_at(&mut self, pos: Pos) -> Option<&mut Inventory> {
        match self.block_entities.get_mut(&pos) {
            Some(BlockEntity::Inventory(inventory)) => Some(inventory),
            _ => None,
        }
    }

    /// `World.tickEntities` for block entities: every block entity ticks once,
    /// in the order it was added. Java defers entities created while ticking to
    /// the next tick (`pendingBlockEntities`); the entities that tick here (a
    /// hopper, a moving piston) never create one, so a single ordered pass ends
    /// in the same state.
    fn tick_block_entities(&mut self) {
        let mut index = 0;
        while index < self.block_entity_order.len() {
            let pos = self.block_entity_order[index];
            index += 1;
            if !self.block_entities.contains_key(&pos) {
                continue;
            }
            let block = self.get_block(pos.x, pos.y, pos.z);
            let is_hopper = block == Block::Hopper;
            let is_moving = block == Block::MovingPiston;
            if is_hopper {
                hopper::tick_hopper(self, pos);
            } else if is_moving {
                let mut finish = None;
                if let Some(BlockEntity::MovingPiston(moving)) = self.block_entities.get_mut(&pos) {
                    moving.tick();
                    if moving.last_progress >= 1.0 {
                        finish = Some((moving.moved_block, moving.moved_metadata));
                    }
                }
                if let Some((moved_block, moved_metadata)) = finish {
                    self.block_entities.remove(&pos);
                    if self.get_block(pos.x, pos.y, pos.z) == Block::MovingPiston {
                        self.set_block_with_metadata(
                            pos.x,
                            pos.y,
                            pos.z,
                            moved_block,
                            moved_metadata,
                            3,
                        );
                        let moved = moved_block;
                        self.neighbor_changed(pos.x, pos.y, pos.z, moved);
                    }
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // Scheduled ticks (`ServerWorld`)
    // ------------------------------------------------------------------

    /// `ServerWorld.scheduleTick` with `doTicksImmediately` false.
    pub fn schedule_tick(
        &mut self,
        x: i32,
        y: i32,
        z: i32,
        block: Block,
        delay: i32,
        priority: i32,
    ) {
        if block == Block::Air {
            return;
        }
        let tick = ScheduledTick {
            x,
            y,
            z,
            block,
            time: self.time + delay as i64,
            priority,
            id: self.next_tick_id,
        };
        if !self.scheduled_ticks.contains(&tick) {
            self.next_tick_id += 1;
            self.scheduled_ticks.insert(tick.clone());
            self.scheduled_ticks_in_order.insert(tick);
        }
    }

    /// `ServerWorld.willTickThisTick`.
    pub fn will_tick_this_tick(&self, x: i32, y: i32, z: i32, block: Block) -> bool {
        self.this_tick
            .iter()
            .any(|t| t.x == x && t.y == y && t.z == z && t.block == block)
    }

    /// `ServerWorld.doScheduledTicks`: up to 1000 due ticks run, in time,
    /// priority and schedule order. Entries stay in `this_tick` until they run,
    /// so `will_tick_this_tick` sees the ones still queued in this batch.
    pub fn do_scheduled_ticks(&mut self, flush: bool) {
        let mut due = 0;
        let total = self.scheduled_ticks_in_order.len();
        let limit = total.min(1000);
        while due < limit {
            let first = self
                .scheduled_ticks_in_order
                .iter()
                .next()
                .expect("ordered set not empty")
                .clone();
            if !flush && first.time > self.time {
                break;
            }
            self.scheduled_ticks_in_order.remove(&first);
            self.scheduled_ticks.remove(&first);
            self.this_tick.push(first);
            due += 1;
        }

        while !self.this_tick.is_empty() {
            let tick = self.this_tick.remove(0);
            let block = self.get_block(tick.x, tick.y, tick.z);
            if block != Block::Air && block == tick.block {
                self.tick_block(tick.x, tick.y, tick.z, block);
            }
        }
        self.this_tick.clear();
    }

    // ------------------------------------------------------------------
    // Block events
    // ------------------------------------------------------------------

    /// `ServerWorld.addBlockEvent`: deduped against the current queue.
    pub fn add_block_event(&mut self, x: i32, y: i32, z: i32, block: Block, kind: i32, data: i32) {
        let event = BlockEvent {
            x,
            y,
            z,
            block,
            kind,
            data,
        };
        if self.block_events[self.next_block_event_queue].contains(&event) {
            return;
        }
        self.block_events[self.next_block_event_queue].push(event);
    }

    /// `ServerWorld.doBlockEvents`: the double-buffered queues drain, new
    /// events raised while draining wait for the next tick.
    fn do_block_events(&mut self) {
        while !self.block_events[self.next_block_event_queue].is_empty() {
            let index = self.next_block_event_queue;
            self.next_block_event_queue ^= 1;
            let events = std::mem::take(&mut self.block_events[index]);
            for event in &events {
                if self.get_block(event.x, event.y, event.z) == event.block {
                    self.do_block_event(
                        event.x,
                        event.y,
                        event.z,
                        event.block,
                        event.kind,
                        event.data,
                    );
                }
            }
            self.block_events[index].clear();
        }
    }

    /// `ServerWorld.doBlockEvent` / `Block.doEvent`: the event handler for the
    /// block kind at the position.
    fn do_block_event(&mut self, x: i32, y: i32, z: i32, block: Block, kind: i32, data: i32) {
        match block {
            Block::Piston | Block::StickyPiston => {
                piston::do_event(self, x, y, z, kind, data);
            }
            _ => {}
        }
    }

    /// `World.doEvent` (sound and particle events are recorded, not played).
    pub fn do_event(&mut self, kind: i32, x: i32, y: i32, z: i32, data: i32) {
        self.events.push((kind, x, y, z, data));
    }

    // ------------------------------------------------------------------
    // Neighbor updates (`World`)
    // ------------------------------------------------------------------

    /// `World.updateNeighbors`.
    pub fn update_neighbors(&mut self, x: i32, y: i32, z: i32, source: Block) {
        self.neighbor_changed(x - 1, y, z, source);
        self.neighbor_changed(x + 1, y, z, source);
        self.neighbor_changed(x, y - 1, z, source);
        self.neighbor_changed(x, y + 1, z, source);
        self.neighbor_changed(x, y, z - 1, source);
        self.neighbor_changed(x, y, z + 1, source);
    }

    /// `World.updateNeighborsExcept`.
    pub fn update_neighbors_except(&mut self, x: i32, y: i32, z: i32, source: Block, dir: i32) {
        if dir != directions::WEST {
            self.neighbor_changed(x - 1, y, z, source);
        }
        if dir != directions::EAST {
            self.neighbor_changed(x + 1, y, z, source);
        }
        if dir != directions::DOWN {
            self.neighbor_changed(x, y - 1, z, source);
        }
        if dir != directions::UP {
            self.neighbor_changed(x, y + 1, z, source);
        }
        if dir != directions::NORTH {
            self.neighbor_changed(x, y, z - 1, source);
        }
        if dir != directions::SOUTH {
            self.neighbor_changed(x, y, z + 1, source);
        }
    }

    /// `World.neighborChanged`: the block at the position reacts.
    pub fn neighbor_changed(&mut self, x: i32, y: i32, z: i32, source: Block) {
        let block = self.get_block(x, y, z);
        self.block_neighbor_changed(x, y, z, block, source);
    }

    /// `World.updateNeighborComparators`: comparators next to or, through a
    /// solid block, one block away on the horizontal ring.
    pub fn update_neighbor_comparators(&mut self, x: i32, y: i32, z: i32, source: Block) {
        for var5 in 0..4 {
            let mut var6 = x + facings::X_OFFSET[var5];
            let mut var7 = z + facings::Z_OFFSET[var5];
            let var8 = self.get_block(var6, y, var7);
            if crate::block::is_comparator(var8) {
                self.block_neighbor_changed(var6, y, var7, var8, source);
            } else if var8.is_solid() {
                var6 += facings::X_OFFSET[var5];
                var7 += facings::Z_OFFSET[var5];
                let var9 = self.get_block(var6, y, var7);
                if crate::block::is_comparator(var9) {
                    self.block_neighbor_changed(var6, y, var7, var9, source);
                }
            }
        }
    }

    /// Item drops are recorded, never spawned: this crate has no entities.
    pub fn drop_items(&mut self, _x: i32, _y: i32, _z: i32, _meta: i32, _fortune: i32) {}

    // ------------------------------------------------------------------
    // Signal queries (`World`)
    // ------------------------------------------------------------------

    /// `World.getDirectSignal`: strong power into a position from one side.
    pub fn get_direct_signal(&self, x: i32, y: i32, z: i32, dir: i32) -> i32 {
        let block = self.get_block(x, y, z);
        self.block_get_direct_signal(x, y, z, block, dir)
    }

    /// `World.getDirectNeighborSignal`: max strong power from the six sides.
    pub fn get_direct_neighbor_signal(&self, x: i32, y: i32, z: i32) -> i32 {
        let mut max = 0;
        for dir in 0..6i32 {
            max = max.max(self.get_direct_signal(
                x + directions::X_OFFSET[dir as usize],
                y + directions::Y_OFFSET[dir as usize],
                z + directions::Z_OFFSET[dir as usize],
                dir,
            ));
            if max >= 15 {
                return max;
            }
        }
        max
    }

    /// `World.getSignal`: through a solid block the strong power of its
    /// neighbors arrives, otherwise the block's own weak emission toward `dir`.
    pub fn get_signal(&self, x: i32, y: i32, z: i32, dir: i32) -> i32 {
        let block = self.get_block(x, y, z);
        if block.is_solid() {
            self.get_direct_neighbor_signal(x, y, z)
        } else {
            self.block_get_signal(x, y, z, block, dir)
        }
    }

    /// `World.hasSignal`.
    pub fn has_signal(&self, x: i32, y: i32, z: i32, dir: i32) -> bool {
        self.get_signal(x, y, z, dir) > 0
    }

    /// `World.getNeighborSignal`: max weak power over the six sides.
    pub fn get_neighbor_signal(&self, x: i32, y: i32, z: i32) -> i32 {
        let mut max = 0;
        for dir in 0..6i32 {
            let signal = self.get_signal(
                x + directions::X_OFFSET[dir as usize],
                y + directions::Y_OFFSET[dir as usize],
                z + directions::Z_OFFSET[dir as usize],
                dir,
            );
            if signal >= 15 {
                return 15;
            }
            if signal > max {
                max = signal;
            }
        }
        max
    }

    /// `World.hasNeighborSignal`.
    pub fn has_neighbor_signal(&self, x: i32, y: i32, z: i32) -> bool {
        if self.get_signal(x, y - 1, z, directions::DOWN) > 0 {
            return true;
        }
        if self.get_signal(x, y + 1, z, directions::UP) > 0 {
            return true;
        }
        if self.get_signal(x, y, z - 1, directions::NORTH) > 0 {
            return true;
        }
        if self.get_signal(x, y, z + 1, directions::SOUTH) > 0 {
            return true;
        }
        self.get_signal(x - 1, y, z, directions::WEST) > 0
            || self.get_signal(x + 1, y, z, directions::EAST) > 0
    }

    // ------------------------------------------------------------------
    // Per-block dispatch
    // ------------------------------------------------------------------

    fn block_get_signal(&self, x: i32, y: i32, z: i32, block: Block, dir: i32) -> i32 {
        match block {
            Block::RedstoneWire => wire::get_signal(self, x, y, z, dir),
            Block::RedstoneTorch | Block::UnlitRedstoneTorch => {
                torch::get_signal(self, x, y, z, dir)
            }
            Block::Repeater
            | Block::PoweredRepeater
            | Block::Comparator
            | Block::PoweredComparator => diode::get_signal(self, x, y, z, block, dir),
            Block::Lever => lever::get_signal(self, x, y, z),
            Block::RedstoneBlock => 15,
            _ => 0,
        }
    }

    fn block_get_direct_signal(&self, x: i32, y: i32, z: i32, block: Block, dir: i32) -> i32 {
        match block {
            Block::RedstoneWire => wire::get_direct_signal(self, x, y, z, dir),
            Block::RedstoneTorch | Block::UnlitRedstoneTorch => {
                torch::get_direct_signal(self, x, y, z, dir)
            }
            Block::Repeater
            | Block::PoweredRepeater
            | Block::Comparator
            | Block::PoweredComparator => diode::get_signal(self, x, y, z, block, dir),
            Block::Lever => lever::get_direct_signal(self, x, y, z, dir),
            _ => 0,
        }
    }

    pub(crate) fn block_neighbor_changed(
        &mut self,
        x: i32,
        y: i32,
        z: i32,
        block: Block,
        source: Block,
    ) {
        match block {
            Block::RedstoneWire => wire::neighbor_changed(self, x, y, z),
            Block::RedstoneTorch | Block::UnlitRedstoneTorch => {
                torch::neighbor_changed(self, x, y, z)
            }
            Block::Repeater
            | Block::PoweredRepeater
            | Block::Comparator
            | Block::PoweredComparator => diode::neighbor_changed(self, x, y, z, block),
            Block::Piston | Block::StickyPiston => piston::neighbor_changed(self, x, y, z),
            Block::PistonHead => piston::head_neighbor_changed(self, x, y, z, source),
            Block::Hopper => hopper::neighbor_changed(self, x, y, z),
            Block::Dispenser | Block::Dropper => dispenser::neighbor_changed(self, x, y, z),
            Block::Lever => lever::neighbor_changed(self, x, y, z),
            Block::RedstoneLamp | Block::LitRedstoneLamp => {
                lamp::neighbor_changed(self, x, y, z, block)
            }
            _ => {}
        }
    }

    fn tick_block(&mut self, x: i32, y: i32, z: i32, block: Block) {
        match block {
            Block::RedstoneTorch | Block::UnlitRedstoneTorch => torch::tick(self, x, y, z),
            Block::Repeater | Block::PoweredRepeater => diode::tick(self, x, y, z, block),
            Block::Comparator | Block::PoweredComparator => {
                diode::comparator_tick(self, x, y, z, block)
            }
            Block::Dispenser | Block::Dropper => dispenser::tick(self, Pos::new(x, y, z)),
            Block::RedstoneLamp | Block::LitRedstoneLamp => lamp::tick(self, x, y, z, block),
            _ => {}
        }
    }

    /// `oldBlock.onRemoved` (`WorldChunk.setBlockWithMetadataAt`, step 4).
    fn block_on_removed(&mut self, x: i32, y: i32, z: i32, block: Block, meta: i32) {
        match block {
            Block::RedstoneWire => wire::on_removed(self, x, y, z),
            Block::RedstoneTorch | Block::UnlitRedstoneTorch => {
                torch::on_removed(self, x, y, z, block)
            }
            Block::Repeater | Block::PoweredRepeater => diode::on_removed(self, x, y, z, block),
            Block::Comparator | Block::PoweredComparator => diode::on_removed(self, x, y, z, block),
            Block::PistonHead => piston::head_on_removed(self, x, y, z, meta),
            Block::MovingPiston => piston::moving_on_removed(self, x, y, z),
            Block::Hopper => hopper::on_removed(self, x, y, z),
            Block::Dispenser | Block::Dropper => dispenser::on_removed(self, x, y, z, block),
            Block::Lever => lever::on_removed(self, x, y, z, meta),
            _ => {}
        }
        // BlockWithBlockEntity.onRemoved removes the block entity.
        if block.has_block_entity() {
            self.remove_block_entity(Pos::new(x, y, z));
        }
    }

    /// `block.onAdded` (`WorldChunk.setBlockWithMetadataAt`, step 8).
    fn block_on_added(&mut self, x: i32, y: i32, z: i32, block: Block) {
        match block {
            Block::RedstoneWire => wire::on_added(self, x, y, z),
            Block::RedstoneTorch | Block::UnlitRedstoneTorch => {
                torch::on_added(self, x, y, z, block)
            }
            Block::Repeater
            | Block::PoweredRepeater
            | Block::Comparator
            | Block::PoweredComparator => diode::on_added(self, x, y, z, block),
            Block::Piston | Block::StickyPiston => piston::on_added(self, x, y, z),
            Block::Hopper => hopper::on_added(self, x, y, z),
            Block::Dispenser | Block::Dropper => dispenser::on_added(self, x, y, z),
            Block::RedstoneLamp | Block::LitRedstoneLamp => lamp::on_added(self, x, y, z, block),
            _ => {}
        }
    }
}

mod lamp {
    //! `RedstoneLampBlock` and `LitRedstoneLampBlock`.

    use crate::block::Block;
    use crate::world::World;

    fn is_lit(block: Block) -> bool {
        block == Block::LitRedstoneLamp
    }

    pub fn on_added(world: &mut World, x: i32, y: i32, z: i32, block: Block) {
        let lit = is_lit(block);
        if lit && !world.has_neighbor_signal(x, y, z) {
            world.schedule_tick(x, y, z, block, 4, 0);
        } else if !lit && world.has_neighbor_signal(x, y, z) {
            world.set_block_with_metadata(x, y, z, Block::LitRedstoneLamp, 0, 2);
        }
    }

    pub fn neighbor_changed(world: &mut World, x: i32, y: i32, z: i32, block: Block) {
        let lit = is_lit(block);
        if lit && !world.has_neighbor_signal(x, y, z) {
            world.schedule_tick(x, y, z, block, 4, 0);
        } else if !lit && world.has_neighbor_signal(x, y, z) {
            world.set_block_with_metadata(x, y, z, Block::LitRedstoneLamp, 0, 2);
        }
    }

    pub fn tick(world: &mut World, x: i32, y: i32, z: i32, block: Block) {
        if is_lit(block) && !world.has_neighbor_signal(x, y, z) {
            world.set_block_with_metadata(x, y, z, Block::RedstoneLamp, 0, 2);
        }
    }
}
