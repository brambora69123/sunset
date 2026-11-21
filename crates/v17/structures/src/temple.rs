//! `TempleStructure`, ported from `TempleStructure.java` (start placement).

use std::collections::HashMap;

use sunset_1_7_core::LegacyRandom;

use crate::biomes::{DESERT, DESERT_HILLS, JUNGLE, JUNGLE_HILLS, SWAMPLAND};
use crate::bounds::StructureBox;
use crate::feature::{StructureFeature, StructureGen, chunk_pos_to_long, next_int, parse_int};
use crate::piece::PieceKind;
use crate::start::StructureStart;

/// `TempleStructure.VALID_BIOMES`.
pub const TEMPLE_BIOMES: [i32; 5] = [DESERT, DESERT_HILLS, JUNGLE, JUNGLE_HILLS, SWAMPLAND];

/// The `setRandomSeed` salt temples use.
pub const TEMPLE_SALT: i64 = 14357617;

/// The scattered-feature feature: desert pyramids, jungle temples and witch
/// huts, on the same region grid as villages.
pub struct TempleStructure {
    pub distance: i32,
    pub min_distance: i32,
    pub structures: HashMap<i64, StructureStart>,
}

impl TempleStructure {
    pub fn new() -> Self {
        Self {
            distance: 32,
            min_distance: 8,
            structures: HashMap::new(),
        }
    }

    /// `TempleStructure(Map options)`: the `distance` option.
    pub fn with_options(options: &[(&str, &str)]) -> Self {
        let mut structure = Self::new();
        for &(key, value) in options {
            if key == "distance" {
                structure.distance =
                    parse_int(value, structure.distance, structure.min_distance + 1);
            }
        }
        structure
    }

    /// `isFeatureChunk`: the region's grid pick, then a biome check at the
    /// chunk's center block on the biome layer (vanilla's `getBiome`).
    pub fn is_feature_chunk(&self, world: &mut StructureGen, chunk_x: i32, chunk_z: i32) -> bool {
        let mut region_x = chunk_x;
        let mut region_z = chunk_z;
        if chunk_x < 0 {
            region_x -= self.distance - 1;
        }
        if chunk_z < 0 {
            region_z -= self.distance - 1;
        }
        let mut pick_x = region_x / self.distance;
        let mut pick_z = region_z / self.distance;
        let mut random = world.set_random_seed(pick_x, pick_z, TEMPLE_SALT);
        pick_x *= self.distance;
        pick_z *= self.distance;
        pick_x += next_int(&mut random, self.distance - self.min_distance);
        pick_z += next_int(&mut random, self.distance - self.min_distance);
        if chunk_x == pick_x && chunk_z == pick_z {
            let biome = world.biome_at(chunk_x * 16 + 8, chunk_z * 16 + 8);
            return TEMPLE_BIOMES.contains(&biome);
        }
        false
    }
}

impl Default for TempleStructure {
    fn default() -> Self {
        Self::new()
    }
}

impl StructureFeature for TempleStructure {
    fn name(&self) -> &'static str {
        "Temple"
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
        // `TempleStructure$Start(world, random, chunkX, chunkZ)`: the biome
        // pick first, then the piece constructor's facing draw and bounds.
        let biome = world.biome_at(chunk_x * 16 + 8, chunk_z * 16 + 8);
        let (kind, width, height, depth) = if biome == JUNGLE || biome == JUNGLE_HILLS {
            (PieceKind::JungleTemple, 12, 10, 15)
        } else if biome == SWAMPLAND {
            (PieceKind::WitchHut, 7, 5, 9)
        } else {
            (PieceKind::DesertPyramid, 21, 15, 21)
        };
        let facing = next_int(random, 4);
        let x = chunk_x * 16;
        let z = chunk_z * 16;
        let bounds = if facing == 0 || facing == 2 {
            StructureBox::new(x, 64, z, x + width - 1, 64 + height - 1, z + depth - 1)
        } else {
            StructureBox::new(x, 64, z, x + depth - 1, 64 + height - 1, z + width - 1)
        };
        let mut start = StructureStart::new(chunk_x, chunk_z);
        start
            .pieces
            .push(crate::piece::StructurePiece::new(kind, bounds));
        start.find_bounds();
        self.structures
            .insert(chunk_pos_to_long(chunk_x, chunk_z), start);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::piece::StructurePiece;
    use sunset_1_7_core::RandomSource;

    /// At most one start per 32x32 region, like villages.
    #[test]
    fn at_most_one_start_per_region() {
        let structure = TempleStructure::new();
        let mut world = StructureGen::new(8675309, false);
        for rx in -2..2 {
            for rz in -2..2 {
                let mut hits = 0;
                for x in rx * 32..(rx + 1) * 32 {
                    for z in rz * 32..(rz + 1) * 32 {
                        if structure.is_feature_chunk(&mut world, x, z) {
                            hits += 1;
                        }
                    }
                }
                assert!(hits <= 1, "region {rx},{rz}: {hits} hits");
            }
        }
    }

    #[test]
    fn start_piece_matches_the_biome_table() {
        let mut structure = TempleStructure::new();
        let mut world = StructureGen::new(8675309, false);
        let mut random = LegacyRandom::new(0);
        let mut found = Vec::new();
        'outer: for x in -96..96 {
            for z in -96..96 {
                random.set_seed(0);
                if structure.check_and_create(&mut world, &mut random, x, z) {
                    found.push((x, z));
                    if found.len() == 3 {
                        break 'outer;
                    }
                }
            }
        }
        assert!(!found.is_empty(), "no temples at seed 8675309");
        for &(x, z) in &found {
            let start = &structure.structures[&chunk_pos_to_long(x, z)];
            let piece: &StructurePiece = &start.pieces[0];
            let (w, h, d) = match piece.kind {
                PieceKind::DesertPyramid => (21, 15, 21),
                PieceKind::JungleTemple => (12, 10, 15),
                PieceKind::WitchHut => (7, 5, 9),
                _ => panic!("unexpected kind"),
            };
            let (span_x, span_y, span_z) = (
                piece.bounds.span_x(),
                piece.bounds.span_y(),
                piece.bounds.span_z(),
            );
            assert!(
                (span_x == w && span_y == h && span_z == d)
                    || (span_x == d && span_y == h && span_z == w),
                "{:?} bounds {}x{}x{}",
                piece.kind,
                span_x,
                span_y,
                span_z
            );
            assert_eq!(piece.bounds.min_y, 64);
            assert_eq!(piece.bounds.min_x, x * 16);
            assert_eq!(piece.bounds.min_z, z * 16);
        }
    }
}
