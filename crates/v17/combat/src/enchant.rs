//! The combat enchantments of 1.7.10: their ids, the protection and damage
//! bonuses they add, and the random stages around them
//! (`EnchantmentHelper.modifyProtection`/`modifyDamage`, the
//! `ProtectionModifier`/`DamageModifier` callbacks, thorns).
//!
//! Protection and damage stack additively per item before the random stage,
//! exactly as in vanilla: every enchantment row on every piece contributes,
//! then the sum is split by a random roll.

use sunset_1_7_core::{LegacyRandom, RandomSource};

use crate::MobType;
use crate::armor::ArmorSet;
use crate::damage::DamageSource;

/// The 1.7.10 enchantment ids.
pub mod ids {
    pub const PROTECTION: i16 = 0;
    pub const FIRE_PROTECTION: i16 = 1;
    pub const FEATHER_FALLING: i16 = 2;
    pub const BLAST_PROTECTION: i16 = 3;
    pub const PROJECTILE_PROTECTION: i16 = 4;
    pub const RESPIRATION: i16 = 5;
    pub const AQUA_AFFINITY: i16 = 6;
    pub const THORNS: i16 = 7;
    pub const SHARPNESS: i16 = 16;
    pub const SMITE: i16 = 17;
    pub const BANE_OF_ARTHROPODS: i16 = 18;
    pub const KNOCKBACK: i16 = 19;
    pub const FIRE_ASPECT: i16 = 20;
    pub const LOOTING: i16 = 21;
    pub const EFFICIENCY: i16 = 32;
    pub const SILK_TOUCH: i16 = 33;
    pub const UNBREAKING: i16 = 34;
    pub const FORTUNE: i16 = 35;
    pub const POWER: i16 = 48;
    pub const PUNCH: i16 = 49;
    pub const FLAME: i16 = 50;
    pub const INFINITY: i16 = 51;
    pub const LUCK_OF_THE_SEA: i16 = 61;
    pub const LURE: i16 = 62;
}

/// `EnchantmentHelper.getLevel`: the first row matching `id` wins.
pub fn level(enchants: &[(i16, i16)], id: i16) -> i32 {
    enchants
        .iter()
        .find(|(ench, _)| *ench == id)
        .map_or(0, |&(_, lvl)| i32::from(lvl))
}

/// `ProtectionEnchantment.getExtraProtection`: the EPF row for a protection
/// enchantment against a source. `(6 + level²) / 3`, scaled per protection
/// type; 0 when the source does not match.
pub fn extra_protection(id: i16, level: i32, source: &DamageSource) -> i32 {
    if !(ids::PROTECTION..=ids::PROJECTILE_PROTECTION).contains(&id) {
        return 0;
    }

    if source.is_out_of_world() {
        return 0;
    }

    let base = (6 + level * level) as f32 / 3.0;
    match id {
        ids::PROTECTION => (base * 0.75).floor() as i32,
        ids::FIRE_PROTECTION if source.is_fire() => (base * 1.25).floor() as i32,
        ids::FEATHER_FALLING if source.name == "fall" => (base * 2.5).floor() as i32,
        ids::BLAST_PROTECTION if source.is_explosive() => (base * 1.5).floor() as i32,
        ids::PROJECTILE_PROTECTION if source.is_projectile() => (base * 1.5).floor() as i32,
        _ => 0,
    }
}

/// `DamageEnchantment.getExtraDamage`: Sharpness adds `1.25 * level` to
/// everything, Smite and Bane of Arthropods `2.5 * level` against their
/// targets.
pub fn extra_damage(id: i16, level: i32, mob_type: MobType) -> f32 {
    match id {
        ids::SHARPNESS => level as f32 * 1.25,
        ids::SMITE if mob_type == MobType::Undead => level as f32 * 2.5,
        ids::BANE_OF_ARTHROPODS if mob_type == MobType::Arthropod => level as f32 * 2.5,
        _ => 0.0,
    }
}

/// `EnchantmentHelper.modifyProtection`: every enchantment row on every piece
/// contributes its EPF (capped at 25), then the sum is randomly split to
/// `ceil(sum / 2) + uniform(floor(sum / 2))`. The draw happens even at sum 0,
/// as in vanilla.
pub fn modify_protection(armor: &ArmorSet, source: &DamageSource, rng: &mut LegacyRandom) -> i32 {
    let mut sum: i32 = armor
        .slots
        .iter()
        .flatten()
        .flat_map(|piece| piece.enchants.iter())
        .map(|&(id, lvl)| extra_protection(id, i32::from(lvl), source))
        .sum();

    if sum > 25 {
        sum = 25;
    }

    let split = (sum + 1) >> 1;
    let rest = rng.next_int_bound((sum >> 1) + 1).unwrap_or(0);
    split + rest
}

/// `EnchantmentHelper.modifyDamage`: the held weapon's damage bonus against a
/// target of `mob_type`.
pub fn modify_damage(weapon: &[(i16, i16)], mob_type: MobType) -> f32 {
    weapon
        .iter()
        .map(|&(id, lvl)| extra_damage(id, i32::from(lvl), mob_type))
        .sum()
}

/// `EnchantmentHelper.getKnockbackLevel`.
pub fn knockback_level(weapon: &[(i16, i16)]) -> i32 {
    level(weapon, ids::KNOCKBACK)
}

/// `EnchantmentHelper.getFireAspectLevel`.
pub fn fire_aspect_level(weapon: &[(i16, i16)]) -> i32 {
    level(weapon, ids::FIRE_ASPECT)
}

/// `EnchantmentHelper.applyProtectionWildcard` + `ThornsEnchantment`: thorns
/// on the victim's armor damages the attacker. Returns the thorns damage
/// dealt (0 when the roll fails) and the armor damage the thorns piece takes.
pub fn thorns(armor: &ArmorSet, attacker: u64, rng: &mut LegacyRandom) -> (DamageSource, i32, i32) {
    let thorns_level = armor
        .slots
        .iter()
        .flatten()
        .map(|piece| piece.level(ids::THORNS))
        .max()
        .unwrap_or(0);
    let source = crate::damage::sources::thorns(attacker);
    if thorns_level <= 0 {
        return (source, 0, 0);
    }

    if rng.next_float() < 0.15 * thorns_level as f32 {
        let damage = if thorns_level > 10 {
            thorns_level - 10
        } else {
            1 + rng.next_int_bound(4).unwrap_or(0)
        };

        (source, damage, 3)
    } else {
        (source, 0, 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::armor::ArmorMaterial;
    use crate::armor::ArmorPiece;
    use crate::damage::Difficulty;
    use crate::damage::sources;

    fn rng() -> LegacyRandom {
        LegacyRandom::new(42)
    }

    fn piece(enchants: &[(i16, i16)]) -> ArmorPiece {
        let mut p = ArmorPiece::new(ArmorMaterial::Diamond, 1);
        p.enchants = enchants.to_vec();
        p
    }

    #[test]
    fn sharpness_adds_per_level() {
        assert_eq!(extra_damage(ids::SHARPNESS, 1, MobType::Undefined), 1.25);
        assert_eq!(extra_damage(ids::SHARPNESS, 5, MobType::Undefined), 6.25);
        // Smite only against undead.
        assert_eq!(extra_damage(ids::SMITE, 5, MobType::Undefined), 0.0);
        assert_eq!(extra_damage(ids::SMITE, 5, MobType::Undead), 12.5);
        assert_eq!(
            extra_damage(ids::BANE_OF_ARTHROPODS, 3, MobType::Arthropod),
            7.5
        );
    }

    #[test]
    fn epf_rows_match_the_protection_types() {
        // Protection IV: (6 + 16) / 3 * 0.75 = 5.5 → 5.
        assert_eq!(extra_protection(ids::PROTECTION, 4, &sources::CACTUS), 5);
        // Fire Protection IV against fire: (6 + 16) / 3 * 1.25 = 9.166 → 9.
        assert_eq!(extra_protection(ids::FIRE_PROTECTION, 4, &sources::FIRE), 9);
        // Fire Protection IV against a non-fire source: 0.
        assert_eq!(
            extra_protection(ids::FIRE_PROTECTION, 4, &sources::CACTUS),
            0
        );
        // Feather Falling IV against fall: (6 + 16) / 3 * 2.5 = 18.333 → 18.
        assert_eq!(
            extra_protection(ids::FEATHER_FALLING, 4, &sources::FALL),
            18
        );
        // Nothing protects against the void.
        assert_eq!(
            extra_protection(ids::PROTECTION, 4, &sources::OUT_OF_WORLD),
            0
        );
    }

    #[test]
    fn modify_protection_caps_the_sum_at_25() {
        let mut armor = ArmorSet {
            slots: [
                Some(piece(&[(ids::PROTECTION, 4)])),
                Some(piece(&[(ids::PROTECTION, 4)])),
                Some(piece(&[(ids::PROTECTION, 4)])),
                Some(piece(&[(ids::PROTECTION, 4)])),
            ],
        };
        // Sum 20: (20 + 1 >> 1) + nextInt(11) → 10..=20.
        let value = modify_protection(&armor, &sources::CACTUS, &mut rng());
        assert!((10..=20).contains(&value), "got {value}");
        // Push the sum to 25 with a fifth-equivalent row: the split becomes
        // (25 + 1 >> 1) + nextInt(13) → 13..=25.
        armor.slots[3]
            .as_mut()
            .unwrap()
            .enchants
            .push((ids::PROTECTION, 4));
        let value = modify_protection(&armor, &sources::CACTUS, &mut rng());
        assert!((13..=25).contains(&value), "got {value}");
    }

    #[test]
    fn difficulty_scales_sources_flagged_for_it() {
        assert_eq!(Difficulty::Peaceful.scale_damage(5.0), 0.0);
        assert_eq!(Difficulty::Easy.scale_damage(5.0), 3.5);
        assert_eq!(Difficulty::Normal.scale_damage(5.0), 5.0);
        assert_eq!(Difficulty::Hard.scale_damage(5.0), 7.5);
    }
}
