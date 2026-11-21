//! `net.minecraft.util.math.MathHelper` (1.7.10), the functions the mob AI uses.

use std::sync::OnceLock;

/// `MathHelper.floor(double)` — `(int)Math.floor(value)`.
pub fn floor(value: f64) -> i32 {
    value.floor() as i32
}

/// `MathHelper.floor(float)`.
pub fn floor_f32(value: f32) -> i32 {
    value.floor() as i32
}

/// `MathHelper.ceil(double)` — `(int)Math.ceil(value)`.
pub fn ceil(value: f64) -> i32 {
    value.ceil() as i32
}

/// `MathHelper.sqrt(float)` — `(float)Math.sqrt((double)value)`.
pub fn sqrt(value: f32) -> f32 {
    (value as f64).sqrt() as f32
}

/// `MathHelper.sqrt_double(double)`.
pub fn sqrt_f64(value: f64) -> f64 {
    value.sqrt()
}

/// `Math.atan2` — std's correctly-rounded atan2.
pub fn atan2(y: f64, x: f64) -> f64 {
    f64::atan2(y, x)
}

/// `MathHelper.wrapDegrees(float)` (1.7.10 float form).
pub fn wrap_degrees(value: f32) -> f32 {
    let mut v = value % 360.0;
    if v >= 180.0 {
        v -= 360.0;
    }
    if v < -180.0 {
        v += 360.0;
    }
    v
}

/// `MathHelper.SIN_TABLE` — 65536 floats, built once on first use.
fn sin_table() -> &'static [f32; 65536] {
    static TABLE: OnceLock<[f32; 65536]> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut table = [0.0f32; 65536];
        for (i, slot) in table.iter_mut().enumerate() {
            *slot = (i as f64 * std::f64::consts::PI * 2.0 / 65536.0).sin() as f32;
        }
        table
    })
}

/// `MathHelper.sin(float)` — table lookup, like vanilla.
pub fn sin(value: f32) -> f32 {
    sin_table()[((value * 10430.378) as i32 & 0xFFFF) as usize]
}

/// `MathHelper.cos(float)` — table lookup, like vanilla.
pub fn cos(value: f32) -> f32 {
    sin_table()[((value * 10430.378 + 16384.0) as i32 & 0xFFFF) as usize]
}
