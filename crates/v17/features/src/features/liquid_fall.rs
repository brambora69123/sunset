//! `LiquidFallFeature` (WorldGenLakes' surface springs): single-source
//! flowing water and lava pockets in stone.

use crate::block::Block;
use crate::world::{FLAG_NO_NEIGHBOR_NOTIFY, GenWorld};

pub(crate) struct LiquidFallFeature {
    liquid: Block,
}

impl LiquidFallFeature {
    pub fn new(liquid: Block) -> LiquidFallFeature {
        LiquidFallFeature { liquid }
    }

    /// Draws nothing; the flow itself (`liquid.tick`) is the fluid pass.
    pub fn place(
        &self,
        world: &mut dyn GenWorld,
        _random: &mut sunset_1_7_core::LegacyRandom,
        x: i32,
        y: i32,
        z: i32,
    ) -> bool {
        if world.get_block(x, y + 1, z) != Block::Stone {
            return false;
        }
        if world.get_block(x, y - 1, z) != Block::Stone {
            return false;
        }
        let at = world.get_block(x, y, z);
        if !at.is_air_material() && at != Block::Stone {
            return false;
        }

        let mut solid_sides = 0;
        if world.get_block(x - 1, y, z) == Block::Stone {
            solid_sides += 1;
        }
        if world.get_block(x + 1, y, z) == Block::Stone {
            solid_sides += 1;
        }
        if world.get_block(x, y, z - 1) == Block::Stone {
            solid_sides += 1;
        }
        if world.get_block(x, y, z + 1) == Block::Stone {
            solid_sides += 1;
        }

        let mut open_sides = 0;
        if world.is_air(x - 1, y, z) {
            open_sides += 1;
        }
        if world.is_air(x + 1, y, z) {
            open_sides += 1;
        }
        if world.is_air(x, y, z - 1) {
            open_sides += 1;
        }
        if world.is_air(x, y, z + 1) {
            open_sides += 1;
        }

        if solid_sides == 3 && open_sides == 1 {
            world.set_block_with_metadata(x, y, z, self.liquid, 0, FLAG_NO_NEIGHBOR_NOTIFY);
            // `liquid.tick(world, x, y, z, random)`: the fluid pass spreads
            // the flow; no RNG is drawn there.
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk_world::ChunkWorld;

    #[test]
    fn spring_fills_the_one_open_side() {
        let mut world = ChunkWorld::new(0, 0);
        world.fill(0, 60, Block::Stone);
        // Carve one open pocket at (8, 30, 8) with stone above and below.
        world.set_block_with_metadata(8, 30, 8, Block::Air, 0, FLAG_NO_NEIGHBOR_NOTIFY);
        world.set_block_with_metadata(8, 30, 9, Block::Air, 0, FLAG_NO_NEIGHBOR_NOTIFY);
        let mut rng = sunset_1_7_core::LegacyRandom::new(1);
        let spring = LiquidFallFeature::new(Block::FlowingWater);
        assert!(spring.place(&mut world, &mut rng, 8, 30, 8));
        assert_eq!(world.get_block(8, 30, 8), Block::FlowingWater);
    }

    #[test]
    fn spring_needs_stone_above_and_below() {
        let mut world = ChunkWorld::new(0, 0);
        world.fill(0, 60, Block::Grass);
        let mut rng = sunset_1_7_core::LegacyRandom::new(1);
        let spring = LiquidFallFeature::new(Block::FlowingLava);
        assert!(!spring.place(&mut world, &mut rng, 8, 30, 8));
    }
}
