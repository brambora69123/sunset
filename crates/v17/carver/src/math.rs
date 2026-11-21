//! The `MathHelper` subset the carvers use, exact to vanilla 1.7.10.
//!
//! `sin` and `cos` are 1.7.10's table lookups over `SINE_TABLE`; the table is
//! what `MathHelper`'s static block generates (`Math.sin` per entry), using
//! [`sunrise_core::jmath`]'s HotSpot-exact `Math.sin`.

use std::sync::LazyLock;
use sunrise_core::jmath;

/// `MathHelper.SINE_TABLE`: `sin(i * PI * 2 / 65536)` as f32, 65536 entries.
static SINE_TABLE: LazyLock<Vec<f32>> = LazyLock::new(|| {
    (0..65536)
        .map(|i| jmath::sin((i as f64) * std::f64::consts::PI * 2.0 / 65536.0) as f32)
        .collect()
});

/// `MathHelper.sin(float)`: `SINE_TABLE[(int)(x * 10430.378F) & 65535]`.
pub fn sin(x: f32) -> f32 {
    SINE_TABLE[((x * 10430.378) as i32 & 0xFFFF) as usize]
}

/// `MathHelper.cos(float)`: `SINE_TABLE[(int)(x * 10430.378F + 16384.0F) & 65535]`.
pub fn cos(x: f32) -> f32 {
    SINE_TABLE[((x * 10430.378 + 16384.0) as i32 & 0xFFFF) as usize]
}

/// `MathHelper.floor(double)`: `(int)x`, one less when the cast truncates upward.
pub fn floor(x: f64) -> i32 {
    let i = x as i32;
    if x < i as f64 { i - 1 } else { i }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_edges() {
        // sin(0) is table[0], sin at the quarter turn is table[16384].
        assert_eq!(sin(0.0), 0.0);
        assert_eq!(cos(0.0), 1.0);
        assert_eq!(sin(std::f32::consts::PI / 2.0), 1.0);
        // sin(pi) through the lookup table is a hair off zero, never exact.
        assert!(sin(std::f32::consts::PI).abs() < 0.001);
    }

    #[test]
    fn table_matches_math_sin() {
        // Every entry is the f32 of `Math.sin` at its own angle.
        let i = 12_345;
        let angle = (i as f64) * std::f64::consts::PI * 2.0 / 65536.0;
        assert_eq!(SINE_TABLE[i], jmath::sin(angle) as f32);
    }

    #[test]
    fn floor_matches_java() {
        assert_eq!(floor(1.5), 1);
        assert_eq!(floor(-0.5), -1);
        assert_eq!(floor(0.0), 0);
        assert_eq!(floor(-1.0), -1);
        assert_eq!(floor(256.7), 256);
        assert_eq!(floor(255.9999999), 255);
    }
}
