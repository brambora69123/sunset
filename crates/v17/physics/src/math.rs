//! 1.7.10 `MathHelper` subset the physics needs, exact.

use std::sync::OnceLock;

use sunrise_core::jmath;

/// `SINE_TABLE[(int)(x * 10430.378F) & 65535]`, built like 1.7.10's static init:
/// `(float)Math.sin((double)i * Math.PI * 2.0 / 65536.0)` per entry.
fn sine_table() -> &'static [f32; 65536] {
    static TABLE: OnceLock<[f32; 65536]> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut table = [0.0f32; 65536];
        for (i, entry) in table.iter_mut().enumerate() {
            *entry = jmath::sin(i as f64 * std::f64::consts::PI * 2.0 / 65536.0) as f32;
        }
        table
    })
}

/// `MathHelper.sin(float)` — the sine table, not `Math.sin`.
pub fn sin(x: f32) -> f32 {
    sine_table()[((x * 10430.378f32) as i32 & 0xFFFF) as usize]
}

/// `MathHelper.cos(float)` — the sine table, offset a quarter turn.
pub fn cos(x: f32) -> f32 {
    sine_table()[((x * 10430.378f32 + 16384.0f32) as i32 & 0xFFFF) as usize]
}

/// `MathHelper.sqrt(float)` — `(float)Math.sqrt(x)`.
pub fn sqrt(x: f32) -> f32 {
    ((x as f64).sqrt()) as f32
}

/// `MathHelper.sqrt(double)` — `(float)Math.sqrt(x)`.
pub fn sqrt_double(x: f64) -> f32 {
    (x.sqrt()) as f32
}

/// `MathHelper.floor(double)`.
pub fn floor(x: f64) -> i32 {
    let truncated = x as i32;
    if x < truncated as f64 {
        truncated - 1
    } else {
        truncated
    }
}

/// `MathHelper.floor(float)`.
pub fn floor_float(x: f32) -> i32 {
    let truncated = x as i32;
    if x < truncated as f32 {
        truncated - 1
    } else {
        truncated
    }
}

/// `MathHelper.ceil(double)`.
pub fn ceil(x: f64) -> i32 {
    let truncated = x as i32;
    if x > truncated as f64 {
        truncated + 1
    } else {
        truncated
    }
}

/// `MathHelper.ceil(float)`.
pub fn ceil_float(x: f32) -> i32 {
    let truncated = x as i32;
    if x > truncated as f32 {
        truncated + 1
    } else {
        truncated
    }
}

/// `MathHelper.absMax(double)`.
pub fn abs_max(a: f64, b: f64) -> f64 {
    let a = if a < 0.0 { -a } else { a };
    let b = if b < 0.0 { -b } else { b };
    if a > b { a } else { b }
}

/// `MathHelper.wrapDegrees(float)`.
pub fn wrap_degrees(degrees: f32) -> f32 {
    let degrees = degrees % 360.0;
    let degrees = if degrees >= 180.0 {
        degrees - 360.0
    } else {
        degrees
    };
    if degrees < -180.0 {
        degrees + 360.0
    } else {
        degrees
    }
}

/// `MathHelper.clamp(float)`.
pub fn clamp_float(x: f32, min: f32, max: f32) -> f32 {
    if x < min {
        min
    } else if x > max {
        max
    } else {
        x
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sine_table_matches_unit_circle() {
        assert_eq!(sin(0.0), 0.0);
        assert_eq!(cos(0.0), 1.0);
        assert_eq!(sin(std::f32::consts::PI / 2.0), 1.0);
        assert_eq!(sin(std::f32::consts::PI * 3.0 / 2.0), -1.0);
    }

    #[test]
    fn floor_ceil_truncate_like_java() {
        assert_eq!(floor(1.9), 1);
        assert_eq!(floor(-0.1), -1);
        assert_eq!(floor(0.0), 0);
        assert_eq!(ceil(1.1), 2);
        assert_eq!(ceil(-1.9), -1);
        assert_eq!(ceil(2.0), 2);
        assert_eq!(ceil_float(0.47), 1);
    }

    #[test]
    fn sqrt_casts_to_f32() {
        assert_eq!(sqrt(2.0f32), (2.0f64).sqrt() as f32);
        assert_eq!(sqrt_double(2.0), (2.0f64).sqrt() as f32);
    }
}
