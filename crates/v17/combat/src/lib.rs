//! Minecraft 1.7.10 combat and survival stats: melee attack, the damage
//! pipeline (armor, enchantments, absorption, invulnerability window),
//! knockback, hunger and saturation regen, experience orbs and levels.
//!
//! Ported from 1.7.10 (`EntityPlayer.attack`, `EntityLivingBase.takeDamage` /
//! `applyPotionDamageCalculations`, `HungerManager` (`FoodStats`),
//! `ExperienceOrbEntity`, `EnchantmentHelper`, `ItemArmor`) so the numbers
//! match vanilla on the same inputs. Combat state lives in [`Living`], player
//! levels in [`PlayerXp`], hunger in [`HungerManager`]; all RNG goes through
//! `sunset_1_7_core`'s exact `java.util.Random`.

pub mod armor;
pub mod attack;
pub mod damage;
pub mod enchant;
pub mod hunger;
pub mod living;
pub mod xp;

pub use armor::{ArmorMaterial, ArmorPiece, ArmorSet};
pub use damage::{DamageSource, Difficulty};
pub use hunger::{HungerManager, HungerTickOutcome, peaceful_regen_due};
pub use living::{DamageContext, DamageResult, Effects, Living};
pub use xp::{ExperienceOrb, PlayerXp, next_level_experience};

/// What a target is made of: decides Sharpness/Smite/Bane of Arthropods
/// bonuses (`MobType`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MobType {
    Undefined,
    Undead,
    Arthropod,
}
