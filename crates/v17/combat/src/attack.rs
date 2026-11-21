//! `PlayerEntity.attack`: the pre-1.9 melee swing. No attack cooldown exists:
//! every click deals full damage, so the state is pure input and the caller
//! does the world bits (the punchable check, particles, sounds, stats).
//!
//! Order: base damage from `ATTACK_DAMAGE`, the weapon's enchantment bonus
//! against the target's `MobType`, Knockback + sprinting, the falling crit
//! (`* 1.5` on base damage only), Fire Aspect arming, then the hit lands
//! through `Living::take_damage`. A sprint hit also drains the attacker.

use sunset_1_7_core::LegacyRandom;

use crate::MobType;
use crate::damage::DamageSource;
use crate::enchant;

/// `PlayerEntity.attack`'s `addFatigue(0.3F)` on a landed hit.
pub const HIT_EXHAUSTION: f32 = 0.3;

/// Everything the swing reads. `crit` is the composite vanilla condition:
/// `fallDistance > 0 && !onGround && !isClimbing && !isInWater &&
/// !hasStatusEffect(BLINDNESS) && vehicle == null && target is living`.
pub struct AttackInput<'a> {
    /// `getAttribute(ATTACK_DAMAGE)`, 1.0 for an unarmed player; the weapon's
    /// attribute modifiers land here (the inventory crate's job).
    pub base_damage: f32,
    /// The held item's enchantments as `(id, level)` rows.
    pub weapon_enchants: &'a [(i16, i16)],
    pub target_mob_type: MobType,
    /// `target instanceof LivingEntity`; tools hit other entities at 0.
    pub target_is_living: bool,
    /// `target.isOnFire()`: Fire Aspect does not re-arm a burning target.
    pub target_on_fire: bool,
    pub sprinting: bool,
    /// The composite crit condition above.
    pub crit_conditions: bool,
}

/// What the swing decides. The caller applies it: the hit through
/// `Living::take_damage`, the knockback velocity and Fire Aspect ticks on
/// success, the extinguish when the hit failed after arming.
#[derive(Clone, Copy, Debug)]
pub struct AttackOutcome {
    /// The damage to deal: base (crit ×1.5) + enchantment bonus.
    pub damage: f32,
    /// Knockback level + sprinting bonus, 0 when no knockback lands.
    pub knockback_level: i32,
    /// `var5`: crit particles and the ×1.5.
    pub crit: bool,
    /// The enchantment bonus landed (enchanted-crit particles).
    pub enchanted: bool,
    /// Fire Aspect armed the target: it was set on fire for 1 tick before the
    /// hit; on a failed hit the caller extinguishes it again.
    pub fire_armed: bool,
    /// Fire Aspect's burn on a landed hit: `level * 4` ticks.
    pub fire_ticks: Option<i32>,
    /// The damage passed 18.0: `Achievements.DEAL_OVERKILL_DAMAGE`.
    pub overkill: bool,
}

/// Compute the swing, `PlayerEntity.attack` up to `target.takeDamage`.
pub fn melee_attack(input: &AttackInput) -> AttackOutcome {
    let mut damage = input.base_damage;
    let mut knockback = enchant::knockback_level(input.weapon_enchants);
    let enchanted_damage = enchant::modify_damage(input.weapon_enchants, input.target_mob_type);
    if input.sprinting {
        knockback += 1;
    }

    let mut outcome = AttackOutcome {
        damage: 0.0,
        knockback_level: 0,
        crit: input.crit_conditions,
        enchanted: false,
        fire_armed: false,
        fire_ticks: None,
        overkill: false,
    };

    if damage <= 0.0 && enchanted_damage <= 0.0 {
        return outcome;
    }

    // The crit multiplies base damage only; the enchantment bonus adds after.
    if input.crit_conditions && damage > 0.0 {
        damage *= 1.5;
    }

    damage += enchanted_damage;
    outcome.enchanted = enchanted_damage > 0.0;

    let fire_aspect = enchant::fire_aspect_level(input.weapon_enchants);
    if input.target_is_living && fire_aspect > 0 && !input.target_on_fire {
        outcome.fire_armed = true; // `target.setOnFireFor(1)` before the hit
    }

    outcome.damage = damage;
    outcome.knockback_level = knockback;
    outcome.overkill = damage >= 18.0;
    if input.target_is_living && fire_aspect > 0 {
        outcome.fire_ticks = Some(fire_aspect * 4);
    }

    outcome
}

/// `PlayerEntity.attack`'s knockback: the target is pushed away along the
/// attacker's yaw (`-sin(yaw) * level * 0.5`, `0.1` up,
/// `cos(yaw) * level * 0.5`), added to its velocity.
pub fn attack_knockback_velocity(yaw: f32, knockback_level: i32) -> [f64; 3] {
    let angle = yaw as f64 * std::f64::consts::PI / 180.0;
    let yaw = angle as f32;
    [
        f64::from(-yaw.sin() * knockback_level as f32 * 0.5),
        0.1,
        f64::from(yaw.cos() * knockback_level as f32 * 0.5),
    ]
}

/// The attacker's slowdown after a sprint hit: `velocityX *= 0.6`,
/// `velocityZ *= 0.6`, sprinting off.
pub fn attacker_sprint_hit(velocity: &mut [f64; 3]) {
    velocity[0] *= 0.6;
    velocity[2] *= 0.6;
}

/// Build the melee damage source, `DamageSource.player(this)`.
pub fn melee_source(attacker: u64) -> DamageSource {
    DamageSource::attacker_source("player", attacker)
}

/// Roll the thorns counterattack the victim's armor can fire after a landed
/// hit (`EnchantmentHelper.applyProtectionWildcard`). Returns the damage the
/// attacker takes and the thorns piece's durability loss.
pub fn thorns_counter(
    victim_armor: &crate::armor::ArmorSet,
    attacker: u64,
    rng: &mut LegacyRandom,
) -> (DamageSource, i32, i32) {
    enchant::thorns(victim_armor, attacker, rng)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enchant::ids;

    fn input<'a>(enchants: &'a [(i16, i16)]) -> AttackInput<'a> {
        AttackInput {
            base_damage: 1.0,
            weapon_enchants: enchants,
            target_mob_type: MobType::Undefined,
            target_is_living: true,
            target_on_fire: false,
            sprinting: false,
            crit_conditions: false,
        }
    }

    #[test]
    fn bare_punch_deals_one() {
        let outcome = melee_attack(&input(&[]));
        assert_eq!(outcome.damage, 1.0);
        assert_eq!(outcome.knockback_level, 0);
        assert!(!outcome.crit);
        assert!(!outcome.overkill);
    }

    #[test]
    fn crit_multiplies_base_only() {
        let mut input = input(&[(ids::SHARPNESS, 3)]);
        input.crit_conditions = true;
        let outcome = melee_attack(&input);
        // 1.0 * 1.5 + 3.75 = 5.25: the bonus adds after the crit.
        assert_eq!(outcome.damage, 1.0 * 1.5 + 3.0 * 1.25);
        assert!(outcome.crit);
        assert!(outcome.enchanted);
    }

    #[test]
    fn sprint_adds_knockback_and_fire_aspect_burns() {
        let mut input = input(&[(ids::FIRE_ASPECT, 2)]);
        input.sprinting = true;
        let outcome = melee_attack(&input);
        // Sprint: 1 knockback level, Fire Aspect II burns 8 ticks.
        assert_eq!(outcome.knockback_level, 1);
        assert_eq!(outcome.fire_ticks, Some(8));
        assert!(outcome.fire_armed);

        // A burning target is not re-armed, but still burns on hit.
        input.target_on_fire = true;
        let outcome = melee_attack(&input);
        assert!(!outcome.fire_armed);
        assert_eq!(outcome.fire_ticks, Some(8));
    }

    #[test]
    fn zero_damage_swing_hits_nothing() {
        let mut base = input(&[]);
        base.base_damage = 0.0;
        let outcome = melee_attack(&base);
        assert_eq!(outcome.damage, 0.0);
        assert_eq!(outcome.knockback_level, 0);
    }

    #[test]
    fn overkill_passes_18() {
        let mut input = input(&[]);
        input.base_damage = 18.0;
        assert!(melee_attack(&input).overkill);
        input.base_damage = 17.0;
        assert!(!melee_attack(&input).overkill);
    }

    #[test]
    fn knockback_velocity_follows_the_yaw() {
        // Facing +x (yaw -90 in Minecraft): the target is pushed +x and up.
        let velocity = attack_knockback_velocity(-90.0, 1);
        assert_eq!(velocity[1], 0.1);
        assert!(velocity[0] > 0.0);
        assert!(velocity[2].abs() < 1.0e-6);
    }
}
