//! Tree features beyond `features::tree`: the acacia, the jungle pair, the
//! bush, the dark oak, the tall birch and the taiga giants.

pub mod acacia;
pub mod bush;
pub mod dark_oak;
pub mod giant;
pub mod giant_jungle;
pub mod giant_pine;
pub mod giant_spruce;
pub mod jungle;
pub mod tall_birch;

/// `Facings.X_OFFSET`, the horizontal direction table.
pub(crate) const X_OFFSET: [i32; 4] = [0, -1, 0, 1];
/// `Facings.Z_OFFSET`, the horizontal direction table.
pub(crate) const Z_OFFSET: [i32; 4] = [1, 0, -1, 0];

#[cfg(test)]
mod tests {
    use crate::block::Block;
    use crate::chunk_world::ChunkWorld;
    use crate::features::trees::acacia::AcaciaTreeFeature;
    use crate::features::trees::bush::BushFeature;
    use crate::features::trees::dark_oak::DarkOakTreeFeature;
    use crate::features::trees::giant_jungle::GiantJungleTreeFeature;
    use crate::features::trees::giant_pine::GiantPineTreeFeature;
    use crate::features::trees::giant_spruce::GiantSpruceTreeFeature;
    use crate::features::trees::jungle::JungleTreeFeature;
    use crate::features::trees::tall_birch::TallBirchTreeFeature;
    use crate::world::GenWorld;
    use sunset_1_7_core::{LegacyRandom, RandomSource};

    fn platform() -> ChunkWorld {
        let mut world = ChunkWorld::new(0, 0);
        world.fill(0, 63, Block::Grass);
        world
    }

    #[test]
    fn acacia_writes_acacia_wood_and_leaves() {
        let mut world = platform();
        let mut random = LegacyRandom::new(101);
        assert!(AcaciaTreeFeature::new().place(&mut world, &mut random, 8, 64, 8));
        assert!(
            world
                .rows()
                .any(|(_, _, _, block, metadata)| block == Block::Log2 && metadata == 0)
        );
        assert!(
            world
                .rows()
                .any(|(_, _, _, block, metadata)| block == Block::Leaves2 && metadata & 3 == 0)
        );
    }

    #[test]
    fn dark_oak_writes_a_2x2_dark_oak_trunk() {
        let mut world = platform();
        let mut random = LegacyRandom::new(101);
        assert!(DarkOakTreeFeature::new().place(&mut world, &mut random, 8, 64, 8));
        let trunk = world
            .rows()
            .filter(|(_, _, _, block, metadata)| *block == Block::Log2 && *metadata == 1)
            .count();
        assert!(trunk >= 4);
        assert!(
            world
                .rows()
                .any(|(_, _, _, block, metadata)| block == Block::Leaves2 && metadata & 3 == 1)
        );
    }

    #[test]
    fn bush_puts_jungle_logs_under_oak_leaves() {
        let mut world = platform();
        let mut random = LegacyRandom::new(101);
        assert!(BushFeature::new(3, 0).place(&mut world, &mut random, 8, 64, 8));
        assert_eq!(world.block_count(Block::Log), 1);
        assert!(
            world
                .rows()
                .any(|(_, _, _, block, metadata)| block == Block::Log && metadata == 3)
        );
        assert!(
            world
                .rows()
                .any(|(_, _, _, block, metadata)| block == Block::Leaves && metadata & 3 == 0)
        );
    }

    #[test]
    fn jungle_places_vines_and_cocoa_on_the_jungle_log() {
        let mut world = platform();
        let mut random = LegacyRandom::new(7);
        let base_height = 4 + random.next_int_bound(7).expect("positive bound");
        assert!(JungleTreeFeature::new(base_height).place(&mut world, &mut random, 8, 64, 8));
        assert!(world.block_count(Block::Vine) > 0);

        let pods: Vec<(i32, i32, i32, u8)> = world
            .rows()
            .filter(|(_, _, _, block, _)| *block == Block::Cocoa)
            .map(|(x, y, z, _, metadata)| (x, y, z, metadata))
            .collect();
        assert!(!pods.is_empty());
        for (x, y, z, metadata) in pods {
            let (dx, dz) = match metadata & 3 {
                0 => (0, 1),
                1 => (-1, 0),
                2 => (0, -1),
                _ => (1, 0),
            };
            assert_eq!(world.get_block(x + dx, y, z + dz), Block::Log);
            assert!((metadata >> 2) & 3 <= 2);
        }
    }

    #[test]
    fn tall_birch_writes_birch_wood_and_leaves() {
        let mut world = platform();
        let mut random = LegacyRandom::new(101);
        assert!(TallBirchTreeFeature::new().place(&mut world, &mut random, 8, 64, 8));
        assert!(
            world
                .rows()
                .any(|(_, _, _, block, metadata)| block == Block::Log && metadata == 2)
        );
        assert!(
            world
                .rows()
                .any(|(_, _, _, block, metadata)| block == Block::Leaves && metadata & 3 == 2)
        );
    }

    #[test]
    fn giant_jungle_writes_a_2x2_jungle_trunk() {
        let mut world = platform();
        let mut random = LegacyRandom::new(101);
        assert!(GiantJungleTreeFeature::new().place(&mut world, &mut random, 8, 64, 8));
        assert!(world.block_count(Block::Vine) > 0);
        let jungle_logs = world
            .rows()
            .filter(|(_, _, _, block, metadata)| *block == Block::Log && *metadata == 3)
            .count();
        assert!(jungle_logs >= 4);
    }

    #[test]
    fn giant_spruce_places_podzol_under_its_crown() {
        let mut world = platform();
        let mut random = LegacyRandom::new(101);
        let spruce = GiantSpruceTreeFeature::new(true);
        assert!(spruce.place(&mut world, &mut random, 8, 64, 8));
        spruce.place_soil(&mut world, &mut random, 8, 64, 8);
        assert!(
            world
                .rows()
                .any(|(_, _, _, block, metadata)| block == Block::Dirt && metadata == 2)
        );
    }

    #[test]
    fn giant_pine_places_spruce_wood() {
        let mut world = platform();
        let mut random = LegacyRandom::new(101);
        let pine = GiantPineTreeFeature::new();
        assert!(pine.place(&mut world, &mut random, 8, 64, 8));
        pine.place_soil(&mut world, &mut random, 8, 64, 8);
        assert!(
            world
                .rows()
                .any(|(_, _, _, block, metadata)| block == Block::Log && metadata == 1)
        );
    }
}
