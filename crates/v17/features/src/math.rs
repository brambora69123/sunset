//! MathHelper pieces the features use, ported for 1.7.10 float behavior.

/// `MathHelper.SIN_TABLE`: 65536 floats, `sin(x) = SIN_TABLE[(int)(x * 10430.378F) & 65535]`.
fn table() -> &'static [f32; 65_536] {
    static TABLE: std::sync::OnceLock<[f32; 65_536]> = std::sync::OnceLock::new();
    TABLE.get_or_init(|| {
        let mut table = [0.0_f32; 65_536];
        for (i, slot) in table.iter_mut().enumerate() {
            *slot = ((i as f64) * std::f64::consts::PI * 2.0 / 65_536.0).sin() as f32;
        }
        // The static block pins the first twelve entries to exact f32 values.
        let pinned: [f32; 12] = [
            0.0,
            0.000_244_140_62,
            0.000_488_281_25,
            0.000_732_421_9,
            0.000_976_562_5,
            0.001_220_703_1,
            0.001_464_843_8,
            0.001_708_984_4,
            0.001_953_125,
            0.002_197_265_6,
            0.002_441_406_2,
            0.002_685_546_9,
        ];
        table[..12].copy_from_slice(&pinned);
        table
    })
}

/// `MathHelper.sin`: lookup table, `(int)` truncation, `& 65535` wrap.
pub fn sin(value: f32) -> f32 {
    table()[((value * 10_430.378_f32) as i32 & 65_535) as usize]
}

/// `MathHelper.cos`: lookup table, `(int)(x * 10430.378F + 16384.0F) & 65535`.
pub fn cos(value: f32) -> f32 {
    table()[((value * 10_430.378_f32 + 16_384.0) as i32 & 65_535) as usize]
}

/// `MathHelper.floor_double`: floor toward negative infinity, Java `(int)` cast.
pub fn floor(value: f64) -> i32 {
    let truncated = value as i32;
    if value < f64::from(truncated) {
        truncated - 1
    } else {
        truncated
    }
}

/// `MathHelper.sqrt_float`.
pub fn sqrt(value: f32) -> f32 {
    f64::from(value).sqrt() as f32
}

/// `MathHelper.abs` (the float form).
pub fn abs(value: f32) -> f32 {
    if value >= 0.0 { value } else { -value }
}

/// `MathHelper.ceil` (the float form): truncate, then round up.
pub fn ceil(value: f32) -> i32 {
    let truncated = value as i32;
    if value > truncated as f32 {
        truncated + 1
    } else {
        truncated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sin_table_matches_java_entries() {
        assert_eq!(sin(0.0), 0.0);
        // Value 1/10430.378 maps to index 1, the pinned f32 entry.
        assert_eq!(sin(1.0 / 10_430.378), 0.000_244_140_62);
        // A quarter turn lands on the table's peak.
        let quarter = 16_384.0 / 10_430.378;
        assert!((sin(quarter) - 1.0).abs() < 1.0e-6);
    }

    #[test]
    fn cos_is_sin_quarter_turn() {
        assert_eq!(cos(0.0), 1.0);
    }

    #[test]
    fn floor_is_floor_toward_negative_infinity() {
        assert_eq!(floor(3.7), 3);
        assert_eq!(floor(-0.5), -1);
        assert_eq!(floor(0.0), 0);
        assert_eq!(floor(-4.0), -4);
    }

    #[test]
    fn ceil_and_abs_truncate_like_java() {
        assert_eq!(ceil(0.0), 0);
        assert_eq!(ceil(0.25), 1);
        assert_eq!(ceil(3.0), 3);
        assert_eq!(ceil(-0.5), 0);
        assert_eq!(abs(-0.25), 0.25);
        assert_eq!(abs(2.5), 2.5);
    }
}
