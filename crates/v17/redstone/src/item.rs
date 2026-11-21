//! Items and inventories for the redstone port: `ItemStack`, the container
//! inventories hoppers and dispensers touch, and the transfer helpers from
//! `net.minecraft.block.entity.HopperBlockEntity` and `InventoryMenu`.

use sunset_1_7_core::{LegacyRandom, RandomSource};

/// `net.minecraft.item.ItemStack`. No NBT data in this port; `matches_nbt` is
/// always true. Java `stackSize` is an int and can briefly hit 0.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemStack {
    pub item: u16,
    pub meta: u16,
    pub size: i32,
}

impl ItemStack {
    pub fn new(item: u16, size: i32) -> Self {
        ItemStack {
            item,
            meta: 0,
            size,
        }
    }

    /// `ItemStack.split(int)`.
    pub fn split(&mut self, amount: i32) -> ItemStack {
        let taken = self.size.min(amount.max(0));
        self.size -= taken;
        ItemStack {
            item: self.item,
            meta: self.meta,
            size: taken,
        }
    }

    /// `ItemStack.getMaxStackSize()` (item-specific limits are not ported).
    pub fn max_size(&self) -> i32 {
        64
    }

    pub fn is_empty(&self) -> bool {
        self.size <= 0
    }

    /// `HopperBlockEntity.canMergeItems`: same item, same metadata, room left,
    /// and equal NBT (always true here).
    pub fn can_merge(a: &ItemStack, b: &ItemStack) -> bool {
        a.item == b.item && a.meta == b.meta && a.size <= a.max_size()
    }
}

/// Which container this inventory belongs to; the block at the position says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvKind {
    Hopper,
    Dispenser,
    Dropper,
}

/// A container inventory: 5 hopper slots, 9 dispenser/dropper slots.
///
/// 1.7.10 hoppers are not `SidedInventory` here (`HopperBlockEntity implements
/// Hopper` only), so pushes and pulls walk every slot in order and the side
/// argument never restricts anything.
#[derive(Clone, Debug)]
pub struct Inventory {
    pub kind: InvKind,
    pub slots: Vec<Option<ItemStack>>,
    /// `HopperBlockEntity.transferCooldown` (hoppers only).
    pub transfer_cooldown: i32,
    /// The dispenser entity's own `Random` for `pickNonEmptySlot`.
    pub rng: LegacyRandom,
}

impl Inventory {
    pub fn new(kind: InvKind, seed: i64) -> Self {
        let slots = match kind {
            InvKind::Hopper => 5,
            InvKind::Dispenser | InvKind::Dropper => 9,
        };
        Inventory {
            kind,
            slots: vec![None; slots],
            transfer_cooldown: -1,
            rng: LegacyRandom::new(seed),
        }
    }

    pub fn size(&self) -> usize {
        self.slots.len()
    }

    pub fn get_item(&self, slot: usize) -> Option<&ItemStack> {
        self.slots.get(slot).and_then(|s| s.as_ref())
    }

    /// `setItem`: clamps to the max stack size.
    pub fn set_item(&mut self, slot: usize, mut item: Option<ItemStack>) {
        if let Some(item) = &mut item {
            let max = item.max_size();
            if item.size > max {
                item.size = max;
            }
        }
        if slot < self.slots.len() {
            self.slots[slot] = item;
        }
    }

    /// `removeItem`: takes up to `amount` out of the slot.
    pub fn remove_item(&mut self, slot: usize, amount: i32) -> Option<ItemStack> {
        let stack = self.slots.get(slot)?.as_ref()?;
        if stack.size <= amount {
            Some(self.slots[slot].take().expect("item checked"))
        } else {
            let (item, meta) = (stack.item, stack.meta);
            let taken_size = stack.size.min(amount.max(0));
            let stack = self.slots.get_mut(slot)?.as_mut()?;
            stack.size -= taken_size;
            Some(ItemStack {
                item,
                meta,
                size: taken_size,
            })
        }
    }

    pub fn remove_item_quietly(&mut self, slot: usize) -> Option<ItemStack> {
        self.slots.get_mut(slot).and_then(|s| s.take())
    }

    pub fn is_empty(&self) -> bool {
        self.slots.iter().all(|s| s.is_none())
    }

    /// `HopperBlockEntity.isFull`: every slot present and at its max size.
    pub fn is_full(&self) -> bool {
        self.slots.iter().all(|s| match s {
            Some(stack) => stack.size == stack.max_size(),
            None => false,
        })
    }

    /// `isFullInventory` / `isEmptyInventory` over every slot (non-sided).
    pub fn is_full_inventory(&self) -> bool {
        self.is_full()
    }

    pub fn set_cooldown(&mut self, cooldown: i32) {
        self.transfer_cooldown = cooldown;
    }

    /// `isOnCooldown`: `transferCooldown > 0`.
    pub fn is_on_cooldown(&self) -> bool {
        self.transfer_cooldown > 0
    }

    /// `DispenserBlockEntity.pickNonEmptySlot`: reservoir sampling over the
    /// entity's own random.
    pub fn pick_non_empty_slot(&mut self) -> i32 {
        let mut picked = -1;
        let mut bound = 1;
        for slot in 0..self.slots.len() {
            if self.slots[slot].is_some() {
                let draw = self.rng.next_int_bound(bound).unwrap_or(0);
                if draw == 0 {
                    picked = slot as i32;
                }
                bound += 1;
            }
        }
        picked
    }

    /// `InventoryMenu.getAnalogSignal` for a comparator reading.
    pub fn get_analog_signal(&self) -> i32 {
        let size = self.slots.len();
        if size == 0 {
            return 0;
        }
        let mut count = 0;
        let mut total = 0.0f32;
        for stack in self.slots.iter().flatten() {
            total += stack.size as f32 / (64f32.min(stack.max_size() as f32));
            count += 1;
        }
        total /= size as f32;
        (total * 14.0).floor() as i32 + if count > 0 { 1 } else { 0 }
    }

    /// `HopperBlockEntity.pushItem(Inventory, ItemStack, int)`: fills and merges
    /// across every slot; returns what did not fit (empty stack = all placed).
    pub fn push_item(&mut self, mut item: ItemStack, _side: i32) -> Option<ItemStack> {
        for slot in 0..self.slots.len() {
            if item.is_empty() {
                break;
            }
            item = self.push_item_to_slot(item, slot);
        }
        if item.is_empty() { None } else { Some(item) }
    }

    /// `pushItem(Inventory, ItemStack, int slot, int side)` for one slot.
    fn push_item_to_slot(&mut self, mut item: ItemStack, slot: usize) -> ItemStack {
        let mut changed = false;
        let mut merged = None;
        match &self.slots[slot] {
            None => {
                let mut placed = item.clone();
                let max = placed.max_size();
                if placed.size > max {
                    placed.size = max;
                }
                self.slots[slot] = Some(placed);
                item.size = 0;
                changed = true;
            }
            Some(existing) if ItemStack::can_merge(existing, &item) => {
                let existing_size = existing.size;
                let room = item.max_size() - existing_size;
                let moved = item.size.min(room);
                item.size -= moved;
                if moved > 0 {
                    merged = Some(existing_size + moved);
                    changed = true;
                }
            }
            _ => {}
        }
        if let Some(size) = merged {
            self.slots[slot] = Some(ItemStack {
                item: item.item,
                meta: item.meta,
                size,
            });
        }
        if changed && self.kind == InvKind::Hopper {
            self.set_cooldown(8);
        }
        item
    }
}
