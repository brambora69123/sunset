//! The world seam the mob AI reads through.
//!
//! The engine's world code lives in other crates; this trait is the contract they
//! implement so [`crate::mob`], [`crate::spawn`] and [`crate::pathfinder`] stay
//! engine-independent. It mirrors the `World` methods the 1.7.10 mob AI calls.

use crate::mob::{Mob, MobCategory, MobKind};
use crate::spawn::SpawnEntry;

/// A player's position, as the spawn cycle and mob AI see players.
#[derive(Clone, Copy, Debug)]
pub struct PlayerPos {
    pub id: u32,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// A snapshot of another mob, in world list order, for targeting and breeding.
#[derive(Clone, Copy, Debug)]
pub struct MobRef {
    pub id: u32,
    pub kind: MobKind,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub width: f32,
    pub height: f32,
    pub breeding_age: i32,
    pub in_love: bool,
    pub walk_target: Option<TargetRef>,
    pub love_causing_player: Option<u32>,
}

/// Target of a mob's attention: another mob or a player (`Entity` refs in 1.7.10).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetRef {
    Mob(u32),
    Player(u32),
}

/// The world contract the mob AI reads through.
pub trait MobWorld {
    /// `World.getBlock(x, y, z)` — the 1.7.10 block id.
    fn get_block(&self, x: i32, y: i32, z: i32) -> u16;

    /// `World.getBrightness(x, y, z)` — 0..1, pathfinding favor reads it.
    fn brightness(&self, x: i32, y: i32, z: i32) -> f32;

    /// `World.getActualLight(x, y, z)` — the combined light the animal spawn check reads.
    fn actual_light(&self, x: i32, y: i32, z: i32) -> i32;

    /// `World.hasSkyAccess(x, y, z)`.
    fn has_sky_access(&self, x: i32, y: i32, z: i32) -> bool;

    /// `World.getSurfaceHeight(x, z)` — the y the passive population spawns at.
    fn surface_height(&self, x: i32, z: i32) -> i32;

    /// `WorldChunk.getHighestSectionOffset()`, None when the chunk is absent.
    fn chunk_top(&self, chunk_x: i32, chunk_z: i32) -> Option<i32>;

    /// `World.getDimensionHeight()`.
    fn dimension_height(&self) -> i32;

    /// `World.getSpawnPoint()`.
    fn spawn_point(&self) -> (f64, f64, f64);

    /// `World.playerEntities`, in world order.
    fn players(&self) -> &[PlayerPos];

    /// `World.countEntities(type)` — mobs of a category's class.
    fn entity_count(&self, category: MobCategory) -> i32;

    /// Mobs of one kind in the world, in world list order (`getEntitiesOfType`).
    fn mobs_of_kind(&self, kind: MobKind) -> Vec<MobRef>;

    /// A mob's snapshot, None when absent (`Entity.isAlive` + fields).
    fn mob_ref(&self, id: u32) -> Option<MobRef>;

    /// Whether a player holds the kind's breeding item (`PlayerEntity.getItemInHand`).
    fn player_holds_breeding_item(&self, player_id: u32, kind: MobKind) -> bool;

    /// Whether a player is in creative mode.
    fn player_is_creative(&self, player_id: u32) -> bool;

    /// `World.canSee` — line of sight between two points.
    fn can_see(&self, from: (f64, f64, f64), to: (f64, f64, f64)) -> bool;

    /// `World.isUnobstructed` + `getCollisions(...).isEmpty()` at the mob's box.
    fn collision_free(&self, x: f64, y: f64, z: f64, width: f32, height: f32) -> bool;

    /// `World.containsLiquid(shape)` at the mob's box.
    fn contains_liquid(&self, x: f64, y: f64, z: f64, width: f32, height: f32) -> bool;

    /// `World.getGameRules().getBoolean(name)`.
    fn game_rule(&self, name: &str) -> bool;

    /// The biome's spawn entries for the category at (x, z)
    /// (`ChunkSource.getSpawnEntries`); the spawner runs the weighted pick.
    fn spawn_entries(&self, category: MobCategory, x: i32, y: i32, z: i32) -> Vec<SpawnEntry>;

    /// `World.addEntity` — assigns the mob's id and takes it into the world.
    fn add_mob(&mut self, mob: Mob) -> u32;

    /// `other.setWalkTarget(target)`.
    fn set_walk_target(&mut self, mob_id: u32, target: Option<TargetRef>);

    /// `other.inLoveTimer += amount`.
    fn add_love_ticks(&mut self, mob_id: u32, amount: i32);

    /// The post-breed reset on the mate: age 6000, love 0, breedingTicks 0, walk target None.
    fn finish_breeding(&mut self, mob_id: u32);

    /// `World.addParticle(name, x, y, z, vx, vy, vz)` — the heart draws.
    fn add_particle(
        &mut self,
        _name: &str,
        _x: f64,
        _y: f64,
        _z: f64,
        _vx: f64,
        _vy: f64,
        _vz: f64,
    ) {
    }

    /// `World.doEntityEvent(mob, data)` — heart particles and friends.
    fn entity_event(&mut self, _mob_id: u32, _data: u8) {}
}
