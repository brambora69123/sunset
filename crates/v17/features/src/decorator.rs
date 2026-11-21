//! `FeatureDecorator` (BiomeDecorator): the chunk decoration pass, in
//! vanilla's order with vanilla's per-feature draws. `decorate` runs the
//! biome's own `decorate` override (`biome_overrides`) around the base body
//! this file holds.

use crate::biome::{BiomeDef, DecoratorConfig, TreeKind};
use crate::block::Block;
use crate::features::huge_mushroom::HugeMushroomFeature;
use crate::features::liquid_fall::LiquidFallFeature;
use crate::features::patch::{BlockPatchFeature, ClayPatchFeature};
use crate::features::plants::{
    CactusFeature, DeadBushFeature, LilyPadFeature, PlantFeature, PumpkinPatchFeature,
    SugarcaneFeature, TallPlantFeature,
};
use crate::features::tree::{
    BirchTreeFeature, LargeOakTreeFeature, OakTreeFeature, PineTreeFeature, SpruceTreeFeature,
    SwampTreeFeature,
};
use crate::features::trees::acacia::AcaciaTreeFeature;
use crate::features::trees::bush::BushFeature;
use crate::features::trees::dark_oak::DarkOakTreeFeature;
use crate::features::trees::giant_jungle::GiantJungleTreeFeature;
use crate::features::trees::giant_pine::GiantPineTreeFeature;
use crate::features::trees::giant_spruce::GiantSpruceTreeFeature;
use crate::features::trees::jungle::JungleTreeFeature;
use crate::features::trees::tall_birch::TallBirchTreeFeature;
use crate::features::vein::VeinFeature;
use crate::world::{GenWorld, HeightMapWorld, next_int};
use sunset_1_7_core::LegacyRandom;

/// Every decoration pass.
pub const STAGE_ALL: usize = usize::MAX;

pub struct FeatureDecorator {
    clay_patch: ClayPatchFeature,
    sand_patch: BlockPatchFeature,
    gravel_patch: BlockPatchFeature,
    dirt_vein: VeinFeature,
    gravel_vein: VeinFeature,
    coal_ore_vein: VeinFeature,
    iron_ore_vein: VeinFeature,
    gold_ore_vein: VeinFeature,
    redstone_ore_vein: VeinFeature,
    diamond_ore_vein: VeinFeature,
    lapis_ore_vein: VeinFeature,
    flower: PlantFeature,
    brown_mushroom: PlantFeature,
    red_mushroom: PlantFeature,
    pub(crate) huge_red_mushroom: HugeMushroomFeature,
    sugarcane: SugarcaneFeature,
    cactus: CactusFeature,
    lily_pad: LilyPadFeature,
    oak_tree: OakTreeFeature,
    birch_tree: BirchTreeFeature,
    large_oak_tree: LargeOakTreeFeature,
    spruce_tree: SpruceTreeFeature,
    pine_tree: PineTreeFeature,
    swamp_tree: SwampTreeFeature,
    /// `placeLakes`: the 50 water and 20 lava surface springs.
    pub place_lakes: bool,
}

impl Default for FeatureDecorator {
    fn default() -> FeatureDecorator {
        FeatureDecorator::new()
    }
}

impl FeatureDecorator {
    pub fn new() -> FeatureDecorator {
        FeatureDecorator {
            clay_patch: ClayPatchFeature { size: 4 },
            sand_patch: BlockPatchFeature {
                block: Block::Sand,
                size: 7,
            },
            gravel_patch: BlockPatchFeature {
                block: Block::Gravel,
                size: 6,
            },
            dirt_vein: VeinFeature::new(Block::Dirt, 32),
            gravel_vein: VeinFeature::new(Block::Gravel, 32),
            coal_ore_vein: VeinFeature::new(Block::CoalOre, 16),
            iron_ore_vein: VeinFeature::new(Block::IronOre, 8),
            gold_ore_vein: VeinFeature::new(Block::GoldOre, 8),
            redstone_ore_vein: VeinFeature::new(Block::RedstoneOre, 7),
            diamond_ore_vein: VeinFeature::new(Block::DiamondOre, 7),
            lapis_ore_vein: VeinFeature::new(Block::LapisOre, 6),
            flower: PlantFeature::new(Block::YellowFlower),
            brown_mushroom: PlantFeature::new(Block::BrownMushroom),
            red_mushroom: PlantFeature::new(Block::RedMushroom),
            huge_red_mushroom: HugeMushroomFeature::new(),
            sugarcane: SugarcaneFeature,
            cactus: CactusFeature,
            lily_pad: LilyPadFeature,
            oak_tree: OakTreeFeature::new(false),
            birch_tree: BirchTreeFeature {
                tall: false,
                notify_neighbors: false,
            },
            large_oak_tree: LargeOakTreeFeature::new(),
            spruce_tree: SpruceTreeFeature {
                notify_neighbors: false,
            },
            pine_tree: PineTreeFeature {
                notify_neighbors: false,
            },
            swamp_tree: SwampTreeFeature,
            place_lakes: true,
        }
    }

    /// `Biome.decorate`: the biome's own override runs the base body with
    /// `super.decorate`.
    pub fn decorate(
        &mut self,
        world: &mut dyn GenWorld,
        random: &mut LegacyRandom,
        biome: &BiomeDef,
        x: i32,
        z: i32,
    ) {
        // Vanilla's features read `World.getHeight`, the chunk height map,
        // which plants and lava never raise. The world here only offers a scan
        // for the highest non-air block, so run the pass on the map instead.
        let mut world = HeightMapWorld::new(world);
        let mut base = |decorator: &mut FeatureDecorator,
                        world: &mut dyn GenWorld,
                        random: &mut LegacyRandom,
                        config: &DecoratorConfig| {
            decorator.base_decorate(world, random, biome, config, x, z, STAGE_ALL);
        };
        crate::biome_overrides::decorate(self, &mut world, random, biome, x, z, &mut base);
    }

    /// `decorate`, stopping after `stages` passes. The parity probe walks the
    /// passes one at a time; `decorate` runs them all. This is the base
    /// decorator body only, so the stage numbering stays the passes' own.
    pub fn decorate_upto(
        &mut self,
        world: &mut dyn GenWorld,
        random: &mut LegacyRandom,
        biome: &BiomeDef,
        x: i32,
        z: i32,
        stages: usize,
    ) {
        let config = biome.config;
        let mut world = HeightMapWorld::new(world);
        self.base_decorate(&mut world, random, biome, &config, x, z, stages);
    }

    /// `FeatureDecorator.decorate(biome)`: the shared body every biome's
    /// `decorate` reaches through `super.decorate`, with the attempt counts
    /// the biome's override may have changed.
    fn base_decorate(
        &mut self,
        world: &mut dyn GenWorld,
        random: &mut LegacyRandom,
        biome: &BiomeDef,
        config: &DecoratorConfig,
        x: i32,
        z: i32,
        stages: usize,
    ) {
        if stages == 0 {
            return;
        }
        self.place_veins(world, random, x, z);

        if stages <= 1 {
            return;
        }
        for _ in 0..config.sand_patch_attempts {
            let px = x + next_int(random, 16) + 8;
            let pz = z + next_int(random, 16) + 8;
            let surface = world.get_surface_height(px, pz);
            self.sand_patch.place(world, random, px, surface, pz);
        }

        if stages <= 2 {
            return;
        }
        for _ in 0..config.clay_patch_attempts {
            let px = x + next_int(random, 16) + 8;
            let pz = z + next_int(random, 16) + 8;
            let surface = world.get_surface_height(px, pz);
            self.clay_patch.place(world, random, px, surface, pz);
        }

        if stages <= 3 {
            return;
        }
        for _ in 0..config.gravel_patch_attempts {
            let px = x + next_int(random, 16) + 8;
            let pz = z + next_int(random, 16) + 8;
            let surface = world.get_surface_height(px, pz);
            self.gravel_patch.place(world, random, px, surface, pz);
        }

        if stages <= 4 {
            return;
        }
        let mut tree_attempts = config.tree_attempts;
        if next_int(random, 10) == 0 {
            tree_attempts += 1;
        }

        for _ in 0..tree_attempts.max(0) {
            let px = x + next_int(random, 16) + 8;
            let pz = z + next_int(random, 16) + 8;
            let py = world.get_height(px, pz);
            self.place_picked_tree(world, random, biome, px, py, pz);
        }

        if stages <= 5 {
            return;
        }
        for _ in 0..config.huge_mushroom_attempts {
            let px = x + next_int(random, 16) + 8;
            let pz = z + next_int(random, 16) + 8;
            let height = world.get_height(px, pz);
            self.huge_red_mushroom.place(world, random, px, height, pz);
        }

        if stages <= 6 {
            return;
        }
        for _ in 0..config.flower_attempts {
            let px = x + next_int(random, 16) + 8;
            let pz = z + next_int(random, 16) + 8;
            let height = world.get_height(px, pz);
            let py = next_int(random, height + 32);
            let (block, variant) = biome.flower_policy.pick(random, px, pz);
            if !block.is_air_material() {
                self.flower = PlantFeature::with_variant(block, variant);
                self.flower.place(world, random, px, py, pz);
            }
        }

        if stages <= 7 {
            return;
        }
        for _ in 0..config.grass_attempts {
            let px = x + next_int(random, 16) + 8;
            let pz = z + next_int(random, 16) + 8;
            let height = world.get_height(px, pz);
            let py = next_int(random, height * 2);
            let (block, variant) = biome.grass_policy.pick(random);
            TallPlantFeature::new(block, variant).place(world, random, px, py, pz);
        }

        if stages <= 8 {
            return;
        }
        for _ in 0..config.dead_bush_attempts {
            let px = x + next_int(random, 16) + 8;
            let pz = z + next_int(random, 16) + 8;
            let height = world.get_height(px, pz);
            let py = next_int(random, height * 2);
            DeadBushFeature.place(world, random, px, py, pz);
        }

        if stages <= 9 {
            return;
        }
        for _ in 0..config.lily_pad_attempts {
            let px = x + next_int(random, 16) + 8;
            let pz = z + next_int(random, 16) + 8;
            let height = world.get_height(px, pz);
            let mut py = next_int(random, height * 2);
            while py > 0 && world.is_air(px, py - 1, pz) {
                py -= 1;
            }
            self.lily_pad.place(world, random, px, py, pz);
        }

        if stages <= 10 {
            return;
        }
        for _ in 0..config.mushroom_attempts {
            if next_int(random, 4) == 0 {
                let px = x + next_int(random, 16) + 8;
                let pz = z + next_int(random, 16) + 8;
                let height = world.get_height(px, pz);
                self.brown_mushroom.place(world, random, px, height, pz);
            }

            if next_int(random, 8) == 0 {
                let px = x + next_int(random, 16) + 8;
                let pz = z + next_int(random, 16) + 8;
                let height = world.get_height(px, pz);
                let py = next_int(random, height * 2);
                self.red_mushroom.place(world, random, px, py, pz);
            }
        }

        if next_int(random, 4) == 0 {
            let px = x + next_int(random, 16) + 8;
            let pz = z + next_int(random, 16) + 8;
            let height = world.get_height(px, pz);
            let py = next_int(random, height * 2);
            self.brown_mushroom.place(world, random, px, py, pz);
        }

        if next_int(random, 8) == 0 {
            let px = x + next_int(random, 16) + 8;
            let pz = z + next_int(random, 16) + 8;
            let height = world.get_height(px, pz);
            let py = next_int(random, height * 2);
            self.red_mushroom.place(world, random, px, py, pz);
        }

        if stages <= 11 {
            return;
        }
        for _ in 0..config.sugarcane_attempts {
            let px = x + next_int(random, 16) + 8;
            let pz = z + next_int(random, 16) + 8;
            let height = world.get_height(px, pz);
            let py = next_int(random, height * 2);
            self.sugarcane.place(world, random, px, py, pz);
        }

        if stages <= 12 {
            return;
        }
        for _ in 0..10 {
            let px = x + next_int(random, 16) + 8;
            let pz = z + next_int(random, 16) + 8;
            let height = world.get_height(px, pz);
            let py = next_int(random, height * 2);
            self.sugarcane.place(world, random, px, py, pz);
        }

        if stages <= 13 {
            return;
        }
        if next_int(random, 32) == 0 {
            let px = x + next_int(random, 16) + 8;
            let pz = z + next_int(random, 16) + 8;
            let height = world.get_height(px, pz);
            let py = next_int(random, height * 2);
            PumpkinPatchFeature.place(world, random, px, py, pz);
        }

        if stages <= 14 {
            return;
        }
        for _ in 0..config.cactus_attempts {
            let px = x + next_int(random, 16) + 8;
            let pz = z + next_int(random, 16) + 8;
            let height = world.get_height(px, pz);
            let py = next_int(random, height * 2);
            self.cactus.place(world, random, px, py, pz);
        }

        if stages <= 15 {
            return;
        }
        if self.place_lakes {
            for _ in 0..50 {
                let px = x + next_int(random, 16) + 8;
                let outer = next_int(random, 248) + 8;
                let py = next_int(random, outer);
                let pz = z + next_int(random, 16) + 8;
                LiquidFallFeature::new(Block::FlowingWater).place(world, random, px, py, pz);
            }

            for _ in 0..20 {
                let px = x + next_int(random, 16) + 8;
                let inner = next_int(random, 240) + 8;
                let middle = next_int(random, inner) + 8;
                let py = next_int(random, middle);
                let pz = z + next_int(random, 16) + 8;
                LiquidFallFeature::new(Block::FlowingLava).place(world, random, px, py, pz);
            }
        }
    }

    /// `biome.pickTree` then `prepare`/`place`/`placeSoil`: the tree loop the
    /// base body and the roofed forest's grid share.
    pub(crate) fn place_picked_tree(
        &mut self,
        world: &mut dyn GenWorld,
        random: &mut LegacyRandom,
        biome: &BiomeDef,
        x: i32,
        y: i32,
        z: i32,
    ) -> bool {
        match biome.tree_policy.pick(random) {
            TreeKind::Oak => self.oak_tree.place(world, random, x, y, z),
            TreeKind::LargeOak => {
                self.large_oak_tree.prepare();
                self.large_oak_tree.place(world, random, x, y, z)
            }
            TreeKind::Birch => self.birch_tree.place(world, random, x, y, z),
            TreeKind::TallBirch => TallBirchTreeFeature::new().place(world, random, x, y, z),
            TreeKind::Spruce => self.spruce_tree.place(world, random, x, y, z),
            TreeKind::Pine => self.pine_tree.place(world, random, x, y, z),
            TreeKind::Swamp => self.swamp_tree.place(world, random, x, y, z),
            TreeKind::Acacia => AcaciaTreeFeature::new().place(world, random, x, y, z),
            TreeKind::DarkOak => DarkOakTreeFeature::new().place(world, random, x, y, z),
            TreeKind::Bush => BushFeature::new(3, 0).place(world, random, x, y, z),
            TreeKind::Jungle { base_height } => {
                JungleTreeFeature::new(base_height).place(world, random, x, y, z)
            }
            TreeKind::GiantJungle => GiantJungleTreeFeature::new().place(world, random, x, y, z),
            // The taiga giants are the only trees with soil to place.
            TreeKind::GiantPine => {
                let feature = GiantPineTreeFeature::new();
                let placed = feature.place(world, random, x, y, z);
                if placed {
                    feature.place_soil(world, random, x, y, z);
                }
                placed
            }
            TreeKind::GiantSpruce => {
                let feature = GiantSpruceTreeFeature::new(true);
                let placed = feature.place(world, random, x, y, z);
                if placed {
                    feature.place_soil(world, random, x, y, z);
                }
                placed
            }
        }
    }

    /// `LargeOakTreeFeature.place` through the decorator, for parity probes.
    pub fn place_large_oak(
        &mut self,
        world: &mut dyn GenWorld,
        random: &mut LegacyRandom,
        x: i32,
        y: i32,
        z: i32,
    ) -> bool {
        self.large_oak_tree.prepare();
        self.large_oak_tree.place(world, random, x, y, z)
    }

    /// The large oak's current trunk height (the shared-instance state).
    pub fn large_oak_height(&self) -> i32 {
        self.large_oak_tree.height
    }

    /// `placeVeinLinear`: chunk-local x and z, no +8; y from the linear band.
    fn place_vein_linear(
        &self,
        world: &mut dyn GenWorld,
        random: &mut LegacyRandom,
        count: i32,
        vein: &VeinFeature,
        min_height: i32,
        max_height: i32,
        origin_x: i32,
        origin_z: i32,
    ) {
        for _ in 0..count {
            let x = origin_x + next_int(random, 16);
            let y = next_int(random, max_height - min_height) + min_height;
            let z = origin_z + next_int(random, 16);
            vein.place(world, random, x, y, z);
        }
    }

    /// `placeVeinTriangular`.
    fn place_vein_triangular(
        &self,
        world: &mut dyn GenWorld,
        random: &mut LegacyRandom,
        count: i32,
        vein: &VeinFeature,
        min_height: i32,
        max_height: i32,
        origin_x: i32,
        origin_z: i32,
    ) {
        for _ in 0..count {
            let x = origin_x + next_int(random, 16);
            let y = next_int(random, max_height)
                + next_int(random, max_height)
                + (min_height - max_height);
            let z = origin_z + next_int(random, 16);
            vein.place(world, random, x, y, z);
        }
    }

    /// `placeVeins`: the eight ore passes in vanilla's order.
    pub fn place_veins(
        &self,
        world: &mut dyn GenWorld,
        random: &mut LegacyRandom,
        origin_x: i32,
        origin_z: i32,
    ) {
        self.place_vein_linear(
            world,
            random,
            20,
            &self.dirt_vein,
            0,
            256,
            origin_x,
            origin_z,
        );
        self.place_vein_linear(
            world,
            random,
            10,
            &self.gravel_vein,
            0,
            256,
            origin_x,
            origin_z,
        );
        self.place_vein_linear(
            world,
            random,
            20,
            &self.coal_ore_vein,
            0,
            128,
            origin_x,
            origin_z,
        );
        self.place_vein_linear(
            world,
            random,
            20,
            &self.iron_ore_vein,
            0,
            64,
            origin_x,
            origin_z,
        );
        self.place_vein_linear(
            world,
            random,
            2,
            &self.gold_ore_vein,
            0,
            32,
            origin_x,
            origin_z,
        );
        self.place_vein_linear(
            world,
            random,
            8,
            &self.redstone_ore_vein,
            0,
            16,
            origin_x,
            origin_z,
        );
        self.place_vein_linear(
            world,
            random,
            1,
            &self.diamond_ore_vein,
            0,
            16,
            origin_x,
            origin_z,
        );
        self.place_vein_triangular(
            world,
            random,
            1,
            &self.lapis_ore_vein,
            16,
            16,
            origin_x,
            origin_z,
        );
    }
}
