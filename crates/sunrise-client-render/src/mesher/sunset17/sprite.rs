//! Atlas sprites of the 1.7.10 block atlas, ported from
//! `net.minecraft.client.render.texture.TextureAtlasSprite`.
//!
//! The era's sprite stores its own `uMin`/`uMax`/`vMin`/`vMax` floats with a
//! 0.01-pixel inset, and `getU`/`getV` map the 0..16 block-face units onto that
//! rectangle with vanilla's exact float operations. The engine's atlas gives
//! every sprite a whole array layer, so the renderer works in that layer's own
//! 0..1 rectangle ([`Sprite::unit`]); the probe uses the oracle's 256x256 cells
//! ([`Sprite::atlas`]) so the UV floats can be compared bit for bit.

#[derive(Clone, Copy, Debug)]
pub struct Sprite {
    /// The block atlas's array layer in the renderer, or the fixture's own id
    /// in the probe.
    pub layer: u16,
    /// `Sprite.getName().equals("grass_side")`: the fancy-graphics side
    /// overlay pass.
    pub grass_side: bool,
    u_min: f32,
    u_max: f32,
    v_min: f32,
    v_max: f32,
}

impl Sprite {
    /// A 16x16 icon at `(x, y)` pixels of an atlas, as
    /// `TextureAtlasSprite.init(atlas_width, atlas_height, x, y, false)`.
    pub fn atlas(layer: u16, x: i32, y: i32, atlas_width: i32, atlas_height: i32) -> Sprite {
        // float var6 = (float)(0.01F / u); the 0.01F widens to double first.
        let inset_u = (0.01_f32 as f64 / atlas_width as f64) as f32;
        let inset_v = (0.01_f32 as f64 / atlas_height as f64) as f32;
        Sprite {
            layer,
            grass_side: false,
            u_min: x as f32 / atlas_width as f32 + inset_u,
            u_max: (x + 16) as f32 / atlas_width as f32 - inset_u,
            v_min: y as f32 / atlas_height as f32 + inset_v,
            v_max: (y + 16) as f32 / atlas_height as f32 - inset_v,
        }
    }

    /// A sprite that fills its own atlas layer: the engine's array texture maps
    /// every layer to 0..1.
    pub fn unit(layer: u16) -> Sprite {
        Sprite {
            layer,
            grass_side: false,
            u_min: 0.0,
            u_max: 1.0,
            v_min: 0.0,
            v_max: 1.0,
        }
    }

    /// This sprite with the `grass_side` marker of `getName()`.
    pub fn marked(mut self, grass_side: bool) -> Sprite {
        self.grass_side = grass_side;
        self
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
        let sprite = Sprite::atlas(0, 0, 0, 256, 256);
        // 0.01F/256 widened to double and narrowed back.
        let inset = (0.01_f32 as f64 / 256.0) as f32;
        assert_eq!(sprite.u_min(), inset);
        assert_eq!(sprite.u_max(), 16.0 / 256.0 - inset);
        assert_eq!(sprite.v_min(), inset);
        assert_eq!(sprite.v_max(), 16.0 / 256.0 - inset);
    }

    #[test]
    fn get_u_and_get_v_follow_the_atlas_rectangle() {
        let sprite = Sprite::atlas(0, 32, 48, 256, 256);
        assert_eq!(sprite.get_u(0.0), sprite.u_min());
        assert_eq!(sprite.get_u(16.0), sprite.u_max());
        assert_eq!(sprite.get_v(0.0), sprite.v_min());
        assert_eq!(sprite.get_v(16.0), sprite.v_max());
        assert_eq!(
            sprite.get_u(8.0),
            sprite.u_min() + (sprite.u_max() - sprite.u_min()) * 8.0 / 16.0
        );
    }

    #[test]
    fn a_unit_sprite_is_the_whole_layer() {
        let sprite = Sprite::unit(7);
        assert_eq!(sprite.layer, 7);
        assert_eq!(sprite.get_u(0.0), 0.0);
        assert_eq!(sprite.get_u(16.0), 1.0);
        assert_eq!(sprite.get_v(4.0), 0.25);
    }
}
