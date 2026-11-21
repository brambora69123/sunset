//! Atlas sprites, ported from `net.minecraft.client.render.texture.TextureAtlasSprite`.
//!
//! A sprite is a pixel rectangle inside one atlas. The 1.7.10 sprite stores
//! its own `uMin`/`uMax`/`vMin`/`vMax` floats with a 0.01-pixel inset, and
//! `getU`/`getV` map the 0..16 block-face units onto that rectangle with
//! vanilla's exact float operations.

#[derive(Clone, Copy, Debug)]
pub struct Sprite {
    pub name: &'static str,
    /// Pixel rectangle inside the atlas.
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub atlas_width: i32,
    pub atlas_height: i32,
    u_min: f32,
    u_max: f32,
    v_min: f32,
    v_max: f32,
}

impl Sprite {
    /// A 16x16 icon at `(x, y)` pixels of an atlas, as
    /// `TextureAtlasSprite.init(atlas_width, atlas_height, x, y, false)`.
    pub fn atlas(
        name: &'static str,
        x: i32,
        y: i32,
        atlas_width: i32,
        atlas_height: i32,
    ) -> Sprite {
        // float var6 = (float)(0.01F / u); the 0.01F widens to double first.
        let inset_u = (0.01_f32 as f64 / atlas_width as f64) as f32;
        let inset_v = (0.01_f32 as f64 / atlas_height as f64) as f32;
        Sprite {
            name,
            x,
            y,
            width: 16,
            height: 16,
            atlas_width,
            atlas_height,
            u_min: x as f32 / atlas_width as f32 + inset_u,
            u_max: (x + 16) as f32 / atlas_width as f32 - inset_u,
            v_min: y as f32 / atlas_height as f32 + inset_v,
            v_max: (y + 16) as f32 / atlas_height as f32 - inset_v,
        }
    }

    /// A synthetic full-UV sprite (0..1, no atlas inset) for tests that do
    /// not care about the atlas rectangle.
    pub const fn unit(name: &'static str) -> Sprite {
        Sprite {
            name,
            x: 0,
            y: 0,
            width: 16,
            height: 16,
            atlas_width: 16,
            atlas_height: 16,
            u_min: 0.0,
            u_max: 1.0,
            v_min: 0.0,
            v_max: 1.0,
        }
    }

    /// A sprite at an atlas grid position of 16x16 icons.
    pub const fn icon(
        name: &'static str,
        column: i32,
        row: i32,
        atlas_columns: i32,
        atlas_rows: i32,
    ) -> Sprite {
        Sprite {
            name,
            x: column * 16,
            y: row * 16,
            width: 16,
            height: 16,
            atlas_width: atlas_columns * 16,
            atlas_height: atlas_rows * 16,
            u_min: 0.0,
            u_max: 1.0,
            v_min: 0.0,
            v_max: 1.0,
        }
    }

    pub fn u_min(&self) -> f32 {
        self.u_min
    }

    pub fn u_max(&self) -> f32 {
        self.u_max
    }

    pub fn v_min(&self) -> f32 {
        self.v_min
    }

    pub fn v_max(&self) -> f32 {
        self.v_max
    }

    /// `Sprite.getU(delta)`: `uMin + (uMax - uMin) * (float)delta / 16.0F`.
    pub fn get_u(&self, delta: f64) -> f32 {
        let span = self.u_max - self.u_min;
        self.u_min + span * (delta as f32) / 16.0
    }

    /// `Sprite.getV(delta)`: `vMin + (vMax - vMin) * ((float)delta / 16.0F)`.
    pub fn get_v(&self, delta: f64) -> f32 {
        let span = self.v_max - self.v_min;
        self.v_min + span * ((delta as f32) / 16.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atlas_sprite_has_vanilla_insets() {
        let sprite = Sprite::atlas("stone", 0, 0, 256, 256);
        // 0.01F/256 widened to double and narrowed back.
        let inset = (0.01_f32 as f64 / 256.0) as f32;
        assert_eq!(sprite.u_min(), inset);
        assert_eq!(sprite.u_max(), 16.0 / 256.0 - inset);
        assert_eq!(sprite.v_min(), inset);
        assert_eq!(sprite.v_max(), 16.0 / 256.0 - inset);
    }

    #[test]
    fn get_u_and_get_v_follow_the_atlas_rectangle() {
        let sprite = Sprite::atlas("stone", 32, 48, 256, 256);
        assert_eq!(sprite.get_u(0.0), sprite.u_min());
        assert_eq!(sprite.get_u(16.0), sprite.u_max());
        assert_eq!(sprite.get_v(0.0), sprite.v_min());
        assert_eq!(sprite.get_v(16.0), sprite.v_max());
        assert_eq!(
            sprite.get_u(8.0),
            sprite.u_min() + (sprite.u_max() - sprite.u_min()) * 8.0 / 16.0
        );
    }
}
