//! `net.minecraft.crafting.SmeltingManager`, 1.7.10: the smelting table and xp.

use crate::item_stack::{ItemStack, WILDCARD_METADATA};
use crate::registry::ids;

struct SmeltingEntry {
    input_id: u16,
    input_metadata: i32,
    result: ItemStack,
    xp: f32,
}

/// `net.minecraft.crafting.SmeltingManager`.
pub struct SmeltingManager {
    entries: Vec<SmeltingEntry>,
}

impl Default for SmeltingManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SmeltingManager {
    /// The vanilla 1.7.10 table, in the constructor's registration order.
    pub fn new() -> Self {
        use ids::*;
        let mut manager = SmeltingManager {
            entries: Vec::new(),
        };
        fn register(manager: &mut SmeltingManager, input: u16, result: ItemStack, xp: f32) {
            manager.entries.push(SmeltingEntry {
                input_id: input,
                input_metadata: WILDCARD_METADATA,
                result,
                xp,
            });
        }
        // Blocks first (`register(&mut manager, Block, ..)` → item id, any metadata).
        register(&mut manager, IRON_ORE, ItemStack::of(IRON_INGOT), 0.7);
        register(&mut manager, GOLD_ORE, ItemStack::of(GOLD_INGOT), 1.0);
        register(&mut manager, DIAMOND_ORE, ItemStack::of(DIAMOND), 1.0);
        register(&mut manager, SAND, ItemStack::of(GLASS), 0.1);
        register(&mut manager, COBBLESTONE, ItemStack::of(STONE), 0.1);
        register(&mut manager, CLAY_BLOCK, ItemStack::of(HARDENED_CLAY), 0.35);
        register(&mut manager, CACTUS, ItemStack::of_meta(DYE, 2), 0.2);
        register(&mut manager, LOG, ItemStack::of_meta(COAL, 1), 0.15);
        register(&mut manager, LOG2, ItemStack::of_meta(COAL, 1), 0.15);
        register(&mut manager, EMERALD_ORE, ItemStack::of(EMERALD), 1.0);
        register(&mut manager, NETHERRACK, ItemStack::of(NETHERBRICK), 0.1);
        // Cookable fish: cod, salmon, clownfish (`FishItem$Type.canBeCooked`).
        for metadata in 0..3 {
            manager.entries.push(SmeltingEntry {
                input_id: FISH,
                input_metadata: metadata,
                result: ItemStack::of_meta(COOKED_FISH, metadata),
                xp: 0.35,
            });
        }
        register(&mut manager, COAL_ORE, ItemStack::of(COAL), 0.1);
        register(&mut manager, REDSTONE_ORE, ItemStack::of(REDSTONE), 0.7);
        register(&mut manager, LAPIS_ORE, ItemStack::of_meta(DYE, 4), 0.2);
        register(&mut manager, QUARTZ_ORE, ItemStack::of(QUARTZ), 0.2);
        // Items (`register(&mut manager, Item, ..)` → any metadata).
        register(&mut manager, PORKCHOP, ItemStack::of(COOKED_PORKCHOP), 0.35);
        register(&mut manager, BEEF, ItemStack::of(COOKED_BEEF), 0.35);
        register(&mut manager, CHICKEN, ItemStack::of(COOKED_CHICKEN), 0.35);
        register(&mut manager, CLAY_BALL, ItemStack::of(BRICK), 0.3);
        register(&mut manager, POTATO, ItemStack::of(BAKED_POTATO), 0.35);
        manager
    }

    /// `SmeltingManager.getResult(ItemStack)`: the first entry whose input matches.
    pub fn get_result(&self, input: &ItemStack) -> Option<ItemStack> {
        self.entries
            .iter()
            .find(|entry| entry.matches_input(input))
            .map(|entry| entry.result.copy())
    }

    /// `SmeltingManager.getXp(ItemStack)`: looks the rate up by the *result*
    /// stack (`xpResults.put(result, xp)`).
    pub fn get_xp(&self, result: &ItemStack) -> f32 {
        self.entries
            .iter()
            .find(|entry| entry.result.matches_item(result))
            .map(|entry| entry.xp)
            .unwrap_or(0.0)
    }

    /// `SmeltingManager.getRecipes`.
    pub fn recipes(&self) -> impl Iterator<Item = (&u16, &ItemStack)> {
        self.entries
            .iter()
            .map(|entry| (&entry.input_id, &entry.result))
    }
}

impl SmeltingEntry {
    /// `SmeltingManager.matchesInput`: item id equal, `32767` matches any metadata.
    fn matches_input(&self, item: &ItemStack) -> bool {
        self.input_id == item.id
            && (self.input_metadata == WILDCARD_METADATA || self.input_metadata == item.metadata)
    }
}
