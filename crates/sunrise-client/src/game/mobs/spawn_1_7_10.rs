//! 1.7.10 natural spawning: `NaturalSpawner`'s per-tick pass and the
//! generation-time animal packs (`spawnOriginalMobs`), run for a
//! [`Profile::V1_7_10`](sunrise_core::profile::Profile::V1_7_10) world where a
//! 26.3 world runs the spawner in `natural_spawning`.
//!
//! The pass is the era's: a union of every player's 17×17 chunk square with
//! the square's border marked ineligible, the per-category caps scaled by
//! `chunks / 256`, one random point in each chunk, three groups of up to four
//! jittered attempts, `isValidSpawnPos`, the 24-block player and 24-block world
//! spawn keep-outs, the spawn table's weighted pick, and the mob's own
//! `canSpawn` (monsters' sky and brightness rolls, animals' grass and light).
//! Creatures try only every 400 ticks, as `MobCategory.CREATURE` is rare.
//!
//! The tables are `Biome`'s own constructor lists: the era export carries no
//! per-biome additions, so the forest's wolf, the plains' horse, the jungle's
//! ocelot and the mushroom island's mooshroom are not in the pick (the eight
//! types the era's `entity-types.json` carries are the only ones the level can
//! create).
//!
//! A fresh chunk also gets `NaturalSpawner.populateChunk`: the animal table in
//! groups of four while `getAnimalSpawnChance` (0.1) keeps winning.
//! The era generator's random after decoration is not reachable across the
//! crate seam (`sunset_1_7_features::populate::populate_chunk` owns it), so the
//! pack pass starts from the chunk's own decoration seed mixed with
//! [`POPULATION_SALT`]; the table, the weights, the group sizes and the rules
//! are the era's.
use super::{Dim, Game};
use sunrise_client::blocks::StateFacts;
use sunrise_client::collisions::{SceneCollisions, no_collision_empty};
use sunrise_client::mobs::Mob;
use sunrise_core::geometry::{Aabb, Vec3};
use sunrise_core::random::{LegacyRandom, RandomSource};

/// The categories of 1.7.10's `MobCategory`, in the enum's order, with the
/// cap, peaceful and rare flags `NaturalSpawner.tick` filters on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Category {
    Monster,
    Creature,
    Ambient,
    WaterCreature,
}

impl Category {
    /// `MobCategory.values()`.
    pub(super) const VALUES: [Self; 4] = [
        Self::Monster,
        Self::Creature,
        Self::Ambient,
        Self::WaterCreature,
    ];

    /// `MobCategory.getCap()`, the cap per 256 spawning chunks.
    pub(super) const fn cap(self) -> i32 {
        match self {
            Self::Monster => 70,
            Self::Creature => 10,
            Self::Ambient => 15,
            Self::WaterCreature => 5,
        }
    }

    /// `MobCategory.isPeaceful()`: spawns while `spawnAnimals` is on, and
    /// never on peaceful difficulty.
    pub(super) const fn peaceful(self) -> bool {
        !matches!(self, Self::Monster)
    }

    /// `MobCategory.isRare()`: `NaturalSpawner.tick` tries it only when the
    /// world time is a multiple of 400.
    pub(super) const fn rare(self) -> bool {
        matches!(self, Self::Creature)
    }

    /// `MobCategory.getSpawnableMaterial() == Material.WATER`.
    pub(super) const fn water(self) -> bool {
        matches!(self, Self::WaterCreature)
    }

    /// The era class a type belongs to, from its `entity-types.json` category.
    fn of_mob(kind: &str) -> Option<Self> {
        Some(match kind {
            "monster" => Self::Monster,
            "creature" => Self::Creature,
            "ambient" => Self::Ambient,
            "water_creature" => Self::WaterCreature,
            _ => return None,
        })
    }
}

/// One `Biome$SpawnEntry`: the type's id, its weight and its group size.
#[derive(Clone, Copy, Debug)]
pub(super) struct SpawnEntry {
    pub(super) kind: &'static str,
    pub(super) weight: i32,
    pub(super) min_group: i32,
    pub(super) max_group: i32,
}

/// `MobEntity.getLimitPerChunk()`: 4 for every type the era export carries
/// (only wolves, horses and ghasts override it, and none of them is here).
const LIMIT_PER_CHUNK: i32 = 4;

/// `Biome.getAnimalSpawnChance()`: the base class's 0.1; no 1.7.10 biome
/// overrides it.
pub(super) const ANIMAL_SPAWN_CHANCE: f32 = 0.1;

/// `Biome`'s `passiveEntries`, in the constructor's order.
pub(super) const CREATURE_ENTRIES: &[SpawnEntry] = &[
    SpawnEntry {
        kind: "minecraft:sheep",
        weight: 12,
        min_group: 4,
        max_group: 4,
    },
    SpawnEntry {
        kind: "minecraft:pig",
        weight: 10,
        min_group: 4,
        max_group: 4,
    },
    SpawnEntry {
        kind: "minecraft:chicken",
        weight: 10,
        min_group: 4,
        max_group: 4,
    },
    SpawnEntry {
        kind: "minecraft:cow",
        weight: 8,
        min_group: 4,
        max_group: 4,
    },
];

/// `Biome`'s `monsterEntries`, in the constructor's order. Slime, enderman
/// and witch are in the era's table even though the era export carries no
/// model for them: the pick is vanilla's, and a type the level does not know
/// simply creates nothing.
pub(super) const MONSTER_ENTRIES: &[SpawnEntry] = &[
    SpawnEntry {
        kind: "minecraft:spider",
        weight: 100,
        min_group: 4,
        max_group: 4,
    },
    SpawnEntry {
        kind: "minecraft:zombie",
        weight: 100,
        min_group: 4,
        max_group: 4,
    },
    SpawnEntry {
        kind: "minecraft:skeleton",
        weight: 100,
        min_group: 4,
        max_group: 4,
    },
    SpawnEntry {
        kind: "minecraft:creeper",
        weight: 100,
        min_group: 4,
        max_group: 4,
    },
    SpawnEntry {
        kind: "minecraft:slime",
        weight: 100,
        min_group: 4,
        max_group: 4,
    },
    SpawnEntry {
        kind: "minecraft:enderman",
        weight: 10,
        min_group: 1,
        max_group: 4,
    },
    SpawnEntry {
        kind: "minecraft:witch",
        weight: 5,
        min_group: 1,
        max_group: 1,
    },
];

/// `Biome`'s `waterEntries`.
pub(super) const WATER_ENTRIES: &[SpawnEntry] = &[SpawnEntry {
    kind: "minecraft:squid",
    weight: 10,
    min_group: 4,
    max_group: 4,
}];

/// `Biome`'s `ambientEntries`.
pub(super) const AMBIENT_ENTRIES: &[SpawnEntry] = &[SpawnEntry {
    kind: "minecraft:bat",
    weight: 10,
    min_group: 8,
    max_group: 8,
}];

impl Category {
    /// `Biome.getSpawnEntries(category)`.
    pub(super) const fn entries(self) -> &'static [SpawnEntry] {
        match self {
            Self::Monster => MONSTER_ENTRIES,
            Self::Creature => CREATURE_ENTRIES,
            Self::Ambient => AMBIENT_ENTRIES,
            Self::WaterCreature => WATER_ENTRIES,
        }
    }
}

/// The fixed mix the generation pack pass takes from a decoration seed; see
/// the module docs.
const POPULATION_SALT: i64 = 0x5DEE_CE66_D1CE_4E5B;

/// `WeightedPicker.pick`: one `nextInt(totalWeight)` draw, then subtract each
/// weight in order.
pub(super) fn weighted_pick<'a>(
    random: &mut LegacyRandom,
    entries: &'a [SpawnEntry],
) -> Option<&'a SpawnEntry> {
    let total: i32 = entries.iter().map(|entry| entry.weight).sum();
    if total <= 0 {
        return None;
    }
    let mut roll = random.next_int_bound(total).unwrap_or(0);
    for entry in entries {
        roll -= entry.weight;
        if roll < 0 {
            return Some(entry);
        }
    }
    None
}

/// `NaturalSpawner.getRandomPosInChunk`'s `nextInt` bound: the chunk's highest
/// non-empty section's offset plus 16, minus one; the dimension's height when
/// the chunk is absent.
pub(super) fn position_bound(chunk_top: Option<i32>, dimension_height: i32) -> i32 {
    chunk_top.map_or(dimension_height, |top| top + 16 - 1)
}

/// `NaturalSpawner.tick`'s category gate: `count <= cap * chunks / 256`.
pub(super) fn cap_allows(count: i32, cap: i32, chunks: usize) -> bool {
    count <= cap * chunks as i32 / 256
}

/// `MonsterEntity.canSpawnAtLightLevel` after its two draws: the sky-light
/// roll against 32, then the raw brightness (sky minus ambient darkness, or
/// block light, whichever is higher) against 8. Thundering makes the ambient
/// darkness 10 for the second read.
pub(super) fn monster_light_allows(
    sky: u8,
    block: u8,
    ambient: i32,
    thundering: bool,
    sky_roll: i32,
    light_roll: i32,
) -> bool {
    if i32::from(sky) > sky_roll {
        return false;
    }
    let ambient = if thundering { 10 } else { ambient };
    let brightness = i32::from(block).max(i32::from(sky) - ambient);
    brightness <= light_roll
}

/// `AnimalEntity.canSpawn`'s light half: `World.getActualLight > 8`. The era's
/// `getActualLight` passes no ambient darkness, so this is the higher of the
/// sky and block light at the feet.
pub(super) fn animal_light_allows(sky: u8, block: u8) -> bool {
    i32::from(sky.max(block)) > 8
}

/// `NaturalSpawner.populateChunk`'s group size:
/// `minGroupSize + random.nextInt(1 + maxGroupSize - minGroupSize)`.
pub(super) fn group_size(entry: &SpawnEntry, random: &mut LegacyRandom) -> i32 {
    entry.min_group
        + random
            .next_int_bound(1 + entry.max_group - entry.min_group)
            .unwrap_or(0)
}

/// `next.nextInt(bound) - next.nextInt(bound)`: the first draw is the positive
/// half.
fn jitter(random: &mut LegacyRandom, bound: i32) -> i32 {
    random.next_int_bound(bound).unwrap_or(0) - random.next_int_bound(bound).unwrap_or(0)
}

impl Game<'_> {
    /// `NaturalSpawner.tick` for the level in place. The spawner of a 26.3
    /// world is `spawn_natural`; this is the era's, run from `tick_spawners`.
    pub(super) fn spawn_natural_1_7_10(&mut self) {
        if self.world_debug
            || !self
                .ui
                .rules
                .bool_at(sunrise_client::gamerules::vanilla::SPAWN_MOBS)
        {
            return;
        }
        let players = self.level_players();
        if players.is_empty() {
            return;
        }
        let chunks = era_spawning_chunks(&players);
        if chunks.is_empty() {
            return;
        }
        let spawn_point = self.level.spawn.unwrap_or([0.0, 64.0, 0.0]);
        let spawn_animals = true;
        // `MinecraftServer`: a single player's `setAllowedMobSpawns` passes
        // `difficulty != PEACEFUL`, and `MonsterEntity.canSpawn` refuses there
        // anyway; a dedicated server's spawner always passes `true`.
        let spawn_monsters = self.ui.difficulty != 0;
        let spawn_rare = self.world.game_time % 400 == 0;
        for category in Category::VALUES {
            if (category.peaceful() && !spawn_animals) || (!category.peaceful() && !spawn_monsters)
            {
                continue;
            }
            if category.rare() && !spawn_rare {
                continue;
            }
            let count = self.era_category_count(category);
            if !cap_allows(count, category.cap(), chunks.len()) {
                continue;
            }
            self.era_spawn_category(category, &chunks, spawn_point);
        }
    }

    /// Mobs of a category the caps count: every loaded mob that is not
    /// persistent (`World.getEntityCount`).
    fn era_category_count(&self, category: Category) -> i32 {
        self.mobs
            .iter()
            .filter(|mob| !mob.removed && !mob.persistent)
            .filter(|mob| Category::of_mob(&mob.kind.category) == Some(category))
            .count() as i32
    }

    /// One category's pass over the eligible chunks (`NaturalSpawner.tick`'s
    /// `label128` loop).
    fn era_spawn_category(
        &mut self,
        category: Category,
        chunks: &[(i32, i32, bool)],
        spawn_point: [f64; 3],
    ) {
        'chunks: for &(chunk_x, chunk_z, ineligible) in chunks {
            if ineligible {
                continue;
            }
            let Some((x, y, z)) = self.era_random_pos_in_chunk(chunk_x, chunk_z) else {
                continue;
            };
            if !self.era_spawnable_material(category, x, y, z) {
                continue;
            }
            let mut spawned = 0;
            for _ in 0..3 {
                let (mut try_x, mut try_y, mut try_z) = (x, y, z);
                let mut entry: Option<&'static SpawnEntry> = None;
                for _ in 0..4 {
                    try_x += jitter(&mut self.world.random, 6);
                    try_y += jitter(&mut self.world.random, 1);
                    try_z += jitter(&mut self.world.random, 6);
                    if !self.era_valid_spawn_pos(category, try_x, try_y, try_z) {
                        continue;
                    }
                    let at = [
                        f64::from(try_x) + 0.5,
                        f64::from(try_y),
                        f64::from(try_z) + 0.5,
                    ];
                    if self.players_near(at, 24.0) {
                        continue;
                    }
                    let (dx, dy, dz) = (
                        at[0] as f32 - spawn_point[0] as f32,
                        at[1] as f32 - spawn_point[1] as f32,
                        at[2] as f32 - spawn_point[2] as f32,
                    );
                    if dx * dx + dy * dy + dz * dz < 576.0 {
                        continue;
                    }
                    if entry.is_none() {
                        entry = weighted_pick(&mut self.world.random, category.entries());
                        if entry.is_none() {
                            break;
                        }
                    }
                    let entry = entry.expect("the pick above set it");
                    let yaw = self.world.random.next_float() * 360.0;
                    if self.era_try_spawn(entry.kind, at, yaw) {
                        spawned += 1;
                        if spawned >= LIMIT_PER_CHUNK {
                            continue 'chunks;
                        }
                    }
                }
            }
        }
    }

    /// `getRandomPosInChunk`: x and z in the chunk, y in the chunk's top
    /// section (or the whole dimension for a chunk the level has not got).
    fn era_random_pos_in_chunk(&mut self, chunk_x: i32, chunk_z: i32) -> Option<(i32, i32, i32)> {
        let top = self.era_chunk_top(chunk_x, chunk_z);
        if top.is_none() && !self.world.chunks.contains_key(&(chunk_x, chunk_z)) {
            return None;
        }
        let bound = position_bound(top, self.world.layout().max_y());
        let x = chunk_x * 16 + self.world.random.next_int_bound(16).unwrap_or(0);
        let z = chunk_z * 16 + self.world.random.next_int_bound(16).unwrap_or(0);
        let y = self.world.random.next_int_bound(bound).unwrap_or(0);
        Some((x, y, z))
    }

    /// `WorldChunk.getHighestSectionOffset`: the offset of the highest
    /// section that holds a block, `0` for an empty column, `None` when the
    /// level has no such chunk.
    fn era_chunk_top(&self, chunk_x: i32, chunk_z: i32) -> Option<i32> {
        let chunk = self.world.chunks.get(&(chunk_x, chunk_z))?;
        let min_y = self.world.layout().min_y();
        Some(
            chunk
                .sections
                .iter()
                .enumerate()
                .rev()
                .find(|(_, section)| !section.is_uniform(self.world.air.0))
                .map_or(0, |(index, _)| min_y + (index as i32) * 16),
        )
    }

    /// `!block.isSolid() && block.getMaterial() == category.getSpawnableMaterial()`.
    fn era_spawnable_material(&self, category: Category, x: i32, y: i32, z: i32) -> bool {
        if category.water() {
            return self.era_is_water(x, y, z);
        }
        self.world.block(x, y, z) == self.world.air
    }

    /// `NaturalSpawner.isValidSpawnPos(category, world, x, y, z)`.
    fn era_valid_spawn_pos(&self, category: Category, x: i32, y: i32, z: i32) -> bool {
        if category.water() {
            return self.era_is_liquid(x, y, z)
                && self.era_is_liquid(x, y - 1, z)
                && !self.era_is_solid(x, y + 1, z);
        }
        self.era_has_solid_top(x, y - 1, z)
            && !self.era_is_named(x, y - 1, z, "minecraft:bedrock")
            && !self.era_is_solid(x, y, z)
            && !self.era_is_liquid(x, y, z)
            && !self.era_is_solid(x, y + 1, z)
    }

    /// `World.hasSolidTop`: `material.isSolidBlocking() && isCube()`, then the
    /// four class overrides for the blocks that are no cubes.
    fn era_has_solid_top(&self, x: i32, y: i32, z: i32) -> bool {
        let state = self.world.block(x, y, z);
        let facts = self.era_facts(state);
        // The export's `Block.isSolid()` is `material.isSolidBlocking() &&
        // isCube() && !isSignalSource()`: the translucent blocks (glass, ice,
        // leaves) and every non-cube answer false, as the material half does.
        // Only a player's signal-source block (a redstone block) is a solid top
        // this reads as none.
        if facts.legacy_solid {
            return true;
        }
        let meta = self.era_meta(state);
        facts.classes.iter().any(|class| match class.as_ref() {
            "StairsBlock" => meta & 4 == 4,
            "SlabBlock" => meta & 8 == 8,
            "HopperBlock" => true,
            "SnowLayerBlock" => meta & 7 == 7,
            _ => false,
        })
    }

    /// `Block.isSolid()` of an era state.
    fn era_is_solid(&self, x: i32, y: i32, z: i32) -> bool {
        self.era_facts(self.world.block(x, y, z)).legacy_solid
    }

    /// `Material.isLiquid()` of an era state.
    fn era_is_liquid(&self, x: i32, y: i32, z: i32) -> bool {
        let facts = self.era_facts(self.world.block(x, y, z));
        facts.fluid != sunrise_client::blocks::Fluid::None
    }

    /// `Material.WATER`: `WATER_CREATURE`'s spawnable material.
    fn era_is_water(&self, x: i32, y: i32, z: i32) -> bool {
        let facts = self.era_facts(self.world.block(x, y, z));
        facts.fluid == sunrise_client::blocks::Fluid::Water
    }

    fn era_is_named(&self, x: i32, y: i32, z: i32, name: &str) -> bool {
        self.era_facts(self.world.block(x, y, z)).name.as_ref() == name
    }

    fn era_facts(&self, state: sunrise_world::registry::BlockStateId) -> &StateFacts {
        self.res.facts().get(state)
    }

    /// The `meta` property of an era state.
    fn era_meta(&self, state: sunrise_world::registry::BlockStateId) -> u8 {
        self.pack
            .assets
            .registry
            .state(state)
            .and_then(|state| state.properties.get("meta"))
            .and_then(|meta| meta.parse().ok())
            .unwrap_or(0)
    }

    /// `World.getNearestPlayer(x, y, z, range) != null`, over the level's
    /// players.
    fn players_near(&self, at: [f64; 3], range: f64) -> bool {
        let range = range * range;
        self.level_players_iter().any(|player| {
            let p = player.position;
            let dx = p.x - at[0];
            let dy = p.y - at[1];
            let dz = p.z - at[2];
            dx * dx + dy * dy + dz * dz < range
        })
    }

    /// Create and add a mob this pass picked, running the era's `canSpawn`
    /// first (`MobEntity.canSpawn` and the category's own check). The mob's
    /// random is the one those light rolls read, as in vanilla.
    fn era_try_spawn(&mut self, kind: &str, at: [f64; 3], yaw: f32) -> bool {
        let Some(kind_type) = self.res.mob_types().get(kind).cloned() else {
            return false;
        };
        let (own, uuid) = sunrise_client::mobs::living_random();
        let id = self.next_entity + 1;
        let mut mob = Mob::with_random(id, kind_type, Vec3::new(at[0], at[1], at[2]), yaw, own);
        if !self.era_can_spawn(&mut mob) {
            return false;
        }
        self.next_entity = id;
        self.persist.uuids.insert(id, uuid);
        let difficulty = self.local_difficulty(at);
        self.finalize_era_spawn(&mut mob, &difficulty);
        self.add_era_mob(mob);
        true
    }

    /// A mob made by the generation pass: `populateChunk` adds it without
    /// `canSpawn`, after placing it on a valid spawn block.
    fn era_add_generated_mob(&mut self, kind: &str, at: [f64; 3], yaw: f32) -> bool {
        let Some(kind_type) = self.res.mob_types().get(kind).cloned() else {
            return false;
        };
        let (own, uuid) = sunrise_client::mobs::living_random();
        let id = self.next_entity + 1;
        self.next_entity = id;
        self.persist.uuids.insert(id, uuid);
        let mut mob = Mob::with_random(id, kind_type, Vec3::new(at[0], at[1], at[2]), yaw, own);
        let difficulty = self.local_difficulty(at);
        self.finalize_era_spawn(&mut mob, &difficulty);
        self.add_era_mob(mob);
        true
    }

    fn add_era_mob(&mut self, mob: Mob) {
        self.mobs.push(mob);
        self.native_mobs_joined(self.mobs.len() - 1, false);
    }

    /// `MobEntity.canSpawn` and the category's class check.
    fn era_can_spawn(&self, mob: &mut Mob) -> bool {
        let at = mob.position;
        let (x, y, z) = (
            at.x.floor() as i32,
            at.y.floor() as i32,
            at.z.floor() as i32,
        );
        let category = Category::of_mob(&mob.kind.category);
        match category {
            Some(Category::Monster) => {
                if self.ui.difficulty == 0 {
                    return false;
                }
                let light = self.world.light(x, y, z);
                let sky_roll = mob.random.next_int_bound(32).unwrap_or(0);
                let light_roll = mob.random.next_int_bound(8).unwrap_or(0);
                if !monster_light_allows(
                    light >> 4,
                    light & 0x0F,
                    self.sky_darken(),
                    self.world.weather.thundering,
                    sky_roll,
                    light_roll,
                ) {
                    return false;
                }
            }
            Some(Category::Creature) => {
                let below = self.era_facts(self.world.block(x, y - 1, z));
                if below.name.as_ref() != "minecraft:grass" {
                    return false;
                }
                let light = self.world.light(x, y, z);
                if !animal_light_allows(light >> 4, light & 0x0F) {
                    return false;
                }
            }
            _ => {}
        }
        let bounds = mob.bounds();
        self.era_unobstructed(bounds)
            && self.era_no_collision(bounds)
            && !self.era_liquid_in(bounds)
    }

    /// `World.isUnobstructed(shape)`: no player and no live mob overlaps the
    /// box.
    fn era_unobstructed(&self, bounds: Aabb) -> bool {
        if self
            .mobs
            .iter()
            .any(|mob| !mob.removed && mob.bounds().intersects(bounds))
        {
            return false;
        }
        !self
            .level_players_iter()
            .any(|player| player.bounds.intersects(bounds))
    }

    /// `noCollision(null, shape)`: the block shapes under an empty context.
    fn era_no_collision(&self, bounds: Aabb) -> bool {
        let mut scene = SceneCollisions {
            world: &self.world,
            binding: self.collision,
            subject: None,
        };
        no_collision_empty(&mut scene, bounds)
    }

    /// `World.containsLiquid(shape)`: any liquid cell the box reaches.
    fn era_liquid_in(&self, bounds: Aabb) -> bool {
        let (min, max) = (bounds.min_position(), bounds.max_position());
        for x in min.x.floor() as i32..=max.x.floor() as i32 {
            for y in min.y.floor() as i32..=max.y.floor() as i32 {
                for z in min.z.floor() as i32..=max.z.floor() as i32 {
                    if self.era_is_liquid(x, y, z) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// `NaturalSpawner.populateChunk` for one fresh chunk: the biome's animal
    /// table, groups of four while the spawn chance keeps winning.
    pub(super) fn spawn_original_mobs_1_7_10(&mut self, chunk_x: i32, chunk_z: i32) {
        if self.dim != Dim::Overworld || !self.world.chunks.contains_key(&(chunk_x, chunk_z)) {
            return;
        }
        let mut random = LegacyRandom::new(self.era_population_seed(chunk_x, chunk_z));
        let center_x = chunk_x * 16 + 8;
        let center_z = chunk_z * 16 + 8;
        while random.next_float() < ANIMAL_SPAWN_CHANCE {
            let Some(entry) = weighted_pick(&mut self.world.random, CREATURE_ENTRIES) else {
                return;
            };
            let entry = *entry;
            let group = group_size(&entry, &mut random);
            let mut x = center_x + random.next_int_bound(16).unwrap_or(0);
            let mut z = center_z + random.next_int_bound(16).unwrap_or(0);
            let (origin_x, origin_z) = (x, z);
            for _ in 0..group {
                let mut placed = false;
                for _ in 0..4 {
                    if !placed {
                        let y = self.era_surface_height(x, z);
                        if self.era_valid_spawn_pos(Category::Creature, x, y, z) {
                            let at = [f64::from(x) + 0.5, f64::from(y), f64::from(z) + 0.5];
                            let yaw = random.next_float() * 360.0;
                            self.era_add_generated_mob(entry.kind, at, yaw);
                            placed = true;
                        }
                    }
                    x += jitter(&mut random, 5);
                    z += jitter(&mut random, 5);
                    while x < center_x || x >= center_x + 16 || z < center_z || z >= center_z + 16 {
                        x = origin_x + jitter(&mut random, 5);
                        z = origin_z + jitter(&mut random, 5);
                    }
                    if placed {
                        break;
                    }
                }
            }
        }
    }

    /// The chunk's decoration seed (`OverworldChunkGenerator.populateChunk`'s
    /// two odd halves), mixed so the pack pass does not replay the decoration
    /// draws. Java's `long` arithmetic wraps, so the products do too.
    fn era_population_seed(&self, chunk_x: i32, chunk_z: i32) -> i64 {
        let seed = self.level.seed;
        let mut random = LegacyRandom::new(seed);
        let half_a = random.next_long() / 2 * 2 + 1;
        let half_b = random.next_long() / 2 * 2 + 1;
        (i64::from(chunk_x)
            .wrapping_mul(half_a)
            .wrapping_add(i64::from(chunk_z).wrapping_mul(half_b))
            ^ seed)
            ^ POPULATION_SALT
    }

    /// `World.getSurfaceHeight`: one above the highest block that blocks
    /// movement and is not leaves, from the top of the highest non-empty
    /// section down.
    fn era_surface_height(&self, x: i32, z: i32) -> i32 {
        let Some(top) = self.era_chunk_top(x >> 4, z >> 4) else {
            return -1;
        };
        for y in (1..=top + 15).rev() {
            let state = self.world.block(x, y, z);
            let facts = self.era_facts(state);
            // `Material.blocksMovement()`: the air, liquid and plant
            // materials, which the era's world generation also places, do
            // not hold a surface; leaves are skipped by name on top.
            if facts.name.ends_with("leaves")
                || self.res.facts().is_air(state)
                || facts.fluid != sunrise_client::blocks::Fluid::None
                || facts
                    .classes
                    .iter()
                    .any(|class| class.as_ref() == "PlantBlock")
            {
                continue;
            }
            return y + 1;
        }
        -1
    }
}

/// `NaturalSpawner.tick`'s `mobSpawningChunks`: each player's 17×17 square,
/// inner chunks eligible and the border ineligible, in insertion order. A
/// later player's inner chunk overwrites an earlier border mark, and its
/// border mark never overwrites an existing chunk.
pub(super) fn era_spawning_chunks(
    players: &[super::natural_spawning::LevelPlayer],
) -> Vec<(i32, i32, bool)> {
    use std::collections::hash_map::Entry;

    let mut order: Vec<(i32, i32, bool)> = Vec::new();
    let mut seen: std::collections::HashMap<(i32, i32), usize> = std::collections::HashMap::new();
    for player in players {
        let cx = (player.position.x / 16.0).floor() as i32;
        let cz = (player.position.z / 16.0).floor() as i32;
        for dx in -8..=8 {
            for dz in -8..=8 {
                let edge = dx == -8 || dx == 8 || dz == -8 || dz == 8;
                let pos = (dx + cx, dz + cz);
                match seen.entry(pos) {
                    // A later player's inner chunk makes its border mark
                    // eligible; a border mark never overwrites a chunk.
                    Entry::Occupied(entry) => {
                        if !edge {
                            order[*entry.get()].2 = false;
                        }
                    }
                    Entry::Vacant(entry) => {
                        entry.insert(order.len());
                        order.push((pos.0, pos.1, edge));
                    }
                }
            }
        }
    }
    order
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `MobCategory`'s four caps and flags, from the era's enum.
    #[test]
    fn categories_pin_the_era_caps_and_flags() {
        assert_eq!(
            Category::VALUES,
            [
                Category::Monster,
                Category::Creature,
                Category::Ambient,
                Category::WaterCreature
            ]
        );
        assert_eq!(Category::Monster.cap(), 70);
        assert_eq!(Category::Creature.cap(), 10);
        assert_eq!(Category::Ambient.cap(), 15);
        assert_eq!(Category::WaterCreature.cap(), 5);
        assert!(!Category::Monster.peaceful());
        assert!(Category::Creature.peaceful());
        assert!(!Category::Monster.rare());
        assert!(Category::Creature.rare());
        assert!(!Category::Ambient.rare());
        assert!(Category::WaterCreature.water());
        assert!(!Category::Monster.water());
    }

    /// `cap * chunks / 256`: one player's 289-chunk square allows 79
    /// monsters and 11 creatures, and the check is vanilla's `<=`.
    #[test]
    fn caps_scale_with_the_spawning_chunk_square() {
        assert_eq!(Category::Monster.cap() * 289 / 256, 79);
        assert_eq!(Category::Creature.cap() * 289 / 256, 11);
        assert!(cap_allows(79, 70, 289));
        assert!(!cap_allows(80, 70, 289));
        assert!(cap_allows(0, 70, 0), "vanilla's 0 <= 0 lets a pass start");
    }

    /// `MonsterEntity.canSpawnAtLightLevel`'s two thresholds: sky must beat a
    /// 0..31 roll, then the raw brightness must not exceed a 0..7 roll.
    #[test]
    fn monsters_need_sky_below_the_roll_and_brightness_at_most_seven() {
        // Sky 15 against a 14 roll fails; against 15 it passes the first gate
        // (midnight's ambient darkness 11 leaves brightness 4 for the second).
        assert!(!monster_light_allows(15, 0, 11, false, 14, 7));
        assert!(monster_light_allows(15, 0, 11, false, 15, 7));
        // Raw brightness is max(block, sky - ambient): 7 passes a 7 roll, 8
        // does not. A dark cave reads block light alone.
        assert!(monster_light_allows(0, 7, 0, false, 0, 7));
        assert!(!monster_light_allows(0, 8, 0, false, 0, 7));
        // Daytime sky 15 with ambient 0 is brightness 15: no spawn. At midnight
        // the ambient darkness 11 leaves 4.
        assert!(!monster_light_allows(15, 0, 0, false, 15, 7));
        assert!(monster_light_allows(15, 0, 11, false, 15, 7));
        // Thundering reads the brightness with ambient darkness 10.
        assert!(monster_light_allows(15, 0, 0, true, 15, 7));
    }

    /// `AnimalEntity.canSpawn`'s light half: actual light above 8, so 9.
    #[test]
    fn animals_need_light_above_eight() {
        assert!(!animal_light_allows(8, 8));
        assert!(!animal_light_allows(8, 0));
        assert!(animal_light_allows(9, 0));
        assert!(animal_light_allows(15, 4));
        assert!(animal_light_allows(0, 9));
    }

    /// `Biome`'s tables and `getAnimalSpawnChance`, in the constructor's
    /// order and with its weights and group sizes.
    #[test]
    fn biome_tables_pin_the_era_weights_and_groups() {
        let creatures: Vec<_> = CREATURE_ENTRIES
            .iter()
            .map(|e| (e.kind, e.weight, e.min_group, e.max_group))
            .collect();
        assert_eq!(
            creatures,
            [
                ("minecraft:sheep", 12, 4, 4),
                ("minecraft:pig", 10, 4, 4),
                ("minecraft:chicken", 10, 4, 4),
                ("minecraft:cow", 8, 4, 4),
            ]
        );
        let monsters: Vec<_> = MONSTER_ENTRIES
            .iter()
            .map(|e| (e.kind, e.weight, e.min_group, e.max_group))
            .collect();
        assert_eq!(
            monsters,
            [
                ("minecraft:spider", 100, 4, 4),
                ("minecraft:zombie", 100, 4, 4),
                ("minecraft:skeleton", 100, 4, 4),
                ("minecraft:creeper", 100, 4, 4),
                ("minecraft:slime", 100, 4, 4),
                ("minecraft:enderman", 10, 1, 4),
                ("minecraft:witch", 5, 1, 1),
            ]
        );
        assert_eq!(AMBIENT_ENTRIES[0].kind, "minecraft:bat");
        assert_eq!(
            (AMBIENT_ENTRIES[0].weight, AMBIENT_ENTRIES[0].min_group),
            (10, 8)
        );
        assert_eq!(WATER_ENTRIES[0].kind, "minecraft:squid");
        assert_eq!(
            (WATER_ENTRIES[0].weight, WATER_ENTRIES[0].min_group),
            (10, 4)
        );
        assert!((ANIMAL_SPAWN_CHANCE - 0.1).abs() < f32::EPSILON);
        assert_eq!(LIMIT_PER_CHUNK, 4);
    }

    /// Every 1.7.10 passive group is four wide: `4 + nextInt(1)`.
    #[test]
    fn passive_groups_are_four() {
        let mut random = LegacyRandom::new(7);
        for _ in 0..16 {
            for entry in CREATURE_ENTRIES {
                assert_eq!(group_size(entry, &mut random), 4);
            }
        }
        // The enderman's 1..4 group is the only 4-step one.
        assert_eq!(
            (MONSTER_ENTRIES[5].min_group, MONSTER_ENTRIES[5].max_group),
            (1, 4)
        );
    }

    /// `getRandomPosInChunk`'s `nextInt` bound: the top section offset plus
    /// 15, or the dimension's height for an absent chunk.
    #[test]
    fn spawn_position_bound_uses_the_top_section() {
        assert_eq!(position_bound(Some(64), 256), 79);
        assert_eq!(position_bound(Some(0), 256), 15);
        assert_eq!(position_bound(None, 256), 256);
    }

    /// One player's square is 17×17, with its 15×15 inner chunks eligible.
    #[test]
    fn one_player_has_a_289_chunk_square_with_225_eligible() {
        let player = super::super::natural_spawning::LevelPlayer {
            uuid: 0,
            position: Vec3::new(8.0, 64.0, 8.0),
            bounds: Aabb::new(Vec3::ZERO, Vec3::new(0.6, 1.8, 0.6)),
            alive: true,
            creative: false,
            spectator: false,
        };
        let chunks = era_spawning_chunks(std::slice::from_ref(&player));
        assert_eq!(chunks.len(), 289);
        assert_eq!(chunks.iter().filter(|chunk| !chunk.2).count(), 225);
    }

    /// Two overlapping squares merge, and a later player's inner chunk
    /// overwrites an earlier border mark.
    #[test]
    fn overlapping_squares_merge_their_eligible_chunks() {
        let player = |x: f64| super::super::natural_spawning::LevelPlayer {
            uuid: 0,
            position: Vec3::new(x, 64.0, 8.0),
            bounds: Aabb::new(Vec3::ZERO, Vec3::new(0.6, 1.8, 0.6)),
            alive: true,
            creative: false,
            spectator: false,
        };
        let chunks = era_spawning_chunks(&[player(8.0), player(24.0)]);
        assert_eq!(chunks.len(), 306, "18 columns by 17 rows");
        assert_eq!(
            chunks.iter().filter(|chunk| !chunk.2).count(),
            240,
            "the inner squares' union"
        );
    }

    /// Two players far apart double the square and the cap room.
    #[test]
    fn distant_players_each_bring_their_square() {
        let player = |x: f64| super::super::natural_spawning::LevelPlayer {
            uuid: 0,
            position: Vec3::new(x, 64.0, 8.0),
            bounds: Aabb::new(Vec3::ZERO, Vec3::new(0.6, 1.8, 0.6)),
            alive: true,
            creative: false,
            spectator: false,
        };
        let chunks = era_spawning_chunks(&[player(8.0), player(8.0 + 16.0 * 64.0)]);
        assert_eq!(chunks.len(), 578);
        assert_eq!(chunks.iter().filter(|chunk| !chunk.2).count(), 450);
    }

    /// `WeightedPicker.pick` subtracts weights in table order.
    #[test]
    fn weighted_pick_walks_the_table_in_order() {
        let entries = [
            SpawnEntry {
                kind: "a",
                weight: 1,
                min_group: 1,
                max_group: 1,
            },
            SpawnEntry {
                kind: "b",
                weight: 3,
                min_group: 1,
                max_group: 1,
            },
        ];
        // Seeds whose first draw is 0..3, pinned through the first draw's own
        // value rather than the pick's result.
        for seed in 0..32 {
            let mut random = LegacyRandom::new(seed);
            let roll = random.next_int_bound(4).unwrap();
            let mut again = LegacyRandom::new(seed);
            let picked = weighted_pick(&mut again, &entries).unwrap();
            assert_eq!(picked.kind, if roll == 0 { "a" } else { "b" });
        }
    }
}
