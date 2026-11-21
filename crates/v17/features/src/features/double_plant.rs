//! `DoublePlantFeature`: the two-block plants the plains, forest, savanna and
//! taiga overrides scatter.

use crate::block::Block;
use crate::world::{FLAG_NO_NEIGHBOR_NOTIFY, GenWorld, next_int};
use sunset_1_7_core::LegacyRandom;

/// `DoublePlantFeature`: 64 scatter attempts of one variant. The variant is
/// set before each call, like vanilla's shared `DOUBLE_PLANT` instance.
pub(crate) struct DoublePlantFeature {
    variant: u8,
}

impl DoublePlantFeature {
    pub fn new() -> DoublePlantFeature {
        DoublePlantFeature { variant: 0 }
    }

    pub fn set_variant(&mut self, variant: u8) {
        self.variant = variant;
    }

    /// All 64 attempts draw, success or not. `DoublePlantBlock.canBePlaced`
    /// wants air at the plant and above it, on grass, dirt or farmland.
    pub fn place(
        &self,
        world: &mut dyn GenWorld,
        random: &mut LegacyRandom,
        x: i32,
        y: i32,
        z: i32,
    ) -> bool {
        let mut placed = false;
        for _ in 0..64 {
            let px = x + next_int(random, 8) - next_int(random, 8);
            let py = y + next_int(random, 4) - next_int(random, 4);
            let pz = z + next_int(random, 8) - next_int(random, 8);
            if world.is_air(px, py, pz)
                && world.is_air(px, py + 1, pz)
                && matches!(
                    world.get_block(px, py - 1, pz),
                    Block::Grass | Block::Dirt | Block::Farmland
                )
            {
                // `DoublePlantBlock.place`: bottom half, then the top half.
                world.set_block_with_metadata(
                    px,
                    py,
                    pz,
                    Block::DoublePlant,
                    self.variant,
                    FLAG_NO_NEIGHBOR_NOTIFY,
                );
                world.set_block_with_metadata(
                    px,
                    py + 1,
                    pz,
                    Block::DoublePlant,
                    8,
                    FLAG_NO_NEIGHBOR_NOTIFY,
                );
                placed = true;
            }
        }
        placed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk_world::ChunkWorld;
    use sunset_1_7_core::RandomSource;

    #[test]
    fn writes_both_halves_and_keeps_the_draw_count() {
        let mut world = ChunkWorld::new(0, 0);
        world.fill(0, 40, Block::Grass);
        let mut plant = DoublePlantFeature::new();
        plant.set_variant(2);
        let mut rng = LegacyRandom::new(3);
        assert!(plant.place(&mut world, &mut rng, 8, 41, 8));

        let mut reference = LegacyRandom::new(3);
        for _ in 0..64 {
            let _ = reference.next_int_bound(8);
            let _ = reference.next_int_bound(8);
            let _ = reference.next_int_bound(4);
            let _ = reference.next_int_bound(4);
            let _ = reference.next_int_bound(8);
            let _ = reference.next_int_bound(8);
        }
        assert_eq!(rng.state(), reference.state());

        let halves: Vec<(i32, i32, i32, u8)> = world
            .rows()
            .filter(|(_, _, _, block, _)| *block == Block::DoublePlant)
            .map(|(x, y, z, _, metadata)| (x, y, z, metadata))
            .collect();
        assert!(!halves.is_empty());
        for (x, y, z, metadata) in halves {
            if metadata == 8 {
                assert_eq!(world.get_block_metadata(x, y - 1, z), 2);
            } else {
                assert_eq!(metadata, 2);
                assert_eq!(world.get_block_metadata(x, y + 1, z), 8);
            }
        }
    }
}
