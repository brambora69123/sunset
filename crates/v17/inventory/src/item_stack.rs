//! `net.minecraft.item.ItemStack`, 1.7.10.

use crate::registry::ItemFacts;

/// Metadata wildcard carried by recipe and smelting ingredients: matches any metadata.
pub const WILDCARD_METADATA: i32 = 32767;

/// NBT carried by a stack, compared by value.
/// ponytail: opaque text identity, not a full NBT tree; swap for a real NBT
/// type once the save or protocol crate exposes one this crate can use.
pub type Nbt = String;

/// A stack of items. A missing stack is `Option<ItemStack>`, like the Java `null`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ItemStack {
    /// Numeric 1.7.10 item id, `Item.getId`.
    pub id: u16,
    pub size: i32,
    pub metadata: i32,
    pub nbt: Option<Nbt>,
}

impl ItemStack {
    pub fn new(id: u16, size: i32, metadata: i32) -> Self {
        let mut stack = ItemStack {
            id,
            size,
            metadata,
            nbt: None,
        };
        if stack.metadata < 0 {
            stack.metadata = 0;
        }
        stack
    }

    pub fn of(id: u16) -> Self {
        Self::new(id, 1, 0)
    }

    pub fn of_meta(id: u16, metadata: i32) -> Self {
        Self::new(id, 1, metadata)
    }

    /// `ItemStack.split`.
    pub fn split(&self, amount: i32) -> ItemStack {
        let mut split = self.copy();
        split.size = amount;
        split
    }

    /// `ItemStack.copy`.
    pub fn copy(&self) -> ItemStack {
        self.clone()
    }

    /// `ItemStack.getMaxSize`.
    pub fn max_size(&self, f: &dyn ItemFacts) -> i32 {
        f.max_stack_size(self.id)
    }

    /// `ItemStack.isStackable`.
    pub fn is_stackable(&self, f: &dyn ItemFacts) -> bool {
        self.max_size(f) > 1 && (!self.is_damageable(f) || !self.is_damaged(f))
    }

    /// `ItemStack.isDamageable`.
    pub fn is_damageable(&self, f: &dyn ItemFacts) -> bool {
        f.max_damage(self.id) > 0
    }

    /// `ItemStack.isDamaged`.
    pub fn is_damaged(&self, f: &dyn ItemFacts) -> bool {
        self.is_damageable(f) && self.metadata > 0
    }

    /// `ItemStack.setDamage`.
    pub fn set_damage(&mut self, damage: i32) {
        self.metadata = damage;
        if self.metadata < 0 {
            self.metadata = 0;
        }
    }

    /// `ItemStack.hasCustomData`.
    pub fn has_custom_data(&self, f: &dyn ItemFacts) -> bool {
        f.has_custom_data(self.id)
    }

    /// `ItemStack.hasRecipeRemainder`.
    pub fn has_recipe_remainder(&self, f: &dyn ItemFacts) -> bool {
        f.recipe_remainder(self.id).is_some()
    }

    /// `ItemStack.getRecipeRemainder`.
    pub fn recipe_remainder(&self, f: &dyn ItemFacts) -> Option<ItemStack> {
        f.recipe_remainder(self.id).map(ItemStack::of)
    }

    /// `ItemStack.matches` (private `matches(ItemStack)`): size, item, metadata and NBT equal.
    pub fn matches(&self, other: &ItemStack) -> bool {
        self.size == other.size
            && self.id == other.id
            && self.metadata == other.metadata
            && self.matches_nbt(other)
    }

    /// `ItemStack.matchesItem`: item and metadata equal.
    pub fn matches_item(&self, other: &ItemStack) -> bool {
        self.id == other.id && self.metadata == other.metadata
    }

    /// `ItemStack.matchesNbt`.
    pub fn matches_nbt(&self, other: &ItemStack) -> bool {
        match (&self.nbt, &other.nbt) {
            (None, Some(_)) => false,
            (None, None) => true,
            (Some(_), Some(_)) => self.nbt == other.nbt,
            (Some(_), None) => false,
        }
    }
}

/// `ItemStack.matches(ItemStack, ItemStack)` static form.
pub fn matches(a: Option<&ItemStack>, b: Option<&ItemStack>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => a.matches(b),
        _ => false,
    }
}
