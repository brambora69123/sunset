//! Loads the written 1.7.10 export back through the engine's own parsers.

use std::{fs, path::PathBuf};
use sunrise_items::{
    blocks::{BlockFacts, Fluid},
    items::{ItemStack, Items},
};
use sunrise_world::{loot::PackOverlay, registry::BlockRegistry, registry::BlockStateId};
use sunset_1_7_content::{export, state_id};

fn fixture() -> (PathBuf, usize) {
    let dir = std::env::temp_dir().join(format!(
        "sunset-1-7-content-{}-{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("load")
    ));
    let _ = fs::remove_dir_all(&dir);
    let summary = export(&dir).unwrap();
    (dir, summary.states)
}

fn stack(item: &str) -> ItemStack {
    ItemStack {
        item: item.into(),
        count: 1,
        damage: 0,
        extra: None,
    }
}

fn property_values(registry: &BlockRegistry, block: &str, property: &str) -> Vec<String> {
    (0..registry.len() as u32)
        .filter_map(|id| registry.state(BlockStateId(id)))
        .filter(|state| state.name == block)
        .map(|state| state.properties.get(property).cloned().unwrap_or_default())
        .collect()
}

#[test]
fn loads_through_the_engine_parsers() {
    let (dir, state_count) = fixture();
    let (registry, fluid_flags) =
        BlockRegistry::from_facts_json_with_flags(fs::File::open(dir.join("facts.json")).unwrap())
            .unwrap();
    assert_eq!(registry.len(), state_count);
    let facts = BlockFacts::load(&dir, &registry).unwrap();
    assert_eq!(facts.states.len(), registry.len());
    let defaults = sunrise_world::item_defaults::read(&dir).unwrap();
    let items =
        Items::load_registry(&dir, &registry, &facts, &PackOverlay::NONE, &defaults).unwrap();

    let default_state = |name: &str| {
        registry
            .default_state(name)
            .unwrap_or_else(|| panic!("{name} is missing from the registry"))
    };
    let state = |name: &str| facts.get(default_state(name));

    // Every block's default row is the (id, 0) pair the export order names.
    for def in sunset_1_7_blocks::BLOCKS {
        let name = format!("minecraft:{}", def.name);
        assert_eq!(
            state_id(def.id, 0),
            Some(default_state(&name).0),
            "{}",
            def.name
        );
    }
    assert_eq!(state_id(1, 15), None, "stone has no metadata 15");

    // Stone's 1.7.10 strength: mining time 1.5, internal blast resistance 30
    // (the public value is 30 / 5).
    let stone = state("minecraft:stone");
    assert_eq!(stone.hardness, 1.5);
    assert_eq!(stone.explosion_resistance, 6.0);
    assert!(stone.requires_tool);

    // Obsidian takes 50 seconds and 6000 internal resistance; only a diamond
    // pickaxe may harvest it.
    let obsidian = state("minecraft:obsidian");
    assert_eq!(obsidian.hardness, 50.0);
    assert_eq!(obsidian.explosion_resistance, 1200.0);
    assert!(obsidian.requires_tool);
    let diamond_pickaxe = stack("minecraft:diamond_pickaxe");
    let iron_pickaxe = stack("minecraft:iron_pickaxe");
    assert!(items.correct_for_drops(Some(&diamond_pickaxe), "minecraft:obsidian"));
    assert!(!items.correct_for_drops(Some(&iron_pickaxe), "minecraft:obsidian"));
    assert!(!items.correct_for_drops(None, "minecraft:obsidian"));

    // The four blocks the port's chunk pipeline resolves by numeric id.
    for (name, id) in [("grass", 2), ("dirt", 3), ("sand", 12), ("gravel", 13)] {
        let block = sunset_1_7_blocks::by_name(name).unwrap();
        assert_eq!(block.id, id, "{name}");
        let registry_name = format!("minecraft:{name}");
        let state_id = default_state(&registry_name);
        assert_eq!(facts.get(state_id).name.as_ref(), registry_name);
        assert_eq!(
            registry
                .state(state_id)
                .unwrap()
                .properties
                .get("meta")
                .map(String::as_str),
            Some("0")
        );
    }

    // facts.json's fluid column covers the flowing blocks too, and the client
    // loader reads the level from the 1.7.10 metadata.
    for name in ["water", "flowing_water", "lava", "flowing_lava"] {
        let id = default_state(&format!("minecraft:{name}"));
        assert_eq!(fluid_flags[id.0 as usize], (false, true), "{name}");
    }
    assert_eq!(state("minecraft:water").fluid, Fluid::Water);
    assert_eq!(state("minecraft:lava").fluid, Fluid::Lava);
    assert_eq!(state("minecraft:flowing_water").fluid, Fluid::Water);
    assert_eq!(state("minecraft:flowing_lava").fluid, Fluid::Lava);
    // Metadata 1..7 is the distance from the source; 8 and above is falling.
    for (meta, level, falling) in [(1u8, 7u8, false), (7, 1, false), (8, 8, true), (9, 8, true)] {
        let id = state_id(10, meta).unwrap_or_else(|| panic!("flowing lava meta {meta}"));
        let lava = facts.get(BlockStateId(id));
        assert_eq!(lava.fluid, Fluid::Lava, "meta {meta}");
        assert_eq!(
            (lava.fluid_level, lava.fluid_falling),
            (level, falling),
            "meta {meta}"
        );
    }

    assert_eq!(state("minecraft:torch").emission, 14);
    assert_eq!(state("minecraft:glowstone").emission, 15);

    let wool_metas = property_values(&registry, "minecraft:wool", "meta");
    assert_eq!(
        wool_metas,
        (0..16).map(|meta| meta.to_string()).collect::<Vec<_>>()
    );

    // Log metadata: wood type is the low two bits, axis bits 2..=3.
    let log_metas = property_values(&registry, "minecraft:log", "meta");
    assert_eq!(
        log_metas,
        (0..12).map(|meta| meta.to_string()).collect::<Vec<_>>()
    );
    assert!(log_metas.contains(&"4".to_owned()), "axis x");
    assert!(log_metas.contains(&"8".to_owned()), "axis z");
    let log_axis_x = (0..registry.len() as u32)
        .find(|id| {
            registry.state(BlockStateId(*id)).is_some_and(|state| {
                state.name == "minecraft:log"
                    && state.properties.get("meta").map(String::as_str) == Some("4")
            })
        })
        .expect("log axis x");
    assert_eq!(state_id(17, 4), Some(log_axis_x));
    assert_eq!(
        property_values(&registry, "minecraft:log2", "meta"),
        ["0", "1", "4", "5", "8", "9"].map(str::to_owned).to_vec()
    );

    // Item facts: 1.7.10 stack sizes, durability and tool rules.
    let pickaxe = items.get("minecraft:diamond_pickaxe").unwrap();
    assert_eq!(pickaxe.max_stack, 1);
    assert_eq!(pickaxe.max_damage, 1561);
    assert_eq!(
        items.mining_speed(Some(&diamond_pickaxe), "minecraft:stone"),
        8.0
    );
    assert_eq!(
        items.mining_speed(Some(&stack("minecraft:wooden_pickaxe")), "minecraft:stone"),
        2.0
    );
    assert!(items.correct_for_drops(Some(&diamond_pickaxe), "minecraft:stone"));
    assert_eq!(items.get("minecraft:stick").unwrap().max_stack, 64);
    assert_eq!(items.get("minecraft:bucket").unwrap().max_stack, 16);
    assert_eq!(items.get("minecraft:ender_pearl").unwrap().max_stack, 16);
    assert_eq!(items.get("minecraft:sign").unwrap().max_stack, 16);
    assert_eq!(items.get("minecraft:mushroom_stew").unwrap().max_stack, 1);
    assert_eq!(
        items.get("minecraft:wool").unwrap().block,
        Some(default_state("minecraft:wool"))
    );
    // The item that places a block 1.7.10 registers without an `ItemBlock`.
    assert_eq!(
        items.get("minecraft:bed").unwrap().block,
        Some(default_state("minecraft:bed"))
    );
    assert_eq!(
        items.get("minecraft:carrots").unwrap().block,
        Some(default_state("minecraft:carrots"))
    );

    // The air row is the engine's fallback state.
    assert_eq!(facts.get(facts.air).name.as_ref(), "minecraft:air");

    fs::remove_dir_all(dir).unwrap();
}
