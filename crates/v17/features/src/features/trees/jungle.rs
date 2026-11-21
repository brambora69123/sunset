//! `JungleTreeFeature`: the jungle's 1x1 tree, `TreeFeature` with jungle
//! variants and the vine/cocoa paths switched on.

use crate::features::tree::OakTreeFeature;
use crate::world::GenWorld;
use sunset_1_7_core::LegacyRandom;

/// `TreeFeature(false, baseHeight, 3, 3, true)`. `JungleBiome.pickTree` draws
/// `4 + nextInt(7)` for `baseHeight` before calling `place`.
pub struct JungleTreeFeature {
    inner: OakTreeFeature,
}

impl JungleTreeFeature {
    pub fn new(base_height: i32) -> JungleTreeFeature {
        JungleTreeFeature {
            inner: OakTreeFeature {
                base_height,
                log_variant: 3,
                leaves_variant: 3,
                place_vines: true,
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
