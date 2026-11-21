//! Minecraft 1.7.10 inventory. See docs/ for the port contract.

pub mod crafting;
pub mod furnace;
pub mod inventory;
pub mod item_stack;
pub mod menu;
pub mod recipes;
pub mod registry;
pub mod smelting;

pub use crafting::{CraftingManager, Recipe};
pub use furnace::FurnaceBlockEntity;
pub use inventory::{
    CraftingInventory, DoubleInventory, Inventory, PlayerInventory, ResultInventory,
    SimpleInventory,
};
pub use item_stack::ItemStack;
pub use menu::{Menu, MenuKind, PlayerCtx, Slot};
pub use registry::{ItemFacts, VanillaFacts};
pub use smelting::SmeltingManager;

#[cfg(test)]
mod tests {

    use crate::furnace::{FurnaceBlockEntity, vanilla_fuel_time};
    use crate::inventory::{CraftingInventory, Inventory, PlayerInventory};
    use crate::item_stack::ItemStack;
    use crate::menu::{Menu, PlayerCtx, action};
    use crate::recipes::vanilla_crafting_manager;
    use crate::registry::{VanillaFacts, ids};
    use crate::smelting::SmeltingManager;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// The same inventory behind a menu and a ctx (the menu's slots and the
    /// player actions must read one object).
    fn setup() -> (Menu, PlayerCtx) {
        let inv = Rc::new(RefCell::new(PlayerInventory::new()));
        let menu = Menu::player(&inv);
        (menu, PlayerCtx::new(inv, false))
    }

    #[test]
    fn stack_matches_and_split() {
        let f = VanillaFacts;
        let a = ItemStack::new(ids::IRON_INGOT, 5, 0);
        let b = ItemStack::new(ids::IRON_INGOT, 5, 0);
        assert!(a.matches(&b));
        let damaged = ItemStack::new(ids::IRON_PICKAXE, 1, 10);
        assert!(!a.matches(&damaged));
        let split = a.split(2);
        assert_eq!(split.size, 2);
        assert_eq!(split.id, ids::IRON_INGOT);
        // Tools stack 1; damaged tools never stack.
        assert_eq!(ItemStack::of(ids::IRON_PICKAXE).max_size(&f), 1);
        let pick = ItemStack::new(ids::IRON_PICKAXE, 1, 4);
        assert!(!pick.is_stackable(&f));
        assert!(ItemStack::new(ids::DIRT, 12, 0).is_stackable(&f));
        // NBT identity: different nbt never matches.
        let mut tagged = ItemStack::new(ids::FILLED_MAP, 1, 0);
        tagged.nbt = Some("scale:1".into());
        let mut other = ItemStack::new(ids::FILLED_MAP, 1, 0);
        other.nbt = Some("scale:3".into());
        assert!(!tagged.matches(&other));
        assert_eq!(vanilla_fuel_time(&ItemStack::of(ids::LAVA_BUCKET)), 20000);
        assert_eq!(vanilla_fuel_time(&ItemStack::of(ids::STICK)), 100);
        assert_eq!(vanilla_fuel_time(&ItemStack::of(ids::DIRT)), 0);
    }

    #[test]
    fn add_item_merges_then_empties_in_index_order() {
        let f = VanillaFacts;
        let mut inv = PlayerInventory::new();
        inv.items[3] = Some(ItemStack::new(ids::DIRT, 60, 0));
        inv.items[0] = Some(ItemStack::new(ids::STICK, 40, 0));
        let mut stack = ItemStack::new(ids::DIRT, 10, 0);
        assert!(inv.add_item(&mut stack, &f, false));
        // 4 dirt into slot 3 (60 -> 64), then 6 into the first empty slot 1.
        assert_eq!(inv.items[3].as_ref().unwrap().size, 64);
        assert_eq!(inv.items[1].as_ref().unwrap().size, 6);
        assert_eq!(stack.size, 0);
        // Full inventory: the leftover stays with the caller.
        let mut inv = PlayerInventory::new();
        inv.items = std::array::from_fn(|_| Some(ItemStack::new(ids::DIRT, 64, 0)));
        let mut stack = ItemStack::new(ids::DIRT, 5, 0);
        assert!(!inv.add_item(&mut stack, &f, false));
        assert_eq!(stack.size, 5);
        // Creative swallows what cannot fit.
        let mut inv = PlayerInventory::new();
        inv.items = std::array::from_fn(|_| Some(ItemStack::new(ids::DIRT, 64, 0)));
        let mut stack = ItemStack::new(ids::DIRT, 5, 0);
        assert!(inv.add_item(&mut stack, &f, true));
        assert_eq!(stack.size, 0);
    }

    #[test]
    fn shaped_recipe_positions_and_mirror() {
        let f = VanillaFacts;
        let manager = vanilla_crafting_manager();
        // Pickaxe: XXX / " # " / " # ".
        let mut grid = CraftingInventory::new(3, 3);
        grid.set_item(0, Some(ItemStack::new(ids::IRON_INGOT, 1, 0)));
        grid.set_item(1, Some(ItemStack::new(ids::IRON_INGOT, 1, 0)));
        grid.set_item(2, Some(ItemStack::new(ids::IRON_INGOT, 1, 0)));
        grid.set_item(4, Some(ItemStack::new(ids::STICK, 1, 0)));
        grid.set_item(7, Some(ItemStack::new(ids::STICK, 1, 0)));
        let result = manager.get_result(&grid, &f).unwrap();
        assert_eq!(result.id, ids::IRON_PICKAXE);
        // 1.7.10 scans 0..=3-width over 3: a 3-wide recipe only anchors at
        // column 0, so the same shape one column right stays empty.
        let mut shifted = CraftingInventory::new(3, 3);
        shifted.set_item(1, Some(ItemStack::new(ids::IRON_INGOT, 1, 0)));
        shifted.set_item(2, Some(ItemStack::new(ids::IRON_INGOT, 1, 0)));
        shifted.set_item(3, Some(ItemStack::new(ids::IRON_INGOT, 1, 0)));
        shifted.set_item(5, Some(ItemStack::new(ids::STICK, 1, 0)));
        shifted.set_item(8, Some(ItemStack::new(ids::STICK, 1, 0)));
        assert!(manager.get_result(&shifted, &f).is_none());
        // Wrong item in the middle fails.
        let mut wrong = CraftingInventory::new(3, 3);
        wrong.set_item(0, Some(ItemStack::new(ids::IRON_INGOT, 1, 0)));
        wrong.set_item(1, Some(ItemStack::new(ids::IRON_INGOT, 1, 0)));
        wrong.set_item(2, Some(ItemStack::new(ids::IRON_INGOT, 1, 0)));
        wrong.set_item(4, Some(ItemStack::new(ids::DIRT, 1, 0)));
        wrong.set_item(7, Some(ItemStack::new(ids::STICK, 1, 0)));
        assert!(manager.get_result(&wrong, &f).is_none());
        // The axe (XX / X# / " #"): 3 iron, 2 sticks.
        let mut axe = CraftingInventory::new(3, 3);
        axe.set_item(0, Some(ItemStack::new(ids::IRON_INGOT, 1, 0)));
        axe.set_item(1, Some(ItemStack::new(ids::IRON_INGOT, 1, 0)));
        axe.set_item(3, Some(ItemStack::new(ids::IRON_INGOT, 1, 0)));
        axe.set_item(4, Some(ItemStack::new(ids::STICK, 1, 0)));
        axe.set_item(7, Some(ItemStack::new(ids::STICK, 1, 0)));
        assert_eq!(manager.get_result(&axe, &f).unwrap().id, ids::IRON_AXE);
        // Mirrored: the iron column on the right.
        let mut axe_mirrored = CraftingInventory::new(3, 3);
        axe_mirrored.set_item(0, Some(ItemStack::new(ids::IRON_INGOT, 1, 0)));
        axe_mirrored.set_item(1, Some(ItemStack::new(ids::IRON_INGOT, 1, 0)));
        axe_mirrored.set_item(4, Some(ItemStack::new(ids::IRON_INGOT, 1, 0)));
        axe_mirrored.set_item(3, Some(ItemStack::new(ids::STICK, 1, 0)));
        axe_mirrored.set_item(6, Some(ItemStack::new(ids::STICK, 1, 0)));
        assert_eq!(
            manager.get_result(&axe_mirrored, &f).unwrap().id,
            ids::IRON_AXE
        );
    }

    #[test]
    fn two_by_two_grid_never_fits_three_wide() {
        let f = VanillaFacts;
        let manager = vanilla_crafting_manager();
        let mut grid = CraftingInventory::new(2, 2);
        grid.set_item(0, Some(ItemStack::new(ids::IRON_INGOT, 1, 0)));
        grid.set_item(1, Some(ItemStack::new(ids::IRON_INGOT, 1, 0)));
        grid.set_item(2, Some(ItemStack::new(ids::IRON_INGOT, 1, 0)));
        // XXX is 3 wide: the third column reads null in a 2x2 grid.
        assert!(manager.get_result(&grid, &f).is_none());
        // Pickaxe in a 2x2: only XX plus a stick below is craftable pieces —
        // the pickaxe needs 3 columns, so this stays empty.
        let mut grid = CraftingInventory::new(2, 2);
        grid.set_item(0, Some(ItemStack::new(ids::STICK, 1, 0)));
        grid.set_item(2, Some(ItemStack::new(ids::STICK, 1, 0)));
        assert!(manager.get_result(&grid, &f).is_none());
    }

    #[test]
    fn shapeless_and_repair() {
        let f = VanillaFacts;
        let manager = vanilla_crafting_manager();
        // Shapeless book: paper, paper, paper, leather — order free.
        let mut grid = CraftingInventory::new(3, 3);
        grid.set_item(3, Some(ItemStack::new(ids::PAPER, 1, 0)));
        grid.set_item(4, Some(ItemStack::new(ids::LEATHER, 1, 0)));
        grid.set_item(5, Some(ItemStack::new(ids::PAPER, 1, 0)));
        grid.set_item(6, Some(ItemStack::new(ids::PAPER, 1, 0)));
        assert_eq!(manager.get_result(&grid, &f).unwrap().id, ids::BOOK);
        // An extra item kills the shapeless match.
        let mut extra = CraftingInventory::new(3, 3);
        extra.set_item(0, Some(ItemStack::new(ids::DIRT, 1, 0)));
        extra.set_item(3, Some(ItemStack::new(ids::PAPER, 1, 0)));
        extra.set_item(4, Some(ItemStack::new(ids::LEATHER, 1, 0)));
        extra.set_item(5, Some(ItemStack::new(ids::PAPER, 1, 0)));
        extra.set_item(6, Some(ItemStack::new(ids::PAPER, 1, 0)));
        assert!(manager.get_result(&extra, &f).is_none());

        // Repair: two damaged picks, life 250 each.
        let mut grid = CraftingInventory::new(3, 3);
        grid.set_item(0, Some(ItemStack::new(ids::IRON_PICKAXE, 1, 100)));
        grid.set_item(1, Some(ItemStack::new(ids::IRON_PICKAXE, 1, 50)));
        let repaired = manager.get_result(&grid, &f).unwrap();
        assert_eq!(repaired.id, ids::IRON_PICKAXE);
        // lives 150 + 200 + 250*5/100 = 362 -> damage 250-362 <0 -> 0.
        assert_eq!(repaired.metadata, 0);
        // Moderate: 10 + 20 damage -> lives 240 + 230 + 12 = 482 -> damage 0.
        let mut grid = CraftingInventory::new(3, 3);
        grid.set_item(0, Some(ItemStack::new(ids::IRON_PICKAXE, 1, 10)));
        grid.set_item(1, Some(ItemStack::new(ids::IRON_PICKAXE, 1, 20)));
        let repaired = manager.get_result(&grid, &f).unwrap();
        assert_eq!(repaired.metadata, 0);
        // Light damage: 2 + 3 -> lives 248 + 247 + 12 = 507 -> damage 0.
        // Formula: remaining = max - (lifeA + lifeB + max*5/100).
        let mut grid = CraftingInventory::new(3, 3);
        grid.set_item(0, Some(ItemStack::new(ids::IRON_PICKAXE, 1, 124)));
        grid.set_item(1, Some(ItemStack::new(ids::IRON_PICKAXE, 1, 124)));
        let repaired = manager.get_result(&grid, &f).unwrap();
        // lives 126+126+12=264 -> 250-264 <0 -> 0.
        assert_eq!(repaired.metadata, 0);
        // A gold pick (max 32), damage 20+20: lives 12+12+32*5/100=25 ->
        // remaining damage 32-25 = 7.
        let mut grid = CraftingInventory::new(3, 3);
        grid.set_item(0, Some(ItemStack::new(ids::GOLDEN_PICKAXE, 1, 20)));
        grid.set_item(1, Some(ItemStack::new(ids::GOLDEN_PICKAXE, 1, 20)));
        assert_eq!(manager.get_result(&grid, &f).unwrap().metadata, 7);
        // Non-damageable pairs go through recipes only.
        let mut grid = CraftingInventory::new(3, 3);
        grid.set_item(0, Some(ItemStack::new(ids::DIRT, 1, 0)));
        grid.set_item(1, Some(ItemStack::new(ids::DIRT, 1, 0)));
        assert!(manager.get_result(&grid, &f).is_none());
    }

    #[test]
    fn recipe_sort_shapeless_after_shaped_size_desc() {
        let manager = vanilla_crafting_manager();
        let mut shapeless_seen = false;
        for recipe in manager.recipes() {
            if recipe.is_shapeless() {
                shapeless_seen = true;
            } else {
                assert!(!shapeless_seen, "shaped recipe after a shapeless one");
            }
        }
        // Sizes descend inside each run.
        let mut last = usize::MAX;
        let mut shapeless = false;
        for recipe in manager.recipes() {
            let now = recipe.is_shapeless();
            if now && !shapeless {
                last = usize::MAX;
            }
            assert!(recipe.size() <= last);
            last = recipe.size();
            shapeless = now;
        }
    }

    #[test]
    fn menu_pickup_place_and_half_split() {
        let (mut menu, mut ctx) = setup();
        let _f = VanillaFacts;
        // Put 10 dirt into main slot 9 (menu index 0 is result; main starts at 9).
        menu.set_slot_item(9, Some(ItemStack::new(ids::DIRT, 10, 0)));
        // Left click picks the whole stack up; the copy is what the client shows.
        assert_eq!(
            menu.on_click_slot(9, 0, action::PICKUP, &mut ctx),
            Some(ItemStack::new(ids::DIRT, 10, 0))
        );
        assert!(menu.slot(9).unwrap().item().is_none());
        assert_eq!(ctx.inventory.borrow().cursor_item().unwrap().size, 10);
        // Left click on the empty slot places everything back.
        menu.on_click_slot(9, 0, action::PICKUP, &mut ctx);
        assert_eq!(menu.slot(9).unwrap().item().unwrap().size, 10);
        assert!(ctx.inventory.borrow().cursor_item().is_none());
        // Right click takes half.
        menu.on_click_slot(9, 1, action::PICKUP, &mut ctx);
        assert_eq!(ctx.inventory.borrow().cursor_item().unwrap().size, 5);
        // Right click on the now-5 stack with a held stack merges 1.
        menu.on_click_slot(9, 1, action::PICKUP, &mut ctx);
        assert_eq!(menu.slot(9).unwrap().item().unwrap().size, 6);
        assert_eq!(ctx.inventory.borrow().cursor_item().unwrap().size, 4);
    }

    #[test]
    fn menu_swap_number_key_and_drop() {
        let (mut menu, mut ctx) = setup();
        menu.set_slot_item(9, Some(ItemStack::new(ids::DIRT, 3, 0)));
        ctx.inventory.borrow_mut().items[0] = Some(ItemStack::new(ids::STICK, 2, 0));
        // Swap slot 9 with hotbar 0.
        menu.on_click_slot(9, 0, action::SWAP, &mut ctx);
        assert_eq!(menu.slot(9).unwrap().item().unwrap().id, ids::STICK);
        assert_eq!(
            ctx.inventory.borrow().items[0].as_ref().unwrap().id,
            ids::DIRT
        );
        // Throw one out of slot 9.
        menu.on_click_slot(9, 0, action::THROW, &mut ctx);
        assert_eq!(ctx.drops[0].id, ids::STICK);
        assert_eq!(ctx.drops[0].size, 1);
        // Throw the whole stack.
        menu.on_click_slot(9, 1, action::THROW, &mut ctx);
        assert_eq!(ctx.drops[1].size, 1);
    }

    #[test]
    fn menu_quick_move_chest_to_player() {
        let inventory = Rc::new(RefCell::new(PlayerInventory::new()));
        let storage = Menu::chest_storage();
        let mut menu = Menu::chest(&inventory, storage.clone());
        let mut ctx = PlayerCtx::new(inventory, false);
        // 70 sticks in chest slot 0 -> menu index 0.
        menu.set_slot_item(0, Some(ItemStack::new(ids::STICK, 70, 0)));
        menu.on_click_slot(0, 0, action::QUICK_MOVE, &mut ctx);
        // The stack moved into the player's main inventory (64 + 6).
        assert!(menu.slot(0).unwrap().item().is_none());
        let mut total = 0;
        for slot in 0..36 {
            if let Some(held) = ctx.inventory.borrow().items[slot].as_ref() {
                total += held.size;
            }
        }
        assert_eq!(total, 70);
    }

    #[test]
    fn menu_craft_result_consumes_grid_and_recomputes() {
        let inventory = Rc::new(RefCell::new(PlayerInventory::new()));
        let mut menu = Menu::player(&inventory);
        let mut ctx = PlayerCtx::new(inventory, false);
        // Two planks (stacked) above one another make sticks, 4 of them.
        menu.set_slot_item(1, Some(ItemStack::new(ids::PLANKS, 3, 0)));
        menu.set_slot_item(3, Some(ItemStack::new(ids::PLANKS, 3, 0)));
        let result = menu.slot(0).unwrap().item().unwrap();
        assert_eq!(result.id, ids::STICK);
        assert_eq!(result.size, 4);
        // Take the result: one plank per cell goes, the result recomputes.
        menu.on_click_slot(0, 0, action::PICKUP, &mut ctx);
        assert_eq!(ctx.inventory.borrow().cursor_item().unwrap().id, ids::STICK);
        assert_eq!(menu.slot(1).unwrap().item().unwrap().size, 2);
        assert_eq!(menu.slot(3).unwrap().item().unwrap().size, 2);
        let result = menu.slot(0).unwrap().item().unwrap();
        assert_eq!(result.id, ids::STICK);
        assert_eq!(result.size, 4);
    }

    #[test]
    fn furnace_burn_and_smelt_cycle() {
        let f = VanillaFacts;
        let smelting = SmeltingManager::new();
        let mut furnace = FurnaceBlockEntity::new();
        furnace.set_item(0, Some(ItemStack::new(ids::IRON_ORE, 3, 0)));
        furnace.set_item(1, Some(ItemStack::new(ids::COAL, 1, 0)));
        // Coal: 1600 ticks; each smelt 200 -> 3 ores in 600 ticks.
        for _ in 0..600 {
            furnace.tick(&f, &smelting);
        }
        assert_eq!(furnace.slots[2].as_ref().unwrap().id, ids::IRON_INGOT);
        assert_eq!(furnace.slots[2].as_ref().unwrap().size, 3);
        assert!(furnace.slots[0].is_none());
        // The coal burned 599 of its 1600 ticks: tick 1 starts with
        // fuelTime 0, so it ignites without decrementing first.
        assert_eq!(furnace.fuel_time, 1001);
        // Idle: nothing more smelts once the input is gone.
        for _ in 0..500 {
            furnace.tick(&f, &smelting);
        }
        assert_eq!(furnace.slots[2].as_ref().unwrap().size, 3);
        assert_eq!(furnace.fuel_time, 501);
    }

    #[test]
    fn furnace_lava_bucket_leaves_a_bucket() {
        let f = VanillaFacts;
        let smelting = SmeltingManager::new();
        let mut furnace = FurnaceBlockEntity::new();
        furnace.set_item(0, Some(ItemStack::new(ids::PORKCHOP, 1, 0)));
        furnace.set_item(1, Some(ItemStack::of(ids::LAVA_BUCKET)));
        for _ in 0..200 {
            furnace.tick(&f, &smelting);
        }
        assert_eq!(furnace.slots[2].as_ref().unwrap().id, ids::COOKED_PORKCHOP);
        // The lava bucket is consumed, the empty bucket remains.
        let fuel = furnace.slots[1].as_ref().unwrap();
        assert_eq!(fuel.id, ids::BUCKET);
        assert_eq!(fuel.size, 1);
    }

    #[test]
    fn furnace_slots_restrict_inserts() {
        let furnace = FurnaceBlockEntity::new();
        assert!(!furnace.is_item_allowed(2, &ItemStack::of(ids::IRON_INGOT)));
        assert!(!furnace.is_item_allowed(1, &ItemStack::of(ids::DIRT)));
        assert!(furnace.is_item_allowed(1, &ItemStack::of(ids::COAL)));
        assert!(furnace.is_item_allowed(0, &ItemStack::of(ids::IRON_ORE)));
    }

    #[test]
    fn menu_click_drag_splits_evenly() {
        let (mut menu, mut ctx) = setup();
        // Hold 12 dirt on the cursor, drag-left (mode 0) over two slots.
        ctx.inventory
            .borrow_mut()
            .set_cursor_item(Some(ItemStack::new(ids::DIRT, 12, 0)));
        let stage = |n: i32, mode: i32| n | (mode << 2);
        // Stage 0: start drag mode 0 (split evenly).
        menu.on_click_slot(-1, stage(0, 0), action::QUICK_CRAFT, &mut ctx);
        // Stage 1: add both main slots (menu indexes 9 and 10).
        menu.on_click_slot(9, stage(1, 0), action::QUICK_CRAFT, &mut ctx);
        menu.on_click_slot(10, stage(1, 0), action::QUICK_CRAFT, &mut ctx);
        // Stage 2: distribute.
        menu.on_click_slot(-1, stage(2, 0), action::QUICK_CRAFT, &mut ctx);
        assert_eq!(menu.slot(9).unwrap().item().unwrap().size, 6);
        assert_eq!(menu.slot(10).unwrap().item().unwrap().size, 6);
        // 12 - 12 = 0 left on the cursor.
        assert!(ctx.inventory.borrow().cursor_item().is_none());
        // Drag-right (mode 1) puts one item per slot — over empty slots 11/12.
        ctx.inventory
            .borrow_mut()
            .set_cursor_item(Some(ItemStack::new(ids::STICK, 5, 0)));
        menu.on_click_slot(-1, stage(0, 1), action::QUICK_CRAFT, &mut ctx);
        menu.on_click_slot(11, stage(1, 1), action::QUICK_CRAFT, &mut ctx);
        menu.on_click_slot(12, stage(1, 1), action::QUICK_CRAFT, &mut ctx);
        menu.on_click_slot(-1, stage(2, 1), action::QUICK_CRAFT, &mut ctx);
        assert_eq!(menu.slot(11).unwrap().item().unwrap().id, ids::STICK);
        assert_eq!(menu.slot(11).unwrap().item().unwrap().size, 1);
        assert_eq!(menu.slot(12).unwrap().item().unwrap().size, 1);
        assert_eq!(ctx.inventory.borrow().cursor_item().unwrap().size, 3);
    }

    #[test]
    fn crafting_table_menu_crafts_and_moves_out_of_the_grid() {
        let inventory = Rc::new(RefCell::new(PlayerInventory::new()));
        let mut menu = Menu::crafting_table(&inventory);
        let mut ctx = PlayerCtx::new(inventory, false);
        menu.set_slot_item(1, Some(ItemStack::new(ids::IRON_INGOT, 5, 0)));
        // Shift-click the grid slot: everything goes to the player's inventory.
        menu.on_click_slot(1, 0, action::QUICK_MOVE, &mut ctx);
        assert!(menu.slot(1).unwrap().item().is_none());
        let held = ctx.inventory.borrow().items[9].clone().unwrap();
        assert_eq!(held.id, ids::IRON_INGOT);
        assert_eq!(held.size, 5);
        // A 3x3 pickaxe shape crafts through the table menu's result slot.
        menu.set_slot_item(1, Some(ItemStack::new(ids::IRON_INGOT, 1, 0)));
        menu.set_slot_item(2, Some(ItemStack::new(ids::IRON_INGOT, 1, 0)));
        menu.set_slot_item(3, Some(ItemStack::new(ids::IRON_INGOT, 1, 0)));
        menu.set_slot_item(5, Some(ItemStack::new(ids::STICK, 1, 0)));
        menu.set_slot_item(8, Some(ItemStack::new(ids::STICK, 1, 0)));
        assert_eq!(menu.slot(0).unwrap().item().unwrap().id, ids::IRON_PICKAXE);
    }

    #[test]
    fn furnace_menu_quick_move_routes_smeltables_and_fuel() {
        let inventory = Rc::new(RefCell::new(PlayerInventory::new()));
        let furnace = Rc::new(RefCell::new(FurnaceBlockEntity::new()));
        let mut menu = Menu::furnace(&inventory, furnace.clone());
        let mut ctx = PlayerCtx::new(inventory, false);
        menu.set_slot_item(3, Some(ItemStack::new(ids::IRON_ORE, 2, 0)));
        menu.set_slot_item(4, Some(ItemStack::new(ids::COAL, 2, 0)));
        menu.set_slot_item(5, Some(ItemStack::new(ids::DIRT, 1, 0)));
        // Ore into the input slot, coal into fuel, dirt into the player only.
        menu.on_click_slot(3, 0, action::QUICK_MOVE, &mut ctx);
        menu.on_click_slot(4, 0, action::QUICK_MOVE, &mut ctx);
        menu.on_click_slot(5, 0, action::QUICK_MOVE, &mut ctx);
        let furnace = furnace.borrow();
        assert_eq!(furnace.slots[0].as_ref().unwrap().id, ids::IRON_ORE);
        assert_eq!(furnace.slots[1].as_ref().unwrap().id, ids::COAL);
        assert!(furnace.slots[2].is_none());
        // The dirt walked to the hotbar (main slots move to 30..39 first).
        assert_eq!(
            ctx.inventory.borrow().items[0].as_ref().unwrap().id,
            ids::DIRT
        );
    }

    #[test]
    fn double_chest_reads_as_one() {
        let storage = Menu::double_chest_storage();
        assert_eq!(storage.borrow().size(), 54);
        let inventory = Rc::new(RefCell::new(PlayerInventory::new()));
        let menu = Menu::chest(&inventory, storage);
        assert_eq!(menu.slot_count(), 54 + 36);
        assert_eq!(menu.player_offset(), 54);
    }
}
