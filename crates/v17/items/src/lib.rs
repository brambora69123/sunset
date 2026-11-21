//! Minecraft 1.7.10 items: the `Item.REGISTRY` built by `net/minecraft/item/Items.java`
//! and `Item.init()`, ported for sunset. See docs/ for the port contract.

mod block_items;
mod plain;
pub mod tier;

pub use tier::{ArmorTier, ToolTier};

use block_items::BLOCK_ITEMS;
use plain::PLAIN;

/// `CreativeModeTab`: the creative inventory tabs, in tab order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CreativeTab {
    BuildingBlocks,
    Decorations,
    Redstone,
    Transportation,
    Misc,
    Search,
    Food,
    Tools,
    Combat,
    Brewing,
    Materials,
    Inventory,
}

impl CreativeTab {
    /// The tab's position in `CreativeModeTab.ALL`.
    pub const fn id(self) -> i32 {
        match self {
            Self::BuildingBlocks => 0,
            Self::Decorations => 1,
            Self::Redstone => 2,
            Self::Transportation => 3,
            Self::Misc => 4,
            Self::Search => 5,
            Self::Food => 6,
            Self::Tools => 7,
            Self::Combat => 8,
            Self::Brewing => 9,
            Self::Materials => 10,
            Self::Inventory => 11,
        }
    }

    /// The tab's registry name.
    pub const fn key(self) -> &'static str {
        match self {
            Self::BuildingBlocks => "buildingBlocks",
            Self::Decorations => "decorations",
            Self::Redstone => "redstone",
            Self::Transportation => "transportation",
            Self::Misc => "misc",
            Self::Search => "search",
            Self::Food => "food",
            Self::Tools => "tools",
            Self::Combat => "combat",
            Self::Brewing => "brewing",
            Self::Materials => "materials",
            Self::Inventory => "inventory",
        }
    }
}

/// The tool side of `Item$Tier`: which tool a tool item is (`PickaxeItem`, `ShovelItem`, `AxeItem`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ToolClass {
    Pickaxe,
    Shovel,
    Axe,
}

/// The `BlockItem` subclasses `Item.init()`'s block loop builds, by class.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BlockItemKind {
    /// Plain `BlockItem`.
    Block,
    /// `ColoredBlockItem`: dyed colors, metadata passes through (`wool`, `carpet`...).
    Colored,
    /// `VariantBlockItem`: metadata picks a variant (`dirt`, `log`, `sandstone`...).
    Variant,
    /// `SlabItem`.
    Slab,
    /// `LeavesItem`.
    Leaves,
    /// `PlantBlockItem(block, true)`: `tallgrass`, whose metadata picks a name.
    Plant,
    /// `PlantBlockItem(block, false)` and `LilyPadItem`: no metadata names.
    PlantSimple,
    /// `SnowLayerItem` (a `LayeredBlockItem`).
    Layered,
    /// `PistonBlockItem`.
    Piston,
    /// `AnvilItem`.
    Anvil,
    /// `BushItem`.
    Bush,
}

impl BlockItemKind {
    /// Whether the class calls `setHasCustomData(true)`.
    const fn has_custom_data(self) -> bool {
        match self {
            Self::Colored
            | Self::Variant
            | Self::Slab
            | Self::Layered
            | Self::Anvil
            | Self::Bush
            | Self::Plant => true,
            Self::Block | Self::PlantSimple | Self::Piston | Self::Leaves => false,
        }
    }
}

/// What an item is, beyond the registry facts: the `Item` subclass it comes from.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ItemKind {
    /// Plain `Item`.
    Plain,
    /// `SwordItem`: 1 damage per attack, 2 per mined block.
    Sword {
        tier: ToolTier,
        /// `4.0F + tier.getAttackDamage()`, the attribute modifier.
        attack_damage: f32,
    },
    /// `ToolItem` (`PickaxeItem`, `ShovelItem`, `AxeItem`): 2 damage per attack, 1 per mined block.
    Tool {
        class: ToolClass,
        tier: ToolTier,
        /// `base + tier.getAttackDamage()`, the attribute modifier.
        attack_damage: f32,
    },
    /// `HoeItem`.
    Hoe { tier: ToolTier },
    /// `ShearsItem`.
    Shears,
    /// `FlintAndSteelItem`.
    FlintAndSteel,
    /// `BowItem`.
    Bow,
    /// `FishingRodItem`.
    FishingRod,
    /// `CarrotOnAStickItem`.
    CarrotOnAStick,
    /// `ArmorItem`, by slot: 0 helmet, 1 chestplate, 2 leggings, 3 boots.
    Armor { tier: ArmorTier, slot: usize },
    /// `FoodItem` and its subclasses. `FishItem`'s real hunger and saturation come from the fish type.
    Food {
        hunger: i32,
        saturation: f32,
        /// `canBeCooked`: meat.
        meat: bool,
    },
    /// `PotionItem`.
    Potion,
    /// `MusicDiscItem`.
    Record { record: &'static str },
    /// A `BlockItem` subclass.
    Block(BlockItemKind),
}

/// One `Item.REGISTRY` entry: the facts of a 1.7.10 item.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ItemDef {
    /// The numeric registry id.
    pub id: i32,
    /// The registry key (`"iron_shovel"`).
    pub name: &'static str,
    /// `setKey`: the translation suffix (`getTranslationKey` builds `"item." + key`).
    pub key: Option<&'static str>,
    /// `setMaxStackSize`; 64 unless the class or `Item.init()` set less.
    pub max_stack_size: i32,
    /// `setMaxDamage`.
    pub max_damage: i32,
    pub kind: ItemKind,
    /// `setCreativeModeTab`: `None` keeps the item out of the creative inventory.
    pub tab: Option<CreativeTab>,
    /// `setHasCustomData`: the item carries stack-unique data (fish type, map data...).
    pub has_custom_data: bool,
    /// `setRecipeRemainder`: the registry name of the item a crafting grid leaves.
    pub recipe_remainder: Option<&'static str>,
    /// The block this item places, for block items: the block's id.
    pub block: Option<i32>,
    /// The block's `setKey`: a block item's translation key is `"tile." + block key`.
    pub block_key: Option<&'static str>,
}

impl ItemDef {
    /// A plain item, with the defaults of `Item`: 64 stack, no damage, no tab.
    #[allow(clippy::too_many_arguments)]
    pub(crate) const fn item(
        id: i32,
        name: &'static str,
        key: &'static str,
        tab: Option<CreativeTab>,
        kind: ItemKind,
    ) -> Self {
        Self {
            id,
            name,
            key: Some(key),
            max_stack_size: 64,
            max_damage: 0,
            kind,
            tab,
            has_custom_data: false,
            recipe_remainder: None,
            block: None,
            block_key: None,
        }
    }

    /// `ToolItem(attackDamage, tier, effectiveBlocks)`: one stack, the tier's durability, TOOLS tab.
    pub(crate) const fn tool(
        id: i32,
        name: &'static str,
        key: &'static str,
        class: ToolClass,
        tier: ToolTier,
        base_attack: f32,
    ) -> Self {
        Self {
            max_stack_size: 1,
            max_damage: tier.durability(),
            kind: ItemKind::Tool {
                class,
                tier,
                attack_damage: base_attack + tier.attack_damage(),
            },
            tab: Some(CreativeTab::Tools),
            ..Self::item(id, name, key, None, ItemKind::Plain)
        }
    }

    /// `SwordItem(tier)`: one stack, the tier's durability, COMBAT tab.
    pub(crate) const fn sword(
        id: i32,
        name: &'static str,
        key: &'static str,
        tier: ToolTier,
    ) -> Self {
        Self {
            max_stack_size: 1,
            max_damage: tier.durability(),
            kind: ItemKind::Sword {
                tier,
                attack_damage: 4.0 + tier.attack_damage(),
            },
            tab: Some(CreativeTab::Combat),
            ..Self::item(id, name, key, None, ItemKind::Plain)
        }
    }

    /// `HoeItem(tier)`: one stack, the tier's durability, TOOLS tab.
    pub(crate) const fn hoe(
        id: i32,
        name: &'static str,
        key: &'static str,
        tier: ToolTier,
    ) -> Self {
        Self {
            max_stack_size: 1,
            max_damage: tier.durability(),
            kind: ItemKind::Hoe { tier },
            tab: Some(CreativeTab::Tools),
            ..Self::item(id, name, key, None, ItemKind::Plain)
        }
    }

    /// `ArmorItem(material, materialId, slot)`: one stack, COMBAT tab,
    /// `BASE_DURABILITY[slot] * tier.durability()` damage.
    pub(crate) const fn armor(
        id: i32,
        name: &'static str,
        key: &'static str,
        tier: ArmorTier,
        slot: usize,
    ) -> Self {
        Self {
            max_stack_size: 1,
            max_damage: tier::ARMOR_BASE_DURABILITY[slot] * tier.durability(),
            kind: ItemKind::Armor { tier, slot },
            tab: Some(CreativeTab::Combat),
            ..Self::item(id, name, key, None, ItemKind::Plain)
        }
    }

    /// `FoodItem(hunger, saturation, meat)`: FOOD tab.
    pub(crate) const fn food(
        id: i32,
        name: &'static str,
        key: &'static str,
        hunger: i32,
        saturation: f32,
        meat: bool,
    ) -> Self {
        Self {
            kind: ItemKind::Food {
                hunger,
                saturation,
                meat,
            },
            tab: Some(CreativeTab::Food),
            ..Self::item(id, name, key, None, ItemKind::Plain)
        }
    }

    /// `MusicDiscItem(record)`: one stack, MISC tab.
    pub(crate) const fn record(id: i32, name: &'static str, record: &'static str) -> Self {
        Self {
            max_stack_size: 1,
            kind: ItemKind::Record { record },
            tab: Some(CreativeTab::Misc),
            ..Self::item(id, name, "record", None, ItemKind::Plain)
        }
    }

    /// `setMaxStackSize`.
    pub(crate) const fn stack(mut self, size: i32) -> Self {
        self.max_stack_size = size;
        self
    }

    /// `setMaxDamage`.
    pub(crate) const fn damage(mut self, damage: i32) -> Self {
        self.max_damage = damage;
        self
    }

    /// `setHasCustomData(true)`.
    pub(crate) const fn custom(mut self) -> Self {
        self.has_custom_data = true;
        self
    }

    /// `setRecipeRemainder`.
    pub(crate) const fn remainder(mut self, name: &'static str) -> Self {
        self.recipe_remainder = Some(name);
        self
    }

    /// `isDamageable`: `maxDamage > 0 && !hasCustomData`.
    pub const fn is_damageable(&self) -> bool {
        self.max_damage > 0 && !self.has_custom_data
    }

    /// The tool class of a tool item: `"sword"`, `"pickaxe"`, ... `None` for other items.
    pub const fn tool_class(&self) -> Option<&'static str> {
        match self.kind {
            ItemKind::Sword { .. } => Some("sword"),
            ItemKind::Tool { class, .. } => match class {
                ToolClass::Pickaxe => Some("pickaxe"),
                ToolClass::Shovel => Some("shovel"),
                ToolClass::Axe => Some("axe"),
            },
            ItemKind::Hoe { .. } => Some("hoe"),
            ItemKind::Shears => Some("shears"),
            ItemKind::FlintAndSteel => Some("flint_and_steel"),
            ItemKind::Bow => Some("bow"),
            ItemKind::FishingRod => Some("fishing_rod"),
            ItemKind::CarrotOnAStick => Some("carrot_on_a_stick"),
            _ => None,
        }
    }

    /// `getTranslationKey`: `"item." + key`, or `"tile." + block key` for block items
    /// (`BlockItem.getTranslationKey` reads the block's).
    pub fn translation_key(&self) -> String {
        if let Some(key) = self.key {
            format!("item.{key}")
        } else if self.block.is_some() {
            format!("tile.{}", self.block_key.unwrap_or("null"))
        } else {
            "item.null".to_owned()
        }
    }
}

fn lookup(table: &'static [ItemDef], id: i32) -> Option<&'static ItemDef> {
    table
        .binary_search_by(|item| item.id.cmp(&id))
        .ok()
        .map(|i| &table[i])
}

/// `Item.byId`: the registry entry, by id. `None` below 256 and in the 409-416 gap.
pub fn by_id(id: i32) -> Option<&'static ItemDef> {
    // Block items live at the block's id (0-175), plain items at 256+.
    if id < 256 {
        lookup(BLOCK_ITEMS, id)
    } else {
        lookup(PLAIN, id)
    }
}

/// `Item.byBlock`: the item of a block, by the block's id. The item is `None` for
/// blocks `Item.init()`'s skip set leaves out (`air`, technical blocks...).
pub fn by_block(block: i32) -> Option<&'static ItemDef> {
    by_id(block)
}

/// `Item.REGISTRY.get(name)`: the registry entry, by key.
pub fn id_of(name: &str) -> Option<&'static ItemDef> {
    items().find(|item| item.name == name)
}

/// Every registry entry, ascending by id: block items first, then the 256+ plain items.
pub fn items() -> impl Iterator<Item = &'static ItemDef> {
    BLOCK_ITEMS.iter().chain(PLAIN.iter())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_is_sorted_and_unique() {
        let mut last = -1;
        for item in items() {
            assert!(item.id > last, "{} not after {}", item.id, last);
            last = item.id;
        }
    }

    #[test]
    fn registry_counts() {
        // 171 blocks in Block.init() minus Item.init()'s 27-entry skip set,
        // plus 153 plain items (256-408), 6 horse armor and carts (417-422),
        // and 12 records.
        assert_eq!(BLOCK_ITEMS.len(), 144);
        assert_eq!(PLAIN.len(), 171);
        assert_eq!(items().count(), 315);
    }

    #[test]
    fn ids_outside_the_registry() {
        assert!(by_id(0).is_none(), "air has no item");
        assert!(by_id(-1).is_none());
        assert!(by_id(255).is_none());
        assert!(by_id(409).is_none(), "the 409-416 gap is empty");
        assert!(by_id(416).is_none());
        assert!(by_id(423).is_none());
        assert!(by_id(2268).is_none());
    }

    #[test]
    fn by_id_spot_checks() {
        assert_eq!(by_id(1).unwrap().name, "stone");
        assert_eq!(by_id(256).unwrap().name, "iron_shovel");
        assert_eq!(by_id(408).unwrap().name, "hopper_minecart");
        assert_eq!(by_id(417).unwrap().name, "iron_horse_armor");
        assert_eq!(by_id(2267).unwrap().name, "record_wait");
    }

    #[test]
    fn tools_match_their_tier() {
        let shovel = by_id(256).unwrap();
        assert_eq!(shovel.tool_class(), Some("shovel"));
        assert_eq!(shovel.max_damage, 250);
        assert_eq!(shovel.max_stack_size, 1);
        assert_eq!(shovel.tab, Some(CreativeTab::Tools));
        assert_eq!(shovel.translation_key(), "item.shovelIron");
        assert!(shovel.is_damageable());
        let ItemKind::Tool {
            tier,
            attack_damage,
            ..
        } = shovel.kind
        else {
            panic!("iron_shovel is a tool");
        };
        assert_eq!(tier, ToolTier::Iron);
        assert!((attack_damage - 3.0).abs() < f32::EPSILON);

        let sword = by_id(276).unwrap();
        assert_eq!(sword.max_damage, 1561);
        assert_eq!(sword.tab, Some(CreativeTab::Combat));
        let ItemKind::Sword { attack_damage, .. } = sword.kind else {
            panic!("diamond_sword is a sword");
        };
        assert!((attack_damage - 7.0).abs() < f32::EPSILON);

        // Hoes have the tier's durability but no enchantability and no attack bonus.
        let hoe = by_id(294).unwrap();
        assert_eq!(hoe.max_damage, 32);
        assert_eq!(hoe.tool_class(), Some("hoe"));
        assert_eq!(hoe.tab, Some(CreativeTab::Tools));
    }

    #[test]
    fn armor_durability() {
        // BASE_DURABILITY[slot] * tier.durability()
        let cases = [
            (298, "leather_helmet", 11 * 5),
            (299, "leather_chestplate", 16 * 5),
            (301, "leather_boots", 13 * 5),
            (302, "chainmail_helmet", 11 * 15),
            (306, "iron_helmet", 11 * 15),
            (314, "golden_helmet", 11 * 7),
            (310, "diamond_helmet", 11 * 33),
            (311, "diamond_chestplate", 16 * 33),
        ];
        for (id, name, damage) in cases {
            let item = by_id(id).unwrap();
            assert_eq!(item.name, name);
            assert_eq!(item.max_damage, damage, "{name}");
            assert_eq!(item.max_stack_size, 1);
            assert_eq!(item.tab, Some(CreativeTab::Combat));
        }
        let ItemKind::Armor { tier, slot } = by_id(311).unwrap().kind else {
            panic!("diamond_chestplate is armor");
        };
        assert_eq!(tier.protection(slot), 8);
        assert_eq!(tier.enchantability(), 10);
        assert_eq!(tier.repair_item_name(), Some("diamond"));
    }

    #[test]
    fn buckets() {
        let bucket = id_of("bucket").unwrap();
        assert_eq!(bucket.max_stack_size, 16);
        assert_eq!(bucket.tab, Some(CreativeTab::Misc));
        let water = id_of("water_bucket").unwrap();
        assert_eq!(water.max_stack_size, 1);
        assert_eq!(water.recipe_remainder, Some("bucket"));
        assert_eq!(
            id_of("milk_bucket").unwrap().recipe_remainder,
            Some("bucket")
        );
    }

    #[test]
    fn stack_sizes_from_init_and_ctors() {
        assert_eq!(id_of("sign").unwrap().max_stack_size, 16);
        assert_eq!(id_of("snowball").unwrap().max_stack_size, 16);
        assert_eq!(id_of("egg").unwrap().max_stack_size, 16);
        assert_eq!(id_of("ender_pearl").unwrap().max_stack_size, 16);
        assert_eq!(
            id_of("written_book").unwrap().max_stack_size,
            16,
            "init overrides the ctor"
        );
        assert_eq!(id_of("writable_book").unwrap().max_stack_size, 1);
        assert_eq!(id_of("enchanted_book").unwrap().max_stack_size, 1);
        assert_eq!(id_of("cake").unwrap().max_stack_size, 1);
        assert_eq!(id_of("mushroom_stew").unwrap().max_stack_size, 1);
        assert_eq!(id_of("record_13").unwrap().max_stack_size, 1);
        assert_eq!(id_of("diamond").unwrap().max_stack_size, 64);
    }

    #[test]
    fn tabs_none_keeps_items_out_of_creative() {
        // Filled maps, written books, fireworks and enchanted books set no tab;
        // fish and cooked fish inherit FOOD from FoodItem's ctor.
        for name in ["filled_map", "written_book", "fireworks", "enchanted_book"] {
            assert_eq!(id_of(name).unwrap().tab, None, "{name}");
        }
        assert_eq!(id_of("fish").unwrap().tab, Some(CreativeTab::Food));
        assert_eq!(id_of("cooked_fished").unwrap().tab, Some(CreativeTab::Food));
        // Items init() sets a tab on keep it.
        assert_eq!(id_of("arrow").unwrap().tab, Some(CreativeTab::Combat));
        assert_eq!(id_of("stick").unwrap().tab, Some(CreativeTab::Materials));
        // init() clears the command block minecart's tab.
        assert_eq!(id_of("command_block_minecart").unwrap().tab, None);
        assert_eq!(
            id_of("tnt_minecart").unwrap().tab,
            Some(CreativeTab::Transportation)
        );
    }

    #[test]
    fn fish_and_other_custom_data() {
        for name in [
            "coal",
            "dye",
            "golden_apple",
            "fish",
            "cooked_fished",
            "spawn_egg",
            "potion",
            "filled_map",
            "skull",
        ] {
            assert!(id_of(name).unwrap().has_custom_data, "{name}");
        }
        let fish = id_of("fish").unwrap();
        assert!(!fish.is_damageable());
        let ItemKind::Food { hunger, .. } = fish.kind else {
            panic!("fish is food")
        };
        assert_eq!(hunger, 0, "FishItem's real hunger comes from the fish type");
    }

    #[test]
    fn damageables() {
        assert_eq!(id_of("flint_and_steel").unwrap().max_damage, 64);
        assert_eq!(id_of("bow").unwrap().max_damage, 384);
        assert_eq!(id_of("shears").unwrap().max_damage, 238);
        assert_eq!(id_of("fishing_rod").unwrap().max_damage, 64);
        assert_eq!(id_of("carrot_on_a_stick").unwrap().max_damage, 25);
    }

    #[test]
    fn block_items_match_their_block() {
        // Block items live at the block's id with the block's registry key.
        let wool = by_block(35).unwrap();
        assert_eq!(wool.name, "wool");
        assert_eq!(wool.block, Some(35));
        assert_eq!(wool.translation_key(), "tile.cloth");
        assert_eq!(wool.kind, ItemKind::Block(BlockItemKind::Colored));
        assert!(wool.has_custom_data);
        assert_eq!(wool.max_stack_size, 64);

        let cobblestone = by_block(4).unwrap();
        assert_eq!(cobblestone.translation_key(), "tile.stonebrick");
        assert_eq!(cobblestone.kind, ItemKind::Block(BlockItemKind::Block));
        assert_eq!(cobblestone.tab, Some(CreativeTab::BuildingBlocks));
        assert!(!cobblestone.has_custom_data);

        // Slabs, leaves, pistons, tallgrass and the double plant keep their class.
        assert_eq!(
            by_block(44).unwrap().kind,
            ItemKind::Block(BlockItemKind::Slab)
        );
        assert_eq!(
            by_block(18).unwrap().kind,
            ItemKind::Block(BlockItemKind::Leaves)
        );
        assert!(
            !by_block(18).unwrap().has_custom_data,
            "LeavesItem sets no custom data"
        );
        assert_eq!(
            by_block(33).unwrap().kind,
            ItemKind::Block(BlockItemKind::Piston)
        );
        assert!(
            by_block(31).unwrap().has_custom_data,
            "tallgrass stacks variants"
        );
        assert!(!by_block(106).unwrap().has_custom_data, "vine does not");
        assert_eq!(
            by_block(106).unwrap().kind,
            ItemKind::Block(BlockItemKind::PlantSimple)
        );
        assert_eq!(
            by_block(111).unwrap().kind,
            ItemKind::Block(BlockItemKind::PlantSimple)
        );
        assert_eq!(
            by_block(145).unwrap().kind,
            ItemKind::Block(BlockItemKind::Anvil)
        );
        assert_eq!(
            by_block(175).unwrap().kind,
            ItemKind::Block(BlockItemKind::Bush)
        );
    }

    #[test]
    fn technical_blocks_have_no_item() {
        // Item.init()'s skip set: air and the technical blocks.
        for id in [
            0, 26, 34, 36, 55, 59, 63, 64, 68, 71, 74, 75, 83, 92, 93, 94, 104, 105, 115, 117, 118,
            124, 132, 140, 144, 149, 150,
        ] {
            assert!(by_block(id).is_none(), "block {id} has no item");
        }
    }
}
