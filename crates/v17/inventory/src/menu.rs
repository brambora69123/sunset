//! `net.minecraft.inventory.menu`: the container contract, slot kinds and the
//! 1.7.10 click operations of `InventoryMenu.onClickSlot`.

use crate::furnace::{FurnaceBlockEntity, is_fuel};
use crate::inventory::{
    CraftingInventory, DoubleInventory, Inventory, PlayerInventory, ResultInventory,
    SimpleInventory,
};
use crate::item_stack::{ItemStack, matches};
use crate::registry::{VanillaFacts, ids};
use crate::smelting::SmeltingManager;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Click actions, `InventoryMenu.onClickSlot` ids.
pub mod action {
    pub const PICKUP: i32 = 0;
    pub const QUICK_MOVE: i32 = 1;
    pub const SWAP: i32 = 2;
    pub const CLONE: i32 = 3;
    pub const THROW: i32 = 4;
    pub const QUICK_CRAFT: i32 = 5;
    pub const PICKUP_ALL: i32 = 6;
}

/// What a player brings to a menu open: the inventory, the mode and where
/// world drops land. `player.abilities.creativeMode` is `creative`.
pub struct PlayerCtx {
    pub inventory: Rc<RefCell<PlayerInventory>>,
    pub creative: bool,
    /// `player.dropItem`: leftovers the player cannot hold, plus close drops.
    pub drops: Vec<ItemStack>,
}

impl PlayerCtx {
    pub fn new(inventory: Rc<RefCell<PlayerInventory>>, creative: bool) -> Self {
        PlayerCtx {
            inventory,
            creative,
            drops: Vec::new(),
        }
    }
}

/// Which special slot a menu position is, `InventorySlot` and its overrides.
enum SlotKind {
    /// `InventorySlot`.
    Normal,
    /// `CraftingResultSlot`: refuses placed items, consumes the grid when taken.
    CraftingResult(Rc<RefCell<CraftingInventory>>, Cell<i32>),
    /// `FurnaceResultSlot`: refuses placed items; xp rolls are the glue's job.
    FurnaceResult(Cell<i32>),
    /// `PlayerMenu$82076943`: one armor piece, max stack 1.
    Armor(usize),
}

/// `net.minecraft.inventory.slot.InventorySlot`: one menu position over a
/// shared inventory, (`inventory`, `id`) plus the menu index.
pub struct Slot {
    /// Menu index, `addSlot`'s `slot.index`.
    pub index: usize,
    /// The inventory slot, `InventorySlot.id`.
    pub id: usize,
    /// GUI position, `InventorySlot.x`/`y`.
    pub x: i32,
    pub y: i32,
    inventory: Rc<RefCell<dyn Inventory>>,
    kind: SlotKind,
}

impl Slot {
    fn new(
        inventory: Rc<RefCell<dyn Inventory>>,
        id: usize,
        x: i32,
        y: i32,
        kind: SlotKind,
    ) -> Self {
        Slot {
            index: 0,
            id,
            x,
            y,
            inventory,
            kind,
        }
    }

    /// `InventorySlot.getItem`.
    pub fn item(&self) -> Option<ItemStack> {
        self.inventory.borrow().get_item(self.id).cloned()
    }

    /// `InventorySlot.hasItem`.
    pub fn has_item(&self) -> bool {
        self.inventory.borrow().get_item(self.id).is_some()
    }

    /// `InventorySlot.setItem`.
    pub fn set_item(&self, item: Option<ItemStack>) {
        self.inventory.borrow_mut().set_item(self.id, item);
        self.mark_dirty();
    }

    /// `InventorySlot.removeItem` (with the result slots' remove counting).
    pub fn remove_item(&self, amount: i32) -> Option<ItemStack> {
        match &self.kind {
            SlotKind::CraftingResult(_, remove_amount) | SlotKind::FurnaceResult(remove_amount)
                if self.has_item() =>
            {
                let size = self.inventory.borrow().get_item(self.id).unwrap().size;
                remove_amount.set(remove_amount.get() + amount.min(size));
            }
            _ => {}
        }
        self.inventory.borrow_mut().remove_item(self.id, amount)
    }

    /// `InventorySlot.getMaxStackSize`.
    pub fn max_stack_size(&self) -> i32 {
        match &self.kind {
            SlotKind::Armor(_) => 1,
            _ => self.inventory.borrow().max_stack_size(),
        }
    }

    /// `InventorySlot.isItemAllowed`.
    pub fn is_item_allowed(&self, item: &ItemStack) -> bool {
        match &self.kind {
            SlotKind::CraftingResult(..) | SlotKind::FurnaceResult(_) => false,
            SlotKind::Armor(slot) => {
                if let Some(armor_slot) = armor_slot_of(item.id) {
                    armor_slot == *slot
                } else {
                    // Pumpkin and skulls only fit the head.
                    (item.id == ids::PUMPKIN || item.id == ids::SKULL) && *slot == 0
                }
            }
            SlotKind::Normal => self.inventory.borrow().is_item_allowed(self.id, item),
        }
    }

    /// `InventorySlot.canPickUp` (always true in 1.7.10's menus).
    pub fn can_pick_up(&self) -> bool {
        true
    }

    /// `InventorySlot.markDirty`.
    pub fn mark_dirty(&self) {
        self.inventory.borrow_mut().mark_dirty();
    }

    /// `InventorySlot.onQuickMoved(oldItem, newItem)`: result slots count the gain.
    pub fn on_quick_moved(&self, old: &ItemStack, new: &ItemStack) {
        match &self.kind {
            SlotKind::CraftingResult(_, remove_amount) | SlotKind::FurnaceResult(remove_amount)
                if old.id == new.id =>
            {
                let grown = new.size - old.size;
                if grown > 0 {
                    remove_amount.set(remove_amount.get() + grown);
                }
            }
            _ => {}
        }
    }

    /// `InventorySlot.onItemRemoved`: the crafting result consumes its grid,
    /// one item per cell, recipe remainders back into the grid or dropped.
    pub fn on_item_removed(&self, ctx: &mut PlayerCtx) {
        self.mark_dirty();
        let SlotKind::CraftingResult(crafting, _) = &self.kind else {
            return;
        };
        let f = &VanillaFacts;
        let size = crafting.borrow().size();
        for slot in 0..size {
            let Some(stack) = crafting.borrow().get_item(slot).cloned() else {
                continue;
            };
            crafting.borrow_mut().remove_item(slot, 1);
            if let Some(mut remainder) = stack.recipe_remainder(f) {
                let mut player = ctx.inventory.borrow_mut();
                let inserted = player.add_item(&mut remainder, f, ctx.creative);
                drop(player);
                if !inserted {
                    let mut grid = crafting.borrow_mut();
                    if grid.get_item(slot).is_none() {
                        grid.set_item(slot, Some(remainder));
                    } else {
                        drop(grid);
                        ctx.drops.push(remainder);
                    }
                }
            }
        }
    }

    /// The remove counter a result slot carries (`checkAchievements` reads it;
    /// here the caller drains it for stats/xp).
    pub fn take_remove_amount(&self) -> i32 {
        match &self.kind {
            SlotKind::CraftingResult(_, remove_amount) | SlotKind::FurnaceResult(remove_amount) => {
                remove_amount.replace(0)
            }
            SlotKind::Normal | SlotKind::Armor(_) => 0,
        }
    }
}

/// `ArmorItem.slot` for the ids the armor slots accept.
fn armor_slot_of(id: u16) -> Option<usize> {
    use ids::*;
    match id {
        LEATHER_HELMET | CHAINMAIL_HELMET | IRON_HELMET | DIAMOND_HELMET | GOLDEN_HELMET => Some(0),
        LEATHER_CHESTPLATE | CHAINMAIL_CHESTPLATE | IRON_CHESTPLATE | DIAMOND_CHESTPLATE
        | GOLDEN_CHESTPLATE => Some(1),
        LEATHER_LEGGINGS | CHAINMAIL_LEGGINGS | IRON_LEGGINGS | DIAMOND_LEGGINGS
        | GOLDEN_LEGGINGS => Some(2),
        LEATHER_BOOTS | CHAINMAIL_BOOTS | IRON_BOOTS | DIAMOND_BOOTS | GOLDEN_BOOTS => Some(3),
        _ => None,
    }
}

/// Per-menu state, `InventoryMenu` subclasses.
pub enum MenuKind {
    /// `PlayerMenu`: 2x2 crafting, 4 armor slots.
    Player {
        crafting: Rc<RefCell<CraftingInventory>>,
        result: Rc<RefCell<ResultInventory>>,
    },
    /// `CraftingTableMenu`: 3x3 crafting.
    CraftingTable {
        crafting: Rc<RefCell<CraftingInventory>>,
        result: Rc<RefCell<ResultInventory>>,
    },
    /// `ChestMenu`: single or double chest, rows from the inventory size.
    Chest {
        rows: usize,
        chest: Rc<RefCell<dyn Inventory>>,
    },
    /// `FurnaceMenu`.
    Furnace {
        furnace: Rc<RefCell<FurnaceBlockEntity>>,
    },
}

/// `net.minecraft.inventory.menu.InventoryMenu`: slots plus the click-drag state.
pub struct Menu {
    pub kind: MenuKind,
    slots: Vec<Slot>,
    /// Last broadcast state of every slot, `InventoryMenu.items`.
    items: Vec<Option<ItemStack>>,
    /// `InventoryMenu.clickDragMode`.
    click_drag_mode: i32,
    /// `InventoryMenu.clickDragStage`.
    click_drag_stage: i32,
    /// `InventoryMenu.clickDragSlots`.
    click_drag_slots: Vec<usize>,
    /// The crafting manager the grids resolve through.
    crafting_manager: Rc<RefCell<crate::crafting::CraftingManager>>,
    smelting_manager: Rc<RefCell<SmeltingManager>>,
}

/// `InventoryMenu.unpackClickDragMode`.
pub fn unpack_click_drag_mode(click_data: i32) -> i32 {
    click_data >> 2 & 3
}

/// `InventoryMenu.unpackClickDragStage`.
pub fn unpack_click_drag_stage(click_data: i32) -> i32 {
    click_data & 3
}

/// `InventoryMenu.isValidClickDragMode`.
pub fn is_valid_click_drag_mode(mode: i32) -> bool {
    mode == 0 || mode == 1
}

/// `InventoryMenu.canClickDragInto(slot, item, stackable)` static form.
pub fn can_click_drag_into_slot(slot: Option<&Slot>, item: &ItemStack) -> bool {
    let Some(slot) = slot else { return true };
    let Some(held) = slot.item() else { return true };
    held.matches_item(item) && held.matches_nbt(item) && held.size <= item.max_size(&VanillaFacts)
}

/// `InventoryMenu.updateClickDragStackSize`.
pub fn update_click_drag_stack_size(
    slot_count: usize,
    mode: i32,
    item: &mut ItemStack,
    stack_size: i32,
) {
    match mode {
        0 => item.size = (item.size as f32 / slot_count as f32).floor() as i32,
        1 => item.size = 1,
        _ => {}
    }
    item.size += stack_size;
}

impl Menu {
    fn empty(kind: MenuKind) -> Menu {
        Menu {
            kind,
            slots: Vec::new(),
            items: Vec::new(),
            click_drag_mode: -1,
            click_drag_stage: 0,
            click_drag_slots: Vec::new(),
            crafting_manager: Rc::new(RefCell::new(crate::recipes::vanilla_crafting_manager())),
            smelting_manager: Rc::new(RefCell::new(SmeltingManager::new())),
        }
    }

    /// `new PlayerMenu(playerInventory, active, player)`: result, 2x2 grid,
    /// 4 armor slots, main inventory, hotbar.
    pub fn player(player_inventory: &Rc<RefCell<PlayerInventory>>) -> Menu {
        let crafting = Rc::new(RefCell::new(CraftingInventory::new(2, 2)));
        let result = Rc::new(RefCell::new(ResultInventory::new()));
        let mut menu = Menu::empty(MenuKind::Player {
            crafting: crafting.clone(),
            result: result.clone(),
        });
        menu.add_slot(
            result.clone() as Rc<RefCell<dyn Inventory>>,
            0,
            144,
            36,
            SlotKind::CraftingResult(crafting.clone(), Cell::new(0)),
        );
        for row in 0..2 {
            for column in 0..2 {
                menu.add_slot(
                    crafting.clone() as Rc<RefCell<dyn Inventory>>,
                    column + row * 2,
                    88 + column as i32 * 18,
                    26 + row as i32 * 18,
                    SlotKind::Normal,
                );
            }
        }
        for armor in 0..4 {
            let id = player_inventory.borrow().size() - 1 - armor;
            menu.add_slot(
                player_inventory.clone() as Rc<RefCell<dyn Inventory>>,
                id,
                8,
                8 + armor as i32 * 18,
                SlotKind::Armor(armor),
            );
        }
        menu.build_player_slots(player_inventory);
        menu.watch_crafting_grid(&crafting);
        menu.recompute_craft_result();
        menu
    }

    /// `new CraftingTableMenu`: result, 3x3 grid, main inventory, hotbar.
    pub fn crafting_table(player_inventory: &Rc<RefCell<PlayerInventory>>) -> Menu {
        let crafting = Rc::new(RefCell::new(CraftingInventory::new(3, 3)));
        let result = Rc::new(RefCell::new(ResultInventory::new()));
        let mut menu = Menu::empty(MenuKind::CraftingTable {
            crafting: crafting.clone(),
            result: result.clone(),
        });
        menu.add_slot(
            result.clone() as Rc<RefCell<dyn Inventory>>,
            0,
            124,
            35,
            SlotKind::CraftingResult(crafting.clone(), Cell::new(0)),
        );
        for row in 0..3 {
            for column in 0..3 {
                menu.add_slot(
                    crafting.clone() as Rc<RefCell<dyn Inventory>>,
                    column + row * 3,
                    30 + column as i32 * 18,
                    17 + row as i32 * 18,
                    SlotKind::Normal,
                );
            }
        }
        menu.build_player_slots(player_inventory);
        menu.watch_crafting_grid(&crafting);
        menu.recompute_craft_result();
        menu
    }

    /// `new ChestMenu`: `rows * 9` chest slots over the (possibly double)
    /// inventory, then the player's.
    pub fn chest(
        player_inventory: &Rc<RefCell<PlayerInventory>>,
        chest: Rc<RefCell<dyn Inventory>>,
    ) -> Menu {
        let rows = chest.borrow().size() / 9;
        let mut menu = Menu::empty(MenuKind::Chest {
            rows,
            chest: chest.clone(),
        });
        menu.build_chest_slots(player_inventory, chest);
        menu
    }

    /// A single chest's storage (`new SimpleInventory(27)`).
    pub fn chest_storage() -> Rc<RefCell<dyn Inventory>> {
        Rc::new(RefCell::new(SimpleInventory::new(27)))
    }

    /// A double chest over two halves.
    pub fn double_chest_storage() -> Rc<RefCell<dyn Inventory>> {
        let first: Rc<RefCell<dyn Inventory>> = Rc::new(RefCell::new(SimpleInventory::new(27)));
        let second: Rc<RefCell<dyn Inventory>> = Rc::new(RefCell::new(SimpleInventory::new(27)));
        Rc::new(RefCell::new(DoubleInventory::new(first, second)))
    }

    /// `new FurnaceMenu`: input, fuel, result, main inventory, hotbar.
    pub fn furnace(
        player_inventory: &Rc<RefCell<PlayerInventory>>,
        furnace: Rc<RefCell<FurnaceBlockEntity>>,
    ) -> Menu {
        let mut menu = Menu::empty(MenuKind::Furnace {
            furnace: furnace.clone(),
        });
        menu.add_slot(
            furnace.clone() as Rc<RefCell<dyn Inventory>>,
            0,
            56,
            17,
            SlotKind::Normal,
        );
        menu.add_slot(
            furnace.clone() as Rc<RefCell<dyn Inventory>>,
            1,
            56,
            53,
            SlotKind::Normal,
        );
        menu.add_slot(
            furnace.clone() as Rc<RefCell<dyn Inventory>>,
            2,
            116,
            35,
            SlotKind::FurnaceResult(Cell::new(0)),
        );
        menu.build_player_slots(player_inventory);
        menu
    }

    /// The crafting manager this menu's grids resolve through.
    pub fn crafting_manager(&self) -> &Rc<RefCell<crate::crafting::CraftingManager>> {
        &self.crafting_manager
    }

    pub fn smelting_manager(&self) -> &Rc<RefCell<SmeltingManager>> {
        &self.smelting_manager
    }

    fn build_player_slots(&mut self, player_inventory: &Rc<RefCell<PlayerInventory>>) {
        // Main inventory, rows 0-2 (slots 9-35); hotbar last (slots 0-8).
        for row in 0..3 {
            for column in 0..9 {
                self.add_slot(
                    player_inventory.clone() as Rc<RefCell<dyn Inventory>>,
                    column + row * 9 + 9,
                    8 + column as i32 * 18,
                    84 + row as i32 * 18,
                    SlotKind::Normal,
                );
            }
        }
        for column in 0..9 {
            self.add_slot(
                player_inventory.clone() as Rc<RefCell<dyn Inventory>>,
                column,
                8 + column as i32 * 18,
                142,
                SlotKind::Normal,
            );
        }
    }

    fn build_chest_slots(
        &mut self,
        player_inventory: &Rc<RefCell<PlayerInventory>>,
        chest: Rc<RefCell<dyn Inventory>>,
    ) {
        let rows = chest.borrow().size() / 9;
        let y_offset = (rows as i32 - 4) * 18;
        for row in 0..rows {
            for column in 0..9 {
                self.add_slot(
                    chest.clone() as Rc<RefCell<dyn Inventory>>,
                    column + row * 9,
                    8 + column as i32 * 18,
                    18 + row as i32 * 18,
                    SlotKind::Normal,
                );
            }
        }
        for row in 0..3 {
            for column in 0..9 {
                self.add_slot(
                    player_inventory.clone() as Rc<RefCell<dyn Inventory>>,
                    column + row * 9 + 9,
                    8 + column as i32 * 18,
                    103 + row as i32 * 18 + y_offset,
                    SlotKind::Normal,
                );
            }
        }
        for column in 0..9 {
            self.add_slot(
                player_inventory.clone() as Rc<RefCell<dyn Inventory>>,
                column,
                8 + column as i32 * 18,
                161 + y_offset,
                SlotKind::Normal,
            );
        }
    }

    fn watch_crafting_grid(&mut self, crafting: &Rc<RefCell<CraftingInventory>>) {
        // `onContentsChanged`: recompute the result slot on every grid change.
        let manager = self.crafting_manager.clone();
        let result_slot = self.first_result_slot_index();
        let result = match &self.kind {
            MenuKind::Player { result, .. } | MenuKind::CraftingTable { result, .. } => {
                result.clone()
            }
            _ => return,
        };
        crafting
            .borrow_mut()
            .set_on_change(Rc::new(move |grid: &CraftingInventory| {
                let recipe = manager.borrow().get_result(grid, &VanillaFacts);
                result.borrow_mut().set_item(0, recipe);
                let _ = result_slot;
            }));
    }

    fn first_result_slot_index(&self) -> usize {
        0
    }

    /// `InventoryMenu.onContentsChanged(crafting)`: recompute the result now.
    pub fn recompute_craft_result(&mut self) {
        let Some(crafting) = self.crafting_inventory() else {
            return;
        };
        // The Java hook fires from `setItem`/`removeItem` while the grid is
        // mid-update; the recipe reads only, so no freeze is needed.
        let recipe = {
            let grid = crafting.borrow();
            let manager = self.crafting_manager.borrow();
            manager.get_result(&grid, &VanillaFacts)
        };
        if let MenuKind::Player { result, .. } | MenuKind::CraftingTable { result, .. } = &self.kind
        {
            result.borrow_mut().set_item(0, recipe);
        }
    }

    fn crafting_inventory(&self) -> Option<Rc<RefCell<CraftingInventory>>> {
        match &self.kind {
            MenuKind::Player { crafting, .. } | MenuKind::CraftingTable { crafting, .. } => {
                Some(crafting.clone())
            }
            _ => None,
        }
    }

    fn add_slot(
        &mut self,
        inventory: Rc<RefCell<dyn Inventory>>,
        id: usize,
        x: i32,
        y: i32,
        kind: SlotKind,
    ) {
        let mut slot = Slot::new(inventory, id, x, y, kind);
        slot.index = self.slots.len();
        self.slots.push(slot);
        self.items.push(None);
    }

    /// `InventoryMenu.getSlot`.
    pub fn slot(&self, index: usize) -> Option<&Slot> {
        self.slots.get(index)
    }

    /// `InventoryMenu.slots.size()`.
    pub fn slot_count(&self) -> usize {
        self.slots.len()
    }

    /// The menu size before the player's own 36 slots (`PlayerMenu`: 9,
    /// `CraftingTableMenu`: 10, `ChestMenu`: chest size, `FurnaceMenu`: 3).
    pub fn player_offset(&self) -> usize {
        match &self.kind {
            MenuKind::Player { .. } => 9,
            MenuKind::CraftingTable { .. } => 10,
            MenuKind::Chest { chest, .. } => chest.borrow().size(),
            MenuKind::Furnace { .. } => 3,
        }
    }

    /// `InventoryMenu.getItems`: a snapshot of every slot.
    pub fn items(&self) -> Vec<Option<ItemStack>> {
        self.slots.iter().map(|slot| slot.item()).collect()
    }

    /// `InventoryMenu.updateListeners`: refresh the broadcast state; returns
    /// the slots whose stack changed (`onSlotChanged` rows).
    pub fn update_listeners(&mut self) -> Vec<usize> {
        let mut changed = Vec::new();
        for index in 0..self.slots.len() {
            let current = self.slots[index].item();
            if !matches(self.items[index].as_ref(), current.as_ref()) {
                self.items[index] = current.clone();
                changed.push(index);
            }
        }
        changed
    }

    /// `InventoryMenu.setItem`.
    pub fn set_slot_item(&mut self, index: usize, item: Option<ItemStack>) {
        self.slots[index].set_item(item);
    }

    /// `InventoryMenu.close`: the cursor drops; crafting grids drop their
    /// contents (`CraftingTableMenu.close`/`PlayerMenu.close`).
    pub fn close(&mut self, ctx: &mut PlayerCtx, drop_grid: bool) {
        let cursor = ctx.inventory.borrow_mut().take_cursor();
        if let Some(cursor) = cursor {
            ctx.drops.push(cursor);
        }
        if drop_grid {
            if let Some(crafting) = self.crafting_inventory() {
                let size = crafting.borrow().size();
                for slot in 0..size {
                    if let Some(item) = crafting.borrow_mut().remove_item_quietly(slot) {
                        ctx.drops.push(item);
                    }
                }
            }
            if let MenuKind::Player { result, .. } = &self.kind {
                result.borrow_mut().set_item(0, None);
            }
        }
        self.click_drag_slots.clear();
        self.click_drag_stage = 0;
    }

    /// `InventoryMenu.moveItem` (protected `moveItem`): fill matching stacks
    /// then empty slots across `[from, to)`, `reverse` walks it backwards.
    pub fn move_item(&mut self, index: usize, from: usize, to: usize, reverse: bool) -> bool {
        let f = &VanillaFacts;
        let Some(mut item) = self.slots[index].item() else {
            return false;
        };
        let mut moved = false;
        let mut cursor = if reverse { to - 1 } else { from };

        if item.is_stackable(f) {
            loop {
                if item.size <= 0 || !(if reverse { cursor >= from } else { cursor < to }) {
                    break;
                }
                if let Some(held) = self.slots[cursor].item()
                    && held.id == item.id
                    && (!item.has_custom_data(f) || item.metadata == held.metadata)
                    && held.matches_nbt(&item)
                {
                    let total = held.size + item.size;
                    if total <= held.max_size(f) {
                        item.size = 0;
                        let mut new_held = held.copy();
                        new_held.size = total;
                        self.slots[cursor].set_item(Some(new_held));
                        moved = true;
                    } else if held.size < held.max_size(f) {
                        let fill = held.max_size(f) - held.size;
                        item.size -= fill;
                        let mut new_held = held.copy();
                        new_held.size = held.max_size(f);
                        self.slots[cursor].set_item(Some(new_held));
                        moved = true;
                    }
                }
                cursor = if reverse { cursor - 1 } else { cursor + 1 };
            }
        }

        if item.size > 0 {
            cursor = if reverse { to - 1 } else { from };
            while if reverse { cursor >= from } else { cursor < to } {
                if self.slots[cursor].item().is_none() {
                    self.slots[cursor].set_item(Some(item.copy()));
                    item.size = 0;
                    moved = true;
                    break;
                }
                cursor = if reverse { cursor - 1 } else { cursor + 1 };
            }
        }

        if moved {
            self.slots[index].set_item(if item.size == 0 { None } else { Some(item) });
        }
        moved
    }

    /// `InventoryMenu.quickMoveItem`: shift-click; returns the copied stack
    /// when something moved, `None` when the click is a no-op.
    pub fn quick_move_item(&mut self, ctx: &mut PlayerCtx, index: usize) -> Option<ItemStack> {
        let slot = self.slots.get(index)?;
        if !slot.has_item() {
            return None;
        }
        let original = slot.item()?;
        let item = original.copy();
        let ok = match &self.kind {
            MenuKind::Player { .. } => {
                // PlayerMenu.quickMoveItem ranges: 0 result, 1-4 grid,
                // 5-8 armor, 9-35 main, 36-44 hotbar.
                if index == 0 {
                    self.move_item(index, 9, 45, true)
                } else if (1..5).contains(&index) || (5..9).contains(&index) {
                    self.move_item(index, 9, 45, false)
                } else if (9..36).contains(&index) {
                    self.move_item(index, 36, 45, false)
                } else if (36..45).contains(&index) {
                    self.move_item(index, 9, 36, false)
                } else {
                    self.move_item(index, 9, 45, false)
                }
            }
            MenuKind::CraftingTable { .. } => {
                // Ranges: 0 result, 1-9 grid, 10-36 main, 37-45 hotbar.
                if index == 0 {
                    self.move_item(index, 10, 46, true)
                } else if (10..37).contains(&index) {
                    self.move_item(index, 37, 46, false)
                } else if (37..46).contains(&index) {
                    self.move_item(index, 10, 37, false)
                } else {
                    self.move_item(index, 10, 46, false)
                }
            }
            MenuKind::Chest { rows, .. } => {
                let rows = *rows;
                if index < rows * 9 {
                    self.move_item(index, rows * 9, self.slot_count(), true)
                } else {
                    self.move_item(index, 0, rows * 9, false)
                }
            }
            MenuKind::Furnace { .. } => {
                // slot 2 result; 0 input, 1 fuel; smeltable -> input,
                // fuel -> fuel; main 3..30; hotbar 30..39.
                let smelting = self.smelting_manager.clone();
                if index == 2 {
                    self.move_item(index, 3, 39, true)
                } else if index != 0 && index != 1 {
                    if smelting.borrow().get_result(&item).is_some() {
                        self.move_item(index, 0, 1, false)
                    } else if is_fuel(&item) {
                        self.move_item(index, 1, 2, false)
                    } else if (3..30).contains(&index) {
                        self.move_item(index, 30, 39, false)
                    } else if (30..39).contains(&index) {
                        self.move_item(index, 3, 30, false)
                    } else {
                        false
                    }
                } else {
                    self.move_item(index, 3, 39, false)
                }
            }
        };
        if !ok {
            return None;
        }
        let after = self.slots[index].item();
        if after.as_ref().is_some_and(|left| left.size == 0) {
            self.slots[index].set_item(None);
        } else {
            self.slots[index].mark_dirty();
        }
        let after = self.slots[index].item();
        if after
            .as_ref()
            .is_some_and(|left| left.size == original.size)
        {
            return None;
        }
        self.slots[index].on_item_removed(ctx);
        if let Some(new) = after {
            self.slots[index].on_quick_moved(&original, &new);
        }
        Some(original.copy())
    }

    /// `InventoryMenu.onClickSlot(id, clickData, action, player)`: the whole
    /// 1.7.10 click table. Returns the stack to show the client (quick moves).
    pub fn on_click_slot(
        &mut self,
        id: i32,
        click_data: i32,
        action: i32,
        ctx: &mut PlayerCtx,
    ) -> Option<ItemStack> {
        let mut returned: Option<ItemStack> = None;
        let f = &VanillaFacts;

        if action == action::QUICK_CRAFT {
            // The drag-split distribution.
            let stage = unpack_click_drag_stage(click_data);
            let previous = self.click_drag_stage;
            self.click_drag_stage = stage;
            let aborted = ((previous != 1 || stage != 2) && previous != stage)
                || ctx.inventory.borrow().cursor_item().is_none();
            if aborted {
                self.drop_click_dragging();
            } else if stage == 0 {
                self.click_drag_mode = unpack_click_drag_mode(click_data);
                if is_valid_click_drag_mode(self.click_drag_mode) {
                    self.click_drag_stage = 1;
                    self.click_drag_slots.clear();
                } else {
                    self.drop_click_dragging();
                }
            } else if stage == 1 {
                let slot_ref = self.slots.get(id as usize)?;
                let cursor = ctx.inventory.borrow().cursor_item().unwrap().copy();
                if can_click_drag_into_slot(Some(slot_ref), &cursor)
                    && slot_ref.is_item_allowed(&cursor)
                    && cursor.size > self.click_drag_slots.len() as i32
                    && self.can_click_drag_into(slot_ref)
                {
                    self.click_drag_slots.push(id as usize);
                }
            } else if stage == 2 {
                if !self.click_drag_slots.is_empty() {
                    let cursor = ctx.inventory.borrow().cursor_item().unwrap().copy();
                    let mut leftover = cursor.size;
                    let slot_indexes: Vec<usize> = self.click_drag_slots.clone();
                    for slot_index in slot_indexes {
                        let Some(slot) = self.slots.get(slot_index) else {
                            continue;
                        };
                        let cursor_now = match ctx.inventory.borrow().cursor_item() {
                            Some(cursor) => cursor.copy(),
                            None => break,
                        };
                        if !can_click_drag_into_slot(Some(slot), &cursor_now)
                            || !slot.is_item_allowed(&cursor_now)
                            || cursor_now.size < self.click_drag_slots.len() as i32
                            || !self.can_click_drag_into(slot)
                        {
                            continue;
                        }
                        let mut placed = cursor_now.copy();
                        let current = slot.item().map(|held| held.size).unwrap_or(0);
                        update_click_drag_stack_size(
                            self.click_drag_slots.len(),
                            self.click_drag_mode,
                            &mut placed,
                            current,
                        );
                        if placed.size > placed.max_size(f) {
                            placed.size = placed.max_size(f);
                        }
                        if placed.size > slot.max_stack_size() {
                            placed.size = slot.max_stack_size();
                        }
                        leftover -= placed.size - current;
                        slot.set_item(Some(placed));
                    }
                    ctx.inventory
                        .borrow_mut()
                        .set_cursor_item(if leftover <= 0 {
                            None
                        } else {
                            let mut rest = cursor;
                            rest.size = leftover;
                            Some(rest)
                        });
                }
                self.drop_click_dragging();
            } else {
                self.drop_click_dragging();
            }
        } else if self.click_drag_stage != 0 {
            self.drop_click_dragging();
        } else if (action == action::PICKUP || action == action::QUICK_MOVE)
            && (click_data == 0 || click_data == 1)
        {
            if id == -999 {
                // Click outside the window: drop the cursor stack.
                let cursor = ctx.inventory.borrow_mut().take_cursor();
                if let Some(cursor) = cursor {
                    if click_data == 0 {
                        ctx.drops.push(cursor);
                    } else {
                        let mut one = cursor.split(1);
                        one.size = 1;
                        let rest = cursor;
                        if rest.size == 0 {
                            // already consumed by split
                        } else {
                            ctx.inventory.borrow_mut().set_cursor_item(Some(rest));
                        }
                        ctx.drops.push(one);
                    }
                }
            } else if action == action::QUICK_MOVE {
                if id < 0 {
                    return None;
                }
                let slot = self.slots.get(id as usize)?;
                if slot.can_pick_up()
                    && let Some(moved) = self.quick_move_item(ctx, id as usize)
                {
                    let same_item = self
                        .slots
                        .get(id as usize)
                        .and_then(|slot| slot.item())
                        .is_some_and(|held| held.id == moved.id);
                    if same_item {
                        // The vanilla re-click: move the remainder on.
                        self.quick_move_item(ctx, id as usize);
                    }
                    returned = Some(moved);
                }
            } else {
                // PICKUP.
                if id < 0 {
                    return None;
                }
                let slot = self.slots.get(id as usize)?;
                let held = slot.item();
                returned = held.as_ref().map(|held| held.copy());
                let cursor = ctx.inventory.borrow().cursor_item().cloned();
                match (&held, &cursor) {
                    (None, Some(cursor)) => {
                        if slot.is_item_allowed(cursor) {
                            let take = if click_data == 0 { cursor.size } else { 1 };
                            let take = take.min(slot.max_stack_size());
                            if cursor.size >= take {
                                slot.set_item(Some(cursor.split(take)));
                                ctx.inventory.borrow_mut().shrink_cursor_by(take);
                                if ctx
                                    .inventory
                                    .borrow()
                                    .cursor_item()
                                    .is_some_and(|left| left.size <= 0)
                                {
                                    ctx.inventory.borrow_mut().set_cursor_item(None);
                                }
                            }
                        }
                    }
                    (Some(held), None) if slot.can_pick_up() => {
                        let take = if click_data == 0 {
                            held.size
                        } else {
                            (held.size + 1) / 2
                        };
                        if let Some(taken) = slot.remove_item(take) {
                            let emptied = held.size - taken.size <= 0;
                            ctx.inventory.borrow_mut().set_cursor_item(Some(taken));
                            if emptied {
                                slot.set_item(None);
                            }
                            // Vanilla: onItemRemoved fires after the pickup
                            // (the crafting result eats its grid here).
                            slot.on_item_removed(ctx);
                        }
                    }
                    (Some(held), Some(cursor)) if slot.can_pick_up() => {
                        if slot.is_item_allowed(cursor) {
                            if held.id == cursor.id
                                && held.metadata == cursor.metadata
                                && held.matches_nbt(cursor)
                            {
                                let mut add = if click_data == 0 { cursor.size } else { 1 };
                                add = add.min(slot.max_stack_size() - held.size);
                                add = add.min(cursor.max_size(f) - held.size);
                                ctx.inventory.borrow_mut().shrink_cursor_by(add);
                                let mut grown = held.copy();
                                grown.size += add;
                                slot.set_item(Some(grown));
                                if cursor.size - add <= 0 {
                                    ctx.inventory.borrow_mut().set_cursor_item(None);
                                }
                            } else if cursor.size <= slot.max_stack_size() {
                                slot.set_item(Some(cursor.copy()));
                                ctx.inventory
                                    .borrow_mut()
                                    .set_cursor_item(Some(held.copy()));
                            }
                        } else if held.id == cursor.id
                            && cursor.max_size(f) > 1
                            && (!held.has_custom_data(f) || held.metadata == cursor.metadata)
                            && held.matches_nbt(cursor)
                        {
                            let take = held.size;
                            if take > 0 && take + cursor.size <= cursor.max_size(f) {
                                let mut grown = cursor.copy();
                                grown.size += take;
                                ctx.inventory.borrow_mut().set_cursor_item(Some(grown));
                                if let Some(taken) = slot.remove_item(take)
                                    && taken.size == 0
                                {
                                    slot.set_item(None);
                                }
                            }
                        }
                    }
                    _ => {}
                }
                slot.mark_dirty();
            }
        } else if action == action::SWAP && (0..9).contains(&click_data) {
            // Number-key swap, exactly the vanilla ladder.
            let slot = self.slots.get(id as usize)?;
            if slot.can_pick_up() {
                let hotbar = ctx.inventory.borrow().items[click_data as usize].clone();
                let from_player = slot.reads_player_inventory();
                let mut allowed = hotbar.is_none()
                    || (from_player && slot.is_item_allowed(hotbar.as_ref().unwrap()));
                let mut empty = -1;
                if !allowed {
                    empty = ctx.inventory.borrow().empty_slot();
                    allowed |= empty > -1;
                }
                if slot.has_item() && allowed {
                    let held = slot.item().unwrap();
                    // var26 stays the hotbar stack read at the start.
                    let displaced = hotbar.clone();
                    ctx.inventory.borrow_mut().items[click_data as usize] = Some(held.copy());
                    let displaced_not_allowed = displaced.is_some()
                        && (!from_player || !slot.is_item_allowed(displaced.as_ref().unwrap()));
                    if displaced_not_allowed {
                        if empty > -1 {
                            let mut leftover = displaced.unwrap();
                            ctx.inventory
                                .borrow_mut()
                                .add_item(&mut leftover, f, ctx.creative);
                            slot.remove_item(held.size);
                            slot.set_item(None);
                        }
                    } else {
                        slot.remove_item(held.size);
                        slot.set_item(displaced);
                    }
                } else if !slot.has_item()
                    && hotbar.is_some()
                    && slot.is_item_allowed(hotbar.as_ref().unwrap())
                {
                    ctx.inventory.borrow_mut().items[click_data as usize] = None;
                    slot.set_item(hotbar);
                }
            }
        } else if action == action::CLONE
            && ctx.creative
            && ctx.inventory.borrow().cursor_item().is_none()
            && id >= 0
        {
            let slot = self.slots.get(id as usize)?;
            if let Some(held) = slot.item() {
                let mut full = held.copy();
                full.size = full.max_size(f);
                ctx.inventory.borrow_mut().set_cursor_item(Some(full));
            }
        } else if action == action::THROW
            && ctx.inventory.borrow().cursor_item().is_none()
            && id >= 0
        {
            let slot = self.slots.get(id as usize)?;
            if slot.has_item() && slot.can_pick_up() {
                let take = if click_data == 0 {
                    1
                } else {
                    slot.item().unwrap().size
                };
                if let Some(thrown) = slot.remove_item(take) {
                    ctx.drops.push(thrown);
                }
            }
        } else if action == action::PICKUP_ALL && id >= 0 {
            let cursor = ctx.inventory.borrow().cursor_item().cloned();
            if let Some(mut cursor) = cursor {
                let blocked = self
                    .slots
                    .get(id as usize)
                    .is_some_and(|slot| slot.has_item() && slot.can_pick_up());
                if !blocked {
                    // Pickup all into the cursor: non-full stacks first, then the rest.
                    let count = self.slots.len();
                    for round in 0..2 {
                        for position in 0..count {
                            if cursor.size >= cursor.max_size(f) {
                                break;
                            }
                            let Some(slot) = self.slots.get(position) else {
                                continue;
                            };
                            let Some(held) = slot.item() else { continue };
                            if !can_click_drag_into_slot(Some(slot), &cursor)
                                || !slot.can_pick_up()
                                || (round == 0 && held.size == held.max_size(f))
                            {
                                continue;
                            }
                            let take = held.size.min(cursor.max_size(f) - cursor.size);
                            if let Some(taken) = slot.remove_item(take) {
                                cursor.size += taken.size;
                                if slot.item().is_none() {
                                    slot.set_item(None);
                                }
                            }
                        }
                    }
                    ctx.inventory.borrow_mut().set_cursor_item(Some(cursor));
                }
            }
            self.update_listeners();
        }

        returned
    }

    /// `InventoryMenu.canClickDragInto(slot)` (the always-true menu hook).
    fn can_click_drag_into(&self, _slot: &Slot) -> bool {
        true
    }

    /// `InventoryMenu.dropClickDragging`.
    pub fn drop_click_dragging(&mut self) {
        self.click_drag_stage = 0;
        self.click_drag_slots.clear();
    }

    /// `InventoryMenu.canRemoveForPickupAll`: result slots never give to pickup-all.
    pub fn can_remove_for_pickup_all(&self, index: usize) -> bool {
        self.slots[index].kind_id() != SlotId::CraftingResult
    }
}

/// Slot identity for the pickup-all and swap checks.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SlotId {
    Normal,
    CraftingResult,
    FurnaceResult,
    Armor,
}

impl Slot {
    fn kind_id(&self) -> SlotId {
        match &self.kind {
            SlotKind::Normal => SlotId::Normal,
            SlotKind::CraftingResult(..) => SlotId::CraftingResult,
            SlotKind::FurnaceResult(_) => SlotId::FurnaceResult,
            SlotKind::Armor(_) => SlotId::Armor,
        }
    }

    /// `var19.inventory == var6`: whether this slot reads a player inventory.
    pub fn reads_player_inventory(&self) -> bool {
        self.inventory.borrow().is_player_inventory()
    }
}
