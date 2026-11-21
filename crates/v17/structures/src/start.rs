//! `StructureStart`, ported from `StructureStart.java`. No NBT here: starts are
//! rebuilt from the seed, and saved starts are the save crate's concern.

use crate::bounds::StructureBox;
use crate::piece::StructurePiece;
use sunset_1_7_core::RandomSource;

/// A structure's start: its pieces, their union bounds, and the start chunk.
#[derive(Clone, Debug)]
pub struct StructureStart {
    pub pieces: Vec<StructurePiece>,
    pub bounds: StructureBox,
    pub chunk_x: i32,
    pub chunk_z: i32,
    /// Village starts: vanilla's `Valid` flag (more than two non-road pieces).
    pub valid: bool,
}

impl StructureStart {
    pub fn new(chunk_x: i32, chunk_z: i32) -> Self {
        Self {
            pieces: Vec::new(),
            bounds: StructureBox::infinite(),
            chunk_x,
            chunk_z,
            valid: true,
        }
    }

    /// `findBounds`: the union of every piece's bounds.
    pub fn find_bounds(&mut self) {
        self.bounds = StructureBox::infinite();
        for piece in &self.pieces {
            self.bounds.union(&piece.bounds);
        }
    }

    /// `moveBelowSeaLevel`: sink the start so its top sits `amount` below sea
    /// level, with a random pick inside the band when there is room.
    pub fn move_below_sea_level(&mut self, random: &mut impl RandomSource, amount: i32) {
        let below = 63 - amount;
        let mut top = self.bounds.span_y() + 1;
        if top < below {
            top += random.next_int_bound(below - top).expect("positive bound");
        }
        self.move_y(top - self.bounds.max_y);
    }

    /// `moveBetweenYCoords`: lift the start into a Y band.
    pub fn move_between_y_coords(
        &mut self,
        random: &mut impl RandomSource,
        min_y: i32,
        max_y: i32,
    ) {
        let room = max_y - min_y + 1 - self.bounds.span_y();
        let top = if room > 1 {
            min_y + random.next_int_bound(room).expect("positive bound")
        } else {
            min_y
        };
        self.move_y(top - self.bounds.min_y);
    }

    /// The shared tail of the two move helpers: shift every bound by `dy`.
    fn move_y(&mut self, dy: i32) {
        self.bounds.move_by(0, dy, 0);
        for piece in &mut self.pieces {
            piece.bounds.move_by(0, dy, 0);
        }
    }

    /// `isValid`: village starts carry the flag, the rest are always valid.
    pub fn is_valid(&self) -> bool {
        self.valid
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::piece::PieceKind;
    use sunset_1_7_core::LegacyRandom;

    #[test]
    fn find_bounds_is_the_pieces_union() {
        let mut start = StructureStart::new(1, 2);
        start.pieces.push(StructurePiece::new(
            PieceKind::VillageStart,
            StructureBox::new(10, 64, 10, 15, 78, 15),
        ));
        start.pieces.push(StructurePiece::new(
            PieceKind::DesertPyramid,
            StructureBox::new(0, 64, 0, 8, 70, 8),
        ));
        start.find_bounds();
        assert_eq!(start.bounds, StructureBox::new(0, 64, 0, 15, 78, 15));
    }

    #[test]
    fn move_below_sea_level_matches_vanilla() {
        // Spiral staircase bounds: y spans 64..=74 (span 11). Vanilla: top =
        // 12, room 53 - 12 = 41, so top = 12 + nextInt(41), dy = top - 74.
        let mut start = StructureStart::new(0, 0);
        start.pieces.push(StructurePiece::new(
            PieceKind::StrongholdStart,
            StructureBox::new(2, 64, 2, 6, 74, 6),
        ));
        start.find_bounds();
        let mut random = LegacyRandom::new(42);
        start.move_below_sea_level(&mut random, 10);
        // Both the start bounds and every piece bound moved by the same dy.
        let expected_dy = (start.bounds.max_y as i64 - 74) as i32;
        assert_eq!(
            start.pieces[0].bounds,
            StructureBox::new(2, 64 + expected_dy, 2, 6, 74 + expected_dy, 6)
        );
        assert!(start.bounds.max_y <= 53);
        assert!(start.bounds.max_y >= 12);
    }
}
