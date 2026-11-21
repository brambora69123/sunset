//! `net.minecraft.block.entity.FurnaceBlockEntity`: the 1.7.10 furnace tick,
//! fuel times and smelting progress.

use crate::inventory::Inventory;
use crate::item_stack::ItemStack;
use crate::registry::ItemFacts;
use crate::registry::ids;
use crate::smelting::SmeltingManager;

/// `FurnaceBlockEntity`: 3 slots (0 input, 1 fuel, 2 output) and the three
/// public fields (`fuelTime`, `totalFuelTime`, `cookTime`).
pub struct FurnaceBlockEntity {
    pub slots: [Option<ItemStack>; 3],
    /// `fuelTime`: burn ticks left.
    pub fuel_time: i32,
    /// `totalFuelTime`: the burn length of the fuel now burning.
    pub total_fuel_time: i32,
    /// `cookTime`: progress toward the 200-tick smelt.
    pub cook_time: i32,
}

impl Default for FurnaceBlockEntity {
    fn default() -> Self {
        Self::new()
    }
}

impl FurnaceBlockEntity {
    pub fn new() -> Self {
        FurnaceBlockEntity {
            slots: [None, None, None],
            fuel_time: 0,
            total_fuel_time: 0,
            cook_time: 0,
        }
    }

    /// `FurnaceBlockEntity.hasFuel`.
    pub fn has_fuel(&self) -> bool {
        self.fuel_time > 0
    }

    /// `FurnaceBlockEntity.tick`: burn down, ignite fuel, advance cooking.
    /// The `lit` result tells the caller when `FurnaceBlock.updateLitState` runs.
    pub fn tick(&mut self, f: &dyn ItemFacts, smelting: &SmeltingManager) -> bool {
        let was_burning = self.fuel_time > 0;
        let mut changed = false;
        if self.fuel_time > 0 {
            self.fuel_time -= 1;
        }

        if self.fuel_time != 0 || (self.slots[1].is_some() && self.slots[0].is_some()) {
            if self.fuel_time == 0 && self.can_cook(f, smelting) {
                let burn = vanilla_fuel_time(self.slots[1].as_ref().unwrap());
                self.total_fuel_time = burn;
                self.fuel_time = burn;
                if self.fuel_time > 0 {
                    changed = true;
                    if let Some(fuel) = self.slots[1].as_mut() {
                        fuel.size -= 1;
                        if fuel.size == 0 {
                            self.slots[1] = fuel.recipe_remainder(f);
                        }
                    }
                }
            }

            if self.has_fuel() && self.can_cook(f, smelting) {
                self.cook_time += 1;
                if self.cook_time == 200 {
                    self.cook_time = 0;
                    self.finish_cooking(f, smelting);
                    changed = true;
                }
            } else {
                self.cook_time = 0;
            }
        }

        let lit = self.fuel_time > 0;
        if was_burning != lit {
            changed = true;
        }
        changed
    }

    /// `FurnaceBlockEntity.canCook`.
    pub fn can_cook(&self, f: &dyn ItemFacts, smelting: &SmeltingManager) -> bool {
        let Some(input) = self.slots[0].as_ref() else {
            return false;
        };
        let Some(result) = smelting.get_result(input) else {
            return false;
        };
        let Some(output) = self.slots[2].as_ref() else {
            return true;
        };
        if !output.matches_item(&result) {
            return false;
        }
        output.size < self.max_stack_size() && output.size < output.max_size(f)
            || output.size < result.max_size(f)
    }

    /// `FurnaceBlockEntity.finishCooking`.
    pub fn finish_cooking(&mut self, f: &dyn ItemFacts, smelting: &SmeltingManager) {
        if !self.can_cook(f, smelting) {
            return;
        }
        let result = smelting
            .get_result(self.slots[0].as_ref().unwrap())
            .unwrap();
        match self.slots[2].as_mut() {
            None => self.slots[2] = Some(result.copy()),
            Some(output) if output.id == result.id => output.size += 1,
            _ => {}
        }
        if let Some(input) = self.slots[0].as_mut() {
            input.size -= 1;
            if input.size <= 0 {
                self.slots[0] = None;
            }
        }
    }

    /// `FurnaceBlockEntity.readNbt` field set (`BurnTime`, `CookTime` and the
    /// recomputed `totalFuelTime`; NBT itself is the save crate's job).
    pub fn load_state(&mut self, fuel_time: i32, cook_time: i32) {
        self.fuel_time = fuel_time;
        self.cook_time = cook_time;
        self.total_fuel_time = self.slots[1].as_ref().map(vanilla_fuel_time).unwrap_or(0);
    }
}

impl Inventory for FurnaceBlockEntity {
    fn size(&self) -> usize {
        3
    }

    fn get_item(&self, slot: usize) -> Option<&ItemStack> {
        self.slots.get(slot).and_then(|slot| slot.as_ref())
    }

    fn set_item(&mut self, slot: usize, item: Option<ItemStack>) {
        if slot < 3 {
            self.slots[slot] = item;
        }
    }

    fn remove_item(&mut self, slot: usize, amount: i32) -> Option<ItemStack> {
        let held = self.slots.get_mut(slot)?.as_mut()?;
        if held.size <= amount {
            self.slots[slot].take()
        } else {
            let split = held.split(amount);
            held.size -= amount;
            if held.size == 0 {
                self.slots[slot] = None;
            }
            Some(split)
        }
    }

    fn remove_item_quietly(&mut self, slot: usize) -> Option<ItemStack> {
        self.slots.get_mut(slot).and_then(|slot| slot.take())
    }

    fn max_stack_size(&self) -> i32 {
        64
    }

    /// `FurnaceBlockEntity.isItemAllowed`: no insert into the output slot,
    /// fuel only in the fuel slot.
    fn is_item_allowed(&self, slot: usize, item: &ItemStack) -> bool {
        if slot == 2 {
            false
        } else if slot == 1 {
            vanilla_fuel_time(item) > 0
        } else {
            true
        }
    }
}

/// `FurnaceBlockEntity.getFuelTime`, the exact 1.7.10 ladder. Reads only the
/// item id and, for block items, the material table of the era.
pub fn vanilla_fuel_time(item: &ItemStack) -> i32 {
    use ids::*;
    // Block items first (`var1 instanceof BlockItem && Block.byItem != AIR`).
    if is_block_item(item.id) {
        if item.id == WOODEN_SLAB {
            return 150;
        }
        if block_material_is_wood(item.id) {
            return 300;
        }
        if item.id == COAL_BLOCK {
            return 16000;
        }
    }
    // `ToolItem`/`SwordItem`/`HoeItem` with a WOOD tier.
    if is_wood_tool(item.id) {
        return 200;
    }
    match item.id {
        STICK => 100,
        COAL => 1600,
        LAVA_BUCKET => 20000,
        SAPLING => 100,
        BLAZE_ROD => 2400,
        _ => 0,
    }
}

/// `FurnaceBlockEntity.isFuel`.
pub fn is_fuel(item: &ItemStack) -> bool {
    vanilla_fuel_time(item) > 0
}

/// The 1.7.10 block items a fuel stack can be (`Item instanceof BlockItem`).
fn is_block_item(id: u16) -> bool {
    id < 256
}

/// `var2.getMaterial() == Material.WOOD` for the era's block table:
/// every block whose 1.7.10 constructor passes `Material.WOOD`.
fn block_material_is_wood(id: u16) -> bool {
    use ids::*;
    matches!(
        id,
        PLANKS
            | LOG
            | LOG2
            | BOOKSHELF
            | CRAFTING_TABLE
            | CHEST
            | TRAPPED_CHEST
            | FENCE
            | WOODEN_SLAB
            | OAK_STAIRS
            | FENCE_GATE
            | NOTEBLOCK
            | JUKEBOX
            | STANDING_SIGN
            | WALL_SIGN
            | WOODEN_DOOR_BLOCK
            | WOODEN_PRESSURE_PLATE
            | WOODEN_TRAPDOOR
            | WOODEN_BUTTON
            | DAYLIGHT_DETECTOR
            | BROWN_MUSHROOM_BLOCK
            | RED_MUSHROOM_BLOCK
            | SPRUCE_STAIRS
            | BIRCH_STAIRS
            | JUNGLE_STAIRS
            | ACACIA_STAIRS
            | DARK_OAK_STAIRS
            | DOUBLE_WOODEN_SLAB
    )
}

/// `((ToolItem|SwordItem|HoeItem) var1).getTierName().equals("WOOD")`.
fn is_wood_tool(id: u16) -> bool {
    matches!(
        id,
        ids::WOODEN_SWORD
            | ids::WOODEN_SHOVEL
            | ids::WOODEN_PICKAXE
            | ids::WOODEN_AXE
            | ids::WOODEN_HOE
    )
}
