//! `MobEntity` and the hierarchy the mob AI hangs off, reduced to one struct with
//! capability flags (`entity/living/mob/MobEntity.java` and friends).

use crate::controls::{JumpControl, LookControl, MovementControl, clamp_and_wrap_angle};
use crate::goal::GoalSelector;
use crate::math;
use crate::navigation::PathNavigation;
use crate::pathfinder::PathFinder;
use crate::world::{MobRef, MobWorld, TargetRef};
use sunset_1_7_core::{LegacyRandom, RandomSource};

/// `MobEntity.getNearestPlayer(this, -1.0)` squared — nearest player, unlimited range.
pub fn nearest_player_sq(world: &dyn MobWorld, x: f64, y: f64, z: f64) -> Option<f64> {
    world
        .players()
        .iter()
        .map(|player| {
            let dx = player.x - x;
            let dy = player.y - y;
            let dz = player.z - z;
            dx * dx + dy * dy + dz * dz
        })
        .min_by(|a, b| a.partial_cmp(b).unwrap())
}

/// `World.getNearestPlayer(x, y, z, range)` — the nearest player's id, if any.
pub fn nearest_player_within(
    world: &dyn MobWorld,
    x: f64,
    y: f64,
    z: f64,
    range: f64,
) -> Option<u32> {
    let range_sq = range * range;
    let mut best: Option<(f64, u32)> = None;
    for player in world.players() {
        let dx = player.x - x;
        let dy = player.y - y;
        let dz = player.z - z;
        let dist_sq = dx * dx + dy * dy + dz * dz;
        if dist_sq < range_sq && best.is_none_or(|(d, _)| dist_sq < d) {
            best = Some((dist_sq, player.id));
        }
    }
    best.map(|(_, id)| id)
}

/// An entity target's current position (`walkTarget` resolution).
pub fn resolve_target(world: &dyn MobWorld, target: TargetRef) -> Option<(f64, f64, f64)> {
    match target {
        TargetRef::Mob(id) => world.mob_ref(id).map(|m| (m.x, m.y, m.z)),
        TargetRef::Player(id) => world
            .players()
            .iter()
            .find(|p| p.id == id)
            .map(|p| (p.x, p.y, p.z)),
    }
}

/// `Entity.isAlive()` for a target.
pub fn target_removed(world: &dyn MobWorld, target: TargetRef) -> bool {
    resolve_target(world, target).is_none()
}

/// `MobCategory`: spawn caps, spawnable material, peaceful/rare flags.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MobCategory {
    Monster,
    Creature,
    Ambient,
    WaterCreature,
}

impl MobCategory {
    /// `getCap()`.
    pub fn cap(self) -> i32 {
        match self {
            MobCategory::Monster => 70,
            MobCategory::Creature => 10,
            MobCategory::Ambient => 15,
            MobCategory::WaterCreature => 5,
        }
    }

    /// `getSpawnableMaterial()`.
    pub fn spawnable_material(self) -> crate::blocks::Material {
        match self {
            MobCategory::Monster | MobCategory::Creature | MobCategory::Ambient => {
                crate::blocks::Material::Air
            }
            MobCategory::WaterCreature => crate::blocks::Material::Water,
        }
    }

    /// `isPeaceful()`.
    pub fn is_peaceful(self) -> bool {
        matches!(
            self,
            MobCategory::Creature | MobCategory::Ambient | MobCategory::WaterCreature
        )
    }

    /// `isRare()`.
    pub fn is_rare(self) -> bool {
        matches!(self, MobCategory::Creature)
    }

    /// `MobCategory.values()` in declaration order.
    pub const VALUES: [MobCategory; 4] = [
        MobCategory::Monster,
        MobCategory::Creature,
        MobCategory::Ambient,
        MobCategory::WaterCreature,
    ];
}

/// The concrete 1.7.10 mobs this crate models. Each carries its AI and spawn table data.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MobKind {
    Cow,
    Pig,
    Sheep,
    Chicken,
    Zombie,
    Skeleton,
    Creeper,
    Spider,
    Slime,
    Bat,
    Squid,
}

impl MobKind {
    /// `MobCategory` the kind belongs to (`instanceof` in vanilla).
    pub fn category(self) -> MobCategory {
        match self {
            MobKind::Cow | MobKind::Pig | MobKind::Sheep | MobKind::Chicken => {
                MobCategory::Creature
            }
            MobKind::Zombie | MobKind::Skeleton | MobKind::Creeper | MobKind::Spider => {
                MobCategory::Monster
            }
            MobKind::Slime => MobCategory::Monster,
            MobKind::Bat => MobCategory::Ambient,
            MobKind::Squid => MobCategory::WaterCreature,
        }
    }

    /// `getLimitPerChunk()` — 4 for most, slimes take their size-based rule.
    pub fn limit_per_chunk(self) -> i32 {
        match self {
            MobKind::Slime => 4,
            _ => 4,
        }
    }

    /// `canDespawn()` — animals and bats persist.
    pub fn can_despawn(self) -> bool {
        !matches!(
            self,
            MobKind::Cow | MobKind::Pig | MobKind::Sheep | MobKind::Chicken | MobKind::Squid
        )
    }

    /// `EntityList.func_151526_b` style: the string key spawn tables and NBT use.
    pub fn entity_key(self) -> &'static str {
        match self {
            MobKind::Cow => "Cow",
            MobKind::Pig => "Pig",
            MobKind::Sheep => "Sheep",
            MobKind::Chicken => "Chicken",
            MobKind::Zombie => "Zombie",
            MobKind::Skeleton => "Skeleton",
            MobKind::Creeper => "Creeper",
            MobKind::Spider => "Spider",
            MobKind::Slime => "Slime",
            MobKind::Bat => "Bat",
            MobKind::Squid => "Squid",
        }
    }

    /// The kind's breeding item is wheat for everything here except chicken→seeds,
    /// pig→carrot (1.7.10: pig is carrot? No: 1.7.10 pig breeding item is wheat-family
    /// carrot is 1.9+; 1.7.10 pigs breed with wheat too? Pigs breed with wheat in 1.7.10,
    /// carrots only tame/breed pigs in 1.9. Correct 1.7.10 table: cow/sheep/pig → wheat,
    /// chicken → seeds.)
    pub fn is_breeding_item_for(self, item_id: u16) -> bool {
        match self {
            MobKind::Cow | MobKind::Pig | MobKind::Sheep => item_id == 296, // wheat
            MobKind::Chicken => item_id == 295,                             // seeds
            _ => false,
        }
    }

    /// Hitbox: `(width, height)` from the 1.7.10 entity constructors.
    pub fn size(self) -> (f32, f32) {
        match self {
            MobKind::Cow => (0.9, 1.3),
            MobKind::Pig => (0.9, 0.9),
            MobKind::Sheep => (0.9, 1.3),
            MobKind::Chicken => (0.4, 0.7),
            MobKind::Zombie | MobKind::Skeleton => (0.6, 1.8),
            MobKind::Creeper => (0.6, 1.8),
            MobKind::Spider => (1.4, 0.9),
            MobKind::Slime => (1.0, 1.0),
            MobKind::Bat => (0.5, 0.9),
            MobKind::Squid => (0.95, 0.95),
        }
    }

    /// Base `MOVEMENT_SPEED` attribute value.
    pub fn movement_speed(self) -> f64 {
        match self {
            MobKind::Cow | MobKind::Sheep => 0.2,
            MobKind::Pig => 0.25,
            MobKind::Chicken => 0.25,
            MobKind::Zombie => 0.23,
            MobKind::Skeleton => 0.25,
            MobKind::Creeper => 0.25,
            MobKind::Spider => 0.3,
            MobKind::Slime => 0.5,
            MobKind::Bat => 0.1,
            MobKind::Squid => 0.7,
        }
    }

    /// Base `FOLLOW_RANGE` attribute value.
    pub fn follow_range(self) -> f64 {
        16.0
    }
}

/// The pathfinder/navigation's view of a mob: a snapshot value, so callers can borrow
/// one subsystem and the shape without overlapping entity borrows.
#[derive(Clone, Copy, Debug)]
pub struct MobShape {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub width: f32,
    pub height: f32,
    pub in_water: bool,
    pub in_lava: bool,
    pub on_ground: bool,
    pub riding_zombie_on_chicken: bool,
    pub safe_fall_distance: i32,
}

impl MobShape {
    /// `Entity.shape.minX`.
    pub fn min_x(&self) -> f64 {
        self.x - self.width as f64 / 2.0
    }

    /// `Entity.shape.minY`.
    pub fn min_y(&self) -> f64 {
        self.y
    }

    /// `Entity.shape.minZ`.
    pub fn min_z(&self) -> f64 {
        self.z - self.width as f64 / 2.0
    }
}

/// The mob entity: `MobEntity` + `PathFinderMobEntity` + `PassiveEntity`/`AnimalEntity`
/// state in one struct, since the port models behavior, not the Java class tree.
pub struct Mob {
    pub id: u32,
    pub kind: MobKind,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub width: f32,
    pub height: f32,
    /// `yaw`, the body rotation the movement control steers.
    pub yaw: f32,
    /// `headYaw`.
    pub head_yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
    /// `isInWater()`.
    pub in_water: bool,
    /// `isInLava()`.
    pub in_lava: bool,
    /// `isRiding() && this instanceof ZombieEntity && vehicle instanceof ChickenEntity`.
    pub riding_zombie_on_chicken: bool,
    /// `removed` — the entity is gone from the world.
    pub removed: bool,
    /// `collidingHorizontally`.
    pub colliding_horizontally: bool,
    /// The four AI speeds (`LivingEntity` movement fields).
    pub forward_speed: f32,
    pub sideways_speed: f32,
    /// `jumping`.
    pub jumping: bool,
    /// `attackTarget`.
    pub attack_target: Option<u32>,
    /// `PathFinderMobEntity.walkTarget`.
    pub walk_target: Option<crate::world::TargetRef>,
    /// `PathFinderMobEntity.path` — the legacy AI's own path.
    pub path: Option<crate::path::Path>,
    /// `stoppedMoving`.
    pub stopped_moving: bool,
    /// `fastMovementCooldown` — the fleeing-speed window.
    pub fast_movement_cooldown: i32,
    /// `farFromPlayerTicks` — the despawn timer.
    pub far_from_player_ticks: i32,
    /// `PersistentEntity`: never despawns (named, leashed...).
    pub persistent: bool,
    // --- PassiveEntity ---
    /// `getBreedingAge()`: negative baby, positive cooldown, zero adult.
    pub breeding_age: i32,
    // --- AnimalEntity ---
    /// `inLoveTimer`.
    pub in_love_timer: i32,
    /// `breedingTicks` — shared breed-progress counter.
    pub breeding_ticks: i32,
    /// `loveCausingPlayer`.
    pub love_causing_player: Option<u32>,
    /// `lookTarget` + `lookTimer` — the base AI's stare.
    pub look_target: Option<TargetRef>,
    pub look_timer: i32,
    /// `rotationSpeed`.
    pub rotation_speed: f32,
    /// `standardPitch`.
    pub standard_pitch: f32,
    /// The entity's own RNG, seeded at spawn (`this.random`; 1.7.10 uses an
    /// unseeded `new Random()`, the fixture passes the world seed).
    pub rng: LegacyRandom,
    /// `SlimeEntity.size` (1..4); 1 for everything else.
    pub slime_size: i32,
    /// The goal selector (`goalSelector`).
    pub goal_selector: GoalSelector,
    /// The target selector (`targetSelector`).
    pub target_selector: GoalSelector,
    /// `movementControl`.
    pub movement_control: MovementControl,
    /// `lookControl`.
    pub look_control: LookControl,
    /// `jumpControl`.
    pub jump_control: JumpControl,
    /// `entityNavigation`.
    pub navigation: PathNavigation,
    /// `FOLLOW_RANGE` modifier stack folded into one multiplier
    /// (`initialize`'s random spawn bonus, attribute operation 1).
    pub follow_range_multiplier: f64,
    /// `getSafeFallDistance()` override: None → compute from health like `MobEntity`.
    pub safe_fall_distance_override: Option<i32>,
    /// `getHealth()`.
    pub health: f32,
    /// `getMaxHealth()`.
    pub max_health: f32,
}

impl Mob {
    /// A mob without a world-assigned id yet (id 0), at full size.
    pub fn new(kind: MobKind, x: f64, y: f64, z: f64) -> Self {
        Mob::seeded(kind, 0, x, y, z)
    }

    /// A mob with the entity's own RNG seeded from the world seed at spawn.
    pub fn seeded(kind: MobKind, seed: i64, x: f64, y: f64, z: f64) -> Self {
        let (width, height) = kind.size();
        let mut mob = Mob::with_id(0, kind, x, y, z, width, height);
        mob.rng = LegacyRandom::new(seed);
        mob.navigation.set_follow_range(kind.follow_range());
        mob
    }

    /// A mob with a fixed id, hitbox and sizes — used by fixtures and tests.
    #[allow(clippy::too_many_arguments)]
    pub fn with_id(
        id: u32,
        kind: MobKind,
        x: f64,
        y: f64,
        z: f64,
        width: f32,
        height: f32,
    ) -> Self {
        Mob {
            id,
            kind,
            x,
            y,
            z,
            width,
            height,
            yaw: 0.0,
            head_yaw: 0.0,
            pitch: 0.0,
            on_ground: false,
            in_water: false,
            in_lava: false,
            riding_zombie_on_chicken: false,
            removed: false,
            forward_speed: 0.0,
            sideways_speed: 0.0,
            jumping: false,
            attack_target: None,
            walk_target: None,
            path: None,
            colliding_horizontally: false,
            stopped_moving: false,
            fast_movement_cooldown: 0,
            far_from_player_ticks: 0,
            persistent: false,
            breeding_age: 0,
            in_love_timer: 0,
            breeding_ticks: 0,
            love_causing_player: None,
            look_target: None,
            look_timer: 0,
            rotation_speed: 0.0,
            standard_pitch: 0.0,
            rng: LegacyRandom::new(0),
            slime_size: 1,
            goal_selector: GoalSelector::new(),
            target_selector: GoalSelector::new(),
            movement_control: MovementControl::new(),
            look_control: LookControl::new(),
            jump_control: JumpControl::new(),
            navigation: PathNavigation::new(kind.follow_range()),
            follow_range_multiplier: 1.0,
            safe_fall_distance_override: None,
            health: 10.0,
            max_health: 10.0,
        }
    }

    /// `Entity.shape.minX`.
    pub fn min_x(&self) -> f64 {
        self.x - self.width as f64 / 2.0
    }

    /// `Entity.shape.minY`.
    pub fn min_y(&self) -> f64 {
        self.y
    }

    /// `Entity.shape.minZ`.
    pub fn min_z(&self) -> f64 {
        self.z - self.width as f64 / 2.0
    }

    /// `MobEntity.isBaby()` (`PassiveEntity`): breeding age below zero.
    pub fn is_baby(&self) -> bool {
        self.breeding_age < 0
    }

    /// `AnimalEntity.isInLove()`.
    pub fn is_in_love(&self) -> bool {
        self.in_love_timer > 0
    }

    /// `MobEntity.canDespawn()`.
    pub fn can_despawn(&self) -> bool {
        !self.persistent && self.kind.can_despawn()
    }

    /// `MobEntity.getSafeFallDistance()`.
    pub fn safe_fall_distance(&self) -> i32 {
        if let Some(override_value) = self.safe_fall_distance_override {
            return override_value;
        }
        if self.attack_target.is_none() {
            return 3;
        }
        let mut v = (self.health - self.max_health * 0.33) as i32;
        // world.difficulty.getId(): normal = 2
        v -= 4;
        if v < 0 {
            v = 0;
        }
        v + 3
    }

    /// `MobEntity.getFollowRange()` — base times the spawn-bonus modifier.
    pub fn follow_range(&self) -> f32 {
        (self.kind.follow_range() * self.follow_range_multiplier) as f32
    }

    /// `MobEntity.initialize(entityData)` — the random spawn bonus on FOLLOW_RANGE
    /// (attribute operation 1: base times one plus the amount).
    pub fn initialize(&mut self) {
        let amount = self.rng.next_gaussian() * 0.05;
        self.follow_range_multiplier = 1.0 + amount;
    }

    /// The world-seam snapshot of this mob.
    pub fn ref_snapshot(&self) -> MobRef {
        MobRef {
            id: self.id,
            kind: self.kind,
            x: self.x,
            y: self.y,
            z: self.z,
            width: self.width,
            height: self.height,
            breeding_age: self.breeding_age,
            in_love: self.is_in_love(),
            walk_target: self.walk_target,
            love_causing_player: self.love_causing_player,
        }
    }

    /// `Entity.getEyeHeight()` — 0.85 * height like `LivingEntity` for these mobs.
    pub fn eye_height(&self) -> f64 {
        self.height as f64 * 0.85
    }

    /// `Entity.squaredDistanceTo(Entity)`.
    pub fn squared_distance_to(&self, other_x: f64, other_y: f64, other_z: f64) -> f64 {
        let dx = other_x - self.x;
        let dy = other_y - self.y;
        let dz = other_z - self.z;
        dx * dx + dy * dy + dz * dz
    }

    /// `MobEntity.getLimitPerChunk()`.
    pub fn limit_per_chunk(&self) -> i32 {
        self.kind.limit_per_chunk()
    }

    /// `setPositionAndAngles(x, y, z, yaw, pitch)`.
    pub fn set_position_and_angles(&mut self, x: f64, y: f64, z: f64, yaw: f32, pitch: f32) {
        self.x = x;
        self.y = y;
        self.z = z;
        self.yaw = yaw;
        self.head_yaw = yaw;
        self.pitch = pitch;
    }

    /// The pathfinder/navigation's view of the mob: a snapshot, so callers can borrow
    /// one subsystem and the shape without overlapping entity borrows.
    pub fn shape(&self) -> MobShape {
        MobShape {
            x: self.x,
            y: self.y,
            z: self.z,
            width: self.width,
            height: self.height,
            in_water: self.in_water,
            in_lava: self.in_lava,
            on_ground: self.on_ground,
            riding_zombie_on_chicken: self.riding_zombie_on_chicken,
            safe_fall_distance: self.safe_fall_distance(),
        }
    }

    /// `MobEntity.canSpawn()`.
    pub fn can_spawn(&self, world: &dyn MobWorld) -> bool {
        world.collision_free(self.x, self.y, self.z, self.width, self.height)
            && !world.contains_liquid(self.x, self.y, self.z, self.width, self.height)
    }

    /// `PathFinderMobEntity.canSpawn()`: base plus a non-negative pathfinding favor.
    pub fn pathfinder_can_spawn(&self, world: &dyn MobWorld) -> bool {
        let favor = self.pathfinding_favor(
            world,
            math::floor(self.x),
            math::floor(self.min_y()),
            math::floor(self.z),
        );
        self.can_spawn(world) && favor >= 0.0
    }

    /// `getPathfindingFavor(x, y, z)` — animals favor grass like `AnimalEntity`.
    pub fn pathfinding_favor(&self, world: &dyn MobWorld, x: i32, y: i32, z: i32) -> f32 {
        if self.kind.category() == MobCategory::Creature {
            if world.get_block(x, y - 1, z) == 2 {
                10.0
            } else {
                world.brightness(x, y, z) - 0.5
            }
        } else {
            0.0
        }
    }

    /// `MobEntity.checkDespawn` — the nearest-player-backed despawn timer.
    pub fn check_despawn(&mut self, world: &dyn MobWorld) {
        if self.persistent {
            self.far_from_player_ticks = 0;
            return;
        }
        let Some(nearest) = nearest_player_sq(world, self.x, self.y, self.z) else {
            return;
        };
        if self.can_despawn() && nearest > 16384.0 {
            self.removed = true;
        }

        if self.far_from_player_ticks > 600
            && self.rng.next_int_bound(800).expect("bound > 0") == 0
            && nearest > 1024.0
            && self.can_despawn()
        {
            self.removed = true;
        } else if nearest < 1024.0 {
            self.far_from_player_ticks = 0;
        }
    }

    /// `MobEntity.serverTickAi` — the AI-enabled per-tick chain.
    pub fn server_tick_ai(&mut self, world: &mut dyn MobWorld) {
        self.far_from_player_ticks += 1;
        self.check_despawn(world);
        let mut target_selector = std::mem::take(&mut self.target_selector);
        target_selector.tick(self, &mut *world);
        self.target_selector = target_selector;
        let mut goal_selector = std::mem::take(&mut self.goal_selector);
        goal_selector.tick(self, &mut *world);
        self.goal_selector = goal_selector;
        let mut navigation = std::mem::take(&mut self.navigation);
        navigation.tick(self, &*world);
        self.navigation = navigation;
        self.mob_ai_tick(world);
        let mut movement_control = std::mem::take(&mut self.movement_control);
        movement_control.tick(self);
        self.movement_control = movement_control;
        let mut look_control = std::mem::take(&mut self.look_control);
        look_control.tick(self);
        self.look_control = look_control;
        let mut jump_control = std::mem::take(&mut self.jump_control);
        jump_control.tick(self);
        self.jump_control = jump_control;
    }

    /// `mobAiTick` — the goal system's mob tick; animals clear love at nonzero age.
    pub fn mob_ai_tick(&mut self, _world: &mut dyn MobWorld) {
        if self.kind.category() == MobCategory::Creature && self.breeding_age != 0 {
            self.in_love_timer = 0;
        }
    }

    /// `LivingEntity.mobTick` + `PassiveEntity.mobTick` + `AnimalEntity.mobTick` —
    /// the every-tick behavior chain. `rng` is the world RNG, shared like vanilla.
    pub fn mob_tick(&mut self, world: &mut dyn MobWorld, rng: &mut LegacyRandom) {
        if self.kind.category() == MobCategory::Creature && self.breeding_age != 0 {
            self.in_love_timer = 0;
        }

        // PassiveEntity.mobTick: the breeding age counter
        let age = self.breeding_age;
        if age < 0 {
            crate::breeding::set_breeding_age(self, age + 1);
        } else if age > 0 {
            crate::breeding::set_breeding_age(self, age - 1);
        }

        // AnimalEntity.mobTick: the love timer
        if self.in_love_timer > 0 {
            self.in_love_timer -= 1;
            if self.in_love_timer % 10 == 0 {
                let _var2 = rng.next_gaussian() * 0.02;
                let _var4 = rng.next_gaussian() * 0.02;
                let _var6 = rng.next_gaussian() * 0.02;
                world.entity_event(self.id, 18);
            }
        } else {
            self.breeding_ticks = 0;
        }
    }

    /// `MobEntity.aiTick` — the legacy-AI base: random looks and water jumps.
    pub fn ai_tick_base(&mut self, world: &mut dyn MobWorld) {
        self.sideways_speed = 0.0;
        self.forward_speed = 0.0;
        self.check_despawn(world);
        let range = 8.0f64;
        if self.rng.next_float() < 0.02 {
            if let Some(player) = nearest_player_within(world, self.x, self.y, self.z, range) {
                self.look_target = Some(TargetRef::Player(player));
                self.look_timer = 10 + self.rng.next_int_bound(20).expect("bound > 0");
            } else {
                self.rotation_speed = (self.rng.next_float() - 0.5) * 20.0;
            }
        }

        if let Some(look) = self.look_target {
            let resolved = resolve_target(world, look);
            if let Some((x, y, z)) = resolved {
                self.look_at(x, y, z, 10.0, 40.0);
                self.look_timer -= 1;
                let gone = self.look_timer <= 0
                    || target_removed(world, look)
                    || self.squared_distance_to(x, y, z) > range * range;
                if gone {
                    self.look_target = None;
                }
            } else {
                self.look_target = None;
            }
        } else {
            if self.rng.next_float() < 0.05 {
                self.rotation_speed = (self.rng.next_float() - 0.5) * 20.0;
            }

            self.yaw += self.rotation_speed;
            self.pitch = self.standard_pitch;
        }

        if self.in_water || self.in_lava {
            self.jumping = self.rng.next_float() < 0.8;
        }
    }

    /// `MobEntity.lookAt(target, maxYawChange, maxPitchChange)`.
    pub fn look_at(&mut self, x: f64, y: f64, z: f64, max_yaw: f32, max_pitch: f32) {
        let dx = x - self.x;
        let dz = z - self.z;
        let dy = y - (self.y + self.eye_height());
        let dist = math::sqrt_f64(dx * dx + dz * dz);
        let target_yaw = (math::atan2(dz, dx) * 180.0 / std::f64::consts::PI) as f32 - 90.0;
        let target_pitch = (-(math::atan2(dy, dist) * 180.0 / std::f64::consts::PI)) as f32;
        self.pitch = clamp_and_wrap_angle(self.pitch, target_pitch, max_pitch);
        self.yaw = clamp_and_wrap_angle(self.yaw, target_yaw, max_yaw);
    }

    /// `MobEntity.getLookPitchSpeed()`.
    pub fn get_look_pitch_speed(&self) -> f32 {
        40.0
    }

    /// `PathFinderMobEntity.aiTick` — the legacy walk-target AI animals use.
    pub fn ai_tick(&mut self, world: &mut dyn MobWorld) {
        // fastMovementCooldown window: the attribute modifier is player-side work
        if self.fast_movement_cooldown > 0 {
            self.fast_movement_cooldown -= 1;
        }

        self.stopped_moving = false;
        if self.walk_target.is_none() {
            self.walk_target = self.find_target(world);
            if let Some(target) = self.walk_target
                && let Some((tx, ty, tz)) = resolve_target(world, target)
            {
                self.set_path_to(world, tx, ty, tz);
            }
        } else if target_removed(world, self.walk_target.unwrap()) {
            self.walk_target = None;
        } else {
            let (tx, ty, tz) = resolve_target(world, self.walk_target.unwrap()).unwrap();
            let distance =
                ((tx - self.x).powi(2) + (ty - self.y).powi(2) + (tz - self.z).powi(2)).sqrt();
            let eye = (self.x, self.y + self.eye_height(), self.z);
            let seen_eye = (
                tx,
                ty + eye_height_of(world, self.walk_target.unwrap(), tz),
                tz,
            );
            if world.can_see(eye, seen_eye) {
                self.target_in_sight(world, self.walk_target.unwrap(), distance);
            }
        }

        // creative players are never followed
        if let Some(TargetRef::Player(pid)) = self.walk_target
            && world.player_is_creative(pid)
        {
            self.walk_target = None;
        }

        let ground_y = math::floor(self.min_y() + 0.5) as f64;
        let in_water = self.in_water;
        let in_lava = self.in_lava;
        self.pitch = 0.0;
        let follow = self.path.is_some() && self.rng.next_int_bound(100).expect("bound > 0") != 0;
        if follow {
            // advance past nodes closer than 2 * width
            let reach = (self.width * 2.0) as f64;
            while let Some(path) = self.path.as_mut() {
                if path.is_done() {
                    self.path = None;
                    break;
                }
                let (px, _py, pz) = path.get_current_pos(self.width);
                let dx = px - self.x;
                // vanilla passes var5.y as the y argument: the dy term is always 0 here
                let dy = 0.0f64;
                let dz = pz - self.z;
                if dx * dx + dy * dy + dz * dz < reach * reach {
                    path.advance();
                } else {
                    break;
                }
            }
            self.jumping = false;
            if let Some(path) = self.path.as_ref()
                && !path.is_done()
            {
                let (px, py, pz) = path.get_current_pos(self.width);
                let dx = px - self.x;
                let dz = pz - self.z;
                let dy = py - ground_y;
                let angle = (math::atan2(dz, dx) * 180.0 / std::f64::consts::PI) as f32 - 90.0;
                let mut turn = math::wrap_degrees(angle - self.yaw);
                self.forward_speed = self.kind.movement_speed() as f32;
                if turn > 30.0 {
                    turn = 30.0;
                }
                if turn < -30.0 {
                    turn = -30.0;
                }
                self.yaw += turn;
                if self.stopped_moving
                    && let Some(walk) = self.walk_target
                    && let Some((wx, _wy, wz)) = resolve_target(world, walk)
                {
                    let toward = (math::atan2(wz - self.z, wx - self.x) * 180.0
                        / std::f64::consts::PI) as f32
                        - 90.0;
                    let strafe_angle = (self.yaw - toward + 90.0) * std::f32::consts::PI / 180.0;
                    self.sideways_speed = -math::sin(strafe_angle) * self.forward_speed;
                    self.forward_speed *= math::cos(strafe_angle);
                }
                if dy > 0.0 {
                    self.jumping = true;
                }
            }
            if let Some(walk) = self.walk_target
                && let Some((wx, wy, wz)) = resolve_target(world, walk)
            {
                self.look_at(wx, wy, wz, 30.0, 30.0);
            }
            if self.colliding_horizontally && self.path.is_none() {
                self.jumping = true;
            }
            if self.rng.next_float() < 0.8 && (in_water || in_lava) {
                self.jumping = true;
            }
        } else {
            self.ai_tick_base(world);
            self.path = None;
        }
    }

    /// `world.findPath(this, walkTarget, 16, true, false, false, true)` — the legacy
    /// path to a walk target.
    fn set_path_to(&mut self, world: &mut dyn MobWorld, tx: f64, ty: f64, tz: f64) {
        let shape = self.shape();
        let mut finder = PathFinder::new(true, false, false, true);
        self.path = finder.find_path_to_entity(&*world, &shape, tx, ty, tz, 16.0);
    }

    /// `PathFinderMobEntity.findTarget()` — animals scan, everything else none.
    fn find_target(&self, world: &mut dyn MobWorld) -> Option<TargetRef> {
        if self.kind.category() == MobCategory::Creature {
            crate::breeding::animal_find_target(self, &*world)
        } else {
            None
        }
    }

    /// `PathFinderMobEntity.targetInSight` — animals run the mating clock.
    fn target_in_sight(&mut self, world: &mut dyn MobWorld, target: TargetRef, distance: f64) {
        if self.kind.category() == MobCategory::Creature {
            crate::breeding::animal_target_in_sight(self, world, target, distance);
        }
    }
}

/// A target's eye height from the seam snapshot.
fn eye_height_of(world: &dyn MobWorld, target: TargetRef, _tz: f64) -> f64 {
    match target {
        TargetRef::Mob(id) => world.mob_ref(id).map_or(0.85, |m| m.height as f64 * 0.85),
        TargetRef::Player(_) => 1.62,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_membership() {
        assert_eq!(MobKind::Zombie.category(), MobCategory::Monster);
        assert_eq!(MobKind::Cow.category(), MobCategory::Creature);
        assert_eq!(MobKind::Bat.category(), MobCategory::Ambient);
        assert_eq!(MobKind::Squid.category(), MobCategory::WaterCreature);
    }

    #[test]
    fn caps_match_vanilla() {
        assert_eq!(MobCategory::Monster.cap(), 70);
        assert_eq!(MobCategory::Creature.cap(), 10);
        assert_eq!(MobCategory::Ambient.cap(), 15);
        assert_eq!(MobCategory::WaterCreature.cap(), 5);
    }

    #[test]
    fn despawn_rules() {
        let mut zombie = Mob::new(MobKind::Zombie, 0.0, 0.0, 0.0);
        assert!(zombie.can_despawn());
        zombie.persistent = true;
        assert!(!zombie.can_despawn());
        let cow = Mob::new(MobKind::Cow, 0.0, 0.0, 0.0);
        assert!(!cow.can_despawn(), "animals never despawn");
    }

    #[test]
    fn breeding_item_table() {
        assert!(MobKind::Cow.is_breeding_item_for(296));
        assert!(MobKind::Pig.is_breeding_item_for(296));
        assert!(MobKind::Sheep.is_breeding_item_for(296));
        assert!(MobKind::Chicken.is_breeding_item_for(295));
        assert!(!MobKind::Chicken.is_breeding_item_for(296));
        assert!(!MobKind::Zombie.is_breeding_item_for(296));
    }

    #[test]
    fn baby_by_age() {
        let mut cow = Mob::new(MobKind::Cow, 0.0, 0.0, 0.0);
        cow.breeding_age = -24000;
        assert!(cow.is_baby());
        cow.breeding_age = 0;
        assert!(!cow.is_baby());
        cow.breeding_age = 6000;
        assert!(!cow.is_baby() && !cow.is_in_love());
    }

    #[test]
    fn safe_fall_distance_without_target() {
        let mob = Mob::new(MobKind::Zombie, 0.0, 0.0, 0.0);
        assert_eq!(mob.safe_fall_distance(), 3);
    }
}
