//! `TallBirchTreeFeature`: `ForestBiome.TALL_BIRCH_TREE`, a birch that draws
//! up to seven extra height blocks.

use crate::features::tree::BirchTreeFeature;
use crate::world::GenWorld;
use sunset_1_7_core::LegacyRandom;

/// `BirchTreeFeature(false, true)`.
pub struct TallBirchTreeFeature {
    inner: BirchTreeFeature,
}

impl Default for TallBirchTreeFeature {
    fn default() -> TallBirchTreeFeature {
        TallBirchTreeFeature::new()
    }
}

impl TallBirchTreeFeature {
    pub fn new() -> TallBirchTreeFeature {
        TallBirchTreeFeature {
            inner: BirchTreeFeature {
                tall: true,
                notify_neighbors: false,
            },
        }
    }

    pub fn place(
        &self,
        world: &mut dyn GenWorld,
        random: &mut LegacyRandom,
        x: i32,
        y: i32,
        z: i32,
    ) -> bool {
        self.inner.place(world, random, x, y, z)
    }
}
