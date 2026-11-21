//! 1.7.10 `LivingEntity` movement (net.minecraft.entity.living.LivingEntity):
//! `moveRelative` (travel) for land, water and lava, `jump`, sprint jump
//! boost, knockback, and the `mobTick` ordering around them.

use crate::entity::{DamageSource, EntityPhysics};
use crate::math;
use crate::world::PhysWorld;
use sunset_1_7_core::RandomSource;

impl EntityPhysics {
    /// `LivingEntity.jump`: velocity 0.42, jump boost, the sprint forward
    /// kick (`yaw`'s sin/cos on X/Z).
    pub fn jump(&mut self) {
        self.velocity_y = 0.42f32 as f64;
        if self.jump_boost_amplifier >= 0 {
            self.velocity_y += (self.jump_boost_amplifier + 1) as f64 * 0.1;
        }

        if self.sprinting {
            let yaw_radians = self.yaw * (std::f64::consts::PI as f32) / 180.0;
            self.velocity_x -= math::sin(yaw_radians) as f64 * 0.2f32 as f64;
            self.velocity_z += math::cos(yaw_radians) as f64 * 0.2f32 as f64;
        }
    }

    /// `LivingEntity.applyKnockback`: halve the velocity, add 0.4 along the
    /// attack direction, cap rise at 0.4, skip on a knockback-resistance roll.
    pub fn apply_knockback(&mut self, amount: f32, attacker_dx: f64, attacker_dz: f64) {
        let roll = self.random.next_double();
        if roll < self.knockback_resistance {
            return;
        }
        let _ = amount;
        let horizontal =
            math::sqrt_double(attacker_dx * attacker_dx + attacker_dz * attacker_dz) as f64;
        let strength = 0.4f32 as f64;
        self.velocity_x /= 2.0;
        self.velocity_y /= 2.0;
        self.velocity_z /= 2.0;
        self.velocity_x -= attacker_dx / horizontal * strength;
        self.velocity_y += strength;
        self.velocity_z -= attacker_dz / horizontal * strength;
        if self.velocity_y > 0.4f32 as f64 {
            self.velocity_y = 0.4f32 as f64;
        }
    }

    /// `LivingEntity.moveRelative` (func_70989_h / travel): the whole
    /// per-tick movement — land friction and acceleration, water, lava,
    /// climbing, gravity 0.08, drag 0.98/0.91/0.8/0.5 — through
    /// [`EntityPhysics::move_entity`].
    pub fn move_relative(&mut self, world: &dyn PhysWorld, sideways: f32, forwards: f32) {
        if !self.is_in_water() && !(self.flying && self.is_player) {
            if !self.is_in_lava(world) && !(self.flying && self.is_player) {
                self.travel_land(world, sideways, forwards);
            } else {
                self.travel_lava(world, sideways, forwards);
            }
        } else {
            self.travel_water(world, sideways, forwards);
        }

        self.last_walk_animation_speed = self.walk_animation_speed;
        let moved_x = self.x - self.last_x;
        let moved_z = self.z - self.last_z;
        let mut animation = math::sqrt_double(moved_x * moved_x + moved_z * moved_z) * 4.0;
        if animation > 1.0 {
            animation = 1.0;
        }
        self.walk_animation_speed += (animation - self.walk_animation_speed) * 0.4;
        self.walk_animation_progress += self.walk_animation_speed;
    }

    /// The `!inWater` branch's inner `!inLava` branch: ground.
    fn travel_land(&mut self, world: &dyn PhysWorld, sideways: f32, forwards: f32) {
        let mut slip = 0.91f32;
        if self.on_ground {
            slip = world.block_slipperiness(
                math::floor(self.x),
                math::floor(self.shape.min_y) - 1,
                math::floor(self.z),
            ) * 0.91;
        }

        let acceleration = 0.16277136f32 / (slip * slip * slip);
        let speed = if self.on_ground {
            self.get_speed() * acceleration
        } else {
            self.flying_speed
        };

        self.update_velocity(sideways, forwards, speed);
        slip = 0.91f32;
        if self.on_ground {
            slip = world.block_slipperiness(
                math::floor(self.x),
                math::floor(self.shape.min_y) - 1,
                math::floor(self.z),
            ) * 0.91;
        }

        if self.is_climbing(world) {
            const CLIMB: f32 = 0.15;
            if self.velocity_x < -(CLIMB as f64) {
                self.velocity_x = -(CLIMB as f64);
            }
            if self.velocity_x > CLIMB as f64 {
                self.velocity_x = CLIMB as f64;
            }
            if self.velocity_z < -(CLIMB as f64) {
                self.velocity_z = -(CLIMB as f64);
            }
            if self.velocity_z > CLIMB as f64 {
                self.velocity_z = CLIMB as f64;
            }
            self.fall_distance = 0.0;
            if self.velocity_y < -0.15 {
                self.velocity_y = -0.15;
            }
            if self.sneaking && self.is_player && self.velocity_y < 0.0 {
                self.velocity_y = 0.0;
            }
        }

        self.move_entity(
            world,
            self.velocity_x,
            self.velocity_y,
            self.velocity_z,
            &[],
        );
        if self.colliding_horizontally && self.is_climbing(world) {
            self.velocity_y = 0.2;
        }

        self.velocity_y -= 0.08;
        self.velocity_y *= 0.98f32 as f64;
        self.velocity_x *= slip as f64;
        self.velocity_z *= slip as f64;
    }

    /// The `isInLava` branch.
    fn travel_lava(&mut self, world: &dyn PhysWorld, sideways: f32, forwards: f32) {
        let start_y = self.y;
        self.update_velocity(sideways, forwards, 0.02);
        self.move_entity(
            world,
            self.velocity_x,
            self.velocity_y,
            self.velocity_z,
            &[],
        );
        self.velocity_x *= 0.5;
        self.velocity_y *= 0.5;
        self.velocity_z *= 0.5;
        self.velocity_y -= 0.02;
        let rise = self.velocity_y + 0.6f32 as f64 - self.y + start_y;
        if self.colliding_horizontally
            && self.can_move(world, self.velocity_x, rise, self.velocity_z, &[])
        {
            self.velocity_y = 0.3f32 as f64;
        }
    }

    /// The `isInWater` branch: acceleration 0.04 with AI, 0.02 without.
    fn travel_water(&mut self, world: &dyn PhysWorld, sideways: f32, forwards: f32) {
        let start_y = self.y;
        let acceleration = if self.ai_enabled { 0.04f32 } else { 0.02f32 };
        self.update_velocity(sideways, forwards, acceleration);
        self.move_entity(
            world,
            self.velocity_x,
            self.velocity_y,
            self.velocity_z,
            &[],
        );
        self.velocity_x *= 0.8;
        self.velocity_y *= 0.8;
        self.velocity_z *= 0.8;
        self.velocity_y -= 0.02;
        let rise = self.velocity_y + 0.6f32 as f64 - self.y + start_y;
        if self.colliding_horizontally
            && self.can_move(world, self.velocity_x, rise, self.velocity_z, &[])
        {
            self.velocity_y = 0.3f32 as f64;
        }
    }

    /// `LivingEntity.isClimbing`.
    pub fn is_climbing(&self, world: &dyn PhysWorld) -> bool {
        world.block_is_climbable(
            math::floor(self.x),
            math::floor(self.shape.min_y),
            math::floor(self.z),
        )
    }

    /// The AI core of `LivingEntity.mobTick`: jump handling, input decay, the
    /// travel call, and the <0.005 velocity snap. Entity pushing and AI goal
    /// selection are other subsystems; `lerp` handling is network-only.
    pub fn mob_tick(&mut self, world: &dyn PhysWorld) {
        if self.jumping_cooldown > 0 {
            self.jumping_cooldown -= 1;
        }

        if self.velocity_x.abs() < 0.005 {
            self.velocity_x = 0.0;
        }
        if self.velocity_y.abs() < 0.005 {
            self.velocity_y = 0.0;
        }
        if self.velocity_z.abs() < 0.005 {
            self.velocity_z = 0.0;
        }

        // The dead/AI-input branch: the caller sets jumping and the speeds.
        if self.jumping {
            if self.is_in_water() || self.is_in_lava(world) {
                self.velocity_y += 0.04f32 as f64;
            } else if self.on_ground && self.jumping_cooldown == 0 {
                self.jump();
                self.jumping_cooldown = 10;
            }
        } else {
            self.jumping_cooldown = 0;
        }

        self.sideways_speed *= 0.98;
        self.forward_speed *= 0.98;
        self.move_relative(world, self.sideways_speed, self.forward_speed);
    }

    /// `LivingEntity.voidTick`: 4 damage a tick below y - 64 (base `Entity`
    /// removes non-living entities instead).
    pub fn void_tick(&mut self) {
        self.pending_damage.push((DamageSource::OutOfWorld, 4.0));
    }

    /// One hurt event's velocity half of `LivingEntity.takeDamage`
    /// (`damagedSwingDir` and knockback); sounds, timers and death are the
    /// combat crate's side. `attacker_dx/dz` is `attacker - this`.
    pub fn on_attacked(&mut self, attacker_dx: f64, attacker_dz: f64) {
        let mut dx = attacker_dx;
        let mut dz = attacker_dz;
        while dx * dx + dz * dz < 1.0e-4 {
            dx = (self.random.next_double() - self.random.next_double()) * 0.01;
            dz = (self.random.next_double() - self.random.next_double()) * 0.01;
        }
        let swing_dir = (dz.atan2(dx) * 180.0 / std::f64::consts::PI) as f32 - self.yaw;
        let _ = swing_dir;
        self.apply_knockback(1.0, dx, dz);
    }
}

impl crate::entity::DamageSource {
    /// `DamageSource.FALL.getOutOfWorld`-style: fall bypasses armor in 1.7.10;
    /// fire sources are blocked by fire resistance. The combat crate owns the
    /// full table; this is the physics-relevant subset.
    pub fn is_fall(self) -> bool {
        matches!(self, DamageSource::Fall)
    }

    pub fn is_fire(self) -> bool {
        matches!(
            self,
            DamageSource::Fire | DamageSource::Lava | DamageSource::OnFire
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::DamageSource;
    use crate::fixture::{Liquid, TestWorld};
    use sunset_1_7_core::LegacyRandom;

    #[test]
    fn gravity_is_minus_08_then_times_098() {
        let world = TestWorld::flat();
        let mut entity = EntityPhysics::living(0.6, 1.8);
        entity.set_position(0.5, 100.0, 0.5);
        entity.tick_start();
        entity.move_relative(&world, 0.0, 0.0);
        assert_eq!(entity.velocity_y, (0.0f64 - 0.08) * (0.98f32 as f64));
        assert_eq!(entity.velocity_x, 0.0);
    }

    #[test]
    fn jump_is_042_and_sprint_kicks_forward() {
        let mut entity = EntityPhysics::living(0.6, 1.8);
        entity.jump();
        assert_eq!(entity.velocity_y, 0.42f32 as f64);

        entity.sprinting = true;
        entity.velocity_y = 0.0;
        entity.velocity_x = 0.0;
        entity.velocity_z = 0.0;
        entity.yaw = 0.0;
        entity.jump();
        assert_eq!(entity.velocity_y, 0.42f32 as f64);
        assert_eq!(entity.velocity_z, 0.2f32 as f64);
        assert_eq!(entity.velocity_x, 0.0);

        entity.velocity_x = 0.0;
        entity.velocity_z = 0.0;
        entity.yaw = 90.0;
        entity.jump();
        assert_eq!(entity.velocity_x, (-(0.2f32)) as f64);
    }

    #[test]
    fn big_fall_deals_ceil_distance_minus_three() {
        let world = TestWorld::flat();
        let mut entity = EntityPhysics::living(0.6, 1.8);
        entity.set_position(0.5, 20.0, 0.5);
        // move_relative applies gravity after each move, vanilla tick order.
        while !entity.on_ground {
            entity.move_relative(&world, 0.0, 0.0);
        }
        assert_eq!(entity.pending_damage, vec![(DamageSource::Fall, 15.0)]);
    }

    #[test]
    fn jump_boost_forgives_three_blocks() {
        // A 5.4-block fall accumulates ~4.57 over the air ticks (the landing
        // tick's partial is excluded in 1.7.10).
        let world = TestWorld::flat();

        // Jump Boost I forgives one more block: ceil(4.57 - 3 - 1) = 1.
        let mut boosted = EntityPhysics::living(0.6, 1.8);
        boosted.jump_boost_amplifier = 0;
        boosted.set_position(0.5, (1.8f32 * 0.85) as f64 + 5.4, 0.5);
        while !boosted.on_ground {
            boosted.move_relative(&world, 0.0, 0.0);
        }
        assert_eq!(boosted.pending_damage, vec![(DamageSource::Fall, 1.0)]);

        // Without the effect: ceil(4.57 - 3) = 2.
        let mut plain = EntityPhysics::living(0.6, 1.8);
        plain.set_position(0.5, (1.8f32 * 0.85) as f64 + 5.4, 0.5);
        while !plain.on_ground {
            plain.move_relative(&world, 0.0, 0.0);
        }
        assert_eq!(plain.pending_damage, vec![(DamageSource::Fall, 2.0)]);
    }

    #[test]
    fn water_travel_dampens_by_08() {
        let world = TestWorld::tank(Liquid::Water, 20, 0, 20, 4, 4, 4);
        let mut entity = EntityPhysics::living(0.6, 1.8);
        entity.set_position(22.5, 3.5, 22.5);
        entity.fall_distance = 12.0;
        assert!(entity.check_water_collisions(&world));
        assert!(entity.in_water);
        assert_eq!(entity.fall_distance, 0.0);
        entity.tick_start();
        entity.move_relative(&world, 0.0, 0.0);
        assert_eq!(entity.velocity_y, (0.0f64 * (0.8f32 as f64)) - 0.02);
    }

    #[test]
    fn lava_travel_dampens_by_half() {
        let world = TestWorld::tank(Liquid::Lava, 20, 0, 20, 4, 4, 4);
        let mut entity = EntityPhysics::living(0.6, 1.8);
        entity.set_position(22.5, 2.0, 22.5);
        assert!(entity.is_in_lava(&world));
        entity.tick_start();
        entity.move_relative(&world, 0.0, 0.0);
        assert_eq!(entity.velocity_y, (0.0f64 * 0.5) - 0.02);
    }

    #[test]
    fn swim_up_at_the_water_surface_against_a_wall() {
        let mut world = TestWorld::tank(Liquid::Water, 20, 0, 20, 4, 4, 4);
        for y in 0..4 {
            world.solid.push((24, y, 20));
        }
        let mut entity = EntityPhysics::living(0.6, 1.8);
        entity.set_position(23.6, 5.0, 20.5);
        assert!(entity.check_water_collisions(&world));
        entity.velocity_x = 0.1;
        entity.tick_start();
        entity.move_relative(&world, 0.0, 0.0);
        assert!(entity.colliding_horizontally);
        assert_eq!(entity.velocity_y, 0.3f32 as f64);
    }

    #[test]
    fn knockback_is_deterministic_per_seed() {
        let mut first = EntityPhysics::living(0.6, 1.8);
        first.random = LegacyRandom::new(42);
        first.apply_knockback(1.0, 0.0, 1.0);
        let mut second = EntityPhysics::living(0.6, 1.8);
        second.random = LegacyRandom::new(42);
        second.apply_knockback(1.0, 0.0, 1.0);
        assert_eq!(first.velocity_y, second.velocity_y);
        assert_eq!(first.velocity_z, second.velocity_z);
        // No resistance: up 0.4, back along -Z by 0.4.
        assert_eq!(second.velocity_y, 0.4f32 as f64);
        assert_eq!(second.velocity_z, -(1.0f64 * (0.4f32 as f64)));

        // Full knockback resistance: the roll always refuses.
        let mut armoured = EntityPhysics::living(0.6, 1.8);
        armoured.knockback_resistance = 1.0;
        armoured.apply_knockback(1.0, 1.0, 0.0);
        assert_eq!(armoured.velocity_y, 0.0);
        assert_eq!(armoured.velocity_x, 0.0);
    }
}
