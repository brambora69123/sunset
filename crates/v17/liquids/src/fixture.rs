//! A plain block grid behind [`crate::LiquidWorld`], for tests and the parity
//! probe. It is a faithful single-chunk server world: storage only, vanilla's
//! scheduled-tick dedupe and order, and records of the presentation and write
//! calls the flow rules make.

use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap, HashSet};

use sunset_1_7_core::LegacyRandom;

use crate::block::{AIR, Block};
use crate::world::{LiquidDimension, LiquidWorld};

/// One cell: the block and its metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub block: Block,
    pub metadata: i32,
}

/// One `set_block_with_metadata` / `set_block_metadata` call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Write {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub block: Block,
    pub metadata: i32,
    pub flags: i32,
}

/// One `play_sound`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sound {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub name: &'static str,
    pub volume: f32,
    pub pitch: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Tick {
    x: i32,
    y: i32,
    z: i32,
    block: Block,
    time: i64,
    priority: i32,
    id: u64,
}

impl Ord for Tick {
    fn cmp(&self, other: &Self) -> Ordering {
        self.time
            .cmp(&other.time)
            .then(self.priority.cmp(&other.priority))
            .then(self.id.cmp(&other.id))
    }
}

impl PartialOrd for Tick {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A single-chunk server world: `isMultiplayer` false, every position loaded,
/// and random chunk ticks left to the caller (as in the redstone port).
pub struct TestWorld {
    cells: HashMap<(i32, i32, i32), Cell>,
    /// `WorldData.getTime`.
    pub time: i64,
    rng: LegacyRandom,
    dimension: LiquidDimension,
    next_tick_id: u64,
    scheduled: BTreeSet<Tick>,
    scheduled_keys: HashSet<(i32, i32, i32, u16)>,
    /// Every block write, in call order.
    pub writes: Vec<Write>,
    /// Every fizz sound.
    pub sounds: Vec<Sound>,
    /// How many `largesmoke` particles the fizz asked for.
    pub fizz_particles: usize,
    /// Every `drop_block` call, in order.
    pub drops: Vec<(i32, i32, i32, Block, i32)>,
}

impl TestWorld {
    pub fn new(seed: i64) -> Self {
        TestWorld::with_dimension(seed, LiquidDimension::OVERWORLD)
    }

    pub fn with_dimension(seed: i64, dimension: LiquidDimension) -> Self {
        TestWorld {
            cells: HashMap::new(),
            time: 0,
            rng: LegacyRandom::new(seed),
            dimension,
            next_tick_id: 0,
            scheduled: BTreeSet::new(),
            scheduled_keys: HashSet::new(),
            writes: Vec::new(),
            sounds: Vec::new(),
            fizz_particles: 0,
            drops: Vec::new(),
        }
    }

    /// Install a block without the world callbacks, as a chunk generator's
    /// block array does.
    pub fn set_direct(&mut self, x: i32, y: i32, z: i32, block: Block, metadata: i32) {
        if block == AIR {
            self.cells.remove(&(x, y, z));
        } else {
            self.cells.insert((x, y, z), Cell { block, metadata });
        }
    }

    /// Fill an inclusive box directly, generator-style.
    pub fn fill(
        &mut self,
        x0: i32,
        y0: i32,
        z0: i32,
        x1: i32,
        y1: i32,
        z1: i32,
        block: Block,
        metadata: i32,
    ) {
        for x in x0..=x1 {
            for y in y0..=y1 {
                for z in z0..=z1 {
                    self.set_direct(x, y, z, block, metadata);
                }
            }
        }
    }

    /// `World.setBlockWithMetadata` as a player or generator places a block:
    /// the liquid-visible callbacks included, with the world's random.
    pub fn place(&mut self, x: i32, y: i32, z: i32, block: Block, metadata: i32, flags: i32) {
        let mut random = self.rng.clone();
        crate::place_block(self, x, y, z, block, metadata, flags, &mut random);
        self.rng = random;
    }

    /// `Block.neighborChanged` on a block, with the world's random.
    pub fn kick(&mut self, x: i32, y: i32, z: i32) {
        crate::neighbor_changed(self, x, y, z);
    }

    pub fn scheduled_tick_count(&self) -> usize {
        self.scheduled.len()
    }

    /// `ServerWorld.doScheduledTicks(false)`: collect the due ticks in order,
    /// then run them.
    pub fn run_scheduled(&mut self) {
        let mut due = Vec::new();
        for tick in &self.scheduled {
            if tick.time > self.time || due.len() >= 1000 {
                break;
            }
            due.push(*tick);
        }
        for tick in due {
            self.scheduled.remove(&tick);
            self.scheduled_keys
                .remove(&(tick.x, tick.y, tick.z, tick.block.id));
            let block = self.get_block(tick.x, tick.y, tick.z);
            if block.material != crate::block::Material::Air && block.id == tick.block.id {
                let mut random = self.rng.clone();
                crate::tick(self, tick.x, tick.y, tick.z, &mut random);
                self.rng = random;
            }
        }
    }

    /// `WorldData.setTime` + `doScheduledTicks(false)`, one server tick.
    pub fn tick_once(&mut self) {
        self.time += 1;
        self.run_scheduled();
    }

    pub fn tick_n(&mut self, ticks: usize) {
        for _ in 0..ticks {
            self.tick_once();
        }
    }

    /// The world's random state, for a probe's exactness line.
    pub fn random_state(&self) -> u64 {
        self.rng.state()
    }
}

impl LiquidWorld for TestWorld {
    fn get_block(&self, x: i32, y: i32, z: i32) -> Block {
        if !(0..256).contains(&y) {
            return AIR;
        }
        self.cells
            .get(&(x, y, z))
            .map(|cell| cell.block)
            .unwrap_or(AIR)
    }

    fn get_block_metadata(&self, x: i32, y: i32, z: i32) -> i32 {
        if !(0..256).contains(&y) {
            return 0;
        }
        self.cells.get(&(x, y, z)).map_or(0, |cell| cell.metadata)
    }

    fn set_block_with_metadata(
        &mut self,
        x: i32,
        y: i32,
        z: i32,
        block: Block,
        metadata: i32,
        flags: i32,
    ) -> bool {
        if !(-30_000_000..30_000_000).contains(&x)
            || !(0..256).contains(&y)
            || !(-30_000_000..30_000_000).contains(&z)
        {
            return false;
        }
        if self.get_block(x, y, z) == block && self.get_block_metadata(x, y, z) == metadata {
            return false;
        }
        self.writes.push(Write {
            x,
            y,
            z,
            block,
            metadata,
            flags,
        });
        if block == AIR {
            self.cells.remove(&(x, y, z));
        } else {
            self.cells.insert((x, y, z), Cell { block, metadata });
        }
        true
    }

    fn set_block_metadata(&mut self, x: i32, y: i32, z: i32, metadata: i32, flags: i32) -> bool {
        if !(0..256).contains(&y) {
            return false;
        }
        let block = match self.cells.get_mut(&(x, y, z)) {
            Some(cell) => {
                if cell.metadata == metadata {
                    return false;
                }
                cell.metadata = metadata;
                cell.block
            }
            None => return false,
        };
        self.writes.push(Write {
            x,
            y,
            z,
            block,
            metadata,
            flags,
        });
        true
    }

    fn schedule_tick(&mut self, x: i32, y: i32, z: i32, block: Block, delay: i32) {
        if !self.scheduled_keys.insert((x, y, z, block.id)) {
            return;
        }
        let id = self.next_tick_id;
        self.next_tick_id += 1;
        self.scheduled.insert(Tick {
            x,
            y,
            z,
            block,
            time: self.time + i64::from(delay),
            priority: 0,
            id,
        });
    }

    fn dimension(&self) -> LiquidDimension {
        self.dimension
    }

    fn random(&mut self) -> &mut LegacyRandom {
        &mut self.rng
    }

    fn play_sound(&mut self, x: f32, y: f32, z: f32, name: &'static str, volume: f32, pitch: f32) {
        self.sounds.push(Sound {
            x,
            y,
            z,
            name,
            volume,
            pitch,
        });
    }

    fn fizz_particles(&mut self, _x: i32, _y: i32, _z: i32) {
        self.fizz_particles += 1;
    }

    fn drop_block(&mut self, x: i32, y: i32, z: i32, block: Block, metadata: i32) {
        self.drops.push((x, y, z, block, metadata));
    }
}
