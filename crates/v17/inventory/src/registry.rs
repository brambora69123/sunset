//! Item facts the inventory subsystem reads: durability, stack sizes, remainders.
//!
//! 1.7.10 asks the `Item` object for these (`getMaxStackSize`, `getMaxDamage`,
//! `getRecipeRemainder`, `hasCustomData`). The items crate owns the full item
//! table; this crate carries only what inventory math needs, behind
//! [`ItemFacts`], with the vanilla 1.7.10 values as [`VanillaFacts`].

use crate::item_stack::ItemStack;

/// What the inventory code needs to know about an item id.
pub trait ItemFacts {
    /// `Item.getMaxStackSize` (default 64).
    fn max_stack_size(&self, id: u16) -> i32;
    /// `Item.getMaxDamage` (0 for most items).
    fn max_damage(&self, id: u16) -> i32;
    /// `Item.getRecipeRemainder`: the id a consumed crafting/smelting item leaves.
    fn recipe_remainder(&self, id: u16) -> Option<u16>;
    /// `Item.hasCustomData`: NBT-bearing items are never damageable and never repair-anvil items.
    fn has_custom_data(&self, id: u16) -> bool;
    /// `ArmorItem.slot` for armor pieces (0 helmet..3 boots), `None` for non-armor.
    fn armor_slot(&self, id: u16) -> Option<usize>;
    /// Furnace fuel: `FurnaceBlockEntity.getFuelTime(item)`.
    fn fuel_time(&self, stack: &ItemStack) -> i32 {
        crate::furnace::vanilla_fuel_time(stack)
    }
}

/// No-op facts: everything default. Tests that do not exercise stacking use it.
pub struct PlainFacts;

impl ItemFacts for PlainFacts {
    fn max_stack_size(&self, _id: u16) -> i32 {
        64
    }
    fn max_damage(&self, _id: u16) -> i32 {
        0
    }
    fn recipe_remainder(&self, _id: u16) -> Option<u16> {
        None
    }
    fn has_custom_data(&self, _id: u16) -> bool {
        false
    }
    fn armor_slot(&self, _id: u16) -> Option<usize> {
        None
    }
}

/// The 1.7.10 vanilla item table, as far as inventory math reads it.
#[derive(Default)]
pub struct VanillaFacts;

/// 1.7.10 ids, `Item.REGISTRY` (and `Block.REGISTRY` for block items).
pub mod ids {
    // Block items, `Block.REGISTRY` order.
    pub const STONE: u16 = 1;
    pub const PISTON: u16 = 33;
    pub const STICKY_PISTON: u16 = 29;
    pub const GRASS: u16 = 2;
    pub const DIRT: u16 = 3;
    pub const COBBLESTONE: u16 = 4;
    pub const PLANKS: u16 = 5;
    pub const SAPLING: u16 = 6;
    pub const LEAVES: u16 = 18;
    pub const LEAVES2: u16 = 161;
    pub const DIAMOND_BLOCK: u16 = 57;
    pub const EMERALD_BLOCK: u16 = 133;
    pub const BROWN_MUSHROOM: u16 = 39;
    pub const RED_MUSHROOM: u16 = 40;
    pub const TRIPWIRE_HOOK: u16 = 131;
    pub const ENDER_CHEST: u16 = 130;
    pub const BROWN_MUSHROOM_BLOCK: u16 = 99;
    pub const RED_MUSHROOM_BLOCK: u16 = 100;
    pub const NOTEBLOCK: u16 = 25;
    pub const JUKEBOX: u16 = 84;
    pub const STANDING_SIGN: u16 = 63;
    pub const WALL_SIGN: u16 = 68;
    pub const WOODEN_DOOR_BLOCK: u16 = 64;
    pub const WOODEN_PRESSURE_PLATE: u16 = 72;
    pub const WOODEN_TRAPDOOR: u16 = 96;
    pub const WOODEN_BUTTON: u16 = 143;
    pub const OAK_STAIRS: u16 = 53;
    pub const SPRUCE_STAIRS: u16 = 134;
    pub const BIRCH_STAIRS: u16 = 135;
    pub const JUNGLE_STAIRS: u16 = 136;
    pub const ACACIA_STAIRS: u16 = 163;
    pub const DARK_OAK_STAIRS: u16 = 164;
    pub const DOUBLE_WOODEN_SLAB: u16 = 125;
    pub const REDSTONE_ORE: u16 = 73;
    pub const LAPIS_BLOCK: u16 = 22;
    pub const EMERALD_ORE: u16 = 129;
    pub const QUARTZ_ORE: u16 = 153;
    pub const SAND: u16 = 12;
    pub const GRAVEL: u16 = 13;
    pub const GOLD_ORE: u16 = 14;
    pub const IRON_ORE: u16 = 15;
    pub const COAL_ORE: u16 = 16;
    pub const LOG: u16 = 17;
    pub const LOG2: u16 = 162;
    pub const GLASS: u16 = 20;
    pub const LAPIS_ORE: u16 = 21;
    pub const SANDSTONE: u16 = 24;
    pub const WOOL: u16 = 35;
    pub const YELLOW_FLOWER: u16 = 37;
    pub const RED_FLOWER: u16 = 38;
    pub const GOLD_BLOCK: u16 = 41;
    pub const IRON_BLOCK: u16 = 42;
    pub const BRICKS: u16 = 45;
    pub const TNT: u16 = 46;
    pub const BOOKSHELF: u16 = 47;
    pub const MOSSY_COBBLESTONE: u16 = 48;
    pub const OBSIDIAN: u16 = 49;
    pub const TORCH: u16 = 50;
    pub const CHEST: u16 = 54;
    pub const DIAMOND_ORE: u16 = 56;
    pub const CRAFTING_TABLE: u16 = 58;
    pub const WHEAT_BLOCK: u16 = 59;
    pub const FURNACE: u16 = 61;
    pub const LADDER: u16 = 65;
    pub const SNOW_LAYER: u16 = 78;
    pub const SNOW_BLOCK: u16 = 80;
    pub const CACTUS: u16 = 81;
    pub const CLAY_BLOCK: u16 = 82;
    pub const FENCE: u16 = 85;
    pub const PUMPKIN: u16 = 86;
    pub const NETHERRACK: u16 = 87;
    pub const SOUL_SAND: u16 = 88;
    pub const GLOWSTONE: u16 = 89;
    pub const LIT_PUMPKIN: u16 = 91;
    pub const STAINED_GLASS: u16 = 95;
    pub const TRAPDOOR: u16 = 96;
    pub const STONE_BRICKS: u16 = 98;
    pub const STONE_SLAB: u16 = 44;
    pub const IRON_BARS: u16 = 101;
    pub const BEACON: u16 = 138;
    pub const FIRE: u16 = 51;
    pub const GLASS_PANE: u16 = 102;
    pub const MELON_BLOCK: u16 = 103;
    pub const FENCE_GATE: u16 = 107;
    pub const BRICK_STAIRS: u16 = 108;
    pub const STONE_BRICK_STAIRS: u16 = 109;
    pub const NETHER_BRICK: u16 = 112;
    pub const NETHER_BRICK_FENCE: u16 = 113;
    pub const NETHER_BRICK_STAIRS: u16 = 114;
    pub const ENCHANTING_TABLE: u16 = 116;
    pub const REDSTONE_LAMP: u16 = 123;
    pub const WOODEN_SLAB: u16 = 126;
    pub const COBBLESTONE_WALL: u16 = 139;
    pub const ANVIL: u16 = 145;
    pub const TRAPPED_CHEST: u16 = 146;
    pub const QUARTZ_BLOCK: u16 = 155;
    pub const HOPPER: u16 = 154;
    pub const DAYLIGHT_DETECTOR: u16 = 151;
    pub const REDSTONE_BLOCK: u16 = 152;
    pub const QUARTZ_STAIRS: u16 = 156;
    pub const DROPPER: u16 = 158;
    pub const STAINED_GLASS_PANE: u16 = 160;
    pub const HAY: u16 = 170;
    pub const CARPET: u16 = 171;
    pub const HARDENED_CLAY: u16 = 172;
    pub const STAINED_HARDENED_CLAY: u16 = 159;
    pub const COAL_BLOCK: u16 = 173;
    pub const PACKED_ICE: u16 = 174;
    pub const DOUBLE_PLANT: u16 = 175;

    // Items, `Item.REGISTRY` ids.
    pub const IRON_SHOVEL: u16 = 256;
    pub const IRON_PICKAXE: u16 = 257;
    pub const IRON_AXE: u16 = 258;
    pub const FLINT_AND_STEEL: u16 = 259;
    pub const APPLE: u16 = 260;
    pub const BOW: u16 = 261;
    pub const ARROW: u16 = 262;
    pub const COAL: u16 = 263;
    pub const DIAMOND: u16 = 264;
    pub const IRON_INGOT: u16 = 265;
    pub const GOLD_INGOT: u16 = 266;
    pub const IRON_SWORD: u16 = 267;
    pub const WOODEN_SWORD: u16 = 268;
    pub const WOODEN_SHOVEL: u16 = 269;
    pub const WOODEN_PICKAXE: u16 = 270;
    pub const WOODEN_AXE: u16 = 271;
    pub const STONE_SWORD: u16 = 272;
    pub const STONE_SHOVEL: u16 = 273;
    pub const STONE_PICKAXE: u16 = 274;
    pub const STONE_AXE: u16 = 275;
    pub const DIAMOND_SWORD: u16 = 276;
    pub const DIAMOND_SHOVEL: u16 = 277;
    pub const DIAMOND_PICKAXE: u16 = 278;
    pub const DIAMOND_AXE: u16 = 279;
    pub const STICK: u16 = 280;
    pub const BOWL: u16 = 281;
    pub const MUSHROOM_STEW: u16 = 282;
    pub const GOLDEN_SWORD: u16 = 283;
    pub const GOLDEN_SHOVEL: u16 = 284;
    pub const GOLDEN_PICKAXE: u16 = 285;
    pub const GOLDEN_AXE: u16 = 286;
    pub const STRING: u16 = 287;
    pub const FEATHER: u16 = 288;
    pub const GUNPOWDER: u16 = 289;
    pub const WOODEN_HOE: u16 = 290;
    pub const STONE_HOE: u16 = 291;
    pub const IRON_HOE: u16 = 292;
    pub const DIAMOND_HOE: u16 = 293;
    pub const GOLDEN_HOE: u16 = 294;
    pub const WHEAT_SEEDS: u16 = 295;
    pub const WHEAT: u16 = 296;
    pub const BREAD: u16 = 297;
    pub const LEATHER_HELMET: u16 = 298;
    pub const LEATHER_CHESTPLATE: u16 = 299;
    pub const LEATHER_LEGGINGS: u16 = 300;
    pub const LEATHER_BOOTS: u16 = 301;
    pub const CHAINMAIL_HELMET: u16 = 302;
    pub const CHAINMAIL_CHESTPLATE: u16 = 303;
    pub const CHAINMAIL_LEGGINGS: u16 = 304;
    pub const CHAINMAIL_BOOTS: u16 = 305;
    pub const IRON_HELMET: u16 = 306;
    pub const IRON_CHESTPLATE: u16 = 307;
    pub const IRON_LEGGINGS: u16 = 308;
    pub const IRON_BOOTS: u16 = 309;
    pub const DIAMOND_HELMET: u16 = 310;
    pub const DIAMOND_CHESTPLATE: u16 = 311;
    pub const DIAMOND_LEGGINGS: u16 = 312;
    pub const DIAMOND_BOOTS: u16 = 313;
    pub const GOLDEN_HELMET: u16 = 314;
    pub const GOLDEN_CHESTPLATE: u16 = 315;
    pub const GOLDEN_LEGGINGS: u16 = 316;
    pub const GOLDEN_BOOTS: u16 = 317;
    pub const FLINT: u16 = 318;
    pub const PORKCHOP: u16 = 319;
    pub const COOKED_PORKCHOP: u16 = 320;
    pub const PAINTING: u16 = 321;
    pub const GOLDEN_APPLE: u16 = 322;
    pub const SIGN: u16 = 323;
    pub const WOODEN_DOOR: u16 = 324;
    pub const BUCKET: u16 = 325;
    pub const WATER_BUCKET: u16 = 326;
    pub const LAVA_BUCKET: u16 = 327;
    pub const MINECART: u16 = 328;
    pub const SADDLE: u16 = 329;
    pub const IRON_DOOR: u16 = 330;
    pub const REDSTONE: u16 = 331;
    pub const SNOWBALL: u16 = 332;
    pub const BOAT: u16 = 333;
    pub const LEATHER: u16 = 334;
    pub const MILK_BUCKET: u16 = 335;
    pub const BRICK: u16 = 336;
    pub const CLAY_BALL: u16 = 337;
    pub const REEDS: u16 = 338;
    pub const PAPER: u16 = 339;
    pub const BOOK: u16 = 340;
    pub const SLIME_BALL: u16 = 341;
    pub const CHEST_MINECART: u16 = 342;
    pub const FURNACE_MINECART: u16 = 343;
    pub const EGG: u16 = 344;
    pub const COMPASS: u16 = 345;
    pub const FISHING_ROD: u16 = 346;
    pub const CLOCK: u16 = 347;
    pub const GLOWSTONE_DUST: u16 = 348;
    pub const FISH: u16 = 349;
    pub const COOKED_FISH: u16 = 350;
    pub const DYE: u16 = 351;
    pub const BONE: u16 = 352;
    pub const SUGAR: u16 = 353;
    pub const CAKE: u16 = 354;
    pub const BED: u16 = 355;
    pub const REPEATER: u16 = 356;
    pub const COOKIE: u16 = 357;
    pub const FILLED_MAP: u16 = 358;
    pub const SHEARS: u16 = 359;
    pub const MELON: u16 = 360;
    pub const PUMPKIN_SEEDS: u16 = 361;
    pub const MELON_SEEDS: u16 = 362;
    pub const BEEF: u16 = 363;
    pub const COOKED_BEEF: u16 = 364;
    pub const CHICKEN: u16 = 365;
    pub const COOKED_CHICKEN: u16 = 366;
    pub const ROTTEN_FLESH: u16 = 367;
    pub const ENDER_PEARL: u16 = 368;
    pub const BLAZE_ROD: u16 = 369;
    pub const GHAST_TEAR: u16 = 370;
    pub const GOLD_NUGGET: u16 = 371;
    pub const NETHER_WART: u16 = 372;
    pub const POTION: u16 = 373;
    pub const GLASS_BOTTLE: u16 = 374;
    pub const SPIDER_EYE: u16 = 375;
    pub const FERMENTED_SPIDER_EYE: u16 = 376;
    pub const BLAZE_POWDER: u16 = 377;
    pub const MAGMA_CREAM: u16 = 378;
    pub const BREWING_STAND: u16 = 379;
    pub const CAULDRON: u16 = 380;
    pub const ENDER_EYE: u16 = 381;
    pub const SPECKLED_MELON: u16 = 382;
    pub const SPAWN_EGG: u16 = 383;
    pub const EXPERIENCE_BOTTLE: u16 = 384;
    pub const FIRE_CHARGE: u16 = 385;
    pub const WRITABLE_BOOK: u16 = 386;
    pub const WRITTEN_BOOK: u16 = 387;
    pub const EMERALD: u16 = 388;
    pub const ITEM_FRAME: u16 = 389;
    pub const FLOWER_POT: u16 = 390;
    pub const CARROT: u16 = 391;
    pub const POTATO: u16 = 392;
    pub const BAKED_POTATO: u16 = 393;
    pub const POISONOUS_POTATO: u16 = 394;
    pub const MAP: u16 = 395;
    pub const GOLDEN_CARROT: u16 = 396;
    pub const SKULL: u16 = 397;
    pub const CARROT_ON_A_STICK: u16 = 398;
    pub const NETHER_STAR: u16 = 399;
    pub const PUMPKIN_PIE: u16 = 400;
    pub const FIREWORKS: u16 = 401;
    pub const FIREWORK_CHARGE: u16 = 402;
    pub const ENCHANTED_BOOK: u16 = 403;
    pub const COMPARATOR: u16 = 404;
    pub const NETHERBRICK: u16 = 405;
    pub const QUARTZ: u16 = 406;
    pub const TNT_MINECART: u16 = 407;
    pub const HOPPER_MINECART: u16 = 408;
    pub const IRON_HORSE_ARMOR: u16 = 417;
    pub const GOLDEN_HORSE_ARMOR: u16 = 418;
    pub const DIAMOND_HORSE_ARMOR: u16 = 419;
    pub const LEAD: u16 = 420;
    pub const NAME_TAG: u16 = 421;
    pub const COMMAND_BLOCK_MINECART: u16 = 422;
}

/// `Item$Tier` durability, the tools and the sword share the table.
pub const TIER_DURABILITY: [i32; 5] = [59, 131, 250, 1561, 32];

/// Wood/stone/iron/diamond/gold tiers in registry order.
const TIER_WOOD: usize = 0;
const TIER_STONE: usize = 1;
const TIER_IRON: usize = 2;
const TIER_DIAMOND: usize = 3;
const TIER_GOLD: usize = 4;

/// `ArmorItem.BASE_DURABILITY`: helmet, chestplate, leggings, boots.
const ARMOR_BASE_DURABILITY: [i32; 4] = [11, 16, 15, 13];

/// `ArmorItem$Tier` per-slot durability multiplier, CLOTH CHAIN IRON GOLD DIAMOND.
const ARMOR_TIER_DURABILITY: [i32; 5] = [5, 15, 15, 7, 33];

fn tool(id: u16) -> Option<usize> {
    use ids::*;
    match id {
        WOODEN_SWORD | WOODEN_SHOVEL | WOODEN_PICKAXE | WOODEN_AXE | WOODEN_HOE => Some(TIER_WOOD),
        STONE_SWORD | STONE_SHOVEL | STONE_PICKAXE | STONE_AXE | STONE_HOE => Some(TIER_STONE),
        IRON_SWORD | IRON_SHOVEL | IRON_PICKAXE | IRON_AXE | IRON_HOE => Some(TIER_IRON),
        DIAMOND_SWORD | DIAMOND_SHOVEL | DIAMOND_PICKAXE | DIAMOND_AXE | DIAMOND_HOE => {
            Some(TIER_DIAMOND)
        }
        GOLDEN_SWORD | GOLDEN_SHOVEL | GOLDEN_PICKAXE | GOLDEN_AXE | GOLDEN_HOE => Some(TIER_GOLD),
        _ => None,
    }
}

fn armor(id: u16) -> Option<(usize, usize)> {
    use ids::*;
    // (tier, slot)
    const CLOTH: usize = 0;
    const CHAIN: usize = 1;
    const IRON_T: usize = 2;
    const GOLD_T: usize = 3;
    const DIAMOND_T: usize = 4;
    match id {
        LEATHER_HELMET => Some((CLOTH, 0)),
        LEATHER_CHESTPLATE => Some((CLOTH, 1)),
        LEATHER_LEGGINGS => Some((CLOTH, 2)),
        LEATHER_BOOTS => Some((CLOTH, 3)),
        CHAINMAIL_HELMET => Some((CHAIN, 0)),
        CHAINMAIL_CHESTPLATE => Some((CHAIN, 1)),
        CHAINMAIL_LEGGINGS => Some((CHAIN, 2)),
        CHAINMAIL_BOOTS => Some((CHAIN, 3)),
        IRON_HELMET => Some((IRON_T, 0)),
        IRON_CHESTPLATE => Some((IRON_T, 1)),
        IRON_LEGGINGS => Some((IRON_T, 2)),
        IRON_BOOTS => Some((IRON_T, 3)),
        DIAMOND_HELMET => Some((DIAMOND_T, 0)),
        DIAMOND_CHESTPLATE => Some((DIAMOND_T, 1)),
        DIAMOND_LEGGINGS => Some((DIAMOND_T, 2)),
        DIAMOND_BOOTS => Some((DIAMOND_T, 3)),
        GOLDEN_HELMET => Some((GOLD_T, 0)),
        GOLDEN_CHESTPLATE => Some((GOLD_T, 1)),
        GOLDEN_LEGGINGS => Some((GOLD_T, 2)),
        GOLDEN_BOOTS => Some((GOLD_T, 3)),
        _ => None,
    }
}

impl ItemFacts for VanillaFacts {
    fn max_stack_size(&self, id: u16) -> i32 {
        use ids::*;
        match id {
            // `setMaxStackSize(1)`.
            MINECART
            | CHEST_MINECART
            | FURNACE_MINECART
            | TNT_MINECART
            | HOPPER_MINECART
            | COMMAND_BLOCK_MINECART
            | BOAT
            | SADDLE
            | BUCKET
            | WATER_BUCKET
            | LAVA_BUCKET
            | MILK_BUCKET
            | WOODEN_DOOR
            | IRON_DOOR
            | CAKE
            | BED
            | IRON_HORSE_ARMOR
            | GOLDEN_HORSE_ARMOR
            | DIAMOND_HORSE_ARMOR
            | ENCHANTED_BOOK => 1,
            // Tools and armor set max size 1 in their constructors.
            id if tool(id).is_some() || armor(id).is_some() => 1,
            FLINT_AND_STEEL | FISHING_ROD | CARROT_ON_A_STICK | SHEARS | BOW | SIGN | SNOWBALL
            | EGG | ENDER_PEARL | WRITTEN_BOOK | POTION => 16,
            _ => 64,
        }
    }

    fn max_damage(&self, id: u16) -> i32 {
        if let Some(tier) = tool(id) {
            return TIER_DURABILITY[tier];
        }
        if let Some((tier, slot)) = armor(id) {
            return ARMOR_BASE_DURABILITY[slot] * ARMOR_TIER_DURABILITY[tier];
        }
        use ids::*;
        match id {
            SHEARS => 238,
            BOW => 384,
            FISHING_ROD => 64,
            FLINT_AND_STEEL => 64,
            CARROT_ON_A_STICK => 25,
            _ => 0,
        }
    }

    fn recipe_remainder(&self, id: u16) -> Option<u16> {
        use ids::*;
        match id {
            WATER_BUCKET | LAVA_BUCKET | MILK_BUCKET => Some(BUCKET),
            POTION => Some(GLASS_BOTTLE),
            _ => None,
        }
    }

    fn has_custom_data(&self, id: u16) -> bool {
        use ids::*;
        matches!(
            id,
            COAL | ids::LEAVES
                | ids::LEAVES2
                | WOOL
                | STAINED_HARDENED_CLAY
                | STAINED_GLASS
                | STAINED_GLASS_PANE
                | CARPET
                | FILLED_MAP
                | GOLDEN_APPLE
                | FISH
                | COOKED_FISH
                | DYE
                | SAPLING
        )
    }

    fn armor_slot(&self, id: u16) -> Option<usize> {
        armor(id).map(|(_, slot)| slot)
    }
}
