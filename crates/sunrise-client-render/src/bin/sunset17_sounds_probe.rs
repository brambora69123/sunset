//! Prints what a 1.7.10 content root resolves for a block's break, place and step sounds: the
//! engine id its block facts name, the era event [`sound_1_7_10`] maps that to, the files the era's
//! `sounds.json` gives it and the file a draw picks.
//!
//! Usage: `sunset17_sounds_probe [ROOT]` (`SUNSET_1_7_CONTENT` or
//! `.local/sunrise-client/1.7.10` by default).

use serde_json::Value;
use std::path::PathBuf;
use sunrise_client_render::{
    resource_source::ResourceSource, sound_1_7_10, sound_events::SoundRegistry,
};
use sunrise_core::random::LegacyRandom;

/// The block states the printed cases read: the item name the facts give the block's metadata-0
/// state, `Block.init`'s name for it, the `Block$Sounds` constant it sets and the engine family
/// its facts use.
const CASES: &[(&str, &str, &str, &str)] = &[
    ("minecraft:stone", "stone", "STONE_SOUNDS", "stone"),
    ("minecraft:grass", "grass", "GRASS_SOUNDS", "grass"),
    ("minecraft:sand", "sand", "SAND_SOUNDS", "sand"),
    ("minecraft:gravel", "gravel", "GRAVEL_SOUNDS", "gravel"),
    ("minecraft:planks", "planks", "WOOD_SOUNDS", "wood"),
    ("minecraft:wool", "wool", "CLOTH_SOUNDS", "wool"),
    ("minecraft:glass", "glass", "GLASS_SOUNDS", "glass"),
    ("minecraft:torch", "torch", "WOOD_SOUNDS", "wood"),
    ("minecraft:ladder", "ladder", "LADDER_SOUNDS", "ladder"),
    ("minecraft:anvil", "anvil", "ANVIL_SOUNDS", "anvil"),
];

/// The era event each case's action must ask for, pinned from `Block$Sounds` and its three
/// subclasses: `getBreaking`, `getPlacing` (with the overrides) and `getStepping`.
fn expected(block: &str, action: &str) -> &'static str {
    match (block, action) {
        ("stone", "break" | "place") => "dig.stone",
        ("grass", "break" | "place") => "dig.grass",
        ("sand", "break" | "place") => "dig.sand",
        ("gravel", "break" | "place") => "dig.gravel",
        ("planks" | "torch", "break" | "place") => "dig.wood",
        ("wool", "break" | "place") => "dig.cloth",
        ("stone", "step") => "step.stone",
        ("grass", "step") => "step.grass",
        ("sand", "step") => "step.sand",
        ("gravel", "step") => "step.gravel",
        ("planks" | "torch", "step") => "step.wood",
        ("wool", "step") => "step.cloth",
        ("glass", "break") => "dig.glass",
        ("glass", "place" | "step") => "step.stone",
        ("ladder", "break" | "place") => "dig.wood",
        ("ladder", "step") => "step.ladder",
        ("anvil", "break") => "dig.stone",
        ("anvil", "place") => "random.anvil_land",
        ("anvil", "step") => "step.anvil",
        _ => panic!("no pinned expectation for {block}'s {action}"),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("SUNSET_1_7_CONTENT").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from(sunrise_client_render::sunset17_assets::DEFAULT_DIR));
    let facts: Value = serde_json::from_slice(&std::fs::read(root.join("gameplay-facts.json"))?)?;
    let states = facts["states"]
        .as_array()
        .ok_or("gameplay-facts.json has no states")?;
    // The metadata-0 state of the block whose item this names: `Block.init`'s id's first state.
    let state = |item: &str| -> Result<&Value, String> {
        states
            .iter()
            .find(|state| state["item"].as_str() == Some(item))
            .ok_or_else(|| format!("gameplay-facts.json has no state for {item}"))
    };

    let registry = SoundRegistry::load(&ResourceSource::directory(root.clone()));
    println!(
        "1.7.10 sounds from {}: {} sound events",
        root.display(),
        registry.names().count()
    );

    println!();
    println!("Block$Sounds table (sound_1_7_10::block_events):");
    let mut families: Vec<String> = Vec::new();
    for (engine, _) in sound_1_7_10::block_events() {
        let family = engine
            .strip_prefix("minecraft:block.")
            .and_then(|rest| rest.rsplit_once('.'))
            .map_or("", |(family, _)| family);
        if families.iter().any(|seen| seen == family) {
            continue;
        }
        families.push(family.to_owned());
        let mut row = format!("  {family:<7}");
        for cause in ["break", "place", "step", "hit", "fall", "land"] {
            let engine = format!("minecraft:block.{family}.{cause}");
            let era = sound_1_7_10::era_event(&engine).unwrap_or_else(|| "none".into());
            row += &format!(" {cause}={era}");
        }
        println!("{row}");
    }

    println!();
    println!("Block cases (block facts -> era event -> file):");
    let mut failures = Vec::new();
    for &(item, label, sounds, family) in CASES {
        let state = state(item).map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
        println!(
            "  {label} ({item}, {sounds}, family {family}, volume {:.2}, pitch {:.2}):",
            state["sound"]["volume"].as_f64().unwrap_or(0.0),
            state["sound"]["pitch"].as_f64().unwrap_or(0.0)
        );
        for action in ["break", "place", "step"] {
            let engine = state["sound"][action]["id"]
                .as_str()
                .unwrap_or_else(|| panic!("{label} has no {action} sound"));
            let want = format!("minecraft:{}", expected(label, action));
            let era = sound_1_7_10::era_event(engine);
            let resolved = era.as_deref() == Some(want.as_str());
            let mut files = era
                .as_deref()
                .map(|era| registry.tracks(era))
                .unwrap_or_default();
            files.sort();
            let chosen = era.as_deref().and_then(|era| {
                let mut random = LegacyRandom::new(7);
                registry.sound(era, &mut random).map(|sound| sound.path().1)
            });
            println!(
                "    {action:<5} {engine:<30} -> {:<20} {:<8} chosen {:<24} files {}",
                era.as_deref().unwrap_or("none"),
                if resolved { "PINNED" } else { "MISMATCH" },
                chosen.as_deref().unwrap_or("none"),
                files.len()
            );
            if !resolved {
                failures.push(format!(
                    "{label} {action}: {engine} -> {era:?}, pinned {want}"
                ));
            }
        }
    }

    // The block facts' ids are what the game asks for: every one the engine can name must have
    // a rule (the anvil's step names `step.anvil`, which the era never defines, so it plays
    // nothing — the mapping and the registry are separate).
    let mut unmapped: Vec<String> = Vec::new();
    let mut mapped = 0;
    for state in states {
        let Some(sound) = state["sound"].as_object() else {
            continue;
        };
        for event in sound.values() {
            let Some(id) = event["id"].as_str() else {
                continue;
            };
            if id.is_empty() {
                continue;
            }
            match sound_1_7_10::era_event(id) {
                Some(_) => mapped += 1,
                None if !unmapped.iter().any(|seen| seen == id) => unmapped.push(id.to_owned()),
                None => {}
            }
        }
    }
    let era_events = std::fs::read(root.join("assets/minecraft/sounds.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .and_then(|sounds| sounds.as_object().map(serde_json::Map::len))
        .unwrap_or(0);
    println!();
    println!("Block facts: {mapped} sound entries mapped, no rule for {unmapped:?}");
    println!(
        "Sound events the root registers: {} ({era_events} from the era's sounds.json, the rest \
         engine ids the table adds; the era's own names play when the engine asks for them — \
         mobs, weather and the rest of 26.3's ids stay silent until named)",
        registry.names().count()
    );
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("\n").into())
    }
}
