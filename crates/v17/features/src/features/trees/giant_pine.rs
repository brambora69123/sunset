//! `GiantPineTreeFeature`: the mega taiga's `GIANT_PINE_TREE`, the short-crown
//! form of `GiantSpruceTreeFeature`.

use crate::features::trees::giant_spruce::GiantSpruceTreeFeature;
use crate::world::GenWorld;
use sunset_1_7_core::LegacyRandom;

/// `TaigaBiome.GIANT_PINE_TREE` = `GiantSpruceTreeFeature(false, false)`.
pub struct GiantPineTreeFeature {
    inner: GiantSpruceTreeFeature,
}

impl Default for GiantPineTreeFeature {
    fn default() -> GiantPineTreeFeature {
        GiantPineTreeFeature::new()
    }
}

impl GiantPineTreeFeature {
    pub fn new() -> GiantPineTreeFeature {
        GiantPineTreeFeature {
            inner: GiantSpruceTreeFeature::new(false),
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

    /// The giant pine carries the same podzol soil as the giant spruce.
    pub fn place_soil(
        &self,
        world: &mut dyn GenWorld,
        random: &mut LegacyRandom,
        x: i32,
        y: i32,
        z: i32,
    ) {
        self.inner.place_soil(world, random, x, y, z);
    }
}
