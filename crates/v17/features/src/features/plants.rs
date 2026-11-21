//! Plant features: `PlantFeature` (flowers/mushrooms), `TallPlantFeature`
//! (tall grass/ferns), `DeadBushFeature`, `PumpkinPatchFeature`,
//! `LilyPadFeature`, `CactusFeature` and `SugarcaneFeature`.

use crate::block::Block;
use crate::world::{FLAG_NO_NEIGHBOR_NOTIFY, GenWorld, next_int};

/// `PlantFeature` (WorldGenFlowers): 64 scatter attempts of one plant.
pub(crate) struct PlantFeature {
    pub block: Block,
    pub variant: u8,
}

impl PlantFeature {
    pub fn new(block: Block) -> PlantFeature {
        PlantFeature { block, variant: 0 }
    }

    pub fn with_variant(block: Block, variant: u8) -> PlantFeature {
        PlantFeature { block, variant }
    }

    /// All 64 attempts draw, success or not.
    pub fn place(
        &self,
        world: &mut dyn GenWorld,
        random: &mut sunset_1_7_core::LegacyRandom,
        x: i32,
        y: i32,
        z: i32,
    ) -> bool {
        for _ in 0..64 {
            let px = x + next_int(random, 8) - next_int(random, 8);
            let py = y + next_int(random, 4) - next_int(random, 4);
            let pz = z + next_int(random, 8) - next_int(random, 8);
            if world.is_air(px, py, pz)
                && (world.has_sky() || py < 255)
                && world.plant_can_survive(self.block, px, py, pz)
            {
                world.set_block_with_metadata(
                    px,
                    py,
                    pz,
                    self.block,
                    self.variant,
                    FLAG_NO_NEIGHBOR_NOTIFY,
                );
            }
        }
        true
    }
}

/// `TallPlantFeature` (WorldGenTallGrass): the origin walks down through air
/// first, then 128 scatter attempts.
pub(crate) struct TallPlantFeature {
    pub block: Block,
    pub variant: u8,
}

impl TallPlantFeature {
    pub fn new(block: Block, variant: u8) -> TallPlantFeature {
        TallPlantFeature { block, variant }
    }

    pub fn place(
        &self,
        world: &mut dyn GenWorld,
        random: &mut sunset_1_7_core::LegacyRandom,
        x: i32,
        y: i32,
        z: i32,
    ) -> bool {
        let mut y = y;
        loop {
            let block = world.get_block(x, y, z);
            if !(block.is_air_material() || matches!(block, Block::Leaves | Block::Leaves2))
                || y <= 0
            {
                break;
            }
            y -= 1;
        }

        for _ in 0..128 {
            let px = x + next_int(random, 8) - next_int(random, 8);
            let py = y + next_int(random, 4) - next_int(random, 4);
            let pz = z + next_int(random, 8) - next_int(random, 8);
            if world.is_air(px, py, pz) && world.plant_can_survive(self.block, px, py, pz) {
                world.set_block_with_metadata(
                    px,
                    py,
                    pz,
                    self.block,
                    self.variant,
                    FLAG_NO_NEIGHBOR_NOTIFY,
                );
            }
        }
        true
    }
}

/// `DeadBushFeature`.
pub(crate) struct DeadBushFeature;

impl DeadBushFeature {
    pub fn place(
        &self,
        world: &mut dyn GenWorld,
        random: &mut sunset_1_7_core::LegacyRandom,
        x: i32,
        y: i32,
        z: i32,
    ) -> bool {
        let mut y = y;
        loop {
            let block = world.get_block(x, y, z);
            if !(block.is_air_material() || matches!(block, Block::Leaves | Block::Leaves2))
                || y <= 0
            {
                break;
            }
            y -= 1;
        }

        for _ in 0..4 {
            let px = x + next_int(random, 8) - next_int(random, 8);
            let py = y + next_int(random, 4) - next_int(random, 4);
            let pz = z + next_int(random, 8) - next_int(random, 8);
            if world.is_air(px, py, pz) && world.plant_can_survive(Block::DeadBush, px, py, pz) {
                world.set_block_with_metadata(
                    px,
                    py,
                    pz,
                    Block::DeadBush,
                    0,
                    FLAG_NO_NEIGHBOR_NOTIFY,
                );
            }
        }
        true
    }
}

/// `PumpkinPatchFeature`.
pub(crate) struct PumpkinPatchFeature;

impl PumpkinPatchFeature {
    pub fn place(
        &self,
        world: &mut dyn GenWorld,
        random: &mut sunset_1_7_core::LegacyRandom,
        x: i32,
        y: i32,
        z: i32,
    ) -> bool {
        for _ in 0..64 {
            let px = x + next_int(random, 8) - next_int(random, 8);
            let py = y + next_int(random, 4) - next_int(random, 4);
            let pz = z + next_int(random, 8) - next_int(random, 8);
            if world.is_air(px, py, pz) && world.get_block(px, py - 1, pz) == Block::Grass {
                let facing = next_int(random, 4);
                world.set_block_with_metadata(
                    px,
                    py,
                    pz,
                    Block::Pumpkin,
                    facing as u8,
                    FLAG_NO_NEIGHBOR_NOTIFY,
                );
            }
        }
        true
    }
}

/// `LilyPadFeature`.
pub(crate) struct LilyPadFeature;

impl LilyPadFeature {
    pub fn place(
        &self,
        world: &mut dyn GenWorld,
        random: &mut sunset_1_7_core::LegacyRandom,
        x: i32,
        y: i32,
        z: i32,
    ) -> bool {
        for _ in 0..10 {
            let px = x + next_int(random, 8) - next_int(random, 8);
            let py = y + next_int(random, 4) - next_int(random, 4);
            let pz = z + next_int(random, 8) - next_int(random, 8);
            if world.is_air(px, py, pz) && world.lily_pad_can_be_placed(px, py, pz) {
                world.set_block_with_metadata(
                    px,
                    py,
                    pz,
                    Block::LilyPad,
                    0,
                    FLAG_NO_NEIGHBOR_NOTIFY,
                );
            }
        }
        true
    }
}

/// `CactusFeature`.
pub(crate) struct CactusFeature;

impl CactusFeature {
    pub fn place(
        &self,
        world: &mut dyn GenWorld,
        random: &mut sunset_1_7_core::LegacyRandom,
        x: i32,
        y: i32,
        z: i32,
    ) -> bool {
        for _ in 0..10 {
            let px = x + next_int(random, 8) - next_int(random, 8);
            let py = y + next_int(random, 4) - next_int(random, 4);
            let pz = z + next_int(random, 8) - next_int(random, 8);
            if world.is_air(px, py, pz) {
                let inner = next_int(random, 3) + 1;
                let height = 1 + next_int(random, inner);
                for step in 0..height {
                    if world.cactus_can_survive(px, py + step, pz) {
                        world.set_block_with_metadata(
                            px,
                            py + step,
                            pz,
                            Block::Cactus,
                            0,
                            FLAG_NO_NEIGHBOR_NOTIFY,
                        );
                    }
                }
            }
        }
        true
    }
}

/// `SugarcaneFeature`.
pub(crate) struct SugarcaneFeature;

impl SugarcaneFeature {
    pub fn place(
        &self,
        world: &mut dyn GenWorld,
        random: &mut sunset_1_7_core::LegacyRandom,
        x: i32,
        y: i32,
        z: i32,
    ) -> bool {
        for _ in 0..20 {
            let px = x + next_int(random, 4) - next_int(random, 4);
            let py = y;
            let pz = z + next_int(random, 4) - next_int(random, 4);
            if world.is_air(px, py, pz)
                && (world.get_block(px - 1, py - 1, pz).is_liquid()
                    || world.get_block(px + 1, py - 1, pz).is_liquid()
                    || world.get_block(px, py - 1, pz - 1).is_liquid()
                    || world.get_block(px, py - 1, pz + 1).is_liquid())
            {
                let inner = next_int(random, 3) + 1;
                let height = 2 + next_int(random, inner);
                for step in 0..height {
                    if world.reeds_can_survive(px, py + step, pz) {
                        world.set_block_with_metadata(
                            px,
                            py + step,
                            pz,
                            Block::Reeds,
                            0,
                            FLAG_NO_NEIGHBOR_NOTIFY,
                        );
                    }
                }
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk_world::ChunkWorld;
    use sunset_1_7_core::RandomSource;

    #[test]
    fn flower_scatter_keeps_draw_count() {
        let mut world = ChunkWorld::new(0, 0);
        world.fill(0, 40, Block::Grass);
        let mut rng = sunset_1_7_core::LegacyRandom::new(13);
        let flower = PlantFeature::new(Block::YellowFlower);
        assert!(flower.place(&mut world, &mut rng, 8, 41, 8));
        // Vanilla's per-attempt draws: two x, two y, two z.
        let mut reference = sunset_1_7_core::LegacyRandom::new(13);
        for _ in 0..64 {
            let _ = reference.next_int_bound(8);
            let _ = reference.next_int_bound(8);
            let _ = reference.next_int_bound(4);
            let _ = reference.next_int_bound(4);
            let _ = reference.next_int_bound(8);
            let _ = reference.next_int_bound(8);
        }
        assert_eq!(rng.state(), reference.state());
    }

    #[test]
    fn lily_pad_only_on_water() {
        let mut world = ChunkWorld::new(0, 0);
        for x in 4..12 {
            for z in 4..12 {
                world.set_block_with_metadata(x, 41, z, Block::Water, 0, FLAG_NO_NEIGHBOR_NOTIFY);
            }
        }
        let mut pads_placed = 0;
        for seed in 0..50_i64 {
            let mut rng = sunset_1_7_core::LegacyRandom::new(seed);
            let pads = LilyPadFeature;
            // The decorator walks the origin down to the water surface first.
            pads.place(&mut world, &mut rng, 8, 42, 8);
            pads_placed += world.block_count(Block::LilyPad);
        }
        assert!(pads_placed > 0);
        for (_, y, _, block, _) in world.rows() {
            if block == Block::LilyPad {
                assert!(y > 41);
            }
        }
    }

    #[test]
    fn cactus_only_on_sand() {
        let mut cacti = 0;
        for seed in 0..50_i64 {
            let mut world = ChunkWorld::new(0, 0);
            world.fill(0, 40, Block::Sand);
            let mut rng = sunset_1_7_core::LegacyRandom::new(seed);
            let cactus = CactusFeature;
            assert!(cactus.place(&mut world, &mut rng, 8, 41, 8));
            cacti += world.block_count(Block::Cactus);
        }
        assert!(cacti > 0);
    }

    #[test]
    fn dead_bushes_stand_on_sand_and_mesa_clay() {
        for floor in [Block::Sand, Block::StainedHardenedClay, Block::Dirt] {
            let mut placed = 0;
            for seed in 0..20_i64 {
                let mut world = ChunkWorld::new(0, 0);
                world.fill(0, 40, floor);
                let mut rng = sunset_1_7_core::LegacyRandom::new(seed);
                assert!(DeadBushFeature.place(&mut world, &mut rng, 8, 41, 8));
                placed += world.block_count(Block::DeadBush);
            }
            assert!(placed > 0, "dead bush on {floor:?}");
        }
    }

    #[test]
    fn dead_bushes_refuse_grass() {
        let mut world = ChunkWorld::new(0, 0);
        world.fill(0, 40, Block::Grass);
        for seed in 0..20_i64 {
            let mut rng = sunset_1_7_core::LegacyRandom::new(seed);
            DeadBushFeature.place(&mut world, &mut rng, 8, 41, 8);
        }
        assert_eq!(world.block_count(Block::DeadBush), 0);
    }
}
