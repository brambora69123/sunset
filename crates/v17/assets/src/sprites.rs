//! The 1.7.10 id → texture-name mapping: `net.minecraft.block.Block.init`'s
//! registry, `net.minecraft.item.Item.init`'s registry, and the per-block
//! and per-item sprite registration of the client (`registerSprites` /
//! `getSprite`), ported exactly.
//!
//! A texture name is atlas-relative: it resolves to
//! `assets/<namespace>/textures/blocks/<name>.png` (atlas 0) or
//! `assets/<namespace>/textures/items/<name>.png` (atlas 1). Face ids are
//! vanilla's: 0 bottom, 1 top, 2 north, 3 south, 4 west, 5 east.

use crate::identifier::Identifier;

/// `TextureAtlas.BLOCKS_LOCATION`'s path.
pub const BLOCKS_ATLAS_PATH: &str = "textures/blocks";
/// `TextureAtlas.ITEMS_LOCATION`'s path.
pub const ITEMS_ATLAS_PATH: &str = "textures/items";

pub const FACE_BOTTOM: u8 = 0;
pub const FACE_TOP: u8 = 1;
pub const FACE_NORTH: u8 = 2;
pub const FACE_SOUTH: u8 = 3;
pub const FACE_WEST: u8 = 4;
pub const FACE_EAST: u8 = 5;

/// `Directions.OPPOSITE`.
const OPPOSITE: [u8; 6] = [1, 0, 3, 2, 5, 4];

/// `net.minecraft.item.DyeItem.COLOR_NAMES`, dye metadata 0..15. Wool and
/// the other colored blocks index it with `~metadata & 15`.
pub const DYE_COLOR_NAMES: [&str; 16] = [
    "black",
    "red",
    "green",
    "brown",
    "blue",
    "purple",
    "cyan",
    "silver",
    "gray",
    "pink",
    "lime",
    "yellow",
    "light_blue",
    "magenta",
    "orange",
    "white",
];

/// `ColoredBlock.getItemMetadataForBlockMetadata`: item metadata of a colored
/// block (dye 0 = white item, block metadata 15).
pub fn dye_color_of_block_metadata(metadata: i32) -> usize {
    (!metadata & 15) as usize
}

/// One registered block: id, translation key (`setKey`), base sprite name
/// (`setSpriteName`; `None` when the vanilla init leaves it unset — those
/// blocks resolve `getSpriteName` to `MISSING_ICON_BLOCK_<id>_<key>`).
#[derive(Clone, Copy, Debug)]
pub struct BlockEntry {
    pub id: i32,
    pub key: &'static str,
    pub sprite: Option<&'static str>,
}

/// The full 1.7.10 block registry, in `Block.init` order.
pub const BLOCKS: &[BlockEntry] = &[
    BlockEntry {
        id: 0,
        key: "air",
        sprite: None,
    },
    BlockEntry {
        id: 1,
        key: "stone",
        sprite: Some("stone"),
    },
    BlockEntry {
        id: 2,
        key: "grass",
        sprite: Some("grass"),
    },
    BlockEntry {
        id: 3,
        key: "dirt",
        sprite: Some("dirt"),
    },
    BlockEntry {
        id: 4,
        key: "stonebrick",
        sprite: Some("cobblestone"),
    },
    BlockEntry {
        id: 5,
        key: "wood",
        sprite: Some("planks"),
    },
    BlockEntry {
        id: 6,
        key: "sapling",
        sprite: Some("sapling"),
    },
    BlockEntry {
        id: 7,
        key: "bedrock",
        sprite: Some("bedrock"),
    },
    BlockEntry {
        id: 8,
        key: "water",
        sprite: Some("water_flow"),
    },
    BlockEntry {
        id: 9,
        key: "water",
        sprite: Some("water_still"),
    },
    BlockEntry {
        id: 10,
        key: "lava",
        sprite: Some("lava_flow"),
    },
    BlockEntry {
        id: 11,
        key: "lava",
        sprite: Some("lava_still"),
    },
    BlockEntry {
        id: 12,
        key: "sand",
        sprite: Some("sand"),
    },
    BlockEntry {
        id: 13,
        key: "gravel",
        sprite: Some("gravel"),
    },
    BlockEntry {
        id: 14,
        key: "oreGold",
        sprite: Some("gold_ore"),
    },
    BlockEntry {
        id: 15,
        key: "oreIron",
        sprite: Some("iron_ore"),
    },
    BlockEntry {
        id: 16,
        key: "oreCoal",
        sprite: Some("coal_ore"),
    },
    BlockEntry {
        id: 17,
        key: "log",
        sprite: Some("log"),
    },
    BlockEntry {
        id: 18,
        key: "leaves",
        sprite: Some("leaves"),
    },
    BlockEntry {
        id: 19,
        key: "sponge",
        sprite: Some("sponge"),
    },
    BlockEntry {
        id: 20,
        key: "glass",
        sprite: Some("glass"),
    },
    BlockEntry {
        id: 21,
        key: "oreLapis",
        sprite: Some("lapis_ore"),
    },
    BlockEntry {
        id: 22,
        key: "blockLapis",
        sprite: Some("lapis_block"),
    },
    BlockEntry {
        id: 23,
        key: "dispenser",
        sprite: Some("dispenser"),
    },
    BlockEntry {
        id: 24,
        key: "sandStone",
        sprite: Some("sandstone"),
    },
    BlockEntry {
        id: 25,
        key: "musicBlock",
        sprite: Some("noteblock"),
    },
    BlockEntry {
        id: 26,
        key: "bed",
        sprite: Some("bed"),
    },
    BlockEntry {
        id: 27,
        key: "goldenRail",
        sprite: Some("rail_golden"),
    },
    BlockEntry {
        id: 28,
        key: "detectorRail",
        sprite: Some("rail_detector"),
    },
    BlockEntry {
        id: 29,
        key: "pistonStickyBase",
        sprite: None,
    },
    BlockEntry {
        id: 30,
        key: "web",
        sprite: Some("web"),
    },
    BlockEntry {
        id: 31,
        key: "tallgrass",
        sprite: None,
    },
    BlockEntry {
        id: 32,
        key: "deadbush",
        sprite: Some("deadbush"),
    },
    BlockEntry {
        id: 33,
        key: "pistonBase",
        sprite: None,
    },
    BlockEntry {
        id: 34,
        key: "pistonHead",
        sprite: None,
    },
    BlockEntry {
        id: 35,
        key: "cloth",
        sprite: Some("wool_colored"),
    },
    BlockEntry {
        id: 36,
        key: "pistonExtension",
        sprite: None,
    },
    BlockEntry {
        id: 37,
        key: "flower1",
        sprite: Some("flower_dandelion"),
    },
    BlockEntry {
        id: 38,
        key: "flower2",
        sprite: Some("flower_rose"),
    },
    BlockEntry {
        id: 39,
        key: "mushroom",
        sprite: Some("mushroom_brown"),
    },
    BlockEntry {
        id: 40,
        key: "mushroom",
        sprite: Some("mushroom_red"),
    },
    BlockEntry {
        id: 41,
        key: "blockGold",
        sprite: Some("gold_block"),
    },
    BlockEntry {
        id: 42,
        key: "blockIron",
        sprite: Some("iron_block"),
    },
    BlockEntry {
        id: 43,
        key: "stoneSlab",
        sprite: None,
    },
    BlockEntry {
        id: 44,
        key: "stoneSlab",
        sprite: None,
    },
    BlockEntry {
        id: 45,
        key: "brick",
        sprite: Some("brick"),
    },
    BlockEntry {
        id: 46,
        key: "tnt",
        sprite: Some("tnt"),
    },
    BlockEntry {
        id: 47,
        key: "bookshelf",
        sprite: Some("bookshelf"),
    },
    BlockEntry {
        id: 48,
        key: "stoneMoss",
        sprite: Some("cobblestone_mossy"),
    },
    BlockEntry {
        id: 49,
        key: "obsidian",
        sprite: Some("obsidian"),
    },
    BlockEntry {
        id: 50,
        key: "torch",
        sprite: Some("torch_on"),
    },
    BlockEntry {
        id: 51,
        key: "fire",
        sprite: Some("fire"),
    },
    BlockEntry {
        id: 52,
        key: "mobSpawner",
        sprite: Some("mob_spawner"),
    },
    BlockEntry {
        id: 53,
        key: "stairsWood",
        sprite: None,
    },
    BlockEntry {
        id: 54,
        key: "chest",
        sprite: None,
    },
    BlockEntry {
        id: 55,
        key: "redstoneDust",
        sprite: Some("redstone_dust"),
    },
    BlockEntry {
        id: 56,
        key: "oreDiamond",
        sprite: Some("diamond_ore"),
    },
    BlockEntry {
        id: 57,
        key: "blockDiamond",
        sprite: Some("diamond_block"),
    },
    BlockEntry {
        id: 58,
        key: "workbench",
        sprite: Some("crafting_table"),
    },
    BlockEntry {
        id: 59,
        key: "crops",
        sprite: Some("wheat"),
    },
    BlockEntry {
        id: 60,
        key: "farmland",
        sprite: Some("farmland"),
    },
    BlockEntry {
        id: 61,
        key: "furnace",
        sprite: None,
    },
    BlockEntry {
        id: 62,
        key: "furnace",
        sprite: None,
    },
    BlockEntry {
        id: 63,
        key: "sign",
        sprite: None,
    },
    BlockEntry {
        id: 64,
        key: "doorWood",
        sprite: Some("door_wood"),
    },
    BlockEntry {
        id: 65,
        key: "ladder",
        sprite: Some("ladder"),
    },
    BlockEntry {
        id: 66,
        key: "rail",
        sprite: Some("rail_normal"),
    },
    BlockEntry {
        id: 67,
        key: "stairsStone",
        sprite: None,
    },
    BlockEntry {
        id: 68,
        key: "sign",
        sprite: None,
    },
    BlockEntry {
        id: 69,
        key: "lever",
        sprite: Some("lever"),
    },
    BlockEntry {
        id: 70,
        key: "pressurePlate",
        sprite: None,
    },
    BlockEntry {
        id: 71,
        key: "doorIron",
        sprite: Some("door_iron"),
    },
    BlockEntry {
        id: 72,
        key: "pressurePlate",
        sprite: None,
    },
    BlockEntry {
        id: 73,
        key: "oreRedstone",
        sprite: Some("redstone_ore"),
    },
    BlockEntry {
        id: 74,
        key: "oreRedstone",
        sprite: Some("redstone_ore"),
    },
    BlockEntry {
        id: 75,
        key: "notGate",
        sprite: Some("redstone_torch_off"),
    },
    BlockEntry {
        id: 76,
        key: "notGate",
        sprite: Some("redstone_torch_on"),
    },
    BlockEntry {
        id: 77,
        key: "button",
        sprite: None,
    },
    BlockEntry {
        id: 78,
        key: "snow",
        sprite: Some("snow"),
    },
    BlockEntry {
        id: 79,
        key: "ice",
        sprite: Some("ice"),
    },
    BlockEntry {
        id: 80,
        key: "snow",
        sprite: Some("snow"),
    },
    BlockEntry {
        id: 81,
        key: "cactus",
        sprite: Some("cactus"),
    },
    BlockEntry {
        id: 82,
        key: "clay",
        sprite: Some("clay"),
    },
    BlockEntry {
        id: 83,
        key: "reeds",
        sprite: Some("reeds"),
    },
    BlockEntry {
        id: 84,
        key: "jukebox",
        sprite: Some("jukebox"),
    },
    BlockEntry {
        id: 85,
        key: "fence",
        sprite: None,
    },
    BlockEntry {
        id: 86,
        key: "pumpkin",
        sprite: Some("pumpkin"),
    },
    BlockEntry {
        id: 87,
        key: "hellrock",
        sprite: Some("netherrack"),
    },
    BlockEntry {
        id: 88,
        key: "hellsand",
        sprite: Some("soul_sand"),
    },
    BlockEntry {
        id: 89,
        key: "lightgem",
        sprite: Some("glowstone"),
    },
    BlockEntry {
        id: 90,
        key: "portal",
        sprite: Some("portal"),
    },
    BlockEntry {
        id: 91,
        key: "litpumpkin",
        sprite: Some("pumpkin"),
    },
    BlockEntry {
        id: 92,
        key: "cake",
        sprite: Some("cake"),
    },
    BlockEntry {
        id: 93,
        key: "diode",
        sprite: Some("repeater_off"),
    },
    BlockEntry {
        id: 94,
        key: "diode",
        sprite: Some("repeater_on"),
    },
    BlockEntry {
        id: 95,
        key: "stainedGlass",
        sprite: Some("glass"),
    },
    BlockEntry {
        id: 96,
        key: "trapdoor",
        sprite: Some("trapdoor"),
    },
    BlockEntry {
        id: 97,
        key: "monsterStoneEgg",
        sprite: None,
    },
    BlockEntry {
        id: 98,
        key: "stonebricksmooth",
        sprite: Some("stonebrick"),
    },
    BlockEntry {
        id: 99,
        key: "mushroom",
        sprite: Some("mushroom_block"),
    },
    BlockEntry {
        id: 100,
        key: "mushroom",
        sprite: Some("mushroom_block"),
    },
    BlockEntry {
        id: 101,
        key: "fenceIron",
        sprite: None,
    },
    BlockEntry {
        id: 102,
        key: "thinGlass",
        sprite: None,
    },
    BlockEntry {
        id: 103,
        key: "melon",
        sprite: Some("melon"),
    },
    BlockEntry {
        id: 104,
        key: "pumpkinStem",
        sprite: Some("pumpkin_stem"),
    },
    BlockEntry {
        id: 105,
        key: "pumpkinStem",
        sprite: Some("melon_stem"),
    },
    BlockEntry {
        id: 106,
        key: "vine",
        sprite: Some("vine"),
    },
    BlockEntry {
        id: 107,
        key: "fenceGate",
        sprite: None,
    },
    BlockEntry {
        id: 108,
        key: "stairsBrick",
        sprite: None,
    },
    BlockEntry {
        id: 109,
        key: "stairsStoneBrickSmooth",
        sprite: None,
    },
    BlockEntry {
        id: 110,
        key: "mycel",
        sprite: Some("mycelium"),
    },
    BlockEntry {
        id: 111,
        key: "waterlily",
        sprite: Some("waterlily"),
    },
    BlockEntry {
        id: 112,
        key: "netherBrick",
        sprite: Some("nether_brick"),
    },
    BlockEntry {
        id: 113,
        key: "netherFence",
        sprite: None,
    },
    BlockEntry {
        id: 114,
        key: "stairsNetherBrick",
        sprite: None,
    },
    BlockEntry {
        id: 115,
        key: "netherStalk",
        sprite: Some("nether_wart"),
    },
    BlockEntry {
        id: 116,
        key: "enchantmentTable",
        sprite: Some("enchanting_table"),
    },
    BlockEntry {
        id: 117,
        key: "brewingStand",
        sprite: Some("brewing_stand"),
    },
    BlockEntry {
        id: 118,
        key: "cauldron",
        sprite: Some("cauldron"),
    },
    BlockEntry {
        id: 119,
        key: "endPortal",
        sprite: None,
    },
    BlockEntry {
        id: 120,
        key: "endPortalFrame",
        sprite: Some("endframe"),
    },
    BlockEntry {
        id: 121,
        key: "whiteStone",
        sprite: Some("end_stone"),
    },
    BlockEntry {
        id: 122,
        key: "dragonEgg",
        sprite: Some("dragon_egg"),
    },
    BlockEntry {
        id: 123,
        key: "redstoneLight",
        sprite: Some("redstone_lamp_off"),
    },
    BlockEntry {
        id: 124,
        key: "redstoneLight",
        sprite: Some("redstone_lamp_on"),
    },
    BlockEntry {
        id: 125,
        key: "woodSlab",
        sprite: None,
    },
    BlockEntry {
        id: 126,
        key: "woodSlab",
        sprite: None,
    },
    BlockEntry {
        id: 127,
        key: "cocoa",
        sprite: Some("cocoa"),
    },
    BlockEntry {
        id: 128,
        key: "stairsSandStone",
        sprite: None,
    },
    BlockEntry {
        id: 129,
        key: "oreEmerald",
        sprite: Some("emerald_ore"),
    },
    BlockEntry {
        id: 130,
        key: "enderChest",
        sprite: None,
    },
    BlockEntry {
        id: 131,
        key: "tripWireSource",
        sprite: Some("trip_wire_source"),
    },
    BlockEntry {
        id: 132,
        key: "tripWire",
        sprite: Some("trip_wire"),
    },
    BlockEntry {
        id: 133,
        key: "blockEmerald",
        sprite: Some("emerald_block"),
    },
    BlockEntry {
        id: 134,
        key: "stairsWoodSpruce",
        sprite: None,
    },
    BlockEntry {
        id: 135,
        key: "stairsWoodBirch",
        sprite: None,
    },
    BlockEntry {
        id: 136,
        key: "stairsWoodJungle",
        sprite: None,
    },
    BlockEntry {
        id: 137,
        key: "commandBlock",
        sprite: Some("command_block"),
    },
    BlockEntry {
        id: 138,
        key: "beacon",
        sprite: Some("beacon"),
    },
    BlockEntry {
        id: 139,
        key: "cobbleWall",
        sprite: None,
    },
    BlockEntry {
        id: 140,
        key: "flowerPot",
        sprite: Some("flower_pot"),
    },
    BlockEntry {
        id: 141,
        key: "carrots",
        sprite: Some("carrots"),
    },
    BlockEntry {
        id: 142,
        key: "potatoes",
        sprite: Some("potatoes"),
    },
    BlockEntry {
        id: 143,
        key: "button",
        sprite: None,
    },
    BlockEntry {
        id: 144,
        key: "skull",
        sprite: None,
    },
    BlockEntry {
        id: 145,
        key: "anvil",
        sprite: None,
    },
    BlockEntry {
        id: 146,
        key: "chestTrap",
        sprite: None,
    },
    BlockEntry {
        id: 147,
        key: "weightedPlate_light",
        sprite: None,
    },
    BlockEntry {
        id: 148,
        key: "weightedPlate_heavy",
        sprite: None,
    },
    BlockEntry {
        id: 149,
        key: "comparator",
        sprite: Some("comparator_off"),
    },
    BlockEntry {
        id: 150,
        key: "comparator",
        sprite: Some("comparator_on"),
    },
    BlockEntry {
        id: 151,
        key: "daylightDetector",
        sprite: Some("daylight_detector"),
    },
    BlockEntry {
        id: 152,
        key: "blockRedstone",
        sprite: Some("redstone_block"),
    },
    BlockEntry {
        id: 153,
        key: "netherquartz",
        sprite: Some("quartz_ore"),
    },
    BlockEntry {
        id: 154,
        key: "hopper",
        sprite: None,
    },
    BlockEntry {
        id: 155,
        key: "quartzBlock",
        sprite: Some("quartz_block"),
    },
    BlockEntry {
        id: 156,
        key: "stairsQuartz",
        sprite: None,
    },
    BlockEntry {
        id: 157,
        key: "activatorRail",
        sprite: Some("rail_activator"),
    },
    BlockEntry {
        id: 158,
        key: "dropper",
        sprite: Some("dropper"),
    },
    BlockEntry {
        id: 159,
        key: "clayHardenedStained",
        sprite: Some("hardened_clay_stained"),
    },
    BlockEntry {
        id: 160,
        key: "thinStainedGlass",
        sprite: Some("glass"),
    },
    BlockEntry {
        id: 161,
        key: "leaves",
        sprite: Some("leaves"),
    },
    BlockEntry {
        id: 162,
        key: "log",
        sprite: Some("log"),
    },
    BlockEntry {
        id: 163,
        key: "stairsWoodAcacia",
        sprite: None,
    },
    BlockEntry {
        id: 164,
        key: "stairsWoodDarkOak",
        sprite: None,
    },
    BlockEntry {
        id: 170,
        key: "hayBlock",
        sprite: Some("hay_block"),
    },
    BlockEntry {
        id: 171,
        key: "woolCarpet",
        sprite: None,
    },
    BlockEntry {
        id: 172,
        key: "clayHardened",
        sprite: Some("hardened_clay"),
    },
    BlockEntry {
        id: 173,
        key: "blockCoal",
        sprite: Some("coal_block"),
    },
    BlockEntry {
        id: 174,
        key: "icePacked",
        sprite: Some("ice_packed"),
    },
    BlockEntry {
        id: 175,
        key: "doublePlant",
        sprite: None,
    },
];

/// One registered item: id, translation key, base sprite name (`None` for
/// the block items, whose sprite comes from their block), and the atlas the
/// item stitches into (`getAtlasType`: 0 blocks, 1 items).
#[derive(Clone, Copy, Debug)]
pub struct ItemEntry {
    pub id: i32,
    pub key: &'static str,
    pub sprite: Option<&'static str>,
    /// `getAtlasType`: 0 = blocks atlas, 1 = items atlas.
    pub atlas_type: u8,
}

/// The full 1.7.10 item registry, in `Item.init` order: the named items, the
/// records, then every block item the init registers (same id as its block).
pub const ITEMS: &[ItemEntry] = &[
    ItemEntry {
        id: 256,
        key: "shovelIron",
        sprite: Some("iron_shovel"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 257,
        key: "pickaxeIron",
        sprite: Some("iron_pickaxe"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 258,
        key: "hatchetIron",
        sprite: Some("iron_axe"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 259,
        key: "flintAndSteel",
        sprite: Some("flint_and_steel"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 260,
        key: "apple",
        sprite: Some("apple"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 261,
        key: "bow",
        sprite: Some("bow"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 262,
        key: "arrow",
        sprite: Some("arrow"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 263,
        key: "coal",
        sprite: Some("coal"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 264,
        key: "diamond",
        sprite: Some("diamond"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 265,
        key: "ingotIron",
        sprite: Some("iron_ingot"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 266,
        key: "ingotGold",
        sprite: Some("gold_ingot"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 267,
        key: "swordIron",
        sprite: Some("iron_sword"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 268,
        key: "swordWood",
        sprite: Some("wood_sword"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 269,
        key: "shovelWood",
        sprite: Some("wood_shovel"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 270,
        key: "pickaxeWood",
        sprite: Some("wood_pickaxe"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 271,
        key: "hatchetWood",
        sprite: Some("wood_axe"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 272,
        key: "swordStone",
        sprite: Some("stone_sword"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 273,
        key: "shovelStone",
        sprite: Some("stone_shovel"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 274,
        key: "pickaxeStone",
        sprite: Some("stone_pickaxe"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 275,
        key: "hatchetStone",
        sprite: Some("stone_axe"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 276,
        key: "swordDiamond",
        sprite: Some("diamond_sword"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 277,
        key: "shovelDiamond",
        sprite: Some("diamond_shovel"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 278,
        key: "pickaxeDiamond",
        sprite: Some("diamond_pickaxe"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 279,
        key: "hatchetDiamond",
        sprite: Some("diamond_axe"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 280,
        key: "stick",
        sprite: Some("stick"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 281,
        key: "bowl",
        sprite: Some("bowl"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 282,
        key: "mushroomStew",
        sprite: Some("mushroom_stew"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 283,
        key: "swordGold",
        sprite: Some("gold_sword"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 284,
        key: "shovelGold",
        sprite: Some("gold_shovel"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 285,
        key: "pickaxeGold",
        sprite: Some("gold_pickaxe"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 286,
        key: "hatchetGold",
        sprite: Some("gold_axe"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 287,
        key: "string",
        sprite: Some("string"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 288,
        key: "feather",
        sprite: Some("feather"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 289,
        key: "sulphur",
        sprite: Some("gunpowder"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 290,
        key: "hoeWood",
        sprite: Some("wood_hoe"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 291,
        key: "hoeStone",
        sprite: Some("stone_hoe"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 292,
        key: "hoeIron",
        sprite: Some("iron_hoe"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 293,
        key: "hoeDiamond",
        sprite: Some("diamond_hoe"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 294,
        key: "hoeGold",
        sprite: Some("gold_hoe"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 295,
        key: "seeds",
        sprite: Some("seeds_wheat"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 296,
        key: "wheat",
        sprite: Some("wheat"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 297,
        key: "bread",
        sprite: Some("bread"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 298,
        key: "helmetCloth",
        sprite: Some("leather_helmet"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 299,
        key: "chestplateCloth",
        sprite: Some("leather_chestplate"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 300,
        key: "leggingsCloth",
        sprite: Some("leather_leggings"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 301,
        key: "bootsCloth",
        sprite: Some("leather_boots"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 302,
        key: "helmetChain",
        sprite: Some("chainmail_helmet"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 303,
        key: "chestplateChain",
        sprite: Some("chainmail_chestplate"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 304,
        key: "leggingsChain",
        sprite: Some("chainmail_leggings"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 305,
        key: "bootsChain",
        sprite: Some("chainmail_boots"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 306,
        key: "helmetIron",
        sprite: Some("iron_helmet"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 307,
        key: "chestplateIron",
        sprite: Some("iron_chestplate"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 308,
        key: "leggingsIron",
        sprite: Some("iron_leggings"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 309,
        key: "bootsIron",
        sprite: Some("iron_boots"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 310,
        key: "helmetDiamond",
        sprite: Some("diamond_helmet"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 311,
        key: "chestplateDiamond",
        sprite: Some("diamond_chestplate"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 312,
        key: "leggingsDiamond",
        sprite: Some("diamond_leggings"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 313,
        key: "bootsDiamond",
        sprite: Some("diamond_boots"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 314,
        key: "helmetGold",
        sprite: Some("gold_helmet"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 315,
        key: "chestplateGold",
        sprite: Some("gold_chestplate"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 316,
        key: "leggingsGold",
        sprite: Some("gold_leggings"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 317,
        key: "bootsGold",
        sprite: Some("gold_boots"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 318,
        key: "flint",
        sprite: Some("flint"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 319,
        key: "porkchopRaw",
        sprite: Some("porkchop_raw"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 320,
        key: "porkchopCooked",
        sprite: Some("porkchop_cooked"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 321,
        key: "painting",
        sprite: Some("painting"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 322,
        key: "appleGold",
        sprite: Some("apple_golden"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 323,
        key: "sign",
        sprite: Some("sign"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 324,
        key: "doorWood",
        sprite: Some("door_wood"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 325,
        key: "bucket",
        sprite: Some("bucket_empty"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 326,
        key: "bucketWater",
        sprite: Some("bucket_water"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 327,
        key: "bucketLava",
        sprite: Some("bucket_lava"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 328,
        key: "minecart",
        sprite: Some("minecart_normal"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 329,
        key: "saddle",
        sprite: Some("saddle"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 330,
        key: "doorIron",
        sprite: Some("door_iron"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 331,
        key: "redstone",
        sprite: Some("redstone_dust"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 332,
        key: "snowball",
        sprite: Some("snowball"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 333,
        key: "boat",
        sprite: Some("boat"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 334,
        key: "leather",
        sprite: Some("leather"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 335,
        key: "milk",
        sprite: Some("bucket_milk"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 336,
        key: "brick",
        sprite: Some("brick"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 337,
        key: "clay",
        sprite: Some("clay_ball"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 338,
        key: "reeds",
        sprite: Some("reeds"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 339,
        key: "paper",
        sprite: Some("paper"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 340,
        key: "book",
        sprite: Some("book_normal"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 341,
        key: "slimeball",
        sprite: Some("slimeball"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 342,
        key: "minecartChest",
        sprite: Some("minecart_chest"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 343,
        key: "minecartFurnace",
        sprite: Some("minecart_furnace"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 344,
        key: "egg",
        sprite: Some("egg"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 345,
        key: "compass",
        sprite: Some("compass"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 346,
        key: "fishingRod",
        sprite: Some("fishing_rod"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 347,
        key: "clock",
        sprite: Some("clock"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 348,
        key: "yellowDust",
        sprite: Some("glowstone_dust"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 349,
        key: "fish",
        sprite: Some("fish_raw"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 350,
        key: "fish",
        sprite: Some("fish_cooked"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 351,
        key: "dyePowder",
        sprite: Some("dye_powder"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 352,
        key: "bone",
        sprite: Some("bone"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 353,
        key: "sugar",
        sprite: Some("sugar"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 354,
        key: "cake",
        sprite: Some("cake"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 355,
        key: "bed",
        sprite: Some("bed"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 356,
        key: "diode",
        sprite: Some("repeater"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 357,
        key: "cookie",
        sprite: Some("cookie"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 358,
        key: "map",
        sprite: Some("map_filled"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 359,
        key: "shears",
        sprite: Some("shears"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 360,
        key: "melon",
        sprite: Some("melon"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 361,
        key: "seeds_pumpkin",
        sprite: Some("seeds_pumpkin"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 362,
        key: "seeds_melon",
        sprite: Some("seeds_melon"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 363,
        key: "beefRaw",
        sprite: Some("beef_raw"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 364,
        key: "beefCooked",
        sprite: Some("beef_cooked"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 365,
        key: "chickenRaw",
        sprite: Some("chicken_raw"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 366,
        key: "chickenCooked",
        sprite: Some("chicken_cooked"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 367,
        key: "rottenFlesh",
        sprite: Some("rotten_flesh"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 368,
        key: "enderPearl",
        sprite: Some("ender_pearl"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 369,
        key: "blazeRod",
        sprite: Some("blaze_rod"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 370,
        key: "ghastTear",
        sprite: Some("ghast_tear"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 371,
        key: "goldNugget",
        sprite: Some("gold_nugget"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 372,
        key: "netherStalkSeeds",
        sprite: Some("nether_wart"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 373,
        key: "potion",
        sprite: Some("potion"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 374,
        key: "glassBottle",
        sprite: Some("potion_bottle_empty"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 375,
        key: "spiderEye",
        sprite: Some("spider_eye"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 376,
        key: "fermentedSpiderEye",
        sprite: Some("spider_eye_fermented"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 377,
        key: "blazePowder",
        sprite: Some("blaze_powder"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 378,
        key: "magmaCream",
        sprite: Some("magma_cream"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 379,
        key: "brewingStand",
        sprite: Some("brewing_stand"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 380,
        key: "cauldron",
        sprite: Some("cauldron"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 381,
        key: "eyeOfEnder",
        sprite: Some("ender_eye"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 382,
        key: "speckledMelon",
        sprite: Some("melon_speckled"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 383,
        key: "monsterPlacer",
        sprite: Some("spawn_egg"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 384,
        key: "expBottle",
        sprite: Some("experience_bottle"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 385,
        key: "fireball",
        sprite: Some("fireball"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 386,
        key: "writingBook",
        sprite: Some("book_writable"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 387,
        key: "writtenBook",
        sprite: Some("book_written"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 388,
        key: "emerald",
        sprite: Some("emerald"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 389,
        key: "frame",
        sprite: Some("item_frame"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 390,
        key: "flowerPot",
        sprite: Some("flower_pot"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 391,
        key: "carrots",
        sprite: Some("carrot"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 392,
        key: "potato",
        sprite: Some("potato"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 393,
        key: "potatoBaked",
        sprite: Some("potato_baked"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 394,
        key: "potatoPoisonous",
        sprite: Some("potato_poisonous"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 395,
        key: "emptyMap",
        sprite: Some("map_empty"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 396,
        key: "carrotGolden",
        sprite: Some("carrot_golden"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 397,
        key: "skull",
        sprite: Some("skull"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 398,
        key: "carrotOnAStick",
        sprite: Some("carrot_on_a_stick"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 399,
        key: "netherStar",
        sprite: Some("nether_star"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 400,
        key: "pumpkinPie",
        sprite: Some("pumpkin_pie"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 401,
        key: "fireworks",
        sprite: Some("fireworks"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 402,
        key: "fireworksCharge",
        sprite: Some("fireworks_charge"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 403,
        key: "enchantedBook",
        sprite: Some("book_enchanted"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 404,
        key: "comparator",
        sprite: Some("comparator"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 405,
        key: "netherbrick",
        sprite: Some("netherbrick"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 406,
        key: "netherquartz",
        sprite: Some("quartz"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 407,
        key: "minecartTnt",
        sprite: Some("minecart_tnt"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 408,
        key: "minecartHopper",
        sprite: Some("minecart_hopper"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 417,
        key: "horsearmormetal",
        sprite: Some("iron_horse_armor"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 418,
        key: "horsearmorgold",
        sprite: Some("gold_horse_armor"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 419,
        key: "horsearmordiamond",
        sprite: Some("diamond_horse_armor"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 420,
        key: "leash",
        sprite: Some("lead"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 421,
        key: "nameTag",
        sprite: Some("name_tag"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 422,
        key: "minecartCommandBlock",
        sprite: Some("minecart_command_block"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 2256,
        key: "record",
        sprite: Some("record_13"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 2257,
        key: "record",
        sprite: Some("record_cat"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 2258,
        key: "record",
        sprite: Some("record_blocks"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 2259,
        key: "record",
        sprite: Some("record_chirp"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 2260,
        key: "record",
        sprite: Some("record_far"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 2261,
        key: "record",
        sprite: Some("record_mall"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 2262,
        key: "record",
        sprite: Some("record_mellohi"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 2263,
        key: "record",
        sprite: Some("record_stal"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 2264,
        key: "record",
        sprite: Some("record_strad"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 2265,
        key: "record",
        sprite: Some("record_ward"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 2266,
        key: "record",
        sprite: Some("record_11"),
        atlas_type: 1,
    },
    ItemEntry {
        id: 2267,
        key: "record",
        sprite: Some("record_wait"),
        atlas_type: 1,
    },
];

/// `PlanksBlock.VARIANTS` (also the planks item's variant names).
pub const PLANKS_VARIANTS: [&str; 6] = ["oak", "spruce", "birch", "jungle", "acacia", "big_oak"];
/// `LogBlock.VARIANTS`.
pub const LOG_VARIANTS: [&str; 4] = ["oak", "spruce", "birch", "jungle"];
/// `Log2Block.VARIANTS`.
pub const LOG2_VARIANTS: [&str; 2] = ["acacia", "big_oak"];
/// `SaplingBlock.VARIANTS`.
pub const SAPLING_VARIANTS: [&str; 6] =
    ["oak", "spruce", "birch", "jungle", "acacia", "roofed_oak"];
/// `LeavesBlock.SPRITE_NAMES` (transparent variants; the opaque row is
/// `SPRITE_NAMES` with `_opaque`).
pub const LEAVES_SPRITES: [&str; 4] = [
    "leaves_oak",
    "leaves_spruce",
    "leaves_birch",
    "leaves_jungle",
];
/// `LeavesBlock.SPRITE_NAMES` row 1.
pub const LEAVES_OPAQUE_SPRITES: [&str; 4] = [
    "leaves_oak_opaque",
    "leaves_spruce_opaque",
    "leaves_birch_opaque",
    "leaves_jungle_opaque",
];
/// `Leaves2Block.BLOCK_IDS` row 0.
pub const LEAVES2_SPRITES: [&str; 2] = ["leaves_acacia", "leaves_big_oak"];
/// `Leaves2Block.BLOCK_IDS` row 1.
pub const LEAVES2_OPAQUE_SPRITES: [&str; 2] = ["leaves_acacia_opaque", "leaves_big_oak_opaque"];
/// `FlowerBlock.SPRITES[1]` (the red flower's 9 sprites, metadata 0..8).
pub const RED_FLOWER_SPRITES: [&str; 9] = [
    "flower_rose",
    "flower_blue_orchid",
    "flower_allium",
    "flower_houstonia",
    "flower_tulip_red",
    "flower_tulip_orange",
    "flower_tulip_white",
    "flower_tulip_pink",
    "flower_oxeye_daisy",
];
/// `TallPlantBlock.SPRITE_NAMES` (the tallgrass block's three sprites).
pub const TALLGRASS_SPRITES: [&str; 3] = ["deadbush", "tallgrass", "fern"];
/// `DoublePlantBlock.TEXTURES`.
pub const DOUBLE_PLANT_TEXTURES: [&str; 6] =
    ["sunflower", "syringa", "grass", "fern", "rose", "paeonia"];
/// `SandstoneBlock.SPRITE_NAMES`.
pub const SANDSTONE_SPRITES: [&str; 3] = ["normal", "carved", "smooth"];
/// `QuartzBlock.SPRITE_NAMES` (the two `None`s repeat the previous sprite).
pub const QUARTZ_SPRITES: [&str; 3] = ["side", "chiseled", "lines"];
/// `StonebrickBlock.SPRITE_NAMES` (`None` is the plain stone brick).
pub const STONEBRICK_SPRITES: [Option<&str>; 4] =
    [None, Some("mossy"), Some("cracked"), Some("carved")];
/// `AnvilBlock.TOP_SPRITE_NAMES`.
pub const ANVIL_TOP_SPRITES: [&str; 3] = [
    "anvil_top_damaged_0",
    "anvil_top_damaged_1",
    "anvil_top_damaged_2",
];
/// `FishItem$Type`: id, key, cooked flag (`canBeCooked`).
pub const FISH_TYPES: [(i32, &str, bool); 4] = [
    (0, "cod", true),
    (1, "salmon", true),
    (2, "clownfish", false),
    (3, "pufferfish", false),
];
/// `SkullItem.PLAYER_SKULL_TYPES`.
pub const SKULL_TYPES: [&str; 5] = ["skeleton", "wither", "zombie", "steve", "creeper"];
/// `BowItem.BOW_PULLING_LEVELS`.
pub const BOW_PULLING_LEVELS: [&str; 3] = ["pulling_0", "pulling_1", "pulling_2"];
/// `ArmorItem.OVERLAYS` (leather only, layer 1).
pub const LEATHER_ARMOR_OVERLAYS: [&str; 4] = [
    "leather_helmet_overlay",
    "leather_chestplate_overlay",
    "leather_leggings_overlay",
    "leather_boots_overlay",
];

/// `Block.getSpriteName`'s fallback: the vanilla init leaves the sprite
/// unset for these blocks, so `registerSprite` receives this name.
pub fn missing_icon_block_name(id: i32) -> String {
    let key = block(id).map(|entry| entry.key).unwrap_or("");
    format!("MISSING_ICON_BLOCK_{id}_{key}")
}

/// The block with `id` (`Block.REGISTRY.get`).
pub fn block(id: i32) -> Option<&'static BlockEntry> {
    BLOCKS.iter().find(|entry| entry.id == id)
}

/// The item with `id` (`Item.REGISTRY.get`).
pub fn item(id: i32) -> Option<&'static ItemEntry> {
    ITEMS.iter().find(|entry| entry.id == id)
}

/// The registered block count (every entry of `BLOCKS`).
pub fn block_count() -> usize {
    BLOCKS.len()
}

/// The registered item count (every entry of `ITEMS`).
pub fn item_count() -> usize {
    ITEMS.len()
}

fn clamp(value: i32, min: i32, max: i32) -> i32 {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

fn clamped(list: &[&'static str], index: i32) -> &'static str {
    list[clamp(index, 0, list.len() as i32 - 1) as usize]
}

/// The full name a sprite resolves to: `TextureAtlas.getResourceId` —
/// `textures/blocks/<path>.png` or `textures/items/<path>.png` under the
/// sprite's own namespace (`<ns>:<name>` sprites keep the namespace).
pub fn texture_location(atlas_type: u8, sprite_name: &str) -> Identifier {
    let id = Identifier::parse(sprite_name);
    let atlas = if atlas_type == 0 {
        BLOCKS_ATLAS_PATH
    } else {
        ITEMS_ATLAS_PATH
    };
    Identifier::new(id.namespace(), &format!("{atlas}/{}.png", id.path()))
}

/// `TextureAtlas.registerSprite`'s validation: a sprite name must not
/// contain a slash, in either direction.
pub fn sprite_name_is_valid(name: &str) -> bool {
    !name.contains('/') && !name.contains('\\')
}

/// The sprite names `TextureAtlas.registerSprite` special-cases on the
/// items atlas: the clock and the compass are dynamic sprites.
pub fn is_dynamic_item_sprite(name: &str) -> bool {
    name == "clock" || name == "compass"
}

/// The registered sprite name of block `id` for `face` and `metadata`
/// (`Block.getSprite(face, metadata)` plus every client
/// `registerSprites`/`getSprite` override). `None` when the block does not
/// register a sprite at all (air, and the overrides that stay empty).
pub fn block_texture(id: i32, metadata: i32, face: u8) -> Option<String> {
    let entry = block(id)?;
    let base = entry.sprite;
    Some(match id {
        2 => {
            // GrassBlock.getSprite: top, dirt under it, grass side else.
            if face == FACE_TOP {
                "grass_top".to_owned()
            } else if face == FACE_BOTTOM {
                "dirt".to_owned()
            } else {
                "grass_side".to_owned()
            }
        }
        3 => {
            // DirtBlock.getSprite: podzol for metadata 2.
            if metadata == 2 {
                if face == FACE_TOP {
                    "dirt_podzol_top".to_owned()
                } else if face != FACE_BOTTOM {
                    "dirt_podzol_side".to_owned()
                } else {
                    "dirt".to_owned()
                }
            } else {
                "dirt".to_owned()
            }
        }
        5 => format!(
            "planks_{}",
            PLANKS_VARIANTS[clamp(metadata, 0, PLANKS_VARIANTS.len() as i32 - 1) as usize]
        ),
        6 => format!("sapling_{}", clamped(&SAPLING_VARIANTS, metadata & 7)),
        8 | 9 => {
            // LiquidBlock.getSprite: still on the top and bottom faces,
            // flow on the sides.
            if face == FACE_TOP || face == FACE_BOTTOM {
                "water_still".to_owned()
            } else {
                "water_flow".to_owned()
            }
        }
        10 | 11 => {
            if face == FACE_TOP || face == FACE_BOTTOM {
                "lava_still".to_owned()
            } else {
                "lava_flow".to_owned()
            }
        }
        12 => {
            // SandBlock.registerSprites: metadata 1 is red sand.
            if metadata == 1 {
                "red_sand".to_owned()
            } else {
                "sand".to_owned()
            }
        }
        17 | 162 => {
            // LogBlock/Log2Block via AxisBlock.getSprite: the axis in bits
            // 2-3 turns the axis faces to the top sprite; bits 0-1 pick the
            // wood.
            let variants: &[&str] = if id == 17 {
                &LOG_VARIANTS
            } else {
                &LOG2_VARIANTS
            };
            let wood = clamp(metadata & 3, 0, variants.len() as i32 - 1) as usize;
            let axis = metadata & 12;
            let top = (axis == 0 && (face == FACE_TOP || face == FACE_BOTTOM))
                || (axis == 4 && (face == FACE_WEST || face == FACE_EAST))
                || (axis == 8 && (face == FACE_NORTH || face == FACE_SOUTH));
            let name = format!("{}_{}", base.expect("log base"), variants[wood]);
            if top { format!("{name}_top") } else { name }
        }
        18 | 161 => {
            // LeavesBlock/Leaves2Block: metadata & 3 picks the wood;
            // spriteIndex 0 (the default) is the transparent row.
            let transparent: &[&str] = if id == 18 {
                &LEAVES_SPRITES
            } else {
                &LEAVES2_SPRITES
            };
            clamped(transparent, metadata & 3).to_owned()
        }
        23 => {
            // DispenserBlock.getSprite: the front face by facing metadata,
            // furnace sides and top elsewhere.
            let facing = metadata & 7;
            if face as i32 == facing {
                if facing == FACE_TOP as i32 || facing == FACE_BOTTOM as i32 {
                    "dispenser_front_vertical".to_owned()
                } else {
                    "dispenser_front_horizontal".to_owned()
                }
            } else if facing == FACE_TOP as i32
                || facing == FACE_BOTTOM as i32
                || face == FACE_TOP
                || face == FACE_BOTTOM
            {
                "furnace_top".to_owned()
            } else {
                "furnace_side".to_owned()
            }
        }
        24 => {
            // SandstoneBlock.getSprite.
            if face == FACE_TOP || (face == FACE_BOTTOM && (metadata == 1 || metadata == 2)) {
                "sandstone_top".to_owned()
            } else if face == FACE_BOTTOM {
                "sandstone_bottom".to_owned()
            } else {
                format!("sandstone_{}", clamped(&SANDSTONE_SPRITES, metadata))
            }
        }
        27 | 157 => {
            // PoweredRailBlock.registerSprites overrides the sprite with
            // the powered one.
            format!("{}_powered", base.expect("rail base"))
        }
        28 => {
            // DetectorRailBlock.getSprite: powered bit 8.
            let base = base.expect("rail base");
            if metadata & 8 != 0 {
                format!("{base}_powered")
            } else {
                base.to_owned()
            }
        }
        29 | 33 => {
            // PistonBaseBlock.getSprite with `isExtended` false: the facing
            // face takes the top sprite, the opposite face the bottom, else
            // the side.
            let facing = metadata & 7;
            if facing > 5 || face == facing as u8 {
                top_sprite_of_piston(id).to_owned()
            } else if face == OPPOSITE[facing as usize] {
                "piston_bottom".to_owned()
            } else {
                "piston_side".to_owned()
            }
        }
        31 => clamped(&TALLGRASS_SPRITES, metadata).to_owned(),
        35 => format!(
            "wool_colored_{}",
            DYE_COLOR_NAMES[dye_color_of_block_metadata(metadata)]
        ),
        36 => {
            // MovingBlock.registerSprites.
            "piston_top_normal".to_owned()
        }
        37 => "flower_dandelion".to_owned(),
        38 => clamped(&RED_FLOWER_SPRITES, metadata).to_owned(),
        43 | 44 => {
            // StoneSlabBlock.getSprite: metadata & 7 picks the material,
            // bit 8 (on the double slab) forces the top sprite.
            let variant = metadata & 7;
            let double = id == 43 && metadata & 8 != 0;
            let face = if double { FACE_TOP } else { face };
            match variant {
                0 => {
                    if face == FACE_TOP || face == FACE_BOTTOM {
                        "stone_slab_top".to_owned()
                    } else {
                        "stone_slab_side".to_owned()
                    }
                }
                1 => block_texture(24, 0, face).unwrap(),
                2 => block_texture(5, 0, face).unwrap(),
                3 => "cobblestone".to_owned(),
                4 => "brick".to_owned(),
                5 => block_texture(98, 0, face).unwrap(),
                6 => "nether_brick".to_owned(),
                _ => block_texture(155, 0, face).unwrap(),
            }
        }
        50 => "torch_on".to_owned(),
        51 => "fire_layer_0".to_owned(),
        53 | 67 | 108 | 109 | 114 | 128 | 134 | 135 | 136 | 156 | 163 | 164 => {
            // StairsBlock.getSprite: the parent block's sprite for the
            // stairs' fixed parent metadata.
            let (parent, parent_meta) = stairs_parent(id);
            block_texture(parent, parent_meta, face)?
        }
        54 | 146 => {
            // ChestBlock.registerSprites.
            "planks_oak".to_owned()
        }
        55 => "redstone_dust_cross".to_owned(),
        59 => format!("wheat_stage_{}", clamp(metadata, 0, 7)),
        60 => {
            // FarmlandBlock.getSprite: moisture in the metadata.
            if face == FACE_TOP {
                if metadata > 0 {
                    "farmland_wet".to_owned()
                } else {
                    "farmland_dry".to_owned()
                }
            } else {
                "dirt".to_owned()
            }
        }
        61 | 62 => {
            // FurnaceBlock.getSprite: the front face by facing metadata.
            if face == FACE_TOP || face == FACE_BOTTOM {
                "furnace_top".to_owned()
            } else if face as i32 != metadata {
                "furnace_side".to_owned()
            } else if id == 61 {
                "furnace_front_off".to_owned()
            } else {
                "furnace_front_on".to_owned()
            }
        }
        64 | 71 => {
            // DoorBlock.getSprite: bit 8 is the upper half.
            let upper = metadata & 8 != 0;
            let name = if id == 64 { "door_wood" } else { "door_iron" };
            if upper {
                format!("{name}_upper")
            } else {
                format!("{name}_lower")
            }
        }
        81 => {
            // CactusBlock.getSprite.
            if face == FACE_TOP {
                "cactus_top".to_owned()
            } else if face == FACE_BOTTOM {
                "cactus_bottom".to_owned()
            } else {
                "cactus_side".to_owned()
            }
        }
        84 => {
            // JukeboxBlock.getSprite.
            if face == FACE_TOP {
                "jukebox_top".to_owned()
            } else {
                "jukebox_side".to_owned()
            }
        }
        85 => "planks_oak".to_owned(),
        86 | 91 => {
            // PumpkinBlock.getSprite: the face sprite on the facing side.
            let lit = id == 91;
            let face_sprite = if lit {
                "pumpkin_face_on"
            } else {
                "pumpkin_face_off"
            };
            if face == FACE_TOP || face == FACE_BOTTOM {
                "pumpkin_top".to_owned()
            } else if (metadata == 2 && face == FACE_NORTH)
                || (metadata == 3 && face == FACE_EAST)
                || (metadata == 0 && face == FACE_SOUTH)
                || (metadata == 1 && face == FACE_WEST)
            {
                face_sprite.to_owned()
            } else {
                "pumpkin_side".to_owned()
            }
        }
        92 => {
            // CakeBlock.getSprite: the bitten inner face on the west.
            if face == FACE_TOP {
                "cake_top".to_owned()
            } else if face == FACE_BOTTOM {
                "cake_bottom".to_owned()
            } else if metadata > 0 && face == FACE_WEST {
                "cake_inner".to_owned()
            } else {
                "cake_side".to_owned()
            }
        }
        93 | 94 => {
            // RepeaterBlock: no getSprite override; the base sprite for
            // every face and metadata.
            base?.to_owned()
        }
        95 => format!(
            "glass_{}",
            DYE_COLOR_NAMES[dye_color_of_block_metadata(metadata)]
        ),
        97 => {
            // InfestedBlock.getSprite: the disguised block's sprite.
            match metadata {
                1 => "cobblestone".to_owned(),
                2 => "stonebrick".to_owned(),
                3 => "stonebrick_mossy".to_owned(),
                4 => "stonebrick_cracked".to_owned(),
                5 => "stonebrick_carved".to_owned(),
                _ => "stone".to_owned(),
            }
        }
        98 => {
            // StonebrickBlock.registerSprites: `None` is the plain brick.
            let name = STONEBRICK_SPRITES
                [clamp(metadata, 0, STONEBRICK_SPRITES.len() as i32 - 1) as usize];
            match name {
                Some(suffix) => format!("stonebrick_{suffix}"),
                None => "stonebrick".to_owned(),
            }
        }
        99 | 100 => {
            // MushroomBlock.getSprite: the skin shows on the metadata's
            // faces, the stem on 10 and 15, the inside elsewhere.
            let variant = if id == 99 {
                "mushroom_block_skin_brown"
            } else {
                "mushroom_block_skin_red"
            };
            let face_shown = ((1..=9).contains(&metadata) && face == FACE_TOP)
                || ((1..=3).contains(&metadata) && face == FACE_NORTH)
                || ((7..=9).contains(&metadata) && face == FACE_SOUTH)
                || (matches!(metadata, 1 | 4 | 7) && face == FACE_WEST)
                || (matches!(metadata, 3 | 6 | 9) && face == FACE_EAST)
                || metadata == 14;
            if metadata == 10 && face > FACE_TOP {
                "mushroom_block_skin_stem".to_owned()
            } else if face_shown {
                variant.to_owned()
            } else if metadata == 15 {
                "mushroom_block_skin_stem".to_owned()
            } else {
                "mushroom_block_inside".to_owned()
            }
        }
        101 | 102 => {
            // PaneBlock.registerSprites: side and top names from the
            // constructor.
            if id == 101 {
                "iron_bars".to_owned()
            } else if face == FACE_TOP || face == FACE_BOTTOM {
                "glass_pane_top".to_owned()
            } else {
                "glass".to_owned()
            }
        }
        103 => {
            // MelonBlock.getSprite.
            if face == FACE_TOP || face == FACE_BOTTOM {
                "melon_top".to_owned()
            } else {
                "melon_side".to_owned()
            }
        }
        104 | 105 => format!("{}_disconnected", base.expect("stem base")),
        107 => {
            // FenceGateBlock.registerSprites is empty: no sprite.
            return None;
        }
        110 => {
            // MyceliumBlock.getSprite.
            if face == FACE_TOP {
                "mycelium_top".to_owned()
            } else if face == FACE_BOTTOM {
                "dirt".to_owned()
            } else {
                "mycelium_side".to_owned()
            }
        }
        113 => "nether_brick".to_owned(),
        115 => format!("nether_wart_stage_{}", clamp(metadata, 0, 2)),
        116 => {
            // EnchantingTableBlock.registerSprites.
            if face == FACE_TOP {
                "enchanting_table_top".to_owned()
            } else if face == FACE_BOTTOM {
                "enchanting_table_bottom".to_owned()
            } else {
                "enchanting_table_side".to_owned()
            }
        }
        117 => "brewing_stand".to_owned(),
        118 => "cauldron_side".to_owned(),
        119 => "portal".to_owned(),
        120 => {
            // EndPortalFrameBlock.getSprite: end stone under it.
            if face == FACE_TOP {
                "endframe_top".to_owned()
            } else if face == FACE_BOTTOM {
                "end_stone".to_owned()
            } else {
                "endframe_side".to_owned()
            }
        }
        125 | 126 => {
            // WoodenSlabBlock.getSprite: the planks sprite of metadata & 7.
            block_texture(5, metadata & 7, face).unwrap()
        }
        127 => "cocoa_stage_2".to_owned(),
        130 => "obsidian".to_owned(),
        139 => {
            // WallBlock.getSprite: metadata 1 is the mossy wall.
            if metadata == 1 {
                "cobblestone_mossy".to_owned()
            } else {
                "cobblestone".to_owned()
            }
        }
        141 => format!("carrots_stage_{}", clamp(metadata, 0, 3)),
        142 => format!("potatoes_stage_{}", clamp(metadata, 0, 3)),
        144 => {
            // SkullBlock.getSprite: the soul sand's sprite.
            block_texture(88, 0, face).unwrap()
        }
        145 => {
            // AnvilBlock.getSprite: the top sprite on the top face of the
            // body render part, by damage.
            if face == FACE_TOP {
                ANVIL_TOP_SPRITES[clamp((metadata >> 2) % 3, 0, 2) as usize].to_owned()
            } else {
                "anvil_base".to_owned()
            }
        }
        147 => "gold_block".to_owned(),
        148 => "iron_block".to_owned(),
        149 | 150 => {
            // ComparatorBlock.getSprite: the torch sprite under it, the
            // own sprite on top, the stone slab elsewhere.
            let lit = id == 150 || metadata & 8 != 0;
            if face == FACE_BOTTOM {
                if lit {
                    block_texture(76, 0, face)?
                } else {
                    block_texture(75, 0, face)?
                }
            } else if face == FACE_TOP {
                if id == 149 {
                    "comparator_off".to_owned()
                } else {
                    "comparator_on".to_owned()
                }
            } else {
                "stone_slab_top".to_owned()
            }
        }
        151 => {
            // DaylightDetectorBlock.registerSprites.
            if face == FACE_TOP {
                "daylight_detector_top".to_owned()
            } else {
                "daylight_detector_side".to_owned()
            }
        }
        154 => {
            // HopperBlock.getSprite.
            if face == FACE_TOP {
                "hopper_top".to_owned()
            } else {
                "hopper_outside".to_owned()
            }
        }
        155 => {
            // QuartzBlock.getSprite: metadata 2-4 turn the axis faces to
            // the chiseled/lines top sprites.
            let side = if (0..5).contains(&metadata) {
                metadata
            } else {
                0
            };
            if !(2..=4).contains(&side) {
                if face == FACE_TOP {
                    "quartz_block_top".to_owned()
                } else if face == FACE_BOTTOM {
                    "quartz_block_bottom".to_owned()
                } else {
                    format!(
                        "quartz_block_{}",
                        QUARTZ_SPRITES[clamp(side, 0, QUARTZ_SPRITES.len() as i32 - 1) as usize]
                    )
                }
            } else if side == 2 && (face == FACE_TOP || face == FACE_BOTTOM) {
                "quartz_block_chiseled_top".to_owned()
            } else if matches!(side, 3 | 4)
                && ((side == 3 && (face == FACE_WEST || face == FACE_EAST))
                    || (side == 4 && (face == FACE_NORTH || face == FACE_SOUTH)))
            {
                "quartz_block_lines_top".to_owned()
            } else {
                format!(
                    "quartz_block_{}",
                    QUARTZ_SPRITES[clamp(side, 0, QUARTZ_SPRITES.len() as i32 - 1) as usize]
                )
            }
        }
        158 => {
            // DropperBlock.getSprite: like the dispenser, with the dropper
            // front.
            let facing = metadata & 7;
            if face as i32 == facing {
                if facing == FACE_TOP as i32 || facing == FACE_BOTTOM as i32 {
                    "dropper_front_vertical".to_owned()
                } else {
                    "dropper_front_horizontal".to_owned()
                }
            } else if facing == FACE_TOP as i32
                || facing == FACE_BOTTOM as i32
                || face == FACE_TOP
                || face == FACE_BOTTOM
            {
                "furnace_top".to_owned()
            } else {
                "furnace_side".to_owned()
            }
        }
        159 => format!(
            "hardened_clay_stained_{}",
            DYE_COLOR_NAMES[dye_color_of_block_metadata(metadata)]
        ),
        160 => format!(
            "glass_{}",
            DYE_COLOR_NAMES[dye_color_of_block_metadata(metadata)]
        ),
        170 => {
            // HayBlock.getSprite.
            if face == FACE_TOP || face == FACE_BOTTOM {
                "hay_block_top".to_owned()
            } else {
                "hay_block_side".to_owned()
            }
        }
        171 => {
            // CarpetBlock.getSprite: the wool sprite of the metadata.
            block_texture(35, metadata, face).unwrap()
        }
        175 => {
            // DoublePlantBlock.getSprite: bit 8 is the bottom half (vanilla
            // names it so), which takes the sunflower's bottom sprite; the
            // variant is metadata & 7.
            let bottom_half = metadata & 8 != 0;
            if bottom_half {
                "double_plant_sunflower_bottom".to_owned()
            } else {
                format!(
                    "double_plant_{}_bottom",
                    DOUBLE_PLANT_TEXTURES[clamp(metadata & 7, 0, 5) as usize]
                )
            }
        }
        _ => base?.to_owned(),
    })
}

/// `StairsBlock`'s parent block and fixed parent metadata, from the local
/// variables of `Block.init` (`new StairsBlock(var, type)`).
fn stairs_parent(id: i32) -> (i32, i32) {
    match id {
        53 => (5, 0),    // oak_stairs: planks
        67 => (4, 0),    // stone_stairs: cobblestone
        108 => (45, 0),  // brick_stairs: brick
        109 => (98, 0),  // stone_brick_stairs: stonebrick
        114 => (112, 0), // nether_brick_stairs: nether brick
        128 => (24, 0),  // sandstone_stairs: sandstone
        134 => (5, 1),   // spruce_stairs: planks spruce
        135 => (5, 2),   // birch_stairs: planks birch
        136 => (5, 3),   // jungle_stairs: planks jungle
        156 => (155, 0), // quartz_stairs: quartz block
        163 => (5, 4),   // acacia_stairs: planks acacia
        _ => (5, 5),     // 164 dark_oak_stairs: planks big_oak
    }
}

fn top_sprite_of_piston(id: i32) -> &'static str {
    if id == 29 {
        "piston_top_sticky"
    } else {
        "piston_top_normal"
    }
}

/// The leaves' opaque-graphics sprite (`spriteIndex` 1): the transparent
/// names with `_opaque`. `block_texture` uses spriteIndex 0, the default.
pub fn leaves_opaque_texture(id: i32, metadata: i32) -> Option<&'static str> {
    if id == 18 {
        Some(clamped(&LEAVES_OPAQUE_SPRITES, metadata & 3))
    } else if id == 161 {
        Some(clamped(&LEAVES2_OPAQUE_SPRITES, metadata & 3))
    } else {
        None
    }
}

/// The blocks `Item.init` gives no item (`var1` in the init, plus air).
const BLOCKS_WITHOUT_ITEM: [i32; 27] = [
    0, 26, 34, 36, 55, 59, 63, 64, 68, 71, 74, 75, 83, 92, 93, 94, 104, 105, 115, 117, 118, 124,
    132, 140, 144, 149, 150,
];

/// The `ColoredBlockItem` blocks: item metadata inverted to block metadata.
const COLORED_ITEM_BLOCKS: [i32; 5] = [35, 95, 159, 160, 171];

/// The `VariantBlockItem` blocks (`spriteProvider` is the block itself).
const VARIANT_ITEM_BLOCKS: [i32; 13] = [3, 5, 6, 12, 17, 24, 37, 38, 97, 98, 139, 155, 162];

/// The `LeavesItem` blocks.
const LEAVES_ITEM_BLOCKS: [i32; 2] = [18, 161];

/// The `SlabItem` blocks.
const SLAB_ITEM_BLOCKS: [i32; 4] = [43, 44, 125, 126];

/// The `PlantBlockItem` blocks.
const PLANT_ITEM_BLOCKS: [i32; 2] = [31, 106];

/// The registered sprite name of item `id` for `metadata`
/// (`Item.getSprite(metadata)` plus every client override). `None` when the
/// id is unregistered or the item draws no sprite.
pub fn item_texture(id: i32, metadata: i32) -> Option<String> {
    // A named item (id 256+): the init's sprite with the per-metadata
    // overrides.
    if item(id).is_some() {
        let base = item(id)?.sprite?;
        return Some(match id {
            261 => {
                // BowItem.registerSprites: the standby sprite is the item's
                // sprite; the pulling sprites come from `getBowSprite`.
                "bow_standby".to_owned()
            }
            263 => {
                // CoalItem.getSprite: metadata 1 is charcoal.
                if metadata == 1 {
                    "charcoal".to_owned()
                } else {
                    "coal".to_owned()
                }
            }
            349 | 350 => {
                // FishItem.getSprite: the type by metadata, cooked where the
                // type can be cooked.
                let ty = &FISH_TYPES[clamp(metadata, 0, 3) as usize];
                if id == 350 && ty.2 {
                    format!("fish_{}_cooked", ty.1)
                } else {
                    format!("fish_{}_raw", ty.1)
                }
            }
            351 => format!(
                "dye_powder_{}",
                DYE_COLOR_NAMES[clamp(metadata, 0, 15) as usize]
            ),
            373 => {
                // PotionItem.getSprite: bit 16384 is the splash potion.
                if metadata & 16384 != 0 {
                    "potion_bottle_splash".to_owned()
                } else {
                    "potion_bottle_drinkable".to_owned()
                }
            }
            397 => format!("skull_{}", SKULL_TYPES[clamp(metadata, 0, 4) as usize]),
            _ => base.to_owned(),
        });
    }

    // A block item (same id as its block).
    let block_entry = block(id)?;
    if BLOCKS_WITHOUT_ITEM.contains(&id) {
        return None;
    }
    if id == 154 {
        // HopperBlock.getItemSpriteName: the literal "hopper".
        return Some("hopper".to_owned());
    }
    if COLORED_ITEM_BLOCKS.contains(&id) {
        // ColoredBlockItem.getSprite: the block sprite for the inverted
        // metadata (dye 0 is white).
        return block_texture(id, !metadata & 15, FACE_NORTH);
    }
    if VARIANT_ITEM_BLOCKS.contains(&id) {
        // VariantBlockItem.getSprite: the block's sprite for the metadata.
        return block_texture(id, metadata, FACE_NORTH);
    }
    if LEAVES_ITEM_BLOCKS.contains(&id) {
        // LeavesItem.getSprite: the block's sprite on the bottom face.
        return block_texture(id, metadata, FACE_BOTTOM);
    }
    if SLAB_ITEM_BLOCKS.contains(&id) {
        // SlabItem.getSprite: the block's sprite on the north face.
        return block_texture(id, metadata, FACE_NORTH);
    }
    if PLANT_ITEM_BLOCKS.contains(&id) {
        // PlantBlockItem.getSprite: the block's sprite on the bottom face.
        return block_texture(id, metadata, FACE_BOTTOM);
    }
    let _ = block_entry;
    // A plain BlockItem: the block's top sprite at metadata 0
    // (`BlockItem.getSprite`).
    block_texture(id, 0, FACE_TOP)
}

/// The item overlay sprite of item `id` for `metadata` on layer 1
/// (`getSprite(metadata, layer)` overrides): the leather armor's colored
/// overlay. The potion's overlay is registered but its color comes from the
/// potion metadata at draw time.
pub fn item_overlay_texture(id: i32, _metadata: i32) -> Option<&'static str> {
    match id {
        298 => Some(LEATHER_ARMOR_OVERLAYS[0]),
        299 => Some(LEATHER_ARMOR_OVERLAYS[1]),
        300 => Some(LEATHER_ARMOR_OVERLAYS[2]),
        301 => Some(LEATHER_ARMOR_OVERLAYS[3]),
        _ => None,
    }
}

/// `VariantBlockItem.getTranslationKey`'s variant name: the loaded
/// translation key of the item with `metadata`.
pub fn item_variant_key(id: i32, metadata: i32) -> Option<&'static str> {
    match id {
        5 => Some(PLANKS_VARIANTS[clamp(metadata, 0, 5) as usize]),
        17 => Some(LOG_VARIANTS[clamp(metadata, 0, 3) as usize]),
        162 => Some(LOG2_VARIANTS[clamp(metadata, 0, 1) as usize]),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_registry_is_complete() {
        // The full 1.7.10 init: ids 0..164 and 170..175 (ids 165-169 are
        // unused), 171 blocks.
        assert_eq!(block_count(), 171);
        assert_eq!(block(0).unwrap().key, "air");
        assert_eq!(block(1).unwrap().sprite, Some("stone"));
        assert_eq!(block(164).unwrap().key, "stairsWoodDarkOak");
        assert_eq!(block(170).unwrap().key, "hayBlock");
        assert_eq!(block(175).unwrap().key, "doublePlant");
        assert!(block(165).is_none());
        assert!(block(169).is_none());
        assert!(block(176).is_none());
    }

    #[test]
    fn item_registry_is_complete() {
        // The named items of `Item.init`: 256..408, 417..422 and the 12
        // records — 171 entries. The block items live in the block table
        // (same ids).
        assert_eq!(item_count(), 171);
        assert_eq!(item(256).unwrap().sprite, Some("iron_shovel"));
        assert_eq!(item(408).unwrap().sprite, Some("minecart_hopper"));
        assert_eq!(item(417).unwrap().sprite, Some("iron_horse_armor"));
        assert_eq!(item(2267).unwrap().sprite, Some("record_wait"));
        assert!(item(409).is_none());
        assert!(item(416).is_none());
        assert_eq!(item(351).unwrap().key, "dyePowder");
    }

    #[test]
    fn texture_locations_resolve_under_the_atlas() {
        assert_eq!(
            texture_location(0, "stone").to_string(),
            "minecraft:textures/blocks/stone.png"
        );
        assert_eq!(
            texture_location(1, "stick").to_string(),
            "minecraft:textures/items/stick.png"
        );
        // A namespaced sprite keeps its namespace.
        assert_eq!(
            texture_location(0, "mymod:block").to_string(),
            "mymod:textures/blocks/block.png"
        );
    }

    #[test]
    fn sprite_names_reject_slashes() {
        assert!(sprite_name_is_valid("stone"));
        assert!(!sprite_name_is_valid("blocks/stone"));
        assert!(!sprite_name_is_valid("blocks\\stone"));
    }

    #[test]
    fn dynamic_item_sprites() {
        assert!(is_dynamic_item_sprite("clock"));
        assert!(is_dynamic_item_sprite("compass"));
        assert!(!is_dynamic_item_sprite("stick"));
    }

    #[test]
    fn dye_color_matches_vanilla_inversion() {
        // Item dye 0 (ink sac, black) is block wool metadata 15.
        assert_eq!(DYE_COLOR_NAMES[dye_color_of_block_metadata(15)], "black");
        assert_eq!(DYE_COLOR_NAMES[dye_color_of_block_metadata(0)], "white");
    }

    #[test]
    fn plain_blocks_use_the_base_sprite() {
        assert_eq!(block_texture(1, 0, 0).as_deref(), Some("stone"));
        assert_eq!(block_texture(4, 3, 5).as_deref(), Some("cobblestone"));
        assert_eq!(block_texture(45, 0, 1).as_deref(), Some("brick"));
        assert_eq!(block_texture(49, 0, 2).as_deref(), Some("obsidian"));
        // Air registers nothing.
        assert_eq!(block_texture(0, 0, 1), None);
    }

    #[test]
    fn variant_blocks_match_their_register_sprites() {
        assert_eq!(block_texture(5, 0, 1).as_deref(), Some("planks_oak"));
        assert_eq!(block_texture(5, 2, 1).as_deref(), Some("planks_birch"));
        assert_eq!(block_texture(5, 9, 1).as_deref(), Some("planks_big_oak"));
        assert_eq!(
            block_texture(35, 0, 1).as_deref(),
            Some("wool_colored_white")
        );
        assert_eq!(
            block_texture(35, 15, 1).as_deref(),
            Some("wool_colored_black")
        );
        assert_eq!(
            block_texture(159, 0, 1).as_deref(),
            Some("hardened_clay_stained_white")
        );
        assert_eq!(block_texture(95, 0, 1).as_deref(), Some("glass_white"));
        assert_eq!(block_texture(160, 0, 1).as_deref(), Some("glass_white"));
        assert_eq!(block_texture(6, 0, 1).as_deref(), Some("sapling_oak"));
        assert_eq!(
            block_texture(6, 5, 1).as_deref(),
            Some("sapling_roofed_oak")
        );
    }

    #[test]
    fn logs_follow_the_axis() {
        // Vertical log: top/bottom faces get the top sprite.
        assert_eq!(block_texture(17, 0, 1).as_deref(), Some("log_oak_top"));
        assert_eq!(block_texture(17, 0, 2).as_deref(), Some("log_oak"));
        // East-west axis (bits 2-3 = 4): west/east faces get the top.
        assert_eq!(block_texture(17, 4, 4).as_deref(), Some("log_oak_top"));
        assert_eq!(block_texture(17, 4, 2).as_deref(), Some("log_oak"));
        // North-south axis (8).
        assert_eq!(block_texture(17, 8, 3).as_deref(), Some("log_oak_top"));
        // All-bark axis (12): every face is the side sprite.
        assert_eq!(block_texture(17, 12, 1).as_deref(), Some("log_oak"));
        // Wood type from bits 0-1.
        assert_eq!(block_texture(17, 2, 1).as_deref(), Some("log_birch_top"));
        assert_eq!(block_texture(162, 1, 2).as_deref(), Some("log_big_oak"));
    }

    #[test]
    fn leaves_default_to_the_transparent_row() {
        assert_eq!(block_texture(18, 0, 1).as_deref(), Some("leaves_oak"));
        assert_eq!(block_texture(18, 2, 1).as_deref(), Some("leaves_birch"));
        assert_eq!(leaves_opaque_texture(18, 0), Some("leaves_oak_opaque"));
        assert_eq!(leaves_opaque_texture(161, 1), Some("leaves_big_oak_opaque"));
    }

    #[test]
    fn ores_and_liquids_match_vanilla() {
        assert_eq!(block_texture(8, 0, 1).as_deref(), Some("water_still"));
        assert_eq!(block_texture(8, 0, 3).as_deref(), Some("water_flow"));
        assert_eq!(block_texture(11, 0, 2).as_deref(), Some("lava_flow"));
        assert_eq!(block_texture(12, 1, 1).as_deref(), Some("red_sand"));
        assert_eq!(block_texture(60, 0, 1).as_deref(), Some("farmland_dry"));
        assert_eq!(block_texture(60, 7, 1).as_deref(), Some("farmland_wet"));
        assert_eq!(block_texture(60, 3, 2).as_deref(), Some("dirt"));
    }

    #[test]
    fn stone_slab_delegates_by_metadata() {
        assert_eq!(block_texture(44, 0, 1).as_deref(), Some("stone_slab_top"));
        assert_eq!(block_texture(44, 0, 2).as_deref(), Some("stone_slab_side"));
        assert_eq!(block_texture(44, 1, 1).as_deref(), Some("sandstone_top"));
        assert_eq!(block_texture(44, 2, 3).as_deref(), Some("planks_oak"));
        assert_eq!(block_texture(44, 4, 1).as_deref(), Some("brick"));
        assert_eq!(block_texture(44, 5, 1).as_deref(), Some("stonebrick"));
        assert_eq!(block_texture(44, 6, 1).as_deref(), Some("nether_brick"));
        assert_eq!(block_texture(44, 7, 1).as_deref(), Some("quartz_block_top"));
        // The double slab forces the top sprite.
        assert_eq!(block_texture(43, 8, 2).as_deref(), Some("stone_slab_top"));
        // The wooden slab is the planks sprite.
        assert_eq!(block_texture(126, 1, 1).as_deref(), Some("planks_spruce"));
    }

    #[test]
    fn machinery_matches_their_get_sprite() {
        assert_eq!(
            block_texture(61, 2, 2).as_deref(),
            Some("furnace_front_off")
        );
        assert_eq!(block_texture(61, 2, 3).as_deref(), Some("furnace_side"));
        assert_eq!(block_texture(62, 4, 4).as_deref(), Some("furnace_front_on"));
        assert_eq!(block_texture(61, 2, 1).as_deref(), Some("furnace_top"));
        assert_eq!(
            block_texture(23, 3, 3).as_deref(),
            Some("dispenser_front_horizontal")
        );
        assert_eq!(
            block_texture(23, 1, 1).as_deref(),
            Some("dispenser_front_vertical")
        );
        assert_eq!(
            block_texture(158, 3, 3).as_deref(),
            Some("dropper_front_horizontal")
        );
        assert_eq!(
            block_texture(158, 1, 1).as_deref(),
            Some("dropper_front_vertical")
        );
        assert_eq!(block_texture(154, 0, 1).as_deref(), Some("hopper_top"));
        assert_eq!(block_texture(154, 0, 2).as_deref(), Some("hopper_outside"));
        assert_eq!(block_texture(84, 0, 1).as_deref(), Some("jukebox_top"));
        assert_eq!(block_texture(84, 0, 2).as_deref(), Some("jukebox_side"));
    }

    #[test]
    fn pistons_take_the_top_sprite_on_the_facing_face() {
        // Facing north (2): the north face takes the top sprite, the south
        // face the bottom, the rest the side.
        assert_eq!(
            block_texture(33, 2, 2).as_deref(),
            Some("piston_top_normal")
        );
        assert_eq!(block_texture(33, 2, 3).as_deref(), Some("piston_bottom"));
        assert_eq!(block_texture(33, 2, 1).as_deref(), Some("piston_side"));
        assert_eq!(
            block_texture(29, 2, 2).as_deref(),
            Some("piston_top_sticky")
        );
    }

    #[test]
    fn redstone_and_lamps() {
        assert_eq!(
            block_texture(55, 0, 1).as_deref(),
            Some("redstone_dust_cross")
        );
        assert_eq!(
            block_texture(75, 0, 1).as_deref(),
            Some("redstone_torch_off")
        );
        assert_eq!(
            block_texture(76, 0, 1).as_deref(),
            Some("redstone_torch_on")
        );
        assert_eq!(
            block_texture(123, 0, 1).as_deref(),
            Some("redstone_lamp_off")
        );
        assert_eq!(
            block_texture(124, 0, 1).as_deref(),
            Some("redstone_lamp_on")
        );
        assert_eq!(block_texture(152, 0, 1).as_deref(), Some("redstone_block"));
        // The repeater has no face override: repeater_off everywhere.
        assert_eq!(block_texture(93, 0, 2).as_deref(), Some("repeater_off"));
        assert_eq!(block_texture(94, 8, 1).as_deref(), Some("repeater_on"));
    }

    #[test]
    fn rails_match_vanilla() {
        assert_eq!(block_texture(28, 0, 1).as_deref(), Some("rail_detector"));
        assert_eq!(
            block_texture(28, 8, 1).as_deref(),
            Some("rail_detector_powered")
        );
        assert_eq!(
            block_texture(27, 0, 1).as_deref(),
            Some("rail_golden_powered")
        );
        assert_eq!(
            block_texture(157, 0, 1).as_deref(),
            Some("rail_activator_powered")
        );
        assert_eq!(block_texture(66, 0, 1).as_deref(), Some("rail_normal"));
    }

    #[test]
    fn plants_and_foods() {
        assert_eq!(block_texture(59, 0, 1).as_deref(), Some("wheat_stage_0"));
        assert_eq!(block_texture(59, 7, 1).as_deref(), Some("wheat_stage_7"));
        assert_eq!(
            block_texture(115, 2, 1).as_deref(),
            Some("nether_wart_stage_2")
        );
        assert_eq!(block_texture(141, 3, 1).as_deref(), Some("carrots_stage_3"));
        assert_eq!(
            block_texture(142, 1, 1).as_deref(),
            Some("potatoes_stage_1")
        );
        assert_eq!(block_texture(127, 0, 1).as_deref(), Some("cocoa_stage_2"));
        assert_eq!(
            block_texture(104, 0, 1).as_deref(),
            Some("pumpkin_stem_disconnected")
        );
        assert_eq!(
            block_texture(105, 0, 1).as_deref(),
            Some("melon_stem_disconnected")
        );
        assert_eq!(block_texture(86, 0, 1).as_deref(), Some("pumpkin_top"));
        assert_eq!(block_texture(86, 0, 3).as_deref(), Some("pumpkin_face_off"));
        assert_eq!(block_texture(91, 3, 5).as_deref(), Some("pumpkin_face_on"));
        assert_eq!(block_texture(103, 0, 1).as_deref(), Some("melon_top"));
        assert_eq!(block_texture(92, 2, 4).as_deref(), Some("cake_inner"));
        assert_eq!(block_texture(92, 0, 4).as_deref(), Some("cake_side"));
        assert_eq!(block_texture(170, 0, 1).as_deref(), Some("hay_block_top"));
    }

    #[test]
    fn stairs_draw_the_parent_block_sprite() {
        assert_eq!(block_texture(53, 0, 1).as_deref(), Some("planks_oak"));
        assert_eq!(block_texture(134, 0, 1).as_deref(), Some("planks_spruce"));
        assert_eq!(block_texture(164, 0, 1).as_deref(), Some("planks_big_oak"));
        assert_eq!(block_texture(67, 0, 1).as_deref(), Some("cobblestone"));
        assert_eq!(block_texture(108, 0, 1).as_deref(), Some("brick"));
        assert_eq!(block_texture(109, 0, 1).as_deref(), Some("stonebrick"));
        assert_eq!(block_texture(114, 0, 1).as_deref(), Some("nether_brick"));
        assert_eq!(block_texture(128, 0, 1).as_deref(), Some("sandstone_top"));
        assert_eq!(
            block_texture(156, 0, 1).as_deref(),
            Some("quartz_block_top")
        );
    }

    #[test]
    fn doors_and_glass() {
        assert_eq!(block_texture(64, 0, 2).as_deref(), Some("door_wood_lower"));
        assert_eq!(block_texture(64, 8, 2).as_deref(), Some("door_wood_upper"));
        assert_eq!(block_texture(71, 8, 2).as_deref(), Some("door_iron_upper"));
        assert_eq!(block_texture(96, 0, 1).as_deref(), Some("trapdoor"));
        assert_eq!(block_texture(20, 0, 1).as_deref(), Some("glass"));
        assert_eq!(block_texture(102, 0, 2).as_deref(), Some("glass"));
        assert_eq!(block_texture(102, 0, 1).as_deref(), Some("glass_pane_top"));
    }

    #[test]
    fn mushroom_blocks_follow_the_metadata() {
        assert_eq!(
            block_texture(99, 0, 1).as_deref(),
            Some("mushroom_block_inside")
        );
        // Metadata 5 shows the skin on the top face only; metadata 1 also
        // shows the west face.
        assert_eq!(
            block_texture(99, 5, 1).as_deref(),
            Some("mushroom_block_skin_brown")
        );
        assert_eq!(
            block_texture(99, 5, 4).as_deref(),
            Some("mushroom_block_inside")
        );
        assert_eq!(
            block_texture(99, 1, 4).as_deref(),
            Some("mushroom_block_skin_brown")
        );
        assert_eq!(
            block_texture(99, 5, 3).as_deref(),
            Some("mushroom_block_inside")
        );
        assert_eq!(
            block_texture(100, 14, 5).as_deref(),
            Some("mushroom_block_skin_red")
        );
        assert_eq!(
            block_texture(99, 10, 2).as_deref(),
            Some("mushroom_block_skin_stem")
        );
        assert_eq!(
            block_texture(99, 15, 1).as_deref(),
            Some("mushroom_block_skin_stem")
        );
    }

    #[test]
    fn blocks_without_sprites_return_none() {
        assert_eq!(block_texture(107, 0, 1), None);
        assert_eq!(block_texture(77, 0, 1), None);
        assert_eq!(block_texture(34, 0, 1), None);
    }

    #[test]
    fn missing_icon_names_match_vanilla() {
        assert_eq!(
            missing_icon_block_name(31),
            "MISSING_ICON_BLOCK_31_tallgrass"
        );
        assert_eq!(
            missing_icon_block_name(139),
            "MISSING_ICON_BLOCK_139_cobbleWall"
        );
    }

    #[test]
    fn item_sprites_match_the_init() {
        assert_eq!(item_texture(256, 0).as_deref(), Some("iron_shovel"));
        assert_eq!(item_texture(268, 0).as_deref(), Some("wood_sword"));
        assert_eq!(item_texture(283, 0).as_deref(), Some("gold_sword"));
        assert_eq!(item_texture(287, 0).as_deref(), Some("string"));
        assert_eq!(item_texture(325, 0).as_deref(), Some("bucket_empty"));
        assert_eq!(item_texture(331, 0).as_deref(), Some("redstone_dust"));
        assert_eq!(item_texture(340, 0).as_deref(), Some("book_normal"));
        assert_eq!(item_texture(374, 0).as_deref(), Some("potion_bottle_empty"));
        assert_eq!(item_texture(395, 0).as_deref(), Some("map_empty"));
        assert_eq!(item_texture(2256, 0).as_deref(), Some("record_13"));
    }

    #[test]
    fn item_metadata_sprites() {
        assert_eq!(item_texture(263, 0).as_deref(), Some("coal"));
        assert_eq!(item_texture(263, 1).as_deref(), Some("charcoal"));
        assert_eq!(item_texture(351, 0).as_deref(), Some("dye_powder_black"));
        assert_eq!(item_texture(351, 15).as_deref(), Some("dye_powder_white"));
        assert_eq!(item_texture(349, 1).as_deref(), Some("fish_salmon_raw"));
        assert_eq!(item_texture(350, 1).as_deref(), Some("fish_salmon_cooked"));
        assert_eq!(item_texture(350, 2).as_deref(), Some("fish_clownfish_raw"));
        assert_eq!(
            item_texture(373, 0).as_deref(),
            Some("potion_bottle_drinkable")
        );
        assert_eq!(
            item_texture(373, 16384).as_deref(),
            Some("potion_bottle_splash")
        );
        assert_eq!(item_texture(397, 3).as_deref(), Some("skull_steve"));
        assert_eq!(item_texture(261, 0).as_deref(), Some("bow_standby"));
    }

    #[test]
    fn leather_overlay_is_layer_1() {
        assert_eq!(item_overlay_texture(298, 0), Some("leather_helmet_overlay"));
        assert_eq!(item_overlay_texture(301, 0), Some("leather_boots_overlay"));
        assert_eq!(item_overlay_texture(306, 0), None);
    }

    #[test]
    fn block_item_sprites_come_from_the_block() {
        // The hopper block hands its item the hopper sprite.
        assert_eq!(item_texture(154, 0).as_deref(), Some("hopper"));
        // The skull block is in the init's no-item set.
        assert_eq!(item_texture(144, 0), None);
        // Other block items draw the block's top sprite at metadata 0.
        assert_eq!(item_texture(1, 0).as_deref(), Some("stone"));
        assert_eq!(item_texture(5, 0).as_deref(), Some("planks_oak"));
        // Variant items pass the metadata to the block.
        assert_eq!(item_texture(5, 2).as_deref(), Some("planks_birch"));
        // Colored items invert the metadata for the sprite: item dye 0
        // (black) shows the black wool sprite, and the plain item
        // metadata 15 shows the white one.
        assert_eq!(item_texture(35, 0).as_deref(), Some("wool_colored_black"));
        assert_eq!(item_texture(35, 15).as_deref(), Some("wool_colored_white"));
        assert_eq!(item_texture(171, 4).as_deref(), Some("wool_colored_blue"));
        // Slab items read the block's sprite at their metadata.
        assert_eq!(item_texture(126, 1).as_deref(), Some("planks_spruce"));
        // Leaves items read the block's bottom-face sprite.
        assert_eq!(item_texture(18, 0).as_deref(), Some("leaves_oak"));
        // Blocks without an item have no item sprite.
        assert_eq!(item_texture(64, 0), None);
        assert_eq!(item_texture(55, 0), None);
    }

    #[test]
    fn wool_block_metadata_matches_vanilla() {
        // Block wool metadata 4 is the yellow wool; the wool item with
        // metadata 4 shows the blue sprite (the inverted item metadata 11
        // reads back as dye 4).
        assert_eq!(
            block_texture(35, 4, 1).as_deref(),
            Some("wool_colored_yellow")
        );
        assert_eq!(item_texture(35, 4).as_deref(), Some("wool_colored_blue"));
    }

    #[test]
    fn item_variant_keys() {
        assert_eq!(item_variant_key(5, 2), Some("birch"));
        assert_eq!(item_variant_key(5, 9), Some("big_oak"));
        assert_eq!(item_variant_key(17, 1), Some("spruce"));
        assert_eq!(item_variant_key(162, 0), Some("acacia"));
        assert_eq!(item_variant_key(1, 0), None);
    }

    #[test]
    fn atlas_types() {
        // Named items stitch into the items atlas.
        assert_eq!(item(280).unwrap().atlas_type, 1);
        assert_eq!(BLOCKS_ATLAS_PATH, "textures/blocks");
        assert_eq!(ITEMS_ATLAS_PATH, "textures/items");
    }
}
