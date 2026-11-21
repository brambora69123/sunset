//! Player experience and experience orbs: the 1.7.10 level curve, the
//! `increaseXp` progress carry, death drops, and `ExperienceOrbEntity`'s
//! rounding table.
//!
//! The curve is the 1.7.10 staircase: 17 per level to 15, then +3 per level
//! to 30, then +7 per level. Orbs round to the canonical sizes {1, 3, 7, 17,
//! 37, 73, 149, 307, 617, 1237, 2477}.

use sunset_1_7_core::{LegacyRandom, RandomSource};

/// `PlayerEntity.getNextLevelExperience`: the 1.7.10 curve.
pub fn next_level_experience(level: i32) -> i32 {
    if level >= 30 {
        62 + (level - 30) * 7
    } else if level >= 15 {
        17 + (level - 15) * 3
    } else {
        17
    }
}

/// `ExperienceOrbEntity.roundSize`: the orb sizes the world spawns.
pub fn round_orb_size(size: i32) -> i32 {
    if size >= 2477 {
        2477
    } else if size >= 1237 {
        1237
    } else if size >= 617 {
        617
    } else if size >= 307 {
        307
    } else if size >= 149 {
        149
    } else if size >= 73 {
        73
    } else if size >= 37 {
        37
    } else if size >= 17 {
        17
    } else if size >= 7 {
        7
    } else if size >= 3 {
        3
    } else {
        1
    }
}

/// `LivingEntity.tickPostDeath`'s split: the xp value becomes orbs, largest
/// size first, until nothing is left.
pub fn split_into_orbs(value: i32) -> Vec<i32> {
    let mut left = value;
    let mut orbs = Vec::new();
    while left > 0 {
        let size = round_orb_size(left);
        left -= size;
        orbs.push(size);
    }

    orbs
}

/// `PlayerEntity.xpLevel/xp/xpProgress` with the carry logic. The caller
/// keeps `ticks` in step with the world tick for the sound throttle.
#[derive(Clone, Debug, Default)]
pub struct PlayerXp {
    /// `xpLevel`.
    pub level: i32,
    /// `xpProgress`, `[0, 1)`.
    pub progress: f32,
    /// `xp`: the lifetime total (NBT `XpTotal`).
    pub total: i32,
    /// `xpCooldown`: blocks orb pickup for 2 ticks after one.
    pub cooldown: i32,
    /// `this.ticks`, set by the caller; the sound throttle reads it.
    pub ticks: i64,
    /// `lastXpSoundTime`, for the level-up sound throttle.
    pub last_sound_time: i64,
}

/// The throttled level-up sound (`random.levelup`, volume by level).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LevelUpSound {
    pub volume: f32,
}

impl PlayerXp {
    /// `PlayerEntity.increaseXp`: xp flows into progress; each full bar
    /// levels up and the remainder re-scales against the next level's cost.
    pub fn increase_xp(&mut self, points: i32) {
        let room = i32::MAX - self.total;
        let points = points.min(room);
        self.progress += points as f32 / next_level_experience(self.level) as f32;
        self.total += points;
        while self.progress >= 1.0 {
            self.progress = (self.progress - 1.0) * next_level_experience(self.level) as f32;
            self.add_levels(1);
            self.progress /= next_level_experience(self.level) as f32;
        }
    }

    /// `PlayerEntity.addXp`: levels move, negative totals reset to zero, and
    /// every 5 levels past 100 ticks since the last one plays the level-up
    /// sound.
    pub fn add_levels(&mut self, levels: i32) -> Option<LevelUpSound> {
        self.level += levels;
        if self.level < 0 {
            // `addXp(-3)` at level 1 drains the whole player, as in vanilla.
            self.level = 0;
            self.progress = 0.0;
            self.total = 0;
            return None;
        }

        if levels > 0 && self.level % 5 == 0 && (self.ticks - self.last_sound_time) as f32 > 100.0 {
            let volume = if self.level > 30 {
                1.0
            } else {
                self.level as f32 / 30.0
            };
            self.last_sound_time = self.ticks;
            return Some(LevelUpSound { volume });
        }

        None
    }

    /// `PlayerEntity.getXpDrop`: 7 per level, capped at 100; none with
    /// keepInventory.
    pub fn death_drop(&self, keep_inventory: bool) -> i32 {
        if keep_inventory {
            return 0;
        }

        (self.level * 7).min(100)
    }
}

/// The xp a killed player drops and how it splits into orbs
/// (`PlayerEntity.getXpDrop` + `LivingEntity.tickPostDeath`'s loop).
pub fn player_death_orbs(level: i32, keep_inventory: bool) -> Vec<i32> {
    if keep_inventory {
        return Vec::new();
    }

    split_into_orbs((level * 7).min(100))
}

/// The orb pickup, `ExperienceOrbEntity.onPlayerCollision`: a 2-tick
/// cooldown, the `random.orb` pitch roll and the xp landing. The orb's own
/// `pickupDelay` check stays with the caller. Returns the value taken.
pub fn orb_pickup(player: &mut PlayerXp, orb_value: i32, rng: &mut LegacyRandom) -> Option<i32> {
    if player.cooldown != 0 {
        return None;
    }

    player.cooldown = 2;
    // `0.5F * ((nextFloat() - nextFloat()) * 0.7F + 1.8F)`: two draws, in
    // order, even though only the sound uses them.
    let _pitch = 0.5 * ((rng.next_float() - rng.next_float()) * 0.7 + 1.8);
    player.increase_xp(orb_value);
    Some(orb_value)
}

/// `ExperienceOrbEntity`: the orb's xp value, age, health and the velocity
/// rules the physics crate drives (position integration stays with the
/// world). The floats widen to doubles exactly as the 1.7.10 bytecode did.
#[derive(Clone, Debug)]
pub struct ExperienceOrb {
    /// `xp` (NBT `Value`).
    pub xp: i32,
    /// `age`: removal at 6000 ticks.
    pub age: i32,
    /// `pickupDelay`.
    pub pickup_delay: i32,
    /// `health`, 5: fire and explosions chip it.
    pub health: i32,
    pub position: [f64; 3],
    pub velocity: [f64; 3],
    pub removed: bool,
}

impl ExperienceOrb {
    /// The constructor's random scatter (`Math.random` in vanilla; the exact
    /// Random here, so a seeded probe is deterministic).
    pub fn new(position: [f64; 3], xp: i32, rng: &mut LegacyRandom) -> Self {
        Self {
            xp,
            age: 0,
            pickup_delay: 0,
            health: 5,
            position,
            velocity: [
                f64::from((rng.next_float() * 0.2 - 0.1) * 2.0),
                f64::from(rng.next_float() * 0.2 * 2.0),
                f64::from((rng.next_float() * 0.2 - 0.1) * 2.0),
            ],
            removed: false,
        }
    }

    /// Gravity (`-0.03`) and the lava bounce, before the world moves the orb.
    pub fn pre_move(&mut self, in_lava: bool, rng: &mut LegacyRandom) {
        self.velocity[1] -= f64::from(0.03f32);
        if in_lava {
            self.velocity[1] = f64::from(0.2f32);
            self.velocity[0] = f64::from((rng.next_float() - rng.next_float()) * 0.2);
            self.velocity[2] = f64::from((rng.next_float() - rng.next_float()) * 0.2);
        }
    }

    /// Friction (`0.98`, the block's slipperiness below on the ground) and
    /// the ground bounce (`-0.9`), after the world moves the orb.
    pub fn post_move(&mut self, on_ground: bool, slipperiness: f32) {
        let drag = if on_ground { slipperiness * 0.98 } else { 0.98 };
        self.velocity[0] *= f64::from(drag);
        self.velocity[1] *= f64::from(0.98f32);
        self.velocity[2] *= f64::from(drag);
        if on_ground {
            self.velocity[1] *= f64::from(-0.9f32);
        }
    }

    /// The pull toward the nearest player within 8 blocks, added to velocity
    /// (`tick`'s igniter block; the caller refreshes the target every 20+
    /// ticks offset by the orb id).
    pub fn attraction(&self, player_pos: [f64; 3], player_eye: f64) -> [f64; 3] {
        let range = 8.0;
        let dx = (player_pos[0] - self.position[0]) / range;
        let dy = (player_pos[1] + player_eye - self.position[1]) / range;
        let dz = (player_pos[2] - self.position[2]) / range;
        let dist = (dx * dx + dy * dy + dz * dz).sqrt();
        let pull = 1.0 - dist;
        if pull > 0.0 {
            let pull = pull * pull;
            [
                dx / dist * pull * 0.1,
                dy / dist * pull * 0.1,
                dz / dist * pull * 0.1,
            ]
        } else {
            [0.0; 3]
        }
    }

    /// `tick`'s age counter: removal at 6000 ticks. Returns false when gone.
    pub fn age_tick(&mut self) -> bool {
        if self.removed {
            return false;
        }

        self.age += 1;
        if self.age >= 6000 {
            self.removed = true;
        }

        !self.removed
    }

    /// `ExperienceOrbEntity.takeDamage`: 5 health, fire and explosions chip
    /// it. Returns whether the orb popped.
    pub fn take_damage(&mut self, amount: f32) -> bool {
        if self.removed {
            return false;
        }

        self.health -= amount as i32;
        if self.health <= 0 {
            self.removed = true;
        }

        self.removed
    }

    /// `onPlayerCollision`: the 2-tick cooldown and the xp landing. The
    /// caller checks `pickupDelay == 0` and plays the sound. Returns whether
    /// the orb was taken.
    pub fn pickup(&mut self, player: &mut PlayerXp, rng: &mut LegacyRandom) -> bool {
        if self.removed || self.pickup_delay > 0 {
            return false;
        }

        if orb_pickup(player, self.xp, rng).is_some() {
            self.removed = true;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_curve_is_the_1_7_10_staircase() {
        assert_eq!(next_level_experience(0), 17);
        assert_eq!(next_level_experience(14), 17);
        assert_eq!(next_level_experience(15), 17);
        assert_eq!(next_level_experience(16), 20);
        assert_eq!(next_level_experience(29), 59);
        assert_eq!(next_level_experience(30), 62);
        assert_eq!(next_level_experience(31), 69);
        assert_eq!(next_level_experience(40), 132);
    }

    #[test]
    fn orbs_round_to_the_canonical_sizes() {
        assert_eq!(round_orb_size(1), 1);
        assert_eq!(round_orb_size(2), 1);
        assert_eq!(round_orb_size(3), 3);
        assert_eq!(round_orb_size(6), 3);
        assert_eq!(round_orb_size(7), 7);
        assert_eq!(round_orb_size(100), 73);
        assert_eq!(round_orb_size(3000), 2477);
    }

    #[test]
    fn a_death_split_never_loses_points() {
        let orbs = split_into_orbs(313);
        assert_eq!(orbs.iter().sum::<i32>(), 313);
        assert_eq!(split_into_orbs(1), vec![1]);
        let orbs = split_into_orbs(2477 * 3 + 5);
        assert_eq!(orbs.iter().sum::<i32>(), 2477 * 3 + 5);
    }

    #[test]
    fn increase_xp_carries_across_levels() {
        let mut xp = PlayerXp::default();
        xp.increase_xp(16);
        assert_eq!(xp.level, 0);
        assert_eq!(xp.total, 16);
        xp.increase_xp(1);
        assert_eq!(xp.level, 1);
        assert_eq!(xp.total, 17);
        assert_eq!(xp.progress, 0.0);
    }

    #[test]
    fn progress_rescales_at_the_new_level_cost() {
        // 34 points = level 2 with 0 progress (17 + 17).
        let mut xp = PlayerXp::default();
        xp.increase_xp(34);
        assert_eq!(xp.level, 2);
        assert_eq!(xp.progress, 0.0);

        // 30 points = level 1 with 13/17 progress at level 1, which is 17
        // wide: the remainder lands 13/17 into level 1.
        let mut xp = PlayerXp::default();
        xp.increase_xp(30);
        assert_eq!(xp.level, 1);
        assert_eq!(xp.total, 30);
    }

    #[test]
    fn death_drop_caps_at_100_and_keepinventory_zeroes_it() {
        let xp = PlayerXp {
            level: 40,
            ..Default::default()
        };
        assert_eq!(xp.death_drop(false), 100);
        assert_eq!(xp.death_drop(true), 0);
        let xp = PlayerXp {
            level: 3,
            ..Default::default()
        };
        assert_eq!(xp.death_drop(false), 21);
        // The orb split of a level 3 death: 17, 3, 1.
        assert_eq!(player_death_orbs(3, false), vec![17, 3, 1]);
    }

    #[test]
    fn orb_pickup_sets_the_cooldown() {
        let mut xp = PlayerXp::default();
        let mut rng = LegacyRandom::new(11);
        assert_eq!(orb_pickup(&mut xp, 5, &mut rng), Some(5));
        assert_eq!(xp.cooldown, 2);
        assert_eq!(xp.total, 5);
        // Cooldown blocks the next orb.
        assert_eq!(orb_pickup(&mut xp, 5, &mut rng), None);
        assert_eq!(xp.total, 5);
        // The caller ticks `cooldown` down.
        xp.cooldown -= 1;
        xp.cooldown -= 1;
        assert_eq!(orb_pickup(&mut xp, 3, &mut rng), Some(3));
        assert_eq!(xp.total, 8);
    }

    #[test]
    fn negative_levels_reset_the_player() {
        let mut xp = PlayerXp::default();
        xp.increase_xp(100);
        assert!(xp.level > 0);
        assert_eq!(xp.add_levels(-xp.level - 5), None);
        assert_eq!(xp.level, 0);
        assert_eq!(xp.progress, 0.0);
        assert_eq!(xp.total, 0);
    }

    #[test]
    fn level_up_sound_throttles_per_100_ticks() {
        let mut xp = PlayerXp {
            ticks: 1000,
            ..Default::default()
        };
        // Level 1→5 with 85 points: every 5th level plays, throttled.
        let mut sound = None;
        for _ in 0..85 {
            sound = xp.add_levels(1).or(sound);
        }
        assert!(
            sound.is_some(),
            "a 5th level past 100 ticks plays the sound"
        );

        // A second run within 100 ticks stays silent.
        xp.last_sound_time = xp.ticks;
        let mut sound = None;
        for _ in 0..85 {
            sound = xp.add_levels(1).or(sound);
        }
        assert!(sound.is_none(), "the throttle blocks the next fanfare");
    }
}
