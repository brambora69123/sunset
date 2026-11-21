//! `StrongholdStructure`, ported from `StrongholdStructure.java` (start
//! placement: the ring positions and the start chunk).

use std::collections::HashMap;

use sunset_1_7_core::{LegacyRandom, RandomSource};

use crate::biomes::STRONGHOLD_BIOMES;
use crate::bounds::StructureBox;
use crate::feature::{
    StructureFeature, StructureGen, chunk_pos_to_long, next_int, parse_double, parse_int,
};
use crate::piece::StructurePiece;
use crate::start::StructureStart;
use sunrise_core::jmath;

/// The stronghold feature: `count` strongholds placed on rings around the
/// origin, searched on the noise layer.
pub struct StrongholdStructure {
    /// The `distance` option: chunks per ring run.
    pub distance: f64,
    /// The `spread` option: positions per ring run.
    pub spread: i32,
    /// The `count` option: how many strongholds the world gets.
    pub count: i32,
    /// The ring positions, computed lazily on the first start check.
    pub positions: Vec<(i32, i32)>,
    pub positions_found: bool,
    pub structures: HashMap<i64, StructureStart>,
}

impl StrongholdStructure {
    pub fn new() -> Self {
        Self {
            distance: 32.0,
            spread: 3,
            count: 3,
            positions: Vec::new(),
            positions_found: false,
            structures: HashMap::new(),
        }
    }

    /// `StrongholdStructure(Map options)`: the `distance`, `count` and
    /// `spread` options.
    pub fn with_options(options: &[(&str, &str)]) -> Self {
        let mut structure = Self::new();
        for &(key, value) in options {
            if key == "distance" {
                structure.distance = parse_double(value, structure.distance, 1.0);
            } else if key == "count" {
                structure.count = parse_int(value, structure.count, 1);
            } else if key == "spread" {
                structure.spread = parse_int(value, structure.spread, 1);
            }
        }
        structure
    }

    /// `isFeatureChunk`: the lazy ring search, then the position match.
    pub fn is_feature_chunk(
        &mut self,
        world: &mut StructureGen,
        chunk_x: i32,
        chunk_z: i32,
    ) -> bool {
        if !self.positions_found {
            self.find_positions(world);
        }
        self.positions
            .iter()
            .any(|&(x, z)| x == chunk_x && z == chunk_z)
    }

    /// The ring search `isFeatureChunk` runs once: the world seed seeds a fresh
    /// RNG (not the neighborhood RNG), each position sits on a ring around the
    /// origin and is pulled to a valid biome on the noise layer.
    fn find_positions(&mut self, world: &mut StructureGen) {
        let mut random = LegacyRandom::new(0);
        random.set_seed(world.seed);
        let mut angle = random.next_double() * std::f64::consts::PI * 2.0;
        let mut ring = 1;
        self.positions = vec![(0, 0); self.count as usize];
        for i in 0..self.count as usize {
            let dist = (1.25 * ring as f64 + random.next_double()) * (self.distance * ring as f64);
            let mut pos_x = jmath::round(jmath::cos(angle) * dist) as i32;
            let mut pos_z = jmath::round(jmath::sin(angle) * dist) as i32;
            if let Some((bx, bz)) = world.find_biome(
                (pos_x << 4) + 8,
                (pos_z << 4) + 8,
                112,
                &STRONGHOLD_BIOMES,
                &mut random,
            ) {
                pos_x = bx >> 4;
                pos_z = bz >> 4;
            }
            self.positions[i] = (pos_x, pos_z);
            angle += (std::f64::consts::PI * 2.0) * ring as f64 / self.spread as f64;
            // `spread` mutates while the loop runs, exactly like vanilla.
            if i as i32 == self.spread {
                ring += 2 + next_int(&mut random, 5);
                self.spread += 1 + next_int(&mut random, 2);
            }
        }
        self.positions_found = true;
    }

    /// `getPotentialPositions`: the stronghold chunks' centers, at y 64.
    pub fn potential_positions(&self) -> Vec<(i32, i32, i32)> {
        self.positions
            .iter()
            .map(|&(x, z)| ((x << 4) + 8, 64, (z << 4) + 8))
            .collect()
    }
}

impl Default for StrongholdStructure {
    fn default() -> Self {
        Self::new()
    }
}

impl StructureFeature for StrongholdStructure {
    fn name(&self) -> &'static str {
        "Stronghold"
    }

    fn structures(&mut self) -> &mut HashMap<i64, StructureStart> {
        &mut self.structures
    }

    fn check_and_create(
        &mut self,
        world: &mut StructureGen,
        random: &mut LegacyRandom,
        chunk_x: i32,
        chunk_z: i32,
    ) -> bool {
        if !self.is_feature_chunk(world, chunk_x, chunk_z) {
            return false;
        }
        // `StrongholdStructure$Start(world, random, chunkX, chunkZ)`:
        // `StrongholdPieces.resetWeights()` is piece-pick state for assembly
        // (remaining work), the start piece is the spiral staircase, and the
        // recreate-until-end-portal-room loop needs the assembly's pieces.
        let mut start = StructureStart::new(chunk_x, chunk_z);
        let x = (chunk_x << 4) + 2;
        let z = (chunk_z << 4) + 2;
        let facing = next_int(random, 4);
        let _ = facing;
        start.pieces.push(StructurePiece::new(
            crate::piece::PieceKind::StrongholdStart,
            StructureBox::new(x, 64, z, x + 5 - 1, 74, z + 5 - 1),
        ));
        start.find_bounds();
        start.move_below_sea_level(random, 10);
        self.structures
            .insert(chunk_pos_to_long(chunk_x, chunk_z), start);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The ring search finds exactly `count` positions, and a start lands
    /// only on one of them.
    #[test]
    fn positions_and_start_match() {
        let mut structure = StrongholdStructure::new();
        let mut world = StructureGen::new(9069452278850244, false);
        assert!(!structure.is_feature_chunk(&mut world, 0, 0));
        assert_eq!(structure.positions.len(), 3);
        // A start is created on a position chunk and nowhere else nearby.
        let mut starts = 0;
        let &(sx, sz) = structure.positions.first().expect("positions");
        for x in sx - 2..=sx + 2 {
            for z in sz - 2..=sz + 2 {
                let mut random = LegacyRandom::new(0);
                if structure.check_and_create(&mut world, &mut random, x, z) {
                    assert_eq!((x, z), (sx, sz));
                    starts += 1;
                }
            }
        }
        assert_eq!(starts, 1);
    }

    /// The first ring's distance sits in 1.25..2.25 ring runs (before the
    /// biome search moves the position by at most the 112-block search range).
    #[test]
    fn first_ring_distance() {
        let mut structure = StrongholdStructure::new();
        let mut world = StructureGen::new(9069452278850244, false);
        structure.find_positions(&mut world);
        for &(x, z) in &structure.positions {
            let dist = ((x * x + z * z) as f64).sqrt();
            let max = (1.25 + 1.0) * 32.0 + 112.0 / 16.0 + 1.0;
            assert!(dist >= 1.25 * 32.0 - 112.0 / 16.0 - 1.0, "dist {dist}");
            assert!(dist <= max, "dist {dist}");
        }
    }

    #[test]
    fn potential_positions_are_chunk_centers() {
        let mut structure = StrongholdStructure::new();
        let mut world = StructureGen::new(9069452278850244, false);
        structure.find_positions(&mut world);
        for (pos, &(x, z)) in structure
            .potential_positions()
            .iter()
            .zip(structure.positions.iter())
        {
            assert_eq!(*pos, ((x << 4) + 8, 64, (z << 4) + 8));
        }
    }
}
