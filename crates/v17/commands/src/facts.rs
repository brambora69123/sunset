//! The 1.7.10 facts commands read: game modes, difficulties, the status
//! effect table, the enchantment table with its item categories, and the
//! entity id registry `/summon` creates from.

/// `WorldSettings$GameMode`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameMode {
    NotSet,
    Survival,
    Creative,
    Adventure,
}

impl GameMode {
    /// `getId`.
    pub fn id(self) -> i32 {
        match self {
            Self::NotSet => -1,
            Self::Survival => 0,
            Self::Creative => 1,
            Self::Adventure => 2,
        }
    }

    /// `getKey`: the `gameMode.<key>` suffix (`""` for `NOT_SET`).
    pub fn key(self) -> &'static str {
        match self {
            Self::NotSet => "",
            Self::Survival => "survival",
            Self::Creative => "creative",
            Self::Adventure => "adventure",
        }
    }

    /// `byId`: an exact id match, else `SURVIVAL`.
    pub fn by_id(id: i32) -> Self {
        match id {
            -1 => Self::NotSet,
            0 => Self::Survival,
            1 => Self::Creative,
            2 => Self::Adventure,
            _ => Self::Survival,
        }
    }

    /// `restrictsWorldModification`.
    pub fn restricts_world_modification(self) -> bool {
        self == Self::Adventure
    }

    /// `isCreative`.
    pub fn is_creative(self) -> bool {
        self == Self::Creative
    }
}

/// `net.minecraft.world.Difficulty`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Difficulty {
    Peaceful,
    Easy,
    Normal,
    Hard,
}

impl Difficulty {
    /// `getId`.
    pub fn id(self) -> i32 {
        match self {
            Self::Peaceful => 0,
            Self::Easy => 1,
            Self::Normal => 2,
            Self::Hard => 3,
        }
    }

    /// `getKey`: the `options.difficulty.<key>` suffix.
    pub fn key(self) -> &'static str {
        match self {
            Self::Peaceful => "peaceful",
            Self::Easy => "easy",
            Self::Normal => "normal",
            Self::Hard => "hard",
        }
    }

    /// `byId`: Java's `BY_ID[id % BY_ID.length]` wrap-around.
    pub fn by_id(id: i32) -> Self {
        let id = id.rem_euclid(4);
        match id {
            0 => Self::Peaceful,
            1 => Self::Easy,
            2 => Self::Normal,
            _ => Self::Hard,
        }
    }
}

/// One status effect as `/effect` reads it: the id, its translation key
/// suffix, and whether it is instant (`InstantStatusEffect`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StatusEffect {
    pub id: i32,
    /// The `potion.<key>` suffix (`StatusEffect.getTranslationKey`).
    pub key: &'static str,
    pub instant: bool,
}

/// `StatusEffect.BY_ID`: 32 slots; ids 1-23 registered, 6, 7 and 23 instant.
pub fn status_effect_by_id(id: i32) -> Option<&'static StatusEffect> {
    const TABLE: [Option<StatusEffect>; 32] = {
        let mut table: [Option<StatusEffect>; 32] = [None; 32];
        table[1] = Some(StatusEffect {
            id: 1,
            key: "moveSpeed",
            instant: false,
        });
        table[2] = Some(StatusEffect {
            id: 2,
            key: "moveSlowdown",
            instant: false,
        });
        table[3] = Some(StatusEffect {
            id: 3,
            key: "digSpeed",
            instant: false,
        });
        table[4] = Some(StatusEffect {
            id: 4,
            key: "digSlowDown",
            instant: false,
        });
        table[5] = Some(StatusEffect {
            id: 5,
            key: "damageBoost",
            instant: false,
        });
        table[6] = Some(StatusEffect {
            id: 6,
            key: "heal",
            instant: true,
        });
        table[7] = Some(StatusEffect {
            id: 7,
            key: "harm",
            instant: true,
        });
        table[8] = Some(StatusEffect {
            id: 8,
            key: "jump",
            instant: false,
        });
        table[9] = Some(StatusEffect {
            id: 9,
            key: "confusion",
            instant: false,
        });
        table[10] = Some(StatusEffect {
            id: 10,
            key: "regeneration",
            instant: false,
        });
        table[11] = Some(StatusEffect {
            id: 11,
            key: "resistance",
            instant: false,
        });
        table[12] = Some(StatusEffect {
            id: 12,
            key: "fireResistance",
            instant: false,
        });
        table[13] = Some(StatusEffect {
            id: 13,
            key: "waterBreathing",
            instant: false,
        });
        table[14] = Some(StatusEffect {
            id: 14,
            key: "invisibility",
            instant: false,
        });
        table[15] = Some(StatusEffect {
            id: 15,
            key: "blindness",
            instant: false,
        });
        table[16] = Some(StatusEffect {
            id: 16,
            key: "nightVision",
            instant: false,
        });
        table[17] = Some(StatusEffect {
            id: 17,
            key: "hunger",
            instant: false,
        });
        table[18] = Some(StatusEffect {
            id: 18,
            key: "weakness",
            instant: false,
        });
        table[19] = Some(StatusEffect {
            id: 19,
            key: "poison",
            instant: false,
        });
        table[20] = Some(StatusEffect {
            id: 20,
            key: "wither",
            instant: false,
        });
        table[21] = Some(StatusEffect {
            id: 21,
            key: "healthBoost",
            instant: false,
        });
        table[22] = Some(StatusEffect {
            id: 22,
            key: "absorption",
            instant: false,
        });
        table[23] = Some(StatusEffect {
            id: 23,
            key: "saturation",
            instant: true,
        });
        table
    };
    if (0..32).contains(&id) {
        TABLE[id as usize].as_ref()
    } else {
        None
    }
}

/// `EnchantmentCategory.canEnchant(item)` over the 1.7.10 item ids.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnchantmentCategory {
    All,
    Armor,
    ArmorFeet,
    ArmorLegs,
    ArmorTorso,
    ArmorHead,
    Weapon,
    Digger,
    FishingRod,
    Breakable,
    Bow,
}

/// Leather, chain, iron, diamond and gold helmets.
const HELMETS: [i32; 5] = [298, 302, 306, 310, 314];
/// Chestplates.
const CHESTPLATES: [i32; 5] = [299, 303, 307, 311, 315];
/// Leggings.
const LEGGINGS: [i32; 5] = [300, 304, 308, 312, 316];
/// Wood, stone, iron, diamond and gold swords.
const SWORDS: [i32; 5] = [268, 272, 267, 276, 283];
/// `ToolItem`: pickaxes, axes and shovels (`ItemHoe` is not a `ToolItem`).
const TOOLS: [i32; 15] = [
    270, 274, 257, 278, 285, // pickaxes
    271, 275, 258, 279, 286, // axes
    269, 273, 256, 277, 284, // shovels
];
/// The bow, the fishing rod, and the items with `maxDamage > 0`
/// (`Item.isDamageable`): tools, swords, armor, bow, rod, flint and steel,
/// shears, carrot on a stick.
const DAMAGEABLE: [i32; 34] = [
    256, 257, 258, 259, 261, 267, 268, 269, 270, 271, 272, 273, 274, 275, 276, 277, 278, 279, 283,
    284, 285, 286, 298, 299, 300, 301, 302, 303, 304, 305, 306, 307, 308, 309,
];
// (diamond and gold armor, the fishing rod 346, shears 359 and carrot on a
// stick 398 follow in `is_damageable` below)

fn is_damageable(id: i32) -> bool {
    DAMAGEABLE.contains(&id) || (310..=317).contains(&id) || id == 346 || id == 359 || id == 398
}

fn is_armor(id: i32) -> bool {
    (298..=317).contains(&id)
}

impl EnchantmentCategory {
    /// `EnchantmentCategory.canEnchant(item)`, over the item's numeric id.
    pub fn can_enchant(self, item: i32) -> bool {
        if self == Self::All {
            return true;
        }
        if self == Self::Breakable {
            return is_damageable(item);
        }
        if is_armor(item) {
            if self == Self::Armor {
                return true;
            }
            return if HELMETS.contains(&item) {
                self == Self::ArmorHead
            } else if LEGGINGS.contains(&item) {
                self == Self::ArmorLegs
            } else if CHESTPLATES.contains(&item) {
                self == Self::ArmorTorso
            } else {
                self == Self::ArmorFeet
            };
        }
        if SWORDS.contains(&item) {
            return self == Self::Weapon;
        }
        if TOOLS.contains(&item) {
            return self == Self::Digger;
        }
        if item == 261 {
            return self == Self::Bow;
        }
        item == 346 && self == Self::FishingRod
    }
}

/// The enchantment family: `ProtectionEnchantment`s reject each other and
/// `DamageEnchantment`s do the same; everything else combines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnchantmentFamily {
    Plain,
    Protection,
    Damage,
}

/// One enchantment as `/enchant` reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Enchantment {
    pub id: i32,
    pub category: EnchantmentCategory,
    pub min_level: i32,
    pub max_level: i32,
    /// The `enchantment.<key>` suffix (`Enchantment.getTranslationKey`).
    pub key: &'static str,
    pub family: EnchantmentFamily,
}

impl Enchantment {
    /// `isCompatible(other)`: same family refuses, else `this != other` (the
    /// id check).
    pub fn is_compatible(&self, other: &Enchantment) -> bool {
        match (self.family, other.family) {
            (EnchantmentFamily::Protection, EnchantmentFamily::Protection) => false,
            (EnchantmentFamily::Damage, EnchantmentFamily::Damage) => false,
            _ => self.id != other.id,
        }
    }

    /// `canEnchant(item)`: the category, plus `DamageEnchantment`'s axes
    /// (`item instanceof AxeItem ? true : super.canEnchant(item)`).
    pub fn can_enchant(&self, item: i32) -> bool {
        const AXES: [i32; 5] = [271, 275, 258, 279, 286];
        if self.family == EnchantmentFamily::Damage && AXES.contains(&item) {
            return true;
        }
        self.category.can_enchant(item)
    }
}

/// `Enchantment.BY_ID`: the 26 registered 1.7.10 enchantments.
pub fn enchantment_by_id(id: i32) -> Option<&'static Enchantment> {
    const fn ench(
        id: i32,
        category: EnchantmentCategory,
        min_level: i32,
        max_level: i32,
        key: &'static str,
        family: EnchantmentFamily,
    ) -> Option<Enchantment> {
        Some(Enchantment {
            id,
            category,
            min_level,
            max_level,
            key,
            family,
        })
    }
    const TABLE: [Option<Enchantment>; 256] = {
        let mut table: [Option<Enchantment>; 256] = [None; 256];
        // The protection family (weight 10, 5, 5, 2, 5; levels 1-4).
        table[0] = ench(
            0,
            EnchantmentCategory::Armor,
            1,
            4,
            "protection",
            EnchantmentFamily::Protection,
        );
        table[1] = ench(
            1,
            EnchantmentCategory::Armor,
            1,
            4,
            "fire",
            EnchantmentFamily::Protection,
        );
        table[2] = ench(
            2,
            EnchantmentCategory::Armor,
            1,
            4,
            "fall",
            EnchantmentFamily::Protection,
        );
        table[3] = ench(
            3,
            EnchantmentCategory::Armor,
            1,
            4,
            "blast",
            EnchantmentFamily::Protection,
        );
        table[4] = ench(
            4,
            EnchantmentCategory::Armor,
            1,
            4,
            "projectile",
            EnchantmentFamily::Protection,
        );
        table[5] = ench(
            5,
            EnchantmentCategory::ArmorHead,
            1,
            3,
            "water",
            EnchantmentFamily::Plain,
        );
        table[6] = ench(
            6,
            EnchantmentCategory::ArmorHead,
            1,
            1,
            "waterWorker",
            EnchantmentFamily::Plain,
        );
        table[7] = ench(
            7,
            EnchantmentCategory::Armor,
            1,
            3,
            "thorns",
            EnchantmentFamily::Plain,
        );
        // The damage family (swords, plus axes for canEnchant; levels 1-5, 1-5,
        // 1-5, 1-2, 1-2, 1-3).
        table[16] = ench(
            16,
            EnchantmentCategory::Weapon,
            1,
            5,
            "damage.all",
            EnchantmentFamily::Damage,
        );
        table[17] = ench(
            17,
            EnchantmentCategory::Weapon,
            1,
            5,
            "damage undead",
            EnchantmentFamily::Damage,
        );
        table[18] = ench(
            18,
            EnchantmentCategory::Weapon,
            1,
            5,
            "damage.arthropods",
            EnchantmentFamily::Damage,
        );
        table[19] = ench(
            19,
            EnchantmentCategory::Weapon,
            1,
            2,
            "knockback",
            EnchantmentFamily::Plain,
        );
        table[20] = ench(
            20,
            EnchantmentCategory::Weapon,
            1,
            2,
            "fire",
            EnchantmentFamily::Plain,
        );
        table[21] = ench(
            21,
            EnchantmentCategory::Weapon,
            1,
            3,
            "lootBonus",
            EnchantmentFamily::Plain,
        );
        // Diggers, bow, and the rest.
        table[32] = ench(
            32,
            EnchantmentCategory::Digger,
            1,
            5,
            "digSpeed",
            EnchantmentFamily::Plain,
        );
        table[33] = ench(
            33,
            EnchantmentCategory::Digger,
            1,
            1,
            "untouching",
            EnchantmentFamily::Plain,
        );
        table[34] = ench(
            34,
            EnchantmentCategory::Breakable,
            1,
            3,
            "durability",
            EnchantmentFamily::Plain,
        );
        table[35] = ench(
            35,
            EnchantmentCategory::Digger,
            1,
            3,
            "lootBonusDigger",
            EnchantmentFamily::Plain,
        );
        table[48] = ench(
            48,
            EnchantmentCategory::Bow,
            1,
            5,
            "arrowDamage",
            EnchantmentFamily::Plain,
        );
        table[49] = ench(
            49,
            EnchantmentCategory::Bow,
            1,
            2,
            "arrowKnockback",
            EnchantmentFamily::Plain,
        );
        table[50] = ench(
            50,
            EnchantmentCategory::Bow,
            1,
            1,
            "arrowFire",
            EnchantmentFamily::Plain,
        );
        table[51] = ench(
            51,
            EnchantmentCategory::Bow,
            1,
            1,
            "arrowInfinite",
            EnchantmentFamily::Plain,
        );
        table[61] = ench(
            61,
            EnchantmentCategory::FishingRod,
            1,
            3,
            "lootBonusFishing",
            EnchantmentFamily::Plain,
        );
        table[62] = ench(
            62,
            EnchantmentCategory::FishingRod,
            1,
            3,
            "fishingSpeed",
            EnchantmentFamily::Plain,
        );
        table
    };
    if (0..256).contains(&id) {
        TABLE[id as usize].as_ref()
    } else {
        None
    }
}

/// The 1.7.10 entity id strings `/summon` accepts, from `Entities`' static
/// registration block, in registration order.
pub const ENTITY_IDS: [&str; 57] = [
    "Item",
    "XPOrb",
    "LeashKnot",
    "Painting",
    "Arrow",
    "Snowball",
    "Fireball",
    "SmallFireball",
    "ThrownEnderpearl",
    "EyeOfEnderSignal",
    "ThrownPotion",
    "ThrownExpBottle",
    "ItemFrame",
    "WitherSkull",
    "PrimedTnt",
    "FallingSand",
    "FireworksRocketEntity",
    "Boat",
    "MinecartRideable",
    "MinecartChest",
    "MinecartFurnace",
    "MinecartTNT",
    "MinecartHopper",
    "MinecartSpawner",
    "MinecartCommandBlock",
    "Mob",
    "Monster",
    "Creeper",
    "Skeleton",
    "Spider",
    "Giant",
    "Zombie",
    "Slime",
    "Ghast",
    "PigZombie",
    "Enderman",
    "CaveSpider",
    "Silverfish",
    "Blaze",
    "LavaSlime",
    "EnderDragon",
    "WitherBoss",
    "Bat",
    "Witch",
    "Pig",
    "Sheep",
    "Cow",
    "Chicken",
    "Squid",
    "Wolf",
    "MushroomCow",
    "SnowMan",
    "Ozelot",
    "VillagerGolem",
    "EntityHorse",
    "Villager",
    "EnderCrystal",
];

/// `Entities.getKeys().contains(id)` (the set order is hash order; the
/// command only asks membership).
pub fn id_is_known(id: &str) -> bool {
    ENTITY_IDS.contains(&id)
}

/// `Entities.create(nbt, world)`'s "Minecart" rewrite: a plain "Minecart" id
/// becomes its rideable, chest or furnace variant from `Type`, and `Type` is
/// removed. Returns the id the entity would be created from.
pub fn resolve_create_id(id: &str, minecart_type: i32) -> String {
    if id == "Minecart" {
        let resolved = match minecart_type {
            1 => "MinecartChest",
            2 => "MinecartFurnace",
            _ => "MinecartRideable",
        };
        return resolved.to_string();
    }
    id.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_mode_ids_and_fallback() {
        assert_eq!(GameMode::NotSet.id(), -1);
        assert_eq!(GameMode::by_id(2), GameMode::Adventure);
        assert_eq!(
            GameMode::by_id(3),
            GameMode::Survival,
            "unknown ids fall back to SURVIVAL"
        );
        assert_eq!(GameMode::by_id(-5), GameMode::Survival);
        assert!(GameMode::Adventure.restricts_world_modification());
        assert!(!GameMode::Creative.restricts_world_modification());
    }

    #[test]
    fn difficulty_wraps_modulo_four() {
        assert_eq!(Difficulty::by_id(0), Difficulty::Peaceful);
        assert_eq!(Difficulty::by_id(3), Difficulty::Hard);
        assert_eq!(
            Difficulty::by_id(4),
            Difficulty::Peaceful,
            "Java's id % length wrap"
        );
        assert_eq!(Difficulty::by_id(-1), Difficulty::Hard);
    }

    #[test]
    fn effect_table() {
        assert_eq!(status_effect_by_id(0), None);
        assert_eq!(status_effect_by_id(1).unwrap().key, "moveSpeed");
        assert!(!status_effect_by_id(1).unwrap().instant);
        assert!(status_effect_by_id(6).unwrap().instant, "instant health");
        assert!(status_effect_by_id(7).unwrap().instant, "instant damage");
        assert!(status_effect_by_id(23).unwrap().instant, "saturation");
        assert!(!status_effect_by_id(20).unwrap().instant, "wither");
        assert_eq!(status_effect_by_id(24), None);
        assert_eq!(status_effect_by_id(31), None);
        assert_eq!(status_effect_by_id(32), None, "BY_ID.length is 32");
        assert_eq!(status_effect_by_id(-1), None);
    }

    #[test]
    fn enchantment_table() {
        assert_eq!(enchantment_by_id(0).unwrap().key, "protection");
        assert_eq!(enchantment_by_id(0).unwrap().max_level, 4);
        assert_eq!(enchantment_by_id(16).unwrap().max_level, 5, "sharpness V");
        assert_eq!(enchantment_by_id(19).unwrap().max_level, 2, "knockback II");
        assert_eq!(enchantment_by_id(51).unwrap().max_level, 1, "infinity I");
        assert_eq!(enchantment_by_id(8), None, "depth strider is 1.8+");
        assert_eq!(enchantment_by_id(255), None);
        assert_eq!(enchantment_by_id(-1), None);
    }

    #[test]
    fn enchantment_compatibility() {
        let sharpness = enchantment_by_id(16).unwrap();
        let smite = enchantment_by_id(17).unwrap();
        let protection = enchantment_by_id(0).unwrap();
        let fire_protection = enchantment_by_id(1).unwrap();
        let knockback = enchantment_by_id(19).unwrap();
        // Same family refuses, same id refuses.
        assert!(!sharpness.is_compatible(smite));
        assert!(!protection.is_compatible(fire_protection));
        assert!(sharpness.is_compatible(protection));
        assert!(
            sharpness.is_compatible(knockback),
            "different families combine"
        );
    }

    #[test]
    fn enchantment_categories() {
        let protection = enchantment_by_id(0).unwrap();
        let respiration = enchantment_by_id(5).unwrap();
        let aqua = enchantment_by_id(6).unwrap();
        let depth = enchantment_by_id(8);
        let sharpness = enchantment_by_id(16).unwrap();
        let efficiency = enchantment_by_id(32).unwrap();
        let silk = enchantment_by_id(33).unwrap();
        let unbreaking = enchantment_by_id(34).unwrap();
        let power = enchantment_by_id(48).unwrap();
        let luck = enchantment_by_id(61).unwrap();
        // Armor.
        assert!(protection.can_enchant(310), "diamond helmet");
        assert!(!protection.can_enchant(276), "sword is not armor");
        // Helmet-only.
        assert!(respiration.can_enchant(298));
        assert!(!respiration.can_enchant(301), "boots take no respiration");
        assert!(aqua.can_enchant(306));
        // Weapons and tools.
        assert!(sharpness.can_enchant(276), "diamond sword");
        assert!(sharpness.can_enchant(258), "DamageEnchantment takes axes");
        assert!(!sharpness.can_enchant(261), "bow takes no sharpness");
        assert!(efficiency.can_enchant(257), "iron pickaxe");
        assert!(
            !efficiency.can_enchant(292),
            "hoe is not a ToolItem in 1.7.10"
        );
        assert!(silk.can_enchant(278));
        // Breakable and bow.
        assert!(unbreaking.can_enchant(259), "flint and steel is damageable");
        assert!(!unbreaking.can_enchant(264), "diamond is not damageable");
        assert!(power.can_enchant(261));
        assert!(luck.can_enchant(346));
        assert_eq!(depth, None);
    }

    #[test]
    fn entity_ids_and_minecart() {
        assert!(id_is_known("Zombie"));
        assert!(id_is_known("MinecartRideable"));
        assert!(!id_is_known("zombie"), "ids are the class-name strings");
        assert!(!id_is_known("Minecart"), "the plain id is not registered");
        assert_eq!(resolve_create_id("Minecart", 0), "MinecartRideable");
        assert_eq!(resolve_create_id("Minecart", 1), "MinecartChest");
        assert_eq!(resolve_create_id("Minecart", 2), "MinecartFurnace");
        assert_eq!(resolve_create_id("Minecart", 9), "MinecartRideable");
        assert_eq!(resolve_create_id("Zombie", 0), "Zombie");
    }
}
