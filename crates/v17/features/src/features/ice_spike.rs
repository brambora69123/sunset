//! `IceSpikeFeature`: the packed-ice spikes of the ice plains spikes.

use crate::block::Block;
use crate::math;
use crate::world::{FLAG_NO_NEIGHBOR_NOTIFY, GenWorld, next_int};
use sunset_1_7_core::{LegacyRandom, RandomSource};

/// `IceSpikeFeature`.
pub(crate) struct IceSpikeFeature;

impl IceSpikeFeature {
    /// RNG order: `nextInt(4)` (start lift), `nextInt(4)+7` (height),
    /// `nextInt(2)` (base radius), the rare `nextInt(60)`/`nextInt(30)` tall
    /// spike, one `nextFloat` per ring cell of each layer, and the taper's
    /// `nextInt(5)`s.
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

        y += next_int(random, 4);
        let height = next_int(random, 4) + 7;
        let radius = height / 4 + next_int(random, 2);
        if radius > 1 && next_int(random, 60) == 0 {
            y += 10 + next_int(random, 30);
        }

        for layer in 0..height {
            let width = (1.0 - layer as f32 / height as f32) * radius as f32;
            let extent = math::ceil(width);

            for dx in -extent..=extent {
                let ax = math::abs(dx as f32) - 0.25;
                for dz in -extent..=extent {
                    let az = math::abs(dz as f32) - 0.25;
                    if (dx == 0 && dz == 0 || ax * ax + az * az <= width * width)
                        && (dx != -extent && dx != extent && dz != -extent && dz != extent
                            || random.next_float() <= 0.75)
                    {
                        let block = world.get_block(x + dx, y + layer, z + dz);
                        if block.is_air_material()
                            || matches!(block, Block::Dirt | Block::SnowBlock | Block::Ice)
                        {
                            self.set(world, x + dx, y + layer, z + dz);
                        }

                        if layer != 0 && extent > 1 {
                            let block = world.get_block(x + dx, y - layer, z + dz);
                            if block.is_air_material()
                                || matches!(block, Block::Dirt | Block::SnowBlock | Block::Ice)
                            {
                                self.set(world, x + dx, y - layer, z + dz);
                            }
                        }
                    }
                }
            }
        }

        // `radius - 1`, clamped to the 0..1 the taper loop allows.
        let taper = (radius - 1).clamp(0, 1);

        for dx in -taper..=taper {
            for dz in -taper..=taper {
                let mut py = y - 1;
                let mut budget = if dx.abs() == 1 && dz.abs() == 1 {
                    next_int(random, 5)
                } else {
                    50
                };

                while py > 50 {
                    let block = world.get_block(x + dx, py, z + dz);
                    if !block.is_air_material()
                        && !matches!(
                            block,
                            Block::Dirt | Block::SnowBlock | Block::Ice | Block::PackedIce
                        )
                    {
                        break;
                    }

                    self.set(world, x + dx, py, z + dz);
                    py -= 1;
                    budget -= 1;
                    if budget <= 0 {
                        py -= next_int(random, 5) + 1;
                        budget = next_int(random, 5);
                    }
                }
            }
        }

        true
    }

    fn set(&self, world: &mut dyn GenWorld, x: i32, y: i32, z: i32) {
        world.set_block_with_metadata(x, y, z, Block::PackedIce, 0, FLAG_NO_NEIGHBOR_NOTIFY);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk_world::ChunkWorld;

    #[test]
    fn spikes_packed_ice_up_from_snow() {
        let mut world = ChunkWorld::new(0, 0);
        world.fill(0, 63, Block::SnowBlock);
        let mut rng = LegacyRandom::new(11);
        assert!(IceSpikeFeature.place(&mut world, &mut rng, 8, 70, 8));
        assert!(world.block_count(Block::PackedIce) > 10);
        assert!(
            world
                .rows()
                .any(|(_, y, _, block, _)| block == Block::PackedIce && y > 64)
        );
    }

    #[test]
    fn no_spike_without_snow_floor() {
        let mut world = ChunkWorld::new(0, 0);
        world.fill(0, 63, Block::Stone);
        let mut rng = LegacyRandom::new(11);
        assert!(!IceSpikeFeature.place(&mut world, &mut rng, 8, 70, 8));
        assert_eq!(world.block_count(Block::PackedIce), 0);
    }
}
