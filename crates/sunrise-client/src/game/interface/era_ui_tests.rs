//! The era UI's wiring: the engine menu slot each of the era's slots stands
//! for. The painters' own layouts are pinned by their crate's tests; the
//! content root's profile picks the painter (`Game::era_ui`).
use super::era_ui::{era_engine_slot, era_slot_position};
use sunrise_client::screens_1_7_10::{Screen17, Slot17};

/// The engine's menu ids for the era's slot names, as `slots()` builds them:
/// hotbar 0..9, main rows 9..36, crafting 100.., result 200, a container's
/// 300.., armour 500..504.
#[test]
fn era_slots_address_the_engines_menu_slots() {
    let menu = Screen17::PlayerInventory.slots();
    let slot = |name: &str| -> &Slot17 {
        menu.iter()
            .find(|slot| slot.name == name)
            .unwrap_or_else(|| panic!("no era slot {name}"))
    };
    assert_eq!(era_engine_slot(slot("result")), Some(200));
    assert_eq!(era_engine_slot(slot("crafting_0")), Some(100));
    assert_eq!(era_engine_slot(slot("crafting_3")), Some(103));
    assert_eq!(era_engine_slot(slot("armor_head")), Some(500));
    assert_eq!(era_engine_slot(slot("armor_chest")), Some(501));
    assert_eq!(era_engine_slot(slot("armor_legs")), Some(502));
    assert_eq!(era_engine_slot(slot("armor_feet")), Some(503));
    assert_eq!(era_engine_slot(slot("inventory_0")), Some(9));
    assert_eq!(era_engine_slot(slot("inventory_26")), Some(35));
    assert_eq!(era_engine_slot(slot("hotbar_0")), Some(0));
    assert_eq!(era_engine_slot(slot("hotbar_8")), Some(8));

    let table = Screen17::CraftingTable.slots();
    let slot = |name: &str| table.iter().find(|slot| slot.name == name).unwrap();
    assert_eq!(era_engine_slot(slot("result")), Some(200));
    assert_eq!(era_engine_slot(slot("crafting_8")), Some(108));

    let furnace = Screen17::Furnace.slots();
    let slot = |name: &str| furnace.iter().find(|slot| slot.name == name).unwrap();
    assert_eq!(era_engine_slot(slot("input")), Some(300));
    assert_eq!(era_engine_slot(slot("fuel")), Some(301));
    assert_eq!(era_engine_slot(slot("output")), Some(302));

    // Every era slot has one engine slot and no two share it: a click on any
    // of the era's rectangles reaches that menu slot.
    for screen in [
        Screen17::PlayerInventory,
        Screen17::CraftingTable,
        Screen17::Furnace,
    ] {
        let slots = screen.slots();
        let mapped: Vec<usize> = slots.iter().filter_map(era_engine_slot).collect();
        assert_eq!(mapped.len(), slots.len(), "{screen:?} has an unmapped slot");
        let unique: std::collections::HashSet<usize> = mapped.iter().copied().collect();
        assert_eq!(
            unique.len(),
            mapped.len(),
            "{screen:?} maps two slots to one"
        );
    }
}

#[test]
fn era_hit_rectangles_are_the_painters_slot_table() {
    // InventoryMenuScreen's panel: the 2x2 grid at (88,26), the result at
    // (144,36), the armour at (8,8..62), the main rows at (8,84..120) and the
    // hotbar at (8,142).
    assert_eq!(
        era_slot_position(Screen17::PlayerInventory, 200),
        Some((144.0, 36.0))
    );
    assert_eq!(
        era_slot_position(Screen17::PlayerInventory, 100),
        Some((88.0, 26.0))
    );
    assert_eq!(
        era_slot_position(Screen17::PlayerInventory, 103),
        Some((106.0, 44.0))
    );
    assert_eq!(
        era_slot_position(Screen17::PlayerInventory, 500),
        Some((8.0, 8.0))
    );
    assert_eq!(
        era_slot_position(Screen17::PlayerInventory, 503),
        Some((8.0, 62.0))
    );
    assert_eq!(
        era_slot_position(Screen17::PlayerInventory, 9),
        Some((8.0, 84.0))
    );
    assert_eq!(
        era_slot_position(Screen17::PlayerInventory, 35),
        Some((152.0, 120.0))
    );
    assert_eq!(
        era_slot_position(Screen17::PlayerInventory, 0),
        Some((8.0, 142.0))
    );
    // The era inventory has no offhand slot.
    assert_eq!(era_slot_position(Screen17::PlayerInventory, 504), None);

    // CraftingTableMenu: the 3x3 grid at (30,17), the result at (124,35).
    assert_eq!(
        era_slot_position(Screen17::CraftingTable, 200),
        Some((124.0, 35.0))
    );
    assert_eq!(
        era_slot_position(Screen17::CraftingTable, 100),
        Some((30.0, 17.0))
    );
    assert_eq!(
        era_slot_position(Screen17::CraftingTable, 108),
        Some((66.0, 53.0))
    );

    // FurnaceMenu: input (56,17), fuel (56,53), output (116,35).
    assert_eq!(
        era_slot_position(Screen17::Furnace, 300),
        Some((56.0, 17.0))
    );
    assert_eq!(
        era_slot_position(Screen17::Furnace, 301),
        Some((56.0, 53.0))
    );
    assert_eq!(
        era_slot_position(Screen17::Furnace, 302),
        Some((116.0, 35.0))
    );
}
