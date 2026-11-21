//! The tessellator, ported from `net.minecraft.client.render.vertex.Tesselator`.
//!
//! Vanilla packs each vertex into one 32-byte row of an int buffer (three
//! position floats, two texture floats, a color int, a normal int and a
//! brightness int); this records the same values into a plain vertex list so
//! the geometry stays checkable without a GPU. Vertex positions and texture
//! coordinates truncate doubles to floats exactly like vanilla.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vertex {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub u: f32,
    pub v: f32,
    /// RGBA, little-endian packed like vanilla's `Tesselator.color` on
    /// little-endian machines: `a << 24 | b << 16 | g << 8 | r`.
    pub color: i32,
    /// Packed sky/block brightness: `sky << 20 | block << 4`, or a mixed
    /// vertex brightness from the smooth-lighting path.
    pub brightness: i32,
}

#[derive(Default)]
pub struct Tesselator {
    pub vertices: Vec<Vertex>,
    u: f32,
    v: f32,
    brightness: i32,
    color: i32,
}

impl Tesselator {
    pub fn new() -> Tesselator {
        Tesselator {
            vertices: Vec::new(),
            u: 0.0,
            v: 0.0,
            brightness: 0,
            color: 0,
        }
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
    }

    pub fn brightness(&mut self, brightness: i32) {
        self.brightness = brightness;
    }

    /// `Tesselator.setColorOpaque_F(float, float, float)`:
    /// `(int)(f * 255.0F)` per channel.
    pub fn color(&mut self, r: f32, g: f32, b: f32) {
        self.color_i32(
            (r * 255.0) as i32,
            (g * 255.0) as i32,
            (b * 255.0) as i32,
            255,
        );
    }

    /// `Tesselator.color(int, int, int, int)`: clamps to 0..255 and packs
    /// little-endian.
    pub fn color_i32(&mut self, r: i32, g: i32, b: i32, a: i32) {
        let r = r.clamp(0, 255);
        let g = g.clamp(0, 255);
        let b = b.clamp(0, 255);
        let a = a.clamp(0, 255);
        self.color = a << 24 | b << 16 | g << 8 | r;
    }

    /// `Tesselator.color(int rgb)`.
    pub fn color_rgb(&mut self, rgb: i32) {
        let r = rgb >> 16 & 0xFF;
        let g = rgb >> 8 & 0xFF;
        let b = rgb & 0xFF;
        self.color_i32(r, g, b, 255);
    }

    pub fn vertex(&mut self, x: f64, y: f64, z: f64, u: f32, v: f32) {
        self.texture(u, v);
        self.vertex_xyz(x, y, z);
    }

    /// `Tesselator.setTextureUV(double, double)`: every RenderBlocks call
    /// site passes floats, which Java widens and the tessellator narrows
    /// back, so the values arrive as floats.
    pub fn texture(&mut self, u: f32, v: f32) {
        self.u = u;
        self.v = v;
    }

    pub fn vertex_xyz(&mut self, x: f64, y: f64, z: f64) {
        self.vertices.push(Vertex {
            x: x as f32,
            y: y as f32,
            z: z as f32,
            u: self.u,
            v: self.v,
            color: self.color,
            brightness: self.brightness,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_clamps_and_packs_little_endian() {
        let mut t = Tesselator::new();
        t.color(2.0, -1.0, 0.5);
        t.vertex_xyz(1.0, 2.0, 3.0);
        let vertex = t.vertices[0];
        assert_eq!(vertex.color & 0xFF, 255);
        assert_eq!(vertex.color >> 16 & 0xFF, 127); // 0.5 * 255 truncated
    }

    #[test]
    fn vertex_truncates_doubles_to_floats() {
        let mut t = Tesselator::new();
        t.brightness(983055);
        t.color(1.0, 1.0, 1.0);
        t.vertex(1.0 / 3.0, -0.5, 16.0, 0.25, 0.75);
        let vertex = t.vertices[0];
        assert_eq!(vertex.x, (1.0f64 / 3.0) as f32);
        assert_eq!(vertex.y, -0.5);
        assert_eq!(vertex.brightness, 983055);
        assert_eq!(vertex.u, 0.25);
    }
}
