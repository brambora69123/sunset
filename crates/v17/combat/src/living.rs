//! `net.minecraft.entity.living.LivingEntity.takeDamage` /
//! `PlayerEntity.takeDamage` + `applyDamage`: the 1.7.10 damage pipeline.
//!
//! Order, exactly: invulnerability and fire-resistance gates, difficulty
//! scaling, the anvil helmet dent, the hurt-resistance window (only damage
//! above the last hit lands within 10 ticks), sword blocking (`(1 + d) / 2`),
//! armor (`d * (25 - points) / 25`), Resistance effect, enchantment
//! protection, absorption, hunger exhaustion and health. Knockback halves the
//! victim's velocity and pushes it away with 0.4 base, capped upward.

use sunset_1_7_core::{LegacyRandom, RandomSource};

use crate::armor::ArmorSet;
use crate::damage::{DEFAULT_EXHAUSTION, DamageSource, Difficulty};
use crate::enchant;

/// `LivingEntity.invulnerableTicks`: the hurt-resistance window, 20 ticks.
pub const INVULNERABLE_TICKS: i32 = 20;

/// The status-effect bits the damage pipeline reads. Resistances beyond these
/// stay with the effects crate.
#[derive(Clone, Debug, Default)]
pub struct Effects {
    /// `StatusEffect.RESISTANCE`: `(amplifier + 1) * 5` of 25 cut.
    pub resistance_amplifier: Option<i32>,
    /// `StatusEffect.FIRE_RESISTANCE`: fire sources refuse entirely.
    pub fire_resistance: bool,
}

/// The combat-relevant state of one living entity. Health, armor, absorption,
/// the hurt window and velocity; the game keeps the rest (position, effects,
/// hunger) beside it and feeds what the pipeline needs through the calls.
#[derive(Clone, Debug)]
pub struct Living {
    pub health: f32,
    pub max_health: f32,
    /// Absorption hearts (`PlayerEntity.setAbsorption`).
    pub absorption: f32,
    pub armor: ArmorSet,
    /// `EntityAttributes.KNOCKBACK_RESISTANCE`, 0.0 for players.
    pub knockback_resistance: f64,
    /// `invulnerableTimer`, ticked down by the caller while > 0.
    pub invulnerable_timer: i32,
    /// `lastDamageTaken` within the hurt window.
    pub last_damage_taken: f32,
    pub velocity: [f64; 3],
    pub yaw: f32,
    /// `damagedSwingDir`: where the hurt flash swings from.
    pub damaged_swing_dir: f32,
    pub effects: Effects,
    pub is_player: bool,
    /// `Entity.isInvulnerable`, set by commands.
    pub invulnerable: bool,
    /// `PlayerAbilities.invulnerable`: blocks everything but the void.
    pub abilities_invulnerable: bool,
}

impl Living {
    pub fn new(max_health: f32) -> Self {
        Self {
            health: max_health,
            max_health,
            absorption: 0.0,
            armor: ArmorSet::default(),
            knockback_resistance: 0.0,
            // `invulnerableTimer` starts at 0; `invulnerableTicks` (20) is
            // only the window's max.
            invulnerable_timer: 0,
            last_damage_taken: 0.0,
            velocity: [0.0; 3],
            yaw: 0.0,
            damaged_swing_dir: 0.0,
            effects: Effects::default(),
            is_player: false,
            invulnerable: false,
            abilities_invulnerable: false,
        }
    }

    /// `LivingEntity.setHealth`: clamped to `[0, max]`.
    pub fn set_health(&mut self, amount: f32) {
        self.health = amount.clamp(0.0, self.max_health);
    }

    pub fn is_alive(&self) -> bool {
        self.health > 0.0
    }

    /// `LivingEntity.heal`: only while alive.
    pub fn heal(&mut self, amount: f32) {
        if self.health > 0.0 {
            self.set_health(self.health + amount);
        }
    }

    pub fn set_absorption(&mut self, absorption: f32) {
        self.absorption = absorption.max(0.0);
    }

    /// `PlayerEntity.takeDamage` then `LivingEntity.takeDamage`, merged: the
    /// player path owns difficulty scaling and blocking, the living path owns
    /// the window and the reductions. Players lose armor durability; mobs do
    /// not (`LivingEntity.damageArmor` is empty for them).
    ///
    /// `ctx.attacker_delta` is the attacker's position minus the victim's; it
    /// drives the knockback direction and the hurt flash. Returns the applied
    /// flag, the health lost and the hunger cost to add.
    pub fn take_damage(
        &mut self,
        source: &DamageSource,
        amount: f32,
        ctx: &mut DamageContext,
    ) -> DamageResult {
        let none = DamageResult::default();
        if self.invulnerable {
            return none;
        }

        if self.is_player && self.abilities_invulnerable && !source.is_out_of_world() {
            return none;
        }

        if self.health <= 0.0 {
            return none;
        }

        if source.is_fire() && self.effects.fire_resistance {
            return none;
        }

        let mut amount = amount;
        if source.is_scaled_with_difficulty() {
            amount = ctx.difficulty.scale_damage(amount);
        }

        if amount == 0.0 {
            return none;
        }

        // (ANVIL || FALLING_BLOCK) with a helmet: the helmet dents
        // (`ItemStack.takeDamageAndBreak`, skipped in creative) and the hit
        // lands at 75%.
        if (source.name == "anvil" || source.name == "fallingBlock")
            && self.armor.helmet().is_some()
        {
            if !ctx.creative {
                let dent = (amount * 4.0 + ctx.rng.next_float() * amount * 2.0) as i32;
                let broke = self.armor.slots[3]
                    .as_mut()
                    .is_some_and(|helmet| helmet.take_damage_and_break(dent, ctx.rng));
                if broke {
                    self.armor.slots[3] = None;
                }
            }

            amount *= 0.75;
        }

        // The hurt window: within half the resistance time only damage above
        // the last hit lands.
        let mut full = true;
        let result;
        if self.invulnerable_timer > INVULNERABLE_TICKS / 2 {
            if amount <= self.last_damage_taken {
                return none;
            }

            result = self.apply_damage(source, amount - self.last_damage_taken, ctx);
            self.last_damage_taken = amount;
            full = false;
        } else {
            self.last_damage_taken = amount;
            self.invulnerable_timer = INVULNERABLE_TICKS;
            result = self.apply_damage(source, amount, ctx);
        }

        self.damaged_swing_dir = 0.0;
        if full {
            if let Some((mut dx, mut dz)) = ctx.attacker_delta {
                // A degenerate direction gets a tiny random push, as in
                // vanilla (`Math.random` there; the exact Random here).
                while dx * dx + dz * dz < 1.0e-4 {
                    dx = (ctx.rng.next_double() - ctx.rng.next_double()) * 0.01;
                    dz = (ctx.rng.next_double() - ctx.rng.next_double()) * 0.01;
                }

                // atan2 over the widened float PI, as vanilla casts it.
                let angle = dz.atan2(dx) * 180.0 / f64::from(std::f32::consts::PI);
                self.damaged_swing_dir = angle as f32 - self.yaw;
                self.apply_knockback(dx, dz, ctx.rng);
            } else {
                self.damaged_swing_dir = (ctx.rng.next_double() * 2.0) as i32 as f32 * 180.0;
            }
        }

        result
    }

    /// `PlayerEntity.applyDamage` (also `LivingEntity.applyDamage`; the two
    /// end at the same absorption state). Returns the health lost and the
    /// hunger cost.
    fn apply_damage(
        &mut self,
        source: &DamageSource,
        damage: f32,
        ctx: &mut DamageContext,
    ) -> DamageResult {
        let mut damage = damage;
        if !source.bypasses_armor() && ctx.blocking && damage > 0.0 {
            damage = (1.0 + damage) * 0.5;
        }

        damage = self.get_damage_after_armor(source, damage, ctx);
        damage = self.get_damage_after_effects(source, damage, ctx);

        let total = damage;
        damage = (damage - self.absorption).max(0.0);
        self.set_absorption(self.absorption - (total - damage));

        let mut result = DamageResult::default();
        if damage != 0.0 {
            result.exhaustion = source.get_exhaustion();
            self.set_health(self.health - damage);
            result.damage = damage;
        }

        result.applied = true;
        result
    }

    /// `LivingEntity.getDamageAfterArmor` + `PlayerInventory.damageArmor`.
    fn get_damage_after_armor(
        &mut self,
        source: &DamageSource,
        damage: f32,
        ctx: &mut DamageContext,
    ) -> f32 {
        if source.bypasses_armor() {
            return damage;
        }

        let points = self.armor.total_protection();
        let reduced = damage * (25 - points) as f32;
        if self.is_player {
            self.armor.damage_all(damage, ctx.creative, ctx.rng);
        }

        reduced / 25.0
    }

    /// `LivingEntity.getDamageAfterEffectsAndEnchantments`: the Resistance
    /// effect stage, then enchantment protection capped at 20.
    fn get_damage_after_effects(
        &mut self,
        source: &DamageSource,
        damage: f32,
        ctx: &mut DamageContext,
    ) -> f32 {
        if source.is_unblockable() {
            return damage;
        }

        if let Some(amplifier) = self.effects.resistance_amplifier
            && !source.is_out_of_world()
        {
            let cut = (amplifier + 1) * 5;
            return damage * (25 - cut) as f32 / 25.0;
        }

        if damage <= 0.0 {
            return 0.0;
        }

        let mut protection = enchant::modify_protection(&self.armor, source, ctx.rng);
        if protection > 20 {
            protection = 20;
        }

        if protection > 0 {
            return damage * (25 - protection) as f32 / 25.0;
        }

        damage
    }

    /// `LivingEntity.applyKnockback`: velocity halves, then 0.4 base away
    /// from the attacker and up, capped at 0.4. `KnockbackResistance` skips
    /// the knockback with probability equal to its value.
    pub fn apply_knockback(&mut self, dx: f64, dz: f64, rng: &mut LegacyRandom) {
        if rng.next_double() < self.knockback_resistance {
            return;
        }

        let len = (dx * dx + dz * dz).sqrt() as f32;
        const BASE: f32 = 0.4;
        self.velocity[0] /= 2.0;
        self.velocity[1] /= 2.0;
        self.velocity[2] /= 2.0;
        self.velocity[0] -= dx / f64::from(len) * f64::from(BASE);
        self.velocity[1] += f64::from(BASE);
        self.velocity[2] -= dz / f64::from(len) * f64::from(BASE);
        if self.velocity[1] > f64::from(BASE) {
            self.velocity[1] = f64::from(BASE);
        }
    }
}

/// What the pipeline reads from the world around one hit.
pub struct DamageContext<'a> {
    pub rng: &'a mut LegacyRandom,
    /// Difficulty scaling for the `scaledWithDifficulty` sources.
    pub difficulty: Difficulty,
    /// Attacker position minus victim position; `None` for world damage.
    pub attacker_delta: Option<(f64, f64)>,
    /// The victim is sword-blocking (players only in 1.7.10).
    pub blocking: bool,
    /// Creative mode: armor keeps its durability.
    pub creative: bool,
}

/// What one hit did.
#[derive(Clone, Debug, Default)]
pub struct DamageResult {
    /// `takeDamage`'s return: false when the hit was refused or scaled away.
    pub applied: bool,
    /// The health the pipeline took (0 when absorbed or refused).
    pub damage: f32,
    /// The hunger cost to add (`source.getExhaustion()`), 0 when refused.
    pub exhaustion: f32,
}

/// The default exhaustion of a melee hit, `PlayerEntity.attack`'s
/// `addFatigue(0.3F)`: the hit lands its hunger cost through the damage
/// source's exhaustion.
pub const MELEE_EXHAUSTION: f32 = DEFAULT_EXHAUSTION;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::armor::{ArmorMaterial, ArmorPiece};
    use crate::damage::sources;

    fn rng() -> LegacyRandom {
        LegacyRandom::new(7)
    }

    fn plain() -> Living {
        Living::new(20.0)
    }

    fn ctx(rng: &mut LegacyRandom) -> DamageContext<'_> {
        DamageContext {
            rng,
            difficulty: Difficulty::Normal,
            attacker_delta: None,
            blocking: false,
            creative: false,
        }
    }

    #[test]
    fn unarmored_generic_hit_lands_in_full() {
        let mut victim = plain();
        let mut rng = rng();
        let result = victim.take_damage(&sources::GENERIC, 5.0, &mut ctx(&mut rng));
        assert!(result.applied);
        // GENERIC bypasses armor, so 5.0 lands whole.
        assert_eq!(victim.health, 15.0);
        assert_eq!(result.damage, 5.0);
    }

    #[test]
    fn armor_reduces_by_points_over_25() {
        let mut victim = plain();
        victim.is_player = true;
        victim.armor.slots[2] = Some(ArmorPiece::new(ArmorMaterial::Diamond, 2)); // 8 points
        let mut rng = rng();
        let result = victim.take_damage(&sources::CACTUS, 5.0, &mut ctx(&mut rng));
        // 5 * (25 - 8) / 25 = 3.4, no enchantments.
        assert_eq!(victim.health, 20.0f32 - 5.0f32 * 17.0f32 / 25.0f32);
        assert_eq!(result.damage, 5.0f32 * 17.0f32 / 25.0f32);
        // Cactus carries the default 0.3 hunger cost.
        assert_eq!(result.exhaustion, 0.3);
        // The chestplate took 5 / 4 → 1 damage.
        assert_eq!(victim.armor.slots[2].as_ref().unwrap().damage_taken, 1);
    }

    #[test]
    fn fire_resistance_refuses_fire() {
        let mut victim = plain();
        victim.effects.fire_resistance = true;
        let mut rng = rng();
        let result = victim.take_damage(&sources::FIRE, 4.0, &mut ctx(&mut rng));
        assert!(!result.applied);
        assert_eq!(victim.health, 20.0);
    }

    #[test]
    fn blocking_halves_plus_one() {
        let mut victim = plain();
        let mut rng = rng();
        let mut context = ctx(&mut rng);
        context.blocking = true;
        let result = victim.take_damage(&sources::CACTUS, 4.0, &mut context);
        // (1 + 4) * 0.5 = 2.5 before armor; armor is empty.
        assert_eq!(victim.health, 17.5);
        assert_eq!(result.damage, 2.5);
        // Blocked hits still cost hunger (blocking is not unblockable).
        assert_eq!(result.exhaustion, 0.3);
    }

    #[test]
    fn absorption_eats_the_hit_first() {
        let mut victim = plain();
        victim.set_absorption(4.0);
        let mut rng = rng();
        victim.take_damage(&sources::GENERIC, 3.0, &mut ctx(&mut rng));
        assert_eq!(victim.health, 20.0);
        assert_eq!(victim.absorption, 1.0);
    }

    #[test]
    fn hurt_window_refuses_weaker_hits() {
        let mut victim = plain();
        let mut rng = rng();
        victim.take_damage(&sources::GENERIC, 5.0, &mut ctx(&mut rng));
        // Within 10 ticks a 4.0 hit is refused; the timer is full.
        assert!(victim.invulnerable_timer > INVULNERABLE_TICKS / 2);
        let refused = victim.take_damage(&sources::GENERIC, 4.0, &mut ctx(&mut rng));
        assert!(!refused.applied);
        assert_eq!(victim.health, 15.0);
        // A 7.0 hit lands only its difference (2.0).
        victim.take_damage(&sources::GENERIC, 7.0, &mut ctx(&mut rng));
        assert_eq!(victim.health, 13.0);
    }

    #[test]
    fn knockback_halves_and_pushes_up() {
        let mut victim = plain();
        let mut rng = rng();
        victim.velocity = [0.0, 0.0, 0.0];
        victim.apply_knockback(4.0, 0.0, &mut rng);
        // velocity / 2 then 0.4 base away: x = -0.4 (as float), y = 0.4.
        assert_eq!(victim.velocity[0], f64::from(-0.4f32));
        assert_eq!(victim.velocity[1], f64::from(0.4f32));
        assert_eq!(victim.velocity[2], 0.0);

        // A rising victim caps at 0.4.
        let mut jumper = plain();
        jumper.velocity = [0.0, 0.6, 0.0];
        jumper.apply_knockback(0.0, 0.0, &mut rng);
        assert_eq!(jumper.velocity[1], f64::from(0.4f32));
    }

    #[test]
    fn full_knockback_resistance_stops_the_push() {
        let mut victim = plain();
        victim.knockback_resistance = 1.0;
        let mut rng = rng();
        victim.apply_knockback(4.0, 0.0, &mut rng);
        assert_eq!(victim.velocity, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn creative_armor_keeps_durability() {
        let mut victim = plain();
        victim.is_player = true;
        victim.armor.slots[1] = Some(ArmorPiece::new(ArmorMaterial::Diamond, 1));
        let mut rng = rng();
        let mut context = ctx(&mut rng);
        context.creative = true;
        victim.take_damage(&sources::CACTUS, 5.0, &mut context);
        assert_eq!(victim.armor.slots[1].as_ref().unwrap().damage_taken, 0);
    }

    #[test]
    fn difficulty_scales_melee_damage() {
        let mut victim = plain();
        let mut rng = rng();
        let mut context = ctx(&mut rng);
        context.difficulty = Difficulty::Hard;
        // Explosions scale with difficulty; cactus hits do not.
        victim.take_damage(&sources::explosion(), 10.0, &mut context);
        // Hard: 10 * 3 / 2 = 15.0.
        assert_eq!(victim.health, 5.0);
    }

    #[test]
    fn peaceful_scales_to_nothing() {
        let mut victim = plain();
        let mut rng = rng();
        let mut context = ctx(&mut rng);
        context.difficulty = Difficulty::Peaceful;
        let result = victim.take_damage(&sources::explosion(), 10.0, &mut context);
        assert!(!result.applied);
        assert_eq!(victim.health, 20.0);
    }
}
