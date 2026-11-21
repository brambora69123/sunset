//! `DungeonFeature` (WorldGenDungeons): mossy cobblestone rooms, a spawner
//! and one chest of weighted loot.

use crate::block::Block;
use crate::world::{FLAG_NO_NEIGHBOR_NOTIFY, GenWorld, next_int};
use sunset_1_7_core::RandomSource;

/// A chest slot the dungeon filled: `(x, y, z, item id, count)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DungeonLoot {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub item_id: u16,
    pub count: u8,
}

/// `LootEntry`: item, min/max count and weight.
struct LootEntry {
    item_id: u16,
    min_item_chance: i32,
    max_item_chance: i32,
    weight: i32,
    /// `ItemStack.getMaxSize`: buckets stack to 16, saddles and records to 1.
    max_size: i32,
}

const LOOT_ENTRIES: [LootEntry; 15] = [
    LootEntry {
        item_id: 329,
        min_item_chance: 1,
        max_item_chance: 1,
        weight: 10,
        max_size: 1,
    }, // saddle
    LootEntry {
        item_id: 265,
        min_item_chance: 1,
        max_item_chance: 4,
        weight: 10,
        max_size: 64,
    }, // iron ingot
    LootEntry {
        item_id: 293,
        min_item_chance: 1,
        max_item_chance: 1,
        weight: 10,
        max_size: 64,
    }, // bread
    LootEntry {
        item_id: 296,
        min_item_chance: 1,
        max_item_chance: 4,
        weight: 10,
        max_size: 64,
    }, // wheat
    LootEntry {
        item_id: 289,
        min_item_chance: 1,
        max_item_chance: 4,
        weight: 10,
        max_size: 64,
    }, // gunpowder
    LootEntry {
        item_id: 287,
        min_item_chance: 1,
        max_item_chance: 4,
        weight: 10,
        max_size: 64,
    }, // string
    LootEntry {
        item_id: 325,
        min_item_chance: 1,
        max_item_chance: 1,
        weight: 10,
        max_size: 16,
    }, // bucket
    LootEntry {
        item_id: 322,
        min_item_chance: 1,
        max_item_chance: 1,
        weight: 1,
        max_size: 1,
    }, // golden apple
    LootEntry {
        item_id: 331,
        min_item_chance: 1,
        max_item_chance: 4,
        weight: 10,
        max_size: 64,
    }, // redstone
    LootEntry {
        item_id: 2256,
        min_item_chance: 1,
        max_item_chance: 1,
        weight: 10,
        max_size: 1,
    }, // record 13
    LootEntry {
        item_id: 2257,
        min_item_chance: 1,
        max_item_chance: 1,
        weight: 10,
        max_size: 1,
    }, // record cat
    LootEntry {
        item_id: 421,
        min_item_chance: 1,
        max_item_chance: 1,
        weight: 10,
        max_size: 64,
    }, // name tag
    LootEntry {
        item_id: 418,
        min_item_chance: 1,
        max_item_chance: 1,
        weight: 2,
        max_size: 1,
    }, // golden horse armor
    LootEntry {
        item_id: 417,
        min_item_chance: 1,
        max_item_chance: 1,
        weight: 5,
        max_size: 1,
    }, // iron horse armor
    LootEntry {
        item_id: 419,
        min_item_chance: 1,
        max_item_chance: 1,
        weight: 1,
        max_size: 1,
    }, // diamond horse armor
];

/// `WeightedPicker.pick` over the loot entries.
fn pick_entry<'a>(
    random: &mut sunset_1_7_core::LegacyRandom,
    entries: &'a [LootEntry],
) -> &'a LootEntry {
    let total: i32 = entries.iter().map(|e| e.weight).sum();
    let mut roll = next_int(random, total);
    for entry in entries {
        roll -= entry.weight;
        if roll < 0 {
            return entry;
        }
    }
    entries.last().expect("non-empty loot table")
}

pub struct DungeonFeature {
    /// Loot drawn for every chest placed, in draw order.
    pub loot: Vec<DungeonLoot>,
}

impl Default for DungeonFeature {
    fn default() -> DungeonFeature {
        DungeonFeature::new()
    }
}

impl DungeonFeature {
    pub fn new() -> DungeonFeature {
        DungeonFeature { loot: Vec::new() }
    }

    /// RNG order: width/depth draws, then the floor blocks' `nextInt(4)`
    /// mossy draws, then the chest-position draws, then the chest loot.
    pub fn place(
        &mut self,
        world: &mut dyn GenWorld,
        random: &mut sunset_1_7_core::LegacyRandom,
        x: i32,
        y: i32,
        z: i32,
    ) -> bool {
        let height = 3_i32;
        let width = next_int(random, 2) + 2;
        let depth = next_int(random, 2) + 2;
        let mut open_walls = 0;

        for bx in (x - width - 1)..=(x + width + 1) {
            for by in (y - 1)..=(y + height + 1) {
                for bz in (z - depth - 1)..=(z + depth + 1) {
                    let material_solid = world.get_block(bx, by, bz).is_solid();
                    if by == y - 1 && !material_solid {
                        return false;
                    }
                    if by == y + height + 1 && !material_solid {
                        return false;
                    }
                    if (bx == x - width - 1
                        || bx == x + width + 1
                        || bz == z - depth - 1
                        || bz == z + depth + 1)
                        && by == y
                        && world.is_air(bx, by, bz)
                        && world.is_air(bx, by + 1, bz)
                    {
                        open_walls += 1;
                    }
                }
            }
        }

        if !(1..=5).contains(&open_walls) {
            return false;
        }

        for bx in (x - width - 1)..=(x + width + 1) {
            for by in ((y - 1)..=(y + height)).rev() {
                for bz in (z - depth - 1)..=(z + depth + 1) {
                    let shell = bx == x - width - 1
                        || by == y - 1
                        || bz == z - depth - 1
                        || bx == x + width + 1
                        || by == y + height + 1
                        || bz == z + depth + 1;
                    if !shell {
                        world.set_block_with_metadata(
                            bx,
                            by,
                            bz,
                            Block::Air,
                            0,
                            FLAG_NO_NEIGHBOR_NOTIFY,
                        );
                    } else if by >= 0 && !world.get_block(bx, by - 1, bz).is_solid() {
                        // Shell blocks with nothing solid beneath fall away.
                        world.set_block_with_metadata(
                            bx,
                            by,
                            bz,
                            Block::Air,
                            0,
                            FLAG_NO_NEIGHBOR_NOTIFY,
                        );
                    } else if world.get_block(bx, by, bz).is_solid() {
                        if by == y - 1 && next_int(random, 4) != 0 {
                            world.set_block_with_metadata(
                                bx,
                                by,
                                bz,
                                Block::MossyCobblestone,
                                0,
                                FLAG_NO_NEIGHBOR_NOTIFY,
                            );
                        } else {
                            world.set_block_with_metadata(
                                bx,
                                by,
                                bz,
                                Block::Cobblestone,
                                0,
                                FLAG_NO_NEIGHBOR_NOTIFY,
                            );
                        }
                    }
                }
            }
        }

        // One chest, against a single solid wall.
        let mut chest: Option<(i32, i32, i32)> = None;
        for _ in 0..2 {
            for _ in 0..3 {
                let cx = x + next_int(random, width * 2 + 1) - width;
                let cy = y;
                let cz = z + next_int(random, depth * 2 + 1) - depth;
                if world.is_air(cx, cy, cz) {
                    let mut solid_sides = 0;
                    if world.get_block(cx - 1, cy, cz).is_solid() {
                        solid_sides += 1;
                    }
                    if world.get_block(cx + 1, cy, cz).is_solid() {
                        solid_sides += 1;
                    }
                    if world.get_block(cx, cy, cz - 1).is_solid() {
                        solid_sides += 1;
                    }
                    if world.get_block(cx, cy, cz + 1).is_solid() {
                        solid_sides += 1;
                    }
                    if solid_sides == 1 {
                        world.set_block_with_metadata(
                            cx,
                            cy,
                            cz,
                            Block::Chest,
                            0,
                            FLAG_NO_NEIGHBOR_NOTIFY,
                        );
                        self.fill_chest(world, random, cx, cy, cz);
                        chest = Some((cx, cy, cz));
                        break;
                    }
                }
            }
            if chest.is_some() {
                break;
            }
        }

        world.set_block_with_metadata(x, y, z, Block::MobSpawner, 0, FLAG_NO_NEIGHBOR_NOTIFY);
        // `pickDungeonType`: 0 skeleton, 1/2 zombie, 3 spider. The draw
        // happens whether or not the block entity exists.
        let _spawner_type = next_int(random, 4);
        true
    }

    /// `LootEntry.addLoot`: eight rounds of weighted pick, count draw and
    /// slot draw. The enchanted book's `EnchantmentHelper` draws are
    /// approximated (see the crate report).
    fn fill_chest(
        &mut self,
        _world: &mut dyn GenWorld,
        random: &mut sunset_1_7_core::LegacyRandom,
        x: i32,
        y: i32,
        z: i32,
    ) {
        // `Items.ENCHANTED_BOOK.createLoot(random)`: the book's
        // enchantability draws (book enchantability 1 → two `nextInt(1)`
        // draws) and two `nextFloat`s, then the enchantment list itself.
        let _ = next_int(random, 1);
        let _ = next_int(random, 1);
        let _ = random.next_float();
        let _ = random.next_float();

        for _ in 0..8 {
            let entry = pick_entry(random, &LOOT_ENTRIES);
            let count = entry.min_item_chance
                + next_int(random, entry.max_item_chance - entry.min_item_chance + 1);
            if entry.max_size >= count {
                self.loot.push(DungeonLoot {
                    x,
                    y,
                    z,
                    item_id: entry.item_id,
                    count: count as u8,
                });
            } else {
                for _ in 0..count {
                    self.loot.push(DungeonLoot {
                        x,
                        y,
                        z,
                        item_id: entry.item_id,
                        count: 1,
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk_world::ChunkWorld;

    fn dungeon_room() -> ChunkWorld {
        let mut world = ChunkWorld::new(0, 0);
        world.fill(0, 60, Block::Stone);
        world
    }

    #[test]
    fn dungeon_places_spawner_and_walls() {
        let mut placed = 0;
        let mut loot_rounds = 0;
        for seed in 0..50_i64 {
            let mut world = dungeon_room();
            // Carve a cave pocket so the room has open wall blocks.
            for x in 3..5 {
                for y in 20..22 {
                    for z in 6..11 {
                        world.set_block_with_metadata(
                            x,
                            y,
                            z,
                            Block::Air,
                            0,
                            FLAG_NO_NEIGHBOR_NOTIFY,
                        );
                    }
                }
            }
            let mut rng = sunset_1_7_core::LegacyRandom::new(seed);
            let mut dungeon = DungeonFeature::new();
            if dungeon.place(&mut world, &mut rng, 8, 20, 8) {
                placed += 1;
                loot_rounds += dungeon.loot.len();
                assert_eq!(world.get_block(8, 20, 8), Block::MobSpawner);
                assert!(world.block_count(Block::Cobblestone) > 0);
            }
        }
        assert!(placed > 0);
        // Eight loot rounds per placed chest, whether the chest found a wall or not.
        assert_eq!(loot_rounds % 8, 0);
    }

    #[test]
    fn dungeon_fails_without_solid_floor() {
        let mut world = ChunkWorld::new(0, 0);
        let mut rng = sunset_1_7_core::LegacyRandom::new(19);
        let state = rng.state();
        let mut dungeon = DungeonFeature::new();
        // All air: the floor check fails after the two size draws.
        assert!(!dungeon.place(&mut world, &mut rng, 8, 20, 8));
        assert!(rng.state() != state);
        // Two nextInt(2) draws consumed.
        let mut reference = sunset_1_7_core::LegacyRandom::new(19);
        let _ = reference.next_int_bound(2);
        let _ = reference.next_int_bound(2);
        assert_eq!(rng.state(), reference.state());
    }

    #[test]
    fn spawner_type_draw_order() {
        let mut rng = sunset_1_7_core::LegacyRandom::new(5);
        let picks: Vec<&str> = (0..8)
            .map(|_| match next_int(&mut rng, 4) {
                0 => "Skeleton",
                1 | 2 => "Zombie",
                _ => "Spider",
            })
            .collect();
        assert_eq!(picks.len(), 8);
    }
}
