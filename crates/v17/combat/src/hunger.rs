//! `net.minecraft.entity.living.player.HungerManager` (Java name `FoodStats`):
//! the food bar, saturation, the exhaustion pool and the regen/starve timers.
//!
//! 1.7.10 rules: exhaustion past 4.0 spends one saturation point (else one
//! food point on non-peaceful). At food >= 18 the player heals one heart
//! every 80 ticks and pays 3.0 exhaustion; at food 0 they starve on the same
//! timer down to a difficulty-dependent floor. Peaceful regenerates on a
//! separate timer in `PlayerEntity.mobTick` and ignores food entirely.

use crate::damage::Difficulty;

/// `HungerManager`: the four saved values plus the regen timer.
#[derive(Clone, Debug)]
pub struct HungerManager {
    /// `foodLevel`, capped at 20.
    pub food_level: i32,
    /// `saturationLevel`, never above `food_level`.
    pub saturation_level: f32,
    /// `exhaustion`, the accumulator.
    pub exhaustion: f32,
    /// `starvationTimer` (NBT `foodTickTimer`).
    pub starvation_timer: i32,
    /// `lastFoodLevel`, read by the HUD's shank wiggle.
    pub last_food_level: i32,
}

impl Default for HungerManager {
    fn default() -> Self {
        Self::new()
    }
}

impl HungerManager {
    /// Fresh spawn: 20 food and the 5.0 starting saturation.
    pub fn new() -> Self {
        Self {
            food_level: 20,
            saturation_level: 5.0,
            exhaustion: 0.0,
            starvation_timer: 0,
            last_food_level: 20,
        }
    }

    /// `HungerManager.add`: food caps at 20; saturation gains
    /// `food * saturationModifier * 2.0` and caps at the new food level.
    pub fn add(&mut self, food: i32, saturation_modifier: f32) {
        self.food_level = (food + self.food_level).min(20);
        self.saturation_level = (self.saturation_level + food as f32 * saturation_modifier * 2.0)
            .min(self.food_level as f32);
    }

    /// `HungerManager.addExhaustion`, capped at 40.
    pub fn add_exhaustion(&mut self, exhaustion: f32) {
        self.exhaustion = (self.exhaustion + exhaustion).min(40.0);
    }

    /// One `HungerManager.tick(player)` against a plain survival player:
    /// healing lands as a heal call on `player`; the starve timer reports
    /// through the outcome so the caller deals the damage through its own
    /// pipeline (starvation draws no attacker roll, but the hurt-flash draw
    /// must come from the player's Random).
    pub fn tick(
        &mut self,
        player: &mut impl HungerPlayer,
        difficulty: Difficulty,
        natural_regen: bool,
    ) -> HungerTickOutcome {
        let mut outcome = HungerTickOutcome::default();
        self.last_food_level = self.food_level;
        if self.exhaustion > 4.0 {
            self.exhaustion -= 4.0;
            if self.saturation_level > 0.0 {
                self.saturation_level = (self.saturation_level - 1.0).max(0.0);
            } else if difficulty != Difficulty::Peaceful {
                self.food_level = (self.food_level - 1).max(0);
                outcome.food_lost = true;
            }
        }

        if natural_regen && self.food_level >= 18 && player.needs_healing() {
            self.starvation_timer += 1;
            if self.starvation_timer >= 80 {
                player.heal(1.0);
                self.add_exhaustion(3.0);
                self.starvation_timer = 0;
                outcome.healed = true;
            }
        } else if self.food_level <= 0 {
            self.starvation_timer += 1;
            if self.starvation_timer >= 80 {
                // `player.getHealth() > 10 || HARD || > 1 && NORMAL`: the
                // starvation floor.
                let health = player.health();
                if health > 10.0
                    || difficulty == Difficulty::Hard
                    || health > 1.0 && difficulty == Difficulty::Normal
                {
                    outcome.starve_due = true;
                }

                self.starvation_timer = 0;
            }
        } else {
            self.starvation_timer = 0;
        }

        outcome
    }

    /// `HungerManager.needsFood`.
    pub fn needs_food(&self) -> bool {
        self.food_level < 20
    }
}

/// What one hunger tick did, for stats and sounds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HungerTickOutcome {
    /// A point of food was spent (saturation empty, non-peaceful).
    pub food_lost: bool,
    /// The regen timer fired: one heart healed, 3.0 exhaustion paid.
    pub healed: bool,
    /// The starve timer fired past the difficulty floor: the caller deals
    /// `sources::STARVE` 1.0 to the player.
    pub starve_due: bool,
}

/// The slice of `PlayerEntity` the hunger tick needs. The game's player
/// struct implements it by delegating to health and the damage pipeline.
pub trait HungerPlayer {
    fn health(&self) -> f32;
    fn max_health(&self) -> f32;
    fn needs_healing(&self) -> bool;
    fn heal(&mut self, amount: f32);
}

impl HungerPlayer for crate::living::Living {
    fn health(&self) -> f32 {
        self.health
    }

    fn max_health(&self) -> f32 {
        self.max_health
    }

    fn needs_healing(&self) -> bool {
        self.health > 0.0 && self.health < self.max_health
    }

    fn heal(&mut self, amount: f32) {
        crate::living::Living::heal(self, amount);
    }
}

/// `PlayerEntity.mobTick`'s peaceful regen: every 20 ticks
/// (`ticks % 20 * 12 == 0` is Java's `ticks % 20 == 0`) heal one heart while
/// hurt, no food cost.
pub fn peaceful_regen_due(ticks: i64) -> bool {
    ticks % 20 == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::living::Living;

    #[test]
    fn food_caps_at_20_and_saturation_at_food() {
        let mut hunger = HungerManager::new();
        // Steak: 8 hunger, 0.8 modifier → +12.8 saturation on the 5.0 start.
        hunger.add(8, 0.8);
        assert_eq!(hunger.food_level, 20);
        assert_eq!(hunger.saturation_level, 17.8);

        let mut partial = HungerManager::new();
        partial.food_level = 10;
        partial.saturation_level = 0.0;
        // Bread: 5 hunger, 0.6 → +6.0 saturation, capped at food 15.
        partial.add(5, 0.6);
        assert_eq!(partial.food_level, 15);
        assert_eq!(partial.saturation_level, 6.0);
    }

    #[test]
    fn exhaustion_spends_saturation_then_food() {
        let mut hunger = HungerManager::new();
        hunger.saturation_level = 2.0;
        hunger.add_exhaustion(4.5);
        let mut player = Living::new(20.0);
        let outcome = hunger.tick(&mut player, Difficulty::Normal, true);
        // One 4.0 chunk drained: saturation 2 → 1, 0.5 carries over.
        assert_eq!(hunger.saturation_level, 1.0);
        assert_eq!(hunger.exhaustion, 0.5);
        assert_eq!(hunger.food_level, 20);
        assert!(!outcome.food_lost);

        // Drain the last saturation point.
        hunger.add_exhaustion(4.0);
        let outcome = hunger.tick(&mut player, Difficulty::Normal, true);
        assert_eq!(hunger.saturation_level, 0.0);
        assert_eq!(hunger.food_level, 20);
        assert!(!outcome.food_lost);

        // With saturation empty, food starts dropping.
        hunger.add_exhaustion(4.0);
        let outcome = hunger.tick(&mut player, Difficulty::Normal, true);
        assert_eq!(hunger.food_level, 19);
        assert!(outcome.food_lost);
    }

    #[test]
    fn peaceful_never_drops_food() {
        let mut hunger = HungerManager::new();
        hunger.saturation_level = 0.0;
        hunger.add_exhaustion(8.0);
        let mut player = Living::new(20.0);
        hunger.tick(&mut player, Difficulty::Peaceful, true);
        assert_eq!(hunger.food_level, 20);
    }

    #[test]
    fn regen_fires_every_80_ticks_and_costs_exhaustion() {
        let mut hunger = HungerManager::new();
        hunger.food_level = 18;
        hunger.saturation_level = 0.0;
        let mut player = Living::new(20.0);
        player.set_health(18.0);
        for _ in 0..79 {
            let outcome = hunger.tick(&mut player, Difficulty::Normal, true);
            assert!(!outcome.healed);
        }

        let outcome = hunger.tick(&mut player, Difficulty::Normal, true);
        assert!(outcome.healed);
        assert_eq!(player.health, 19.0);
        assert_eq!(hunger.exhaustion, 3.0);
        assert_eq!(hunger.starvation_timer, 0);
    }

    #[test]
    fn regen_waits_below_18_food() {
        let mut hunger = HungerManager::new();
        hunger.food_level = 17;
        hunger.saturation_level = 0.0;
        let mut player = Living::new(20.0);
        player.set_health(10.0);
        for _ in 0..100 {
            hunger.tick(&mut player, Difficulty::Normal, true);
        }

        assert_eq!(player.health, 10.0);
        assert_eq!(hunger.starvation_timer, 0);
    }

    #[test]
    fn starvation_floors_by_difficulty() {
        use crate::damage::sources;
        use sunset_1_7_core::LegacyRandom;

        // Normal stops at 1 heart.
        let mut hunger = HungerManager::new();
        hunger.food_level = 0;
        hunger.saturation_level = 0.0;
        let mut player = Living::new(20.0);
        player.set_health(1.5);
        for _ in 0..80 {
            hunger.tick(&mut player, Difficulty::Normal, false);
        }
        assert_eq!(player.health, 1.5);

        // Hard starves to death: the caller deals STARVE when due, and the
        // victim's hurt-resistance timer ticks down each tick like baseTick.
        let mut hunger = HungerManager::new();
        hunger.food_level = 0;
        hunger.saturation_level = 0.0;
        let mut player = Living::new(20.0);
        let mut rng = LegacyRandom::new(3);
        for _ in 0..80 * 25 {
            if player.invulnerable_timer > 0 {
                player.invulnerable_timer -= 1;
            }

            if hunger.tick(&mut player, Difficulty::Hard, false).starve_due {
                let mut ctx = crate::living::DamageContext {
                    rng: &mut rng,
                    difficulty: Difficulty::Normal,
                    attacker_delta: None,
                    blocking: false,
                    creative: false,
                };
                player.take_damage(&sources::STARVE, 1.0, &mut ctx);
            }
        }
        assert_eq!(player.health, 0.0);

        // Easy stops at 10 hearts.
        let mut hunger = HungerManager::new();
        hunger.food_level = 0;
        hunger.saturation_level = 0.0;
        let mut player = Living::new(20.0);
        let mut rng = LegacyRandom::new(3);
        for _ in 0..80 * 15 {
            if player.invulnerable_timer > 0 {
                player.invulnerable_timer -= 1;
            }

            if hunger.tick(&mut player, Difficulty::Easy, false).starve_due {
                let mut ctx = crate::living::DamageContext {
                    rng: &mut rng,
                    difficulty: Difficulty::Normal,
                    attacker_delta: None,
                    blocking: false,
                    creative: false,
                };
                player.take_damage(&sources::STARVE, 1.0, &mut ctx);
            }
        }
        assert_eq!(player.health, 10.0);
    }

    #[test]
    fn peaceful_regen_runs_on_its_own_timer() {
        assert!(!peaceful_regen_due(19));
        assert!(peaceful_regen_due(20));
        assert!(peaceful_regen_due(40));
    }
}
