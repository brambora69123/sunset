//! Biome facts the features need: temperature, surface blocks, decorator
//! attempt counts, the tree/flower/plant pickers and which `decorate`
//! override (if any) the biome runs around the base decorator body. Ported
//! from the 1.7.10 biome classes.

use crate::block::Block;
use crate::world::next_int;
use sunset_1_7_core::{LegacyRandom, RandomSource};
use sunset_1_7_noise::PerlinSimplexNoise;

/// `Biome.FOLIAGE_NOISE`: one simplex level from the fixed 2345 seed.
fn foliage_noise() -> &'static PerlinSimplexNoise {
    static NOISE: std::sync::OnceLock<PerlinSimplexNoise> = std::sync::OnceLock::new();
    NOISE.get_or_init(|| PerlinSimplexNoise::new(&mut LegacyRandom::new(2345), 1))
}

/// `FOLIAGE_NOISE.getValue(x, z)`, the noise the plains and flower forest
/// flower pickers and the plains/swamp overrides read.
pub(crate) fn foliage(x: f64, z: f64) -> f64 {
    foliage_noise().get_value(x, z)
}

/// The trees `Biome.pickTree` hands to the decorator. `Jungle` carries the
/// base height `JungleBiome.pickTree` draws before the tree is built.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TreeKind {
    Oak,
    LargeOak,
    Birch,
    TallBirch,
    Spruce,
    Pine,
    Swamp,
    Acacia,
    DarkOak,
    Bush,
    Jungle { base_height: i32 },
    GiantJungle,
    GiantPine,
    GiantSpruce,
}

/// Per-biome `pickTree` behavior.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TreePolicy {
    /// `nextInt(10) == 0` → large oak, else oak.
    Default,
    /// `ForestBiome`: variant 3 draws `nextInt(3)` for the dark oak, variant 2
    /// is birch-only and draws nothing, variants 0 and 1 draw `nextInt(5)`.
    Forest { variant: u8 },
    /// `BirchForest` mutated (`ForestBiome$99103037`): `nextBoolean`.
    BirchForestMutated,
    /// `TaigaBiome`: variants 1 and 2 draw `nextInt(3)` for a giant first.
    Taiga { variant: u8 },
    /// `SavannaBiome`: `nextInt(5) > 0` → acacia, else the plain tree.
    Savanna,
    /// `JungleBiome`: `nextInt(10)`, `nextInt(2)`, then the hill giant's
    /// `nextInt(3)` and the tree's `4 + nextInt(7)` base height.
    Jungle { edge: bool },
    /// `ExtremeHillsBiome`: `nextInt(3) > 0` → spruce, else the base pick.
    ExtremeHills,
    /// Swampland: always the swamp tree, no draw.
    Swamp,
    /// Ice plains and mesa: always this tree, no draw.
    Fixed(TreeKind),
}

impl TreePolicy {
    pub fn pick(&self, random: &mut LegacyRandom) -> TreeKind {
        match self {
            TreePolicy::Default => {
                if next_int(random, 10) == 0 {
                    TreeKind::LargeOak
                } else {
                    TreeKind::Oak
                }
            }
            TreePolicy::Forest { variant } => {
                if *variant == 3 && next_int(random, 3) > 0 {
                    return TreeKind::DarkOak;
                }
                if *variant != 2 && next_int(random, 5) != 0 {
                    TreeKind::Oak
                } else {
                    TreeKind::Birch
                }
            }
            TreePolicy::BirchForestMutated => {
                if random.next_boolean() {
                    TreeKind::TallBirch
                } else {
                    TreeKind::Birch
                }
            }
            TreePolicy::Taiga { variant } => {
                if (*variant == 1 || *variant == 2) && next_int(random, 3) == 0 {
                    if *variant != 2 && next_int(random, 13) != 0 {
                        TreeKind::GiantPine
                    } else {
                        TreeKind::GiantSpruce
                    }
                } else if next_int(random, 3) == 0 {
                    TreeKind::Pine
                } else {
                    TreeKind::Spruce
                }
            }
            TreePolicy::Savanna => {
                if next_int(random, 5) > 0 {
                    TreeKind::Acacia
                } else {
                    TreeKind::Oak
                }
            }
            TreePolicy::Jungle { edge } => {
                if next_int(random, 10) == 0 {
                    TreeKind::LargeOak
                } else if next_int(random, 2) == 0 {
                    TreeKind::Bush
                } else if !*edge && next_int(random, 3) == 0 {
                    TreeKind::GiantJungle
                } else {
                    // `new TreeFeature(false, 4 + nextInt(7), 3, 3, true)`.
                    TreeKind::Jungle {
                        base_height: 4 + next_int(random, 7),
                    }
                }
            }
            TreePolicy::ExtremeHills => {
                if next_int(random, 3) > 0 {
                    TreeKind::Spruce
                } else {
                    TreePolicy::Default.pick(random)
                }
            }
            TreePolicy::Swamp => TreeKind::Swamp,
            TreePolicy::Fixed(kind) => *kind,
        }
    }
}

/// Flower type on the red flower block: metadata 0..8 (poppy, blue orchid,
/// allium, houstonia, tulips, oxeye daisy).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FlowerPolicy {
    /// `nextInt(3) > 0` → dandelion, else poppy.
    Default,
    /// Swampland: blue orchid, no draw.
    Swamp,
    /// Plains: under the -0.8 foliage noise the dry branch draws
    /// `nextInt(4)` tulips; otherwise the usual `nextInt(3)` pair.
    Plains,
    /// Flower forest: the noise picks the flower, no draw at all.
    FlowerForest,
}

impl FlowerPolicy {
    /// Returns `(block, metadata)`; `x` and `z` are the world coordinates
    /// vanilla passes to `getRandomFlower`.
    pub fn pick(&self, random: &mut LegacyRandom, x: i32, z: i32) -> (Block, u8) {
        match self {
            FlowerPolicy::Default => {
                if next_int(random, 3) > 0 {
                    (Block::YellowFlower, 0)
                } else {
                    (Block::RedFlower, 0)
                }
            }
            FlowerPolicy::Swamp => (Block::RedFlower, 1),
            FlowerPolicy::Plains => {
                let value = foliage(f64::from(x) / 200.0, f64::from(z) / 200.0);
                if value < -0.8 {
                    (Block::RedFlower, (4 + next_int(random, 4)) as u8)
                } else if next_int(random, 3) > 0 {
                    match next_int(random, 3) {
                        0 => (Block::RedFlower, 0),
                        1 => (Block::RedFlower, 3),
                        _ => (Block::RedFlower, 8),
                    }
                } else {
                    (Block::YellowFlower, 0)
                }
            }
            FlowerPolicy::FlowerForest => {
                let value = foliage(f64::from(x) / 48.0, f64::from(z) / 48.0);
                let value = ((1.0 + value) / 2.0).clamp(0.0, 0.9999);
                let mut variant = (value * 9.0) as u8;
                if variant == 1 {
                    variant = 0;
                }
                (Block::RedFlower, variant)
            }
        }
    }
}

/// Per-biome tall grass (`pickPlant`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GrassPolicy {
    /// Tall grass, no draw.
    Default,
    /// Taiga: `nextInt(5) > 0` → fern (metadata 2), else tall grass.
    Taiga,
    /// Jungle: `nextInt(4) == 0` → fern (metadata 2), else tall grass.
    Jungle,
}

impl GrassPolicy {
    pub fn pick(&self, random: &mut LegacyRandom) -> (Block, u8) {
        match self {
            GrassPolicy::Default => (Block::TallGrass, 1),
            GrassPolicy::Taiga => {
                if next_int(random, 5) > 0 {
                    (Block::TallGrass, 2)
                } else {
                    (Block::TallGrass, 1)
                }
            }
            GrassPolicy::Jungle => {
                if next_int(random, 4) == 0 {
                    (Block::TallGrass, 2)
                } else {
                    (Block::TallGrass, 1)
                }
            }
        }
    }
}

/// The biome's own `decorate` override, run around the base decorator body.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BiomeDecorator {
    /// `Biome.decorate`: the base body only. The plain `MutatedBiome` mutants
    /// land here (`MutatedBiome.decorate` runs `original.decorator.decorate`,
    /// the original's body without its `Biome.decorate` override); the
    /// mutants built as a subclass or an anonymous class have their own entry.
    Base,
    /// `PlainsBiome`: the foliage-noise flower/grass counts, the double
    /// plants, and the sunflower variant's second pass.
    Plains { mutated: bool },
    /// `ForestBiome`: the roofed forest's 4x4 grid, then the double plants.
    Forest { variant: u8 },
    /// `SavannaBiome`: seven double plants, then the base body.
    Savanna,
    /// `DesertBiome`: the base body, then the `nextInt(1000)` well.
    Desert,
    /// `ExtremeHillsBiome`: the base body, then emerald and monster-egg veins.
    ExtremeHills,
    /// `IceBiome` spikes: three ice spikes and two ice patches, then the base.
    IceSpikes,
    /// `JungleBiome`: the base body, then a melon patch and 50 vines.
    Jungle,
    /// `TaigaBiome` variants 1 and 2: mossy boulders, then seven double plants.
    Taiga { variant: u8 },
}

/// `FeatureDecorator` attempt counts per biome.
#[derive(Clone, Copy, Debug)]
pub struct DecoratorConfig {
    pub tree_attempts: i32,
    pub flower_attempts: i32,
    pub grass_attempts: i32,
    pub dead_bush_attempts: i32,
    pub mushroom_attempts: i32,
    pub sugarcane_attempts: i32,
    pub cactus_attempts: i32,
    pub lily_pad_attempts: i32,
    pub gravel_patch_attempts: i32,
    pub sand_patch_attempts: i32,
    pub clay_patch_attempts: i32,
    pub huge_mushroom_attempts: i32,
}

impl Default for DecoratorConfig {
    fn default() -> DecoratorConfig {
        DecoratorConfig::DEFAULT
    }
}

impl DecoratorConfig {
    /// A fresh `FeatureDecorator`: two flowers, one grass, one of each patch.
    pub const DEFAULT: DecoratorConfig = DecoratorConfig {
        tree_attempts: 0,
        flower_attempts: 2,
        grass_attempts: 1,
        dead_bush_attempts: 0,
        mushroom_attempts: 0,
        sugarcane_attempts: 0,
        cactus_attempts: 0,
        lily_pad_attempts: 0,
        gravel_patch_attempts: 1,
        sand_patch_attempts: 3,
        clay_patch_attempts: 1,
        huge_mushroom_attempts: 0,
    };
}

/// Biome facts for the feature pass.
#[derive(Clone, Copy, Debug)]
pub struct BiomeDef {
    pub id: i32,
    pub temperature: f32,
    pub surface_block: Block,
    pub subsurface_block: Block,
    pub config: DecoratorConfig,
    pub tree_policy: TreePolicy,
    pub flower_policy: FlowerPolicy,
    pub grass_policy: GrassPolicy,
    pub decorator: BiomeDecorator,
}

const fn cfg(id: i32, temperature: f32, config: DecoratorConfig) -> BiomeDef {
    BiomeDef {
        id,
        temperature,
        surface_block: Block::Grass,
        subsurface_block: Block::Dirt,
        config,
        tree_policy: TreePolicy::Default,
        flower_policy: FlowerPolicy::Default,
        grass_policy: GrassPolicy::Default,
        decorator: BiomeDecorator::Base,
    }
}

/// The plains counts, shared by plains and sunflower plains.
const fn plains_config() -> DecoratorConfig {
    DecoratorConfig {
        tree_attempts: -999,
        flower_attempts: 4,
        grass_attempts: 10,
        ..DecoratorConfig::DEFAULT
    }
}

const fn desert_config() -> DecoratorConfig {
    DecoratorConfig {
        tree_attempts: -999,
        dead_bush_attempts: 2,
        sugarcane_attempts: 50,
        cactus_attempts: 10,
        ..DecoratorConfig::DEFAULT
    }
}

const fn swamp_config() -> DecoratorConfig {
    DecoratorConfig {
        tree_attempts: 2,
        flower_attempts: 1,
        dead_bush_attempts: 1,
        mushroom_attempts: 8,
        sugarcane_attempts: 10,
        lily_pad_attempts: 4,
        gravel_patch_attempts: 0,
        sand_patch_attempts: 0,
        clay_patch_attempts: 1,
        grass_attempts: 5,
        ..DecoratorConfig::DEFAULT
    }
}

/// Forest, in the constructor's terms: variant 1 is the flower forest (fewer
/// trees, a hundred flower attempts) and variant 3 the roofed forest (no
/// decorator trees at all, its own 4x4 grid instead).
const fn forest_config(variant: u8) -> DecoratorConfig {
    match variant {
        1 => DecoratorConfig {
            tree_attempts: 6,
            flower_attempts: 100,
            grass_attempts: 1,
            ..DecoratorConfig::DEFAULT
        },
        3 => DecoratorConfig {
            tree_attempts: -999,
            grass_attempts: 2,
            ..DecoratorConfig::DEFAULT
        },
        _ => DecoratorConfig {
            tree_attempts: 10,
            grass_attempts: 2,
            ..DecoratorConfig::DEFAULT
        },
    }
}

/// Taiga, in the constructor's terms: variants 1 and 2 are the mega taigas.
const fn taiga_config(variant: u8) -> DecoratorConfig {
    match variant {
        1 | 2 => DecoratorConfig {
            tree_attempts: 10,
            grass_attempts: 7,
            dead_bush_attempts: 1,
            mushroom_attempts: 3,
            ..DecoratorConfig::DEFAULT
        },
        _ => DecoratorConfig {
            tree_attempts: 10,
            grass_attempts: 1,
            mushroom_attempts: 1,
            ..DecoratorConfig::DEFAULT
        },
    }
}

/// Jungle: 50 trees, or 2 on the edge.
const fn jungle_config(edge: bool) -> DecoratorConfig {
    DecoratorConfig {
        tree_attempts: if edge { 2 } else { 50 },
        flower_attempts: 4,
        grass_attempts: 25,
        ..DecoratorConfig::DEFAULT
    }
}

/// Mesa: no flowers, twenty dead bushes, three sugarcane, five cacti; the
/// plateau F variant is the only one that keeps its trees.
const fn mesa_config(has_trees: bool) -> DecoratorConfig {
    DecoratorConfig {
        tree_attempts: if has_trees { 5 } else { -999 },
        flower_attempts: 0,
        dead_bush_attempts: 20,
        sugarcane_attempts: 3,
        cactus_attempts: 5,
        ..DecoratorConfig::DEFAULT
    }
}

const fn extreme_hills(id: i32, more_trees: bool) -> BiomeDef {
    BiomeDef {
        tree_policy: TreePolicy::ExtremeHills,
        decorator: BiomeDecorator::ExtremeHills,
        ..cfg(
            id,
            0.2,
            DecoratorConfig {
                tree_attempts: if more_trees { 3 } else { 0 },
                ..DecoratorConfig::DEFAULT
            },
        )
    }
}

const fn forest(id: i32, variant: u8, temperature: f32) -> BiomeDef {
    BiomeDef {
        tree_policy: TreePolicy::Forest { variant },
        flower_policy: if variant == 1 {
            FlowerPolicy::FlowerForest
        } else {
            FlowerPolicy::Default
        },
        decorator: BiomeDecorator::Forest { variant },
        ..cfg(id, temperature, forest_config(variant))
    }
}

const fn taiga(id: i32, variant: u8, temperature: f32) -> BiomeDef {
    BiomeDef {
        tree_policy: TreePolicy::Taiga { variant },
        grass_policy: GrassPolicy::Taiga,
        decorator: BiomeDecorator::Taiga { variant },
        ..cfg(id, temperature, taiga_config(variant))
    }
}

const fn jungle(id: i32, edge: bool) -> BiomeDef {
    BiomeDef {
        tree_policy: TreePolicy::Jungle { edge },
        grass_policy: GrassPolicy::Jungle,
        decorator: BiomeDecorator::Jungle,
        ..cfg(id, 0.95, jungle_config(edge))
    }
}

const fn mesa(id: i32, has_trees: bool) -> BiomeDef {
    BiomeDef {
        surface_block: Block::Sand,
        subsurface_block: Block::StainedHardenedClay,
        tree_policy: TreePolicy::Fixed(TreeKind::Oak),
        ..cfg(id, 2.0, mesa_config(has_trees))
    }
}

/// Biome facts by 1.7.10 id. Mutated ids (the original + 128) that
/// `Biome.mutate` builds as a plain `MutatedBiome` run the original's base
/// decorator body only: the original's counts and tree picker, with the
/// default flower and plant picks. The mutants that `mutate` turns into a
/// subclass of their own (`ExtremeHillsBiome`, `MesaBiome`, `TaigaBiome`, the
/// forest and savanna mutants) keep that class's override.
pub fn biome(id: i32) -> BiomeDef {
    match id {
        // Plains, and the sunflower plains its own `mutate` builds.
        1 | 129 => BiomeDef {
            flower_policy: FlowerPolicy::Plains,
            decorator: BiomeDecorator::Plains { mutated: id == 129 },
            ..cfg(id, 0.8, plains_config())
        },
        // Desert, hills, and the `MutatedBiome` desert M.
        2 | 17 => BiomeDef {
            surface_block: Block::Sand,
            subsurface_block: Block::Sand,
            decorator: BiomeDecorator::Desert,
            ..cfg(id, 2.0, desert_config())
        },
        130 => BiomeDef {
            surface_block: Block::Sand,
            subsurface_block: Block::Sand,
            ..cfg(130, 2.0, desert_config())
        },
        3 => extreme_hills(3, false),
        20 | 34 => extreme_hills(id, true),
        131 | 162 => extreme_hills(id, false),
        4 | 18 => forest(id, 0, 0.7),
        132 => forest(132, 1, 0.7),
        27 | 28 => forest(id, 2, 0.6),
        155 | 156 => BiomeDef {
            tree_policy: TreePolicy::BirchForestMutated,
            ..cfg(id, 0.6, forest_config(2))
        },
        29 | 157 => forest(id, 3, 0.7),
        5 | 19 => taiga(id, 0, 0.25),
        30 | 31 => taiga(id, 0, -0.5),
        32 | 33 => taiga(id, 1, 0.3),
        // 161 is `BY_ID[MEGA_TAIGA_HILLS.id + 128] = BY_ID[MEGA_TAIGA.id + 128]`,
        // the Mega Spruce Taiga object itself, so it behaves as 160.
        160 | 161 => taiga(id, 2, 0.25),
        133 => BiomeDef {
            tree_policy: TreePolicy::Taiga { variant: 0 },
            ..cfg(133, 0.25, taiga_config(0))
        },
        158 => BiomeDef {
            tree_policy: TreePolicy::Taiga { variant: 0 },
            ..cfg(158, -0.5, taiga_config(0))
        },
        6 => BiomeDef {
            tree_policy: TreePolicy::Swamp,
            flower_policy: FlowerPolicy::Swamp,
            ..cfg(6, 0.8, swamp_config())
        },
        134 => BiomeDef {
            tree_policy: TreePolicy::Swamp,
            ..cfg(134, 0.8, swamp_config())
        },
        // Frozen ocean and frozen river.
        10 | 11 => cfg(id, 0.0, DecoratorConfig::DEFAULT),
        12 | 13 => BiomeDef {
            tree_policy: TreePolicy::Fixed(TreeKind::Spruce),
            ..cfg(id, 0.0, DecoratorConfig::DEFAULT)
        },
        140 => BiomeDef {
            surface_block: Block::SnowBlock,
            tree_policy: TreePolicy::Fixed(TreeKind::Spruce),
            decorator: BiomeDecorator::IceSpikes,
            ..cfg(140, 0.0, DecoratorConfig::DEFAULT)
        },
        14 | 15 => BiomeDef {
            surface_block: Block::Mycelium,
            ..cfg(
                id,
                0.9,
                DecoratorConfig {
                    tree_attempts: -100,
                    flower_attempts: -100,
                    grass_attempts: -100,
                    mushroom_attempts: 1,
                    huge_mushroom_attempts: 1,
                    ..DecoratorConfig::DEFAULT
                },
            )
        },
        16 | 26 => BiomeDef {
            surface_block: Block::Sand,
            subsurface_block: Block::Sand,
            ..cfg(
                id,
                if id == 26 { 0.05 } else { 0.8 },
                DecoratorConfig {
                    tree_attempts: -999,
                    ..DecoratorConfig::DEFAULT
                },
            )
        },
        25 => BiomeDef {
            surface_block: Block::Stone,
            subsurface_block: Block::Stone,
            ..cfg(
                25,
                0.2,
                DecoratorConfig {
                    tree_attempts: -999,
                    ..DecoratorConfig::DEFAULT
                },
            )
        },
        21 | 22 => jungle(id, false),
        23 => jungle(23, true),
        149 => BiomeDef {
            tree_policy: TreePolicy::Jungle { edge: false },
            ..cfg(149, 0.95, jungle_config(false))
        },
        151 => BiomeDef {
            tree_policy: TreePolicy::Jungle { edge: true },
            ..cfg(151, 0.95, jungle_config(true))
        },
        35 | 36 => BiomeDef {
            tree_policy: TreePolicy::Savanna,
            decorator: BiomeDecorator::Savanna,
            ..cfg(
                id,
                if id == 35 { 1.2 } else { 1.0 },
                DecoratorConfig {
                    tree_attempts: 1,
                    flower_attempts: 4,
                    grass_attempts: 20,
                    ..DecoratorConfig::DEFAULT
                },
            )
        },
        // The shattered savanna builds its own decorator counts and runs the
        // plain base body.
        163 | 164 => BiomeDef {
            tree_policy: TreePolicy::Savanna,
            ..cfg(
                id,
                if id == 163 { 1.1 } else { 1.0 },
                DecoratorConfig {
                    tree_attempts: 2,
                    flower_attempts: 2,
                    grass_attempts: 5,
                    ..DecoratorConfig::DEFAULT
                },
            )
        },
        37 | 39 | 165 | 167 => mesa(id, false),
        38 | 166 => mesa(id, true),
        // Oceans, rivers and the remaining biomes run the base decorator.
        _ => cfg(id, 0.5, DecoratorConfig::DEFAULT),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draw_counts_match_java_short_circuits() {
        let mut rng = sunset_1_7_core::LegacyRandom::new(7);
        let state = rng.state();
        // Birch-only forests pick birch without a draw.
        assert_eq!(
            TreePolicy::Forest { variant: 2 }.pick(&mut rng),
            TreeKind::Birch
        );
        assert_eq!(rng.state(), state);
        // Swamp picks its tree and flower without a draw.
        assert_eq!(TreePolicy::Swamp.pick(&mut rng), TreeKind::Swamp);
        assert_eq!(
            FlowerPolicy::Swamp.pick(&mut rng, 0, 0),
            (Block::RedFlower, 1)
        );
        assert_eq!(rng.state(), state);
        // Default grass has no draw.
        assert_eq!(GrassPolicy::Default.pick(&mut rng), (Block::TallGrass, 1));
        assert_eq!(rng.state(), state);
        // Ice plains and mesa hand back a fixed tree, no draw.
        assert_eq!(
            TreePolicy::Fixed(TreeKind::Spruce).pick(&mut rng),
            TreeKind::Spruce
        );
        assert_eq!(rng.state(), state);
    }

    #[test]
    fn temperatures_match_the_biome_classes() {
        assert_eq!(biome(1).temperature, 0.8);
        assert_eq!(biome(2).temperature, 2.0);
        assert_eq!(biome(5).temperature, 0.25);
        assert_eq!(biome(6).temperature, 0.8);
        assert_eq!(biome(12).temperature, 0.0);
        assert_eq!(biome(26).temperature, 0.05);
        assert_eq!(biome(30).temperature, -0.5);
        assert_eq!(biome(32).temperature, 0.3);
    }

    #[test]
    fn desert_attempts_match_the_biome_class() {
        let desert = biome(2).config;
        assert_eq!(desert.tree_attempts, -999);
        assert_eq!(desert.dead_bush_attempts, 2);
        assert_eq!(desert.sugarcane_attempts, 50);
        assert_eq!(desert.cactus_attempts, 10);
        let swamp = biome(6).config;
        assert_eq!(swamp.lily_pad_attempts, 4);
        assert_eq!(swamp.mushroom_attempts, 8);
        assert_eq!(swamp.sand_patch_attempts, 0);
    }

    #[test]
    fn mutated_biomes_keep_the_original_tree_picker() {
        // `MutatedBiome.pickTree` delegates, and the swamp tree draws nothing.
        assert_eq!(biome(134).tree_policy, TreePolicy::Swamp);
        assert_eq!(biome(134).decorator, BiomeDecorator::Base);
        // `MutatedBiome` overrides neither the flower nor the plant picker.
        assert_eq!(biome(134).flower_policy, FlowerPolicy::Default);
        assert_eq!(biome(134).grass_policy, GrassPolicy::Default);
        assert_eq!(biome(149).tree_policy, TreePolicy::Jungle { edge: false });
        assert_eq!(biome(151).tree_policy, TreePolicy::Jungle { edge: true });
        assert_eq!(biome(133).tree_policy, TreePolicy::Taiga { variant: 0 });
        assert_eq!(biome(163).tree_policy, TreePolicy::Savanna);
        assert_eq!(biome(155).tree_policy, TreePolicy::BirchForestMutated);
        assert_eq!(biome(157).decorator, BiomeDecorator::Forest { variant: 3 });
        assert_eq!(biome(160).decorator, BiomeDecorator::Taiga { variant: 2 });
        assert_eq!(biome(140).decorator, BiomeDecorator::IceSpikes);
    }

    /// A `DecoratorConfig`'s twelve attempt counts, for table comparisons.
    fn counts(config: DecoratorConfig) -> [i32; 12] {
        [
            config.tree_attempts,
            config.flower_attempts,
            config.grass_attempts,
            config.dead_bush_attempts,
            config.mushroom_attempts,
            config.huge_mushroom_attempts,
            config.sugarcane_attempts,
            config.cactus_attempts,
            config.lily_pad_attempts,
            config.gravel_patch_attempts,
            config.sand_patch_attempts,
            config.clay_patch_attempts,
        ]
    }

    /// The plain `MutatedBiome` mutants run the *original's* decorator
    /// instance, so their base body uses the original's counts; the mutant
    /// itself overrides neither flower nor plant picker.
    #[test]
    fn mutated_base_bodies_use_the_original_decorator_counts() {
        let want = [
            (130, desert_config()),
            (133, taiga_config(0)),
            (134, swamp_config()),
            (149, jungle_config(false)),
            (151, jungle_config(true)),
            (155, forest_config(2)),
            (156, forest_config(2)),
            (158, taiga_config(0)),
        ];
        for (id, config) in want {
            let mutated = biome(id);
            assert_eq!(counts(mutated.config), counts(config), "biome {id}");
            assert_eq!(mutated.decorator, BiomeDecorator::Base, "biome {id}");
            assert_eq!(mutated.flower_policy, FlowerPolicy::Default, "biome {id}");
            assert_eq!(mutated.grass_policy, GrassPolicy::Default, "biome {id}");
        }
    }

    /// The mutants whose `mutate` builds a class of its own keep that class's
    /// `decorate` override; 161 is the Mega Taiga Hills alias of 160.
    #[test]
    fn mutated_subclasses_keep_their_class_override() {
        assert_eq!(
            biome(129).decorator,
            BiomeDecorator::Plains { mutated: true }
        );
        assert_eq!(biome(129).flower_policy, FlowerPolicy::Plains);
        assert_eq!(biome(131).decorator, BiomeDecorator::ExtremeHills);
        assert_eq!(biome(162).decorator, BiomeDecorator::ExtremeHills);
        assert_eq!(biome(132).decorator, BiomeDecorator::Forest { variant: 1 });
        assert_eq!(biome(132).flower_policy, FlowerPolicy::FlowerForest);
        assert_eq!(biome(157).decorator, BiomeDecorator::Forest { variant: 3 });
        assert_eq!(biome(163).decorator, BiomeDecorator::Base);
        assert_eq!(biome(164).decorator, BiomeDecorator::Base);

        let mega = biome(160);
        let alias = biome(161);
        assert_eq!(mega.temperature, alias.temperature);
        assert_eq!(mega.decorator, alias.decorator);
        assert_eq!(mega.tree_policy, alias.tree_policy);
        assert_eq!(mega.flower_policy, alias.flower_policy);
        assert_eq!(mega.grass_policy, alias.grass_policy);
        assert_eq!(counts(mega.config), counts(alias.config));
    }

    #[test]
    fn constructor_counts_match_the_biome_classes() {
        // `ExtremeHillsBiome(id, false)` leaves the tree attempts at zero.
        assert_eq!(biome(3).config.tree_attempts, 0);
        assert_eq!(biome(20).config.tree_attempts, 3);
        assert_eq!(biome(131).config.tree_attempts, 0);
        // Every taiga with variant 0 keeps one mushroom attempt, hills too.
        for id in [5, 19, 30, 31, 133, 158] {
            assert_eq!(biome(id).config.mushroom_attempts, 1, "biome {id}");
        }
        // Only the mesa plateau F keeps trees.
        assert_eq!(biome(38).config.tree_attempts, 5);
        assert_eq!(biome(166).config.tree_attempts, 5);
        assert_eq!(biome(37).config.tree_attempts, -999);
        assert_eq!(biome(39).config.tree_attempts, -999);
        // Roofed forest decorates no trees of its own.
        assert_eq!(biome(29).config.tree_attempts, -999);
        assert_eq!(biome(157).config.tree_attempts, -999);
    }

    #[test]
    fn jungle_pick_tree_draws_the_base_height() {
        for seed in 0..64 {
            let mut rng = LegacyRandom::new(seed);
            let mut reference = LegacyRandom::new(seed);
            // `nextInt(10)`, `nextInt(2)`, `nextInt(3)`, then `nextInt(7)`.
            let first = reference.next_int_bound(10).expect("positive bound");
            let second = if first == 0 {
                None
            } else {
                Some(reference.next_int_bound(2).expect("positive bound"))
            };
            let picked = TreePolicy::Jungle { edge: false }.pick(&mut rng);
            match (first, second) {
                (0, _) => assert_eq!(picked, TreeKind::LargeOak),
                (_, Some(0)) => assert_eq!(picked, TreeKind::Bush),
                _ => {
                    let third = reference.next_int_bound(3).expect("positive bound");
                    if third == 0 {
                        assert_eq!(picked, TreeKind::GiantJungle);
                    } else {
                        let height = 4 + reference.next_int_bound(7).expect("positive bound");
                        assert_eq!(
                            picked,
                            TreeKind::Jungle {
                                base_height: height
                            }
                        );
                    }
                }
            }
            assert_eq!(rng.state(), reference.state(), "seed {seed}");
        }
    }

    #[test]
    fn flower_forest_reads_the_noise_without_drawing() {
        let mut rng = sunset_1_7_core::LegacyRandom::new(11);
        let state = rng.state();
        let (block, variant) = FlowerPolicy::FlowerForest.pick(&mut rng, 40, -80);
        assert_eq!(block, Block::RedFlower);
        assert!(variant < 9 && variant != 1);
        assert_eq!(rng.state(), state);
    }
}
