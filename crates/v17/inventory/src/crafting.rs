//! `net.minecraft.crafting`: `CraftingManager`, `ShapedRecipe`, `ShapelessRecipe`
//! and the two-tool repair path in `CraftingManager.getResult`.

use crate::inventory::{CraftingInventory, Inventory};
use crate::item_stack::{ItemStack, WILDCARD_METADATA};
use crate::registry::ItemFacts;

/// `net.minecraft.crafting.recipe.Recipe`.
pub trait Recipe {
    /// `Recipe.matches`.
    fn matches(&self, inventory: &CraftingInventory, f: &dyn ItemFacts) -> bool;
    /// `Recipe.getResult(CraftingInventory)`.
    fn result(&self, inventory: &CraftingInventory, f: &dyn ItemFacts) -> Option<ItemStack>;
    /// `Recipe.getResult` (the stored output).
    fn output(&self) -> &ItemStack;
    /// `Recipe.size`.
    fn size(&self) -> usize;
    /// Java instanceof for the stable sort (`CraftingManager$57536588`).
    fn is_shapeless(&self) -> bool;
}

/// `net.minecraft.crafting.recipe.ShapedRecipe`.
pub struct ShapedRecipe {
    width: usize,
    height: usize,
    /// Row-major `width * height` ingredients; `None` is a required-empty cell.
    ingredients: Vec<Option<ItemStack>>,
    result: ItemStack,
}

impl ShapedRecipe {
    /// `new ShapedRecipe(width, height, ingredients, result)`.
    pub fn new(
        width: usize,
        height: usize,
        ingredients: Vec<Option<ItemStack>>,
        result: ItemStack,
    ) -> Self {
        ShapedRecipe {
            width,
            height,
            ingredients,
            result,
        }
    }

    /// `CraftingManager.registerShaped(result, pattern, key -> ingredient)`.
    /// Each entry maps one symbol to its ingredient stack (`None` = the
    /// required-empty `' '` cells); symbols with no entry are empty too.
    pub fn parse(pattern: &[&str], key: &[(char, Option<ItemStack>)], result: ItemStack) -> Self {
        let height = pattern.len();
        let width = pattern.first().map(|row| row.len()).unwrap_or(0);
        let mut ingredients = vec![None; width * height];
        for (row, line) in pattern.iter().enumerate() {
            for (column, symbol) in line.chars().enumerate() {
                let index = column + row * width;
                if let Some((_, ingredient)) = key.iter().find(|(c, _)| *c == symbol) {
                    ingredients[index] = ingredient.clone();
                }
            }
        }
        ShapedRecipe::new(width, height, ingredients, result)
    }

    /// `ShapedRecipe.matches(inventory, x, y, rotate)`: the pattern anchored at
    /// `(x, y)`, optionally mirrored; always scanned over the 3x3 inventory.
    fn matches_at(
        &self,
        inventory: &CraftingInventory,
        offset_x: usize,
        offset_y: usize,
        mirrored: bool,
    ) -> bool {
        for row in 0..3usize {
            for column in 0..3usize {
                let pattern_x = column as i32 - offset_x as i32;
                let pattern_y = row as i32 - offset_y as i32;
                let mut ingredient: Option<&ItemStack> = None;
                if pattern_x >= 0
                    && pattern_y >= 0
                    && (pattern_x as usize) < self.width
                    && (pattern_y as usize) < self.height
                {
                    let (px, py) = (pattern_x as usize, pattern_y as usize);
                    let index = if mirrored {
                        self.width - px - 1 + py * self.width
                    } else {
                        px + py * self.width
                    };
                    ingredient = self.ingredients[index].as_ref();
                }
                let cell = inventory.get(column, row);
                match (cell, ingredient) {
                    (None, None) => {}
                    (Some(_), None) | (None, Some(_)) => return false,
                    (Some(cell), Some(ingredient)) => {
                        if cell.id != ingredient.id {
                            return false;
                        }
                        if ingredient.metadata != WILDCARD_METADATA
                            && ingredient.metadata != cell.metadata
                        {
                            return false;
                        }
                    }
                }
            }
        }
        true
    }
}

impl Recipe for ShapedRecipe {
    /// `ShapedRecipe.matches`: every anchor, both mirrors. 1.7.10 reads no
    /// item facts here (the world arg is unused too), so `f` is ignored.
    fn matches(&self, inventory: &CraftingInventory, _f: &dyn ItemFacts) -> bool {
        for offset_x in 0..=3 - self.width {
            for offset_y in 0..=3 - self.height {
                if self.matches_at(inventory, offset_x, offset_y, true) {
                    return true;
                }
                if self.matches_at(inventory, offset_x, offset_y, false) {
                    return true;
                }
            }
        }
        false
    }

    /// `ShapedRecipe.getResult(CraftingInventory)`. 1.7.10 has no
    /// `copyNbt` recipe registered, so the plain copy is the whole behavior.
    fn result(&self, _inventory: &CraftingInventory, _f: &dyn ItemFacts) -> Option<ItemStack> {
        Some(self.result.copy())
    }

    fn output(&self) -> &ItemStack {
        &self.result
    }

    fn size(&self) -> usize {
        self.width * self.height
    }

    fn is_shapeless(&self) -> bool {
        false
    }
}

/// `net.minecraft.crafting.recipe.ShapelessRecipe`.
pub struct ShapelessRecipe {
    ingredients: Vec<ItemStack>,
    result: ItemStack,
}

impl ShapelessRecipe {
    pub fn new(ingredients: Vec<ItemStack>, result: ItemStack) -> Self {
        ShapelessRecipe {
            ingredients,
            result,
        }
    }
}

impl Recipe for ShapelessRecipe {
    /// `ShapelessRecipe.matches`: each grid cell consumes one matching
    /// ingredient; every ingredient must be consumed.
    fn matches(&self, inventory: &CraftingInventory, _f: &dyn ItemFacts) -> bool {
        let mut remaining: Vec<&ItemStack> = self.ingredients.iter().collect();
        for row in 0..3usize {
            for column in 0..3usize {
                if let Some(cell) = inventory.get(column, row) {
                    let found = remaining.iter().position(|ingredient| {
                        cell.id == ingredient.id
                            && (ingredient.metadata == WILDCARD_METADATA
                                || cell.metadata == ingredient.metadata)
                    });
                    match found {
                        Some(i) => {
                            remaining.remove(i);
                        }
                        None => return false,
                    }
                }
            }
        }
        remaining.is_empty()
    }

    fn result(&self, _inventory: &CraftingInventory, _f: &dyn ItemFacts) -> Option<ItemStack> {
        Some(self.result.copy())
    }

    fn output(&self) -> &ItemStack {
        &self.result
    }

    fn size(&self) -> usize {
        self.ingredients.len()
    }

    fn is_shapeless(&self) -> bool {
        true
    }
}

/// `net.minecraft.crafting.CraftingManager`: the recipe list and `getResult`.
pub struct CraftingManager {
    recipes: Vec<Box<dyn Recipe>>,
}

impl Default for CraftingManager {
    fn default() -> Self {
        Self::new()
    }
}

impl CraftingManager {
    pub fn new() -> Self {
        CraftingManager {
            recipes: Vec::new(),
        }
    }

    /// `CraftingManager.registerShaped`.
    pub fn register_shaped(&mut self, recipe: ShapedRecipe) {
        self.recipes.push(Box::new(recipe));
    }

    /// `CraftingManager.registerShapeless`.
    pub fn register_shapeless(&mut self, recipe: ShapelessRecipe) {
        self.recipes.push(Box::new(recipe));
    }

    /// The end-of-constructor `Collections.sort`: shapeless after shaped, then
    /// by `size()` descending, insertion order kept for ties.
    pub fn sort_recipes(&mut self) {
        // Java's Collections.sort is stable; sort_by keeps insertion order on ties.
        self.recipes
            .sort_by(|a, b| match (a.is_shapeless(), b.is_shapeless()) {
                (false, true) => std::cmp::Ordering::Less,
                (true, false) => std::cmp::Ordering::Greater,
                _ => b.size().cmp(&a.size()),
            });
    }

    /// `CraftingManager.getResult(inventory, world)`: the two-tool repair
    /// special case first, then the recipe list in order.
    pub fn get_result(
        &self,
        inventory: &CraftingInventory,
        f: &dyn ItemFacts,
    ) -> Option<ItemStack> {
        let mut count = 0;
        let mut first: Option<&ItemStack> = None;
        let mut second: Option<&ItemStack> = None;
        for slot in 0..inventory.size() {
            if let Some(item) = inventory.get_item(slot) {
                if count == 0 {
                    first = Some(item);
                }
                if count == 1 {
                    second = Some(item);
                }
                count += 1;
            }
        }

        if count == 2
            && first.zip(second).is_some_and(|(a, b)| a.id == b.id)
            && first.is_some_and(|a| a.size == 1)
            && second.is_some_and(|a| a.size == 1)
            && first.is_some_and(|a| a.is_damageable(f))
        {
            let (a, b) = first.zip(second).unwrap();
            let max_damage = f.max_damage(a.id);
            let life_a = max_damage - a.metadata;
            let life_b = max_damage - b.metadata;
            let mut repaired = life_a + life_b + max_damage * 5 / 100;
            repaired = max_damage - repaired;
            if repaired < 0 {
                repaired = 0;
            }
            return Some(ItemStack::new(a.id, 1, repaired));
        }

        for recipe in &self.recipes {
            if recipe.matches(inventory, f) {
                return recipe.result(inventory, f);
            }
        }
        None
    }

    /// `CraftingManager.getRecipes`.
    pub fn recipes(&self) -> &[Box<dyn Recipe>] {
        &self.recipes
    }
}
