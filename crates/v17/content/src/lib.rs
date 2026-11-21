//! The 1.7.10 content export Sunrise's engine loads.
//!
//! Sunrise reads its whole block and item content from a version-pinned export
//! (`facts.json`, `gameplay-facts.json`, `block-behaviours.json` and
//! `items.json`). This crate writes the 1.7.10 export in the same schema:
//!
//! * every 1.7.10 block id becomes the registry state
//!   `minecraft:<registry name>` with one `meta` property, using only the
//!   metadata values the block actually has where that is knowable and `0..=15`
//!   otherwise;
//! * rows are emitted in ascending `(numeric id, metadata)` order, and
//!   `facts.json`'s `defaults` maps each block name to the state id of its
//!   `meta = 0` row, so a saved `(id, meta)` resolves deterministically;
//! * the gameplay facts carry 1.7.10's hardness, blast resistance, light,
//!   opacity, slipperiness, material flags, sound type, item and map colour;
//! * the item table carries 1.7.10 stack sizes, durability and tool rules.
//!
//! # What the engine cannot express yet
//!
//! * `sunrise_items::blocks::BlockFacts` recognises only the 26.3 source names
//!   `minecraft:water` and `minecraft:lava` (and a `level` property) as fluids;
//!   the 1.7.10 flowing blocks and metadata levels are carried in
//!   `gameplay-facts.json`'s fluid row, which the world-side behaviour loader
//!   reads, but `BlockFacts.fluid` reports `Fluid::None` for
//!   `minecraft:flowing_water` and `minecraft:flowing_lava`.
//! * The 26.3 map-colour palette has no entry for 1.7.10's dirt, wood or spruce
//!   colours, so those load as `MapColor.NONE`.
//! * 1.7.10 metadata is carried as a `meta` property, not as 26.3 properties
//!   (`facing`, `half`, `waterlogged`, ...), so 26.3 class behaviours that read
//!   those properties see their defaults. The behaviours file carries the
//!   1.7.10 class chain and per-state legacy flags; translating metadata into
//!   typed properties is the port layer above this export.
//! * Collision and outline shapes (`cached-collision.nbt`, `outline`,
//!   `outline_edges`), block-entity types and fire odds are other prepared
//!   tables, not this content export.
//! * 1.7.10 item metadata and damage are not a table of their own: the item
//!   table carries item type facts, while a stack's variant is NBT in 1.7.10
//!   and a data component in 26.3.

mod blocks;

use serde_json::{Map, Value, json};
use std::{
    fs::{self, File},
    io::{self, BufWriter, Write},
    path::Path,
};
use sunset_1_7_blocks::{BLOCKS, BlockDef, MAX_ID, Material, ToolClass, by_id as block_by_id};
use sunset_1_7_items::{ItemDef, ItemKind, by_block, id_of, items as all_items};

/// Where the game looks for the export when no directory is given.
pub const DEFAULT_DIR: &str = ".local/sunrise-client/1.7.10";

/// What an [`export`] wrote.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Summary {
    /// Distinct block names (the registry's blocks).
    pub blocks: usize,
    /// Registry states across all blocks and metadata values.
    pub states: usize,
    /// Items in `items.json`.
    pub items: usize,
}

struct Row<'a> {
    def: &'a BlockDef,
    extra: &'a blocks::Extra,
    meta: u8,
    index: u32,
}

/// Writes the full 1.7.10 export into `dir`, creating it when needed.
pub fn export(dir: &Path) -> io::Result<Summary> {
    if BLOCKS.len() != blocks::EXTRAS.len() {
        return Err(io::Error::other(
            "the block extras table does not cover the block registry",
        ));
    }
    fs::create_dir_all(dir)?;

    let mut rows = Vec::new();
    let mut defaults = Map::new();
    for (def, extra) in BLOCKS.iter().zip(blocks::EXTRAS) {
        if def.id != extra.id {
            return Err(io::Error::other(format!(
                "block {} has id {}, its extras row {}",
                def.name, def.id, extra.id
            )));
        }
        let name = state_name(def);
        let mut default = None;
        for meta in blocks::metas(def.name) {
            let index = rows.len() as u32;
            if meta == 0 {
                default = Some(index);
            }
            rows.push(Row {
                def,
                extra,
                meta,
                index,
            });
        }
        let Some(default) = default else {
            return Err(io::Error::other(format!(
                "block {} has no metadata 0 state",
                def.name
            )));
        };
        defaults.insert(name, json!(default));
    }

    let (items, item_count) = item_table();
    write_json(&dir.join("facts.json"), &facts(&rows, defaults))?;
    write_json(&dir.join("gameplay-facts.json"), &gameplay(&rows))?;
    write_json(&dir.join("block-behaviours.json"), &behaviours(&rows))?;
    write_json(&dir.join("items.json"), &items)?;

    Ok(Summary {
        blocks: BLOCKS.len(),
        states: rows.len(),
        items: item_count,
    })
}

/// [`export`] into [`DEFAULT_DIR`].
pub fn export_default() -> io::Result<Summary> {
    export(Path::new(DEFAULT_DIR))
}

/// The registry state id [`export`] assigns to a 1.7.10 `(block id, metadata)`
/// pair, or `None` when the block or metadata value is not in the export.
///
/// The export writes rows in ascending `(id, meta)` order, so this is the
/// deterministic mapping a chunk converter uses to turn a saved id and metadata
/// into a state id. The table is built once.
pub fn state_id(block_id: u16, meta: u8) -> Option<u32> {
    static IDS: std::sync::OnceLock<Vec<Option<u32>>> = std::sync::OnceLock::new();
    if meta > 15 || block_id > MAX_ID {
        return None;
    }
    let ids = IDS.get_or_init(|| {
        let mut ids = vec![None; (usize::from(MAX_ID) + 1) * 16];
        let mut next = 0u32;
        for (def, _) in BLOCKS.iter().zip(blocks::EXTRAS) {
            for value in blocks::metas(def.name) {
                ids[usize::from(def.id) * 16 + usize::from(value)] = Some(next);
                next += 1;
            }
        }
        ids
    });
    ids[usize::from(block_id) * 16 + usize::from(meta)]
}

fn state_name(def: &BlockDef) -> String {
    format!("minecraft:{}", def.name)
}

fn item_name(def: &BlockDef) -> String {
    let plain = match def.name {
        "air" | "piston_extension" => return "minecraft:air".to_owned(),
        "bed" => "bed",
        "brewing_stand" => "brewing_stand",
        "cauldron" => "cauldron",
        "flower_pot" => "flower_pot",
        "skull" => "skull",
        "standing_sign" | "wall_sign" => "sign",
        "wooden_door" => "wooden_door",
        "iron_door" => "iron_door",
        "unpowered_repeater" | "powered_repeater" => "repeater",
        "unpowered_comparator" | "powered_comparator" => "comparator",
        "unlit_redstone_torch" | "redstone_torch" => "redstone_torch",
        "tripwire" => "string",
        "reeds" => "reeds",
        "cake" => "cake",
        "nether_wart" => "nether_wart",
        "wheat" => "wheat_seeds",
        "carrots" => "carrot",
        "potatoes" => "potato",
        "pumpkin_stem" => "pumpkin_seeds",
        "melon_stem" => "melon_seeds",
        "lit_furnace" => "furnace",
        "lit_redstone_lamp" => "redstone_lamp",
        "lit_redstone_ore" => "redstone_ore",
        "piston_head" => "piston",
        _ => {
            return by_block(def.id as i32).map_or_else(
                || "minecraft:air".to_owned(),
                |item| format!("minecraft:{}", item.name),
            );
        }
    };
    if id_of(plain).is_some() {
        format!("minecraft:{plain}")
    } else {
        "minecraft:air".to_owned()
    }
}

/// `facts.json`: `[name, air, fluid, description, state]` rows and the default
/// state of every block name.
fn facts(rows: &[Row], defaults: Map<String, Value>) -> Value {
    let states: Vec<Value> = rows
        .iter()
        .map(|row| {
            let name = state_name(row.def);
            let description = format!(
                "Block{{minecraft:{}, id: {}, meta: {}}}",
                row.def.name, row.def.id, row.meta
            );
            let state = json!({
                "id": name,
                "properties": {"meta": row.meta.to_string()},
            });
            json!([
                name,
                row.def.id == 0,
                row.def.material.is_liquid(),
                description,
                state
            ])
        })
        .collect();
    json!({"states": states, "defaults": defaults})
}

/// `gameplay-facts.json`: the per-state facts the gameplay loaders read.
fn gameplay(rows: &[Row]) -> Value {
    let states: Vec<Value> = rows.iter().map(gameplay_row).collect();
    json!({"states": states})
}

fn gameplay_row(row: &Row) -> Value {
    let def = row.def;
    let shape = blocks::shape(def);
    let cube = shape == blocks::Shape::Cube;
    let opaque_cube = cube && def.opacity == 255;
    let solid_render = opaque_cube && def.material.is_solid_blocking();
    let item = item_name(def);
    let (sound_key, volume, pitch) = blocks::sound(def, row.extra);
    let [break_sound, step, place, hit, fall] = blocks::sound_ids(sound_key);
    json!({
        "id": row.index,
        "emission": def.light,
        "dynamic_light": false,
        "dampening": if def.opacity > 15 { 15 } else { def.opacity },
        "skylight_down": def.opacity == 0,
        "shape_occlusion": solid_render,
        "can_occlude": solid_render,
        "solid_render": solid_render,
        "hardness": def.hardness,
        "requires_tool": def.material.requires_tool(),
        "replaceable": def.material.is_replaceable(),
        "random_ticks": blocks::random_ticks(row.extra),
        "block_entity": blocks::block_entity(row.extra),
        "liquid": def.material.is_liquid(),
        "ignited_by_lava": def.material.is_flammable() || def.material == Material::Tnt,
        "push": blocks::push(def),
        "render_shape": if shape == blocks::Shape::Air { "INVISIBLE" } else { "MODEL" },
        "no_destroy_overlay": false,
        "full_collision": cube && (def.material.blocks_movement() || def.name == "web"),
        "friction": def.slipperiness,
        "explosion_resistance": def.blast_resistance(),
        "item": item,
        "pick_item": item,
        "sound": {
            "volume": volume,
            "pitch": pitch,
            "break": {"id": break_sound},
            "step": {"id": step},
            "place": {"id": place},
            "hit": {"id": hit},
            "fall": {"id": fall},
        },
        "offset": false,
        "outline_offset": false,
        "map_color": blocks::map_color(def, row.meta),
    })
}

/// `block-behaviours.json`: the class chain per block, the per-state legacy
/// flags, the fluid registry and the fire odds.
fn behaviours(rows: &[Row]) -> Value {
    let blocks: Vec<Value> = BLOCKS
        .iter()
        .zip(blocks::EXTRAS)
        .map(|(def, extra)| {
            json!({
                "name": state_name(def),
                "class": extra.class,
                "fields": {},
            })
        })
        .collect();
    let states: Vec<Value> = rows
        .iter()
        .map(|row| {
            let def = row.def;
            let shape = blocks::shape(def);
            let cube = shape == blocks::Shape::Cube;
            let opaque_cube = cube && def.opacity == 255;
            let signal = blocks::is_signal_source(def.name);
            let mut flags = 0u32;
            if cube && def.material.is_solid_blocking() && !signal {
                flags |= 1;
            }
            if def.id == 0 {
                flags |= 1 << 1;
            }
            if opaque_cube {
                flags |= 1 << 2;
            }
            if signal {
                flags |= 1 << 3;
            }
            if blocks::is_analog_output(def.name) {
                flags |= 1 << 4;
            }
            if blocks::collision_empty(def) {
                flags |= 1 << 5;
            }
            if opaque_cube && def.material.is_solid_blocking() {
                flags |= 1 << 6;
            }
            let (fluid, level, falling) = blocks::fluid(def, row.meta);
            json!([
                blocks::sturdy(def),
                flags,
                fluid,
                level,
                falling,
                0,
                0,
                0,
                "HARP"
            ])
        })
        .collect();
    let items: Map<String, Value> = all_items()
        .filter_map(|item| {
            let block = u16::try_from(item.block?).ok()?;
            let block = block_by_id(block)?;
            Some((
                format!("minecraft:{}", item.name),
                json!({
                    "class": ["BlockItem"],
                    "fields": {"block": format!("minecraft:{}", block.name)},
                }),
            ))
        })
        .collect();
    json!({
        "blocks": blocks,
        "states": states,
        "items": items,
        "fluids": [
            "minecraft:empty",
            "minecraft:flowing_water",
            "minecraft:water",
            "minecraft:flowing_lava",
            "minecraft:lava",
        ],
        "fire": {},
    })
}

/// `items.json`: 1.7.10 stack sizes, durability and tool facts.
fn item_table() -> (Value, usize) {
    let mut items = Map::new();
    let mut count = 0;
    for item in all_items() {
        let mut components = Map::new();
        components.insert(
            "minecraft:max_stack_size".to_owned(),
            json!(item.max_stack_size),
        );
        if item.max_damage > 0 {
            components.insert("minecraft:max_damage".to_owned(), json!(item.max_damage));
        }
        if let Some(tool) = tool_component(item) {
            components.insert("minecraft:tool".to_owned(), tool);
        }
        items.insert(
            format!("minecraft:{}", item.name),
            Value::Object(components),
        );
        count += 1;
    }
    (Value::Object(items), count)
}

fn tool_component(item: &ItemDef) -> Option<Value> {
    match item.kind {
        ItemKind::Tool { class, tier, .. } => {
            let mut rules = Vec::new();
            let class = block_tool(class);
            let effective = blocks_with_tool(class);
            if !effective.is_empty() {
                rules.push(rule(effective, Some(tier.mining_speed()), None));
            }
            let harvestable = blocks_harvestable_by(class, u8::try_from(tier.level()).unwrap_or(0));
            if !harvestable.is_empty() {
                rules.push(rule(harvestable, None, Some(true)));
            }
            Some(tool_json(rules, 1))
        }
        ItemKind::Sword { .. } => {
            let mut rules = vec![rule(vec!["minecraft:web".to_owned()], Some(15.0), None)];
            let plants: Vec<String> = BLOCKS
                .iter()
                .filter(|block| {
                    matches!(
                        block.material,
                        Material::Plant
                            | Material::ReplaceablePlant
                            | Material::Leaves
                            | Material::Pumpkin
                    )
                })
                .map(block_name)
                .collect();
            if !plants.is_empty() {
                rules.push(rule(plants, Some(1.5), None));
            }
            rules.push(rule(vec!["minecraft:web".to_owned()], None, Some(true)));
            Some(tool_json(rules, 2))
        }
        ItemKind::Shears => {
            let rules = vec![
                rule(
                    vec![
                        "minecraft:web".to_owned(),
                        "minecraft:leaves".to_owned(),
                        "minecraft:leaves2".to_owned(),
                    ],
                    Some(15.0),
                    None,
                ),
                rule(vec!["minecraft:wool".to_owned()], Some(5.0), None),
                rule(
                    vec![
                        "minecraft:web".to_owned(),
                        "minecraft:redstone_wire".to_owned(),
                        "minecraft:tripwire".to_owned(),
                    ],
                    None,
                    Some(true),
                ),
            ];
            Some(tool_json(rules, 1))
        }
        _ => None,
    }
}

fn block_tool(class: sunset_1_7_items::ToolClass) -> ToolClass {
    match class {
        sunset_1_7_items::ToolClass::Pickaxe => ToolClass::Pickaxe,
        sunset_1_7_items::ToolClass::Shovel => ToolClass::Shovel,
        sunset_1_7_items::ToolClass::Axe => ToolClass::Axe,
    }
}

fn blocks_with_tool(tool: ToolClass) -> Vec<String> {
    BLOCKS
        .iter()
        .filter(|block| block.tool == Some(tool))
        .map(block_name)
        .collect()
}

fn blocks_harvestable_by(tool: ToolClass, level: u8) -> Vec<String> {
    BLOCKS
        .iter()
        .filter(|block| {
            block.tool == Some(tool)
                && block.material.requires_tool()
                && block.harvest_level <= level
        })
        .map(block_name)
        .collect()
}

fn block_name(block: &BlockDef) -> String {
    format!("minecraft:{}", block.name)
}

fn rule(blocks: Vec<String>, speed: Option<f32>, correct: Option<bool>) -> Value {
    let mut rule = Map::new();
    rule.insert("blocks".to_owned(), json!(blocks));
    if let Some(speed) = speed {
        rule.insert("speed".to_owned(), json!(speed));
    }
    if let Some(correct) = correct {
        rule.insert("correct_for_drops".to_owned(), json!(correct));
    }
    Value::Object(rule)
}

fn tool_json(rules: Vec<Value>, damage_per_block: u32) -> Value {
    json!({
        "rules": rules,
        "default_mining_speed": 1.0,
        "damage_per_block": damage_per_block,
    })
}

fn write_json(path: &Path, value: &Value) -> io::Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer(&mut writer, value).map_err(io::Error::other)?;
    writer.write_all(b"\n")?;
    writer.flush()
}
