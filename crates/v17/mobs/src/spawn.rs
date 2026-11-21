//! The spawn cycle, from `world/NaturalSpawner.java` (1.7.10 `SpawnerAnimals`):
//! the per-tick hostile spawn attempt and the initial passive population.

use crate::blocks::{Material, block_by_id};
use crate::math;
use crate::mob::{Mob, MobCategory};
use crate::world::MobWorld;
use sunset_1_7_core::{LegacyRandom, RandomSource};

/// The per-tick spawn cycle. Holds the eligible chunk set between calls like
/// vanilla's `mobSpawningChunks` map.
#[derive(Default)]
pub struct NaturalSpawner {
    /// (chunk_x, chunk_z, ineligible), in insertion order. Java's `HashMap` iterates in
    /// table order; the port iterates in insertion order, which the parity harness feeds
    /// both sides.
    chunks: Vec<(i32, i32, bool)>,
}

impl NaturalSpawner {
    pub fn new() -> Self {
        NaturalSpawner::default()
    }

    /// `tick(ServerWorld, spawnAnimals, spawnMonsters, spawnRareMobs)` — one spawn
    /// attempt, returns the spawned count like vanilla.
    #[allow(clippy::too_many_arguments)]
    pub fn tick(
        &mut self,
        world: &mut dyn MobWorld,
        rng: &mut LegacyRandom,
        entity_seed: i64,
        spawn_animals: bool,
        spawn_monsters: bool,
        spawn_rare: bool,
    ) -> i32 {
        if !spawn_animals && !spawn_monsters {
            return 0;
        }

        self.chunks.clear();
        for player in world.players() {
            let cx = math::floor(player.x / 16.0);
            let cz = math::floor(player.z / 16.0);
            let radius = 8;
            for var10 in -radius..=radius {
                for var11 in -radius..=radius {
                    let edge =
                        var10 == -radius || var10 == radius || var11 == -radius || var11 == radius;
                    let pos = (var10 + cx, var11 + cz);
                    if !edge {
                        self.put(pos.0, pos.1, false);
                    } else if !self.contains(pos.0, pos.1) {
                        self.put(pos.0, pos.1, true);
                    }
                }
            }
        }

        let mut var34 = 0;
        let var35 = world.spawn_point();

        for var39 in MobCategory::VALUES {
            if (!var39.is_peaceful() || spawn_monsters)
                && (var39.is_peaceful() || spawn_animals)
                && (!var39.is_rare() || spawn_rare)
                && world.entity_count(var39) <= var39.cap() * self.chunks.len() as i32 / 256
            {
                'chunks: for &(var41_x, var41_z, ineligible) in &self.chunks {
                    if ineligible {
                        continue;
                    }
                    let (var14, var15, var16) =
                        get_random_pos_in_chunk(world, rng, var41_x, var41_z);
                    let block = block_by_id(world.get_block(var14, var15, var16));
                    if !block.get_material().is_solid()
                        && block.get_material() == var39.spawnable_material()
                    {
                        let mut var17 = 0;

                        for _var18 in 0..3 {
                            let mut var19 = var14;
                            let mut var20 = var15;
                            let mut var21 = var16;
                            let var22 = 6;
                            let mut var23: Option<SpawnEntry> = None;

                            for _var25 in 0..4 {
                                var19 += jitter(rng, var22);
                                var20 += jitter(rng, 1);
                                var21 += jitter(rng, var22);
                                if is_valid_spawn_pos(var39, world, var19, var20, var21) {
                                    let var26 = var19 as f32 + 0.5;
                                    let var27 = var20 as f32;
                                    let var28 = var21 as f32 + 0.5;
                                    if !nearest_player_within(
                                        world,
                                        var26 as f64,
                                        var27 as f64,
                                        var28 as f64,
                                        24.0,
                                    ) {
                                        let var29 = var26 - var35.0 as f32;
                                        let var30 = var27 - var35.1 as f32;
                                        let var31 = var28 - var35.2 as f32;
                                        let var32 = var29 * var29 + var30 * var30 + var31 * var31;
                                        if !(var32 < 576.0) {
                                            if var23.is_none() {
                                                let entries =
                                                    world.spawn_entries(var39, var19, var20, var21);
                                                var23 = if entries.is_empty() {
                                                    None
                                                } else {
                                                    weighted_pick(rng, &entries)
                                                };
                                                if var23.is_none() {
                                                    break;
                                                }
                                            }

                                            let mut var43 = (var23.unwrap().make)(entity_seed);
                                            var43.set_position_and_angles(
                                                var26 as f64,
                                                var27 as f64,
                                                var28 as f64,
                                                rng.next_float() * 360.0,
                                                0.0,
                                            );
                                            // initialize runs before add_mob: it only draws the
                                            // entity's own RNG (vanilla draws after addEntity).
                                            var43.initialize();
                                            let limit = var43.limit_per_chunk();
                                            if spawn_check(&var43, world) {
                                                var17 += 1;
                                                world.add_mob(var43);
                                                if var17 >= limit {
                                                    continue 'chunks;
                                                }
                                            }

                                            var34 += var17;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        var34
    }
}

/// One biome spawn entry: kind, weight, group size, constructor.
#[derive(Clone, Copy)]
pub struct SpawnEntry {
    pub kind: crate::mob::MobKind,
    pub weight: i32,
    pub min_group_size: i32,
    pub max_group_size: i32,
    /// The entity constructor: builds a mob whose own RNG draws from `seed`
    /// (replacing 1.7.10's unseeded `new Random()` for determinism).
    pub make: fn(i64) -> Mob,
}

impl SpawnEntry {
    /// `minGroupSize + random.nextInt(1 + maxGroupSize - minGroupSize)`.
    pub fn group_size(&self, rng: &mut LegacyRandom) -> i32 {
        self.min_group_size
            + rng
                .next_int_bound(1 + self.max_group_size - self.min_group_size)
                .expect("bound > 0")
    }
}

/// `WeightedPicker.pick` — the exact vanilla weighted draw.
pub fn weighted_pick(rng: &mut LegacyRandom, entries: &[SpawnEntry]) -> Option<SpawnEntry> {
    let total: i32 = entries.iter().map(|entry| entry.weight).sum();
    if total <= 0 {
        panic!("totalWeight <= 0");
    }
    let mut var3 = rng.next_int_bound(total).expect("bound > 0");
    for entry in entries {
        var3 -= entry.weight;
        if var3 < 0 {
            return Some(*entry);
        }
    }
    None
}

/// `next.nextInt(bound) - next.nextInt(bound)` — two draws, first then second.
fn jitter(rng: &mut LegacyRandom, bound: i32) -> i32 {
    rng.next_int_bound(bound).expect("bound > 0") - rng.next_int_bound(bound).expect("bound > 0")
}

/// `getRandomPosInChunk(world, chunkX, chunkZ)`.
pub fn get_random_pos_in_chunk(
    world: &dyn MobWorld,
    rng: &mut LegacyRandom,
    chunk_x: i32,
    chunk_z: i32,
) -> (i32, i32, i32) {
    let var3 = world.chunk_top(chunk_x, chunk_z);
    let var4 = chunk_x * 16 + rng.next_int_bound(16).expect("bound > 0");
    let var5 = chunk_z * 16 + rng.next_int_bound(16).expect("bound > 0");
    let var6 = rng
        .next_int_bound(match var3 {
            None => world.dimension_height(),
            Some(top) => top + 16 - 1,
        })
        .expect("bound > 0");
    (var4, var6, var5)
}

/// `isValidSpawnPos(category, world, x, y, z)`.
pub fn is_valid_spawn_pos(
    category: MobCategory,
    world: &dyn MobWorld,
    x: i32,
    y: i32,
    z: i32,
) -> bool {
    if category.spawnable_material() == Material::Water {
        return block_by_id(world.get_block(x, y, z))
            .get_material()
            .is_liquid()
            && block_by_id(world.get_block(x, y - 1, z))
                .get_material()
                .is_liquid()
            && !block_by_id(world.get_block(x, y + 1, z))
                .get_material()
                .is_solid();
    }

    if !has_solid_top(world, x, y - 1, z) {
        return false;
    }

    let var5 = block_by_id(world.get_block(x, y - 1, z));
    !var5.is_bedrock
        && !block_by_id(world.get_block(x, y, z))
            .get_material()
            .is_solid()
        && !block_by_id(world.get_block(x, y, z))
            .get_material()
            .is_liquid()
        && !block_by_id(world.get_block(x, y + 1, z))
            .get_material()
            .is_solid()
}

/// `World.hasSolidTop(world, x, y, z)` — the block is a solid full cube.
/// ponytail: only the plain-cube case; upgrade when the blocks crate exposes
/// slab/stair top surfaces.
pub fn has_solid_top(world: &dyn MobWorld, x: i32, y: i32, z: i32) -> bool {
    block_by_id(world.get_block(x, y, z))
        .get_material()
        .is_solid()
}

/// `World.getNearestPlayer(x, y, z, range) == null` — any player within range.
pub fn nearest_player_within(world: &dyn MobWorld, x: f64, y: f64, z: f64, range: f64) -> bool {
    let range_sq = range * range;
    world.players().iter().any(|player| {
        let dx = player.x - x;
        let dy = player.y - y;
        let dz = player.z - z;
        dx * dx + dy * dy + dz * dz < range_sq
    })
}

/// The entity's own `canSpawn()`, dispatched by category.
fn spawn_check(mob: &Mob, world: &dyn MobWorld) -> bool {
    match mob.kind.category() {
        MobCategory::Creature => crate::breeding::animal_can_spawn(mob, world),
        // ponytail: ambient bats and squid get their own light/water checks in vanilla;
        // upgrade when those entities are ported.
        _ => mob.pathfinder_can_spawn(world),
    }
}

impl NaturalSpawner {
    /// `mobSpawningChunks.put(pos, value)` — overwrite.
    fn put(&mut self, x: i32, z: i32, value: bool) {
        for entry in &mut self.chunks {
            if entry.0 == x && entry.1 == z {
                entry.2 = value;
                return;
            }
        }
        self.chunks.push((x, z, value));
    }

    /// `mobSpawningChunks.containsKey(pos)`.
    fn contains(&self, x: i32, z: i32) -> bool {
        self.chunks.iter().any(|entry| entry.0 == x && entry.1 == z)
    }
}

/// `populateChunk(world, biome, chunkCenterX, chunkCenterZ, rangeX, rangeZ, random)` —
/// the initial passive population. `animal_spawn_chance` is `biome.getAnimalSpawnChance()`.
#[allow(clippy::too_many_arguments)]
pub fn populate_chunk(
    world: &mut dyn MobWorld,
    rng: &mut LegacyRandom,
    entity_seed: i64,
    spawn_entries: &[SpawnEntry],
    animal_spawn_chance: f32,
    chunk_center_x: i32,
    chunk_center_z: i32,
    range_x: i32,
    range_z: i32,
) {
    if spawn_entries.is_empty() {
        return;
    }
    while rng.next_float() < animal_spawn_chance {
        let Some(var8) = weighted_pick(rng, spawn_entries) else {
            return;
        };
        let var10 = var8.group_size(rng);
        let mut var11 = chunk_center_x + rng.next_int_bound(range_x).expect("bound > 0");
        let mut var12 = chunk_center_z + rng.next_int_bound(range_z).expect("bound > 0");
        let var13 = var11;
        let var14 = var12;

        for _var15 in 0..var10 {
            let mut var16 = false;
            let mut var17 = 0;
            while !var16 && var17 < 4 {
                let var18 = world.surface_height(var11, var12);
                if is_valid_spawn_pos(MobCategory::Creature, world, var11, var18, var12) {
                    let var19 = var11 as f64 + 0.5;
                    let var20 = var18 as f64;
                    let var21 = var12 as f64 + 0.5;

                    let mut var22 = (var8.make)(entity_seed);
                    var22.set_position_and_angles(
                        var19,
                        var20,
                        var21,
                        rng.next_float() * 360.0,
                        0.0,
                    );
                    var22.initialize();
                    world.add_mob(var22);
                    var16 = true;
                }

                var11 += jitter(rng, 5);

                // the vanilla re-jitter loop, including the rangeX bound on z like vanilla
                var12 += jitter(rng, 5);
                while var11 < chunk_center_x
                    || var11 >= chunk_center_x + range_x
                    || var12 < chunk_center_z
                    || var12 >= chunk_center_z + range_x
                {
                    var11 = var13 + jitter(rng, 5);
                    var12 = var14 + jitter(rng, 5);
                }
                var17 += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::FixtureWorld;
    use crate::mob::MobKind;

    fn make_cow(_seed: i64) -> Mob {
        Mob::new(MobKind::Cow, 0.0, 0.0, 0.0)
    }

    #[test]
    fn valid_spawn_pos_on_grass() {
        let mut world = FixtureWorld::new();
        world.set_block(0, 0, 0, 2); // grass
        assert!(is_valid_spawn_pos(MobCategory::Monster, &world, 0, 1, 0));
        // air floor → no solid top
        assert!(!is_valid_spawn_pos(MobCategory::Monster, &world, 1, 1, 1));
        // bedrock floor → invalid
        world.set_block(2, 0, 2, 7);
        assert!(!is_valid_spawn_pos(MobCategory::Monster, &world, 2, 1, 2));
    }

    #[test]
    fn valid_spawn_pos_water_needs_liquid_column() {
        let mut world = FixtureWorld::new();
        world.set_block(0, 0, 0, 9); // water
        world.set_block(0, 1, 0, 9);
        assert!(is_valid_spawn_pos(
            MobCategory::WaterCreature,
            &world,
            0,
            1,
            0
        ));
        // solid above → invalid
        world.set_block(0, 2, 0, 1);
        assert!(!is_valid_spawn_pos(
            MobCategory::WaterCreature,
            &world,
            0,
            1,
            0
        ));
    }

    #[test]
    fn water_creature_needs_water_material() {
        let mut world = FixtureWorld::new();
        // air block with water below the column: feet material must be the spawnable material
        world.set_block(0, 0, 0, 9);
        assert!(!is_valid_spawn_pos(
            MobCategory::WaterCreature,
            &world,
            0,
            1,
            0
        ));
    }

    #[test]
    fn weighted_pick_respects_weights() {
        let mut rng = LegacyRandom::new(0);
        let entries = vec![
            SpawnEntry {
                kind: MobKind::Cow,
                weight: 0,
                min_group_size: 1,
                max_group_size: 1,
                make: make_cow,
            },
            SpawnEntry {
                kind: MobKind::Pig,
                weight: 1000,
                min_group_size: 1,
                max_group_size: 1,
                make: make_cow,
            },
        ];
        for _ in 0..20 {
            let picked = weighted_pick(&mut rng, &entries).unwrap();
            assert_eq!(picked.kind, MobKind::Pig, "zero weight never picked");
        }
    }

    #[test]
    fn eligible_chunk_set_marks_border_ineligible() {
        let mut world = FixtureWorld::new();
        world.add_player(0.0, 64.0, 0.0);
        let mut spawner = NaturalSpawner::new();
        let mut rng = LegacyRandom::new(0);
        let count = spawner.tick(&mut world, &mut rng, 0, true, true, true);
        // 17x17 = 289 chunks; inner 15x15 = 225 eligible
        assert_eq!(spawner.chunks.len(), 289);
        let eligible = spawner.chunks.iter().filter(|e| !e.2).count();
        assert_eq!(eligible, 225);
        // an empty air world spawns nothing
        assert_eq!(count, 0);
    }
}
