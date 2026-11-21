//! The 1.7.10 content export's differences from 26.3, for the behaviour tables.
//!
//! A prepared 1.7.10 resource root (`assets-1.7.10.json`, or a registry whose
//! states carry only `meta`) has 1.7.10's block names (`grass`, `snow`,
//! `reeds`, ...), one `meta` property per state instead of 26.3's typed
//! properties (`facing`, `half`, `axis`, ...), and none of the 26.3 data-pack
//! tables (`block-tags.json`, `cached-collision.nbt`, the fluid tags). This
//! module supplies what [`super::data`] needs for such a root:
//!
//! * [`block_name`] maps every 26.3 block identity [`super::data::Named`]
//!   looks up to the era's name, or to a documented stand-in where 1.7.10 has
//!   no equivalent;
//! * [`install_properties`] materialises the typed properties 1.7.10 encodes
//!   in metadata, so the ported behaviour code reads the same values;
//! * [`class_from_java`] maps the era's class names onto the port's classes;
//! * [`tag_contains`] answers the block tags the ported code consults from the
//!   era's block names;
//! * [`fluid_tags`] is the era's liquid tag set;
//! * [`collision_shape`] builds the collision shape of an era state from
//!   1.7.10's shapes, because the era export has no collision table.
//!
//! # Stand-ins
//!
//! Every mapping that is not the era's own block is marked "stand-in" in
//! [`block_name`]; a stand-in means 1.7.10 has no equivalent block, so the
//! ported behaviour that asks for it reads the closest block instead (or
//! `air`, for a block that did not exist yet). Behaviour that depended on the
//! missing block is approximated: the world opens and the era's own blocks
//! behave normally, but a rule about, say, observers or campfires has nothing
//! to act on.
//!
//! # What remains
//!
//! The metadata properties installed below cover the already-ported liquid,
//! falling, growth and redstone code and the block identities world entry
//! needs. A property the registry already carries is kept as it is: the
//! client's era registry adds a constant `level` to the source liquids and a
//! constant `facing` to ladders for the 26.3 fluid and climbing code, so those
//! two properties stay constant on that root. Not yet translated (values read
//! as the property's default): doors, trapdoors and fence gates
//! (`facing`/`half`/`hinge`/`open`/`in_wall` from metadata), beds
//! (`facing`/`part`/`occupied`), signs (`rotation`, `waterlogged`), torches and
//! wall torches (`facing`), rails (`shape`/`powered`/`waterlogged`), the
//! diagonal `facing` of pumpkins, piston heads' `short`, and the era's
//! separate `flowing_water`/`flowing_lava` blocks when a fluid state leaves the
//! engine as a legacy block.
use super::{
    classes::Class,
    data::{BlockSet, FluidTags},
    states::{BlockId, StateTable},
};
use crate::{
    compression::invalid,
    fluid::FluidKind,
    registry::{BlockRegistry, BlockStateId},
};
use std::{collections::HashMap, io, path::Path, sync::Arc};
use sunrise_core::geometry::{Aabb, Vec3, VoxelShape};

/// The manifest `sunrise-prepare` writes beside a prepared 1.7.10 folder.
pub(super) const MANIFEST: &str = "assets-1.7.10.json";

/// Whether `root` holds the era's content export: its manifest is there, or
/// every registry state carries 1.7.10's `meta` property (a client root adds
/// `level` to the liquid states and `facing` to ladders, so `meta` need not be
/// the only property).
pub(super) fn detect(root: &Path, registry: &BlockRegistry) -> bool {
    root.join(MANIFEST).is_file() || states_carry_meta(registry)
}

fn states_carry_meta(registry: &BlockRegistry) -> bool {
    !registry.is_empty()
        && (0..registry.len()).all(|index| {
            registry
                .state(BlockStateId(index as u32))
                .is_some_and(|state| state.properties.contains_key("meta"))
        })
}

/// The era's block name for a 26.3 identity `Named` resolves, or `None` when
/// the era has the block under its own name and the caller's own lookup does.
///
/// Names marked "stand-in" are approximations; see the module docs. A stand-in
/// of `air` is a block 1.7.10 did not have, so ported behaviour about it has
/// nothing to act on.
pub(super) fn block_name(modern: &str) -> Option<&'static str> {
    Some(match modern {
        // The era's own names.
        "minecraft:air" => "minecraft:air",
        "minecraft:water" => "minecraft:water",
        "minecraft:flowing_water" => "minecraft:flowing_water",
        "minecraft:lava" => "minecraft:lava",
        "minecraft:flowing_lava" => "minecraft:flowing_lava",
        "minecraft:stone" => "minecraft:stone",
        "minecraft:cobblestone" => "minecraft:cobblestone",
        "minecraft:obsidian" => "minecraft:obsidian",
        "minecraft:ice" => "minecraft:ice",
        "minecraft:dirt" => "minecraft:dirt",
        "minecraft:mycelium" => "minecraft:mycelium",
        "minecraft:farmland" => "minecraft:farmland",
        "minecraft:clay" => "minecraft:clay",
        "minecraft:sponge" => "minecraft:sponge",
        "minecraft:fire" => "minecraft:fire",
        "minecraft:redstone_wire" => "minecraft:redstone_wire",
        "minecraft:redstone_block" => "minecraft:redstone_block",
        "minecraft:redstone_lamp" => "minecraft:redstone_lamp",
        "minecraft:piston" => "minecraft:piston",
        "minecraft:sticky_piston" => "minecraft:sticky_piston",
        "minecraft:piston_head" => "minecraft:piston_head",
        "minecraft:tnt" => "minecraft:tnt",
        "minecraft:soul_sand" => "minecraft:soul_sand",
        "minecraft:cactus" => "minecraft:cactus",
        "minecraft:pumpkin" => "minecraft:pumpkin",
        "minecraft:cake" => "minecraft:cake",
        "minecraft:flower_pot" => "minecraft:flower_pot",
        "minecraft:cauldron" => "minecraft:cauldron",
        "minecraft:dragon_egg" => "minecraft:dragon_egg",
        "minecraft:tripwire" => "minecraft:tripwire",
        "minecraft:tripwire_hook" => "minecraft:tripwire_hook",
        "minecraft:hopper" => "minecraft:hopper",
        "minecraft:end_stone" => "minecraft:end_stone",
        "minecraft:vine" => "minecraft:vine",
        "minecraft:ladder" => "minecraft:ladder",
        "minecraft:netherrack" => "minecraft:netherrack",
        "minecraft:daylight_detector" => "minecraft:daylight_detector",
        "minecraft:lever" => "minecraft:lever",
        "minecraft:end_portal_frame" => "minecraft:end_portal_frame",
        // The era's names for 26.3 identities.
        "minecraft:grass_block" => "minecraft:grass",
        "minecraft:snow_block" => "minecraft:snow",
        "minecraft:snow" => "minecraft:snow_layer",
        "minecraft:sugar_cane" => "minecraft:reeds",
        "minecraft:melon" => "minecraft:melon_block",
        "minecraft:lily_pad" => "minecraft:waterlily",
        "minecraft:note_block" => "minecraft:noteblock",
        "minecraft:nether_portal" => "minecraft:portal",
        "minecraft:repeater" => "minecraft:unpowered_repeater",
        "minecraft:comparator" => "minecraft:unpowered_comparator",
        "minecraft:redstone_torch" => "minecraft:unlit_redstone_torch",
        "minecraft:jack_o_lantern" => "minecraft:lit_pumpkin",
        "minecraft:short_grass" => "minecraft:tallgrass",
        "minecraft:tall_grass" => "minecraft:double_plant",
        "minecraft:dead_bush" => "minecraft:deadbush",
        "minecraft:moving_piston" => "minecraft:piston_extension",
        // Stand-ins: the era has the block only as a metadata variant.
        "minecraft:redstone_wall_torch" => "minecraft:unlit_redstone_torch", // stand-in: a wall torch is redstone torch metadata 1..=4
        "minecraft:wet_sponge" => "minecraft:sponge", // stand-in: sponge metadata 1 is the wet sponge
        "minecraft:podzol" => "minecraft:dirt",       // stand-in: dirt metadata 2 is podzol
        "minecraft:coarse_dirt" => "minecraft:dirt",  // stand-in: dirt metadata 1 is coarse dirt
        "minecraft:carved_pumpkin" => "minecraft:pumpkin", // stand-in: pumpkin carries its facing in metadata
        "minecraft:water_cauldron" => "minecraft:cauldron", // stand-in: cauldron metadata is the water level
        "minecraft:fern" => "minecraft:tallgrass", // stand-in: tallgrass metadata 2 is a fern
        "minecraft:large_fern" => "minecraft:double_plant", // stand-in: double plant metadata 3 is a large fern
        "minecraft:big_dripleaf_fallback" => "minecraft:air", // stand-in: no big dripleaf
        // Stand-ins: the era has a close block under another name.
        "minecraft:basalt" => "minecraft:netherrack", // stand-in: no basalt until 1.16
        "minecraft:soul_soil" => "minecraft:soul_sand", // stand-in: soul soil is 1.16
        "minecraft:blue_ice" => "minecraft:packed_ice", // stand-in: no blue ice
        "minecraft:frosted_ice" => "minecraft:ice",   // stand-in: no frosted ice
        "minecraft:powder_snow" => "minecraft:snow_layer", // stand-in: no powder snow
        "minecraft:dirt_path" => "minecraft:dirt",    // stand-in: no path until 1.9
        "minecraft:mud" => "minecraft:dirt",          // stand-in: no mud
        "minecraft:soul_fire" => "minecraft:fire",    // stand-in: no soul fire
        "minecraft:magma_block" => "minecraft:netherrack", // stand-in: no magma block until 1.8
        "minecraft:moss_block" => "minecraft:mossy_cobblestone", // stand-in: no moss block
        "minecraft:chain_command_block" => "minecraft:command_block", // stand-in: no chain command block
        "minecraft:crimson_nylium" | "minecraft:warped_nylium" => "minecraft:netherrack", // stand-in: no nylium
        "minecraft:rooted_dirt" => "minecraft:dirt", // stand-in: no rooted dirt
        // Stand-ins: blocks 1.7.10 did not have. `air` keeps every rule about
        // them inert instead of matching an unrelated era block.
        "minecraft:cave_air" | "minecraft:void_air" => "minecraft:air", // stand-in: no cave or void air
        "minecraft:observer" => "minecraft:air", // stand-in: no observer until 1.11
        "minecraft:bubble_column"
        | "minecraft:kelp"
        | "minecraft:kelp_plant"
        | "minecraft:seagrass"
        | "minecraft:tall_seagrass" => "minecraft:air", // stand-in: no ocean plants
        "minecraft:cactus_flower" => "minecraft:air", // stand-in: no cactus flower
        "minecraft:bamboo" | "minecraft:bamboo_sapling" | "minecraft:scaffolding" => {
            "minecraft:air"
        } // stand-in: no bamboo or scaffolding
        "minecraft:barrier" => "minecraft:air",  // stand-in: no barrier
        "minecraft:lava_cauldron" | "minecraft:powder_snow_cauldron" => "minecraft:air", // stand-in: only a water cauldron
        "minecraft:slime_block" | "minecraft:honey_block" => "minecraft:air", // stand-in: no slime or honey block
        "minecraft:chorus_plant" | "minecraft:chorus_flower" => "minecraft:air", // stand-in: no chorus plants
        "minecraft:small_dripleaf"
        | "minecraft:big_dripleaf"
        | "minecraft:big_dripleaf_stem"
        | "minecraft:pointed_dripstone" => "minecraft:air", // stand-in: no dripleaf or dripstone
        "minecraft:sweet_berry_bush" => "minecraft:air", // stand-in: no sweet berries
        "minecraft:torchflower" | "minecraft:pitcher_plant" | "minecraft:pitcher_crop" => {
            "minecraft:air"
        } // stand-in: no torchflower or pitcher plant
        "minecraft:soul_campfire" | "minecraft:campfire" => "minecraft:air", // stand-in: no campfire until 1.14
        "minecraft:light" => "minecraft:air", // stand-in: no light block
        "minecraft:sculk_sensor"
        | "minecraft:calibrated_sculk_sensor"
        | "minecraft:sculk_shrieker"
        | "minecraft:sculk_catalyst"
        | "minecraft:sculk"
        | "minecraft:sculk_vein" => "minecraft:air", // stand-in: no sculk
        "minecraft:lectern" => "minecraft:air", // stand-in: no lectern until 1.14
        "minecraft:target" => "minecraft:air", // stand-in: no target
        "minecraft:beehive" | "minecraft:bee_nest" => "minecraft:air", // stand-in: no bees
        "minecraft:composter" => "minecraft:air", // stand-in: no composter
        "minecraft:respawn_anchor" => "minecraft:air", // stand-in: no respawn anchor
        "minecraft:crafter" | "minecraft:copper_bulb" => "minecraft:air", // stand-in: no crafter or copper bulb
        "minecraft:glow_lichen" => "minecraft:air", // stand-in: no glow lichen
        "minecraft:hanging_roots" | "minecraft:mangrove_roots" => "minecraft:air", // stand-in: no hanging or mangrove roots
        "minecraft:frogspawn"
        | "minecraft:turtle_egg"
        | "minecraft:sniffer_egg"
        | "minecraft:sea_pickle" => "minecraft:air", // stand-in: no frogspawn, eggs or sea pickle
        _ => return None,
    })
}

/// The era's Java class for a 1.7.10 class-chain entry.
pub(super) fn class_from_java(java: &str) -> Option<Class> {
    Class::from_java(java).or_else(|| {
        Some(match java {
            // Growth.
            "WheatBlock" => Class::Crop,
            "CarrotsBlock" => Class::Carrot,
            "PotatoesBlock" => Class::Potato,
            "DeadBushBlock" => Class::DryVegetation,
            "TallPlantBlock" => Class::TallGrass,
            "MushroomPlantBlock" => Class::Vegetation,
            // Redstone and the pistons.
            "RedstoneOreBlock" => Class::RedStoneOre,
            "FurnaceBlock" => Class::AbstractFurnace,
            "MovingBlock" => Class::MovingPiston,
            "AbstractRailBlock" => Class::BaseRail,
            "CobwebBlock" => Class::Web,
            "PortalBlock" => Class::NetherPortal,
            // Common blocks whose era classes differ in name only. The rest
            // fall back to `Block` through the first recognised chain entry.
            "LogBlock" | "Log2Block" | "AbstractLogBlock" | "AxisBlock" => Class::RotatedPillar,
            "LeavesBlock" | "Leaves2Block" | "AbstractLeavesBlock" => Class::Leaves,
            "StoneSlabBlock" | "WoodenSlabBlock" => Class::Slab,
            "StairsBlock" => Class::Stair,
            "PaneBlock" => Class::IronBars,
            "TrapdoorBlock" => Class::TrapDoor,
            _ => return None,
        })
    })
}

/// A typed property 1.7.10 stores in metadata.
struct Rule {
    name: &'static str,
    value: fn(i32) -> &'static str,
}

const AGE_8: [&str; 16] = [
    "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15",
];

fn age(meta: i32) -> &'static str {
    AGE_8[(meta.max(0) as usize) & 15]
}

fn age_4(meta: i32) -> &'static str {
    AGE_8[(meta.max(0) as usize) & 3]
}

fn power(meta: i32) -> &'static str {
    AGE_8[(meta.max(0) as usize) & 15]
}

fn horizontal_facing(meta: i32) -> &'static str {
    match meta & 3 {
        0 => "east",
        1 => "west",
        2 => "south",
        _ => "north",
    }
}

/// `StairsBlock` metadata bit 4.
fn stair_half(meta: i32) -> &'static str {
    if meta & 4 != 0 { "top" } else { "bottom" }
}

/// `SlabBlock` metadata bit 8.
fn slab_half(meta: i32) -> &'static str {
    if meta & 8 != 0 { "top" } else { "bottom" }
}

/// `AbstractLogBlock.getLogAxis`: metadata bits 2..=3.
fn log_axis(meta: i32) -> &'static str {
    match (meta >> 2) & 3 {
        1 => "x",
        2 => "z",
        _ => "y",
    }
}

fn sapling_stage(meta: i32) -> &'static str {
    if meta & 8 != 0 { "1" } else { "0" }
}

fn farmland_moisture(meta: i32) -> &'static str {
    AGE_8[(meta.max(0) as usize) & 7]
}

fn cauldron_level(meta: i32) -> &'static str {
    AGE_8[(meta.max(0) as usize) & 3]
}

fn snow_layers(meta: i32) -> &'static str {
    AGE_8[((meta.max(0) as usize) & 7) + 1]
}

fn end_portal_eye(meta: i32) -> &'static str {
    if meta & 4 != 0 { "true" } else { "false" }
}

/// `RedstoneWireBlock` metadata is the signal strength.
fn wire_power(meta: i32) -> &'static str {
    power(meta)
}

fn diode_facing(meta: i32) -> &'static str {
    horizontal_facing(meta)
}

/// `RepeaterBlock.getDelay`: metadata bits 2..=3 index a 2, 4, 6 or 8 tick delay.
fn repeater_delay(meta: i32) -> &'static str {
    AGE_8[((meta >> 2) & 3) as usize + 1]
}

/// A wall torch's metadata 1..=4 names the wall it sits on; 5 (and the
/// impossible 0) stand on the floor, which has no horizontal facing.
fn wall_torch_facing(meta: i32) -> &'static str {
    match meta & 7 {
        1 => "east",
        2 => "west",
        3 => "south",
        _ => "north",
    }
}

fn ladder_facing(meta: i32) -> &'static str {
    match meta {
        2 => "north",
        3 => "south",
        4 => "west",
        5 => "east",
        _ => "north",
    }
}

/// `PistonBaseBlock.getFacing`/`isExtended`.
fn piston_facing(meta: i32) -> &'static str {
    match meta & 7 {
        0 => "down",
        1 => "up",
        2 => "north",
        3 => "south",
        4 => "west",
        5 => "east",
        _ => "north",
    }
}

fn piston_extended(meta: i32) -> &'static str {
    if meta & 8 != 0 { "true" } else { "false" }
}

/// `PortalBlock` metadata 1 runs the portal along x, 2 along z.
fn portal_axis(meta: i32) -> &'static str {
    if meta & 3 == 2 { "z" } else { "x" }
}

fn always_false(_: i32) -> &'static str {
    "false"
}

fn always_true(_: i32) -> &'static str {
    "true"
}

fn pressure_plate_powered(meta: i32) -> &'static str {
    if meta & 15 != 0 { "true" } else { "false" }
}

/// A lever or button: metadata bit 3 is the pressed state.
fn pressed_bit(meta: i32) -> &'static str {
    if meta & 8 != 0 { "true" } else { "false" }
}

fn weighted_plate_power(meta: i32) -> &'static str {
    AGE_8[(meta.max(0) as usize) & 15]
}

const STAIRS: [Rule; 2] = [
    Rule {
        name: "facing",
        value: horizontal_facing,
    },
    Rule {
        name: "half",
        value: stair_half,
    },
];
const SLAB: [Rule; 1] = [Rule {
    name: "half",
    value: slab_half,
}];
const AXIS: [Rule; 1] = [Rule {
    name: "axis",
    value: log_axis,
}];
const PORTAL: [Rule; 1] = [Rule {
    name: "axis",
    value: portal_axis,
}];
const CROP: [Rule; 1] = [Rule {
    name: "age",
    value: age,
}];
const NETHER_WART: [Rule; 1] = [Rule {
    name: "age",
    value: age_4,
}];
const SAPLING: [Rule; 1] = [Rule {
    name: "stage",
    value: sapling_stage,
}];
const FARMLAND: [Rule; 1] = [Rule {
    name: "moisture",
    value: farmland_moisture,
}];
const CAULDRON: [Rule; 1] = [Rule {
    name: "level",
    value: cauldron_level,
}];
const SNOW: [Rule; 1] = [Rule {
    name: "layers",
    value: snow_layers,
}];
const END_PORTAL_FRAME: [Rule; 1] = [Rule {
    name: "eye",
    value: end_portal_eye,
}];
const WIRE: [Rule; 1] = [Rule {
    name: "power",
    value: wire_power,
}];
const REPEATER_PROPERTIES: [Rule; 3] = [
    Rule {
        name: "facing",
        value: diode_facing,
    },
    Rule {
        name: "delay",
        value: repeater_delay,
    },
    Rule {
        name: "powered",
        value: always_false,
    },
];
const REPEATER_POWERED: [Rule; 3] = [
    Rule {
        name: "facing",
        value: diode_facing,
    },
    Rule {
        name: "delay",
        value: repeater_delay,
    },
    Rule {
        name: "powered",
        value: always_true,
    },
];
const COMPARATOR: [Rule; 2] = [
    Rule {
        name: "facing",
        value: diode_facing,
    },
    Rule {
        name: "powered",
        value: always_false,
    },
];
const COMPARATOR_POWERED: [Rule; 2] = [
    Rule {
        name: "facing",
        value: diode_facing,
    },
    Rule {
        name: "powered",
        value: always_true,
    },
];
const TORCH_LIT: [Rule; 2] = [
    Rule {
        name: "lit",
        value: always_true,
    },
    Rule {
        name: "facing",
        value: wall_torch_facing,
    },
];
const TORCH_UNLIT: [Rule; 2] = [
    Rule {
        name: "lit",
        value: always_false,
    },
    Rule {
        name: "facing",
        value: wall_torch_facing,
    },
];
const LAMP: [Rule; 1] = [Rule {
    name: "lit",
    value: always_false,
}];
const LAMP_LIT: [Rule; 1] = [Rule {
    name: "lit",
    value: always_true,
}];
const PISTON: [Rule; 2] = [
    Rule {
        name: "facing",
        value: piston_facing,
    },
    Rule {
        name: "extended",
        value: piston_extended,
    },
];
const PISTON_HEAD: [Rule; 1] = [Rule {
    name: "facing",
    value: piston_facing,
}];
const LADDER: [Rule; 1] = [Rule {
    name: "facing",
    value: ladder_facing,
}];
const LEVER: [Rule; 1] = [Rule {
    name: "powered",
    value: pressed_bit,
}];
const BUTTON: [Rule; 1] = [Rule {
    name: "powered",
    value: pressed_bit,
}];
const PLATE: [Rule; 1] = [Rule {
    name: "powered",
    value: pressure_plate_powered,
}];
const WEIGHTED_PLATE: [Rule; 1] = [Rule {
    name: "power",
    value: weighted_plate_power,
}];

/// The metadata-backed properties of an era block, by its era name.
fn rules(short: &str) -> &'static [Rule] {
    if short.ends_with("_stairs") {
        &STAIRS
    } else {
        match short {
            "stone_slab" | "wooden_slab" => &SLAB,
            "log" | "log2" | "hay_block" => &AXIS,
            "portal" => &PORTAL,
            "wheat" | "carrots" | "potatoes" | "pumpkin_stem" | "melon_stem" => &CROP,
            "nether_wart" => &NETHER_WART,
            "sapling" => &SAPLING,
            "farmland" => &FARMLAND,
            "cauldron" => &CAULDRON,
            "snow_layer" => &SNOW,
            "end_portal_frame" => &END_PORTAL_FRAME,
            "redstone_wire" => &WIRE,
            "unpowered_repeater" => &REPEATER_PROPERTIES,
            "powered_repeater" => &REPEATER_POWERED,
            "unpowered_comparator" => &COMPARATOR,
            "powered_comparator" => &COMPARATOR_POWERED,
            "unlit_redstone_torch" => &TORCH_UNLIT,
            "redstone_torch" => &TORCH_LIT,
            "redstone_lamp" => &LAMP,
            "lit_redstone_lamp" => &LAMP_LIT,
            "piston" | "sticky_piston" => &PISTON,
            "piston_head" => &PISTON_HEAD,
            "ladder" => &LADDER,
            "lever" => &LEVER,
            "stone_button" | "wooden_button" => &BUTTON,
            "stone_pressure_plate" | "wooden_pressure_plate" => &PLATE,
            "light_weighted_pressure_plate" | "heavy_weighted_pressure_plate" => &WEIGHTED_PLATE,
            _ => &[],
        }
    }
}

/// Materialise the properties 1.7.10 stores in each block's metadata.
pub(super) fn install_properties(table: &mut StateTable) {
    for index in 0..table.block_count() {
        let block = BlockId(index as u16);
        let name = table.info(block).name.clone();
        let short = name.strip_prefix("minecraft:").unwrap_or(&name);
        for rule in rules(short) {
            table.install_derived_property(block, rule.name, rule.value);
        }
    }
}

/// The block tags the ported behaviour consults, answered from era block names.
/// A tag with no era concept is left empty; see the module docs.
pub(super) fn tag_contains(tag: &str, block: &str) -> bool {
    let tag = tag.strip_prefix("minecraft:").unwrap_or(tag);
    let block = block.strip_prefix("minecraft:").unwrap_or(block);
    match tag {
        "leaves" => matches!(block, "leaves" | "leaves2"),
        "logs" | "prevents_nearby_leaf_decay" => matches!(block, "log" | "log2"),
        "walls" => block == "cobblestone_wall",
        "fences" => matches!(block, "fence" | "nether_brick_fence"),
        "wooden_fences" => block == "fence",
        "fence_gates" => block == "fence_gate",
        "stairs" => block.ends_with("_stairs"),
        "slabs" => matches!(block, "stone_slab" | "wooden_slab"),
        "doors" => matches!(block, "wooden_door" | "iron_door"),
        "trapdoors" => block == "trapdoor",
        "rails" => matches!(
            block,
            "rail" | "golden_rail" | "detector_rail" | "activator_rail"
        ),
        "beds" => block == "bed",
        "wall_post_override" => matches!(
            block,
            "cobblestone_wall"
                | "fence"
                | "nether_brick_fence"
                | "glass_pane"
                | "stained_glass_pane"
                | "iron_bars"
        ),
        "replaceable" => matches!(
            block,
            "air"
                | "water"
                | "flowing_water"
                | "lava"
                | "flowing_lava"
                | "fire"
                | "sapling"
                | "tallgrass"
                | "deadbush"
                | "yellow_flower"
                | "red_flower"
                | "snow_layer"
                | "waterlily"
                | "vine"
        ),
        "dirt" => matches!(block, "dirt" | "grass" | "farmland"),
        "sand" => block == "sand",
        "ice" => matches!(block, "ice" | "packed_ice"),
        "snow" => matches!(block, "snow" | "snow_layer"),
        "soul_fire_base_blocks" => block == "soul_sand",
        "washed_away_by_fluids" => matches!(
            block,
            "sapling"
                | "tallgrass"
                | "deadbush"
                | "yellow_flower"
                | "red_flower"
                | "reeds"
                | "wheat"
                | "carrots"
                | "potatoes"
                | "pumpkin_stem"
                | "melon_stem"
                | "waterlily"
                | "snow_layer"
        ),
        "blocks_fluid_flow" => matches!(
            block,
            "waterlily" | "portal" | "end_portal" | "end_portal_frame"
        ),
        "supports_vegetation" => matches!(block, "grass" | "dirt" | "farmland"),
        "supports_crops" => block == "farmland",
        "supports_cactus" => block == "sand",
        "supports_sugar_cane" => matches!(block, "grass" | "dirt" | "sand"),
        "supports_cocoa" => matches!(block, "log" | "log2"),
        "supports_dry_vegetation" => matches!(block, "sand" | "dirt" | "hardened_clay"),
        "supports_nether_wart" => block == "soul_sand",
        "supports_chorus_plant" | "supports_chorus_flower" => block == "end_stone",
        "overrides_mushroom_light_requirement" | "mushroom_grow_block" => block == "mycelium",
        "convertible_to_mud" => matches!(block, "dirt" | "grass"),
        "grows_crops" => block == "farmland",
        "buttons" => matches!(block, "stone_button" | "wooden_button"),
        "pressure_plates" => matches!(
            block,
            "stone_pressure_plate"
                | "wooden_pressure_plate"
                | "light_weighted_pressure_plate"
                | "heavy_weighted_pressure_plate"
        ),
        "wool" => block == "wool",
        "crops" => matches!(block, "wheat" | "carrots" | "potatoes"),
        "small_flowers" | "flowers" => matches!(block, "yellow_flower" | "red_flower"),
        "saplings" => block == "sapling",
        "anvil" => block == "anvil",
        "climbable" => matches!(block, "ladder" | "vine"),
        _ => false,
    }
}

/// The era's fluid tags: water and lava, and every tag 26.3 defines over
/// water. The frogspawn, lily pad, sugar cane and bubble column tags are all
/// water in 1.7.10, which has no separate fluid tags of its own.
pub(super) fn fluid_tags() -> FluidTags {
    let mask = |kind: FluidKind| 1u8 << kind.vanilla_index().unwrap_or(0);
    let water = mask(FluidKind::FlowingWater) | mask(FluidKind::Water);
    let lava = mask(FluidKind::FlowingLava) | mask(FluidKind::Lava);
    FluidTags {
        water,
        lava,
        supports_frogspawn: water,
        supports_lily_pad: water,
        supports_sugar_cane_adjacently: water,
        bubble_column_can_occupy: water,
    }
}

/// Build the block-tag sets from era names, for every tag the port consults.
pub(super) fn tags(table: &StateTable) -> HashMap<String, Arc<BlockSet>> {
    let mut all = HashMap::new();
    for &name in super::data::Tags::NAMES {
        let mut set = BlockSet::with_capacity(table.block_count());
        for index in 0..table.block_count() {
            let block = BlockId(index as u16);
            if tag_contains(name, &table.info(block).name) {
                set.insert(block);
            }
        }
        all.insert(name.to_owned(), Arc::new(set));
    }
    all
}

/// The collision shape of an era state, from 1.7.10's `setShape` boxes.
/// States whose 1.7.10 shape depends on neighbours (stairs' inner corners,
/// connected fences) get their static part; see the module docs for the
/// blocks left with an empty shape.
pub(super) fn collision_shape(
    table: &StateTable,
    state: BlockStateId,
    collision_empty: bool,
    full_collision: bool,
) -> io::Result<VoxelShape> {
    if collision_empty {
        return Ok(VoxelShape::empty());
    }
    if full_collision {
        return Ok(VoxelShape::block());
    }
    let name = table.name(state);
    let short = name.strip_prefix("minecraft:").unwrap_or(name);
    let meta = table.key_of("meta").map_or(0, |key| table.int(state, key));
    match short {
        "stone_slab" | "wooden_slab" => box_16(0, 0, 0, 16, if meta & 8 != 0 { 16 } else { 8 }, 16),
        name if name.ends_with("_stairs") => {
            box_16(0, if meta & 4 != 0 { 8 } else { 0 }, 0, 16, 16, 16)
        }
        "farmland" => box_16(0, 0, 0, 16, 15, 16),
        "soul_sand" => box_16(0, 0, 0, 16, 14, 16),
        "cactus" => box_16(1, 0, 1, 15, 16, 15),
        "bed" => box_16(0, 0, 0, 16, 9, 16),
        "fence" | "cobblestone_wall" | "nether_brick_fence" => {
            aabb(6.0, 0.0, 6.0, 10.0, 24.0, 10.0)
        }
        "glass_pane" | "stained_glass_pane" | "iron_bars" => aabb(7.0, 0.0, 7.0, 9.0, 16.0, 9.0),
        "anvil" => box_16(2, 0, 2, 14, 16, 14),
        "brewing_stand" => box_16(7, 0, 7, 9, 14, 9),
        "cauldron" => box_16(0, 0, 0, 16, 5, 16),
        "chest" | "trapped_chest" | "ender_chest" => box_16(1, 0, 1, 15, 14, 15),
        "cocoa" => box_16(4, 5, 4, 12, 12, 12),
        "daylight_detector" => box_16(0, 0, 0, 16, 6, 16),
        "dragon_egg" => box_16(1, 0, 1, 15, 16, 15),
        "enchanting_table" => box_16(0, 0, 0, 16, 12, 16),
        "end_portal" => Ok(VoxelShape::empty()),
        "end_portal_frame" => box_16(0, 0, 0, 16, 13, 16),
        "flower_pot" => box_16(5, 0, 5, 11, 6, 11),
        "hopper" => aabb(0.0, 10.0, 0.0, 16.0, 16.0, 16.0),
        "cake" => box_16(1, 0, 1, 15, 8, 15),
        "piston_head" => Ok(VoxelShape::block()),
        "piston_extension" => Ok(VoxelShape::empty()),
        // 1.7.10's doors, trapdoors and gates change shape with their
        // metadata; a closed one blocks like the full block, an open one
        // lets the player through.
        "wooden_door" | "iron_door" => {
            if meta & 4 != 0 {
                Ok(VoxelShape::empty())
            } else {
                Ok(VoxelShape::block())
            }
        }
        "trapdoor" => {
            if meta & 4 != 0 {
                Ok(VoxelShape::empty())
            } else {
                Ok(VoxelShape::block())
            }
        }
        "fence_gate" => {
            if meta & 4 != 0 {
                Ok(VoxelShape::empty())
            } else {
                aabb(0.0, 0.0, 6.0, 16.0, 24.0, 10.0)
            }
        }
        // Flat or thin decorations 1.7.10 gives no collision box.
        "carpet"
        | "snow_layer"
        | "ladder"
        | "torch"
        | "redstone_torch"
        | "unlit_redstone_torch"
        | "rail"
        | "golden_rail"
        | "detector_rail"
        | "activator_rail"
        | "redstone_wire"
        | "tripwire"
        | "vine"
        | "waterlily"
        | "tallgrass"
        | "deadbush"
        | "yellow_flower"
        | "red_flower"
        | "sapling"
        | "wheat"
        | "carrots"
        | "potatoes"
        | "nether_wart"
        | "pumpkin_stem"
        | "melon_stem"
        | "reeds"
        | "double_plant" => Ok(VoxelShape::empty()),
        // Standing and wall signs are posts; 1.7.10 gives them a 0.6 to 0.9
        // tall box by metadata, approximated by the post height.
        "standing_sign" | "wall_sign" => box_16(0, 0, 0, 16, 10, 16),
        _ => Ok(VoxelShape::empty()),
    }
}

fn box_16(x0: i32, y0: i32, z0: i32, x1: i32, y1: i32, z1: i32) -> io::Result<VoxelShape> {
    let px = |value: i32| f64::from(value) / 16.0;
    aabb(px(x0), px(y0), px(z0), px(x1), px(y1), px(z1))
}

fn aabb(x0: f64, y0: f64, z0: f64, x1: f64, y1: f64, z1: f64) -> io::Result<VoxelShape> {
    VoxelShape::from_box(Aabb::new(Vec3::new(x0, y0, z0), Vec3::new(x1, y1, z1)))
        .map_err(|error| invalid(error.to_string()))
}
