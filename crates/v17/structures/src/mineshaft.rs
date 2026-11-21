//! `MineshaftStructure`, ported from `MineshaftStructure.java` (start placement).

use std::collections::HashMap;

use sunset_1_7_core::{LegacyRandom, RandomSource};

use crate::bounds::StructureBox;
use crate::feature::{
    StructureFeature, StructureGen, chunk_pos_to_long, next_int, parse_double_unclamped,
};
use crate::piece::StructurePiece;
use crate::start::StructureStart;

/// The mineshaft feature: a chance-gated start in every chunk, weighted by the
/// distance from the origin.
pub struct MineshaftStructure {
    /// The `chance` option.
    pub spawn_chance: f64,
    pub structures: HashMap<i64, StructureStart>,
}

impl MineshaftStructure {
    pub fn new() -> Self {
        Self {
            spawn_chance: 0.004,
            structures: HashMap::new(),
        }
    }

    /// `MineshaftStructure(Map options)`: the `chance` option.
    pub fn with_options(options: &[(&str, &str)]) -> Self {
        let mut structure = Self::new();
        for &(key, value) in options {
            if key == "chance" {
                structure.spawn_chance = parse_double_unclamped(value, structure.spawn_chance);
            }
        }
        structure
    }

    /// `isFeatureChunk`: two draws from the neighborhood RNG, the second gated
    /// by the chunk's distance from the origin.
    pub fn is_feature_chunk(&self, random: &mut LegacyRandom, chunk_x: i32, chunk_z: i32) -> bool {
        random.next_double() < self.spawn_chance
            && next_int(random, 80) < chunk_x.abs().max(chunk_z.abs())
    }
}

impl Default for MineshaftStructure {
    fn default() -> Self {
        Self::new()
    }
}

impl StructureFeature for MineshaftStructure {
    fn name(&self) -> &'static str {
        "Mineshaft"
    }

    fn structures(&mut self) -> &mut HashMap<i64, StructureStart> {
        &mut self.structures
    }

    fn check_and_create(
        &mut self,
        _gen: &mut StructureGen,
        random: &mut LegacyRandom,
        chunk_x: i32,
        chunk_z: i32,
    ) -> bool {
        if !self.is_feature_chunk(random, chunk_x, chunk_z) {
            return false;
        }
        // `MineshaftStart(world, random, chunkX, chunkZ)`.
        let mut start = StructureStart::new(chunk_x, chunk_z);
        // `MineshaftPieces$Room`: the room's bounds, drawn in x, y, z order.
        // The corridor assembly the room feeds is remaining work; until it
        // lands the bounds are the room's alone and `moveBelowSeaLevel`'s
        // draw misses the corridor draws vanilla does first.
        let x = (chunk_x << 4) + 2;
        let z = (chunk_z << 4) + 2;
        let max_x = x + 7 + next_int(random, 6);
        let max_y = 54 + next_int(random, 6);
        let max_z = z + 7 + next_int(random, 6);
        start.pieces.push(StructurePiece::new(
            crate::piece::PieceKind::MineshaftRoom,
            StructureBox::new(x, 50, z, max_x, max_y, max_z),
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

    /// Same chunk, same draws: the neighborhood RNG reseed makes the check
    /// deterministic per chunk.
    #[test]
    fn deterministic_per_chunk() {
        let structure = MineshaftStructure::new();
        for x in -20..20 {
            for z in -20..20 {
                let mut a = LegacyRandom::new(12345);
                let mut b = LegacyRandom::new(12345);
                assert_eq!(
                    structure.is_feature_chunk(&mut a, x, z),
                    structure.is_feature_chunk(&mut b, x, z)
                );
            }
        }
    }

    #[test]
    fn room_bounds_and_sea_level_move() {
        let mut structure = MineshaftStructure::new();
        let mut world = StructureGen::new(12345, false);
        let mut random = LegacyRandom::new(0);
        let mut found = None;
        'outer: for x in 0..120 {
            for z in 0..120 {
                random.set_seed(0);
                if structure.check_and_create(&mut world, &mut random, x, z) {
                    found = Some((x, z));
                    break 'outer;
                }
            }
        }
        let Some((x, z)) = found else {
            return;
        };
        let start = &structure.structures[&chunk_pos_to_long(x, z)];
        let bounds = &start.pieces[0].bounds;
        // The room: x/z spans 8..=13, y at most 54+5=59 before the move, the
        // move puts the top at or below sea level - 10.
        assert!((8..=13).contains(&bounds.span_x()));
        assert!((8..=13).contains(&bounds.span_z()));
        assert!(bounds.max_y <= 53, "max_y {}", bounds.max_y);
    }
}
