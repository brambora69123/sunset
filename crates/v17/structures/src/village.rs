//! `VillageStructure`, ported from `VillageStructure.java` (start placement).

use std::collections::HashMap;

use sunset_1_7_core::LegacyRandom;

use crate::biomes::{DESERT, DESERT_HILLS, PLAINS, SAVANNA};
use crate::bounds::StructureBox;
use crate::feature::{
    StructureFeature, StructureGen, chunk_pos_to_long, next_int, next_int_between, parse_int,
};
use crate::piece::{PieceKind, StructurePiece};
use crate::start::StructureStart;

/// `VillageStructure.VALID_BIOMES`.
pub const VILLAGE_BIOMES: [i32; 3] = [PLAINS, DESERT, SAVANNA];

/// The `setRandomSeed` salt villages use.
pub const VILLAGE_SALT: i64 = 10387312;

/// The village feature: region-grid starts every `distance` chunks, at least
/// `min_distance` into the region, in the valid biomes.
pub struct VillageStructure {
    /// The `size` option.
    pub size: i32,
    pub distance: i32,
    pub min_distance: i32,
    pub structures: HashMap<i64, StructureStart>,
}

impl VillageStructure {
    pub fn new() -> Self {
        Self {
            size: 0,
            distance: 32,
            min_distance: 8,
            structures: HashMap::new(),
        }
    }

    /// `VillageStructure(Map options)`: the `size` and `distance` options.
    pub fn with_options(options: &[(&str, &str)]) -> Self {
        let mut structure = Self::new();
        for &(key, value) in options {
            if key == "size" {
                structure.size = parse_int(value, structure.size, 0);
            } else if key == "distance" {
                structure.distance =
                    parse_int(value, structure.distance, structure.min_distance + 1);
            }
        }
        structure
    }

    /// `isFeatureChunk`: the region's grid pick, then a biome check at the
    /// chunk's center block on the noise layer (vanilla's `isBiomeWithin`,
    /// range 0).
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
        let mut random = world.set_random_seed(pick_x, pick_z, VILLAGE_SALT);
        pick_x *= self.distance;
        pick_z *= self.distance;
        pick_x += next_int(&mut random, self.distance - self.min_distance);
        pick_z += next_int(&mut random, self.distance - self.min_distance);
        if chunk_x == pick_x && chunk_z == pick_z {
            return world.biome_within(chunk_x * 16 + 8, chunk_z * 16 + 8, 0, &VILLAGE_BIOMES);
        }
        false
    }
}

impl Default for VillageStructure {
    fn default() -> Self {
        Self::new()
    }
}

impl StructureFeature for VillageStructure {
    fn name(&self) -> &'static str {
        "Village"
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
        // `VillageStructure$Start(world, random, chunkX, chunkZ, size)`.
        let mut start = StructureStart::new(chunk_x, chunk_z);
        // First the building weights (`VillagePieces.getPieceWeights`): the
        // drawn max amounts, in getPieceWeights order (SmallHouse, Priest,
        // Librarian, Farmer, Butcher, DoubleFarm, Farm, Blacksmith, Large).
        // The draws must happen for the RNG sequence to stay exact; the full
        // weight list and the piece assembly it feeds are remaining work.
        let size = self.size;
        let _weights = [
            next_int_between(random, 2 + size, 4 + size * 2),
            next_int_between(random, size, 1 + size),
            next_int_between(random, size, 2 + size),
            next_int_between(random, 2 + size, 5 + size * 3),
            next_int_between(random, size, 2 + size),
            next_int_between(random, 1 + size, 4 + size),
            next_int_between(random, 2 + size, 4 + size * 2),
            next_int_between(random, 0, 1 + size),
            next_int_between(random, size, 3 + size * 2),
        ];
        // Then the well (`VillagePieces$Start`): the facing draw, the biome
        // flag, the bounds at the chunk's +2 corner.
        let x = (chunk_x << 4) + 2;
        let z = (chunk_z << 4) + 2;
        // The facing draw must happen for the RNG sequence; the well bounds
        // are the same for every facing.
        let _facing = next_int(random, 4);
        let biome = world.biome_at(x, z);
        let is_desert = biome == DESERT || biome == DESERT_HILLS;
        start.pieces.push(StructurePiece {
            kind: PieceKind::VillageStart,
            is_desert,
            bounds: StructureBox::new(x, 64, z, x + 6 - 1, 78, z + 6 - 1),
        });
        start.find_bounds();
        // vanilla's Valid flag counts the assembled non-road pieces (> 2);
        // the piece-assembly step (remaining work) must set it.
        start.valid = true;
        self.structures
            .insert(chunk_pos_to_long(chunk_x, chunk_z), start);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::piece::PieceKind;
    use sunset_1_7_core::RandomSource;

    /// Vanilla gives a village start to at most one chunk per 32x32 region,
    /// inside the region's first `distance - min_distance` cells.
    #[test]
    fn at_most_one_start_per_region() {
        let structure = VillageStructure::new();
        let mut world = StructureGen::new(8675309, false);
        for rx in -2..2 {
            for rz in -2..2 {
                let mut hits = Vec::new();
                for x in rx * 32..(rx + 1) * 32 {
                    for z in rz * 32..(rz + 1) * 32 {
                        if structure.is_feature_chunk(&mut world, x, z) {
                            hits.push((x, z));
                        }
                    }
                }
                assert!(hits.len() <= 1, "region {rx},{rz}: {:?}", hits);
                if let Some(&(x, z)) = hits.first() {
                    assert!((rx * 32..rx * 32 + 24).contains(&x));
                    assert!((rz * 32..rz * 32 + 24).contains(&z));
                }
            }
        }
    }

    #[test]
    fn start_is_the_well_at_the_chunk_corner() {
        let mut structure = VillageStructure::new();
        let mut world = StructureGen::new(8675309, false);
        let mut random = LegacyRandom::new(0);
        // Some chunk the biome check passes on (any chunk where a start gets
        // created); scan until one appears.
        let mut hit = None;
        'outer: for x in 0..32 {
            for z in 0..32 {
                random.set_seed(0);
                if structure.check_and_create(&mut world, &mut random, x, z) {
                    hit = Some((x, z));
                    break 'outer;
                }
            }
        }
        let Some((x, z)) = hit else {
            return; // no viable biome at this seed's region picks; nothing to assert
        };
        let start = &structure.structures[&chunk_pos_to_long(x, z)];
        let piece = &start.pieces[0];
        assert_eq!(piece.kind, PieceKind::VillageStart);
        assert_eq!(
            piece.bounds,
            StructureBox::new(
                (x << 4) + 2,
                64,
                (z << 4) + 2,
                (x << 4) + 7,
                78,
                (z << 4) + 7
            )
        );
        assert_eq!(start.bounds, piece.bounds);
        assert!(start.is_valid());
    }
}
