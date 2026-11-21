//! The vanilla 1.7.10 recipe registrations: the `CraftingManager` constructor
//! plus the seven recipe books (tool, weapon, mineral, food, block, armor, dye).

use crate::crafting::{CraftingManager, ShapedRecipe, ShapelessRecipe};
use crate::item_stack::{ItemStack, WILDCARD_METADATA};
use crate::registry::ids;

/// One shaped-recipe ingredient: a plain item, a metadata-specific stack, or
/// a block item (any metadata, `new ItemStack(block, 1, 32767)`).
#[derive(Clone, Copy)]
enum Ing {
    Item(u16),
    Meta(u16, i32),
    Block(u16),
}

fn ing(id: u16) -> Ing {
    Ing::Item(id)
}

fn meta(id: u16, metadata: i32) -> Ing {
    Ing::Meta(id, metadata)
}

/// `registerShaped(result, pattern, key -> ingredient)`.
fn shaped(manager: &mut CraftingManager, result: ItemStack, pattern: &[&str], key: &[(char, Ing)]) {
    let key: Vec<(char, Option<ItemStack>)> =
        key.iter().map(|(c, i)| (*c, Some(i.stack()))).collect();
    manager.register_shaped(ShapedRecipe::parse(pattern, &key, result));
}

fn shapeless(manager: &mut CraftingManager, result: ItemStack, ingredients: &[ItemStack]) {
    manager.register_shapeless(ShapelessRecipe::new(ingredients.to_vec(), result));
}

impl Ing {
    fn stack(&self) -> ItemStack {
        match *self {
            Ing::Item(id) => ItemStack::of(id),
            Ing::Meta(id, m) => ItemStack::new(id, 1, m),
            Ing::Block(id) => ItemStack::new(id, 1, WILDCARD_METADATA),
        }
    }
}

/// The five tool/armor materials: planks, cobblestone, iron, diamond, gold.
const TOOL_MATERIALS: [Ing; 5] = [
    Ing::Block(ids::PLANKS),
    Ing::Block(ids::COBBLESTONE),
    Ing::Item(ids::IRON_INGOT),
    Ing::Item(ids::DIAMOND),
    Ing::Item(ids::GOLD_INGOT),
];

/// `ToolRecipes.register`: pickaxe, shovel, axe, hoe patterns plus shears.
fn tool_recipes(manager: &mut CraftingManager) {
    use ids::*;
    let tools = [
        [
            WOODEN_PICKAXE,
            STONE_PICKAXE,
            IRON_PICKAXE,
            DIAMOND_PICKAXE,
            GOLDEN_PICKAXE,
        ],
        [
            WOODEN_SHOVEL,
            STONE_SHOVEL,
            IRON_SHOVEL,
            DIAMOND_SHOVEL,
            GOLDEN_SHOVEL,
        ],
        [WOODEN_AXE, STONE_AXE, IRON_AXE, DIAMOND_AXE, GOLDEN_AXE],
        [WOODEN_HOE, STONE_HOE, IRON_HOE, DIAMOND_HOE, GOLDEN_HOE],
    ];
    let patterns = [
        ["XXX", " # ", " # "],
        ["X", "#", "#"],
        ["XX", "X#", " #"],
        ["XX", " #", " #"],
    ];
    for (material, material_id) in TOOL_MATERIALS.iter().enumerate() {
        for (pattern, tool_ids) in patterns.iter().zip(tools.iter()) {
            shaped(
                manager,
                ItemStack::of(tool_ids[material]),
                pattern,
                &[('#', ing(STICK)), ('X', *material_id)],
            );
        }
    }
    shaped(
        manager,
        ItemStack::of(SHEARS),
        &[" #", "# "],
        &[('#', ing(IRON_INGOT))],
    );
}

/// `WeaponRecipes.register`: swords, bow, arrow.
fn weapon_recipes(manager: &mut CraftingManager) {
    use ids::*;
    let swords = [
        WOODEN_SWORD,
        STONE_SWORD,
        IRON_SWORD,
        DIAMOND_SWORD,
        GOLDEN_SWORD,
    ];
    for (material, sword) in TOOL_MATERIALS.iter().zip(swords.iter()) {
        shaped(
            manager,
            ItemStack::of(*sword),
            &["X", "X", "#"],
            &[('#', ing(STICK)), ('X', *material)],
        );
    }
    shaped(
        manager,
        ItemStack::of(BOW),
        &[" #X", "# X", " #X"],
        &[('X', ing(STRING)), ('#', ing(STICK))],
    );
    shaped(
        manager,
        ItemStack::new(ARROW, 4, 0),
        &["X", "#", "Y"],
        &[('Y', ing(FEATHER)), ('X', ing(FLINT)), ('#', ing(STICK))],
    );
}

/// `MineralRecipes.register`: the 3x3 blocks, their 1-item reverses, nuggets.
fn mineral_recipes(manager: &mut CraftingManager) {
    use ids::*;
    let blocks: [(u16, ItemStack); 8] = [
        (GOLD_BLOCK, ItemStack::of(GOLD_INGOT)),
        (IRON_BLOCK, ItemStack::of(IRON_INGOT)),
        (DIAMOND_BLOCK, ItemStack::of(DIAMOND)),
        (EMERALD_BLOCK, ItemStack::of(EMERALD)),
        (LAPIS_BLOCK, ItemStack::new(DYE, 9, 4)),
        (REDSTONE_BLOCK, ItemStack::of(REDSTONE)),
        (COAL_BLOCK, ItemStack::new(COAL, 9, 0)),
        (HAY, ItemStack::of(WHEAT)),
    ];
    for (block, material) in blocks {
        shaped(
            manager,
            ItemStack::of(block),
            &["###", "###", "###"],
            &[('#', Ing::Item(material.id))],
        );
        shaped(manager, material, &["#"], &[('#', Ing::Item(block))]);
    }
    shaped(
        manager,
        ItemStack::of(GOLD_INGOT),
        &["###", "###", "###"],
        &[('#', ing(GOLD_NUGGET))],
    );
    shaped(
        manager,
        ItemStack::new(GOLD_NUGGET, 9, 0),
        &["#"],
        &[('#', ing(GOLD_INGOT))],
    );
}

/// `FoodRecipes.register`.
fn food_recipes(manager: &mut CraftingManager) {
    use ids::*;
    shapeless(
        manager,
        ItemStack::of(MUSHROOM_STEW),
        &[
            ItemStack::new(BROWN_MUSHROOM, 1, WILDCARD_METADATA),
            ItemStack::new(RED_MUSHROOM, 1, WILDCARD_METADATA),
            ItemStack::of(BOWL),
        ],
    );
    shaped(
        manager,
        ItemStack::new(COOKIE, 8, 0),
        &["#X#"],
        &[('#', ing(WHEAT)), ('X', meta(DYE, 3))],
    );
    shaped(
        manager,
        ItemStack::of(MELON_BLOCK),
        &["MMM", "MMM", "MMM"],
        &[('M', ing(MELON))],
    );
    shaped(
        manager,
        ItemStack::of(MELON_SEEDS),
        &["M"],
        &[('M', ing(MELON))],
    );
    shaped(
        manager,
        ItemStack::new(PUMPKIN_SEEDS, 4, 0),
        &["M"],
        &[('M', Ing::Block(PUMPKIN))],
    );
    shapeless(
        manager,
        ItemStack::of(PUMPKIN_PIE),
        &[
            ItemStack::new(PUMPKIN, 1, WILDCARD_METADATA),
            ItemStack::of(SUGAR),
            ItemStack::of(EGG),
        ],
    );
    shapeless(
        manager,
        ItemStack::of(FERMENTED_SPIDER_EYE),
        &[
            ItemStack::of(SPIDER_EYE),
            ItemStack::new(BROWN_MUSHROOM, 1, WILDCARD_METADATA),
            ItemStack::of(SUGAR),
        ],
    );
    shapeless(
        manager,
        ItemStack::new(BLAZE_POWDER, 2, 0),
        &[ItemStack::of(BLAZE_ROD)],
    );
    shapeless(
        manager,
        ItemStack::of(MAGMA_CREAM),
        &[ItemStack::of(BLAZE_POWDER), ItemStack::of(SLIME_BALL)],
    );
}

/// `BlockRecipes.register`.
fn block_recipes(manager: &mut CraftingManager) {
    use ids::*;
    shaped(
        manager,
        ItemStack::of(CHEST),
        &["###", "# #", "###"],
        &[('#', Ing::Block(PLANKS))],
    );
    shaped(
        manager,
        ItemStack::of(TRAPPED_CHEST),
        &["#-"],
        &[('#', Ing::Block(CHEST)), ('-', Ing::Block(TRIPWIRE_HOOK))],
    );
    shaped(
        manager,
        ItemStack::of(ENDER_CHEST),
        &["###", "#E#", "###"],
        &[('#', Ing::Block(OBSIDIAN)), ('E', ing(ENDER_EYE))],
    );
    shaped(
        manager,
        ItemStack::of(FURNACE),
        &["###", "# #", "###"],
        &[('#', Ing::Block(COBBLESTONE))],
    );
    shaped(
        manager,
        ItemStack::of(CRAFTING_TABLE),
        &["##", "##"],
        &[('#', Ing::Block(PLANKS))],
    );
    shaped(
        manager,
        ItemStack::of(SANDSTONE),
        &["##", "##"],
        &[('#', meta(SAND, 0))],
    );
    shaped(
        manager,
        ItemStack::new(SANDSTONE, 4, 2),
        &["##", "##"],
        &[('#', Ing::Block(SANDSTONE))],
    );
    shaped(
        manager,
        ItemStack::new(SANDSTONE, 1, 1),
        &["#", "#"],
        &[('#', meta(STONE_SLAB, 1))],
    );
    shaped(
        manager,
        ItemStack::new(QUARTZ_BLOCK, 1, 1),
        &["#", "#"],
        &[('#', meta(STONE_SLAB, 7))],
    );
    shaped(
        manager,
        ItemStack::new(QUARTZ_BLOCK, 2, 2),
        &["#", "#"],
        &[('#', meta(QUARTZ_BLOCK, 0))],
    );
    shaped(
        manager,
        ItemStack::new(STONE_BRICKS, 4, 0),
        &["##", "##"],
        &[('#', Ing::Block(STONE))],
    );
    shaped(
        manager,
        ItemStack::new(IRON_BARS, 16, 0),
        &["###", "###"],
        &[('#', ing(IRON_INGOT))],
    );
    shaped(
        manager,
        ItemStack::new(GLASS_PANE, 16, 0),
        &["###", "###"],
        &[('#', Ing::Block(GLASS))],
    );
    shaped(
        manager,
        ItemStack::of(REDSTONE_LAMP),
        &[" R ", "RGR", " R "],
        &[('R', ing(REDSTONE)), ('G', Ing::Block(GLOWSTONE))],
    );
    shaped(
        manager,
        ItemStack::of(BEACON),
        &["GGG", "GSG", "OOO"],
        &[
            ('G', Ing::Block(GLASS)),
            ('S', ing(NETHER_STAR)),
            ('O', Ing::Block(OBSIDIAN)),
        ],
    );
    shaped(
        manager,
        ItemStack::of(NETHER_BRICK),
        &["NN", "NN"],
        &[('N', ing(NETHERBRICK))],
    );
}

/// `ArmorRecipes.register`.
fn armor_recipes(manager: &mut CraftingManager) {
    use ids::*;
    let materials = [
        ing(LEATHER),
        Ing::Block(FIRE),
        ing(IRON_INGOT),
        ing(DIAMOND),
        ing(GOLD_INGOT),
    ];
    let armor = [
        [
            LEATHER_HELMET,
            CHAINMAIL_HELMET,
            IRON_HELMET,
            DIAMOND_HELMET,
            GOLDEN_HELMET,
        ],
        [
            LEATHER_CHESTPLATE,
            CHAINMAIL_CHESTPLATE,
            IRON_CHESTPLATE,
            DIAMOND_CHESTPLATE,
            GOLDEN_CHESTPLATE,
        ],
        [
            LEATHER_LEGGINGS,
            CHAINMAIL_LEGGINGS,
            IRON_LEGGINGS,
            DIAMOND_LEGGINGS,
            GOLDEN_LEGGINGS,
        ],
        [
            LEATHER_BOOTS,
            CHAINMAIL_BOOTS,
            IRON_BOOTS,
            DIAMOND_BOOTS,
            GOLDEN_BOOTS,
        ],
    ];
    let patterns: [&[&str]; 4] = [
        &["XXX", "X X"],
        &["X X", "XXX", "XXX"],
        &["XXX", "X X", "X X"],
        &["X X", "X X"],
    ];
    for (material, material_id) in materials.iter().enumerate() {
        for (pattern, piece_ids) in patterns.iter().zip(armor.iter()) {
            shaped(
                manager,
                ItemStack::of(piece_ids[material]),
                pattern,
                &[('X', *material_id)],
            );
        }
    }
}

/// `DyeRecipes.register`: wool, stained clay/glass/panes, dye mixes, carpets.
fn dye_recipes(manager: &mut CraftingManager) {
    use ids::*;
    for dye in 0..16 {
        shapeless(
            manager,
            ItemStack::new(WOOL, 1, !dye & 15),
            &[ItemStack::new(DYE, 1, dye), ItemStack::new(WOOL, 1, 0)],
        );
        shaped(
            manager,
            ItemStack::new(STAINED_HARDENED_CLAY, 8, !dye & 15),
            &["###", "#X#", "###"],
            &[('#', Ing::Block(HARDENED_CLAY)), ('X', meta(DYE, dye))],
        );
        shaped(
            manager,
            ItemStack::new(STAINED_GLASS, 8, !dye & 15),
            &["###", "#X#", "###"],
            &[('#', Ing::Block(GLASS)), ('X', meta(DYE, dye))],
        );
        shaped(
            manager,
            ItemStack::new(STAINED_GLASS_PANE, 16, dye),
            &["###", "###"],
            &[('#', meta(STAINED_GLASS, dye))],
        );
    }
    shapeless(
        manager,
        ItemStack::new(DYE, 1, 11),
        &[ItemStack::new(YELLOW_FLOWER, 1, WILDCARD_METADATA)],
    );
    shapeless(
        manager,
        ItemStack::new(DYE, 1, 1),
        &[ItemStack::new(RED_FLOWER, 1, WILDCARD_METADATA)],
    );
    shapeless(manager, ItemStack::new(DYE, 3, 15), &[ItemStack::of(BONE)]);
    let mixes: [(i32, i32, Vec<(i32, i32)>); 12] = [
        (2, 9, vec![(1, 1), (15, 1)]),
        (2, 14, vec![(1, 1), (11, 1)]),
        (2, 10, vec![(2, 1), (15, 1)]),
        (2, 8, vec![(0, 1), (15, 1)]),
        (2, 7, vec![(8, 1), (15, 1)]),
        (3, 7, vec![(0, 1), (15, 1), (15, 1)]),
        (2, 12, vec![(4, 1), (15, 1)]),
        (2, 6, vec![(4, 1), (2, 1)]),
        (2, 5, vec![(4, 1), (1, 1)]),
        (2, 13, vec![(5, 1), (9, 1)]),
        (3, 13, vec![(4, 1), (1, 1), (9, 1)]),
        (4, 13, vec![(4, 1), (1, 1), (1, 1), (15, 1)]),
    ];
    for (count, meta_value, inputs) in mixes {
        let ingredients: Vec<ItemStack> = inputs
            .iter()
            .map(|(dye, n)| ItemStack::new(DYE, *n, *dye))
            .collect();
        shapeless(
            manager,
            ItemStack::new(DYE, count, meta_value),
            &ingredients,
        );
    }
    shapeless(
        manager,
        ItemStack::new(DYE, 2, 11),
        &[ItemStack::new(DOUBLE_PLANT, 1, 0)],
    );
    shapeless(
        manager,
        ItemStack::new(DYE, 2, 13),
        &[ItemStack::new(DOUBLE_PLANT, 1, 1)],
    );
    shapeless(
        manager,
        ItemStack::new(DYE, 2, 1),
        &[ItemStack::new(DOUBLE_PLANT, 1, 4)],
    );
    shapeless(
        manager,
        ItemStack::new(DYE, 2, 9),
        &[ItemStack::new(DOUBLE_PLANT, 1, 5)],
    );
    for wool in 0..16 {
        shaped(
            manager,
            ItemStack::new(CARPET, 3, wool),
            &["##"],
            &[('#', meta(WOOL, wool))],
        );
    }
}

/// The `CraftingManager` constructor's own registrations, in order.
fn manager_recipes(manager: &mut CraftingManager) {
    use ids::*;
    shaped(
        manager,
        ItemStack::new(PLANKS, 4, 0),
        &["#"],
        &[('#', Ing::Block(LOG))],
    );
    shaped(
        manager,
        ItemStack::new(STICK, 4, 0),
        &["#", "#"],
        &[('#', Ing::Block(PLANKS))],
    );
    shaped(
        manager,
        ItemStack::new(TORCH, 4, 0),
        &["C", "#"],
        &[('C', Ing::Item(COAL)), ('#', ing(STICK))],
    );
    shaped(
        manager,
        ItemStack::new(WOODEN_SLAB, 6, 0),
        &["###"],
        &[('#', meta(PLANKS, 0))],
    );
    shaped(
        manager,
        ItemStack::new(WOODEN_SLAB, 6, 1),
        &["###"],
        &[('#', meta(PLANKS, 1))],
    );
    shaped(
        manager,
        ItemStack::new(WOODEN_SLAB, 6, 2),
        &["###"],
        &[('#', meta(PLANKS, 2))],
    );
    shaped(
        manager,
        ItemStack::new(WOODEN_SLAB, 6, 3),
        &["###"],
        &[('#', meta(PLANKS, 3))],
    );
    shaped(
        manager,
        ItemStack::new(WOODEN_SLAB, 6, 4),
        &["###"],
        &[('#', meta(PLANKS, 4))],
    );
    shaped(
        manager,
        ItemStack::new(WOODEN_SLAB, 6, 5),
        &["###"],
        &[('#', meta(PLANKS, 5))],
    );
    shaped(
        manager,
        ItemStack::new(PAPER, 3, 0),
        &["###"],
        &[('#', ing(REEDS))],
    );
    shapeless(
        manager,
        ItemStack::of(BOOK),
        &[
            ItemStack::of(PAPER),
            ItemStack::of(PAPER),
            ItemStack::of(PAPER),
            ItemStack::of(LEATHER),
        ],
    );
    shapeless(
        manager,
        ItemStack::of(WRITABLE_BOOK),
        &[
            ItemStack::of(BOOK),
            ItemStack::new(DYE, 1, 0),
            ItemStack::of(FEATHER),
        ],
    );
    shaped(
        manager,
        ItemStack::new(FENCE, 2, 0),
        &["###", "###"],
        &[('#', ing(STICK))],
    );
    shaped(
        manager,
        ItemStack::new(COBBLESTONE_WALL, 6, 0),
        &["###", "###"],
        &[('#', Ing::Block(COBBLESTONE))],
    );
    shaped(
        manager,
        ItemStack::new(COBBLESTONE_WALL, 6, 1),
        &["###", "###"],
        &[('#', Ing::Block(MOSSY_COBBLESTONE))],
    );
    shaped(
        manager,
        ItemStack::new(NETHER_BRICK_FENCE, 6, 0),
        &["###", "###"],
        &[('#', Ing::Block(NETHER_BRICK))],
    );
    shaped(
        manager,
        ItemStack::of(FENCE_GATE),
        &["#W#", "#W#"],
        &[('#', ing(STICK)), ('W', Ing::Block(PLANKS))],
    );
    shaped(
        manager,
        ItemStack::of(JUKEBOX),
        &["###", "#X#", "###"],
        &[('#', Ing::Block(PLANKS)), ('X', ing(DIAMOND))],
    );
    shaped(
        manager,
        ItemStack::new(LEAD, 2, 0),
        &["~~ ", "~O ", "  ~"],
        &[('~', ing(STRING)), ('O', ing(SLIME_BALL))],
    );
    shaped(
        manager,
        ItemStack::of(NOTEBLOCK),
        &["###", "#X#", "###"],
        &[('#', Ing::Block(PLANKS)), ('X', ing(REDSTONE))],
    );
    shaped(
        manager,
        ItemStack::of(BOOKSHELF),
        &["###", "XXX", "###"],
        &[('#', Ing::Block(PLANKS)), ('X', ing(BOOK))],
    );
    shaped(
        manager,
        ItemStack::of(SNOW_BLOCK),
        &["##", "##"],
        &[('#', ing(SNOWBALL))],
    );
    shaped(
        manager,
        ItemStack::new(SNOW_LAYER, 6, 0),
        &["###"],
        &[('#', Ing::Block(SNOW_BLOCK))],
    );
    shaped(
        manager,
        ItemStack::of(CLAY_BLOCK),
        &["##", "##"],
        &[('#', ing(CLAY_BALL))],
    );
    shaped(
        manager,
        ItemStack::of(BRICKS),
        &["##", "##"],
        &[('#', ing(BRICK))],
    );
    shaped(
        manager,
        ItemStack::of(GLOWSTONE),
        &["##", "##"],
        &[('#', ing(GLOWSTONE_DUST))],
    );
    shaped(
        manager,
        ItemStack::of(QUARTZ_BLOCK),
        &["##", "##"],
        &[('#', ing(QUARTZ))],
    );
    shaped(
        manager,
        ItemStack::of(WOOL),
        &["##", "##"],
        &[('#', ing(STRING))],
    );
    shaped(
        manager,
        ItemStack::of(TNT),
        &["X#X", "#X#", "X#X"],
        &[('X', ing(GUNPOWDER)), ('#', Ing::Block(SAND))],
    );
    shaped(
        manager,
        ItemStack::new(STONE_SLAB, 6, 3),
        &["###"],
        &[('#', Ing::Block(COBBLESTONE))],
    );
    shaped(
        manager,
        ItemStack::new(STONE_SLAB, 6, 0),
        &["###"],
        &[('#', Ing::Block(STONE))],
    );
    shaped(
        manager,
        ItemStack::new(STONE_SLAB, 6, 1),
        &["###"],
        &[('#', Ing::Block(SANDSTONE))],
    );
    shaped(
        manager,
        ItemStack::new(STONE_SLAB, 6, 4),
        &["###"],
        &[('#', Ing::Block(BRICKS))],
    );
    shaped(
        manager,
        ItemStack::new(STONE_SLAB, 6, 5),
        &["###"],
        &[('#', Ing::Block(STONE_BRICKS))],
    );
    shaped(
        manager,
        ItemStack::new(STONE_SLAB, 6, 6),
        &["###"],
        &[('#', Ing::Block(NETHER_BRICK))],
    );
    shaped(
        manager,
        ItemStack::new(STONE_SLAB, 6, 7),
        &["###"],
        &[('#', Ing::Block(QUARTZ_BLOCK))],
    );
    shaped(
        manager,
        ItemStack::new(LADDER, 3, 0),
        &["# #", "# #"],
        &[('#', ing(STICK))],
    );
    shaped(
        manager,
        ItemStack::new(WOODEN_DOOR, 3, 0),
        &["##", "##", "##"],
        &[('#', Ing::Block(PLANKS))],
    );
    shaped(
        manager,
        ItemStack::new(IRON_DOOR, 3, 0),
        &["##", "##", "##"],
        &[('#', ing(IRON_INGOT))],
    );
    shaped(
        manager,
        ItemStack::new(SIGN, 3, 0),
        &["###", "###", " # "],
        &[('#', Ing::Block(PLANKS))],
    );
    shaped(
        manager,
        ItemStack::of(PISTON),
        &["TTT", "#X#", "#R#"],
        &[
            ('#', Ing::Block(COBBLESTONE)),
            ('X', ing(IRON_INGOT)),
            ('R', ing(REDSTONE)),
            ('T', Ing::Block(PLANKS)),
        ],
    );
    shaped(
        manager,
        ItemStack::of(STICKY_PISTON),
        &["S", "P"],
        &[('S', ing(SLIME_BALL)), ('P', Ing::Block(PISTON))],
    );
    shaped(
        manager,
        ItemStack::of(BED),
        &["###", "XXX"],
        &[('#', Ing::Block(WOOL)), ('X', Ing::Block(PLANKS))],
    );
    shaped(
        manager,
        ItemStack::of(ENCHANTING_TABLE),
        &[" B ", "D#D", "###"],
        &[
            ('#', Ing::Block(OBSIDIAN)),
            ('B', ing(BOOK)),
            ('D', ing(DIAMOND)),
        ],
    );
    shaped(
        manager,
        ItemStack::of(ANVIL),
        &["III", " i ", "iii"],
        &[('I', Ing::Block(IRON_BLOCK)), ('i', ing(IRON_INGOT))],
    );
    shapeless(
        manager,
        ItemStack::of(ENDER_EYE),
        &[ItemStack::of(ENDER_PEARL), ItemStack::of(BLAZE_POWDER)],
    );
    shapeless(
        manager,
        ItemStack::new(FIRE_CHARGE, 3, 0),
        &[
            ItemStack::of(GUNPOWDER),
            ItemStack::of(BLAZE_POWDER),
            ItemStack::of(COAL),
        ],
    );
    shaped(
        manager,
        ItemStack::of(DAYLIGHT_DETECTOR),
        &["GGG", "QQQ", "WWW"],
        &[
            ('G', Ing::Block(GLASS)),
            ('Q', ing(QUARTZ)),
            ('W', Ing::Block(WOODEN_SLAB)),
        ],
    );
    shaped(
        manager,
        ItemStack::of(HOPPER),
        &["I I", "ICI", " I "],
        &[('I', ing(IRON_INGOT)), ('C', Ing::Block(CHEST))],
    );
}

/// `CraftingManager.new` with every vanilla 1.7.10 recipe, sorted as the
/// constructor's final `Collections.sort` sorts them.
pub fn vanilla_crafting_manager() -> CraftingManager {
    let mut manager = CraftingManager::new();
    tool_recipes(&mut manager);
    weapon_recipes(&mut manager);
    mineral_recipes(&mut manager);
    food_recipes(&mut manager);
    block_recipes(&mut manager);
    armor_recipes(&mut manager);
    dye_recipes(&mut manager);
    manager_recipes(&mut manager);
    manager.sort_recipes();
    manager
}
