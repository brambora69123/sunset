//! Deterministic inventory probe: prints a fixed run of the 1.7.10 inventory
//! mechanics for later parity checks. No RNG is drawn (none of this run needs
//! it), so the output is stable.
//!
//! Run: cargo +1.99.1 run -p sunset-1-7-inventory --bin inventory-probe

use std::cell::RefCell;
use std::rc::Rc;
use sunset_1_7_inventory::inventory::{CraftingInventory, Inventory};
use sunset_1_7_inventory::item_stack::ItemStack;
use sunset_1_7_inventory::menu::{Menu, PlayerCtx, action};
use sunset_1_7_inventory::recipes::vanilla_crafting_manager;
use sunset_1_7_inventory::registry::{ItemFacts, VanillaFacts, ids};
use sunset_1_7_inventory::smelting::SmeltingManager;

fn stack(id: u16, size: i32) -> Option<ItemStack> {
    Some(ItemStack::new(id, size, 0))
}

fn grid_line(grid: &CraftingInventory, row: usize, width: usize) -> String {
    (0..width)
        .map(|column| match grid.get(column, row) {
            Some(stack) => format!("{:>4}", stack.id),
            None => format!("{:>4}", "."),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn main() {
    let f = VanillaFacts;
    let inventory = Rc::new(RefCell::new(
        sunset_1_7_inventory::inventory::PlayerInventory::new(),
    ));
    let mut ctx = PlayerCtx::new(inventory, false);

    // 1. Fuel table.
    println!("== fuel ticks ==");
    for id in [
        ids::STICK,
        ids::COAL,
        ids::LAVA_BUCKET,
        ids::BLAZE_ROD,
        ids::SAPLING,
        ids::PLANKS,
        ids::LOG,
        ids::WOODEN_SLAB,
        ids::COAL_BLOCK,
        ids::WOODEN_PICKAXE,
        ids::DIRT,
    ] {
        println!(
            "{:>16} {}",
            id,
            sunset_1_7_inventory::furnace::vanilla_fuel_time(&ItemStack::of(id))
        );
    }

    // 2. Stack facts.
    println!("== stack facts ==");
    println!(
        "iron_pickaxe max_size={} max_damage={}",
        ItemStack::of(ids::IRON_PICKAXE).max_size(&f),
        f.max_damage(ids::IRON_PICKAXE)
    );
    println!(
        "iron_chestplate max_damage={}",
        f.max_damage(ids::IRON_CHESTPLATE)
    );
    println!(
        "bucket max_size={}",
        ItemStack::of(ids::BUCKET).max_size(&f)
    );
    println!("dirt max_size={}", ItemStack::of(ids::DIRT).max_size(&f));

    // 3. Crafting: pickaxe in the 3x3 grid, stick in the 2x2.
    println!("== crafting ==");
    let manager = vanilla_crafting_manager();
    let mut table = CraftingInventory::new(3, 3);
    for (index, id) in [
        Some(ids::IRON_INGOT),
        Some(ids::IRON_INGOT),
        Some(ids::IRON_INGOT),
        None,
        Some(ids::STICK),
        None,
        None,
        Some(ids::STICK),
        None,
    ]
    .into_iter()
    .enumerate()
    {
        if let Some(id) = id {
            table.set_item(index, Some(ItemStack::new(id, 1, 0)));
        }
    }
    for row in 0..3 {
        println!("grid {}", grid_line(&table, row, 3));
    }
    let out = manager.get_result(&table, &f).unwrap();
    println!("table -> {}x{}", out.id, out.size);

    let mut small = CraftingInventory::new(2, 2);
    small.set_item(0, stack(ids::PLANKS, 1));
    small.set_item(2, stack(ids::PLANKS, 1));
    let out = manager.get_result(&small, &f).unwrap();
    println!("2x2 planks column -> {}x{}", out.id, out.size);

    // 4. Repair.
    let mut repair = CraftingInventory::new(3, 3);
    repair.set_item(0, Some(ItemStack::new(ids::IRON_PICKAXE, 1, 60)));
    repair.set_item(1, Some(ItemStack::new(ids::IRON_PICKAXE, 1, 30)));
    let out = manager.get_result(&repair, &f).unwrap();
    println!("repair 60+30 damage -> damage {}", out.metadata);

    // 5. Menu clicks: craft sticks in the player menu and take them twice.
    println!("== menu clicks ==");
    let inv = ctx.inventory.clone();
    let mut menu = Menu::player(&inv);
    menu.set_slot_item(1, stack(ids::PLANKS, 4));
    menu.set_slot_item(3, stack(ids::PLANKS, 4));
    let result = menu.slot(0).unwrap().item().unwrap();
    println!("result {}x{}", result.id, result.size);
    menu.on_click_slot(0, 0, action::PICKUP, &mut ctx);
    let cursor = ctx.inventory.borrow().cursor_item().unwrap().copy();
    println!("took cursor {}x{}", cursor.id, cursor.size);
    println!(
        "grid planks left {} / {}",
        menu.slot(1).unwrap().item().unwrap().size,
        menu.slot(3).unwrap().item().unwrap().size
    );
    let result = menu.slot(0).unwrap().item().unwrap();
    println!("recomputed {}x{}", result.id, result.size);

    // 6. Chest quick move.
    let storage = Menu::chest_storage();
    let mut chest = Menu::chest(&inv, storage);
    chest.set_slot_item(0, Some(ItemStack::new(ids::DIRT, 70, 0)));
    chest.on_click_slot(0, 0, action::QUICK_MOVE, &mut ctx);
    let total: i32 = (0..36)
        .filter_map(|slot| ctx.inventory.borrow().items[slot].as_ref().map(|s| s.size))
        .sum();
    println!("quick move 70 dirt -> player holds {}", total);

    // 7. Furnace: 2 iron ore + 1 coal.
    println!("== furnace ==");
    let smelting = SmeltingManager::new();
    let mut furnace = sunset_1_7_inventory::furnace::FurnaceBlockEntity::new();
    furnace.set_item(0, Some(ItemStack::new(ids::IRON_ORE, 2, 0)));
    furnace.set_item(1, Some(ItemStack::new(ids::COAL, 1, 0)));
    for _ in 0..400 {
        furnace.tick(&f, &smelting);
    }
    println!(
        "after 400 ticks: out={:?} in={:?} fuel_left={}",
        furnace.slots[2].as_ref().map(|s| (s.id, s.size)),
        furnace.slots[0].as_ref().map(|s| s.size),
        furnace.fuel_time
    );
}
