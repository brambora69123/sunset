//! 1.7.10 `Entity` physics (net.minecraft.entity.Entity): the AABB move and
//! clip (`Entity.move`), strafe acceleration (`updateVelocity`), water and
//! lava detection, fall distance. Ported from the 1.7.10 decompiled sources.
//!
//! Position note: 1.7.10 keeps `y` at eye level (`setPosition` puts the box at
//! `y - eyeHeight + eyeHeightSneakOffset`), and `move` re-derives `x`/`y`/`z`
//! from the box. The port keeps both exactly.

// The negated comparisons are the Java source's shape, kept line for line
// (and NaN-safe); clippy would rewrite them into the same machine code.
#![allow(clippy::neg_cmp_op_on_partial_ord)]

use sunset_1_7_core::LegacyRandom;

use crate::aabb::Aabb;
use crate::material::Material;
use crate::math;
use crate::world::{
    PhysWorld, apply_liquid_drag, contains_fire_source, contains_liquid, contains_material,
    get_collisions,
};

/// Damage inputs physics produces; the game maps them to its own sources.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DamageSource {
    Fall,
    Lava,
    Fire,
    OnFire,
    InWall,
    Drown,
    OutOfWorld,
}

/// The physics state and movement of one 1.7.10 entity.
pub struct EntityPhysics {
    /// Position; `y` is at eye level (vanilla 1.7.10).
    pub x: f64,
    pub y: f64,
    pub z: f64,
    /// Last-tick position (`Entity.lastX/Y/Z`), for animation measures.
    pub last_x: f64,
    pub last_y: f64,
    pub last_z: f64,
    pub velocity_x: f64,
    pub velocity_y: f64,
    pub velocity_z: f64,
    pub yaw: f32,
    /// The authoritative AABB; position is re-derived from it after a move.
    pub shape: Aabb,

    pub width: f32,
    pub height: f32,
    pub eye_height: f32,
    pub eye_height_sneak_offset: f32,
    pub step_height: f32,

    pub on_ground: bool,
    pub colliding_horizontally: bool,
    pub colliding_vertically: bool,
    pub colliding: bool,
    pub fall_distance: f32,
    pub in_water: bool,
    pub in_cobweb: bool,
    pub no_clip: bool,
    pub immovable: bool,
    pub sneaking: bool,
    pub sprinting: bool,
    pub first_tick: bool,
    pub removed: bool,

    /// Behaviour switches the game drives. `Entity.move`'s sneak edge guard
    /// runs for players only, `checkFallDamage`'s water recheck for living
    /// entities only.
    pub is_player: bool,
    pub is_living: bool,
    pub flying: bool,
    pub ai_enabled: bool,
    pub riding: bool,
    pub makes_steps: bool,
    pub has_liquid_collision: bool,
    pub immune_to_fire: bool,
    /// Vanilla `JUMP_BOOST` amplifier (0 = level I); -1 = no effect.
    pub jump_boost_amplifier: i32,
    pub knockback_resistance: f64,

    /// Living speeds (`LivingEntity` fields).
    pub speed: f32,
    pub flying_speed: f32,
    pub sideways_speed: f32,
    pub forward_speed: f32,
    pub jumping: bool,
    pub jumping_cooldown: i32,
    pub walk_animation_speed: f32,
    pub walk_animation_progress: f32,
    pub last_walk_animation_speed: f32,

    pub on_fire_timer: i32,
    pub safe_on_fire_time: i32,
    pub walk_distance: f32,
    pub last_walk_distance: f32,
    pub distance_moved: f32,
    pub blocks_walked_on: i32,
    pub push_speed_reduction: f32,

    /// The entity's own `Random` (vanilla field); the game sets the seed.
    pub random: LegacyRandom,
    /// Damage events the game drains each tick.
    pub pending_damage: Vec<(DamageSource, f32)>,
}

impl EntityPhysics {
    /// Base `Entity` defaults: `stepHeight` 0, no fire immunity.
    pub fn new(width: f32, height: f32, eye_height: f32) -> Self {
        let mut entity = Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            velocity_x: 0.0,
            velocity_y: 0.0,
            velocity_z: 0.0,
            yaw: 0.0,
            shape: Aabb::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            last_x: 0.0,
            last_y: 0.0,
            last_z: 0.0,
            width,
            height,
            eye_height,
            eye_height_sneak_offset: 0.0,
            step_height: 0.0,
            on_ground: false,
            colliding_horizontally: false,
            colliding_vertically: false,
            colliding: false,
            fall_distance: 0.0,
            in_water: false,
            in_cobweb: false,
            no_clip: false,
            immovable: true,
            sneaking: false,
            sprinting: false,
            first_tick: true,
            removed: false,
            is_player: false,
            is_living: false,
            flying: false,
            ai_enabled: false,
            riding: false,
            makes_steps: true,
            has_liquid_collision: true,
            immune_to_fire: false,
            jump_boost_amplifier: -1,
            knockback_resistance: 0.0,
            speed: 0.1,
            flying_speed: 0.02,
            sideways_speed: 0.0,
            forward_speed: 0.0,
            jumping: false,
            jumping_cooldown: 0,
            walk_animation_speed: 0.0,
            walk_animation_progress: 0.0,
            last_walk_animation_speed: 0.0,
            on_fire_timer: 0,
            safe_on_fire_time: 1,
            walk_distance: 0.0,
            last_walk_distance: 0.0,
            distance_moved: 0.0,
            blocks_walked_on: 1,
            push_speed_reduction: 0.0,
            random: LegacyRandom::new(0),
            pending_damage: Vec::new(),
        };
        entity.set_position(0.0, 0.0, 0.0);
        entity
    }

    /// `LivingEntity` defaults: eye height `height * 0.85`, `stepHeight` 0.5.
    pub fn living(width: f32, height: f32) -> Self {
        let mut entity = Self::new(width, height, height * 0.85);
        entity.is_living = true;
        entity.step_height = 0.5;
        entity
    }

    /// `Entity.setPosition`.
    pub fn set_position(&mut self, x: f64, y: f64, z: f64) {
        self.x = x;
        self.y = y;
        self.z = z;
        let half = self.width / 2.0;
        let height = self.height;
        let bottom = y - self.eye_height as f64 + self.eye_height_sneak_offset as f64;
        self.shape = Aabb::new(
            x - half as f64,
            bottom,
            z - half as f64,
            x + half as f64,
            bottom + height as f64,
            z + half as f64,
        );
    }

    /// `Entity.baseTick`'s position snapshot: `lastX/Y/Z = x/y/z`.
    pub fn tick_start(&mut self) {
        self.last_walk_distance = self.walk_distance;
        self.last_x = self.x;
        self.last_y = self.y;
        self.last_z = self.z;
    }

    /// `LivingEntity.getEyeHeight` — `height * 0.85` (players set 1.62 via
    /// `eye_height`).
    pub fn get_eye_height(&self) -> f32 {
        self.eye_height
    }

    /// `LivingEntity.getSpeed`: the AI speed when AI is on, else 0.1.
    pub fn get_speed(&self) -> f32 {
        if self.ai_enabled { self.speed } else { 0.1 }
    }

    /// `Entity.updateVelocity` (strafe acceleration): normalize the input
    /// above 1, scale it, and add it rotated by yaw.
    pub fn update_velocity(&mut self, mut sideways: f32, mut forwards: f32, scale: f32) {
        let mut length = sideways * sideways + forwards * forwards;
        if !(length < 1.0e-4) {
            length = math::sqrt(length);
            if length < 1.0 {
                length = 1.0;
            }
            length = scale / length;
            sideways *= length;
            forwards *= length;
            let sin = math::sin(self.yaw * (std::f64::consts::PI as f32) / 180.0);
            let cos = math::cos(self.yaw * (std::f64::consts::PI as f32) / 180.0);
            self.velocity_x += (sideways * cos - forwards * sin) as f64;
            self.velocity_z += (forwards * cos + sideways * sin) as f64;
        }
    }

    /// `Entity.canMove`.
    pub fn can_move(
        &self,
        world: &dyn PhysWorld,
        dx: f64,
        dy: f64,
        dz: f64,
        nearby: &[Aabb],
    ) -> bool {
        let moved = self.shape.moved(dx, dy, dz);
        if !get_collisions(world, nearby, &moved).is_empty() {
            return false;
        }
        !contains_liquid(world, &moved)
    }

    /// `Entity.checkWaterCollisions`: liquid drag over the box grown down 0.4;
    /// entering water zeroes fall distance and fire.
    pub fn check_water_collisions(&mut self, world: &dyn PhysWorld) -> bool {
        let bounds = self
            .shape
            .grown(0.0, -0.4f32 as f64, 0.0)
            .contract(0.001, 0.001, 0.001);
        let drag = apply_liquid_drag(world, &bounds, Material::Water);
        // Splash sound and particles are the game's side effects.
        if drag.in_liquid {
            self.fall_distance = 0.0;
            self.in_water = true;
            self.on_fire_timer = 0;
        } else {
            self.in_water = false;
        }
        if self.has_liquid_collision {
            self.velocity_x += drag.push[0];
            self.velocity_y += drag.push[1];
            self.velocity_z += drag.push[2];
        }
        self.in_water
    }

    /// `Entity.isInWater`.
    pub fn is_in_water(&self) -> bool {
        self.in_water
    }

    /// `Entity.isInLava`: the box grown -0.1 sideways, -0.4 down, in lava.
    pub fn is_in_lava(&self, world: &dyn PhysWorld) -> bool {
        contains_material(
            world,
            &self
                .shape
                .grown(-0.1f32 as f64, -0.4f32 as f64, -0.1f32 as f64),
            Material::Lava,
        )
    }

    /// `Entity.isSubmergedIn`: the eye block under the liquid surface.
    pub fn is_submerged_in(&self, world: &dyn PhysWorld, material: Material) -> bool {
        let eye_y = self.y + self.get_eye_height() as f64;
        let x = math::floor(self.x);
        let y = math::floor(math::floor(eye_y) as f64);
        let z = math::floor(self.z);
        if world.block_material(x, y, z) == material {
            let offset = world.liquid_height_loss(x, y, z) - 0.11111111f32;
            let surface = (y + 1) as f32 - offset;
            eye_y < surface as f64
        } else {
            false
        }
    }

    /// `Entity.isInWaterOrRain`.
    pub fn is_in_water_or_rain(&self, world: &dyn PhysWorld) -> bool {
        self.in_water
            || world.is_raining(
                math::floor(self.x),
                math::floor(self.y),
                math::floor(self.z),
            )
            || world.is_raining(
                math::floor(self.x),
                math::floor(self.y + self.height as f64),
                math::floor(self.z),
            )
    }

    /// `Entity.onCobwebCollision`.
    pub fn on_cobweb_collision(&mut self) {
        self.in_cobweb = true;
        self.fall_distance = 0.0;
    }

    /// `Entity.setOnFireFor`, without the protection enchantment modifier
    /// (the game pre-adjusts the seconds).
    pub fn set_on_fire_for(&mut self, seconds: i32) {
        let ticks = seconds * 20;
        if self.on_fire_timer < ticks {
            self.on_fire_timer = ticks;
        }
    }

    /// `Entity.takeFallDamage` (base): only the rider inherits it.
    pub fn take_fall_damage(&mut self, distance: f32) {
        if !self.is_living {
            return;
        }
        let boost = if self.jump_boost_amplifier >= 0 {
            (self.jump_boost_amplifier + 1) as f32
        } else {
            0.0
        };
        let damage = math::ceil_float(distance - 3.0 - boost);
        if damage > 0 {
            // Fall and landing sounds are the game's side effects.
            self.pending_damage
                .push((DamageSource::Fall, damage as f32));
        }
    }

    /// `LivingEntity.checkFallDamage`: a water recheck, then the base
    /// accumulation and damage. The landing block's `onFallenOn` and the
    /// 2006 event are the game's side effects.
    fn dispatch_check_fall_damage(&mut self, world: &dyn PhysWorld, dy: f64, on_ground: bool) {
        if self.is_living && !self.in_water {
            self.check_water_collisions(world);
        }
        if on_ground {
            if self.fall_distance > 0.0 {
                let distance = self.fall_distance;
                self.take_fall_damage(distance);
                self.fall_distance = 0.0;
            }
        } else if dy < 0.0 {
            self.fall_distance = (self.fall_distance as f64 - dy) as f32;
        }
    }

    /// `Entity.move` (func_70091_d): the full AABB move and clip — cobweb
    /// dampening, the player sneak edge guard, Y/X/Z clip order, the step-up
    /// retry, ground detection, fall damage and velocity zeroing.
    ///
    /// `nearby` holds the collision shapes of the entities around this one
    /// (vanilla gathers them with `getEntities(shape.grown(0.25))`).
    pub fn move_entity(
        &mut self,
        world: &dyn PhysWorld,
        dx: f64,
        dy: f64,
        dz: f64,
        nearby: &[Aabb],
    ) {
        if self.no_clip {
            self.shape = self.shape.moved(dx, dy, dz);
            self.x = (self.shape.min_x + self.shape.max_x) / 2.0;
            self.y =
                self.shape.min_y + self.eye_height as f64 - self.eye_height_sneak_offset as f64;
            self.z = (self.shape.min_z + self.shape.max_z) / 2.0;
            return;
        }

        self.eye_height_sneak_offset *= 0.4;
        let start_x = self.x;
        let start_y = self.y;
        let start_z = self.z;

        let mut dx = dx;
        let mut dy = dy;
        let mut dz = dz;
        if self.in_cobweb {
            self.in_cobweb = false;
            dx *= 0.25;
            dy *= 0.05f32 as f64;
            dz *= 0.25;
            self.velocity_x = 0.0;
            self.velocity_y = 0.0;
            self.velocity_z = 0.0;
        }

        // var13/var15/var17: the shifts the collision zeroing compares against.
        // The sneak loops update the X and Z ones.
        let mut shift_x = dx;
        let shift_y = dy;
        let mut shift_z = dz;
        let original_shape = self.shape;
        let sneak_guard = self.on_ground && self.sneaking && self.is_player;
        if sneak_guard {
            const EDGE: f64 = 0.05;
            while dx != 0.0
                && get_collisions(world, nearby, &self.shape.moved(dx, -1.0, 0.0)).is_empty()
            {
                if (-EDGE..EDGE).contains(&dx) {
                    dx = 0.0;
                } else if dx > 0.0 {
                    dx -= EDGE;
                } else {
                    dx += EDGE;
                }
                shift_x = dx;
            }

            while dz != 0.0
                && get_collisions(world, nearby, &self.shape.moved(0.0, -1.0, dz)).is_empty()
            {
                if (-EDGE..EDGE).contains(&dz) {
                    dz = 0.0;
                } else if dz > 0.0 {
                    dz -= EDGE;
                } else {
                    dz += EDGE;
                }
                shift_z = dz;
            }

            while dx != 0.0
                && dz != 0.0
                && get_collisions(world, nearby, &self.shape.moved(dx, -1.0, dz)).is_empty()
            {
                if (-EDGE..EDGE).contains(&dx) {
                    dx = 0.0;
                } else if dx > 0.0 {
                    dx -= EDGE;
                } else {
                    dx += EDGE;
                }
                if (-EDGE..EDGE).contains(&dz) {
                    dz = 0.0;
                } else if dz > 0.0 {
                    dz -= EDGE;
                } else {
                    dz += EDGE;
                }
                shift_x = dx;
                shift_z = dz;
            }
        }

        let mut collisions = get_collisions(world, nearby, &self.shape.expanded(dx, dy, dz));

        for box_ in &collisions {
            dy = box_.intersect_y(&self.shape, dy);
        }
        self.shape = self.shape.moved(0.0, dy, 0.0);
        if !self.immovable && shift_y != dy {
            dz = 0.0;
            dy = 0.0;
            dx = 0.0;
        }

        let landed = self.on_ground || (shift_y != dy && shift_y < 0.0);

        for box_ in &collisions {
            dx = box_.intersect_x(&self.shape, dx);
        }
        self.shape = self.shape.moved(dx, 0.0, 0.0);
        if !self.immovable && shift_x != dx {
            dz = 0.0;
            dy = 0.0;
            dx = 0.0;
        }

        for box_ in &collisions {
            dz = box_.intersect_z(&self.shape, dz);
        }
        self.shape = self.shape.moved(0.0, 0.0, dz);
        if !self.immovable && shift_z != dz {
            dz = 0.0;
            dy = 0.0;
            dx = 0.0;
        }

        if self.step_height > 0.0
            && landed
            && (sneak_guard || self.eye_height_sneak_offset < 0.05)
            && (shift_x != dx || shift_z != dz)
        {
            let pre_dx = dx;
            let pre_dy = dy;
            let pre_dz = dz;
            let pre_shape = self.shape;
            dx = shift_x;
            dy = self.step_height as f64;
            dz = shift_z;
            self.shape = original_shape;
            collisions = get_collisions(world, nearby, &self.shape.expanded(dx, dy, dz));

            for box_ in &collisions {
                dy = box_.intersect_y(&self.shape, dy);
            }
            self.shape = self.shape.moved(0.0, dy, 0.0);
            if !self.immovable && shift_y != dy {
                dz = 0.0;
                dy = 0.0;
                dx = 0.0;
            }

            for box_ in &collisions {
                dx = box_.intersect_x(&self.shape, dx);
            }
            self.shape = self.shape.moved(dx, 0.0, 0.0);
            if !self.immovable && shift_x != dx {
                dz = 0.0;
                dy = 0.0;
                dx = 0.0;
            }

            for box_ in &collisions {
                dz = box_.intersect_z(&self.shape, dz);
            }
            self.shape = self.shape.moved(0.0, 0.0, dz);
            if !self.immovable && shift_z != dz {
                dz = 0.0;
                dy = 0.0;
                dx = 0.0;
            }

            if !self.immovable && shift_y != dy {
                dz = 0.0;
                dy = 0.0;
                dx = 0.0;
            } else {
                dy = -(self.step_height as f64);
                for box_ in &collisions {
                    dy = box_.intersect_y(&self.shape, dy);
                }
                self.shape = self.shape.moved(0.0, dy, 0.0);
            }

            if pre_dx * pre_dx + pre_dz * pre_dz >= dx * dx + dz * dz {
                dx = pre_dx;
                dy = pre_dy;
                dz = pre_dz;
                self.shape = pre_shape;
            }
        }

        self.x = (self.shape.min_x + self.shape.max_x) / 2.0;
        self.y = self.shape.min_y + self.eye_height as f64 - self.eye_height_sneak_offset as f64;
        self.z = (self.shape.min_z + self.shape.max_z) / 2.0;
        self.colliding_horizontally = shift_x != dx || shift_z != dz;
        self.colliding_vertically = shift_y != dy;
        self.on_ground = shift_y != dy && shift_y < 0.0;
        self.colliding = self.colliding_horizontally || self.colliding_vertically;
        self.dispatch_check_fall_damage(world, dy, self.on_ground);
        if shift_x != dx {
            self.velocity_x = 0.0;
        }
        if shift_y != dy {
            self.velocity_y = 0.0;
        }
        if shift_z != dz {
            self.velocity_z = 0.0;
        }

        // Walk distance and fire (the "rest" section). Step sounds, block
        // `onSteppedOn` and `onEntityCollision` are the game's side effects.
        let moved_x = self.x - start_x;
        let moved_y = self.y - start_y;
        let moved_z = self.z - start_z;
        if self.makes_steps && !sneak_guard && !self.riding {
            let x = math::floor(self.x);
            let y = math::floor(self.y - 0.2f32 as f64 - self.eye_height as f64);
            let z = math::floor(self.z);
            // The Y distance only counts while on a ladder.
            let climbed_y = if world.block_is_ladder(x, y, z) {
                moved_y
            } else {
                0.0
            };
            let horizontal = math::sqrt_double(moved_x * moved_x + moved_z * moved_z) as f64 * 0.6;
            self.walk_distance = (self.walk_distance as f64 + horizontal) as f32;
            self.distance_moved = (self.distance_moved as f64
                + math::sqrt_double(moved_x * moved_x + climbed_y * climbed_y + moved_z * moved_z)
                    as f64
                    * 0.6) as f32;
            if self.distance_moved > self.blocks_walked_on as f32 {
                self.blocks_walked_on = self.distance_moved as i32 + 1;
            }
        }

        let water_or_rain = self.is_in_water_or_rain(world);
        if contains_fire_source(world, &self.shape.contract(0.001, 0.001, 0.001)) {
            if !self.immune_to_fire {
                self.pending_damage.push((DamageSource::Fire, 1.0));
            }
            if !water_or_rain {
                self.on_fire_timer += 1;
                if self.on_fire_timer == 0 {
                    self.set_on_fire_for(8);
                }
            }
        } else if self.on_fire_timer <= 0 {
            self.on_fire_timer = -self.safe_on_fire_time;
        }
        if water_or_rain && self.on_fire_timer > 0 {
            self.on_fire_timer = -self.safe_on_fire_time;
        }
        let _ = (start_y, start_z, moved_x, moved_y, moved_z);
    }

    /// `Entity.push`: horizontal push-off between two entities. The caller
    /// guards rider/vehicle pairs and pushability.
    pub fn push(&mut self, other: &mut EntityPhysics) {
        let dx = other.x - self.x;
        let dz = other.z - self.z;
        let mut distance = math::abs_max(dx, dz);
        if distance >= 0.01f32 as f64 {
            distance = math::sqrt_double(distance) as f64;
            let mut nx = dx / distance;
            let mut nz = dz / distance;
            let mut scale = 1.0 / distance;
            if scale > 1.0 {
                scale = 1.0;
            }
            nx *= scale;
            nz *= scale;
            nx *= 0.05f32 as f64;
            nz *= 0.05f32 as f64;
            let reduction = (1.0f32 - self.push_speed_reduction) as f64;
            nx *= reduction;
            nz *= reduction;
            self.velocity_x += -nx;
            self.velocity_z += -nz;
            other.velocity_x += nx;
            other.velocity_z += nz;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::{Liquid, TestWorld};

    #[test]
    fn fall_lands_exactly_and_damages() {
        let world = TestWorld::flat();
        let mut entity = EntityPhysics::living(0.6, 1.8);
        entity.set_position(0.5, 20.0, 0.5);
        while !entity.on_ground {
            entity.velocity_y -= 0.08;
            entity.velocity_y *= 0.98;
            entity.move_entity(&world, 0.0, entity.velocity_y, 0.0, &[]);
        }
        let eye = (1.8f32 * 0.85) as f64;
        assert!(entity.on_ground);
        assert_eq!(entity.y, eye);
        // The landing tick's partial fall is excluded (1.7.10 behavior):
        // accumulated over the air ticks only, ceil(17.64 - 3) = 15.
        assert_eq!(entity.pending_damage, vec![(DamageSource::Fall, 15.0)]);
        assert_eq!(entity.fall_distance, 0.0);
    }

    #[test]
    fn step_up_onto_a_half_block() {
        let mut world = TestWorld::flat();
        world.half.push(((3, 0, 0), 0.5));
        let mut entity = EntityPhysics::living(0.6, 1.8);
        entity.set_position(2.5, (1.8f32 * 0.85) as f64, 0.5);
        entity.on_ground = true;
        entity.move_entity(&world, 0.3, -0.0784, 0.0, &[]);
        let eye = (1.8f32 * 0.85) as f64;
        assert_eq!(entity.x, 2.5 + 0.3);
        assert_eq!(entity.y, 0.5 + eye);
        assert!(entity.on_ground);
        assert!(!entity.colliding_horizontally);
    }

    #[test]
    fn sneak_does_not_walk_off_an_edge() {
        let mut world = TestWorld::flat();
        world.ground = false;
        for x in 0..=4 {
            world.solid.push((x, -1, 0));
        }
        // Floating over the hole, sneaking: the 0.05 edge guard stops the walk.
        let mut entity = EntityPhysics::living(0.6, 1.8);
        entity.is_player = true;
        entity.sneaking = true;
        entity.set_position(5.4, (1.8f32 * 0.85) as f64, 0.5);
        entity.on_ground = true;
        entity.move_entity(&world, 0.6, -0.0784, 0.0, &[]);
        assert_eq!(entity.x, 5.4);
        // The guard zeroes the whole move, so 1.7.10 reports no horizontal
        // collision either — vanilla sets `collidingHorizontally` false here.
        assert!(!entity.colliding_horizontally);
        // Without sneaking the same move walks on.
        let mut walker = EntityPhysics::living(0.6, 1.8);
        walker.set_position(5.4, (1.8f32 * 0.85) as f64, 0.5);
        walker.on_ground = true;
        walker.move_entity(&world, 0.6, -0.0784, 0.0, &[]);
        assert_eq!(walker.x, 5.4 + 0.6);
    }

    #[test]
    fn wall_clip_zeroes_velocity() {
        let mut world = TestWorld::flat();
        for y in 0..2 {
            world.solid.push((10, y, 0));
        }
        let mut entity = EntityPhysics::living(0.6, 1.8);
        entity.set_position(9.5, (1.8f32 * 0.85) as f64, 0.5);
        entity.on_ground = true;
        entity.move_entity(&world, 0.5, -0.0784, 0.0, &[]);
        assert!(entity.colliding_horizontally);
        assert_eq!(entity.velocity_x, 0.0);
        // Clamped exactly at the wall face; position re-derived from the box.
        assert_eq!(entity.shape.max_x, 10.0);
        assert_eq!(entity.x, (entity.shape.min_x + entity.shape.max_x) / 2.0);
    }

    #[test]
    fn strafe_accelerates_by_yaw() {
        let mut entity = EntityPhysics::living(0.6, 1.8);
        entity.update_velocity(0.0, 1.0, 0.02);
        assert_eq!(entity.velocity_z, 0.02f32 as f64);
        assert_eq!(entity.velocity_x, 0.0);

        entity.yaw = 90.0;
        entity.update_velocity(0.0, 1.0, 0.02);
        assert_eq!(entity.velocity_x, (-(0.02f32)) as f64);
    }

    #[test]
    fn can_move_rejects_walls_and_liquids() {
        let mut world = TestWorld::tank(Liquid::Water, 20, 0, 20, 4, 4, 4);
        world.solid.push((24, 1, 20));
        let mut entity = EntityPhysics::living(0.6, 1.8);
        entity.set_position(23.0, 2.5, 20.5);
        // The wall east of the tank.
        assert!(!entity.can_move(&world, 1.0, 0.0, 0.0, &[]));
        // The ground below.
        assert!(!entity.can_move(&world, 0.0, -4.0, 0.0, &[]));
        // Clear air above the tank.
        assert!(entity.can_move(&world, 0.0, 4.0, 0.0, &[]));
    }

    #[test]
    fn cobweb_dampens_the_move() {
        let world = TestWorld::flat();
        let mut entity = EntityPhysics::living(0.6, 1.8);
        entity.set_position(0.5, 10.0, 0.5);
        entity.velocity_x = 1.0;
        entity.on_cobweb_collision();
        entity.move_entity(&world, 1.0, -0.0784, 0.0, &[]);
        assert_eq!(entity.velocity_x, 0.0);
        // dx scaled to 0.25
        assert_eq!(entity.x, 0.5 + 0.25);
    }

    #[test]
    fn water_tank_submersion_checks() {
        let world = TestWorld::tank(Liquid::Water, 20, 0, 20, 4, 4, 4);
        let mut entity = EntityPhysics::living(0.6, 1.8);
        // Eye under the y=3 block's surface (3 + 1 - 1/9 - 1/9).
        entity.set_position(22.5, 2.0, 22.5);
        assert!(entity.is_submerged_in(&world, Material::Water));
        // Eye above every surface, and out of the tank entirely for lava.
        entity.set_position(22.5, 3.5, 22.5);
        assert!(!entity.is_submerged_in(&world, Material::Water));
        let lava = TestWorld::tank(Liquid::Lava, 20, 0, 20, 4, 4, 4);
        entity.set_position(22.5, 10.0, 22.5);
        assert!(!entity.is_in_lava(&lava));
        entity.set_position(22.5, 0.3, 22.5);
        assert!(entity.is_in_lava(&lava));
    }
}
