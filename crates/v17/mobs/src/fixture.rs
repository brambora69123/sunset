//! A fixture [`MobWorld`]: HashMap-backed blocks, players and mobs, for probes
//! and tests. Deterministic: no time, no threads, all state explicit.

use crate::blocks::{Material, block_by_id};
use crate::mob::{Mob, MobCategory, MobKind};
use crate::spawn::SpawnEntry;
use crate::world::{MobRef, MobWorld, PlayerPos, TargetRef};
use std::collections::HashMap;

/// The fixture world.
pub struct FixtureWorld {
    blocks: HashMap<(i32, i32, i32), u16>,
    players: Vec<PlayerPos>,
    mobs: Vec<Mob>,
    next_id: u32,
    light: HashMap<(i32, i32, i32), i32>,
    brightness: HashMap<(i32, i32, i32), f32>,
    spawn: (f64, f64, f64),
    rules: HashMap<String, bool>,
    entries: Vec<SpawnEntry>,
}

impl FixtureWorld {
    pub fn new() -> Self {
        FixtureWorld {
            blocks: HashMap::new(),
            players: Vec::new(),
            mobs: Vec::new(),
            next_id: 0,
            light: HashMap::new(),
            brightness: HashMap::new(),
            spawn: (0.0, 64.0, 0.0),
            rules: HashMap::new(),
            entries: Vec::new(),
        }
    }

    /// Sets a block by 1.7.10 id.
    pub fn set_block(&mut self, x: i32, y: i32, z: i32, id: u16) {
        self.blocks.insert((x, y, z), id);
    }

    /// Sets the combined light at a block.
    pub fn set_light(&mut self, x: i32, y: i32, z: i32, light: i32) {
        self.light.insert((x, y, z), light);
    }

    /// Sets the block brightness at a block.
    pub fn set_brightness(&mut self, x: i32, y: i32, z: i32, brightness: f32) {
        self.brightness.insert((x, y, z), brightness);
    }

    /// Adds a player.
    pub fn add_player(&mut self, x: f64, y: f64, z: f64) {
        self.players.push(PlayerPos {
            id: self.next_id,
            x,
            y,
            z,
        });
        self.next_id += 1;
    }

    /// Sets the biome spawn entries the seam returns.
    pub fn set_spawn_entries(&mut self, entries: Vec<SpawnEntry>) {
        self.entries = entries;
    }

    /// Sets a game rule.
    pub fn set_rule(&mut self, name: &str, value: bool) {
        self.rules.insert(name.to_string(), value);
    }

    /// The world's mobs.
    pub fn mobs(&self) -> &[Mob] {
        &self.mobs
    }

    /// A mob by id.
    pub fn mob_mut(&mut self, id: u32) -> Option<&mut Mob> {
        self.mobs.iter_mut().find(|mob| mob.id == id)
    }

    /// Blocks in the box that block movement, like `World.getCollisions`.
    fn solid_blocks_in_box(
        &self,
        x: f64,
        y: f64,
        z: f64,
        width: f32,
        height: f32,
    ) -> Vec<(i32, i32, i32)> {
        let min_x = (x - width as f64 / 2.0).floor() as i32;
        let max_x = (x + width as f64 / 2.0).floor() as i32;
        let min_y = y.floor() as i32;
        let max_y = (y + height as f64).floor() as i32;
        let min_z = (z - width as f64 / 2.0).floor() as i32;
        let max_z = (z + width as f64 / 2.0).floor() as i32;
        let mut out = Vec::new();
        for bx in min_x..=max_x {
            for by in min_y..=max_y {
                for bz in min_z..=max_z {
                    if block_by_id(self.get_block(bx, by, bz))
                        .get_material()
                        .blocks_movement()
                    {
                        out.push((bx, by, bz));
                    }
                }
            }
        }
        out
    }
}

impl Default for FixtureWorld {
    fn default() -> Self {
        Self::new()
    }
}

impl MobWorld for FixtureWorld {
    fn get_block(&self, x: i32, y: i32, z: i32) -> u16 {
        *self.blocks.get(&(x, y, z)).unwrap_or(&0)
    }

    fn brightness(&self, x: i32, y: i32, z: i32) -> f32 {
        *self.brightness.get(&(x, y, z)).unwrap_or(&0.5)
    }

    fn actual_light(&self, x: i32, y: i32, z: i32) -> i32 {
        *self.light.get(&(x, y, z)).unwrap_or(&15)
    }

    fn has_sky_access(&self, _x: i32, _y: i32, _z: i32) -> bool {
        true
    }

    fn surface_height(&self, x: i32, z: i32) -> i32 {
        // first free y above the highest solid block, scanning down from 256
        for y in (0..256).rev() {
            if block_by_id(self.get_block(x, y, z))
                .get_material()
                .blocks_movement()
            {
                return y + 1;
            }
        }
        1
    }

    fn chunk_top(&self, _chunk_x: i32, _chunk_z: i32) -> Option<i32> {
        None
    }

    fn dimension_height(&self) -> i32 {
        256
    }

    fn spawn_point(&self) -> (f64, f64, f64) {
        self.spawn
    }

    fn players(&self) -> &[PlayerPos] {
        &self.players
    }

    fn entity_count(&self, category: MobCategory) -> i32 {
        self.mobs
            .iter()
            .filter(|mob| mob.kind.category() == category)
            .count() as i32
    }

    fn mobs_of_kind(&self, kind: MobKind) -> Vec<MobRef> {
        self.mobs
            .iter()
            .filter(|mob| mob.kind == kind)
            .map(|mob| mob.ref_snapshot())
            .collect()
    }

    fn mob_ref(&self, id: u32) -> Option<MobRef> {
        self.mobs
            .iter()
            .find(|mob| mob.id == id)
            .map(|mob| mob.ref_snapshot())
    }

    fn player_holds_breeding_item(&self, player_id: u32, kind: MobKind) -> bool {
        // the fixture hands every player the kind's breeding item
        self.players.iter().any(|p| p.id == player_id) && !matches!(kind, MobKind::Zombie)
    }

    fn player_is_creative(&self, _player_id: u32) -> bool {
        false
    }

    fn can_see(&self, _from: (f64, f64, f64), _to: (f64, f64, f64)) -> bool {
        true
    }

    fn collision_free(&self, x: f64, y: f64, z: f64, width: f32, height: f32) -> bool {
        self.solid_blocks_in_box(x, y, z, width, height).is_empty()
    }

    fn contains_liquid(&self, x: f64, y: f64, z: f64, width: f32, height: f32) -> bool {
        let min_x = (x - width as f64 / 2.0).floor() as i32;
        let max_x = (x + width as f64 / 2.0).floor() as i32;
        let min_y = y.floor() as i32;
        let max_y = (y + height as f64).floor() as i32;
        let min_z = (z - width as f64 / 2.0).floor() as i32;
        let max_z = (z + width as f64 / 2.0).floor() as i32;
        for bx in min_x..=max_x {
            for by in min_y..=max_y {
                for bz in min_z..=max_z {
                    let material = block_by_id(self.get_block(bx, by, bz)).get_material();
                    if material == Material::Water || material == Material::Lava {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn game_rule(&self, name: &str) -> bool {
        *self.rules.get(name).unwrap_or(&true)
    }

    fn spawn_entries(&self, _category: MobCategory, _x: i32, _y: i32, _z: i32) -> Vec<SpawnEntry> {
        self.entries.clone()
    }

    fn add_mob(&mut self, mut mob: Mob) -> u32 {
        mob.id = self.next_id;
        self.next_id += 1;
        let id = mob.id;
        self.mobs.push(mob);
        id
    }

    fn set_walk_target(&mut self, mob_id: u32, target: Option<TargetRef>) {
        if let Some(mob) = self.mob_mut(mob_id) {
            mob.walk_target = target;
        }
    }

    fn add_love_ticks(&mut self, mob_id: u32, amount: i32) {
        if let Some(mob) = self.mob_mut(mob_id) {
            mob.in_love_timer += amount;
        }
    }

    fn finish_breeding(&mut self, mob_id: u32) {
        if let Some(mob) = self.mob_mut(mob_id) {
            crate::breeding::set_breeding_age(mob, 6000);
            mob.in_love_timer = 0;
            mob.breeding_ticks = 0;
            mob.walk_target = None;
        }
    }

    fn entity_event(&mut self, _mob_id: u32, _data: u8) {}
}

/// A mob at a position, for fixtures.
pub fn mob_at(x: f64, y: f64, z: f64) -> Mob {
    Mob::new(MobKind::Cow, x, y, z)
}
