//! The placement driver: `Generator.java`'s neighborhood scan plus
//! `StructureFeature.java`'s start check and the biome source calls it needs.

use std::collections::HashMap;

use sunset_1_7::biome::Layer;
use sunset_1_7_core::{LegacyRandom, RandomSource};

use crate::start::StructureStart;

/// `MathHelper.parseInt`: parse or keep the default, then clamp to `min`.
pub fn parse_int(s: &str, default: i32, min: i32) -> i32 {
    let v = s.parse().unwrap_or(default);
    if v < min { min } else { v }
}

/// `MathHelper.parseInt` without the clamp.
pub fn parse_int_unclamped(s: &str, default: i32) -> i32 {
    s.parse().unwrap_or(default)
}

/// `MathHelper.parseDouble`: parse or keep the default, then clamp to `min`.
pub fn parse_double(s: &str, default: f64, min: f64) -> f64 {
    let v = s.parse().unwrap_or(default);
    if v < min { min } else { v }
}

/// `MathHelper.parseDouble` without the clamp.
pub fn parse_double_unclamped(s: &str, default: f64) -> f64 {
    s.parse().unwrap_or(default)
}

/// `MathHelper.nextInt(random, min, max)`.
pub fn next_int_between(random: &mut impl RandomSource, min: i32, max: i32) -> i32 {
    if min >= max {
        min
    } else {
        random
            .next_int_bound(max - min + 1)
            .expect("positive bound")
            + min
    }
}

/// `Random.nextInt(bound)`: the bound is a positive constant at every call
/// site, so the invalid-bound error is unreachable.
pub fn next_int(random: &mut impl RandomSource, bound: i32) -> i32 {
    random.next_int_bound(bound).expect("positive bound")
}

/// `ChunkPos.toLong`.
pub fn chunk_pos_to_long(x: i32, z: i32) -> i64 {
    (x as i64 & 0xFFFF_FFFF) | ((z as i64 & 0xFFFF_FFFF) << 32)
}

/// The world-side data structure placement reads: the world seed and the biome
/// source's two layers, from `Layer.init`.
pub struct StructureGen {
    pub seed: i64,
    /// `BiomeSource.noiseBiomes`: the layer `isBiomeWithin` and `findBiome`
    /// read at 1/4 block scale.
    pub noise: Layer,
    /// `BiomeSource.biomes`: the layer `getBiome` reads at block scale
    /// (through vanilla's 16x16 biome cache, which this skips; the layer is a
    /// pure function of the position, so the answer is the same).
    pub biomes: Layer,
}

impl StructureGen {
    /// The biome source for a world: `Layer.init`'s `[noise_biomes, biomes]`.
    pub fn new(seed: i64, large_biomes: bool) -> Self {
        let [noise, biomes, _] = sunset_1_7::biome::init(seed, large_biomes);
        Self {
            seed,
            noise,
            biomes,
        }
    }

    /// `World.setRandomSeed`: the region RNG villages and temples draw from.
    pub fn set_random_seed(&self, x: i32, z: i32, salt: i64) -> LegacyRandom {
        // Java's multiply wraps; the terms stay in range for reachable
        // regions, the wrap keeps it exact regardless.
        let seed = (x as i64)
            .wrapping_mul(341873128712)
            .wrapping_add((z as i64).wrapping_mul(132897987541))
            .wrapping_add(self.seed)
            .wrapping_add(salt);
        LegacyRandom::new(seed)
    }

    /// `BiomeSource.getBiome`: the biome id at a block position.
    pub fn biome_at(&mut self, x: i32, z: i32) -> i32 {
        self.biomes.area(x, z, 1, 1)[0]
    }

    /// `BiomeSource.isBiomeWithin`: every noise-layer cell within `range` of
    /// the block position must be one of `allowed`.
    pub fn biome_within(&mut self, x: i32, z: i32, range: i32, allowed: &[i32]) -> bool {
        let min_x = (x - range) >> 2;
        let min_z = (z - range) >> 2;
        let max_x = (x + range) >> 2;
        let max_z = (z + range) >> 2;
        let w = (max_x - min_x + 1) as usize;
        let h = (max_z - min_z + 1) as usize;
        self.noise
            .area(min_x, min_z, w as i32, h as i32)
            .iter()
            .all(|id| allowed.contains(id))
    }

    /// `BiomeSource.findBiome`: pick a random viable noise-layer cell within
    /// `range` of the block position; draws from `random`. Returns the block
    /// position of the cell's corner (vanilla's `BlockPos`, y 0).
    pub fn find_biome(
        &mut self,
        x: i32,
        z: i32,
        range: i32,
        allowed: &[i32],
        random: &mut LegacyRandom,
    ) -> Option<(i32, i32)> {
        let min_x = (x - range) >> 2;
        let min_z = (z - range) >> 2;
        let max_x = (x + range) >> 2;
        let max_z = (z + range) >> 2;
        let w = (max_x - min_x + 1) as usize;
        let h = (max_z - min_z + 1) as usize;
        let area = self.noise.area(min_x, min_z, w as i32, h as i32);
        let mut found: Option<(i32, i32)> = None;
        let mut count: i32 = 0;
        for (i, id) in area.iter().enumerate() {
            if !allowed.contains(id) {
                continue;
            }
            if found.is_none() || random.next_int_bound(count + 1).expect("positive bound") == 0 {
                let bx = (min_x + (i % w) as i32) << 2;
                let bz = (min_z + (i / w) as i32) << 2;
                found = Some((bx, bz));
                count += 1;
            }
        }
        found
    }
}

/// A structure feature: which start chunks get a start, ported from
/// `StructureFeature`'s abstract `isFeatureChunk`/`createStructure` pair.
pub trait StructureFeature {
    /// `getName`: the saved-data id.
    fn name(&self) -> &'static str;
    /// The known starts, keyed by `ChunkPos.toLong`.
    fn structures(&mut self) -> &mut HashMap<i64, StructureStart>;
    /// Run `isFeatureChunk` and, on a hit, `createStructure`, which registers
    /// the start in the map. Returns whether a start was created.
    fn check_and_create(
        &mut self,
        world: &mut StructureGen,
        random: &mut LegacyRandom,
        chunk_x: i32,
        chunk_z: i32,
    ) -> bool;

    /// `StructureFeature.place` (the per-start-chunk check): one free
    /// `nextInt()` draw before the check, and the map skips known starts.
    fn prepare_start(
        &mut self,
        world: &mut StructureGen,
        random: &mut LegacyRandom,
        chunk_x: i32,
        chunk_z: i32,
    ) -> bool {
        if !self
            .structures()
            .contains_key(&chunk_pos_to_long(chunk_x, chunk_z))
        {
            random.next_int();
            return self.check_and_create(world, random, chunk_x, chunk_z);
        }
        false
    }

    /// `Generator.place` (MapGenBase's generate): scan the 8-chunk
    /// neighborhood around the target chunk, reseeding the RNG for every start
    /// chunk.
    ///
    /// ponytail: this reference lacks real 1.7.10's `/ 2 * 2 + 1` odd-adjustment
    /// of the two seed multipliers (MapGenBase.generate); ported as the
    /// decompiled source reads. Add the adjustment if parity shows it.
    fn generate(&mut self, world: &mut StructureGen, chunk_x: i32, chunk_z: i32) {
        let mut random = LegacyRandom::new(0);
        random.set_seed(world.seed);
        let a = random.next_long();
        let b = random.next_long();
        for x in (chunk_x - 8)..=(chunk_x + 8) {
            for z in (chunk_z - 8)..=(chunk_z + 8) {
                random
                    .set_seed((x as i64).wrapping_mul(a) ^ (z as i64).wrapping_mul(b) ^ world.seed);
                self.prepare_start(world, &mut random, x, z);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_pos_to_long_packs_and_orders() {
        assert_eq!(
            chunk_pos_to_long(1, 2),
            (1i64 & 0xFFFF_FFFF) | ((2i64 & 0xFFFF_FFFF) << 32)
        );
        assert_eq!(chunk_pos_to_long(-1, -1), -1i64);
        assert_ne!(chunk_pos_to_long(1, 0), chunk_pos_to_long(0, 1));
    }

    #[test]
    fn parse_helpers_match_mathhelper() {
        assert_eq!(parse_int("7", 3, 0), 7);
        assert_eq!(parse_int("x", 3, 0), 3);
        assert_eq!(parse_int("1", 3, 8), 8);
        assert_eq!(parse_int_unclamped("5", 3), 5);
        assert_eq!(parse_double("0.5", 0.004, 0.0), 0.5);
        assert_eq!(parse_double_unclamped("q", 2.0), 2.0);
    }

    #[test]
    fn next_int_between_matches_mathhelper() {
        let mut random = LegacyRandom::new(7);
        assert_eq!(next_int_between(&mut random, 5, 5), 5);
        assert_eq!(next_int_between(&mut random, 6, 5), 6);
        let v = next_int_between(&mut random, 2, 4);
        assert!((2..=4).contains(&v));
    }
}
