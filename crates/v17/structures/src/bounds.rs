//! A 3D structure bounding box, ported from `StructureBox.java`.
//!
//! Min and max are inclusive on every axis, like vanilla's.

/// A structure bounding box. Fields are public like vanilla's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StructureBox {
    pub min_x: i32,
    pub min_y: i32,
    pub min_z: i32,
    pub max_x: i32,
    pub max_y: i32,
    pub max_z: i32,
}

impl StructureBox {
    pub const fn new(
        min_x: i32,
        min_y: i32,
        min_z: i32,
        max_x: i32,
        max_y: i32,
        max_z: i32,
    ) -> Self {
        Self {
            min_x,
            min_y,
            min_z,
            max_x,
            max_y,
            max_z,
        }
    }

    /// The two-argument constructor: a full-height box over an XZ rectangle
    /// (vanilla fixes its y span to 1..=512).
    pub const fn over_xz(min_x: i32, min_z: i32, max_x: i32, max_z: i32) -> Self {
        Self {
            min_x,
            min_y: 1,
            min_z,
            max_x,
            max_y: 512,
            max_z,
        }
    }

    /// `StructureBox.infinite()`: the empty box `findBounds` starts from.
    pub const fn infinite() -> Self {
        Self::new(i32::MAX, i32::MAX, i32::MAX, i32::MIN, i32::MIN, i32::MIN)
    }

    pub fn intersects(&self, other: &StructureBox) -> bool {
        self.max_x >= other.min_x
            && self.min_x <= other.max_x
            && self.max_z >= other.min_z
            && self.min_z <= other.max_z
            && self.max_y >= other.min_y
            && self.min_y <= other.max_y
    }

    /// The 2D overload: an XZ rectangle against this box, y ignored.
    pub fn intersects_xz(&self, min_x: i32, min_z: i32, max_x: i32, max_z: i32) -> bool {
        self.max_x >= min_x && self.min_x <= max_x && self.max_z >= min_z && self.min_z <= max_z
    }

    pub fn union(&mut self, other: &StructureBox) {
        self.min_x = self.min_x.min(other.min_x);
        self.min_y = self.min_y.min(other.min_y);
        self.min_z = self.min_z.min(other.min_z);
        self.max_x = self.max_x.max(other.max_x);
        self.max_y = self.max_y.max(other.max_y);
        self.max_z = self.max_z.max(other.max_z);
    }

    pub fn move_by(&mut self, dx: i32, dy: i32, dz: i32) {
        self.min_x += dx;
        self.min_y += dy;
        self.min_z += dz;
        self.max_x += dx;
        self.max_y += dy;
        self.max_z += dz;
    }

    pub fn contains(&self, x: i32, y: i32, z: i32) -> bool {
        x >= self.min_x
            && x <= self.max_x
            && z >= self.min_z
            && z <= self.max_z
            && y >= self.min_y
            && y <= self.max_y
    }

    pub fn span_x(&self) -> i32 {
        self.max_x - self.min_x + 1
    }

    pub fn span_y(&self) -> i32 {
        self.max_y - self.min_y + 1
    }

    pub fn span_z(&self) -> i32 {
        self.max_z - self.min_z + 1
    }

    pub fn center_x(&self) -> i32 {
        self.min_x + (self.max_x - self.min_x + 1) / 2
    }

    pub fn center_y(&self) -> i32 {
        self.min_y + (self.max_y - self.min_y + 1) / 2
    }

    pub fn center_z(&self) -> i32 {
        self.min_z + (self.max_z - self.min_z + 1) / 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infinite_is_empty() {
        let inf = StructureBox::infinite();
        assert!(!inf.contains(0, 64, 0));
        let box1 = StructureBox::new(0, 64, 0, 10, 70, 10);
        let mut grown = StructureBox::infinite();
        grown.union(&box1);
        assert_eq!(grown, box1);
    }

    #[test]
    fn intersects_and_contains_boundaries() {
        let box1 = StructureBox::new(0, 1, 0, 15, 512, 15);
        // Inclusive edges.
        assert!(box1.contains(0, 1, 0));
        assert!(box1.contains(15, 512, 15));
        assert!(!box1.contains(16, 1, 0));
        assert!(box1.intersects_xz(15, 15, 20, 20));
        assert!(!box1.intersects_xz(16, 0, 20, 20));
        let box2 = StructureBox::new(10, 64, 10, 20, 70, 20);
        assert!(box1.intersects(&box2));
    }

    #[test]
    fn union_and_move() {
        let mut b = StructureBox::over_xz(0, 0, 0, 0);
        b.union(&StructureBox::over_xz(5, -3, 7, 2));
        assert_eq!(b, StructureBox::over_xz(0, -3, 7, 2));
        assert_eq!(b.span_x(), 8);
        b.move_by(1, 2, 3);
        assert_eq!(b, StructureBox::new(1, 3, 0, 8, 514, 5));
    }

    #[test]
    fn centers() {
        let b = StructureBox::new(0, 0, 0, 9, 9, 9);
        assert_eq!((b.center_x(), b.center_y(), b.center_z()), (5, 5, 5));
    }
}
