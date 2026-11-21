//! `net.minecraft.inventory`: the `Inventory` contract and its 1.7.10
//! implementations the menus read.

use crate::item_stack::ItemStack;
use crate::registry::ItemFacts;
use std::cell::RefCell;
use std::rc::Rc;

/// `net.minecraft.inventory.Inventory`. A missing stack is `None`, the Java `null`.
pub trait Inventory {
    fn size(&self) -> usize;
    fn get_item(&self, slot: usize) -> Option<&ItemStack>;
    fn set_item(&mut self, slot: usize, item: Option<ItemStack>);
    /// `Inventory.removeItem(slot, amount)`.
    fn remove_item(&mut self, slot: usize, amount: i32) -> Option<ItemStack>;
    /// `Inventory.removeItemQuietly(slot)`.
    fn remove_item_quietly(&mut self, slot: usize) -> Option<ItemStack>;
    /// `Inventory.getMaxStackSize`.
    fn max_stack_size(&self) -> i32 {
        64
    }
    /// `Inventory.isItemAllowed(slot, item)` — `SidedInventory` and friends.
    fn is_item_allowed(&self, _slot: usize, _item: &ItemStack) -> bool {
        true
    }
    /// `Inventory.markDirty`.
    fn mark_dirty(&mut self) {}
    /// `Inventory.isValid`: distance checks live in the caller's glue.
    fn is_valid(&self) -> bool {
        true
    }
    /// `var19.inventory == var6`: the menus compare this against the player's.
    fn is_player_inventory(&self) -> bool {
        false
    }
}

/// `net.minecraft.entity.living.player.PlayerInventory`: 36 carried slots
/// (0-8 hotbar, 9-35 main) plus 4 armor slots, no offhand.
pub struct PlayerInventory {
    pub items: [Option<ItemStack>; 36],
    pub armor: [Option<ItemStack>; 4],
    pub selected_slot: usize,
    cursor_item: Option<ItemStack>,
    pub dirty: bool,
}

impl PlayerInventory {
    pub fn new() -> Self {
        PlayerInventory {
            items: std::array::from_fn(|_| None),
            armor: [
                const { None },
                const { None },
                const { None },
                const { None },
            ],
            selected_slot: 0,
            cursor_item: None,
            dirty: false,
        }
    }

    pub const HOTBAR_SIZE: usize = 9;

    /// `PlayerInventory.getSelectedItem`.
    pub fn selected_item(&self) -> Option<&ItemStack> {
        if self.selected_slot < 9 {
            self.items[self.selected_slot].as_ref()
        } else {
            None
        }
    }

    /// `PlayerInventory.getArmor`.
    pub fn armor(&self, slot: usize) -> Option<&ItemStack> {
        self.armor[slot].as_ref()
    }

    /// `PlayerInventory.setCursorItem`.
    pub fn set_cursor_item(&mut self, item: Option<ItemStack>) {
        self.cursor_item = item;
    }

    /// `PlayerInventory.getCursorItem`.
    pub fn cursor_item(&self) -> Option<&ItemStack> {
        self.cursor_item.as_ref()
    }

    /// Takes the cursor stack (the `setCursorItem(null)` + read pair).
    pub fn take_cursor(&mut self) -> Option<ItemStack> {
        self.cursor_item.take()
    }

    /// Removes `amount` from the cursor (`var33.split(var37)`).
    pub fn shrink_cursor_by(&mut self, amount: i32) {
        if let Some(cursor) = self.cursor_item.as_mut() {
            cursor.size -= amount;
        }
    }

    /// `PlayerInventory.getEmptySlot`: first empty carried slot, armor excluded.
    pub fn empty_slot(&self) -> i32 {
        for (i, slot) in self.items.iter().enumerate() {
            if slot.is_none() {
                return i as i32;
            }
        }
        -1
    }

    fn slot_with_space(&self, item: &ItemStack, f: &dyn ItemFacts) -> i32 {
        for (i, slot) in self.items.iter().enumerate() {
            if let Some(held) = slot
                && held.id == item.id
                && held.is_stackable(f)
                && held.size < held.max_size(f)
                && held.size < self.max_stack_size()
                && (!held.has_custom_data(f) || held.metadata == item.metadata)
                && held.matches_nbt(item)
            {
                return i as i32;
            }
        }
        -1
    }

    /// `PlayerInventory.addItemStack`: merge into a matching stack, else an
    /// empty slot; returns the size still left over.
    fn add_item_stack(&mut self, item: &ItemStack, f: &dyn ItemFacts) -> i32 {
        if item.max_size(f) == 1 {
            let slot = self.empty_slot();
            if slot < 0 {
                return item.size;
            }
            let slot = slot as usize;
            if self.items[slot].is_none() {
                self.items[slot] = Some(item.copy());
            }
            return 0;
        }
        let mut slot = self.slot_with_space(item, f);
        if slot < 0 {
            slot = self.empty_slot();
        }
        if slot < 0 {
            return item.size;
        }
        let slot = slot as usize;
        if self.items[slot].is_none() {
            let mut fresh = ItemStack::new(item.id, 0, item.metadata);
            fresh.nbt = item.nbt.clone();
            self.items[slot] = Some(fresh);
        }
        let mut add = {
            let held = self.items[slot].as_ref().unwrap();
            let mut add = item.size;
            if add > held.max_size(f) - held.size {
                add = held.max_size(f) - held.size;
            }
            add
        };
        if add > self.max_stack_size() - self.items[slot].as_ref().unwrap().size {
            add = self.max_stack_size() - self.items[slot].as_ref().unwrap().size;
        }
        if add == 0 {
            return item.size;
        }
        let held = self.items[slot].as_mut().unwrap();
        held.size += add;
        item.size - add
    }

    /// `PlayerInventory.addItem`: takes the whole item or reports what stayed.
    pub fn add_item(&mut self, item: &mut ItemStack, f: &dyn ItemFacts, creative: bool) -> bool {
        if item.size == 0 {
            return false;
        }
        if item.is_damaged(f) {
            let slot = self.empty_slot();
            if slot >= 0 {
                self.items[slot as usize] = Some(item.copy());
                item.size = 0;
                true
            } else if creative {
                item.size = 0;
                true
            } else {
                false
            }
        } else {
            let mut before;
            loop {
                before = item.size;
                item.size = self.add_item_stack(item, f);
                if !(item.size > 0 && item.size < before) {
                    break;
                }
            }
            if item.size == before && creative {
                item.size = 0;
                return true;
            }
            item.size < before
        }
    }

    /// `PlayerInventory.dropAll`: every carried and armor stack, `drops` sinks them.
    pub fn drop_all(&mut self, drops: &mut Vec<ItemStack>) {
        for slot in self.items.iter_mut().chain(self.armor.iter_mut()) {
            if let Some(item) = slot.take() {
                drops.push(item);
            }
        }
    }
}

impl Default for PlayerInventory {
    fn default() -> Self {
        Self::new()
    }
}

impl Inventory for PlayerInventory {
    fn size(&self) -> usize {
        self.items.len() + 4
    }

    fn get_item(&self, slot: usize) -> Option<&ItemStack> {
        if slot >= self.items.len() {
            self.armor[slot - self.items.len()].as_ref()
        } else {
            self.items[slot].as_ref()
        }
    }

    fn set_item(&mut self, slot: usize, item: Option<ItemStack>) {
        if slot >= self.items.len() {
            self.armor[slot - self.items.len()] = item;
        } else {
            self.items[slot] = item;
        }
    }

    fn remove_item(&mut self, slot: usize, amount: i32) -> Option<ItemStack> {
        if slot >= self.items.len() {
            let slot = slot - self.items.len();
            let held = self.armor[slot].take();
            return held;
        }
        let held = self.items[slot].as_mut()?;
        if held.size <= amount {
            self.items[slot].take()
        } else {
            let split = held.split(amount);
            held.size -= amount;
            if held.size == 0 {
                self.items[slot] = None;
            }
            Some(split)
        }
    }

    fn remove_item_quietly(&mut self, slot: usize) -> Option<ItemStack> {
        if slot >= self.items.len() {
            self.armor[slot - self.items.len()].take()
        } else {
            self.items[slot].take()
        }
    }

    fn max_stack_size(&self) -> i32 {
        64
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    fn is_player_inventory(&self) -> bool {
        true
    }
}

/// `net.minecraft.inventory.SimpleInventory`: a plain fixed-size inventory
/// (single chests, and everything the menus store by hand).
pub struct SimpleInventory {
    items: Vec<Option<ItemStack>>,
    max_stack_size: i32,
}

impl SimpleInventory {
    pub fn new(size: usize) -> Self {
        SimpleInventory {
            items: vec![None; size],
            max_stack_size: 64,
        }
    }

    pub fn with_max_stack_size(size: usize, max_stack_size: i32) -> Self {
        SimpleInventory {
            items: vec![None; size],
            max_stack_size,
        }
    }
}

impl Inventory for SimpleInventory {
    fn size(&self) -> usize {
        self.items.len()
    }

    fn get_item(&self, slot: usize) -> Option<&ItemStack> {
        self.items.get(slot).and_then(|slot| slot.as_ref())
    }

    fn set_item(&mut self, slot: usize, item: Option<ItemStack>) {
        if slot < self.items.len() {
            self.items[slot] = item;
        }
    }

    fn remove_item(&mut self, slot: usize, amount: i32) -> Option<ItemStack> {
        let held = self.items.get_mut(slot)?.as_mut()?;
        if held.size <= amount {
            self.items[slot].take()
        } else {
            let split = held.split(amount);
            held.size -= amount;
            if held.size == 0 {
                self.items[slot] = None;
            }
            Some(split)
        }
    }

    fn remove_item_quietly(&mut self, slot: usize) -> Option<ItemStack> {
        self.items.get_mut(slot).and_then(|slot| slot.take())
    }

    fn max_stack_size(&self) -> i32 {
        self.max_stack_size
    }
}

/// `net.minecraft.inventory.CraftingInventory`: the crafting grid; every
/// change recomputes the result through the hook, `menu.onContentsChanged`.
pub struct CraftingInventory {
    items: Vec<Option<ItemStack>>,
    width: usize,
    height: usize,
    on_change: RefCell<Option<Rc<dyn Fn(&CraftingInventory)>>>,
}

impl CraftingInventory {
    pub fn new(width: usize, height: usize) -> Self {
        CraftingInventory {
            items: vec![None; width * height],
            width,
            height,
            on_change: RefCell::new(None),
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    /// `CraftingInventory.getItem(column, row)`: out-of-grid columns read `None`.
    pub fn get(&self, column: usize, row: usize) -> Option<&ItemStack> {
        if column < self.width && row < self.height {
            self.items[column + row * self.width].as_ref()
        } else {
            None
        }
    }

    /// Installs the result recompute, `new CraftingInventory(this, ..)`.
    pub fn set_on_change(&mut self, hook: Rc<dyn Fn(&CraftingInventory)>) {
        self.on_change = RefCell::new(Some(hook));
    }

    fn fire_on_change(&self) {
        let hook = self.on_change.borrow().clone();
        if let Some(hook) = hook {
            hook(self);
        }
    }
}

impl Inventory for CraftingInventory {
    fn size(&self) -> usize {
        self.items.len()
    }

    fn get_item(&self, slot: usize) -> Option<&ItemStack> {
        self.items.get(slot).and_then(|slot| slot.as_ref())
    }

    fn set_item(&mut self, slot: usize, item: Option<ItemStack>) {
        if slot < self.items.len() {
            self.items[slot] = item;
        }
        self.fire_on_change();
    }

    fn remove_item(&mut self, slot: usize, amount: i32) -> Option<ItemStack> {
        let taken = match self.items.get_mut(slot).and_then(|slot| slot.as_mut()) {
            Some(held) => {
                if held.size <= amount {
                    self.items[slot].take()
                } else {
                    let split = held.split(amount);
                    held.size -= amount;
                    if held.size == 0 {
                        self.items[slot] = None;
                    }
                    Some(split)
                }
            }
            None => None,
        };
        self.fire_on_change();
        taken
    }

    fn remove_item_quietly(&mut self, slot: usize) -> Option<ItemStack> {
        self.items.get_mut(slot).and_then(|slot| slot.take())
    }

    fn max_stack_size(&self) -> i32 {
        64
    }
}

/// `net.minecraft.inventory.ResultInventory`: one result slot, no storage side.
pub struct ResultInventory {
    item: Option<ItemStack>,
}

impl ResultInventory {
    pub fn new() -> Self {
        ResultInventory { item: None }
    }
}

impl Default for ResultInventory {
    fn default() -> Self {
        Self::new()
    }
}

impl Inventory for ResultInventory {
    fn size(&self) -> usize {
        1
    }

    fn get_item(&self, _slot: usize) -> Option<&ItemStack> {
        self.item.as_ref()
    }

    fn set_item(&mut self, _slot: usize, item: Option<ItemStack>) {
        self.item = item;
    }

    fn remove_item(&mut self, _slot: usize, _amount: i32) -> Option<ItemStack> {
        self.item.take()
    }

    fn remove_item_quietly(&mut self, _slot: usize) -> Option<ItemStack> {
        self.item.take()
    }

    fn max_stack_size(&self) -> i32 {
        64
    }
}

/// `net.minecraft.inventory.DoubleInventory`: two inventories read as one.
///
/// The Java class forwards every slot access into one of the two halves;
/// the same rows live in [`crate::menu::Menu::forwarded_slot`] when a menu
/// is built over shared halves (`ChestMenu` over a double chest). This
/// struct stays available for callers that only need size and validity.
pub struct DoubleInventory {
    pub first: Rc<RefCell<dyn Inventory>>,
    pub second: Rc<RefCell<dyn Inventory>>,
}

impl DoubleInventory {
    pub fn new(first: Rc<RefCell<dyn Inventory>>, second: Rc<RefCell<dyn Inventory>>) -> Self {
        DoubleInventory { first, second }
    }

    /// Java-side half split: `slot >= inventory1.getSize()` reads the second.
    pub fn forward(&self, slot: usize) -> (usize, usize) {
        let first_size = self.first.borrow().size();
        if slot >= first_size {
            (1, slot - first_size)
        } else {
            (0, slot)
        }
    }
}

impl Inventory for DoubleInventory {
    fn size(&self) -> usize {
        self.first.borrow().size() + self.second.borrow().size()
    }

    fn get_item(&self, _slot: usize) -> Option<&ItemStack> {
        unreachable!("DoubleInventory holds shared halves; see forward()")
    }

    fn set_item(&mut self, _slot: usize, _item: Option<ItemStack>) {
        unreachable!("DoubleInventory holds shared halves; see forward()")
    }

    fn remove_item(&mut self, _slot: usize, _amount: i32) -> Option<ItemStack> {
        unreachable!("DoubleInventory holds shared halves; see forward()")
    }

    fn remove_item_quietly(&mut self, _slot: usize) -> Option<ItemStack> {
        unreachable!("DoubleInventory holds shared halves; see forward()")
    }

    fn max_stack_size(&self) -> i32 {
        self.first.borrow().max_stack_size()
    }
}
