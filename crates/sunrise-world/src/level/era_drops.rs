//! 1.7.10's block drops (`Block.afterMinedByPlayer`, `dropItems`, `getDropItem`,
//! `getDropCount`, `getDropItemMetadata`, `getSilkTouchDrop`) for the era
//! export's block names.
//!
//! The 26.3 loot path rolls `data/minecraft/loot_table/blocks/<name>.json`; the
//! era export has no loot tables, so the table below is the era's own rule for
//! every block it registers. The names are the export's (`minecraft:grass`,
//! `minecraft:reeds`, ...) and `meta` is the state's `meta` property, which is
//! 1.7.10's `Block.getMetadata`.
//!
//! # How the rules are laid out
//!
//! Only the blocks whose drop differs from "one of its own item" are listed;
//! the rest fall through to [`plain`]'s default arm, as 1.7.10's
//! `Block.getDrops` gives `Item.byBlock(this)` with `getDropItemMetadata`
//! applied. Each helper names the vanilla method it stands for. The draw order
//! is vanilla's: the drop count first, then one luck `nextFloat` per iteration
//! of `dropItems`, then the item (a few rules draw there: gravel's flint,
//! tall grass's seeds), then the three position floats of `dropItem`, and
//! finally the experience draws.
//!
//! # Silk touch
//!
//! [`plain`] is the rule `afterMinedByPlayer` runs without silk touch. With
//! silk touch on a block whose `hasSilkTouchDrops` holds, vanilla never runs
//! `dropItems`: it drops one stack and makes no drop-count or luck draw, so
//! [`silk`] is the branch for every such block, even where its stack equals
//! the plain one. [`no_silk_touch_drops`] lists the blocks whose
//! `hasSilkTouchDrops` is false (non-cubes, and blocks with a block entity):
//! they fall through to [`plain`] as vanilla does.
//!
//! # What is not here
//!
//! * No `doTileDrops` check of its own: the caller passes [`after_mined_by_player`]'s
//!   `do_drops`, and the position draws are skipped exactly where vanilla's
//!   `dropItem` returns before them.
//! * No block entity contents: a broken container's contents spill through the
//!   engine's own container path.
//! * `InfestedBlock`'s silverfish and `IceBlock`'s melt-back are the removal's
//!   own side effects, not drops.
use sunrise_core::random::RandomSource;

/// The held stack's facts the drop rules read (`EnchantmentHelper` and the
/// shears checks).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tool {
    /// `Items.SHEARS` is held.
    pub shears: bool,
    /// `EnchantmentHelper.hasSilkTouch`.
    pub silk_touch: bool,
    /// `EnchantmentHelper.getFortuneLevel`.
    pub fortune: u32,
}

/// One stack a broken block drops, and where its item entity appears.
#[derive(Clone, Debug, PartialEq)]
pub struct Drop {
    pub item: String,
    pub count: u32,
    /// `getDropItemMetadata`: the era item's `Damage` (its metadata).
    pub damage: u16,
    /// `Block.dropItem`'s position: the block's corner plus 0.15..0.85 drawn
    /// from the level random on each axis.
    pub position: [f64; 3],
}

/// What a player's break drops: `Block.afterMinedByPlayer`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Drops {
    pub items: Vec<Drop>,
    /// `Block.dropXp`'s total, to split into orbs with
    /// `ExperienceOrbEntity.roundSize`.
    pub experience: u32,
}

/// `Block.afterMinedByPlayer` for the era block `block` with metadata `meta`:
/// the shears special cases first, then silk touch, then the plain rule.
/// `do_drops` is `doTileDrops`; when it is off the positions are not drawn,
/// as vanilla's `dropItem` returns before its random offsets.
pub fn after_mined_by_player<R: RandomSource>(
    block: &str,
    meta: u8,
    tool: Tool,
    pos: [i32; 3],
    do_drops: bool,
    random: &mut R,
) -> Drops {
    let name = block.strip_prefix("minecraft:").unwrap_or(block);
    // Levels past this have no distinct rule; the cap keeps the bounds of the
    // draws finite for a stack a command gave an extreme enchantment level.
    let tool = Tool {
        fortune: tool.fortune.min(30),
        ..tool
    };
    let mut ctx = Ctx {
        pos,
        do_drops,
        random,
        items: Vec::new(),
        experience: 0,
    };
    if tool.shears && ctx.shears(name, meta) {
        return ctx.finish();
    }
    if tool.silk_touch && ctx.silk(name, meta) {
        return ctx.finish();
    }
    ctx.plain(name, meta, tool.fortune);
    ctx.finish()
}

struct Ctx<'a, R> {
    pos: [i32; 3],
    do_drops: bool,
    random: &'a mut R,
    items: Vec<Drop>,
    experience: u32,
}

impl<R: RandomSource> Ctx<'_, R> {
    fn finish(self) -> Drops {
        Drops {
            items: self.items,
            experience: self.experience,
        }
    }

    /// `MathHelper.nextInt(random, min, max)`.
    fn int(&mut self, min: i32, max: i32) -> i32 {
        self.random
            .next_int_bound(max - min + 1)
            .unwrap_or(0)
            .wrapping_add(min)
    }

    /// `MathHelper.nextInt(random, 0, bound - 1)` (`nextInt(bound)`).
    fn below(&mut self, bound: i32) -> i32 {
        self.random.next_int_bound(bound).unwrap_or(0)
    }

    /// `Block.dropItem`: one item entity, at `x/y/z` plus the three offsets of
    /// the level random, unless `doTileDrops` is off (which skips the draws).
    fn drop_item(&mut self, item: &str, count: u32, damage: u16) {
        if !self.do_drops || count == 0 {
            return;
        }
        let mut offset = || 0.15 + f64::from(self.random.next_float()) * 0.7;
        let position = [
            f64::from(self.pos[0]) + offset(),
            f64::from(self.pos[1]) + offset(),
            f64::from(self.pos[2]) + offset(),
        ];
        self.items.push(Drop {
            item: format!("minecraft:{item}"),
            count,
            damage,
            position,
        });
    }

    /// `Block.dropItems` for a rule whose `getDropItem` is the same item for
    /// every iteration: `count` single-item drops, each behind its luck draw.
    fn constant(&mut self, item: &str, count: u32, damage: u16) {
        for _ in 0..count {
            // `world.random.nextFloat() > luck`, with luck 1: never skips, but
            // the draw stays in the sequence.
            let _luck = self.random.next_float();
            self.drop_item(item, 1, damage);
        }
    }

    /// A block that drops one of its own item (`Item.byBlock(this)`).
    fn itself(&mut self, name: &str, count: u32, meta: u8) {
        self.constant(name, count, default_damage(name, meta));
    }

    /// `AbstractLeavesBlock.afterMinedByPlayer`'s shears branch, `TallPlantBlock`'s,
    /// `DeadBushBlock`'s, `VineBlock`'s and `DoublePlantBlock`'s. True when the
    /// branch handled the break (even when it drops nothing).
    fn shears(&mut self, name: &str, meta: u8) -> bool {
        match name {
            "leaves" | "leaves2" => {
                let damage = u16::from(meta & 3);
                self.drop_item(name, 1, damage);
                true
            }
            "tallgrass" => {
                self.drop_item("tallgrass", 1, u16::from(meta));
                true
            }
            "deadbush" => {
                self.drop_item("deadbush", 1, u16::from(meta));
                true
            }
            "vine" => {
                self.drop_item("vine", 1, 0);
                true
            }
            "double_plant" => {
                if meta & 8 != 0 {
                    // The upper half: `getDropItem` gives null, and
                    // `DoublePlantBlock` skips `onMinedByPlayer`.
                    let _luck = self.random.next_float();
                    return true;
                }
                match meta & 7 {
                    2 | 3 => {
                        // `onMinedByPlayer` drops two tall grass; fern is
                        // metadata 2 of tall grass, grass is 1.
                        let damage = if meta & 7 == 3 { 2 } else { 1 };
                        self.drop_item("tallgrass", 2, damage);
                        true
                    }
                    // A flower: `onMinedByPlayer` answers false and
                    // `super.afterMinedByPlayer` runs the plain rule.
                    _ => false,
                }
            }
            _ => false,
        }
    }

    /// `Block.afterMinedByPlayer`'s silk branch. False when the block has no
    /// silk drop (`hasSilkTouchDrops`), which leaves the plain rule in charge.
    ///
    /// `getSilkTouchDrop` is the block's own item with its metadata when that
    /// item carries custom data, so most blocks need no entry here; the arms
    /// below are the overrides in the era's block classes.
    fn silk(&mut self, name: &str, meta: u8) -> bool {
        if no_silk_touch_drops(name) {
            return false;
        }
        let (item, count, damage) = match name {
            // `DirtBlock.getSilkTouchDrop` maps metadata 1 to 0; the item keeps
            // the podzol's 2.
            "dirt" => ("dirt", 1, if meta == 1 { 0 } else { u16::from(meta) }),
            // `InfestedBlock.getSilkTouchDrop` maps the variant back to the
            // block it hides in.
            "monster_egg" => match meta {
                1 => ("cobblestone", 1, 0),
                2 => ("stonebrick", 1, 0),
                3 => ("stonebrick", 1, 1),
                4 => ("stonebrick", 1, 2),
                5 => ("stonebrick", 1, 3),
                _ => ("stone", 1, 0),
            },
            // `AbstractLeavesBlock.getSilkTouchDrop` keeps the variant bits.
            "leaves" | "leaves2" => (name, 1, u16::from(meta & 3)),
            // `RedstoneOreBlock.getSilkTouchDrop` gives the unlit block.
            "redstone_ore" | "lit_redstone_ore" => ("redstone_ore", 1, 0),
            // `RedstoneLampBlock.getSilkTouchDrop` gives the unlit lamp.
            "lit_redstone_lamp" => ("redstone_lamp", 1, 0),
            // `StoneSlabBlock`/`WoodenSlabBlock`: two single slabs.
            "double_stone_slab" => ("stone_slab", 2, u16::from(meta & 7)),
            "double_wooden_slab" => ("wooden_slab", 2, u16::from(meta & 7)),
            // `PaneBlock.getSilkTouchDrop` keeps the metadata: the colour of a
            // stained pane and the connections of iron bars.
            "stained_glass" | "stained_glass_pane" | "iron_bars" => (name, 1, u16::from(meta)),
            _ => (name, 1, default_damage(name, meta)),
        };
        self.drop_item(item, count, damage);
        true
    }

    /// `Block.dropItems` with the plain (non-silk) rule of `name`.
    fn plain(&mut self, name: &str, meta: u8, fortune: u32) {
        match name {
            // No item and no experience.
            "air" | "cake" | "command_block" | "piston_head" | "piston_extension" | "fire"
            | "portal" | "end_portal" => {}
            // `EndPortalFrameBlock.getDropItem` is null, but `dropItems` still
            // makes its one luck draw.
            "end_portal_frame" => {
                let _luck = self.random.next_float();
            }
            // `GlassBlock`, `StainedGlassBlock`, `PaneBlock` without its own
            // drops, `IceBlock` and `PackedIceBlock` drop nothing.
            "glass" | "stained_glass" | "ice" | "packed_ice" => {}
            // `PaneBlock.getDropItem` answers null for `glass_pane` and
            // `stained_glass_pane` (`hasDrops` false), but `dropItems` still
            // makes its one luck draw.
            "glass_pane" | "stained_glass_pane" => {
                let _luck = self.random.next_float();
            }
            // `SkullBlock.dropItems` is empty and `InfestedBlock` only spawns
            // its silverfish.
            "skull" | "monster_egg" => {}
            // `DeadBushBlock` has no item (`getDropItem` null) but `dropItems`
            // makes its one luck draw; `VineBlock.getBaseDropCount` is 0, so
            // `dropItems` draws nothing at all.
            "deadbush" => {
                let _luck = self.random.next_float();
            }
            "vine" => {}
            // `StoneBlock`.
            "stone" => self.constant("cobblestone", 1, 0),
            // `GrassBlock`, `MyceliumBlock` and `FarmlandBlock` drop dirt;
            // `DirtBlock.getDropItemMetadata` is 0.
            "grass" | "mycelium" | "farmland" | "dirt" => self.constant("dirt", 1, 0),
            // `OreBlock`: the item (or the block for gold and iron), the lapis
            // count, fortune's multiplier and the experience range.
            "coal_ore" | "diamond_ore" | "emerald_ore" | "lapis_ore" | "quartz_ore"
            | "gold_ore" | "iron_ore" => self.ore(name, fortune),
            // `RedstoneOreBlock`.
            "redstone_ore" | "lit_redstone_ore" => {
                let base = 4 + self.below(2);
                let count = base + self.below(fortune as i32 + 1);
                self.constant("redstone", count as u32, 0);
                self.experience += (1 + self.below(5)) as u32;
            }
            // `AbstractLeavesBlock.dropItems`: a sapling roll then an apple
            // roll, both `dropItem` calls without a luck draw.
            "leaves" | "leaves2" => self.leaves(name, meta, fortune),
            // `GravelBlock.getDropItem`: flint, or the gravel itself.
            "gravel" => {
                let fortune = fortune.min(3) as i32;
                let _luck = self.random.next_float();
                let item = if self.below(10 - fortune * 3) == 0 {
                    "flint"
                } else {
                    "gravel"
                };
                self.drop_item(item, 1, 0);
            }
            // `ClayBlock`: four clay balls.
            "clay" => self.constant("clay_ball", 4, 0),
            // `BookshelfBlock`: three books.
            "bookshelf" => self.constant("book", 3, 0),
            // `CobwebBlock`: one string.
            "web" => self.constant("string", 1, 0),
            // `EnderChestBlock`: eight obsidian.
            "ender_chest" => self.constant("obsidian", 8, 0),
            // `SnowBlock`: four snowballs.
            "snow" => self.constant("snowball", 4, 0),
            // `SnowLayerBlock.afterMinedByPlayer`: one stack of
            // `layers + 1` snowballs.
            "snow_layer" => {
                let count = u32::from(meta & 7) + 1;
                self.drop_item("snowball", count, 0);
            }
            // `GlowstoneBlock.getDropCount`: 2..4 dust, fortune up to four.
            "glowstone" => {
                let base = 2 + self.below(3);
                let count = (base + self.below(fortune as i32 + 1)).clamp(1, 4);
                self.constant("glowstone_dust", count as u32, 0);
            }
            // `MelonBlock.getDropCount`: 3..7 slices, fortune up to nine.
            "melon_block" => {
                let base = 3 + self.below(5);
                let count = (base + self.below(fortune as i32 + 1)).min(9);
                self.constant("melon", count as u32, 0);
            }
            // `MushroomBlock.getBaseDropCount`: `nextInt(10) - 7`, floored at 0.
            "brown_mushroom_block" => {
                let count = (self.below(10) - 7).max(0);
                self.constant("brown_mushroom", count as u32, 0);
            }
            "red_mushroom_block" => {
                let count = (self.below(10) - 7).max(0);
                self.constant("red_mushroom", count as u32, 0);
            }
            // `MobSpawnerBlock.dropItems`: only experience.
            "mob_spawner" => {
                self.experience += (15 + self.below(15) + self.below(15)) as u32;
            }
            // `TallPlantBlock.getDropCount`/`getDropItem`: a seed per roll with
            // a 1-in-8 chance.
            "tallgrass" => {
                let count = 1 + self.below(fortune as i32 * 2 + 1);
                for _ in 0..count {
                    let _luck = self.random.next_float();
                    if self.below(8) == 0 {
                        self.drop_item("wheat_seeds", 1, 0);
                    }
                }
            }
            // `DoublePlantBlock.getDropItem`: the upper half and the grass and
            // fern variants give null; a flower gives the item with its
            // variant.
            "double_plant" => {
                let variant = meta & 7;
                if meta & 8 != 0 || variant == 2 || variant == 3 {
                    let _luck = self.random.next_float();
                } else {
                    self.constant("double_plant", 1, u16::from(variant));
                }
            }
            // `WheatBlock`, `CarrotsBlock` and `PotatoesBlock`.
            "wheat" | "carrots" | "potatoes" => self.crop(name, meta, fortune),
            // `StemBlock.dropItems`: three rolls for a seed, `nextInt(15) <=
            // metadata`; `getDropItem` is null, so the base rule draws its luck
            // and drops nothing.
            "pumpkin_stem" | "melon_stem" => {
                let item = if name == "pumpkin_stem" {
                    "pumpkin_seeds"
                } else {
                    "melon_seeds"
                };
                let _luck = self.random.next_float();
                for _ in 0..3 {
                    if self.below(15) <= i32::from(meta) {
                        self.drop_item(item, 1, 0);
                    }
                }
            }
            // `NetherWartBlock.dropItems`: one below age 3, 2..4 plus fortune
            // from age 3 on.
            "nether_wart" => {
                let count = if meta >= 3 {
                    let mut count = 2 + self.below(3);
                    if fortune > 0 {
                        count += self.below(fortune as i32 + 1);
                    }
                    count
                } else {
                    1
                };
                for _ in 0..count {
                    self.drop_item("nether_wart", 1, 0);
                }
            }
            // `CocoaBlock.dropItems`: one brown dye, three at the last two
            // growth stages.
            "cocoa" => {
                let stage = (meta & 12) >> 2;
                let count = if stage >= 2 { 3 } else { 1 };
                for _ in 0..count {
                    self.drop_item("dye", 1, 3);
                }
            }
            // `BedBlock`: the foot drops the bed, the head nothing; both run
            // `dropItems`' count and luck draws.
            "bed" => {
                let _luck = self.random.next_float();
                if meta & 8 == 0 {
                    self.drop_item("bed", 1, 0);
                }
            }
            // `DoorBlock`: the lower half drops the door, the upper half's
            // `getDropItem` is null but its luck draw still happens.
            "wooden_door" | "iron_door" => {
                if meta & 8 == 0 {
                    self.constant(name, 1, 0);
                } else {
                    let _luck = self.random.next_float();
                }
            }
            // `SignBlock`, `RepeaterBlock`, `ComparatorBlock`, `FurnaceBlock`,
            // `RedstoneLampBlock`, `BrewingStandBlock`, `CauldronBlock`,
            // `FlowerPotBlock`, `RedstoneWireBlock`, `RedstoneTorchBlock`,
            // `TripwireBlock` and `SugarCaneBlock` drop an item that is not
            // their block's.
            "standing_sign" | "wall_sign" => self.constant("sign", 1, 0),
            "unpowered_repeater" | "powered_repeater" => self.constant("repeater", 1, 0),
            "unpowered_comparator" | "powered_comparator" => self.constant("comparator", 1, 0),
            "furnace" | "lit_furnace" => self.constant("furnace", 1, 0),
            "lit_redstone_lamp" => self.constant("redstone_lamp", 1, 0),
            "brewing_stand" => self.constant("brewing_stand", 1, 0),
            "cauldron" => self.constant("cauldron", 1, 0),
            "flower_pot" => self.constant("flower_pot", 1, 0),
            "redstone_wire" => self.constant("redstone", 1, 0),
            "unlit_redstone_torch" | "redstone_torch" => self.constant("redstone_torch", 1, 0),
            "tripwire" => self.constant("string", 1, 0),
            "reeds" => self.constant("reeds", 1, 0),
            // The slab and double-slab pairs: `SlabBlock.getBaseDropCount` is
            // two for a double slab, and `getDropItemMetadata` is `meta & 7`.
            "stone_slab" => self.constant("stone_slab", 1, u16::from(meta & 7)),
            "double_stone_slab" => self.constant("stone_slab", 2, u16::from(meta & 7)),
            "wooden_slab" => self.constant("wooden_slab", 1, u16::from(meta & 7)),
            "double_wooden_slab" => self.constant("wooden_slab", 2, u16::from(meta & 7)),
            // Everything else: one of its own item, with the block's own
            // `getDropItemMetadata`.
            _ => self.itself(name, 1, meta),
        }
    }

    /// `OreBlock.getDropCount`/`getDropItem`/`dropItems`.
    fn ore(&mut self, name: &str, fortune: u32) {
        let (item, drops_self) = match name {
            "coal_ore" => ("coal", false),
            "diamond_ore" => ("diamond", false),
            "emerald_ore" => ("emerald", false),
            "lapis_ore" => ("dye", false),
            "quartz_ore" => ("quartz", false),
            _ => (name, true),
        };
        // `getDropCount`: the multiplier is drawn before the base count.
        let mut multiplier = 0;
        if fortune > 0 && !drops_self {
            multiplier = (self.below(fortune as i32 + 2) - 1).max(0);
        }
        let base = if name == "lapis_ore" {
            4 + self.below(5)
        } else {
            1
        };
        let count = base * (multiplier + 1);
        let damage = if name == "lapis_ore" { 4 } else { 0 };
        self.constant(item, count as u32, damage);
        if !drops_self {
            // `dropItems`' experience: `getDropItem` is not the block's item.
            self.experience += match name {
                "coal_ore" => self.int(0, 2),
                "diamond_ore" | "emerald_ore" => self.int(3, 7),
                "lapis_ore" | "quartz_ore" => self.int(2, 5),
                _ => 0,
            } as u32;
        }
    }

    /// `AbstractLeavesBlock.dropItems` and the two leaf classes' chances.
    fn leaves(&mut self, name: &str, meta: u8, fortune: u32) {
        // `LeavesBlock.getSaplingDropChance`: 20, 40 for jungle leaves.
        let mut chance = if name == "leaves" && meta & 3 == 3 {
            40
        } else {
            20
        };
        if fortune > 0 {
            chance -= 2i32 << fortune.min(20);
            chance = chance.max(10);
        }
        // `AbstractLeavesBlock.getDropItemMetadata` (leaves2 adds four).
        let sapling = if name == "leaves2" {
            (meta & 3) | 4
        } else {
            meta & 3
        };
        if self.below(chance) == 0 {
            self.drop_item("sapling", 1, u16::from(sapling));
        }
        // `LeavesBlock.dropAppleWithChance` (oak) and `Leaves2Block`'s (dark
        // oak) draw only when their variant matches.
        let apple = match name {
            "leaves" => meta & 3 == 0,
            _ => meta & 3 == 1,
        };
        let mut chance = 200;
        if fortune > 0 {
            chance -= 10i32 << fortune.min(20);
            chance = chance.max(40);
        }
        if apple && self.below(chance) == 0 {
            self.drop_item("apple", 1, 0);
        }
    }

    /// `WheatBlock.dropItems` in its three subclasses: one plant or seed at
    /// harvest age, then `3 + fortune` seed rolls, then (potatoes) the
    /// poisonous potato chance.
    fn crop(&mut self, name: &str, meta: u8, fortune: u32) {
        let (seed, plant) = match name {
            "wheat" => ("wheat_seeds", "wheat"),
            "carrots" => ("carrot", "carrot"),
            _ => ("potato", "potato"),
        };
        // `super.dropItems(world, x, y, z, metadata, luck, 0)`: one luck draw,
        // then `getDropItem`'s plant at age 7, the seed before it.
        let _luck = self.random.next_float();
        let item = if meta == 7 { plant } else { seed };
        self.drop_item(item, 1, 0);
        if meta >= 7 {
            let rolls = 3 + fortune;
            for _ in 0..rolls {
                if self.below(15) <= i32::from(meta) {
                    self.drop_item(seed, 1, 0);
                }
            }
        }
        if name == "potatoes" && meta >= 7 && self.below(50) == 0 {
            self.drop_item("poisonous_potato", 1, 0);
        }
    }
}

/// `Block.hasSilkTouchDrops` for the era block `block`: a full cube without a
/// block entity, unless the class overrides the answer.
///
/// The names below are the blocks of the era export whose `isCube()` is false
/// (stairs, the single slabs, plants, rails, torches and the rest of the
/// non-cubes) or that carry a block entity. The blocks that override
/// `hasSilkTouchDrops` back to true (glass, stained glass, the cobweb, the
/// panes and the ender chest) are absent, so they keep their silk drop.
fn no_silk_touch_drops(block: &str) -> bool {
    matches!(
        block,
        "acacia_stairs"
            | "activator_rail"
            | "anvil"
            | "beacon"
            | "bed"
            | "birch_stairs"
            | "brewing_stand"
            | "brick_stairs"
            | "brown_mushroom"
            | "cactus"
            | "cake"
            | "carpet"
            | "carrots"
            | "cauldron"
            | "chest"
            | "cobblestone_wall"
            | "cocoa"
            | "command_block"
            | "dark_oak_stairs"
            | "daylight_detector"
            | "deadbush"
            | "detector_rail"
            | "dispenser"
            | "double_plant"
            | "dragon_egg"
            | "dropper"
            | "enchanting_table"
            | "end_portal"
            | "farmland"
            | "fence"
            | "fence_gate"
            | "fire"
            | "flower_pot"
            | "flowing_lava"
            | "flowing_water"
            | "furnace"
            | "golden_rail"
            | "heavy_weighted_pressure_plate"
            | "hopper"
            | "iron_door"
            | "jukebox"
            | "jungle_stairs"
            | "ladder"
            | "lava"
            | "lever"
            | "light_weighted_pressure_plate"
            | "lit_furnace"
            | "melon_stem"
            | "mob_spawner"
            | "nether_brick_fence"
            | "nether_brick_stairs"
            | "nether_wart"
            | "noteblock"
            | "oak_stairs"
            | "piston"
            | "piston_extension"
            | "piston_head"
            | "portal"
            | "potatoes"
            | "powered_comparator"
            | "powered_repeater"
            | "pumpkin_stem"
            | "quartz_stairs"
            | "rail"
            | "red_flower"
            | "red_mushroom"
            | "redstone_torch"
            | "redstone_wire"
            | "reeds"
            | "sandstone_stairs"
            | "sapling"
            | "skull"
            | "snow_layer"
            | "spruce_stairs"
            | "standing_sign"
            | "sticky_piston"
            | "stone_brick_stairs"
            | "stone_button"
            | "stone_pressure_plate"
            | "stone_slab"
            | "stone_stairs"
            | "tallgrass"
            | "torch"
            | "trapdoor"
            | "trapped_chest"
            | "tripwire"
            | "tripwire_hook"
            | "unlit_redstone_torch"
            | "unpowered_comparator"
            | "unpowered_repeater"
            | "vine"
            | "wall_sign"
            | "water"
            | "waterlily"
            | "wheat"
            | "wooden_button"
            | "wooden_door"
            | "wooden_pressure_plate"
            | "wooden_slab"
            | "yellow_flower"
    )
}

/// `Block.getDropItemMetadata` for the blocks whose plain rule drops their own
/// item: the variant metadata, or 0 for the rest.
fn default_damage(name: &str, meta: u8) -> u16 {
    match name {
        // `AxisBlock.getDropItemMetadata`.
        "log" | "log2" | "hay_block" => u16::from(meta & 3),
        // `ColoredBlock`, `PlanksBlock`, `SandBlock`, `SandstoneBlock`,
        // `StonebrickBlock`, `CarpetBlock`, `WallBlock` and `FlowerBlock`.
        "planks"
        | "wool"
        | "stained_hardened_clay"
        | "sand"
        | "sandstone"
        | "stonebrick"
        | "carpet"
        | "cobblestone_wall"
        | "yellow_flower"
        | "red_flower" => u16::from(meta),
        // `SaplingBlock.getDropItemMetadata`: `clamp(meta & 7, 0, 5)`.
        "sapling" => u16::from((meta & 7).min(5)),
        // `AnvilBlock.getDropItemMetadata`.
        "anvil" => u16::from(meta >> 2),
        // `QuartzBlock.getDropItemMetadata`: the chiseled and pillar variants
        // come back as the default block.
        "quartz_block" => u16::from(if matches!(meta, 3 | 4) { 2 } else { meta }),
        // `SlabBlock.getDropItemMetadata` for the four slab blocks.
        "stone_slab" | "double_stone_slab" | "wooden_slab" | "double_wooden_slab" => {
            u16::from(meta & 7)
        }
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sunrise_core::random::LegacyRandom;

    fn drops(block: &str, meta: u8, tool: Tool) -> Drops {
        drops_with(block, meta, tool, true, 4242)
    }

    fn drops_with(block: &str, meta: u8, tool: Tool, do_drops: bool, seed: u64) -> Drops {
        let mut random = LegacyRandom::new(seed as i64);
        after_mined_by_player(block, meta, tool, [10, 64, -3], do_drops, &mut random)
    }

    /// The stack list as `(item, count, damage)`.
    fn stacks(drops: &Drops) -> Vec<(&str, u32, u16)> {
        drops
            .items
            .iter()
            .map(|drop| (&*drop.item, drop.count, drop.damage))
            .collect()
    }

    fn plain() -> Tool {
        Tool::default()
    }

    fn silk() -> Tool {
        Tool {
            silk_touch: true,
            ..Tool::default()
        }
    }

    #[test]
    fn the_table_matches_the_era_sources() {
        // `StoneBlock`, `GrassBlock`, `GlassBlock`, `ClayBlock`, `SnowBlock`,
        // `BookshelfBlock`, `EnderChestBlock`, `OreBlock`, `RedstoneOreBlock`,
        // `GlowstoneBlock` and `MelonBlock`.
        assert_eq!(
            stacks(&drops("minecraft:stone", 0, plain())),
            [("minecraft:cobblestone", 1, 0)]
        );
        assert_eq!(
            stacks(&drops("minecraft:grass", 0, plain())),
            [("minecraft:dirt", 1, 0)]
        );
        assert!(stacks(&drops("minecraft:glass", 0, plain())).is_empty());
        assert_eq!(
            stacks(&drops("minecraft:clay", 0, plain())),
            [("minecraft:clay_ball", 1, 0); 4]
        );
        assert_eq!(
            stacks(&drops("minecraft:snow", 0, plain())),
            [("minecraft:snowball", 1, 0); 4]
        );
        assert_eq!(
            stacks(&drops("minecraft:bookshelf", 0, plain())),
            [("minecraft:book", 1, 0); 3]
        );
        assert_eq!(
            stacks(&drops("minecraft:ender_chest", 0, plain())),
            [("minecraft:obsidian", 1, 0); 8]
        );
        assert_eq!(
            stacks(&drops("minecraft:coal_ore", 0, plain())),
            [("minecraft:coal", 1, 0)]
        );
        assert_eq!(
            stacks(&drops("minecraft:quartz_ore", 0, plain())),
            [("minecraft:quartz", 1, 0)]
        );
        let redstone = drops("minecraft:redstone_ore", 0, plain());
        assert!(
            stacks(&redstone)
                .iter()
                .all(|(item, ..)| *item == "minecraft:redstone")
        );
        assert!((1..=5).contains(&redstone.experience));
        let glowstone = drops("minecraft:glowstone", 0, plain());
        assert!((1..=4).contains(&(glowstone.items.len() as u32)));
        assert!(
            stacks(&glowstone)
                .iter()
                .all(|(item, ..)| *item == "minecraft:glowstone_dust")
        );
        let melon = drops("minecraft:melon_block", 0, plain());
        assert!((3..=9).contains(&(melon.items.len() as u32)));
        assert!(
            stacks(&melon)
                .iter()
                .all(|(item, ..)| *item == "minecraft:melon")
        );
    }

    #[test]
    fn silk_touch_replaces_the_plain_drop() {
        // `Block.getSilkTouchDrop`, `OreBlock`, `RedstoneOreBlock`,
        // `GlassBlock`, `IceBlock`, `ClayBlock`, `GlowstoneBlock`,
        // `GravelBlock` and `AbstractLeavesBlock`.
        assert_eq!(
            stacks(&drops("minecraft:stone", 0, silk())),
            [("minecraft:stone", 1, 0)]
        );
        assert_eq!(
            stacks(&drops("minecraft:grass", 0, silk())),
            [("minecraft:grass", 1, 0)]
        );
        assert_eq!(
            stacks(&drops("minecraft:glass", 0, silk())),
            [("minecraft:glass", 1, 0)]
        );
        assert_eq!(
            stacks(&drops("minecraft:ice", 0, silk())),
            [("minecraft:ice", 1, 0)]
        );
        assert_eq!(
            stacks(&drops("minecraft:clay", 0, silk())),
            [("minecraft:clay", 1, 0)]
        );
        assert_eq!(
            stacks(&drops("minecraft:gravel", 0, silk())),
            [("minecraft:gravel", 1, 0)]
        );
        assert_eq!(
            stacks(&drops("minecraft:coal_ore", 0, silk())),
            [("minecraft:coal_ore", 1, 0)]
        );
        assert_eq!(
            stacks(&drops("minecraft:redstone_ore", 0, silk())),
            [("minecraft:redstone_ore", 1, 0)]
        );
        assert_eq!(drops("minecraft:coal_ore", 0, silk()).experience, 0);
        assert_eq!(
            stacks(&drops("minecraft:leaves", 2, silk())),
            [("minecraft:leaves", 1, 2)]
        );
        assert_eq!(
            stacks(&drops("minecraft:stained_glass", 5, silk())),
            [("minecraft:stained_glass", 1, 5)]
        );
        // `DirtBlock.getSilkTouchDrop` only folds metadata 1; podzol's 2 stays.
        assert_eq!(
            stacks(&drops("minecraft:dirt", 1, silk())),
            [("minecraft:dirt", 1, 0)]
        );
        assert_eq!(
            stacks(&drops("minecraft:dirt", 2, silk())),
            [("minecraft:dirt", 1, 2)]
        );
        // `PaneBlock.getSilkTouchDrop` keeps the metadata: iron bars keep their
        // connection bits, and the stained pane its colour.
        assert_eq!(
            stacks(&drops("minecraft:iron_bars", 3, silk())),
            [("minecraft:iron_bars", 1, 3)]
        );
        assert_eq!(
            stacks(&drops("minecraft:stained_glass_pane", 7, silk())),
            [("minecraft:stained_glass_pane", 1, 7)]
        );
        // `StoneSlabBlock`/`WoodenSlabBlock.getSilkTouchDrop`: two singles.
        assert_eq!(
            stacks(&drops("minecraft:double_stone_slab", 3, silk())),
            [("minecraft:stone_slab", 2, 3)]
        );
        assert_eq!(
            stacks(&drops("minecraft:double_wooden_slab", 1, silk())),
            [("minecraft:wooden_slab", 2, 1)]
        );
        // `RedstoneLampBlock.getSilkTouchDrop` gives the unlit lamp, and the
        // cobweb's own item is `web`.
        assert_eq!(
            stacks(&drops("minecraft:lit_redstone_lamp", 0, silk())),
            [("minecraft:redstone_lamp", 1, 0)]
        );
        assert_eq!(
            stacks(&drops("minecraft:web", 0, silk())),
            [("minecraft:web", 1, 0)]
        );
        // `FarmlandBlock.isCube()` is false, so silk touch does not apply and
        // the plain dirt drop stands.
        assert_eq!(
            stacks(&drops("minecraft:farmland", 0, silk())),
            [("minecraft:dirt", 1, 0)]
        );
    }

    #[test]
    fn the_silk_branch_skips_the_plain_draws() {
        // `Block.afterMinedByPlayer`: the silk branch calls `dropItem` alone. A
        // plain `dropItems` first draws a luck float per iteration, then the
        // item, then `dropItem`'s three offsets, so its break consumes one
        // draw more. The count below is the number of floats the level random
        // is left at.
        fn draws(block: &str, tool: Tool) -> usize {
            let mut random = LegacyRandom::new(7);
            let _ = after_mined_by_player(block, 0, tool, [10, 64, -3], true, &mut random);
            let next = [random.next_float(), random.next_float()];
            for n in 0..16 {
                let mut fresh = LegacyRandom::new(7);
                for _ in 0..n {
                    let _ = fresh.next_float();
                }
                if [fresh.next_float(), fresh.next_float()] == next {
                    return n;
                }
            }
            panic!("the draws did not line up with the plain sequence");
        }
        assert_eq!(draws("minecraft:cobblestone", silk()), 3);
        assert_eq!(draws("minecraft:cobblestone", plain()), 4);
        assert_eq!(draws("minecraft:stone", silk()), 3);
        // A block without `hasSilkTouchDrops` keeps the plain rule under silk.
        assert_eq!(draws("minecraft:stone_slab", silk()), 4);
        assert_eq!(draws("minecraft:farmland", silk()), 4);
    }

    #[test]
    fn variants_keep_their_metadata() {
        // `ColoredBlock` (wool), `PlanksBlock`, `AxisBlock` (log),
        // `SaplingBlock`, `SlabBlock`, `QuartzBlock` and `AnvilBlock`.
        assert_eq!(
            stacks(&drops("minecraft:wool", 14, plain())),
            [("minecraft:wool", 1, 14)]
        );
        assert_eq!(
            stacks(&drops("minecraft:planks", 4, plain())),
            [("minecraft:planks", 1, 4)]
        );
        assert_eq!(
            stacks(&drops("minecraft:log", 2, plain())),
            [("minecraft:log", 1, 2)]
        );
        assert_eq!(
            stacks(&drops("minecraft:log2", 1, plain())),
            [("minecraft:log2", 1, 1)]
        );
        assert_eq!(
            stacks(&drops("minecraft:sapling", 6, plain())),
            [("minecraft:sapling", 1, 5)]
        );
        assert_eq!(
            stacks(&drops("minecraft:stone_slab", 3, plain())),
            [("minecraft:stone_slab", 1, 3)]
        );
        assert_eq!(
            stacks(&drops("minecraft:double_stone_slab", 3, plain())),
            [("minecraft:stone_slab", 1, 3); 2]
        );
        assert_eq!(
            stacks(&drops("minecraft:quartz_block", 3, plain())),
            [("minecraft:quartz_block", 1, 2)]
        );
        assert_eq!(
            stacks(&drops("minecraft:anvil", 5, plain())),
            [("minecraft:anvil", 1, 1)]
        );
    }

    #[test]
    fn shears_keep_the_plant_and_leaves() {
        // `AbstractLeavesBlock`, `TallPlantBlock`, `DeadBushBlock`,
        // `VineBlock` and `DoublePlantBlock`.
        let shears = Tool {
            shears: true,
            ..Tool::default()
        };
        assert!(
            stacks(&drops("minecraft:tallgrass", 2, plain())).len() <= 1,
            "without shears tall grass only has its seed chance"
        );
        assert_eq!(
            stacks(&drops("minecraft:tallgrass", 2, shears)),
            [("minecraft:tallgrass", 1, 2)]
        );
        assert_eq!(
            stacks(&drops("minecraft:deadbush", 0, shears)),
            [("minecraft:deadbush", 1, 0)]
        );
        assert_eq!(
            stacks(&drops("minecraft:vine", 0, shears)),
            [("minecraft:vine", 1, 0)]
        );
        assert!(stacks(&drops("minecraft:vine", 0, plain())).is_empty());
        assert_eq!(
            stacks(&drops("minecraft:leaves", 3, shears)),
            [("minecraft:leaves", 1, 3)]
        );
        assert_eq!(
            stacks(&drops("minecraft:leaves2", 1, shears)),
            [("minecraft:leaves2", 1, 1)]
        );
        // The upper half of a double plant gives nothing; the lower half of
        // grass/fern gives two tall grass.
        assert!(stacks(&drops("minecraft:double_plant", 8 | 3, shears)).is_empty());
        assert_eq!(
            stacks(&drops("minecraft:double_plant", 3, shears)),
            [("minecraft:tallgrass", 2, 2)]
        );
        assert_eq!(
            stacks(&drops("minecraft:double_plant", 0, plain())),
            [("minecraft:double_plant", 1, 0)]
        );
    }

    #[test]
    fn crops_drop_their_produce_and_seeds() {
        // `WheatBlock`, `CarrotsBlock`, `PotatoesBlock`, `StemBlock`,
        // `CocoaBlock` and `NetherWartBlock`.
        let wheat = drops("minecraft:wheat", 7, plain());
        assert!(stacks(&wheat).contains(&("minecraft:wheat", 1, 0)));
        assert!(
            stacks(&wheat)
                .iter()
                .any(|(item, ..)| *item == "minecraft:wheat_seeds")
        );
        // Age 6 has no produce, only its seed chance.
        let young = drops("minecraft:wheat", 6, plain());
        assert_eq!(stacks(&young), [("minecraft:wheat_seeds", 1, 0)]);
        let carrots = drops("minecraft:carrots", 7, plain());
        assert!(stacks(&carrots).contains(&("minecraft:carrot", 1, 0)));
        let potatoes = drops("minecraft:potatoes", 7, plain());
        assert!(stacks(&potatoes).contains(&("minecraft:potato", 1, 0)));
        let stems = drops("minecraft:pumpkin_stem", 7, plain());
        assert!(
            stacks(&stems)
                .iter()
                .all(|(item, ..)| *item == "minecraft:pumpkin_seeds")
        );
        let cocoa = drops("minecraft:cocoa", 8, plain());
        assert_eq!(stacks(&cocoa), [("minecraft:dye", 1, 3); 3]);
        let wart = drops("minecraft:nether_wart", 3, plain());
        assert!((2..=4).contains(&(wart.items.len() as u32)));
        assert!(
            stacks(&wart)
                .iter()
                .all(|(item, ..)| *item == "minecraft:nether_wart")
        );
    }

    #[test]
    fn the_random_rules_follow_the_era_draw_order() {
        // Values from `java.util.Random(seed)` with the 1.7.10 methods
        // transcribed from the sources: the count draw, one luck float per
        // `dropItems` iteration, the item draw, the position floats, then
        // experience. Each `dye` etc. is one entity.
        fn fortune(level: u32) -> Tool {
            Tool {
                fortune: level,
                ..Tool::default()
            }
        }
        // `OreBlock`: coal's experience, and lapis' count with fortune 0 and 2.
        let coal = drops_with("minecraft:coal_ore", 0, fortune(3), true, 7);
        assert_eq!(stacks(&coal), [("minecraft:coal", 1, 0)]);
        assert_eq!(coal.experience, 1);
        let lapis = drops_with("minecraft:lapis_ore", 0, fortune(0), true, 7);
        assert_eq!(stacks(&lapis), [("minecraft:dye", 1, 4); 5]);
        assert_eq!(lapis.experience, 2);
        let lapis = drops_with("minecraft:lapis_ore", 0, fortune(2), true, 7);
        assert_eq!(stacks(&lapis), [("minecraft:dye", 1, 4); 16]);
        assert_eq!(lapis.experience, 2);
        // `RedstoneOreBlock`: six dust and five experience at fortune 1.
        let redstone = drops_with("minecraft:redstone_ore", 0, fortune(1), true, 7);
        assert_eq!(stacks(&redstone), [("minecraft:redstone", 1, 0); 6]);
        assert_eq!(redstone.experience, 5);
        // `GlowstoneBlock` and `MelonBlock` fortune counts.
        assert_eq!(
            stacks(&drops_with("minecraft:glowstone", 0, fortune(2), true, 9)),
            [("minecraft:glowstone_dust", 1, 0); 4]
        );
        assert_eq!(
            stacks(&drops_with("minecraft:melon_block", 0, fortune(1), true, 9)),
            [("minecraft:melon", 1, 0); 7]
        );
        // `GravelBlock.getDropItem`: flint at seed 3, the gravel otherwise.
        assert_eq!(
            stacks(&drops_with("minecraft:gravel", 0, Tool::default(), true, 3)),
            [("minecraft:flint", 1, 0)]
        );
        assert_eq!(
            stacks(&drops_with("minecraft:gravel", 0, Tool::default(), true, 0)),
            [("minecraft:gravel", 1, 0)]
        );
        assert_eq!(
            stacks(&drops_with("minecraft:gravel", 0, fortune(3), true, 7)),
            [("minecraft:flint", 1, 0)]
        );
        // `AbstractLeavesBlock`: a sapling at seed 0, an apple at 329, the
        // jungle chance and the leaves2 sapling metadata.
        assert_eq!(
            stacks(&drops_with("minecraft:leaves", 0, Tool::default(), true, 0)),
            [("minecraft:sapling", 1, 0)]
        );
        assert_eq!(
            stacks(&drops_with(
                "minecraft:leaves",
                0,
                Tool::default(),
                true,
                329
            )),
            [("minecraft:apple", 1, 0)]
        );
        assert_eq!(
            stacks(&drops_with("minecraft:leaves", 3, Tool::default(), true, 0)),
            [("minecraft:sapling", 1, 3)]
        );
        assert_eq!(
            stacks(&drops_with(
                "minecraft:leaves2",
                1,
                Tool::default(),
                true,
                0
            )),
            [("minecraft:sapling", 1, 5)]
        );
        // `TallPlantBlock`: one roll for the seed at age 2.
        assert_eq!(
            stacks(&drops_with("minecraft:tallgrass", 2, fortune(1), true, 5)),
            [("minecraft:wheat_seeds", 1, 0)]
        );
        // `WheatBlock`: the plant, then four seed rolls at age 7, fortune 1.
        assert_eq!(
            stacks(&drops_with("minecraft:wheat", 7, fortune(1), true, 5)),
            [
                ("minecraft:wheat", 1, 0),
                ("minecraft:wheat_seeds", 1, 0),
                ("minecraft:wheat_seeds", 1, 0),
                ("minecraft:wheat_seeds", 1, 0),
                ("minecraft:wheat_seeds", 1, 0),
            ]
        );
        assert_eq!(
            stacks(&drops_with("minecraft:potatoes", 7, fortune(3), true, 5)),
            [("minecraft:potato", 1, 0); 7]
        );
        // `MushroomBlock.getBaseDropCount` and `NetherWartBlock`.
        assert_eq!(
            stacks(&drops_with(
                "minecraft:brown_mushroom_block",
                0,
                Tool::default(),
                true,
                11
            )),
            [("minecraft:brown_mushroom", 1, 0)]
        );
        assert_eq!(
            stacks(&drops_with(
                "minecraft:nether_wart",
                2,
                Tool::default(),
                true,
                13
            )),
            [("minecraft:nether_wart", 1, 0)]
        );
        assert_eq!(
            stacks(&drops_with(
                "minecraft:nether_wart",
                3,
                fortune(2),
                true,
                13
            )),
            [("minecraft:nether_wart", 1, 0); 4]
        );
        // `SnowLayerBlock`: one stack of `layers + 1` snowballs.
        assert_eq!(
            stacks(&drops_with(
                "minecraft:snow_layer",
                3,
                Tool::default(),
                true,
                0
            )),
            [("minecraft:snowball", 4, 0)]
        );
    }

    #[test]
    fn every_era_block_answers() {
        // The era export's block names, `Block.REGISTRY` 1.7.10 registers.
        let blocks = [
            "air",
            "stone",
            "grass",
            "dirt",
            "cobblestone",
            "planks",
            "sapling",
            "bedrock",
            "flowing_water",
            "water",
            "flowing_lava",
            "lava",
            "sand",
            "gravel",
            "gold_ore",
            "iron_ore",
            "coal_ore",
            "log",
            "leaves",
            "sponge",
            "glass",
            "lapis_ore",
            "lapis_block",
            "dispenser",
            "sandstone",
            "noteblock",
            "bed",
            "golden_rail",
            "detector_rail",
            "sticky_piston",
            "web",
            "tallgrass",
            "deadbush",
            "piston",
            "piston_head",
            "wool",
            "piston_extension",
            "yellow_flower",
            "red_flower",
            "brown_mushroom",
            "red_mushroom",
            "gold_block",
            "iron_block",
            "double_stone_slab",
            "stone_slab",
            "brick_block",
            "tnt",
            "bookshelf",
            "mossy_cobblestone",
            "obsidian",
            "torch",
            "fire",
            "mob_spawner",
            "oak_stairs",
            "chest",
            "redstone_wire",
            "diamond_ore",
            "diamond_block",
            "crafting_table",
            "wheat",
            "farmland",
            "furnace",
            "lit_furnace",
            "standing_sign",
            "wooden_door",
            "ladder",
            "rail",
            "stone_stairs",
            "wall_sign",
            "lever",
            "stone_pressure_plate",
            "iron_door",
            "wooden_pressure_plate",
            "redstone_ore",
            "lit_redstone_ore",
            "unlit_redstone_torch",
            "redstone_torch",
            "stone_button",
            "snow_layer",
            "ice",
            "snow",
            "cactus",
            "clay",
            "reeds",
            "jukebox",
            "fence",
            "pumpkin",
            "netherrack",
            "soul_sand",
            "glowstone",
            "portal",
            "lit_pumpkin",
            "cake",
            "unpowered_repeater",
            "powered_repeater",
            "stained_glass",
            "trapdoor",
            "monster_egg",
            "stonebrick",
            "brown_mushroom_block",
            "red_mushroom_block",
            "iron_bars",
            "glass_pane",
            "melon_block",
            "pumpkin_stem",
            "melon_stem",
            "vine",
            "fence_gate",
            "brick_stairs",
            "stone_brick_stairs",
            "mycelium",
            "waterlily",
            "nether_brick",
            "nether_brick_fence",
            "nether_brick_stairs",
            "nether_wart",
            "enchanting_table",
            "brewing_stand",
            "cauldron",
            "end_portal",
            "end_portal_frame",
            "end_stone",
            "dragon_egg",
            "redstone_lamp",
            "lit_redstone_lamp",
            "double_wooden_slab",
            "wooden_slab",
            "cocoa",
            "sandstone_stairs",
            "emerald_ore",
            "ender_chest",
            "tripwire_hook",
            "tripwire",
            "emerald_block",
            "spruce_stairs",
            "birch_stairs",
            "jungle_stairs",
            "command_block",
            "beacon",
            "cobblestone_wall",
            "flower_pot",
            "carrots",
            "potatoes",
            "wooden_button",
            "skull",
            "anvil",
            "trapped_chest",
            "light_weighted_pressure_plate",
            "heavy_weighted_pressure_plate",
            "unpowered_comparator",
            "powered_comparator",
            "daylight_detector",
            "redstone_block",
            "quartz_ore",
            "hopper",
            "quartz_block",
            "quartz_stairs",
            "activator_rail",
            "dropper",
            "stained_hardened_clay",
            "stained_glass_pane",
            "leaves2",
            "log2",
            "acacia_stairs",
            "dark_oak_stairs",
            "hay_block",
            "carpet",
            "hardened_clay",
            "coal_block",
            "packed_ice",
            "double_plant",
        ];
        assert_eq!(blocks.len(), 171);
        for block in blocks {
            for meta in 0..16u8 {
                let name = format!("minecraft:{block}");
                // Every rule must answer without panicking for every metadata,
                // with and without silk touch.
                for tool in [plain(), silk()] {
                    let drops = drops_with(&name, meta, tool, true, 7);
                    assert!(drops.items.len() <= 64, "{block}:{meta}");
                }
            }
        }
    }

    #[test]
    fn do_tile_drops_off_lists_nothing_but_still_finishes() {
        // `Block.dropItem` returns before its offsets, so the experience is
        // still awarded.
        let off = drops_with("minecraft:coal_ore", 0, plain(), false, 4242);
        assert!(off.items.is_empty());
        assert!((0..=2).contains(&off.experience));
    }

    #[test]
    fn a_drop_lands_inside_the_block() {
        // `Block.dropItem`: each axis is the block corner plus 0.15..0.85.
        let block = [10, 64, -3];
        for drop in drops("minecraft:stone", 0, plain()).items {
            for axis in 0..3 {
                let corner = f64::from(block[axis]);
                assert!(
                    (corner + 0.15..corner + 0.85).contains(&drop.position[axis]),
                    "{:?}",
                    drop.position
                );
            }
        }
    }
}
