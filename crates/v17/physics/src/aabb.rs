//! 1.7.10 `Box` (net.minecraft.util.math.Box): the entity AABB.

// The negated comparisons are the Java source's shape, kept line for line
// (and NaN-safe); clippy would rewrite them into the same machine code.
#![allow(clippy::neg_cmp_op_on_partial_ord)]

/// The entity and block collision box. `intersect_*` clip a moving shape: the
/// receiver is the collision box, the `box` argument the moving shape.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb {
    pub min_x: f64,
    pub min_y: f64,
    pub min_z: f64,
    pub max_x: f64,
    pub max_y: f64,
    pub max_z: f64,
}

impl Aabb {
    pub const fn new(
        min_x: f64,
        min_y: f64,
        min_z: f64,
        max_x: f64,
        max_y: f64,
        max_z: f64,
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

    /// A full-cube block at this block position.
    pub const fn cube(x: i32, y: i32, z: i32) -> Self {
        Self::new(
            x as f64,
            y as f64,
            z as f64,
            x as f64 + 1.0,
            y as f64 + 1.0,
            z as f64 + 1.0,
        )
    }

    /// `Box.move`: every face by the same delta.
    pub fn moved(&self, dx: f64, dy: f64, dz: f64) -> Self {
        Self::new(
            self.min_x + dx,
            self.min_y + dy,
            self.min_z + dz,
            self.max_x + dx,
            self.max_y + dy,
            self.max_z + dz,
        )
    }

    /// `Box.expanded`: grown only along the movement, for a collision query.
    pub fn expanded(&self, dx: f64, dy: f64, dz: f64) -> Self {
        let (mut min_x, mut min_y, mut min_z) = (self.min_x, self.min_y, self.min_z);
        let (mut max_x, mut max_y, mut max_z) = (self.max_x, self.max_y, self.max_z);
        if dx < 0.0 {
            min_x += dx;
        }
        if dx > 0.0 {
            max_x += dx;
        }
        if dy < 0.0 {
            min_y += dy;
        }
        if dy > 0.0 {
            max_y += dy;
        }
        if dz < 0.0 {
            min_z += dz;
        }
        if dz > 0.0 {
            max_z += dz;
        }
        Self::new(min_x, min_y, min_z, max_x, max_y, max_z)
    }

    /// `Box.grown`: every face outward.
    pub fn grown(&self, dx: f64, dy: f64, dz: f64) -> Self {
        Self::new(
            self.min_x - dx,
            self.min_y - dy,
            self.min_z - dz,
            self.max_x + dx,
            self.max_y + dy,
            self.max_z + dz,
        )
    }

    /// `Box.contract`: every face inward.
    pub fn contract(&self, dx: f64, dy: f64, dz: f64) -> Self {
        Self::new(
            self.min_x + dx,
            self.min_y + dy,
            self.min_z + dz,
            self.max_x - dx,
            self.max_y - dy,
            self.max_z - dz,
        )
    }

    /// `Box.intersects`: touching faces do not intersect.
    pub fn intersects(&self, other: &Aabb) -> bool {
        if other.max_x <= self.min_x || other.min_x >= self.max_x {
            return false;
        }
        if other.max_y <= self.min_y || other.min_y >= self.max_y {
            return false;
        }
        !(other.max_z <= self.min_z) && !(other.min_z >= self.max_z)
    }

    /// `Box.intersectX`: clip `box`'s X movement against this box.
    pub fn intersect_x(&self, box_: &Aabb, limit: f64) -> f64 {
        if box_.max_y <= self.min_y || box_.min_y >= self.max_y {
            return limit;
        }
        if !(box_.max_z <= self.min_z) && !(box_.min_z >= self.max_z) {
            if limit > 0.0 && box_.max_x <= self.min_x {
                let gap = self.min_x - box_.max_x;
                if gap < limit {
                    return gap;
                }
            }
            if limit < 0.0 && box_.min_x >= self.max_x {
                let gap = self.max_x - box_.min_x;
                if gap > limit {
                    return gap;
                }
            }
        }
        limit
    }

    /// `Box.intersectY`: clip `box`'s Y movement against this box.
    pub fn intersect_y(&self, box_: &Aabb, limit: f64) -> f64 {
        if box_.max_x <= self.min_x || box_.min_x >= self.max_x {
            return limit;
        }
        if !(box_.max_z <= self.min_z) && !(box_.min_z >= self.max_z) {
            if limit > 0.0 && box_.max_y <= self.min_y {
                let gap = self.min_y - box_.max_y;
                if gap < limit {
                    return gap;
                }
            }
            if limit < 0.0 && box_.min_y >= self.max_y {
                let gap = self.max_y - box_.min_y;
                if gap > limit {
                    return gap;
                }
            }
        }
        limit
    }

    /// `Box.intersectZ`: clip `box`'s Z movement against this box.
    pub fn intersect_z(&self, box_: &Aabb, limit: f64) -> f64 {
        if box_.max_x <= self.min_x || box_.min_x >= self.max_x {
            return limit;
        }
        if !(box_.max_y <= self.min_y) && !(box_.min_y >= self.max_y) {
            if limit > 0.0 && box_.max_z <= self.min_z {
                let gap = self.min_z - box_.max_z;
                if gap < limit {
                    return gap;
                }
            }
            if limit < 0.0 && box_.min_z >= self.max_z {
                let gap = self.max_z - box_.min_z;
                if gap > limit {
                    return gap;
                }
            }
        }
        limit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn landing_is_clamped_to_the_block_top() {
        let ground = Aabb::cube(0, -1, 0);
        // Entity floating 0.05 above the ground, falling 0.1.
        let shape = Aabb::new(-0.3, 0.05, -0.3, 0.3, 1.85, 0.3);
        let dy = ground.intersect_y(&shape, -0.1);
        assert_eq!(dy, -0.05);
    }

    #[test]
    fn rising_over_a_block_is_free_when_clear() {
        let block = Aabb::cube(3, 0, 0);
        // Entity standing on the ground next to the block, rising half a step.
        let shape = Aabb::new(2.2, 0.5, -0.3, 2.8, 2.35, 0.3);
        assert_eq!(block.intersect_y(&shape, 0.5), 0.5);
    }

    #[test]
    fn aabb_clamp_gaps_are_exact() {
        let wall = Aabb::cube(3, 0, 0);
        let shape = Aabb::new(2.2, 0.0, -0.3, 2.8, 1.85, 0.3);
        assert_eq!(wall.intersect_x(&shape, 0.5), 3.0 - 2.8);
        assert_eq!(wall.intersect_x(&shape, -0.5), -0.5);
    }

    #[test]
    fn touching_faces_do_not_intersect() {
        let a = Aabb::new(0.0, 0.0, 0.0, 1.0, 1.0, 1.0);
        let b = Aabb::new(1.0, 0.0, 0.0, 2.0, 1.0, 1.0);
        assert!(!a.intersects(&b));
        let c = Aabb::new(0.5, 0.0, 0.0, 1.5, 1.0, 1.0);
        assert!(a.intersects(&c));
    }
}
