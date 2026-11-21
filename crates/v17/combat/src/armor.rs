//! `net.minecraft.item.ArmorItem` and `PlayerInventory.damageArmor`: armor
//! points per material and slot, and the durability loss a hit deals.
//!
//! Slot indices follow `PlayerInventory.armor`: 0 boots, 1 leggings,
//! 2 chestplate, 3 helmet. `PlayerEntity.getEquipment(4)` reads slot 3, so a
//! falling anvil dents the helmet.

use sunset_1_7_core::{LegacyRandom, RandomSource};

use crate::enchant;

/// Protection and durability per material (`ArmorItem$Tier`). Slot indices
/// follow `PlayerInventory.armor` (boots first): 0 boots, 1 leggings,
/// 2 chestplate, 3 helmet — the tables below are flipped from vanilla's
/// helmet-first arrays. `PlayerEntity.getEquipment(4)` reads slot 3, so a
/// falling anvil dents the helmet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArmorMaterial {
    Cloth,
    Chain,
    Iron,
    Gold,
    Diamond,
}

impl ArmorMaterial {
    /// Vanilla's helmet-first `{1,3,2,1}`-style arrays, flipped to
    /// `{boots, leggings, chestplate, helmet}`.
    const PROTECTION: [[i32; 4]; 5] = [
        [1, 2, 3, 1], // Cloth
        [1, 4, 5, 2], // Chain
        [2, 5, 6, 2], // Iron
        [1, 3, 5, 2], // Gold
        [3, 6, 8, 3], // Diamond
    ];
    const DURABILITY: [i32; 5] = [5, 15, 15, 7, 33];
    /// Vanilla's helmet-first `{11,16,15,13}`, flipped to
    /// `{boots, leggings, chestplate, helmet}`.
    const BASE_DURABILITY: [i32; 4] = [13, 15, 16, 11];

    /// `ArmorItem$Tier.getProtection(slot)`.
    pub fn protection(self, slot: usize) -> i32 {
        Self::PROTECTION[self as usize][slot]
    }

    /// `ArmorItem$Tier.getDurability(slot)`: `base[slot] * multiplier`.
    pub fn durability(self, slot: usize) -> i32 {
        Self::BASE_DURABILITY[slot] * Self::DURABILITY[self as usize]
    }

    /// `ArmorItem$Tier.getEnchantability()`.
    pub fn enchantability(self) -> i32 {
        match self {
            Self::Cloth => 15,
            Self::Chain => 12,
            Self::Iron => 9,
            Self::Gold => 25,
            Self::Diamond => 10,
        }
    }
}

/// One worn armor piece: its material, slot and the damage it has taken
/// (`ItemStack.metadata`; it breaks when damage > max). `enchants` carries the
/// item's enchantments as `(id, level)` rows, read by Unbreaking here.
#[derive(Clone, Debug)]
pub struct ArmorPiece {
    pub material: ArmorMaterial,
    pub slot: usize,
    pub damage_taken: i32,
    pub enchants: Vec<(i16, i16)>,
}

impl ArmorPiece {
    pub fn new(material: ArmorMaterial, slot: usize) -> Self {
        Self {
            material,
            slot,
            damage_taken: 0,
            enchants: Vec::new(),
        }
    }

    pub fn max_damage(&self) -> i32 {
        self.material.durability(self.slot)
    }

    pub fn is_broken(&self) -> bool {
        self.damage_taken > self.max_damage()
    }

    /// The piece's level of an enchantment (`EnchantmentHelper.getLevel`).
    pub fn level(&self, id: i16) -> i32 {
        self.enchants
            .iter()
            .find(|(ench, _)| *ench == id)
            .map_or(0, |&(_, lvl)| i32::from(lvl))
    }
}

/// The four worn pieces. Empty slots are `None`; the damage stages read the
/// whole set for protection points.
#[derive(Clone, Debug, Default)]
pub struct ArmorSet {
    pub slots: [Option<ArmorPiece>; 4],
}

impl ArmorSet {
    /// `PlayerInventory.getArmorProtection`: the sum of the worn pieces'
    /// protection points. In vanilla only players lose armor durability
    /// (`LivingEntity.damageArmor` is empty for mobs).
    pub fn total_protection(&self) -> i32 {
        self.slots
            .iter()
            .flatten()
            .map(|piece| piece.material.protection(piece.slot))
            .sum()
    }

    /// `PlayerInventory.damageArmor`: each worn piece takes `damage / 4`
    /// damage (at least 1), so armor wears with every absorbed hit. Creative
    /// keeps durability (`ItemStack.takeDamageAndBreak` skips for creative).
    pub fn damage_all(&mut self, damage: f32, creative: bool, rng: &mut LegacyRandom) {
        if creative {
            return;
        }

        let mut damage = damage / 4.0;
        if damage < 1.0 {
            damage = 1.0;
        }

        let damage = damage as i32;
        for slot in 0..4 {
            let broken = match &mut self.slots[slot] {
                Some(piece) => piece.take_damage_and_break(damage, rng),
                None => false,
            };

            if broken {
                self.slots[slot] = None;
            }
        }
    }

    /// `PlayerEntity.getEquipment(4)`: the helmet.
    pub fn helmet(&self) -> Option<&ArmorPiece> {
        self.slots[3].as_ref()
    }
}

impl ArmorPiece {
    /// `ItemStack.takeDamage` + `takeDamageAndBreak`: per unit of damage,
    /// armor ignores the Unbreaking roll 60% of the time and otherwise the
    /// level absorbs it with probability `level / (level + 1)`. Returns
    /// whether the piece broke (the stack empties).
    pub(crate) fn take_damage_and_break(&mut self, amount: i32, rng: &mut LegacyRandom) -> bool {
        let unbreaking = self.level(enchant::ids::UNBREAKING);
        let mut absorbed = 0;
        if amount > 0 {
            for _ in 0..amount {
                if unbreaking > 0 && self.should_reduce_damage(unbreaking, rng) {
                    absorbed += 1;
                }
            }

            if amount - absorbed <= 0 {
                return false;
            }
        }

        self.damage_taken += amount - absorbed;
        self.damage_taken > self.max_damage()
    }

    /// `UnbreakingEnchantment.shouldReduceDamage`.
    fn should_reduce_damage(&self, level: i32, rng: &mut LegacyRandom) -> bool {
        if rng.next_float() < 0.6 {
            false
        } else {
            rng.next_int_bound(level + 1).unwrap_or(0) > 0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rng() -> LegacyRandom {
        LegacyRandom::new(17)
    }

    #[test]
    fn protection_totals_match_the_tier_tables() {
        let set = ArmorSet {
            slots: [
                Some(ArmorPiece::new(ArmorMaterial::Diamond, 0)),
                Some(ArmorPiece::new(ArmorMaterial::Diamond, 1)),
                Some(ArmorPiece::new(ArmorMaterial::Diamond, 2)),
                Some(ArmorPiece::new(ArmorMaterial::Diamond, 3)),
            ],
        };
        // Diamond full set: 3 + 6 + 8 + 3 = 20 points, the 1.7.10 cap.
        assert_eq!(set.total_protection(), 20);

        let leather = ArmorSet {
            slots: [
                Some(ArmorPiece::new(ArmorMaterial::Cloth, 0)),
                Some(ArmorPiece::new(ArmorMaterial::Cloth, 1)),
                Some(ArmorPiece::new(ArmorMaterial::Cloth, 2)),
                Some(ArmorPiece::new(ArmorMaterial::Cloth, 3)),
            ],
        };
        assert_eq!(leather.total_protection(), 7);
    }

    #[test]
    fn durability_matches_the_tier_tables() {
        // Diamond: helmet 11 * 33, boots 13 * 33, chestplate 16 * 33.
        let helmet = ArmorPiece::new(ArmorMaterial::Diamond, 3);
        let boots = ArmorPiece::new(ArmorMaterial::Diamond, 0);
        assert_eq!(helmet.max_damage(), 11 * 33);
        assert_eq!(boots.max_damage(), 13 * 33);
        assert_eq!(
            ArmorPiece::new(ArmorMaterial::Diamond, 2).max_damage(),
            16 * 33
        );
        // Gold enchantability 25, iron 9.
        assert_eq!(ArmorMaterial::Gold.enchantability(), 25);
        assert_eq!(ArmorMaterial::Iron.enchantability(), 9);
    }

    #[test]
    fn damage_all_breaks_and_clears_pieces() {
        let mut set = ArmorSet::default();
        set.slots[0] = Some(ArmorPiece::new(ArmorMaterial::Gold, 0)); // 13 * 7 durability
        set.damage_all(400.0, false, &mut rng());
        // 400 / 4 = 100 damage > 91 max: the boots broke and the slot emptied.
        assert!(set.slots[0].is_none());
    }

    #[test]
    fn small_hits_deal_at_least_one_armor_damage() {
        let mut set = ArmorSet::default();
        set.slots[3] = Some(ArmorPiece::new(ArmorMaterial::Diamond, 3));
        set.damage_all(0.5, false, &mut rng());
        assert_eq!(set.slots[3].as_ref().unwrap().damage_taken, 1);
    }

    #[test]
    fn unbreaking_absorbs_or_blocks_the_roll() {
        let mut piece = ArmorPiece::new(ArmorMaterial::Diamond, 3);
        piece.enchants = vec![(crate::enchant::ids::UNBREAKING, 3)];
        // Level 3: 3 of 4 units absorbed on average; a 0-damage unit adds none.
        piece.take_damage_and_break(0, &mut rng());
        assert_eq!(piece.damage_taken, 0);
    }
}
