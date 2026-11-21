//! `MelonPatchFeature`: the jungle's melon cluster.

use crate::block::Block;
use crate::world::{FLAG_NO_NEIGHBOR_NOTIFY, GenWorld, next_int};
use sunset_1_7_core::LegacyRandom;

/// `MelonPatchFeature`: 64 scatter attempts, each drawing six values.
pub(crate) struct MelonPatchFeature;

impl MelonPatchFeature {
    pub fn place(
        &self,
        world: &mut dyn GenWorld,
        random: &mut LegacyRandom,
        x: i32,
        y: i32,
        z: i32,
    ) -> bool {
        for _ in 0..64 {
            let px = x + next_int(random, 8) - next_int(random, 8);
            let py = y + next_int(random, 4) - next_int(random, 4);
            let pz = z + next_int(random, 8) - next_int(random, 8);
            // `MelonBlock.canBePlaced`: the target's material is replaceable.
            if world.get_block(px, py, pz).is_replaceable()
                && world.get_block(px, py - 1, pz) == Block::Grass
            {
                world.set_block_with_metadata(
                    px,
                    py,
                    pz,
                    Block::MelonBlock,
                    0,
                    FLAG_NO_NEIGHBOR_NOTIFY,
                );
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk_world::ChunkWorld;

    #[test]
    fn melons_only_on_grass() {
        let mut world = ChunkWorld::new(0, 0);
        world.fill(0, 40, Block::Grass);
        let mut rng = LegacyRandom::new(2);
        assert!(MelonPatchFeature.place(&mut world, &mut rng, 8, 41, 8));
        assert!(world.block_count(Block::MelonBlock) > 0);
        for (x, y, z, block, _) in world.rows() {
            if block == Block::MelonBlock {
                assert_eq!(world.get_block(x, y - 1, z), Block::Grass);
            }
        }
    }
}
