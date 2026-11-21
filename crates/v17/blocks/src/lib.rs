//! Minecraft 1.7.10 blocks: the numeric registry from `Block.init()`, ported
//! as tables. Values come from `net.minecraft.block.Block.init()` and the
//! block classes' constructors (calamus decompiled sources), not from memory.
//!
//! Field meanings match the vanilla fields:
//! - `hardness` = `Block.miningTime` (`setStrength`), negative = unbreakable.
//! - `resistance` = `Block.blastResistance`, the internal field: `setStrength`
//!   lifts it to `strength * 5`, `setBlastResistance(r)` sets it to `r * 3`.
//!   Vanilla's public `getBlastResistance` returns `field / 5`.
//! - `opacity` = `Block.opacity`: 255 for solid-render blocks, 0 otherwise,
//!   then registration-time `setOpacity` overrides.
//! - `light` = `Block.light` = `(int)(15 * setLight(f))`.
//! - `slipperiness` = `Block.slipperiness`, 0.6 unless overridden (ice 0.98).
//! - `tool`/`harvest_level`: the tool class that mines fast and gates drops,
//!   from the `EFFECTIVE_BLOCKS` sets and `canMineBlock` chains of
//!   `PickaxeItem`/`AxeItem`/`ShovelItem`/`ShearsItem`.
//!
//! See docs/ for the port contract.

/// `net.minecraft.block.material.Material` (1.7.10): the flags each material
/// carries, with the class overrides folded into per-variant methods.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Material {
    Air,
    Grass,
    Dirt,
    Wood,
    Stone,
    Iron,
    Anvil,
    Water,
    Lava,
    Leaves,
    Plant,
    ReplaceablePlant,
    Sponge,
    Wool,
    Fire,
    Sand,
    Decoration,
    Carpet,
    Glass,
    RedstoneLamp,
    Tnt,
    Ice,
    PackedIce,
    SnowLayer,
    Snow,
    Cactus,
    Clay,
    Pumpkin,
    Egg,
    Portal,
    Cake,
    Cobweb,
    Piston,
}

impl Material {
    /// `Material.isSolid()`: false for the air, liquid, plant and portal
    /// subclasses.
    pub fn is_solid(self) -> bool {
        !matches!(
            self,
            Material::Air
                | Material::Fire
                | Material::Water
                | Material::Lava
                | Material::Plant
                | Material::ReplaceablePlant
                | Material::Decoration
                | Material::Carpet
                | Material::SnowLayer
                | Material::Portal
        )
    }

    /// `Material.isOpaque()`: false for air and plant subclasses (portal too).
    pub fn is_opaque(self) -> bool {
        !matches!(
            self,
            Material::Air
                | Material::Plant
                | Material::ReplaceablePlant
                | Material::Decoration
                | Material::Carpet
                | Material::SnowLayer
                | Material::Portal
                | Material::Fire
        )
    }

    /// `Material.blocksMovement()`: false for air, liquid, plant, portal and
    /// cobweb subclasses.
    pub fn blocks_movement(self) -> bool {
        !matches!(
            self,
            Material::Air
                | Material::Water
                | Material::Lava
                | Material::Plant
                | Material::ReplaceablePlant
                | Material::Decoration
                | Material::Carpet
                | Material::SnowLayer
                | Material::Portal
                | Material::Fire
                | Material::Cobweb
        )
    }

    /// `Material.isSolidBlocking()`: `translucent ? false : blocksMovement()`.
    /// Translucent: leaves, glass, TNT, ice, snow layer, cactus.
    pub fn is_solid_blocking(self) -> bool {
        match self {
            Material::Leaves
            | Material::Glass
            | Material::Tnt
            | Material::Ice
            | Material::SnowLayer
            | Material::Cactus => false,
            material => material.blocks_movement(),
        }
    }

    /// `Material.isLiquid()`: the liquid subclass only.
    pub fn is_liquid(self) -> bool {
        matches!(self, Material::Water | Material::Lava)
    }

    /// `Material.isFlammable()`: `setFlammable` variants.
    pub fn is_flammable(self) -> bool {
        matches!(
            self,
            Material::Wood
                | Material::Leaves
                | Material::ReplaceablePlant
                | Material::Wool
                | Material::Carpet
        )
    }

    /// `Material.isReplaceable()`: `setReplaceable` variants.
    pub fn is_replaceable(self) -> bool {
        matches!(
            self,
            Material::Air
                | Material::Water
                | Material::Lava
                | Material::ReplaceablePlant
                | Material::SnowLayer
        )
    }

    /// `Material.isToolNotRequired()`, inverted: the `setRequiresTool`
    /// variants (stone, iron, anvil, both snows, cobweb).
    pub fn requires_tool(self) -> bool {
        matches!(
            self,
            Material::Stone
                | Material::Iron
                | Material::Anvil
                | Material::Snow
                | Material::SnowLayer
                | Material::Cobweb
        )
    }

    /// `Material.getPistonMoveBehavior()`: 0 normal, 1 destroy, 2 block.
    pub fn piston_move_behavior(self) -> u8 {
        match self {
            Material::Water
            | Material::Lava
            | Material::Leaves
            | Material::Plant
            | Material::ReplaceablePlant
            | Material::Decoration
            | Material::Fire
            | Material::Pumpkin
            | Material::Egg
            | Material::Cake
            | Material::Cobweb
            | Material::Cactus
            | Material::SnowLayer => 1,
            Material::Anvil | Material::Portal | Material::Piston => 2,
            _ => 0,
        }
    }
}

/// The tool class that mines a block fast and gates its drops, from the item
/// classes' `EFFECTIVE_BLOCKS` sets and `canMineBlock` overrides. `Shears`
/// covers `web` (shears harvest it; swords also do, per `SwordItem`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolClass {
    Pickaxe,
    Axe,
    Shovel,
    Shears,
}

/// One registry row: everything `Block.init()` and the block constructors fix.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlockDef {
    /// Numeric id in `Block.REGISTRY`, 0..=175 (165..=169 unused in 1.7.10).
    pub id: u16,
    /// Registry name (`Block.REGISTRY.register` key).
    pub name: &'static str,
    /// Unlocalized-name suffix: `Block.key`, `getTranslationKey()` builds
    /// `"tile." + key`. `None` for the three blocks vanilla registers without
    /// `setKey` (`piston_head`, `piston_extension`, `end_portal`), where Java
    /// concatenates `"tile." + null` = `"tilenull"`.
    pub key: Option<&'static str>,
    pub material: Material,
    /// `Block.miningTime`; negative = unbreakable.
    pub hardness: f32,
    /// `Block.blastResistance` internal field (see crate docs).
    pub resistance: f32,
    /// `Block.slipperiness`.
    pub slipperiness: f32,
    /// `Block.opacity`.
    pub opacity: u8,
    /// `Block.light`, 0..=15.
    pub light: u8,
    /// Tool class that mines this block fast and harvests its drops.
    pub tool: Option<ToolClass>,
    /// Tier level needed to harvest (0 = any tool of the class; blocks whose
    /// material needs no tool are 0 too).
    pub harvest_level: u8,
}

impl BlockDef {
    /// `getTranslationKey()`: `"tile." + key` (Java string concat, so a
    /// missing key gives `"tilenull"`).
    pub fn translation_key(&self) -> String {
        match self.key {
            Some(key) => format!("tile.{key}"),
            None => "tilenull".to_owned(),
        }
    }

    /// `getName()`: the unlocalized name, `getTranslationKey() + ".name"`.
    pub fn unlocalized_name(&self) -> String {
        format!("{}.name", self.translation_key())
    }

    /// Vanilla's public `getBlastResistance()`: internal field over 5.
    pub fn blast_resistance(&self) -> f32 {
        self.resistance / 5.0
    }
}

/// `Block.REGISTRY` in registration order, every block 1.7.10 registers.
/// Ids 165..=169 have no block in 1.7.10 and are absent here.
pub static BLOCKS: &[BlockDef] = &[
    BlockDef {
        id: 0,
        name: "air",
        key: Some("air"),
        material: Material::Air,
        hardness: 0.0,
        resistance: 0.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: None,
        harvest_level: 0,
    },
    BlockDef {
        id: 1,
        name: "stone",
        key: Some("stone"),
        material: Material::Stone,
        hardness: 1.5,
        resistance: 30.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 2,
        name: "grass",
        key: Some("grass"),
        material: Material::Grass,
        hardness: 0.6,
        resistance: 3.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Shovel),
        harvest_level: 0,
    },
    BlockDef {
        id: 3,
        name: "dirt",
        key: Some("dirt"),
        material: Material::Dirt,
        hardness: 0.5,
        resistance: 2.5,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Shovel),
        harvest_level: 0,
    },
    BlockDef {
        id: 4,
        name: "cobblestone",
        key: Some("stonebrick"),
        material: Material::Stone,
        hardness: 2.0,
        resistance: 30.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 5,
        name: "planks",
        key: Some("wood"),
        material: Material::Wood,
        hardness: 2.0,
        resistance: 15.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 6,
        name: "sapling",
        key: Some("sapling"),
        material: Material::Plant,
        hardness: 0.0,
        resistance: 0.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 7,
        name: "bedrock",
        key: Some("bedrock"),
        material: Material::Stone,
        hardness: -1.0,
        resistance: 18000000.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 8,
        name: "flowing_water",
        key: Some("water"),
        material: Material::Water,
        hardness: 100.0,
        resistance: 500.0,
        slipperiness: 0.6,
        opacity: 3,
        light: 0,
        tool: None,
        harvest_level: 0,
    },
    BlockDef {
        id: 9,
        name: "water",
        key: Some("water"),
        material: Material::Water,
        hardness: 100.0,
        resistance: 500.0,
        slipperiness: 0.6,
        opacity: 3,
        light: 0,
        tool: None,
        harvest_level: 0,
    },
    BlockDef {
        id: 10,
        name: "flowing_lava",
        key: Some("lava"),
        material: Material::Lava,
        hardness: 100.0,
        resistance: 500.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 15,
        tool: None,
        harvest_level: 0,
    },
    BlockDef {
        id: 11,
        name: "lava",
        key: Some("lava"),
        material: Material::Lava,
        hardness: 100.0,
        resistance: 500.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 15,
        tool: None,
        harvest_level: 0,
    },
    BlockDef {
        id: 12,
        name: "sand",
        key: Some("sand"),
        material: Material::Sand,
        hardness: 0.5,
        resistance: 2.5,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Shovel),
        harvest_level: 0,
    },
    BlockDef {
        id: 13,
        name: "gravel",
        key: Some("gravel"),
        material: Material::Sand,
        hardness: 0.6,
        resistance: 3.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Shovel),
        harvest_level: 0,
    },
    BlockDef {
        id: 14,
        name: "gold_ore",
        key: Some("oreGold"),
        material: Material::Stone,
        hardness: 3.0,
        resistance: 15.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 2,
    },
    BlockDef {
        id: 15,
        name: "iron_ore",
        key: Some("oreIron"),
        material: Material::Stone,
        hardness: 3.0,
        resistance: 15.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 1,
    },
    BlockDef {
        id: 16,
        name: "coal_ore",
        key: Some("oreCoal"),
        material: Material::Stone,
        hardness: 3.0,
        resistance: 15.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 17,
        name: "log",
        key: Some("log"),
        material: Material::Wood,
        hardness: 2.0,
        resistance: 10.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 18,
        name: "leaves",
        key: Some("leaves"),
        material: Material::Leaves,
        hardness: 0.2,
        resistance: 1.0,
        slipperiness: 0.6,
        opacity: 1,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 19,
        name: "sponge",
        key: Some("sponge"),
        material: Material::Sponge,
        hardness: 0.6,
        resistance: 3.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: None,
        harvest_level: 0,
    },
    BlockDef {
        id: 20,
        name: "glass",
        key: Some("glass"),
        material: Material::Glass,
        hardness: 0.3,
        resistance: 1.5,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: None,
        harvest_level: 0,
    },
    BlockDef {
        id: 21,
        name: "lapis_ore",
        key: Some("oreLapis"),
        material: Material::Stone,
        hardness: 3.0,
        resistance: 15.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 1,
    },
    BlockDef {
        id: 22,
        name: "lapis_block",
        key: Some("blockLapis"),
        material: Material::Iron,
        hardness: 3.0,
        resistance: 15.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 1,
    },
    BlockDef {
        id: 23,
        name: "dispenser",
        key: Some("dispenser"),
        material: Material::Stone,
        hardness: 3.5,
        resistance: 17.5,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 24,
        name: "sandstone",
        key: Some("sandStone"),
        material: Material::Stone,
        hardness: 0.8,
        resistance: 4.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 25,
        name: "noteblock",
        key: Some("musicBlock"),
        material: Material::Wood,
        hardness: 0.8,
        resistance: 4.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 26,
        name: "bed",
        key: Some("bed"),
        material: Material::Wool,
        hardness: 0.2,
        resistance: 1.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: None,
        harvest_level: 0,
    },
    BlockDef {
        id: 27,
        name: "golden_rail",
        key: Some("goldenRail"),
        material: Material::Decoration,
        hardness: 0.7,
        resistance: 3.5,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 28,
        name: "detector_rail",
        key: Some("detectorRail"),
        material: Material::Decoration,
        hardness: 0.7,
        resistance: 3.5,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 29,
        name: "sticky_piston",
        key: Some("pistonStickyBase"),
        material: Material::Piston,
        hardness: 0.5,
        resistance: 2.5,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: None,
        harvest_level: 0,
    },
    BlockDef {
        id: 30,
        name: "web",
        key: Some("web"),
        material: Material::Cobweb,
        hardness: 4.0,
        resistance: 20.0,
        slipperiness: 0.6,
        opacity: 1,
        light: 0,
        tool: Some(ToolClass::Shears),
        harvest_level: 0,
    },
    BlockDef {
        id: 31,
        name: "tallgrass",
        key: Some("tallgrass"),
        material: Material::ReplaceablePlant,
        hardness: 0.0,
        resistance: 0.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 32,
        name: "deadbush",
        key: Some("deadbush"),
        material: Material::ReplaceablePlant,
        hardness: 0.0,
        resistance: 0.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 33,
        name: "piston",
        key: Some("pistonBase"),
        material: Material::Piston,
        hardness: 0.5,
        resistance: 2.5,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: None,
        harvest_level: 0,
    },
    BlockDef {
        id: 34,
        name: "piston_head",
        key: None,
        material: Material::Piston,
        hardness: 0.5,
        resistance: 2.5,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: None,
        harvest_level: 0,
    },
    BlockDef {
        id: 35,
        name: "wool",
        key: Some("cloth"),
        material: Material::Wool,
        hardness: 0.8,
        resistance: 4.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: None,
        harvest_level: 0,
    },
    BlockDef {
        id: 36,
        name: "piston_extension",
        key: None,
        material: Material::Piston,
        hardness: -1.0,
        resistance: 0.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: None,
        harvest_level: 0,
    },
    BlockDef {
        id: 37,
        name: "yellow_flower",
        key: Some("flower1"),
        material: Material::Plant,
        hardness: 0.0,
        resistance: 0.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 38,
        name: "red_flower",
        key: Some("flower2"),
        material: Material::Plant,
        hardness: 0.0,
        resistance: 0.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 39,
        name: "brown_mushroom",
        key: Some("mushroom"),
        material: Material::Plant,
        hardness: 0.0,
        resistance: 0.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 1,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 40,
        name: "red_mushroom",
        key: Some("mushroom"),
        material: Material::Plant,
        hardness: 0.0,
        resistance: 0.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 41,
        name: "gold_block",
        key: Some("blockGold"),
        material: Material::Iron,
        hardness: 3.0,
        resistance: 30.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 2,
    },
    BlockDef {
        id: 42,
        name: "iron_block",
        key: Some("blockIron"),
        material: Material::Iron,
        hardness: 5.0,
        resistance: 30.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 1,
    },
    BlockDef {
        id: 43,
        name: "double_stone_slab",
        key: Some("stoneSlab"),
        material: Material::Stone,
        hardness: 2.0,
        resistance: 30.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 44,
        name: "stone_slab",
        key: Some("stoneSlab"),
        material: Material::Stone,
        hardness: 2.0,
        resistance: 30.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 45,
        name: "brick_block",
        key: Some("brick"),
        material: Material::Stone,
        hardness: 2.0,
        resistance: 30.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 46,
        name: "tnt",
        key: Some("tnt"),
        material: Material::Tnt,
        hardness: 0.0,
        resistance: 0.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: None,
        harvest_level: 0,
    },
    BlockDef {
        id: 47,
        name: "bookshelf",
        key: Some("bookshelf"),
        material: Material::Wood,
        hardness: 1.5,
        resistance: 7.5,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 48,
        name: "mossy_cobblestone",
        key: Some("stoneMoss"),
        material: Material::Stone,
        hardness: 2.0,
        resistance: 30.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 49,
        name: "obsidian",
        key: Some("obsidian"),
        material: Material::Stone,
        hardness: 50.0,
        resistance: 6000.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 3,
    },
    BlockDef {
        id: 50,
        name: "torch",
        key: Some("torch"),
        material: Material::Decoration,
        hardness: 0.0,
        resistance: 0.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 14,
        tool: None,
        harvest_level: 0,
    },
    BlockDef {
        id: 51,
        name: "fire",
        key: Some("fire"),
        material: Material::Fire,
        hardness: 0.0,
        resistance: 0.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 15,
        tool: None,
        harvest_level: 0,
    },
    BlockDef {
        id: 52,
        name: "mob_spawner",
        key: Some("mobSpawner"),
        material: Material::Stone,
        hardness: 5.0,
        resistance: 25.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 53,
        name: "oak_stairs",
        key: Some("stairsWood"),
        material: Material::Wood,
        hardness: 2.0,
        resistance: 10.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 54,
        name: "chest",
        key: Some("chest"),
        material: Material::Wood,
        hardness: 2.5,
        resistance: 12.5,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 55,
        name: "redstone_wire",
        key: Some("redstoneDust"),
        material: Material::Decoration,
        hardness: 0.0,
        resistance: 0.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: None,
        harvest_level: 0,
    },
    BlockDef {
        id: 56,
        name: "diamond_ore",
        key: Some("oreDiamond"),
        material: Material::Stone,
        hardness: 3.0,
        resistance: 15.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 2,
    },
    BlockDef {
        id: 57,
        name: "diamond_block",
        key: Some("blockDiamond"),
        material: Material::Iron,
        hardness: 5.0,
        resistance: 30.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 2,
    },
    BlockDef {
        id: 58,
        name: "crafting_table",
        key: Some("workbench"),
        material: Material::Wood,
        hardness: 2.5,
        resistance: 12.5,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 59,
        name: "wheat",
        key: Some("crops"),
        material: Material::Plant,
        hardness: 0.0,
        resistance: 0.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 60,
        name: "farmland",
        key: Some("farmland"),
        material: Material::Dirt,
        hardness: 0.6,
        resistance: 3.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Shovel),
        harvest_level: 0,
    },
    BlockDef {
        id: 61,
        name: "furnace",
        key: Some("furnace"),
        material: Material::Stone,
        hardness: 3.5,
        resistance: 17.5,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 62,
        name: "lit_furnace",
        key: Some("furnace"),
        material: Material::Stone,
        hardness: 3.5,
        resistance: 17.5,
        slipperiness: 0.6,
        opacity: 255,
        light: 13,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 63,
        name: "standing_sign",
        key: Some("sign"),
        material: Material::Wood,
        hardness: 1.0,
        resistance: 5.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 64,
        name: "wooden_door",
        key: Some("doorWood"),
        material: Material::Wood,
        hardness: 3.0,
        resistance: 15.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 65,
        name: "ladder",
        key: Some("ladder"),
        material: Material::Decoration,
        hardness: 0.4,
        resistance: 2.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: None,
        harvest_level: 0,
    },
    BlockDef {
        id: 66,
        name: "rail",
        key: Some("rail"),
        material: Material::Decoration,
        hardness: 0.7,
        resistance: 3.5,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 67,
        name: "stone_stairs",
        key: Some("stairsStone"),
        material: Material::Stone,
        hardness: 2.0,
        resistance: 10.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 68,
        name: "wall_sign",
        key: Some("sign"),
        material: Material::Wood,
        hardness: 1.0,
        resistance: 5.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 69,
        name: "lever",
        key: Some("lever"),
        material: Material::Decoration,
        hardness: 0.5,
        resistance: 2.5,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: None,
        harvest_level: 0,
    },
    BlockDef {
        id: 70,
        name: "stone_pressure_plate",
        key: Some("pressurePlate"),
        material: Material::Stone,
        hardness: 0.5,
        resistance: 2.5,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 71,
        name: "iron_door",
        key: Some("doorIron"),
        material: Material::Iron,
        hardness: 5.0,
        resistance: 25.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 72,
        name: "wooden_pressure_plate",
        key: Some("pressurePlate"),
        material: Material::Wood,
        hardness: 0.5,
        resistance: 2.5,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 73,
        name: "redstone_ore",
        key: Some("oreRedstone"),
        material: Material::Stone,
        hardness: 3.0,
        resistance: 15.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 2,
    },
    BlockDef {
        id: 74,
        name: "lit_redstone_ore",
        key: Some("oreRedstone"),
        material: Material::Stone,
        hardness: 3.0,
        resistance: 15.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 9,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 2,
    },
    BlockDef {
        id: 75,
        name: "unlit_redstone_torch",
        key: Some("notGate"),
        material: Material::Decoration,
        hardness: 0.0,
        resistance: 0.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: None,
        harvest_level: 0,
    },
    BlockDef {
        id: 76,
        name: "redstone_torch",
        key: Some("notGate"),
        material: Material::Decoration,
        hardness: 0.0,
        resistance: 0.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 7,
        tool: None,
        harvest_level: 0,
    },
    BlockDef {
        id: 77,
        name: "stone_button",
        key: Some("button"),
        material: Material::Stone,
        hardness: 0.5,
        resistance: 2.5,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 78,
        name: "snow_layer",
        key: Some("snow"),
        material: Material::SnowLayer,
        hardness: 0.1,
        resistance: 0.5,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: Some(ToolClass::Shovel),
        harvest_level: 0,
    },
    BlockDef {
        id: 79,
        name: "ice",
        key: Some("ice"),
        material: Material::Ice,
        hardness: 0.5,
        resistance: 2.5,
        slipperiness: 0.98,
        opacity: 3,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 80,
        name: "snow",
        key: Some("snow"),
        material: Material::Snow,
        hardness: 0.2,
        resistance: 1.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Shovel),
        harvest_level: 0,
    },
    BlockDef {
        id: 81,
        name: "cactus",
        key: Some("cactus"),
        material: Material::Cactus,
        hardness: 0.4,
        resistance: 2.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: None,
        harvest_level: 0,
    },
    BlockDef {
        id: 82,
        name: "clay",
        key: Some("clay"),
        material: Material::Clay,
        hardness: 0.6,
        resistance: 3.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Shovel),
        harvest_level: 0,
    },
    BlockDef {
        id: 83,
        name: "reeds",
        key: Some("reeds"),
        material: Material::Plant,
        hardness: 0.0,
        resistance: 0.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 84,
        name: "jukebox",
        key: Some("jukebox"),
        material: Material::Wood,
        hardness: 2.0,
        resistance: 30.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 85,
        name: "fence",
        key: Some("fence"),
        material: Material::Wood,
        hardness: 2.0,
        resistance: 15.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 86,
        name: "pumpkin",
        key: Some("pumpkin"),
        material: Material::Pumpkin,
        hardness: 1.0,
        resistance: 5.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 87,
        name: "netherrack",
        key: Some("hellrock"),
        material: Material::Stone,
        hardness: 0.4,
        resistance: 2.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 88,
        name: "soul_sand",
        key: Some("hellsand"),
        material: Material::Sand,
        hardness: 0.5,
        resistance: 2.5,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Shovel),
        harvest_level: 0,
    },
    BlockDef {
        id: 89,
        name: "glowstone",
        key: Some("lightgem"),
        material: Material::Glass,
        hardness: 0.3,
        resistance: 1.5,
        slipperiness: 0.6,
        opacity: 255,
        light: 15,
        tool: None,
        harvest_level: 0,
    },
    BlockDef {
        id: 90,
        name: "portal",
        key: Some("portal"),
        material: Material::Portal,
        hardness: -1.0,
        resistance: 0.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 11,
        tool: None,
        harvest_level: 0,
    },
    BlockDef {
        id: 91,
        name: "lit_pumpkin",
        key: Some("litpumpkin"),
        material: Material::Pumpkin,
        hardness: 1.0,
        resistance: 5.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 15,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 92,
        name: "cake",
        key: Some("cake"),
        material: Material::Cake,
        hardness: 0.5,
        resistance: 2.5,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: None,
        harvest_level: 0,
    },
    BlockDef {
        id: 93,
        name: "unpowered_repeater",
        key: Some("diode"),
        material: Material::Decoration,
        hardness: 0.0,
        resistance: 0.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: None,
        harvest_level: 0,
    },
    BlockDef {
        id: 94,
        name: "powered_repeater",
        key: Some("diode"),
        material: Material::Decoration,
        hardness: 0.0,
        resistance: 0.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 9,
        tool: None,
        harvest_level: 0,
    },
    BlockDef {
        id: 95,
        name: "stained_glass",
        key: Some("stainedGlass"),
        material: Material::Glass,
        hardness: 0.3,
        resistance: 1.5,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: None,
        harvest_level: 0,
    },
    BlockDef {
        id: 96,
        name: "trapdoor",
        key: Some("trapdoor"),
        material: Material::Wood,
        hardness: 3.0,
        resistance: 15.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 97,
        name: "monster_egg",
        key: Some("monsterStoneEgg"),
        material: Material::Clay,
        hardness: 0.75,
        resistance: 3.75,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: None,
        harvest_level: 0,
    },
    BlockDef {
        id: 98,
        name: "stonebrick",
        key: Some("stonebricksmooth"),
        material: Material::Stone,
        hardness: 1.5,
        resistance: 30.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 99,
        name: "brown_mushroom_block",
        key: Some("mushroom"),
        material: Material::Wood,
        hardness: 0.2,
        resistance: 1.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 100,
        name: "red_mushroom_block",
        key: Some("mushroom"),
        material: Material::Wood,
        hardness: 0.2,
        resistance: 1.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 101,
        name: "iron_bars",
        key: Some("fenceIron"),
        material: Material::Iron,
        hardness: 5.0,
        resistance: 30.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 102,
        name: "glass_pane",
        key: Some("thinGlass"),
        material: Material::Glass,
        hardness: 0.3,
        resistance: 1.5,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: None,
        harvest_level: 0,
    },
    BlockDef {
        id: 103,
        name: "melon_block",
        key: Some("melon"),
        material: Material::Pumpkin,
        hardness: 1.0,
        resistance: 5.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 104,
        name: "pumpkin_stem",
        key: Some("pumpkinStem"),
        material: Material::Plant,
        hardness: 0.0,
        resistance: 0.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 105,
        name: "melon_stem",
        key: Some("pumpkinStem"),
        material: Material::Plant,
        hardness: 0.0,
        resistance: 0.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 106,
        name: "vine",
        key: Some("vine"),
        material: Material::ReplaceablePlant,
        hardness: 0.2,
        resistance: 1.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 107,
        name: "fence_gate",
        key: Some("fenceGate"),
        material: Material::Wood,
        hardness: 2.0,
        resistance: 15.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 108,
        name: "brick_stairs",
        key: Some("stairsBrick"),
        material: Material::Stone,
        hardness: 2.0,
        resistance: 10.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 109,
        name: "stone_brick_stairs",
        key: Some("stairsStoneBrickSmooth"),
        material: Material::Stone,
        hardness: 1.5,
        resistance: 7.5,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 110,
        name: "mycelium",
        key: Some("mycel"),
        material: Material::Grass,
        hardness: 0.6,
        resistance: 3.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Shovel),
        harvest_level: 0,
    },
    BlockDef {
        id: 111,
        name: "waterlily",
        key: Some("waterlily"),
        material: Material::Plant,
        hardness: 0.0,
        resistance: 0.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 112,
        name: "nether_brick",
        key: Some("netherBrick"),
        material: Material::Stone,
        hardness: 2.0,
        resistance: 30.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 113,
        name: "nether_brick_fence",
        key: Some("netherFence"),
        material: Material::Stone,
        hardness: 2.0,
        resistance: 30.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 114,
        name: "nether_brick_stairs",
        key: Some("stairsNetherBrick"),
        material: Material::Stone,
        hardness: 2.0,
        resistance: 10.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 115,
        name: "nether_wart",
        key: Some("netherStalk"),
        material: Material::Plant,
        hardness: 0.0,
        resistance: 0.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 116,
        name: "enchanting_table",
        key: Some("enchantmentTable"),
        material: Material::Stone,
        hardness: 5.0,
        resistance: 6000.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 117,
        name: "brewing_stand",
        key: Some("brewingStand"),
        material: Material::Iron,
        hardness: 0.5,
        resistance: 2.5,
        slipperiness: 0.6,
        opacity: 0,
        light: 1,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 118,
        name: "cauldron",
        key: Some("cauldron"),
        material: Material::Iron,
        hardness: 2.0,
        resistance: 10.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 119,
        name: "end_portal",
        key: None,
        material: Material::Portal,
        hardness: -1.0,
        resistance: 18000000.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 15,
        tool: None,
        harvest_level: 0,
    },
    BlockDef {
        id: 120,
        name: "end_portal_frame",
        key: Some("endPortalFrame"),
        material: Material::Stone,
        hardness: -1.0,
        resistance: 18000000.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 1,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 121,
        name: "end_stone",
        key: Some("whiteStone"),
        material: Material::Stone,
        hardness: 3.0,
        resistance: 45.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 122,
        name: "dragon_egg",
        key: Some("dragonEgg"),
        material: Material::Egg,
        hardness: 3.0,
        resistance: 45.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 1,
        tool: None,
        harvest_level: 0,
    },
    BlockDef {
        id: 123,
        name: "redstone_lamp",
        key: Some("redstoneLight"),
        material: Material::RedstoneLamp,
        hardness: 0.3,
        resistance: 1.5,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: None,
        harvest_level: 0,
    },
    BlockDef {
        id: 124,
        name: "lit_redstone_lamp",
        key: Some("redstoneLight"),
        material: Material::RedstoneLamp,
        hardness: 0.3,
        resistance: 1.5,
        slipperiness: 0.6,
        opacity: 255,
        light: 15,
        tool: None,
        harvest_level: 0,
    },
    BlockDef {
        id: 125,
        name: "double_wooden_slab",
        key: Some("woodSlab"),
        material: Material::Wood,
        hardness: 2.0,
        resistance: 15.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 126,
        name: "wooden_slab",
        key: Some("woodSlab"),
        material: Material::Wood,
        hardness: 2.0,
        resistance: 15.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 127,
        name: "cocoa",
        key: Some("cocoa"),
        material: Material::Plant,
        hardness: 0.2,
        resistance: 15.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 128,
        name: "sandstone_stairs",
        key: Some("stairsSandStone"),
        material: Material::Stone,
        hardness: 0.8,
        resistance: 4.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 129,
        name: "emerald_ore",
        key: Some("oreEmerald"),
        material: Material::Stone,
        hardness: 3.0,
        resistance: 15.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 2,
    },
    BlockDef {
        id: 130,
        name: "ender_chest",
        key: Some("enderChest"),
        material: Material::Stone,
        hardness: 22.5,
        resistance: 3000.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 7,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 131,
        name: "tripwire_hook",
        key: Some("tripWireSource"),
        material: Material::Decoration,
        hardness: 0.0,
        resistance: 0.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: None,
        harvest_level: 0,
    },
    BlockDef {
        id: 132,
        name: "tripwire",
        key: Some("tripWire"),
        material: Material::Decoration,
        hardness: 0.0,
        resistance: 0.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: None,
        harvest_level: 0,
    },
    BlockDef {
        id: 133,
        name: "emerald_block",
        key: Some("blockEmerald"),
        material: Material::Iron,
        hardness: 5.0,
        resistance: 30.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 2,
    },
    BlockDef {
        id: 134,
        name: "spruce_stairs",
        key: Some("stairsWoodSpruce"),
        material: Material::Wood,
        hardness: 2.0,
        resistance: 10.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 135,
        name: "birch_stairs",
        key: Some("stairsWoodBirch"),
        material: Material::Wood,
        hardness: 2.0,
        resistance: 10.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 136,
        name: "jungle_stairs",
        key: Some("stairsWoodJungle"),
        material: Material::Wood,
        hardness: 2.0,
        resistance: 10.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 137,
        name: "command_block",
        key: Some("commandBlock"),
        material: Material::Iron,
        hardness: -1.0,
        resistance: 18000000.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 138,
        name: "beacon",
        key: Some("beacon"),
        material: Material::Glass,
        hardness: 3.0,
        resistance: 15.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 15,
        tool: None,
        harvest_level: 0,
    },
    BlockDef {
        id: 139,
        name: "cobblestone_wall",
        key: Some("cobbleWall"),
        material: Material::Stone,
        hardness: 2.0,
        resistance: 30.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 140,
        name: "flower_pot",
        key: Some("flowerPot"),
        material: Material::Decoration,
        hardness: 0.0,
        resistance: 0.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: None,
        harvest_level: 0,
    },
    BlockDef {
        id: 141,
        name: "carrots",
        key: Some("carrots"),
        material: Material::Plant,
        hardness: 0.0,
        resistance: 0.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 142,
        name: "potatoes",
        key: Some("potatoes"),
        material: Material::Plant,
        hardness: 0.0,
        resistance: 0.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 143,
        name: "wooden_button",
        key: Some("button"),
        material: Material::Wood,
        hardness: 0.5,
        resistance: 2.5,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 144,
        name: "skull",
        key: Some("skull"),
        material: Material::Decoration,
        hardness: 1.0,
        resistance: 5.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: None,
        harvest_level: 0,
    },
    BlockDef {
        id: 145,
        name: "anvil",
        key: Some("anvil"),
        material: Material::Anvil,
        hardness: 5.0,
        resistance: 6000.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 146,
        name: "trapped_chest",
        key: Some("chestTrap"),
        material: Material::Wood,
        hardness: 2.5,
        resistance: 12.5,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 147,
        name: "light_weighted_pressure_plate",
        key: Some("weightedPlate_light"),
        material: Material::Iron,
        hardness: 0.5,
        resistance: 2.5,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 148,
        name: "heavy_weighted_pressure_plate",
        key: Some("weightedPlate_heavy"),
        material: Material::Iron,
        hardness: 0.5,
        resistance: 2.5,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 149,
        name: "unpowered_comparator",
        key: Some("comparator"),
        material: Material::Decoration,
        hardness: 0.0,
        resistance: 0.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: None,
        harvest_level: 0,
    },
    BlockDef {
        id: 150,
        name: "powered_comparator",
        key: Some("comparator"),
        material: Material::Decoration,
        hardness: 0.0,
        resistance: 0.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 9,
        tool: None,
        harvest_level: 0,
    },
    BlockDef {
        id: 151,
        name: "daylight_detector",
        key: Some("daylightDetector"),
        material: Material::Wood,
        hardness: 0.2,
        resistance: 1.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 152,
        name: "redstone_block",
        key: Some("blockRedstone"),
        material: Material::Iron,
        hardness: 5.0,
        resistance: 30.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 153,
        name: "quartz_ore",
        key: Some("netherquartz"),
        material: Material::Stone,
        hardness: 3.0,
        resistance: 15.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 154,
        name: "hopper",
        key: Some("hopper"),
        material: Material::Iron,
        hardness: 3.0,
        resistance: 24.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 155,
        name: "quartz_block",
        key: Some("quartzBlock"),
        material: Material::Stone,
        hardness: 0.8,
        resistance: 4.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 156,
        name: "quartz_stairs",
        key: Some("stairsQuartz"),
        material: Material::Stone,
        hardness: 0.8,
        resistance: 4.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 157,
        name: "activator_rail",
        key: Some("activatorRail"),
        material: Material::Decoration,
        hardness: 0.7,
        resistance: 3.5,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 158,
        name: "dropper",
        key: Some("dropper"),
        material: Material::Stone,
        hardness: 3.5,
        resistance: 17.5,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 159,
        name: "stained_hardened_clay",
        key: Some("clayHardenedStained"),
        material: Material::Stone,
        hardness: 1.25,
        resistance: 21.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 160,
        name: "stained_glass_pane",
        key: Some("thinStainedGlass"),
        material: Material::Glass,
        hardness: 0.3,
        resistance: 1.5,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: None,
        harvest_level: 0,
    },
    BlockDef {
        id: 161,
        name: "leaves2",
        key: Some("leaves"),
        material: Material::Leaves,
        hardness: 0.2,
        resistance: 1.0,
        slipperiness: 0.6,
        opacity: 1,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 162,
        name: "log2",
        key: Some("log"),
        material: Material::Wood,
        hardness: 2.0,
        resistance: 10.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 163,
        name: "acacia_stairs",
        key: Some("stairsWoodAcacia"),
        material: Material::Wood,
        hardness: 2.0,
        resistance: 10.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 164,
        name: "dark_oak_stairs",
        key: Some("stairsWoodDarkOak"),
        material: Material::Wood,
        hardness: 2.0,
        resistance: 10.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
    BlockDef {
        id: 170,
        name: "hay_block",
        key: Some("hayBlock"),
        material: Material::Grass,
        hardness: 0.5,
        resistance: 2.5,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: None,
        harvest_level: 0,
    },
    BlockDef {
        id: 171,
        name: "carpet",
        key: Some("woolCarpet"),
        material: Material::Carpet,
        hardness: 0.1,
        resistance: 0.5,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: None,
        harvest_level: 0,
    },
    BlockDef {
        id: 172,
        name: "hardened_clay",
        key: Some("clayHardened"),
        material: Material::Stone,
        hardness: 1.25,
        resistance: 21.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 173,
        name: "coal_block",
        key: Some("blockCoal"),
        material: Material::Stone,
        hardness: 5.0,
        resistance: 30.0,
        slipperiness: 0.6,
        opacity: 255,
        light: 0,
        tool: Some(ToolClass::Pickaxe),
        harvest_level: 0,
    },
    BlockDef {
        id: 174,
        name: "packed_ice",
        key: Some("icePacked"),
        material: Material::PackedIce,
        hardness: 0.5,
        resistance: 2.5,
        slipperiness: 0.98,
        opacity: 255,
        light: 0,
        tool: None,
        harvest_level: 0,
    },
    BlockDef {
        id: 175,
        name: "double_plant",
        key: Some("doublePlant"),
        material: Material::Plant,
        hardness: 0.0,
        resistance: 0.0,
        slipperiness: 0.6,
        opacity: 0,
        light: 0,
        tool: Some(ToolClass::Axe),
        harvest_level: 0,
    },
];

/// `Block.REGISTRY`'s id space: 0..=175 with 165..=169 unassigned in 1.7.10.
pub const MAX_ID: u16 = 175;

/// `Block.byId(id)`: the registry row, or `None` for unused ids.
pub fn by_id(id: u16) -> Option<&'static BlockDef> {
    if id > MAX_ID || (165..=169).contains(&id) {
        return None;
    }
    // The table is sorted by id and covers 0..=164 plus 170..=175: index
    // directly, with one subtract for the five missing slots.
    let index = if id <= 164 { id } else { id - 5 } as usize;
    debug_assert_eq!(BLOCKS[index].id, id);
    Some(&BLOCKS[index])
}

/// `Block.byKey(name)`: the registry row with that name.
pub fn by_name(name: &str) -> Option<&'static BlockDef> {
    BLOCKS.iter().find(|block| block.name == name)
}

/// `Block.getId(block)` style: the id of a registry row.
pub fn id_of(block: &BlockDef) -> u16 {
    block.id
}

/// DefaultedIdRegistry's fallback name: vanilla registers air at id 0.
pub const AIR: &BlockDef = &BLOCKS[0];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_strictly_increasing_and_in_range() {
        let mut last = None;
        for block in BLOCKS {
            assert!(block.id <= MAX_ID);
            assert_ne!(Some(block.id), last, "duplicate id {}", block.id);
            if let Some(last) = last {
                assert!(block.id > last);
            }
            last = Some(block.id);
        }
        // 171 registered blocks: 165 ids in 0..=164 plus 6 in 170..=175.
        assert_eq!(BLOCKS.len(), 165 + 6);
    }

    #[test]
    fn names_unique() {
        for (i, a) in BLOCKS.iter().enumerate() {
            for b in &BLOCKS[i + 1..] {
                assert_ne!(a.name, b.name, "duplicate name {}", a.name);
            }
        }
    }

    #[test]
    fn light_never_exceeds_15() {
        for block in BLOCKS {
            assert!(block.light <= 15, "{}", block.name);
        }
    }

    #[test]
    fn resistance_never_negative() {
        for block in BLOCKS {
            assert!(block.resistance >= 0.0, "{}", block.name);
        }
    }

    #[test]
    fn slipperiness_only_vanilla_values() {
        for block in BLOCKS {
            assert!(
                block.slipperiness == 0.6 || block.slipperiness == 0.98,
                "{} has slipperiness {}",
                block.name,
                block.slipperiness
            );
        }
    }

    #[test]
    fn tool_required_materials_all_have_a_tool() {
        for block in BLOCKS {
            if block.material.requires_tool() {
                assert!(
                    block.tool.is_some(),
                    "{} needs a tool but has none",
                    block.name
                );
            }
        }
    }

    #[test]
    fn by_id_roundtrips_every_row() {
        for block in BLOCKS {
            assert_eq!(by_id(block.id), Some(block));
            assert_eq!(by_name(block.name), Some(block));
        }
        for id in [165, 166, 167, 168, 169] {
            assert_eq!(by_id(id), None, "id {id} is unused in 1.7.10");
        }
        assert_eq!(by_id(176), None);
        assert_eq!(by_name("not_a_block"), None);
    }

    #[test]
    fn translation_keys_match_vanilla() {
        assert_eq!(by_name("log").unwrap().translation_key(), "tile.log");
        assert_eq!(by_name("log").unwrap().unlocalized_name(), "tile.log.name");
        assert_eq!(by_name("wool").unwrap().translation_key(), "tile.cloth");
        assert_eq!(by_name("planks").unwrap().translation_key(), "tile.wood");
        // Vanilla registers these without setKey; Java concats "tile." + null.
        for name in ["piston_head", "piston_extension", "end_portal"] {
            assert_eq!(by_name(name).unwrap().translation_key(), "tilenull");
        }
    }

    #[test]
    fn spot_checks_against_vanilla_values() {
        let stone = by_name("stone").unwrap();
        assert_eq!(stone.id, 1);
        assert_eq!(stone.hardness, 1.5);
        assert_eq!(stone.resistance, 30.0);
        assert_eq!(stone.blast_resistance(), 6.0);
        assert_eq!(stone.material, Material::Stone);

        let obsidian = by_name("obsidian").unwrap();
        assert_eq!(obsidian.harvest_level, 3);
        assert_eq!(obsidian.resistance, 6000.0);

        assert_eq!(by_name("iron_ore").unwrap().harvest_level, 1);
        assert_eq!(by_name("gold_ore").unwrap().harvest_level, 2);
        assert_eq!(by_name("coal_ore").unwrap().harvest_level, 0);

        let ice = by_name("ice").unwrap();
        assert_eq!(ice.slipperiness, 0.98);
        assert_eq!(ice.opacity, 3);
        assert_eq!(ice.tool, Some(ToolClass::Pickaxe));

        assert_eq!(by_name("packed_ice").unwrap().slipperiness, 0.98);

        let torch = by_name("torch").unwrap();
        assert_eq!(torch.light, 14); // (int)(15 * 0.9375)

        let lava = by_name("lava").unwrap();
        assert_eq!(lava.light, 15);
        assert_eq!(lava.opacity, 0);
        assert_eq!(lava.resistance, 500.0);

        let water = by_name("water").unwrap();
        assert_eq!(water.opacity, 3);
        assert_eq!(water.light, 0);

        let web = by_name("web").unwrap();
        assert_eq!(web.tool, Some(ToolClass::Shears));
        assert!(web.material.requires_tool());
        assert_eq!(web.opacity, 1);

        let bedrock = by_name("bedrock").unwrap();
        assert_eq!(bedrock.hardness, -1.0);
        assert_eq!(bedrock.resistance, 18000000.0);

        // Half slabs keep full opacity in vanilla (`SlabBlock.setOpacity(255)`).
        assert_eq!(by_name("stone_slab").unwrap().opacity, 255);
        assert_eq!(by_name("snow_layer").unwrap().opacity, 0);

        // Gravel extends FallingBlock, whose material is sand.
        assert_eq!(by_name("gravel").unwrap().material, Material::Sand);
        // Leaves are translucent but keep opacity 1 (`setOpacity(1)`).
        assert_eq!(by_name("leaves").unwrap().opacity, 1);
        // Stairs inherit the base block's hardness (oak stairs over planks).
        assert_eq!(by_name("oak_stairs").unwrap().hardness, 2.0);
        assert_eq!(by_name("sandstone_stairs").unwrap().hardness, 0.8);
    }

    #[test]
    fn material_flags_match_vanilla_materials() {
        assert!(Material::Air.is_replaceable());
        assert!(!Material::Air.blocks_movement());
        assert!(Material::Water.is_liquid());
        assert!(Material::Water.is_replaceable());
        assert!(!Material::Water.blocks_movement());
        assert!(Material::Stone.requires_tool());
        assert!(Material::Snow.requires_tool());
        assert!(Material::SnowLayer.requires_tool());
        assert!(Material::Cobweb.requires_tool());
        // The cobweb material subclass does not block movement (walk through).
        assert!(!Material::Cobweb.blocks_movement());
        assert!(!Material::Decoration.blocks_movement());
        assert!(Material::Wood.is_flammable());
        assert!(Material::Leaves.is_flammable());
        // Translucent materials do not block solid despite blocking movement.
        assert!(!Material::Leaves.is_solid_blocking());
        assert!(!Material::Ice.is_solid_blocking());
        assert!(!Material::Glass.is_solid_blocking());
        // Packed ice is not translucent: it blocks solid.
        assert!(Material::PackedIce.is_solid_blocking());
        assert!(!Material::Fire.is_solid());
        assert!(!Material::Portal.is_solid());
        assert_eq!(Material::Piston.piston_move_behavior(), 2);
        assert_eq!(Material::Anvil.piston_move_behavior(), 2);
        assert_eq!(Material::Plant.piston_move_behavior(), 1);
        assert_eq!(Material::Sponge.piston_move_behavior(), 0);
        assert!(!Material::Plant.is_solid());
        assert!(!Material::Plant.is_opaque());
        assert!(!Material::Portal.is_solid());
        assert!(!Material::Fire.is_opaque());
    }
}
