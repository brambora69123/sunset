//! `VineFeature`: the jungle's vine pass, and the "cave" vines that climb
//! from the asked height to y 128.

use crate::block::Block;
use crate::world::{FLAG_NO_NEIGHBOR_NOTIFY, GenWorld, next_int};
use sunset_1_7_core::LegacyRandom;

/// `VineFeature`. `JungleBiome.decorate` calls it at y 128, where the
/// `while (y < 128)` loop never runs: the call only draws its x and z.
pub(crate) struct VineFeature;

/// `1 << Facings.FROM_DIR[Directions.OPPOSITE[face]]` for faces 2..=5.
const FACE_METADATA: [u8; 4] = [1, 4, 8, 2];

impl VineFeature {
    pub fn place(
        &self,
        world: &mut dyn GenWorld,
        random: &mut LegacyRandom,
        x: i32,
        y: i32,
        z: i32,
    ) -> bool {
        let (origin_x, origin_z) = (x, z);
        let mut x = x;
        let mut z = z;
        let mut y = y;

        while y < 128 {
            if world.is_air(x, y, z) {
                for face in 2..=5 {
                    if self.can_be_placed(world, x, y, z, face) {
                        world.set_block_with_metadata(
                            x,
                            y,
                            z,
                            Block::Vine,
                            FACE_METADATA[(face - 2) as usize],
                            FLAG_NO_NEIGHBOR_NOTIFY,
                        );
                        break;
                    }
                }
            } else {
                x = origin_x + next_int(random, 4) - next_int(random, 4);
                z = origin_z + next_int(random, 4) - next_int(random, 4);
            }

            y += 1;
        }

        true
    }

    /// `VineBlock.canBePlaced(face)`: an attached face grows on a full cube.
    fn can_be_placed(&self, world: &dyn GenWorld, x: i32, y: i32, z: i32, face: i32) -> bool {
        let neighbour = match face {
            2 => world.get_block(x, y, z + 1),
            3 => world.get_block(x, y, z - 1),
            4 => world.get_block(x + 1, y, z),
            5 => world.get_block(x - 1, y, z),
            _ => return false,
        };
        neighbour.blocks_movement()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk_world::ChunkWorld;

    #[test]
    fn vines_climb_the_air_beside_a_wall() {
        let mut world = ChunkWorld::new(0, 0);
        world.fill(0, 100, Block::Stone);
        // A stone wall beside the air column at x 8, z 8.
        for y in 101..=110 {
            world.set_block_with_metadata(7, y, 8, Block::Stone, 0, FLAG_NO_NEIGHBOR_NOTIFY);
        }
        let mut rng = LegacyRandom::new(4);
        assert!(VineFeature.place(&mut world, &mut rng, 8, 101, 8));
        assert!(world.block_count(Block::Vine) > 0);
        // Face 5 (west) maps to metadata 2 through the facing tables.
        assert_eq!(world.get_block_metadata(8, 101, 8), 2);
    }

    #[test]
    fn a_call_at_y_128_draws_nothing() {
        let mut world = ChunkWorld::new(0, 0);
        let mut rng = LegacyRandom::new(4);
        let state = rng.state();
        assert!(VineFeature.place(&mut world, &mut rng, 8, 128, 8));
        assert_eq!(rng.state(), state);
        assert_eq!(world.block_count(Block::Vine), 0);
    }
}
