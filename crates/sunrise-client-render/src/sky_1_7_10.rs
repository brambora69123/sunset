//! The 1.7.10 sky's own arithmetic: `Dimension.getTimeOfDay`, `World.getSkyColor`,
//! `Dimension.getFogColor`, `getSunriseColor`, `getStarBrightness` and `getCloudColor`.
//!
//! The era keeps no timeline: its sky changes because these expressions read the day clock.
//! `environment::Environment::sample` runs them for a 1.7.10 content root and writes the
//! result into the [`crate::environment::Sky`] the sky pass, the fog and the lightmap read.
//! Every constant here is a line of the 1.7.10 source: the names in the comments are that
//! source's.

/// `Biome.getSkyColor`'s temperature when the caller names no biome: plains',
/// 0.8, the era's most common sky. [`environment::Environment::sample`] gets the
/// camera biome's own temperature on a 1.7.10 root.
///
/// [`environment::Environment::sample`]: crate::environment::Environment::sample
pub const PLAINS_TEMPERATURE: f32 = 0.8;

/// `Dimension.getTimeOfDay(time, partialTick)`: the celestial time, 0 at noon.
pub fn time_of_day(day_time: i64) -> f32 {
    let mut t = (day_time.rem_euclid(24000) as f32) / 24000.0 - 0.25;
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    let before = t;
    // `1.0F - (float)((Math.cos(t * PI) + 1.0) / 2.0)`
    let eased = 1.0 - ((f64::from(t) * std::f64::consts::PI).cos() + 1.0) as f32 / 2.0;
    before + (eased - before) / 3.0
}

/// `MathHelper.cos(value)`: 1.7.10's 65,536-entry sine table, indexed the way the era does
/// (`(int)(value * 10430.378F + 16384.0F) & 65535`).
pub fn math_helper_cos(value: f32) -> f32 {
    let i = (value * 10430.378 + 16384.0) as i32 & 0xFFFF;
    sunrise_core::geometry::trig::sine_table()[i as usize]
}

/// `MathHelper.sin(value)`, the same table without the quarter-turn offset.
pub fn math_helper_sin(value: f32) -> f32 {
    let i = (value * 10430.378) as i32 & 0xFFFF;
    sunrise_core::geometry::trig::sine_table()[i as usize]
}

/// `MathHelper.cos(timeOfDay * 2 * PI) * 2 + 0.5`, clamped to 0..1: how bright the era's
/// sunlit colours are.
pub fn daylight(time_of_day: f32) -> f32 {
    (math_helper_cos(time_of_day * std::f32::consts::TAU) * 2.0 + 0.5).clamp(0.0, 1.0)
}

/// `Biome.getSkyColor(temperature)`:
///
/// ```text
/// temperature /= 3;
/// return Color.getHSBColor(0.62222224F - temperature * 0.05F,
///                         0.5F + temperature * 0.1F, 1.0F).getRGB();
/// ```
///
/// `Color.getHSBColor` builds a `java.awt.Color` from the floats, so each channel is the
/// 8-bit value `(int)(channel * 255 + 0.5)` and the 26.3 byte channels divide it by 255.
pub fn biome_sky_color(temperature: f32) -> [f32; 3] {
    let t = (temperature / 3.0).clamp(-1.0, 1.0);
    let [r, g, b] = hsb_to_rgb(0.622_222_24 - t * 0.05, 0.5 + t * 0.1, 1.0);
    [quantize(r), quantize(g), quantize(b)]
}

/// `World.getSkyColor`: the biome's colour at the day clock's brightness, then the rain and
/// thunder desaturations.
pub fn sky_color(temperature: f32, time_of_day: f32, rain: f32, thunder: f32) -> [f32; 3] {
    let f = daylight(time_of_day);
    let mut c = biome_sky_color(temperature).map(|v| v * f);
    if rain > 0.0 {
        let gray = (c[0] * 0.3 + c[1] * 0.59 + c[2] * 0.11) * 0.6;
        let k = 1.0 - rain * 0.75;
        c = c.map(|v| v * k + gray * (1.0 - k));
    }
    if thunder > 0.0 {
        let gray = (c[0] * 0.3 + c[1] * 0.59 + c[2] * 0.11) * 0.2;
        let k = 1.0 - thunder * 0.75;
        c = c.map(|v| v * k + gray * (1.0 - k));
    }
    c
}

/// `Dimension.getFogColor(timeOfDay, tickDelta)` for the dimension `dimension`
/// (`minecraft:overworld`, `minecraft:the_nether`, `minecraft:the_end`).
pub fn fog_color(dimension: &str, time_of_day: f32) -> [f32; 3] {
    match dimension {
        "the_nether" | "minecraft:the_nether" => [0.2, 0.03, 0.03],
        "the_end" | "minecraft:the_end" => {
            // 10518688 = 0xA080A0; `var4 * 0.0F + 0.15F` is a constant 0.15.
            const END: u32 = 10_518_688;
            [
                f32::from((END >> 16 & 255) as u8) / 255.0 * 0.15,
                f32::from((END >> 8 & 255) as u8) / 255.0 * 0.15,
                f32::from((END & 255) as u8) / 255.0 * 0.15,
            ]
        }
        _ => {
            let f =
                (math_helper_cos(time_of_day * std::f32::consts::TAU) * 2.0 + 0.5).clamp(0.0, 1.0);
            [
                0.752_941_2 * (f * 0.94 + 0.06),
                0.847_058_83 * (f * 0.94 + 0.06),
                1.0 * (f * 0.91 + 0.09),
            ]
        }
    }
}

/// `Dimension.getSunriseColor`: the sunset fan's colour and alpha, or zero alpha when the sun
/// is not within 0.4 of the horizon (`null` in the source).
pub fn sunrise_color(dimension: &str, time_of_day: f32) -> [f32; 4] {
    if matches!(
        dimension,
        "the_nether" | "minecraft:the_nether" | "the_end" | "minecraft:the_end"
    ) {
        return [0.0; 4];
    }
    const TILT: f32 = 0.4;
    let f = math_helper_cos(time_of_day * std::f32::consts::TAU);
    if f < -TILT || f > TILT {
        return [0.0; 4];
    }
    let t = f / TILT * 0.5 + 0.5;
    let mut alpha = 1.0 - (1.0 - math_helper_sin(t * std::f32::consts::PI)) * 0.99;
    alpha *= alpha;
    [t * 0.3 + 0.7, t * t * 0.7 + 0.2, t * t * 0.0 + 0.2, alpha]
}

/// `World.getStarBrightness`.
pub fn star_brightness(time_of_day: f32) -> f32 {
    let f =
        (1.0 - (math_helper_cos(time_of_day * std::f32::consts::TAU) * 2.0 + 0.25)).clamp(0.0, 1.0);
    f * f * 0.5
}

/// `World.getCloudColor`: white darkened by the day clock and by rain and thunder. The era
/// draws every cloud at alpha 0.8 (`WorldRenderer.renderClouds` and `renderFancyClouds`).
pub fn cloud_color(time_of_day: f32, rain: f32, thunder: f32) -> [f32; 4] {
    let mut c = [1.0f32; 3];
    if rain > 0.0 {
        let gray = (c[0] * 0.3 + c[1] * 0.59 + c[2] * 0.11) * 0.6;
        let k = 1.0 - rain * 0.95;
        c = c.map(|v| v * k + gray * (1.0 - k));
    }
    let f = daylight(time_of_day);
    c = [
        c[0] * (f * 0.9 + 0.1),
        c[1] * (f * 0.9 + 0.1),
        c[2] * (f * 0.85 + 0.15),
    ];
    if thunder > 0.0 {
        let gray = (c[0] * 0.3 + c[1] * 0.59 + c[2] * 0.11) * 0.2;
        let k = 1.0 - thunder * 0.95;
        c = c.map(|v| v * k + gray * (1.0 - k));
    }
    [c[0], c[1], c[2], 0.8]
}

/// `World.getSunAngle(tickDelta)`, in degrees: the sun, moon and stars turn this far about the
/// X axis after the sky's -90 degree Y turn (`WorldRenderer.renderSky`).
pub fn sun_angle(time_of_day: f32) -> f32 {
    time_of_day * 360.0
}

/// `Dimension.getMoonPhase` in `SkyRenderer.buildMoonPhases`'s order, from the era's world
/// time: the moon is the full moon at phase 0.
pub fn moon_phase(day_time: i64) -> usize {
    (day_time.div_euclid(24000) % 8 + 8).rem_euclid(8) as usize
}

/// `World.calculateAmbientDarkness`: 0 at noon, 11 at midnight, less in rain and thunder.
pub fn ambient_darkness(time_of_day: f32, rain: f32, thunder: f32) -> i32 {
    let daylight = daylight(time_of_day);
    let light = daylight * (1.0 - rain * 5.0 / 16.0) * (1.0 - thunder * 5.0 / 16.0);
    ((1.0 - light) * 11.0) as i32
}

/// `World.calculateAmbientLight`: the day's light, 0.2 at midnight and 1.0 at noon,
/// scaled by rain and thunder. `GameRenderer.updateLightMap` multiplies the sky's
/// brightness by `ambient * 0.95 + 0.05`.
pub fn ambient_light(time_of_day: f32, rain: f32, thunder: f32) -> f32 {
    // `var3 = 1.0F - (cos(timeOfDay * 2 * PI) * 2.0F + 0.2F)`, clamped, then
    // `1.0F - var3` and the weather factors.
    let raw = 1.0 - (math_helper_cos(time_of_day * std::f32::consts::TAU) * 2.0 + 0.2);
    let light = 1.0 - raw.clamp(0.0, 1.0);
    let light = light * (1.0 - rain * 5.0 / 16.0);
    let light = light * (1.0 - thunder * 5.0 / 16.0);
    light * 0.8 + 0.2
}

/// The lightmap's sky factor: `GameRenderer.updateLightMap`'s
/// `calculateAmbientLight * 0.95F + 0.05F`. At noon 1.0, at midnight 0.24.
pub fn lightmap_sky_factor(time_of_day: f32, rain: f32, thunder: f32) -> f32 {
    ambient_light(time_of_day, rain, thunder) * 0.95 + 0.05
}

/// The surface sky light the era's lightmap and fog brightness read: `15 - ambientDarkness`.
pub fn sky_light_level(time_of_day: f32, rain: f32, thunder: f32) -> f32 {
    15.0 - ambient_darkness(time_of_day, rain, thunder) as f32
}

/// `Dimension.initBrightnessTable`'s table entry for a raw light level 0..=15.
pub fn brightness_table(level: u8) -> f32 {
    let f = 1.0 - f32::from(level.min(15)) / 15.0;
    (1.0 - f) / (f * 3.0 + 1.0)
}

/// `GameRenderer.tick`'s steady fog brightness: the camera block's brightness, blended toward
/// full by the view distance (`brightness * (1 - viewDistance / 16) + viewDistance / 16`).
pub fn fog_brightness(sky_light: f32, view_distance_chunks: i32) -> f32 {
    let level = sky_light.round().clamp(0.0, 15.0) as u8;
    let brightness = brightness_table(level);
    let view = (view_distance_chunks as f32 / 16.0).clamp(0.0, 1.0);
    brightness * (1.0 - view) + view
}

/// `WorldRenderer.renderClouds`'s cloud plane height: 128 above the overworld, 8 in the End.
pub fn cloud_height(dimension: &str) -> f32 {
    if matches!(dimension, "the_end" | "minecraft:the_end") {
        8.0
    } else {
        128.0
    }
}

/// `java.awt.Color.getHSBColor`.
fn hsb_to_rgb(h: f32, s: f32, b: f32) -> [f32; 3] {
    if s == 0.0 {
        return [b, b, b];
    }
    let h = (h - h.floor()) * 6.0;
    let i = h.floor() as i32;
    let f = h - h.floor();
    let p = b * (1.0 - s);
    let q = b * (1.0 - s * f);
    let t = b * (1.0 - s * (1.0 - f));
    match i {
        0 => [b, t, p],
        1 => [q, b, p],
        2 => [p, b, t],
        3 => [p, q, b],
        4 => [t, p, b],
        _ => [b, p, q],
    }
}

/// `Color`'s 8-bit channel: Java's `(int)(c * 255 + 0.5)`, then the renderer's `/ 255`.
fn quantize(c: f32) -> f32 {
    f32::from((c * 255.0 + 0.5).clamp(0.0, 255.0) as u8) / 255.0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Values read off the 1.7.10 expressions, not computed by the code under test:
    /// `(24000 ticks - 6000) / 24000` maps noon to 0, and the cosine ease is applied twice.
    #[test]
    fn the_day_clock_maps_noon_to_zero_and_midnight_to_half() {
        assert_eq!(time_of_day(6000), 0.0);
        assert!((time_of_day(0) - 0.784_517_8).abs() < 1e-6);
        assert!((time_of_day(12000) - 0.215_482_2).abs() < 1e-6);
        assert!((time_of_day(18000) - 0.5).abs() < 1e-6);
        assert!((time_of_day(24000 + 6000)).to_bits() == time_of_day(6000).to_bits());
    }

    #[test]
    fn plains_sky_colour_is_the_eras_end_of_dusk_hue() {
        // Color.getHSBColor(0.62222224 - 0.8/3*0.05, 0.5 + 0.8/3*0.1, 1.0) is
        // (0.473333, 0.655911, 1.0); java.awt.Color rounds it to (121, 167, 255).
        assert_eq!(
            biome_sky_color(PLAINS_TEMPERATURE),
            [121.0 / 255.0, 167.0 / 255.0, 1.0]
        );
        // A cold biome is redder, a hot one greener-blue; both stay within the era's clamp.
        assert_eq!(biome_sky_color(0.0), [128.0 / 255.0, 161.0 / 255.0, 1.0]);
        // 9.0 clamps to 1: HSB(0.57222224, 0.6, 1) is (0.4, 0.74, 1.0) -> (102, 189, 255).
        assert_eq!(biome_sky_color(9.0), [102.0 / 255.0, 189.0 / 255.0, 1.0]);
    }

    #[test]
    fn the_sky_is_the_biome_colour_scaled_by_the_sun() {
        // Noon: daylight = cos(0) * 2 + 0.5 clamped to 1.
        assert_eq!(
            sky_color(PLAINS_TEMPERATURE, 0.0, 0.0, 0.0),
            [121.0 / 255.0, 167.0 / 255.0, 1.0]
        );
        // Midnight: cos(timeOfDay * 2PI) = -1 -> -1.5, clamped to 0.
        assert_eq!(
            sky_color(PLAINS_TEMPERATURE, time_of_day(18000), 0.0, 0.0),
            [0.0; 3]
        );
        // Dawn at midnight, rain at full: the grey is (r*.3+g*.59+b*.11)*.6 of the
        // clamped colour, mixed at 1 - .75 * rain.
        let night = sky_color(PLAINS_TEMPERATURE, time_of_day(18000), 1.0, 0.0);
        assert_eq!(night, [0.0; 3]);
        let day_rain = sky_color(PLAINS_TEMPERATURE, 0.0, 1.0, 0.0);
        let gray = (121.0 / 255.0 * 0.3 + 167.0 / 255.0 * 0.59 + 0.11) * 0.6;
        let k = 0.25;
        assert!((day_rain[0] - (121.0 / 255.0 * k + gray * (1.0 - k))).abs() < 1e-6);
        assert!((day_rain[1] - (167.0 / 255.0 * k + gray * (1.0 - k))).abs() < 1e-6);
        assert!((day_rain[2] - (k + gray * (1.0 - k))).abs() < 1e-6);
    }

    /// The overworld fog is `(0.7529412, 0.84705883, 1.0)` scaled per channel; the Nether and
    /// the End return their own constants.
    #[test]
    fn each_dimension_fogs_as_its_own_provider_does() {
        let noon = fog_color("minecraft:overworld", 0.0);
        assert!((noon[0] - 0.752_941_2).abs() < 1e-6);
        assert!((noon[1] - 0.847_058_83).abs() < 1e-6);
        assert!((noon[2] - 1.0).abs() < 1e-6);
        // Midnight: f = 0, so 0.94 * 0 + 0.06 and 0.91 * 0 + 0.09.
        let night = fog_color("minecraft:overworld", time_of_day(18000));
        assert!((night[0] - 0.752_941_2 * 0.06).abs() < 1e-6);
        assert!((night[1] - 0.847_058_83 * 0.06).abs() < 1e-6);
        assert!((night[2] - 0.09).abs() < 1e-6);
        assert_eq!(fog_color("minecraft:the_nether", 0.0), [0.2, 0.03, 0.03]);
        let end = fog_color("minecraft:the_end", 0.0);
        assert!((end[0] - 160.0 / 255.0 * 0.15).abs() < 1e-6);
        assert!((end[1] - 128.0 / 255.0 * 0.15).abs() < 1e-6);
        assert!((end[2] - 160.0 / 255.0 * 0.15).abs() < 1e-6);
    }

    /// The sunrise fan is drawn only while the cosine is within 0.4 of the horizon; at
    /// `timeOfDay = 0` (`t = 0.25` of the ease) its apex colour is the source's
    /// `(0.85, 0.375, 0.2)` reached through the sine curve.
    #[test]
    fn sunrise_has_colour_only_near_the_horizon() {
        // Noon: cos(0) = 1, outside the ±0.4 band.
        assert_eq!(sunrise_color("minecraft:overworld", 0.0), [0.0; 4]);
        let dawn = sunrise_color("minecraft:overworld", time_of_day(0));
        assert!(dawn[3] > 0.0);
        // cos(0.7845178 * 2PI) = 0.2194, so t = 0.7743; the source blends 0.7..1.0, 0.2..0.9.
        assert!((dawn[0] - 0.930_689).abs() < 1e-6);
        assert!((dawn[1] - 0.613_913_2).abs() < 1e-6);
        assert!((dawn[2] - 0.2).abs() < 1e-6);
        assert!((dawn[3] - 0.445_077_9).abs() < 1e-6);
        assert_eq!(sunrise_color("minecraft:the_end", 0.0), [0.0; 4]);
        assert_eq!(sunrise_color("minecraft:the_nether", 0.0), [0.0; 4]);
    }

    #[test]
    fn stars_peak_at_midnight() {
        assert_eq!(star_brightness(0.0), 0.0);
        // Midnight is 1.7.10's `timeOfDay = 0.5`: cos(PI) = -1, so 1 - (-2 + 0.25) = 2.75,
        // clamped to 1, squared and halved.
        assert!((star_brightness(time_of_day(18000)) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn clouds_are_white_dimmed_by_the_clock() {
        assert_eq!(cloud_color(0.0, 0.0, 0.0), [1.0, 1.0, 1.0, 0.8]);
        // Midnight: f = 0 -> 0.1, 0.1, 0.15.
        let night = cloud_color(time_of_day(18000), 0.0, 0.0);
        assert!((night[0] - 0.1).abs() < 1e-6);
        assert!((night[1] - 0.1).abs() < 1e-6);
        assert!((night[2] - 0.15).abs() < 1e-6);
        assert_eq!(night[3], 0.8);
    }

    #[test]
    fn the_light_shares_the_eras_brightness_table() {
        assert_eq!(brightness_table(15), 1.0);
        // Level 0: (1 - 1) / (3 + 1) = 0.
        assert_eq!(brightness_table(0), 0.0);
        // Level 4 is 1 - 4/15 = 11/15: (4/15) / (11/5 + 1) = 4/48 = 1/12.
        assert!((brightness_table(4) - 1.0 / 12.0).abs() < 1e-6);
        // Halfway through the view distances the fog is halfway to the camera's brightness;
        // at 16 chunks it is full, and at 2 it is blended an eighth over.
        assert!((fog_brightness(4.0, 8) - (1.0 / 12.0 * 0.5 + 0.5)).abs() < 1e-6);
        assert_eq!(fog_brightness(4.0, 16), 1.0);
        assert!((fog_brightness(4.0, 2) - (1.0 / 12.0 * 0.875 + 0.125)).abs() < 1e-6);
        assert!((fog_brightness(15.0, 2) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn light_levels_are_the_surface_sky_light() {
        assert_eq!(sky_light_level(0.0, 0.0, 0.0), 15.0);
        let midnight = time_of_day(18000);
        let dark = ambient_darkness(midnight, 0.0, 0.0);
        assert_eq!(dark, 11);
        assert_eq!(sky_light_level(midnight, 0.0, 0.0), 4.0);
        assert_eq!(moon_phase(0), 0);
        assert_eq!(moon_phase(24_000), 1);
        assert_eq!(moon_phase(-24_000), 7);
    }

    /// `World.calculateAmbientLight`: 0.2 at midnight, 1.0 at noon, and the lightmap's
    /// `* 0.95 + 0.05` over it. The lightmap's block term keeps its own curve.
    #[test]
    fn the_lightmap_sky_factor_follows_the_eras_ambient_light() {
        assert!((ambient_light(0.0, 0.0, 0.0) - 1.0).abs() < 1e-6);
        let midnight = time_of_day(18000);
        assert!((ambient_light(midnight, 0.0, 0.0) - 0.2).abs() < 1e-6);
        assert!((lightmap_sky_factor(0.0, 0.0, 0.0) - 1.0).abs() < 1e-6);
        assert!((lightmap_sky_factor(midnight, 0.0, 0.0) - 0.24).abs() < 1e-6);
        // Rain and thunder multiply the daylight by `1 - 5/16` each before the 0.8/0.2 mix.
        let storm = ambient_light(0.0, 1.0, 1.0);
        assert!((storm - (11.0 / 16.0 * 11.0 / 16.0 * 0.8 + 0.2)).abs() < 1e-6);
    }

    #[test]
    fn the_sun_turns_a_full_circle_a_day() {
        assert_eq!(sun_angle(0.0), 0.0);
        assert_eq!(sun_angle(0.25), 90.0);
        assert_eq!(sun_angle(0.5), 180.0);
        assert_eq!(cloud_height("minecraft:overworld"), 128.0);
        assert_eq!(cloud_height("minecraft:the_end"), 8.0);
    }
}
