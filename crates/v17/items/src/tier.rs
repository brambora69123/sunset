//! Tool and armor material tiers, ported from `Item$Tier` and `ArmorItem$Tier`.

/// `Item$Tier`: the material of a tool or sword.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ToolTier {
    Wood,
    Stone,
    Iron,
    Diamond,
    Gold,
}

impl ToolTier {
    /// The mining level the tier reaches (`getLevel`).
    pub const fn level(self) -> i32 {
        match self {
            Self::Wood | Self::Gold => 0,
            Self::Stone => 1,
            Self::Iron => 2,
            Self::Diamond => 3,
        }
    }

    /// Uses before the tool breaks (`getDurability`).
    pub const fn durability(self) -> i32 {
        match self {
            Self::Wood => 59,
            Self::Stone => 131,
            Self::Iron => 250,
            Self::Diamond => 1561,
            Self::Gold => 32,
        }
    }

    /// Mining speed on the tier's effective blocks (`getMiningSpeed`).
    pub const fn mining_speed(self) -> f32 {
        match self {
            Self::Wood => 2.0,
            Self::Stone => 4.0,
            Self::Iron => 6.0,
            Self::Diamond => 8.0,
            Self::Gold => 12.0,
        }
    }

    /// Attack damage added on top of the tool's base (`getAttackDamage`).
    pub const fn attack_damage(self) -> f32 {
        match self {
            Self::Wood | Self::Gold => 0.0,
            Self::Stone => 1.0,
            Self::Iron => 2.0,
            Self::Diamond => 3.0,
        }
    }

    /// `getEnchantability`.
    pub const fn enchantability(self) -> i32 {
        match self {
            Self::Wood => 15,
            Self::Stone => 5,
            Self::Iron => 14,
            Self::Diamond => 10,
            Self::Gold => 22,
        }
    }

    /// The item that repairs this tier's tools, by registry name (`getRepairIngredient`).
    pub const fn repair_item_name(self) -> Option<&'static str> {
        match self {
            Self::Wood => Some("planks"),
            Self::Stone => Some("cobblestone"),
            Self::Iron => Some("iron_ingot"),
            Self::Gold => Some("gold_ingot"),
            Self::Diamond => Some("diamond"),
        }
    }
}

/// `ArmorItem$Tier`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ArmorTier {
    Cloth,
    Chain,
    Iron,
    Gold,
    Diamond,
}

/// `ArmorItem.BASE_DURABILITY`, by slot: helmet, chestplate, leggings, boots.
pub const ARMOR_BASE_DURABILITY: [i32; 4] = [11, 16, 15, 13];

impl ArmorTier {
    /// Multiplier on `ARMOR_BASE_DURABILITY` (`getDurability`).
    pub const fn durability(self) -> i32 {
        match self {
            Self::Cloth => 5,
            Self::Chain | Self::Iron => 15,
            Self::Gold => 7,
            Self::Diamond => 33,
        }
    }

    /// Armor points by slot: helmet, chestplate, leggings, boots (`getProtection`).
    pub const fn protection(self, slot: usize) -> i32 {
        let points = match self {
            Self::Cloth => [1, 3, 2, 1],
            Self::Chain => [2, 5, 4, 1],
            Self::Iron => [2, 6, 5, 2],
            Self::Gold => [2, 5, 3, 1],
            Self::Diamond => [3, 8, 6, 3],
        };
        points[slot]
    }

    /// `getEnchantability`.
    pub const fn enchantability(self) -> i32 {
        match self {
            Self::Cloth => 15,
            Self::Chain => 12,
            Self::Iron => 9,
            Self::Gold => 25,
            Self::Diamond => 10,
        }
    }

    /// The item that repairs this tier's armor, by registry name (`getRepairIngredient`).
    pub const fn repair_item_name(self) -> Option<&'static str> {
        match self {
            Self::Cloth => Some("leather"),
            Self::Chain | Self::Iron => Some("iron_ingot"),
            Self::Gold => Some("gold_ingot"),
            Self::Diamond => Some("diamond"),
        }
    }
}
