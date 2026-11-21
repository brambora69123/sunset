//! The 1.7.10 block data a registry row does not carry: the Java class chain,
//! sound type, random-tick and block-entity flags, the metadata values a block
//! uses, its shape class and its map colour.
//!
//! `EXTRAS` is generated from `Block.init()` and the block constructors in the
//! decompiled 1.7.10 server jar; `metas` and the shape lists are read off the
//! block classes' metadata use.

use sunset_1_7_blocks::{BlockDef, Material};

pub(crate) struct Extra {
    pub id: u16,
    pub class: &'static [&'static str],
    pub sound: Option<&'static str>,
    pub volume: f32,
    pub pitch: f32,
    pub random_ticks: bool,
    pub block_entity: bool,
}

/// The 1.7.10 class chain, sound type and tick/block-entity flags of every
/// `Block.REGISTRY` entry, in the order of [`sunset_1_7_blocks::BLOCKS`].
pub(crate) static EXTRAS: &[Extra] = &[
    Extra {
        id: 0,
        class: &["AirBlock", "Block"],
        sound: None,
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 1,
        class: &["StoneBlock", "Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 2,
        class: &["GrassBlock", "Block"],
        sound: Some("grass"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 3,
        class: &["DirtBlock", "Block"],
        sound: Some("gravel"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 4,
        class: &["Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 5,
        class: &["PlanksBlock", "Block"],
        sound: Some("wood"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 6,
        class: &["SaplingBlock", "PlantBlock", "Block"],
        sound: Some("grass"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 7,
        class: &["Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 8,
        class: &["FlowingLiquidBlock", "LiquidBlock", "Block"],
        sound: None,
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 9,
        class: &["LiquidSourceBlock", "LiquidBlock", "Block"],
        sound: None,
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 10,
        class: &["FlowingLiquidBlock", "LiquidBlock", "Block"],
        sound: None,
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 11,
        class: &["LiquidSourceBlock", "LiquidBlock", "Block"],
        sound: None,
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 12,
        class: &["SandBlock", "FallingBlock", "Block"],
        sound: Some("sand"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 13,
        class: &["GravelBlock", "FallingBlock", "Block"],
        sound: Some("gravel"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 14,
        class: &["OreBlock", "Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 15,
        class: &["OreBlock", "Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 16,
        class: &["OreBlock", "Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 17,
        class: &["LogBlock", "AbstractLogBlock", "AxisBlock", "Block"],
        sound: Some("wood"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 18,
        class: &[
            "LeavesBlock",
            "AbstractLeavesBlock",
            "TranslucentBlock",
            "Block",
        ],
        sound: Some("grass"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 19,
        class: &["SpongeBlock", "Block"],
        sound: Some("grass"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 20,
        class: &["GlassBlock", "TransparentBlock", "Block"],
        sound: Some("glass"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 21,
        class: &["OreBlock", "Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 22,
        class: &["MineralBlock", "Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 23,
        class: &["DispenserBlock", "BlockWithBlockEntity", "Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: true,
    },
    Extra {
        id: 24,
        class: &["SandstoneBlock", "Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 25,
        class: &["NoteBlock", "BlockWithBlockEntity", "Block"],
        sound: None,
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: true,
    },
    Extra {
        id: 26,
        class: &["BedBlock", "HorizontalFacingBlock", "Block"],
        sound: None,
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 27,
        class: &["PoweredRailBlock", "AbstractRailBlock", "Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.5,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 28,
        class: &["DetectorRailBlock", "AbstractRailBlock", "Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.5,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 29,
        class: &["PistonBaseBlock", "Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 30,
        class: &["CobwebBlock", "Block"],
        sound: None,
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 31,
        class: &["TallPlantBlock", "PlantBlock", "Block"],
        sound: Some("grass"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 32,
        class: &["DeadBushBlock", "PlantBlock", "Block"],
        sound: Some("grass"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 33,
        class: &["PistonBaseBlock", "Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 34,
        class: &["PistonHeadBlock", "Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 35,
        class: &["ColoredBlock", "Block"],
        sound: Some("cloth"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 36,
        class: &["MovingBlock", "BlockWithBlockEntity", "Block"],
        sound: None,
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: true,
    },
    Extra {
        id: 37,
        class: &["FlowerBlock", "PlantBlock", "Block"],
        sound: Some("grass"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 38,
        class: &["FlowerBlock", "PlantBlock", "Block"],
        sound: Some("grass"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 39,
        class: &["MushroomPlantBlock", "PlantBlock", "Block"],
        sound: Some("grass"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 40,
        class: &["MushroomPlantBlock", "PlantBlock", "Block"],
        sound: Some("grass"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 41,
        class: &["MineralBlock", "Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.5,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 42,
        class: &["MineralBlock", "Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.5,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 43,
        class: &["StoneSlabBlock", "SlabBlock", "Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 44,
        class: &["StoneSlabBlock", "SlabBlock", "Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 45,
        class: &["Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 46,
        class: &["TntBlock", "Block"],
        sound: Some("grass"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 47,
        class: &["BookshelfBlock", "Block"],
        sound: Some("wood"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 48,
        class: &["Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 49,
        class: &["ObsidianBlock", "StoneBlock", "Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 50,
        class: &["TorchBlock", "Block"],
        sound: Some("wood"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 51,
        class: &["FireBlock", "Block"],
        sound: Some("wood"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 52,
        class: &["MobSpawnerBlock", "BlockWithBlockEntity", "Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.5,
        random_ticks: false,
        block_entity: true,
    },
    Extra {
        id: 53,
        class: &["StairsBlock", "Block"],
        sound: None,
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 54,
        class: &["ChestBlock", "BlockWithBlockEntity", "Block"],
        sound: Some("wood"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: true,
    },
    Extra {
        id: 55,
        class: &["RedstoneWireBlock", "Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 56,
        class: &["OreBlock", "Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 57,
        class: &["MineralBlock", "Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.5,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 58,
        class: &["CraftingTableBlock", "Block"],
        sound: Some("wood"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 59,
        class: &["WheatBlock", "PlantBlock", "Block"],
        sound: Some("grass"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 60,
        class: &["FarmlandBlock", "Block"],
        sound: Some("gravel"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 61,
        class: &["FurnaceBlock", "BlockWithBlockEntity", "Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: true,
    },
    Extra {
        id: 62,
        class: &["FurnaceBlock", "BlockWithBlockEntity", "Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: true,
    },
    Extra {
        id: 63,
        class: &["SignBlock", "BlockWithBlockEntity", "Block"],
        sound: Some("wood"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: true,
    },
    Extra {
        id: 64,
        class: &["DoorBlock", "Block"],
        sound: Some("wood"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 65,
        class: &["LadderBlock", "Block"],
        sound: Some("ladder"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 66,
        class: &["RailBlock", "AbstractRailBlock", "Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.5,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 67,
        class: &["StairsBlock", "Block"],
        sound: None,
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 68,
        class: &["SignBlock", "BlockWithBlockEntity", "Block"],
        sound: Some("wood"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: true,
    },
    Extra {
        id: 69,
        class: &["LeverBlock", "Block"],
        sound: Some("wood"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 70,
        class: &["PressurePlateBlock", "AbstractPressurePlateBlock", "Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 71,
        class: &["DoorBlock", "Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.5,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 72,
        class: &["PressurePlateBlock", "AbstractPressurePlateBlock", "Block"],
        sound: Some("wood"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 73,
        class: &["RedstoneOreBlock", "Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 74,
        class: &["RedstoneOreBlock", "Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 75,
        class: &["RedstoneTorchBlock", "TorchBlock", "Block"],
        sound: Some("wood"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 76,
        class: &["RedstoneTorchBlock", "TorchBlock", "Block"],
        sound: Some("wood"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 77,
        class: &["StoneButtonBlock", "ButtonBlock", "Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 78,
        class: &["SnowLayerBlock", "Block"],
        sound: Some("snow"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 79,
        class: &["IceBlock", "TransparentBlock", "Block"],
        sound: Some("glass"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 80,
        class: &["SnowBlock", "Block"],
        sound: Some("snow"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 81,
        class: &["CactusBlock", "Block"],
        sound: Some("cloth"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 82,
        class: &["ClayBlock", "Block"],
        sound: Some("gravel"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 83,
        class: &["SugarCaneBlock", "Block"],
        sound: Some("grass"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 84,
        class: &["JukeboxBlock", "BlockWithBlockEntity", "Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: true,
    },
    Extra {
        id: 85,
        class: &["FenceBlock", "Block"],
        sound: Some("wood"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 86,
        class: &["PumpkinBlock", "HorizontalFacingBlock", "Block"],
        sound: Some("wood"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 87,
        class: &["NetherrackBlock", "Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 88,
        class: &["SoulSandBlock", "Block"],
        sound: Some("sand"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 89,
        class: &["GlowstoneBlock", "Block"],
        sound: Some("glass"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 90,
        class: &["PortalBlock", "TransparentBlock", "Block"],
        sound: Some("glass"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 91,
        class: &["PumpkinBlock", "HorizontalFacingBlock", "Block"],
        sound: Some("wood"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 92,
        class: &["CakeBlock", "Block"],
        sound: Some("cloth"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 93,
        class: &[
            "RepeaterBlock",
            "DiodeBlock",
            "HorizontalFacingBlock",
            "Block",
        ],
        sound: Some("wood"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 94,
        class: &[
            "RepeaterBlock",
            "DiodeBlock",
            "HorizontalFacingBlock",
            "Block",
        ],
        sound: Some("wood"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 95,
        class: &["StainedGlassBlock", "TransparentBlock", "Block"],
        sound: Some("glass"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 96,
        class: &["TrapdoorBlock", "Block"],
        sound: Some("wood"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 97,
        class: &["InfestedBlock", "Block"],
        sound: None,
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 98,
        class: &["StonebrickBlock", "Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 99,
        class: &["MushroomBlock", "Block"],
        sound: Some("wood"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 100,
        class: &["MushroomBlock", "Block"],
        sound: Some("wood"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 101,
        class: &["PaneBlock", "Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.5,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 102,
        class: &["PaneBlock", "Block"],
        sound: Some("glass"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 103,
        class: &["MelonBlock", "Block"],
        sound: Some("wood"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 104,
        class: &["StemBlock", "PlantBlock", "Block"],
        sound: Some("wood"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 105,
        class: &["StemBlock", "PlantBlock", "Block"],
        sound: Some("wood"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 106,
        class: &["VineBlock", "Block"],
        sound: Some("grass"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 107,
        class: &["FenceGateBlock", "HorizontalFacingBlock", "Block"],
        sound: Some("wood"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 108,
        class: &["StairsBlock", "Block"],
        sound: None,
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 109,
        class: &["StairsBlock", "Block"],
        sound: None,
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 110,
        class: &["MyceliumBlock", "Block"],
        sound: Some("grass"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 111,
        class: &["LilyPadBlock", "PlantBlock", "Block"],
        sound: Some("grass"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 112,
        class: &["Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 113,
        class: &["FenceBlock", "Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 114,
        class: &["StairsBlock", "Block"],
        sound: None,
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 115,
        class: &["NetherWartBlock", "PlantBlock", "Block"],
        sound: None,
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 116,
        class: &["EnchantingTableBlock", "BlockWithBlockEntity", "Block"],
        sound: None,
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: true,
    },
    Extra {
        id: 117,
        class: &["BrewingStandBlock", "BlockWithBlockEntity", "Block"],
        sound: None,
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: true,
    },
    Extra {
        id: 118,
        class: &["CauldronBlock", "Block"],
        sound: None,
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 119,
        class: &["EndPortalBlock", "BlockWithBlockEntity", "Block"],
        sound: None,
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: true,
    },
    Extra {
        id: 120,
        class: &["EndPortalFrameBlock", "Block"],
        sound: Some("glass"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 121,
        class: &["Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 122,
        class: &["DragonEggBlock", "Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 123,
        class: &["RedstoneLampBlock", "Block"],
        sound: Some("glass"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 124,
        class: &["RedstoneLampBlock", "Block"],
        sound: Some("glass"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 125,
        class: &["WoodenSlabBlock", "SlabBlock", "Block"],
        sound: Some("wood"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 126,
        class: &["WoodenSlabBlock", "SlabBlock", "Block"],
        sound: Some("wood"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 127,
        class: &["CocoaBlock", "HorizontalFacingBlock", "Block"],
        sound: Some("wood"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 128,
        class: &["StairsBlock", "Block"],
        sound: None,
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 129,
        class: &["OreBlock", "Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 130,
        class: &["EnderChestBlock", "BlockWithBlockEntity", "Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: true,
    },
    Extra {
        id: 131,
        class: &["TripwireHookBlock", "Block"],
        sound: None,
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 132,
        class: &["TripwireBlock", "Block"],
        sound: None,
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 133,
        class: &["MineralBlock", "Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.5,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 134,
        class: &["StairsBlock", "Block"],
        sound: None,
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 135,
        class: &["StairsBlock", "Block"],
        sound: None,
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 136,
        class: &["StairsBlock", "Block"],
        sound: None,
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 137,
        class: &["CommandBlock", "BlockWithBlockEntity", "Block"],
        sound: None,
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: true,
    },
    Extra {
        id: 138,
        class: &["BeaconBlock", "BlockWithBlockEntity", "Block"],
        sound: None,
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: true,
    },
    Extra {
        id: 139,
        class: &["WallBlock", "Block"],
        sound: None,
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 140,
        class: &["FlowerPotBlock", "BlockWithBlockEntity", "Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: true,
    },
    Extra {
        id: 141,
        class: &["CarrotsBlock", "WheatBlock", "PlantBlock", "Block"],
        sound: Some("grass"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 142,
        class: &["PotatoesBlock", "WheatBlock", "PlantBlock", "Block"],
        sound: Some("grass"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 143,
        class: &["WoodenButtonBlock", "ButtonBlock", "Block"],
        sound: Some("wood"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 144,
        class: &["SkullBlock", "BlockWithBlockEntity", "Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: true,
    },
    Extra {
        id: 145,
        class: &["AnvilBlock", "FallingBlock", "Block"],
        sound: Some("anvil"),
        volume: 0.3,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 146,
        class: &["ChestBlock", "BlockWithBlockEntity", "Block"],
        sound: Some("wood"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: true,
    },
    Extra {
        id: 147,
        class: &[
            "WeightedPressurePlateBlock",
            "AbstractPressurePlateBlock",
            "Block",
        ],
        sound: Some("wood"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 148,
        class: &[
            "WeightedPressurePlateBlock",
            "AbstractPressurePlateBlock",
            "Block",
        ],
        sound: Some("wood"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 149,
        class: &[
            "ComparatorBlock",
            "DiodeBlock",
            "HorizontalFacingBlock",
            "Block",
        ],
        sound: Some("wood"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: true,
    },
    Extra {
        id: 150,
        class: &[
            "ComparatorBlock",
            "DiodeBlock",
            "HorizontalFacingBlock",
            "Block",
        ],
        sound: Some("wood"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: true,
    },
    Extra {
        id: 151,
        class: &["DaylightDetectorBlock", "BlockWithBlockEntity", "Block"],
        sound: Some("wood"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: true,
    },
    Extra {
        id: 152,
        class: &["RedstoneBlock", "MineralBlock", "Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.5,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 153,
        class: &["OreBlock", "Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 154,
        class: &["HopperBlock", "BlockWithBlockEntity", "Block"],
        sound: Some("wood"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: true,
    },
    Extra {
        id: 155,
        class: &["QuartzBlock", "Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 156,
        class: &["StairsBlock", "Block"],
        sound: None,
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 157,
        class: &["PoweredRailBlock", "AbstractRailBlock", "Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.5,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 158,
        class: &[
            "DropperBlock",
            "DispenserBlock",
            "BlockWithBlockEntity",
            "Block",
        ],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: true,
    },
    Extra {
        id: 159,
        class: &["ColoredBlock", "Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 160,
        class: &["StainedGlassPaneBlock", "PaneBlock", "Block"],
        sound: Some("glass"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 161,
        class: &[
            "Leaves2Block",
            "AbstractLeavesBlock",
            "TranslucentBlock",
            "Block",
        ],
        sound: Some("grass"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 162,
        class: &["Log2Block", "AbstractLogBlock", "AxisBlock", "Block"],
        sound: Some("wood"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 163,
        class: &["StairsBlock", "Block"],
        sound: None,
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 164,
        class: &["StairsBlock", "Block"],
        sound: None,
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 170,
        class: &["HayBlock", "AxisBlock", "Block"],
        sound: Some("grass"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 171,
        class: &["CarpetBlock", "Block"],
        sound: Some("cloth"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
    Extra {
        id: 172,
        class: &["HardenedClayBlock", "Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 173,
        class: &["Block"],
        sound: Some("stone"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 174,
        class: &["PackedIceBlock", "Block"],
        sound: Some("glass"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: false,
        block_entity: false,
    },
    Extra {
        id: 175,
        class: &["DoublePlantBlock", "PlantBlock", "Block"],
        sound: Some("grass"),
        volume: 1.0,
        pitch: 1.0,
        random_ticks: true,
        block_entity: false,
    },
];

/// The shape class a 1.7.10 block belongs to, for the collision and occlusion
/// facts the export carries.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Shape {
    Air,
    Liquid,
    Cube,
    Slab,
    Stairs,
    Other,
}

pub(crate) fn shape(def: &BlockDef) -> Shape {
    match def.name {
        "air" => Shape::Air,
        "flowing_water" | "water" | "flowing_lava" | "lava" => Shape::Liquid,
        "double_stone_slab" | "double_wooden_slab" => Shape::Cube,
        "stone_slab" | "wooden_slab" => Shape::Slab,
        name if name.ends_with("_stairs") => Shape::Stairs,
        name if is_not_cube(name) => Shape::Other,
        _ => Shape::Cube,
    }
}

/// Blocks whose 1.7.10 collision or model is not a full cube. Everything not
/// named here is a full cube; the plants, rails, torches and furniture are the
/// exceptions.
fn is_not_cube(name: &str) -> bool {
    matches!(
        name,
        "anvil"
            | "bed"
            | "brewing_stand"
            | "cactus"
            | "cake"
            | "cauldron"
            | "carrots"
            | "chest"
            | "cocoa"
            | "cobblestone_wall"
            | "daylight_detector"
            | "deadbush"
            | "double_plant"
            | "dragon_egg"
            | "enchanting_table"
            | "end_portal"
            | "end_portal_frame"
            | "ender_chest"
            | "farmland"
            | "fence"
            | "fence_gate"
            | "fire"
            | "flower_pot"
            | "glass_pane"
            | "heavy_weighted_pressure_plate"
            | "hopper"
            | "iron_bars"
            | "iron_door"
            | "ladder"
            | "lever"
            | "light_weighted_pressure_plate"
            | "melon_stem"
            | "nether_brick_fence"
            | "nether_wart"
            | "piston"
            | "piston_extension"
            | "piston_head"
            | "portal"
            | "potatoes"
            | "pumpkin_stem"
            | "rail"
            | "red_flower"
            | "red_mushroom"
            | "redstone_torch"
            | "redstone_wire"
            | "reeds"
            | "sapling"
            | "skull"
            | "snow_layer"
            | "stained_glass_pane"
            | "standing_sign"
            | "stone_button"
            | "stone_pressure_plate"
            | "tallgrass"
            | "torch"
            | "trapped_chest"
            | "trapdoor"
            | "tripwire"
            | "tripwire_hook"
            | "unlit_redstone_torch"
            | "unpowered_comparator"
            | "unpowered_repeater"
            | "vine"
            | "wall_sign"
            | "waterlily"
            | "wheat"
            | "wooden_button"
            | "wooden_door"
            | "wooden_pressure_plate"
            | "yellow_flower"
            | "powered_comparator"
            | "powered_repeater"
            | "activator_rail"
            | "detector_rail"
            | "golden_rail"
            | "brown_mushroom"
            | "carpet"
    )
}

/// The metadata values a block uses. Blocks whose variants are known emit only
/// those; a block whose metadata is opaque (direction, flags, state bits) emits
/// all sixteen, which is what a saved world may contain.
pub(crate) fn metas(name: &str) -> Vec<u8> {
    match name {
        "air" => vec![0],
        "stone" => (0..=6).collect(),
        "dirt" => (0..=2).collect(),
        "planks" => (0..=5).collect(),
        "sapling" => vec![0, 1, 2, 3, 4, 5, 8, 9, 10, 11, 12, 13],
        "sand" => (0..=1).collect(),
        // The wood type is the low two bits, the axis bits 2..=3: 0..=11 are
        // the types on the three axes.
        "log" => (0..=11).collect(),
        "log2" => vec![0, 1, 4, 5, 8, 9],
        "yellow_flower" => vec![0],
        "red_flower" => (0..=8).collect(),
        "sandstone" => (0..=2).collect(),
        "stonebrick" => (0..=3).collect(),
        "pumpkin" | "lit_pumpkin" => (0..=3).collect(),
        "monster_egg" => (0..=5).collect(),
        "cobblestone_wall" => (0..=1).collect(),
        "anvil" => (0..=11).collect(),
        "double_plant" => (0..=14).collect(),
        "tallgrass" => (0..=2).collect(),
        "snow_layer" => (0..=7).collect(),
        "cake" => (0..=6).collect(),
        "cauldron" => (0..=3).collect(),
        "bed" => vec![0, 1, 2, 3, 8, 9, 10, 11],
        "nether_wart" => (0..=3).collect(),
        "wheat" | "carrots" | "potatoes" => (0..=7).collect(),
        "cocoa" => (0..=11).collect(),
        "farmland" => (0..=7).collect(),
        "portal" => (0..=3).collect(),
        "pumpkin_stem" | "melon_stem" => (0..=7).collect(),
        "sponge" => (0..=1).collect(),
        "hay_block" => (0..=2).collect(),
        "quartz_block" => (0..=4).collect(),
        "jukebox" => (0..=1).collect(),
        _ => (0..=15).collect(),
    }
}

/// `Material.getColor()` of 1.7.10 with the block classes' `getMapColor`
/// overrides; `wool` and `stained_hardened_clay` use `MapColor.forDyeColor`.
pub(crate) fn map_color(def: &BlockDef, meta: u8) -> u32 {
    match def.name {
        "wool" | "stained_hardened_clay" => dye_color(meta),
        "sand" if meta == 1 => 12_020_271,
        "sand" => 16_247_203,
        "end_portal" | "obsidian" => 1_381_407,
        "fire" | "redstone_block" => 16_711_680,
        "glowstone" => 16_247_203,
        "hardened_clay" => 14_188_339,
        "lapis_block" => 4_882_687,
        "gold_block" => 16_445_005,
        "iron_block" => 10_987_431,
        "diamond_block" => 6_085_589,
        "emerald_block" => 55_610,
        "netherrack" => 7_340_544,
        "quartz_block" | "quartz_stairs" => 16_776_437,
        "oak_stairs" | "spruce_stairs" | "birch_stairs" | "jungle_stairs" | "acacia_stairs"
        | "dark_oak_stairs" => 6_837_042,
        _ => material_color(def.material),
    }
}

fn material_color(material: Material) -> u32 {
    match material {
        Material::Air => 0,
        Material::Grass => 8_368_696,
        Material::Dirt => 12_020_271,
        Material::Wood => 6_837_042,
        Material::Stone => 7_368_816,
        Material::Iron | Material::Anvil => 10_987_431,
        Material::Water => 4_210_943,
        Material::Lava => 16_711_680,
        Material::Leaves | Material::Plant | Material::ReplaceablePlant => 31_744,
        Material::Sponge | Material::Wool | Material::Carpet | Material::Cobweb => 10_987_431,
        Material::Fire | Material::Glass | Material::RedstoneLamp | Material::Portal => 0,
        Material::Sand => 16_247_203,
        Material::Decoration | Material::Cake => 0,
        Material::Tnt => 16_711_680,
        Material::Ice | Material::PackedIce => 10_526_975,
        Material::SnowLayer | Material::Snow => 16_777_215,
        Material::Cactus | Material::Pumpkin | Material::Egg => 31_744,
        Material::Clay => 10_791_096,
        Material::Piston => 7_368_816,
    }
}

/// `MapColor.forDyeColor` for a block metadata (0 white .. 15 black).
fn dye_color(meta: u8) -> u32 {
    match meta {
        0 => 16_777_215,
        1 => 14_188_339,
        2 => 11_685_080,
        3 => 6_724_056,
        4 => 15_066_419,
        5 => 8_375_321,
        6 => 15_892_389,
        7 => 5_000_268,
        8 => 10_066_329,
        9 => 5_013_401,
        10 => 8_339_378,
        11 => 3_361_970,
        12 => 6_704_179,
        13 => 6_717_235,
        14 => 10_040_115,
        15 => 1_644_825,
        _ => 0,
    }
}

pub(crate) fn sound(def: &BlockDef, extra: &Extra) -> (&'static str, f32, f32) {
    let key = extra.sound.unwrap_or_else(|| {
        // Stairs take their base block's sound in their constructor; the other
        // classes without a literal sound keep `Block`'s default, stone.
        if extra.class.contains(&"StairsBlock") {
            fallback_sound(def.material)
        } else {
            "stone"
        }
    });
    (key, extra.volume, extra.pitch)
}

fn fallback_sound(material: Material) -> &'static str {
    match material {
        Material::Wood => "wood",
        Material::Sand => "sand",
        Material::Wool | Material::Carpet => "cloth",
        Material::Snow | Material::SnowLayer => "snow",
        Material::Grass
        | Material::Dirt
        | Material::Leaves
        | Material::Plant
        | Material::ReplaceablePlant
        | Material::Pumpkin
        | Material::Egg
        | Material::Cactus
        | Material::Sponge
        | Material::Tnt => "grass",
        _ => "stone",
    }
}

/// The five sound events 1.7.10's `Block$Sounds` builds, as 26.3 event ids:
/// break, step, place, hit, fall.
pub(crate) fn sound_ids(key: &str) -> [String; 5] {
    let event = |kind: &str| format!("minecraft:block.{key}.{kind}");
    match key {
        // 1.7.10 calls the wool sound `cloth`; 26.3's event is `block.wool`.
        "cloth" => [
            "minecraft:block.wool.break".to_owned(),
            "minecraft:block.wool.step".to_owned(),
            "minecraft:block.wool.place".to_owned(),
            "minecraft:block.wool.hit".to_owned(),
            "minecraft:block.wool.fall".to_owned(),
        ],
        "glass" => [
            event("break"),
            "minecraft:block.stone.step".to_owned(),
            event("place"),
            "minecraft:block.stone.hit".to_owned(),
            "minecraft:block.stone.fall".to_owned(),
        ],
        "ladder" => [
            "minecraft:block.wood.break".to_owned(),
            event("step"),
            "minecraft:block.wood.place".to_owned(),
            event("hit"),
            event("fall"),
        ],
        "anvil" => [
            "minecraft:block.stone.break".to_owned(),
            event("step"),
            "minecraft:block.anvil.land".to_owned(),
            event("hit"),
            event("fall"),
        ],
        _ => [
            event("break"),
            event("step"),
            event("place"),
            event("hit"),
            event("fall"),
        ],
    }
}

/// The piston reaction 1.7.10's `Material.getPistonMoveBehavior` names, or
/// `None` for `PUSH_PULL` (the engine's default).
pub(crate) fn push(def: &BlockDef) -> Option<&'static str> {
    match def.material.piston_move_behavior() {
        1 => Some("POPPED"),
        2 => Some("IMMOVEABLE"),
        _ => None,
    }
}

pub(crate) fn is_signal_source(name: &str) -> bool {
    matches!(
        name,
        "redstone_wire"
            | "unlit_redstone_torch"
            | "redstone_torch"
            | "unpowered_repeater"
            | "powered_repeater"
            | "unpowered_comparator"
            | "powered_comparator"
            | "lever"
            | "stone_button"
            | "wooden_button"
            | "stone_pressure_plate"
            | "wooden_pressure_plate"
            | "light_weighted_pressure_plate"
            | "heavy_weighted_pressure_plate"
            | "detector_rail"
            | "daylight_detector"
            | "redstone_block"
            | "tripwire_hook"
            | "tripwire"
    )
}

pub(crate) fn is_analog_output(name: &str) -> bool {
    matches!(
        name,
        "daylight_detector" | "unpowered_comparator" | "powered_comparator"
    )
}

/// The fluid kind (in `FluidKind::ALL` order), level and falling flag of a
/// water or lava state; 1.7.10 stores the level in the low three metadata bits
/// and the falling flag in bit 3.
pub(crate) fn fluid(def: &BlockDef, meta: u8) -> (u8, u8, bool) {
    let kind = match def.name {
        "flowing_water" => 1,
        "water" => 2,
        "flowing_lava" => 3,
        "lava" => 4,
        _ => return (0, 0, false),
    };
    let level = match meta & 7 {
        0 => 8,
        level => 8 - level,
    };
    (kind, level, meta >= 8)
}

pub(crate) fn sturdy(def: &BlockDef) -> u32 {
    match shape(def) {
        Shape::Cube => 63,
        _ => 0,
    }
}

pub(crate) fn collision_empty(def: &BlockDef) -> bool {
    match shape(def) {
        Shape::Air | Shape::Liquid => true,
        Shape::Other => {
            matches!(
                def.name,
                "activator_rail"
                    | "brown_mushroom"
                    | "carrots"
                    | "cocoa"
                    | "deadbush"
                    | "detector_rail"
                    | "double_plant"
                    | "end_portal"
                    | "fire"
                    | "golden_rail"
                    | "lever"
                    | "melon_stem"
                    | "nether_wart"
                    | "portal"
                    | "potatoes"
                    | "pumpkin_stem"
                    | "rail"
                    | "red_flower"
                    | "red_mushroom"
                    | "redstone_torch"
                    | "redstone_wire"
                    | "reeds"
                    | "sapling"
                    | "stone_button"
                    | "tallgrass"
                    | "torch"
                    | "tripwire"
                    | "unlit_redstone_torch"
                    | "vine"
                    | "waterlily"
                    | "wheat"
                    | "wooden_button"
                    | "yellow_flower"
            )
        }
        Shape::Cube | Shape::Slab | Shape::Stairs => false,
    }
}

pub(crate) fn random_ticks(extra: &Extra) -> bool {
    extra.random_ticks
}

pub(crate) fn block_entity(extra: &Extra) -> bool {
    extra.block_entity
}
