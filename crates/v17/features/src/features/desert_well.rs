//! `DesertWellFeature`: the sandstone well `DesertBiome.decorate` places on a
//! `nextInt(1000)` draw.

use crate::block::Block;
use crate::world::{FLAG_NO_NEIGHBOR_NOTIFY, GenWorld};

/// `DesertWellFeature`. It draws nothing from the RNG: the well is fixed
/// once the site passes its sand checks.
pub(crate) struct DesertWellFeature;

impl DesertWellFeature {
    pub fn place(&self, world: &mut dyn GenWorld, x: i32, y: i32, z: i32) -> bool {
        let mut y = y;
        while world.is_air(x, y, z) && y > 2 {
            y -= 1;
        }

        if world.get_block(x, y, z) != Block::Sand {
            return false;
        }

        // The 5x5 footprint must not sit over a hole.
        for dx in -2..=2 {
            for dz in -2..=2 {
                if world.is_air(x + dx, y - 1, z + dz) && world.is_air(x + dx, y - 2, z + dz) {
                    return false;
                }
            }
        }

        for dy in -1..=0 {
            for dx in -2..=2 {
                for dz in -2..=2 {
                    self.set(world, x + dx, y + dy, z + dz, Block::Sandstone, 0);
                }
            }
        }

        self.set(world, x, y, z, Block::FlowingWater, 0);
        self.set(world, x - 1, y, z, Block::FlowingWater, 0);
        self.set(world, x + 1, y, z, Block::FlowingWater, 0);
        self.set(world, x, y, z - 1, Block::FlowingWater, 0);
        self.set(world, x, y, z + 1, Block::FlowingWater, 0);

        for dx in -2..=2 {
            for dz in -2..=2 {
                if dx == -2 || dx == 2 || dz == -2 || dz == 2 {
                    self.set(world, x + dx, y + 1, z + dz, Block::Sandstone, 0);
                }
            }
        }

        self.set(world, x + 2, y + 1, z, Block::StoneSlab, 1);
        self.set(world, x - 2, y + 1, z, Block::StoneSlab, 1);
        self.set(world, x, y + 1, z + 2, Block::StoneSlab, 1);
        self.set(world, x, y + 1, z - 2, Block::StoneSlab, 1);

        for dx in -1..=1 {
            for dz in -1..=1 {
                if dx == 0 && dz == 0 {
                    self.set(world, x, y + 4, z, Block::Sandstone, 0);
                } else {
                    self.set(world, x + dx, y + 4, z + dz, Block::StoneSlab, 1);
                }
            }
        }

        for dy in 1..=3 {
            self.set(world, x - 1, y + dy, z - 1, Block::Sandstone, 0);
            self.set(world, x - 1, y + dy, z + 1, Block::Sandstone, 0);
            self.set(world, x + 1, y + dy, z - 1, Block::Sandstone, 0);
            self.set(world, x + 1, y + dy, z + 1, Block::Sandstone, 0);
        }

        true
    }

    fn set(&self, world: &mut dyn GenWorld, x: i32, y: i32, z: i32, block: Block, metadata: u8) {
        world.set_block_with_metadata(x, y, z, block, metadata, FLAG_NO_NEIGHBOR_NOTIFY);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk_world::ChunkWorld;

    #[test]
    fn builds_the_well_on_sand() {
        let mut world = ChunkWorld::new(0, 0);
        world.fill(0, 63, Block::Sand);
        assert!(DesertWellFeature.place(&mut world, 8, 70, 8));
        // The walk down stops on the sand at y 63: water there, roof at 67.
        assert_eq!(world.get_block(8, 63, 8), Block::FlowingWater);
        assert_eq!(world.get_block(7, 63, 8), Block::FlowingWater);
        assert_eq!(world.get_block(8, 62, 8), Block::Sandstone);
        assert_eq!(world.get_block(8, 67, 8), Block::Sandstone);
        assert_eq!(world.get_block(6, 64, 8), Block::StoneSlab);
        assert_eq!(world.get_block_metadata(6, 64, 8), 1);
    }

    #[test]
    fn refuses_a_grass_floor() {
        let mut world = ChunkWorld::new(0, 0);
        world.fill(0, 63, Block::Grass);
        assert!(!DesertWellFeature.place(&mut world, 8, 70, 8));
        assert_eq!(world.block_count(Block::Sandstone), 0);
    }
}
