//! The AI controls, from `entity/ai/control/`: `MovementControl`, `LookControl`,
//! `JumpControl`. (1.7.10's `BodyControl` runs inside `LivingEntity.tick`, the
//! movement renderer reads `bodyYaw`; the AI-visible part is covered here.)

use crate::math;

/// `MovementControl`: the navigation's steering requests, applied to yaw and speed.
pub struct MovementControl {
    x: f64,
    y: f64,
    z: f64,
    speed: f64,
    moving: bool,
}

impl MovementControl {
    pub fn new() -> Self {
        MovementControl {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            speed: 0.0,
            moving: false,
        }
    }

    /// `isMoving()`.
    pub fn is_moving(&self) -> bool {
        self.moving
    }

    /// `getSpeed()`.
    pub fn get_speed(&self) -> f64 {
        self.speed
    }

    /// `update(x, y, z, speed)`.
    pub fn update(&mut self, x: f64, y: f64, z: f64, speed: f64) {
        self.x = x;
        self.y = y;
        self.z = z;
        self.speed = speed;
        self.moving = true;
    }

    /// `tick()` — turn toward the target at most 30 degrees, set the move speed,
    /// jump when the target is above and close.
    pub fn tick(&mut self, mob: &mut crate::mob::Mob) {
        mob.forward_speed = 0.0;
        if !self.moving {
            return;
        }
        self.moving = false;
        let feet = math::floor(mob.min_y() + 0.5) as f64;
        let dx = self.x - mob.x;
        let dz = self.z - mob.z;
        let dy = self.y - feet;
        let dist_sq = dx * dx + dy * dy + dz * dz;
        if !(dist_sq < 2.5000003E-7) {
            let target_yaw = (math::atan2(dz, dx) * 180.0 / std::f64::consts::PI) as f32 - 90.0;
            mob.yaw = clamp_and_wrap_angle(mob.yaw, target_yaw, 30.0);
            mob.forward_speed = (self.speed * mob_kind_speed(mob)) as f32;
            if dy > 0.0 && dx * dx + dz * dz < 1.0 {
                mob.jump_control.set_active();
            }
        }
    }
}

impl Default for MovementControl {
    fn default() -> Self {
        Self::new()
    }
}

/// The mob's `MOVEMENT_SPEED` attribute value (base times nothing; the port keeps
/// one speed per kind, with no modifier stack on the hot path).
fn mob_kind_speed(mob: &crate::mob::Mob) -> f64 {
    mob.kind.movement_speed()
}

/// `LookControl`: where the head should point.
pub struct LookControl {
    yaw_speed: f32,
    pitch_speed: f32,
    active: bool,
    look_x: f64,
    look_y: f64,
    look_z: f64,
}

impl LookControl {
    pub fn new() -> Self {
        LookControl {
            yaw_speed: 0.0,
            pitch_speed: 0.0,
            active: false,
            look_x: 0.0,
            look_y: 0.0,
            look_z: 0.0,
        }
    }

    /// `setLookatValues(entity, yaw, pitch)`.
    pub fn set_look_at(&mut self, x: f64, y: f64, z: f64, yaw_speed: f32, pitch_speed: f32) {
        self.look_x = x;
        self.look_y = y;
        self.look_z = z;
        self.yaw_speed = yaw_speed;
        self.pitch_speed = pitch_speed;
        self.active = true;
    }

    /// `tick()` — turn head/pitch toward the requested point, or realign to the body.
    pub fn tick(&mut self, mob: &mut crate::mob::Mob) {
        mob.pitch = 0.0;
        if self.active {
            self.active = false;
            let dx = self.look_x - mob.x;
            let dy = self.look_y - (mob.y + mob.eye_height());
            let dz = self.look_z - mob.z;
            let horiz = math::sqrt_f64(dx * dx + dz * dz);
            let target_yaw = (math::atan2(dz, dx) * 180.0 / std::f64::consts::PI) as f32 - 90.0;
            let target_pitch = (-(math::atan2(dy, horiz) * 180.0 / std::f64::consts::PI)) as f32;
            mob.pitch = clamp_and_wrap_angle(mob.pitch, target_pitch, self.pitch_speed);
            mob.head_yaw = clamp_and_wrap_angle(mob.head_yaw, target_yaw, self.yaw_speed);
        } else {
            mob.head_yaw = clamp_and_wrap_angle(mob.head_yaw, mob.yaw, 10.0);
        }

        let wrap = math::wrap_degrees(mob.head_yaw - mob.yaw);
        if !mob.navigation.is_done() {
            if wrap < -75.0 {
                mob.head_yaw = mob.yaw - 75.0;
            }

            if wrap > 75.0 {
                mob.head_yaw = mob.yaw + 75.0;
            }
        }
    }
}

impl Default for LookControl {
    fn default() -> Self {
        Self::new()
    }
}

/// `JumpControl`: a one-tick jump request.
#[derive(Default)]
pub struct JumpControl {
    active: bool,
}

impl JumpControl {
    pub fn new() -> Self {
        JumpControl { active: false }
    }

    /// `setActive()`.
    pub fn set_active(&mut self) {
        self.active = true;
    }

    /// Whether a jump request is pending.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// `tick()` — copies the request into `mob.jumping` and clears it.
    pub fn tick(&mut self, mob: &mut crate::mob::Mob) {
        mob.jumping = self.active;
        self.active = false;
    }
}

/// `MobEntity.changeAngle` / `LookControl.clampAndWrapAngle` — the shared turn helper.
pub fn clamp_and_wrap_angle(old: f32, new: f32, max_change: f32) -> f32 {
    let mut delta = math::wrap_degrees(new - old);
    if delta > max_change {
        delta = max_change;
    }

    if delta < -max_change {
        delta = -max_change;
    }

    old + delta
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::mob_at;
    use crate::mob::MobKind;

    #[test]
    fn movement_control_turns_toward_target() {
        let mut mob = mob_at(0.0, 1.0, 0.0);
        mob.kind = MobKind::Zombie;
        let mut control = MovementControl::new();
        // target due east (+x): yaw -90
        control.update(10.0, 1.0, 0.0, 1.0);
        control.tick(&mut mob);
        // one tick turns at most 30 degrees toward yaw -90
        assert!((mob.yaw - (-30.0)).abs() < 1e-4);
        assert!(mob.forward_speed > 0.0);
    }

    #[test]
    fn movement_control_caps_turn_at_30() {
        let mut mob = mob_at(0.0, 1.0, 0.0);
        mob.yaw = 0.0; // facing +z (south)
        let mut control = MovementControl::new();
        control.update(-10.0, 1.0, 0.0, 1.0); // due west: yaw -90 (or +270)
        control.tick(&mut mob);
        let turn = math::wrap_degrees(mob.yaw);
        assert!(
            (turn - 30.0).abs() < 1e-4,
            "turn capped at +30, got {}",
            turn
        );
    }

    #[test]
    fn movement_control_jumps_up_close_targets() {
        let mut mob = mob_at(0.0, 1.0, 0.0);
        mob.yaw = -90.0; // already facing +x
        let mut control = MovementControl::new();
        control.update(0.5, 2.5, 0.0, 1.0);
        control.tick(&mut mob);
        assert!(mob.jump_control.is_active(), "jump requested via control");
    }

    #[test]
    fn jump_control_is_one_tick() {
        let mut mob = mob_at(0.0, 1.0, 0.0);
        let mut jump = JumpControl::new();
        jump.set_active();
        jump.tick(&mut mob);
        assert!(mob.jumping);
        jump.tick(&mut mob);
        assert!(!mob.jumping);
    }

    #[test]
    fn look_control_turns_head() {
        let mut mob = mob_at(0.0, 1.0, 0.0);
        let mut look = LookControl::new();
        look.set_look_at(10.0, 2.0, 0.0, 40.0, 40.0);
        look.tick(&mut mob);
        // one tick turns at most 40 degrees toward yaw -90
        assert!((mob.head_yaw - (-40.0)).abs() < 1e-4);
    }

    #[test]
    fn clamp_and_wrap_angle_wraps() {
        // -170 - 170 = -340 → wraps to +20, capped at 50 → 170 + 20 = 190
        assert!((clamp_and_wrap_angle(170.0, -170.0, 50.0) - 190.0).abs() < 1e-4);
        // plain clamp
        assert_eq!(clamp_and_wrap_angle(0.0, 100.0, 30.0), 30.0);
        assert_eq!(clamp_and_wrap_angle(0.0, -100.0, 30.0), -30.0);
    }
}
