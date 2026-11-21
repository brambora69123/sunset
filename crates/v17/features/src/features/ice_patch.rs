//! `IcePatchFeature`: the packed-ice discs beside the ice spikes.

use crate::block::Block;
use crate::world::{FLAG_NO_NEIGHBOR_NOTIFY, GenWorld, next_int};
use sunset_1_7_core::LegacyRandom;

/// `IcePatchFeature(size)`: one `nextInt(size - 2) + 2` radius draw.
pub(crate) struct IcePatchFeature {
    size: i32,
}

impl IcePatchFeature {
    pub fn new(size: i32) -> IcePatchFeature {
        IcePatchFeature { size }
    }

    pub fn place(
        &self,
        world: &mut dyn GenWorld,
        random: &mut LegacyRandom,
        x: i32,
        y: i32,
        z: i32,
    ) -> bool {
        let mut y = y;
        while world.is_air(x, y, z) && y > 2 {
            y -= 1;
        }

        if world.get_block(x, y, z) != Block::SnowBlock {
            return false;
        }

        let radius = next_int(random, self.size - 2) + 2;
        for px in (x - radius)..=(x + radius) {
            for pz in (z - radius)..=(z + radius) {
                let dx = px - x;
                let dz = pz - z;
                if dx * dx + dz * dz > radius * radius {
                    continue;
                }
                for py in (y - 1)..=(y + 1) {
                    let block = world.get_block(px, py, pz);
                    if matches!(block, Block::Dirt | Block::SnowBlock | Block::Ice) {
                        world.set_block_with_metadata(
                            px,
                            py,
                            pz,
                            Block::PackedIce,
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

    #[test]
    fn patches_packed_ice_into_the_snow() {
        let mut world = ChunkWorld::new(0, 0);
        world.fill(0, 63, Block::SnowBlock);
        let mut rng = LegacyRandom::new(5);
        assert!(IcePatchFeature::new(4).place(&mut world, &mut rng, 8, 70, 8));
        // Radius 2..3 disc over three layers, the centre always included.
        assert!(world.block_count(Block::PackedIce) >= 5);
        assert_eq!(world.get_block(8, 63, 8), Block::PackedIce);
        assert_eq!(world.get_block(8, 62, 8), Block::PackedIce);
    }
}
