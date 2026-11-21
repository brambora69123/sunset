//! The block renderer, ported from `net.minecraft.client.render.block.BlockRenderer`
//! (vanilla 1.7.10 `RenderBlocks`).
//!
//! Covers the render-type dispatch, the standard cube path with per-face
//! textures, tint and the three lighting modes (off, smooth, weighted), the
//! six face tessellators with UV rotation, axis blocks (logs), glass, torches,
//! crossed squares (plants), crops, liquids, stairs, fences, cacti, lily pads
//! and snow layers. Its block facts, sprites and light come from the era's own
//! tables ([`super::State17`], `sunset17_assets`, [`super::Colormaps`]).
//!
//! The vertex stream is checked against the real 1.7.10 client by the engine's
//! `sunset17_render_probe` bin and `tools/javaoracle_1_7_10/RenderOracle.java`
//! (bit for bit), and against the reference port `sunset-1-7-render`'s
//! `render_probe`; see the bin's module docs for the diff command.

use super::blocks::{
    BlockDef, Bounds, CrossKind, FACE_BOTTOM, FACE_EAST, FACE_NORTH, FACE_SOUTH, FACE_TOP,
    FACE_WEST, Kind, opposite_offset,
};
use super::material::Material;
use super::sprite::Sprite;
use super::stairs;
use super::tesselator::Tesselator;
use super::world::WorldView;

/// Client options the renderer reads: `Minecraft.gameSettings`.
#[derive(Clone, Copy, Debug, Default)]
pub struct RenderOptions {
    /// `ambientOcclusion`: 0 off, 1 smooth (vanilla 1.7.10), 2 weighted.
    pub ao: i32,
    /// `fancyGraphics`.
    pub fancy_graphics: bool,
    /// `GameRenderer.anaglyph`.
    pub anaglyph: bool,
}

/// `RenderBlocks`. One instance renders one world.
pub struct RenderBlocks<'w, W: WorldView> {
    pub world: &'w W,
    pub options: RenderOptions,
    pub tesselator: Tesselator,
    // Prepared bounds of the current render pass.
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
    min_z: f64,
    max_z: f64,
    render_flipped: bool,
    /// `f_45954551`: flips the U of the north and east faces (stair inner
    /// corners and fence rails set it).
    mirror_uv: bool,
    flip_depth: bool,
    render_face_always: bool,
    forced_sprite: Option<Sprite>,
    /// Per-face UV rotation (bottom, top, north, south, west, east).
    face_rotations: [i32; 6],
    /// `maxAmbientOcclusion`: the weighted smooth-lighting variant.
    max_ao: bool,
    /// `ambientOcclusion`: the current pass carries per-vertex light.
    ambient_occlusion: bool,
    // Per-vertex light of the current face (vertex1..4).
    vertex_brightness: [i32; 4],
    vertex_color: [[f32; 3]; 4],
}

impl<'w, W: WorldView> RenderBlocks<'w, W> {
    pub fn new(world: &'w W, options: RenderOptions) -> RenderBlocks<'w, W> {
        RenderBlocks {
            world,
            options,
            tesselator: Tesselator::new(),
            min_x: 0.0,
            max_x: 1.0,
            min_y: 0.0,
            max_y: 1.0,
            min_z: 0.0,
            max_z: 1.0,
            render_flipped: false,
            mirror_uv: false,
            flip_depth: false,
            render_face_always: false,
            forced_sprite: None,
            face_rotations: [0; 6],
            max_ao: false,
            ambient_occlusion: false,
            vertex_brightness: [0; 4],
            vertex_color: [[0.0; 3]; 4],
        }
    }

    pub fn set_forced_sprite(&mut self, sprite: Sprite) {
        self.forced_sprite = Some(sprite);
    }

    pub fn clear_forced_sprite(&mut self) {
        self.forced_sprite = None;
    }

    pub fn has_forced_sprite(&self) -> bool {
        self.forced_sprite.is_some()
    }

    /// `RenderBlocks.renderMiningProgress`.
    pub fn render_mining_progress(
        &mut self,
        block: &BlockDef,
        x: i32,
        y: i32,
        z: i32,
        sprite: Sprite,
    ) {
        self.set_forced_sprite(sprite);
        self.tesselate_in_world(block, x, y, z);
        self.clear_forced_sprite();
    }

    /// `RenderBlocks.renderMovingBlock`.
    pub fn render_moving_block(&mut self, block: &BlockDef, x: i32, y: i32, z: i32) {
        self.render_face_always = true;
        self.tesselate_in_world(block, x, y, z);
        self.render_face_always = false;
    }

    fn prepare_bounds(&mut self, bounds: Bounds) {
        self.min_x = bounds.min_x;
        self.max_x = bounds.max_x;
        self.min_y = bounds.min_y;
        self.max_y = bounds.max_y;
        self.min_z = bounds.min_z;
        self.max_z = bounds.max_z;
        self.max_ao = self.options.ao >= 2 && !bounds.is_full_cube();
    }

    /// `prepare(Block)`: the shape `updateShape` set.
    fn prepare_block(&mut self, block: &BlockDef, x: i32, y: i32, z: i32) {
        let meta = self.world.metadata(x, y, z) as u8;
        let bounds = block.shape(meta, self.world, x, y, z);
        self.prepare_bounds(bounds);
    }

    /// `getSprite(block, world, x, y, z, face)`: the state's per-face sprite.
    fn sprite_for(&self, _block: &BlockDef, x: i32, y: i32, z: i32, face: i32) -> Sprite {
        self.world.state(x, y, z).sprites[face as usize]
    }

    /// The sprite the fancy-graphics side overlay pass draws (`grass_side_overlay`).
    fn overlay_sprite(&self, x: i32, y: i32, z: i32) -> Option<Sprite> {
        self.world.state(x, y, z).overlay
    }

    /// The face cull of `tesselateInWorld`: the block rule, the glass and
    /// liquid overrides, or the slab rule for single slabs.
    fn should_render_face(&self, block: &BlockDef, x: i32, y: i32, z: i32, face: i32) -> bool {
        let (ox, oy, oz) = opposite_offset(face);
        let fancy = self.options.fancy_graphics;
        let bounds = Bounds::new(
            self.min_x, self.min_y, self.min_z, self.max_x, self.max_y, self.max_z,
        );
        match block.kind {
            Kind::Slab { .. } => block.slab_should_render_face(self.world, x, y, z, face),
            Kind::Glass => {
                // `TransparentBlock.shouldRenderFace`: the neighbour's
                // metadata against the glass's own decides first, then
                // same-block glass never draws the shared face.
                let own = self.world.state(x + ox, y + oy, z + oz);
                if self.world.metadata(x, y, z) as u8 != own.meta {
                    return true;
                }
                if self.world.block(x, y, z).key == block.key {
                    return false;
                }
                block.should_render_face(self.world, x, y, z, face, &bounds, fancy)
            }
            Kind::Ice => {
                // `TransparentBlock.shouldRenderFace` with ice's `culling`
                // off (two ice blocks hide their shared face), then
                // `IceBlock`'s mirrored face id for the default rule.
                if self.world.block(x, y, z).key == block.key {
                    return false;
                }
                let mirrored = 1 - face;
                block.should_render_face(self.world, x, y, z, mirrored, &bounds, fancy)
            }
            Kind::Leaves => {
                // `TranslucentBlock.shouldRenderFace`: with fast graphics the
                // shared face of two leaves is never drawn.
                let neighbour = self.world.block(x, y, z);
                if !fancy && neighbour.key == block.key {
                    return false;
                }
                block.should_render_face(self.world, x, y, z, face, &bounds, fancy)
            }
            Kind::Liquid(_) => {
                // `LiquidBlock.shouldRenderFace`: a face between two
                // volumes of the same liquid is never drawn, and the top
                // face always is.
                if self.world.block(x, y, z).material == block.material {
                    return false;
                }
                if face == FACE_TOP {
                    return true;
                }
                block.should_render_face(self.world, x, y, z, face, &bounds, fancy)
            }
            _ => block.should_render_face(self.world, x, y, z, face, &bounds, fancy),
        }
    }

    /// `tesselateInWorld` (`renderBlockByRenderType`).
    pub fn tesselate_in_world(&mut self, block: &BlockDef, x: i32, y: i32, z: i32) -> bool {
        if block.kind == Kind::Air {
            return false;
        }
        self.prepare_block(block, x, y, z);
        match block.kind {
            Kind::Log => self.tesselate_axis(block, x, y, z),
            Kind::Quartz => self.tesselate_quartz(block, x, y, z),
            Kind::Cross(_) => self.tesselate_cross(block, x, y, z),
            Kind::Plant => self.tesselate_plant(block, x, y, z),
            Kind::Torch => self.tesselate_torch(block, x, y, z),
            Kind::Liquid(_) => self.tesselate_liquid(block, x, y, z),
            Kind::Stairs => self.tesselate_stairs(block, x, y, z),
            Kind::Fence => self.tesselate_fence(block, x, y, z),
            Kind::Cactus => self.tesselate_cactus(block, x, y, z),
            Kind::LilyPad => self.tesselate_lily_pad(block, x, y, z),
            _ => self.tesselate_block(block, x, y, z),
        }
    }

    /// `tesselateAxisBlock`: axis block UV rotation from metadata bit 4 and 8.
    fn tesselate_axis(&mut self, block: &BlockDef, x: i32, y: i32, z: i32) -> bool {
        let metadata = self.world.metadata(x, y, z);
        if metadata & 12 == 4 {
            self.face_rotations[FACE_NORTH as usize] = 1;
            self.face_rotations[FACE_SOUTH as usize] = 1;
            self.face_rotations[FACE_TOP as usize] = 1;
            self.face_rotations[FACE_BOTTOM as usize] = 1;
        } else if metadata & 12 == 8 {
            self.face_rotations[FACE_WEST as usize] = 1;
            self.face_rotations[FACE_EAST as usize] = 1;
        }
        let rendered = self.tesselate_block(block, x, y, z);
        self.face_rotations = [0; 6];
        rendered
    }

    /// `tesselateQuartz`: the same rotation table on metadata 3 and 4.
    fn tesselate_quartz(&mut self, block: &BlockDef, x: i32, y: i32, z: i32) -> bool {
        let metadata = self.world.metadata(x, y, z);
        if metadata == 3 {
            self.face_rotations[FACE_NORTH as usize] = 1;
            self.face_rotations[FACE_SOUTH as usize] = 1;
            self.face_rotations[FACE_TOP as usize] = 1;
            self.face_rotations[FACE_BOTTOM as usize] = 1;
        } else if metadata == 4 {
            self.face_rotations[FACE_WEST as usize] = 1;
            self.face_rotations[FACE_EAST as usize] = 1;
        }
        let rendered = self.tesselate_block(block, x, y, z);
        self.face_rotations = [0; 6];
        rendered
    }

    /// `tesselateBlock`: the color multiplier, then the lighting mode.
    fn tesselate_block(&mut self, block: &BlockDef, x: i32, y: i32, z: i32) -> bool {
        let color = block.color(self.world.metadata(x, y, z) as u8, self.world, x, y, z);
        let mut r = (color >> 16 & 0xFF) as f32 / 255.0;
        let mut g = (color >> 8 & 0xFF) as f32 / 255.0;
        let mut b = (color & 0xFF) as f32 / 255.0;
        if self.options.anaglyph {
            let gray = (r * 30.0 + g * 59.0 + b * 11.0) / 100.0;
            let red = (r * 30.0 + g * 70.0) / 100.0;
            let blue = (r * 30.0 + b * 70.0) / 100.0;
            r = gray;
            g = red;
            b = blue;
        }

        if self.options.ao == 0 || self.world.state(x, y, z).light != 0 {
            self.tesselate_without_ao(block, x, y, z, r, g, b)
        } else {
            self.tesselate_with_ao(block, x, y, z, r, g, b, self.max_ao)
        }
    }

    /// The light color and AO light of one neighbor position. Vanilla calls
    /// the *rendered* block's `getLightColor` for every sample, so liquid
    /// and slab overrides apply at the neighbors too; the AO light always
    /// comes from the neighbor block itself.
    fn neighbor_light(&self, block: &BlockDef, x: i32, y: i32, z: i32) -> (i32, f32) {
        (
            block.light_color(self.world, x, y, z),
            self.world.block(x, y, z).ambient_occlusion_light(),
        )
    }

    /// `block.getLightColor(world, x, y, z)` at a neighbor position.
    fn neighbor_light_color(&self, block: &BlockDef, x: i32, y: i32, z: i32) -> i32 {
        block.light_color(self.world, x, y, z)
    }

    fn neighbor_translucent(&self, x: i32, y: i32, z: i32) -> bool {
        self.world.block(x, y, z).translucent
    }

    fn neighbor_ao(&self, x: i32, y: i32, z: i32) -> f32 {
        self.world.block(x, y, z).ambient_occlusion_light()
    }

    /// `tesselateWithoutAmbientOcclusion` (`renderStandardBlockWithColorMultiplier`):
    /// one light value and one color per face.
    fn tesselate_without_ao(
        &mut self,
        block: &BlockDef,
        x: i32,
        y: i32,
        z: i32,
        r: f32,
        g: f32,
        b: f32,
    ) -> bool {
        self.ambient_occlusion = false;
        let light = block.light_color(self.world, x, y, z);
        let mut rendered = false;
        if self.render_face_always || self.should_render_face(block, x, y - 1, z, FACE_BOTTOM) {
            self.without_ao_bottom(block, x, y, z, light, r, g, b);
            rendered = true;
        }
        if self.render_face_always || self.should_render_face(block, x, y + 1, z, FACE_TOP) {
            self.without_ao_top(block, x, y, z, light, r, g, b);
            rendered = true;
        }
        if self.render_face_always || self.should_render_face(block, x, y, z - 1, FACE_NORTH) {
            self.without_ao_side(block, x, y, z, FACE_NORTH, light, r, g, b);
            rendered = true;
        }
        if self.render_face_always || self.should_render_face(block, x, y, z + 1, FACE_SOUTH) {
            self.without_ao_side(block, x, y, z, FACE_SOUTH, light, r, g, b);
            rendered = true;
        }
        if self.render_face_always || self.should_render_face(block, x - 1, y, z, FACE_WEST) {
            self.without_ao_side(block, x, y, z, FACE_WEST, light, r, g, b);
            rendered = true;
        }
        if self.render_face_always || self.should_render_face(block, x + 1, y, z, FACE_EAST) {
            self.without_ao_side(block, x, y, z, FACE_EAST, light, r, g, b);
            rendered = true;
        }
        rendered
    }

    /// The `0.5` bottom face of the flat-lit path. Grass keeps its tint out
    /// of the faces; every other block scales the face color by it.
    fn without_ao_bottom(
        &mut self,
        block: &BlockDef,
        x: i32,
        y: i32,
        z: i32,
        light: i32,
        r: f32,
        g: f32,
        b: f32,
    ) {
        let face_light = if self.min_y > 0.0 {
            light
        } else {
            self.neighbor_light_color(block, x, y - 1, z)
        };
        let (scale_r, scale_g, scale_b) = Self::flat_face_scale(block, 0.5, r, g, b);
        self.tesselator.brightness(face_light);
        self.tesselator.color(scale_r, scale_g, scale_b);
        let sprite = self.sprite_for(block, x, y, z, FACE_BOTTOM);
        self.tesselate_bottom_face(x as f64, y as f64, z as f64, sprite);
    }

    /// The fully lit top face of the flat-lit path.
    fn without_ao_top(
        &mut self,
        block: &BlockDef,
        x: i32,
        y: i32,
        z: i32,
        light: i32,
        r: f32,
        g: f32,
        b: f32,
    ) {
        let face_light = if self.max_y < 1.0 {
            light
        } else {
            self.neighbor_light_color(block, x, y + 1, z)
        };
        self.tesselator.brightness(face_light);
        self.tesselator.color(r, g, b);
        let sprite = self.sprite_for(block, x, y, z, FACE_TOP);
        self.tesselate_top_face(x as f64, y as f64, z as f64, sprite);
    }

    /// A `0.8` (north/south) or `0.6` (west/east) side face of the flat-lit
    /// path, plus the grass side overlay pass.
    fn without_ao_side(
        &mut self,
        block: &BlockDef,
        x: i32,
        y: i32,
        z: i32,
        face: i32,
        light: i32,
        r: f32,
        g: f32,
        b: f32,
    ) {
        let (dx, dy, dz) = face_offset(face);
        let face_light = if self.flat_face_uses_own_light(face) {
            light
        } else {
            self.neighbor_light_color(block, x + dx, y + dy, z + dz)
        };
        let base = if face == FACE_NORTH || face == FACE_SOUTH {
            0.8
        } else {
            0.6
        };
        let (scale_r, scale_g, scale_b) = Self::flat_face_scale(block, base, r, g, b);
        self.tesselator.brightness(face_light);
        self.tesselator.color(scale_r, scale_g, scale_b);
        let sprite = self.sprite_for(block, x, y, z, face);
        self.tesselate_face(face, x as f64, y as f64, z as f64, sprite);
        if self.options.fancy_graphics
            && sprite.grass_side
            && !self.has_forced_sprite()
            && let Some(overlay) = self.overlay_sprite(x, y, z)
        {
            self.tesselator.color(scale_r * r, scale_g * g, scale_b * b);
            self.tesselate_face(face, x as f64, y as f64, z as f64, overlay);
        }
    }

    /// Whether the flat-lit face takes the block's own light instead of the
    /// neighbour's, vanilla's `minX > 0.0` style checks.
    fn flat_face_uses_own_light(&self, face: i32) -> bool {
        match face {
            FACE_BOTTOM => self.min_y > 0.0,
            FACE_TOP => self.max_y < 1.0,
            FACE_NORTH => self.min_z > 0.0,
            FACE_SOUTH => self.max_z < 1.0,
            FACE_WEST => self.min_x > 0.0,
            _ => self.max_x < 1.0,
        }
    }

    /// The face color of the flat-lit path: grass keeps its tint out of the
    /// faces, everything else multiplies the base shade by the tint.
    fn flat_face_scale(block: &BlockDef, base: f32, r: f32, g: f32, b: f32) -> (f32, f32, f32) {
        if block.kind == Kind::Grass {
            (base, base, base)
        } else {
            (base * r, base * g, base * b)
        }
    }

    /// `tesselateNorthFace` and friends, by face id.
    fn tesselate_face(&mut self, face: i32, x: f64, y: f64, z: f64, sprite: Sprite) {
        match face {
            FACE_BOTTOM => self.tesselate_bottom_face(x, y, z, sprite),
            FACE_TOP => self.tesselate_top_face(x, y, z, sprite),
            FACE_NORTH => self.tesselate_north_face(x, y, z, sprite),
            FACE_SOUTH => self.tesselate_south_face(x, y, z, sprite),
            FACE_WEST => self.tesselate_west_face(x, y, z, sprite),
            _ => self.tesselate_east_face(x, y, z, sprite),
        }
    }

    /// `interpolateLightColor`: the average of three neighbor light colors
    /// and the face's own, with zero colors replaced by the center.
    fn interpolate_light_color(color1: i32, color2: i32, color3: i32, center: i32) -> i32 {
        let color1 = if color1 == 0 { center } else { color1 };
        let color2 = if color2 == 0 { center } else { color2 };
        let color3 = if color3 == 0 { center } else { color3 };
        color1
            .wrapping_add(color2)
            .wrapping_add(color3)
            .wrapping_add(center)
            >> 2
            & 0xFF00FF
    }

    /// `getVertexBrightness`: the weighted mix of four light colors, per
    /// channel of the packed `& 0xFF00FF` form.
    fn get_vertex_brightness(
        corner1: i32,
        corner2: i32,
        corner3: i32,
        corner4: i32,
        scale1: f64,
        scale2: f64,
        scale3: f64,
        scale4: f64,
    ) -> i32 {
        let red = ((corner1 >> 16 & 0xFF) as f64 * scale1
            + (corner2 >> 16 & 0xFF) as f64 * scale2
            + (corner3 >> 16 & 0xFF) as f64 * scale3
            + (corner4 >> 16 & 0xFF) as f64 * scale4) as i32
            & 0xFF;
        let blue = ((corner1 & 0xFF) as f64 * scale1
            + (corner2 & 0xFF) as f64 * scale2
            + (corner3 & 0xFF) as f64 * scale3
            + (corner4 & 0xFF) as f64 * scale4) as i32
            & 0xFF;
        red << 16 | blue
    }

    /// `tesselateWithMaxAmbientOcclusion` and `tesselateWithMinAmbientOcclusion`:
    /// smooth lighting per vertex. `max` picks the weighted variant; the two
    /// share the neighbour gathering and the bottom/top faces, and differ in
    /// the per-vertex light mix of the side faces.
    fn tesselate_with_ao(
        &mut self,
        block: &BlockDef,
        x: i32,
        y: i32,
        z: i32,
        r: f32,
        g: f32,
        b: f32,
        max: bool,
    ) -> bool {
        self.ambient_occlusion = true;
        self.tesselator.brightness(983055);
        let light = block.light_color(self.world, x, y, z);
        let tinted = !block.top_is_grass_top && !self.has_forced_sprite();
        let mut rendered = false;
        if self.render_face_always || self.should_render_face(block, x, y - 1, z, FACE_BOTTOM) {
            self.ao_bottom(block, x, y, z, light, tinted, r, g, b);
            rendered = true;
        }
        if self.render_face_always || self.should_render_face(block, x, y + 1, z, FACE_TOP) {
            self.ao_top(block, x, y, z, light, r, g, b);
            rendered = true;
        }
        if self.render_face_always || self.should_render_face(block, x, y, z - 1, FACE_NORTH) {
            self.ao_north(block, x, y, z, light, tinted, r, g, b, max);
            rendered = true;
        }
        if self.render_face_always || self.should_render_face(block, x, y, z + 1, FACE_SOUTH) {
            self.ao_south(block, x, y, z, light, tinted, r, g, b, max);
            rendered = true;
        }
        if self.render_face_always || self.should_render_face(block, x - 1, y, z, FACE_WEST) {
            self.ao_west(block, x, y, z, light, tinted, r, g, b, max);
            rendered = true;
        }
        if self.render_face_always || self.should_render_face(block, x + 1, y, z, FACE_EAST) {
            self.ao_east(block, x, y, z, light, tinted, r, g, b, max);
            rendered = true;
        }
        self.ambient_occlusion = false;
        rendered
    }

    /// The `bottom` face of both smooth-lit variants.
    fn ao_bottom(
        &mut self,
        block: &BlockDef,
        x: i32,
        y: i32,
        z: i32,
        light: i32,
        tinted: bool,
        r: f32,
        g: f32,
        b: f32,
    ) {
        let mut yy = y;
        if self.min_y <= 0.0 {
            yy -= 1;
        }
        let west = self.neighbor_light(block, x - 1, yy, z);
        let north = self.neighbor_light(block, x, yy, z - 1);
        let south = self.neighbor_light(block, x, yy, z + 1);
        let east = self.neighbor_light(block, x + 1, yy, z);
        let east_translucent = self.neighbor_translucent(x + 1, yy - 1, z);
        let west_translucent = self.neighbor_translucent(x - 1, yy - 1, z);
        let south_translucent = self.neighbor_translucent(x, yy - 1, z + 1);
        let north_translucent = self.neighbor_translucent(x, yy - 1, z - 1);
        let nw = if !north_translucent && !west_translucent {
            west
        } else {
            self.neighbor_light(block, x - 1, yy, z - 1)
        };
        let sw = if !south_translucent && !west_translucent {
            west
        } else {
            self.neighbor_light(block, x - 1, yy, z + 1)
        };
        let ne = if !north_translucent && !east_translucent {
            east
        } else {
            self.neighbor_light(block, x + 1, yy, z - 1)
        };
        let se = if !south_translucent && !east_translucent {
            east
        } else {
            self.neighbor_light(block, x + 1, yy, z + 1)
        };
        if self.min_y <= 0.0 {
            yy += 1;
        }
        let mut center = light;
        if self.min_y <= 0.0
            || !self
                .world
                .block(x, yy - 1, z)
                .is_solid_render(self.options.fancy_graphics)
        {
            center = self.neighbor_light_color(block, x, yy - 1, z);
        }
        let face_ao = self.neighbor_ao(x, yy - 1, z);
        // Vertex order: SW, NW, NE, SE.
        let ao = [
            (sw.1 + west.1 + south.1 + face_ao) / 4.0,
            (west.1 + nw.1 + face_ao + north.1) / 4.0,
            (face_ao + north.1 + east.1 + ne.1) / 4.0,
            (south.1 + face_ao + se.1 + east.1) / 4.0,
        ];
        self.vertex_brightness = [
            Self::interpolate_light_color(sw.0, west.0, south.0, center),
            Self::interpolate_light_color(west.0, nw.0, north.0, center),
            Self::interpolate_light_color(north.0, east.0, ne.0, center),
            Self::interpolate_light_color(south.0, se.0, east.0, center),
        ];
        let base = if tinted { r * 0.5 } else { 0.5 };
        let base_g = if tinted { g * 0.5 } else { 0.5 };
        let base_b = if tinted { b * 0.5 } else { 0.5 };
        self.vertex_color = [
            [base * ao[0], base_g * ao[0], base_b * ao[0]],
            [base * ao[1], base_g * ao[1], base_b * ao[1]],
            [base * ao[2], base_g * ao[2], base_b * ao[2]],
            [base * ao[3], base_g * ao[3], base_b * ao[3]],
        ];
        self.tesselate_bottom_face(
            x as f64,
            y as f64,
            z as f64,
            self.sprite_for(block, x, y, z, FACE_BOTTOM),
        );
    }

    /// The `top` face of both smooth-lit variants.
    fn ao_top(
        &mut self,
        block: &BlockDef,
        x: i32,
        y: i32,
        z: i32,
        light: i32,
        r: f32,
        g: f32,
        b: f32,
    ) {
        let mut yy = y;
        if self.max_y >= 1.0 {
            yy += 1;
        }
        let west = self.neighbor_light(block, x - 1, yy, z);
        let east = self.neighbor_light(block, x + 1, yy, z);
        let north = self.neighbor_light(block, x, yy, z - 1);
        let south = self.neighbor_light(block, x, yy, z + 1);
        let east_translucent = self.neighbor_translucent(x + 1, yy + 1, z);
        let west_translucent = self.neighbor_translucent(x - 1, yy + 1, z);
        let south_translucent = self.neighbor_translucent(x, yy + 1, z + 1);
        let north_translucent = self.neighbor_translucent(x, yy + 1, z - 1);
        let nw = if !north_translucent && !west_translucent {
            west
        } else {
            self.neighbor_light(block, x - 1, yy, z - 1)
        };
        let ne = if !north_translucent && !east_translucent {
            east
        } else {
            self.neighbor_light(block, x + 1, yy, z - 1)
        };
        let sw = if !south_translucent && !west_translucent {
            west
        } else {
            self.neighbor_light(block, x - 1, yy, z + 1)
        };
        let se = if !south_translucent && !east_translucent {
            east
        } else {
            self.neighbor_light(block, x + 1, yy, z + 1)
        };
        if self.max_y >= 1.0 {
            yy -= 1;
        }
        let mut center = light;
        if self.max_y >= 1.0
            || !self
                .world
                .block(x, yy + 1, z)
                .is_solid_render(self.options.fancy_graphics)
        {
            center = self.neighbor_light_color(block, x, yy + 1, z);
        }
        let face_ao = self.neighbor_ao(x, yy + 1, z);
        // Vertex order: SE, NE, NW, SW.
        let ao = [
            (south.1 + face_ao + se.1 + east.1) / 4.0,
            (face_ao + north.1 + east.1 + ne.1) / 4.0,
            (west.1 + nw.1 + face_ao + north.1) / 4.0,
            (sw.1 + west.1 + south.1 + face_ao) / 4.0,
        ];
        self.vertex_brightness = [
            Self::interpolate_light_color(south.0, se.0, east.0, center),
            Self::interpolate_light_color(north.0, east.0, ne.0, center),
            Self::interpolate_light_color(west.0, nw.0, north.0, center),
            Self::interpolate_light_color(sw.0, west.0, south.0, center),
        ];
        self.vertex_color = [
            [r * ao[0], g * ao[0], b * ao[0]],
            [r * ao[1], g * ao[1], b * ao[1]],
            [r * ao[2], g * ao[2], b * ao[2]],
            [r * ao[3], g * ao[3], b * ao[3]],
        ];
        self.tesselate_top_face(
            x as f64,
            y as f64,
            z as f64,
            self.sprite_for(block, x, y, z, FACE_TOP),
        );
    }

    /// The `north` face of both smooth-lit variants.
    fn ao_north(
        &mut self,
        block: &BlockDef,
        x: i32,
        y: i32,
        z: i32,
        light: i32,
        tinted: bool,
        r: f32,
        g: f32,
        b: f32,
        max: bool,
    ) {
        let mut zz = z;
        if self.min_z <= 0.0 {
            zz -= 1;
        }
        let west = self.neighbor_light(block, x - 1, y, zz);
        let bottom = self.neighbor_light(block, x, y - 1, zz);
        let top = self.neighbor_light(block, x, y + 1, zz);
        let east = self.neighbor_light(block, x + 1, y, zz);
        let east_translucent = self.neighbor_translucent(x + 1, y, zz - 1);
        let west_translucent = self.neighbor_translucent(x - 1, y, zz - 1);
        let top_translucent = self.neighbor_translucent(x, y + 1, zz - 1);
        let bottom_translucent = self.neighbor_translucent(x, y - 1, zz - 1);
        // Corner order NW, NE, SE, SW with their fallbacks; NW/NE read
        // the top corners, SE/SW the bottom ones.
        let nw = if !west_translucent && !top_translucent {
            west
        } else {
            self.neighbor_light(block, x - 1, y + 1, zz)
        };
        let ne = if !east_translucent && !top_translucent {
            east
        } else {
            self.neighbor_light(block, x + 1, y + 1, zz)
        };
        let se = if !east_translucent && !bottom_translucent {
            east
        } else {
            self.neighbor_light(block, x + 1, y - 1, zz)
        };
        let sw = if !west_translucent && !bottom_translucent {
            west
        } else {
            self.neighbor_light(block, x - 1, y - 1, zz)
        };
        if self.min_z <= 0.0 {
            zz += 1;
        }
        let mut center = light;
        if self.min_z <= 0.0
            || !self
                .world
                .block(x, y, zz - 1)
                .is_solid_render(self.options.fancy_graphics)
        {
            center = self.neighbor_light_color(block, x, y, zz - 1);
        }
        let face_ao = self.neighbor_ao(x, y, zz - 1);
        // Canonical corner order NW, NE, SE, SW.
        let corners_ao = [
            (west.1 + nw.1 + face_ao + top.1) / 4.0,
            (face_ao + top.1 + east.1 + ne.1) / 4.0,
            (bottom.1 + face_ao + se.1 + east.1) / 4.0,
            (sw.1 + west.1 + bottom.1 + face_ao) / 4.0,
        ];
        let corners_color = [
            Self::interpolate_light_color(west.0, nw.0, top.0, center),
            Self::interpolate_light_color(top.0, east.0, ne.0, center),
            Self::interpolate_light_color(bottom.0, se.0, east.0, center),
            Self::interpolate_light_color(sw.0, west.0, bottom.0, center),
        ];
        // Vertex order NW, NE, SE, SW; weights per vertex.
        let weights = [
            [
                self.max_y * (1.0 - self.min_x),
                self.max_y * self.min_x,
                (1.0 - self.max_y) * self.min_x,
                (1.0 - self.max_y) * (1.0 - self.min_x),
            ],
            [
                self.max_y * (1.0 - self.max_x),
                self.max_y * self.max_x,
                (1.0 - self.max_y) * self.max_x,
                (1.0 - self.max_y) * (1.0 - self.max_x),
            ],
            [
                self.min_y * (1.0 - self.max_x),
                self.min_y * self.max_x,
                (1.0 - self.min_y) * self.max_x,
                (1.0 - self.min_y) * (1.0 - self.max_x),
            ],
            [
                self.min_y * (1.0 - self.min_x),
                self.min_y * self.min_x,
                (1.0 - self.min_y) * self.min_x,
                (1.0 - self.min_y) * (1.0 - self.min_x),
            ],
        ];
        self.set_side_vertex_light(
            &corners_ao,
            &corners_color,
            &weights,
            [0, 1, 2, 3],
            max,
            tinted,
            r,
            g,
            b,
            0.8,
        );
        let sprite = self.sprite_for(block, x, y, z, FACE_NORTH);
        self.tesselate_north_face(x as f64, y as f64, z as f64, sprite);
        if self.options.fancy_graphics && sprite.grass_side && !self.has_forced_sprite() {
            for vertex in &mut self.vertex_color {
                vertex[0] *= r;
                vertex[1] *= g;
                vertex[2] *= b;
            }
            if let Some(overlay) = self.overlay_sprite(x, y, z) {
                self.tesselate_north_face(x as f64, y as f64, z as f64, overlay);
            }
        }
    }

    /// The `south` face of both smooth-lit variants.
    fn ao_south(
        &mut self,
        block: &BlockDef,
        x: i32,
        y: i32,
        z: i32,
        light: i32,
        tinted: bool,
        r: f32,
        g: f32,
        b: f32,
        max: bool,
    ) {
        let mut zz = z;
        if self.max_z >= 1.0 {
            zz += 1;
        }
        let west = self.neighbor_light(block, x - 1, y, zz);
        let east = self.neighbor_light(block, x + 1, y, zz);
        let bottom = self.neighbor_light(block, x, y - 1, zz);
        let top = self.neighbor_light(block, x, y + 1, zz);
        let east_translucent = self.neighbor_translucent(x + 1, y, zz + 1);
        let west_translucent = self.neighbor_translucent(x - 1, y, zz + 1);
        let top_translucent = self.neighbor_translucent(x, y + 1, zz + 1);
        let bottom_translucent = self.neighbor_translucent(x, y - 1, zz + 1);
        // Canonical corner order top-SW, top-SE, bottom-SE, bottom-SW.
        let tsw = if !west_translucent && !top_translucent {
            west
        } else {
            self.neighbor_light(block, x - 1, y + 1, zz)
        };
        let tse = if !east_translucent && !top_translucent {
            east
        } else {
            self.neighbor_light(block, x + 1, y + 1, zz)
        };
        let bse = if !east_translucent && !bottom_translucent {
            east
        } else {
            self.neighbor_light(block, x + 1, y - 1, zz)
        };
        let bsw = if !west_translucent && !bottom_translucent {
            west
        } else {
            self.neighbor_light(block, x - 1, y - 1, zz)
        };
        if self.max_z >= 1.0 {
            zz -= 1;
        }
        let mut center = light;
        if self.max_z >= 1.0
            || !self
                .world
                .block(x, y, zz + 1)
                .is_solid_render(self.options.fancy_graphics)
        {
            center = self.neighbor_light_color(block, x, y, zz + 1);
        }
        let face_ao = self.neighbor_ao(x, y, zz + 1);
        let corners_ao = [
            (west.1 + tsw.1 + face_ao + top.1) / 4.0,
            (face_ao + top.1 + east.1 + tse.1) / 4.0,
            (bottom.1 + face_ao + bse.1 + east.1) / 4.0,
            (bsw.1 + west.1 + bottom.1 + face_ao) / 4.0,
        ];
        let corners_color = [
            Self::interpolate_light_color(west.0, tsw.0, top.0, center),
            Self::interpolate_light_color(top.0, east.0, tse.0, center),
            Self::interpolate_light_color(bottom.0, bse.0, east.0, center),
            Self::interpolate_light_color(bsw.0, west.0, bottom.0, center),
        ];
        // Vertex order: top-SW, bottom-SW, bottom-SE, top-SE.
        let weights = [
            [
                self.max_y * (1.0 - self.min_x),
                self.max_y * self.min_x,
                (1.0 - self.max_y) * self.min_x,
                (1.0 - self.max_y) * (1.0 - self.min_x),
            ],
            [
                self.min_y * (1.0 - self.min_x),
                self.min_y * self.min_x,
                (1.0 - self.min_y) * self.min_x,
                (1.0 - self.min_y) * (1.0 - self.min_x),
            ],
            [
                self.min_y * (1.0 - self.max_x),
                self.min_y * self.max_x,
                (1.0 - self.min_y) * self.max_x,
                (1.0 - self.min_y) * (1.0 - self.max_x),
            ],
            [
                self.max_y * (1.0 - self.max_x),
                self.max_y * self.max_x,
                (1.0 - self.max_y) * self.max_x,
                (1.0 - self.max_y) * (1.0 - self.max_x),
            ],
        ];
        self.set_side_vertex_light(
            &corners_ao,
            &corners_color,
            &weights,
            [0, 3, 2, 1],
            max,
            tinted,
            r,
            g,
            b,
            0.8,
        );
        let sprite = self.sprite_for(block, x, y, z, FACE_SOUTH);
        self.tesselate_south_face(x as f64, y as f64, z as f64, sprite);
        if self.options.fancy_graphics && sprite.grass_side && !self.has_forced_sprite() {
            for vertex in &mut self.vertex_color {
                vertex[0] *= r;
                vertex[1] *= g;
                vertex[2] *= b;
            }
            if let Some(overlay) = self.overlay_sprite(x, y, z) {
                self.tesselate_south_face(x as f64, y as f64, z as f64, overlay);
            }
        }
    }

    /// The `west` face of both smooth-lit variants.
    fn ao_west(
        &mut self,
        block: &BlockDef,
        x: i32,
        y: i32,
        z: i32,
        light: i32,
        tinted: bool,
        r: f32,
        g: f32,
        b: f32,
        max: bool,
    ) {
        let mut xx = x;
        if self.min_x <= 0.0 {
            xx -= 1;
        }
        let bottom = self.neighbor_light(block, xx, y - 1, z);
        let north = self.neighbor_light(block, xx, y, z - 1);
        let south = self.neighbor_light(block, xx, y, z + 1);
        let top = self.neighbor_light(block, xx, y + 1, z);
        let top_translucent = self.neighbor_translucent(xx - 1, y + 1, z);
        let bottom_translucent = self.neighbor_translucent(xx - 1, y - 1, z);
        let north_translucent = self.neighbor_translucent(xx - 1, y, z - 1);
        let south_translucent = self.neighbor_translucent(xx - 1, y, z + 1);
        // Canonical corner order bottom-SW, top-SW, top-NW, bottom-NW.
        let bsw = if !south_translucent && !bottom_translucent {
            south
        } else {
            self.neighbor_light(block, xx, y - 1, z + 1)
        };
        let tsw = if !south_translucent && !top_translucent {
            south
        } else {
            self.neighbor_light(block, xx, y + 1, z + 1)
        };
        let tnw = if !north_translucent && !top_translucent {
            north
        } else {
            self.neighbor_light(block, xx, y + 1, z - 1)
        };
        let bnw = if !north_translucent && !bottom_translucent {
            north
        } else {
            self.neighbor_light(block, xx, y - 1, z - 1)
        };
        if self.min_x <= 0.0 {
            xx += 1;
        }
        let mut center = light;
        if self.min_x <= 0.0
            || !self
                .world
                .block(xx - 1, y, z)
                .is_solid_render(self.options.fancy_graphics)
        {
            center = self.neighbor_light_color(block, xx - 1, y, z);
        }
        let face_ao = self.neighbor_ao(xx - 1, y, z);
        let corners_ao = [
            (bottom.1 + bsw.1 + face_ao + south.1) / 4.0,
            (face_ao + south.1 + top.1 + tsw.1) / 4.0,
            (north.1 + face_ao + tnw.1 + top.1) / 4.0,
            (bnw.1 + bottom.1 + north.1 + face_ao) / 4.0,
        ];
        let corners_color = [
            Self::interpolate_light_color(bottom.0, bsw.0, south.0, center),
            Self::interpolate_light_color(south.0, top.0, tsw.0, center),
            Self::interpolate_light_color(north.0, tnw.0, top.0, center),
            Self::interpolate_light_color(bnw.0, bottom.0, north.0, center),
        ];
        // Vertex order: top-SW, top-NW, bottom-NW, bottom-SW. The
        // corners are (bottom-SW, top-SW, top-NW, bottom-NW), so each
        // weight tuple starts one corner further round (vanilla's
        // `getVertexBrightness(var104, var107, var110, var101, ...)`).
        let weights = [
            [
                (1.0 - self.max_y) * self.max_z,
                self.max_y * self.max_z,
                self.max_y * (1.0 - self.max_z),
                (1.0 - self.max_y) * (1.0 - self.max_z),
            ],
            [
                (1.0 - self.max_y) * self.min_z,
                self.max_y * self.min_z,
                self.max_y * (1.0 - self.min_z),
                (1.0 - self.max_y) * (1.0 - self.min_z),
            ],
            [
                (1.0 - self.min_y) * self.min_z,
                self.min_y * self.min_z,
                self.min_y * (1.0 - self.min_z),
                (1.0 - self.min_y) * (1.0 - self.min_z),
            ],
            [
                (1.0 - self.min_y) * self.max_z,
                self.min_y * self.max_z,
                self.min_y * (1.0 - self.max_z),
                (1.0 - self.min_y) * (1.0 - self.max_z),
            ],
        ];
        self.set_side_vertex_light(
            &corners_ao,
            &corners_color,
            &weights,
            [1, 2, 3, 0],
            max,
            tinted,
            r,
            g,
            b,
            0.6,
        );
        let sprite = self.sprite_for(block, x, y, z, FACE_WEST);
        self.tesselate_west_face(x as f64, y as f64, z as f64, sprite);
        if self.options.fancy_graphics && sprite.grass_side && !self.has_forced_sprite() {
            for vertex in &mut self.vertex_color {
                vertex[0] *= r;
                vertex[1] *= g;
                vertex[2] *= b;
            }
            if let Some(overlay) = self.overlay_sprite(x, y, z) {
                self.tesselate_west_face(x as f64, y as f64, z as f64, overlay);
            }
        }
    }

    /// The `east` face of both smooth-lit variants.
    fn ao_east(
        &mut self,
        block: &BlockDef,
        x: i32,
        y: i32,
        z: i32,
        light: i32,
        tinted: bool,
        r: f32,
        g: f32,
        b: f32,
        max: bool,
    ) {
        let mut xx = x;
        if self.max_x >= 1.0 {
            xx += 1;
        }
        let bottom = self.neighbor_light(block, xx, y - 1, z);
        let north = self.neighbor_light(block, xx, y, z - 1);
        let south = self.neighbor_light(block, xx, y, z + 1);
        let top = self.neighbor_light(block, xx, y + 1, z);
        let top_translucent = self.neighbor_translucent(xx + 1, y + 1, z);
        let bottom_translucent = self.neighbor_translucent(xx + 1, y - 1, z);
        let north_translucent = self.neighbor_translucent(xx + 1, y, z - 1);
        let south_translucent = self.neighbor_translucent(xx + 1, y, z + 1);
        // Canonical corner order bottom-SE, bottom-NE, top-NE, top-SE.
        let bse = if !south_translucent && !bottom_translucent {
            south
        } else {
            self.neighbor_light(block, xx, y - 1, z + 1)
        };
        let bne = if !north_translucent && !bottom_translucent {
            north
        } else {
            self.neighbor_light(block, xx, y - 1, z - 1)
        };
        let tne = if !north_translucent && !top_translucent {
            north
        } else {
            self.neighbor_light(block, xx, y + 1, z - 1)
        };
        let tse = if !south_translucent && !top_translucent {
            south
        } else {
            self.neighbor_light(block, xx, y + 1, z + 1)
        };
        if self.max_x >= 1.0 {
            xx -= 1;
        }
        let mut center = light;
        if self.max_x >= 1.0
            || !self
                .world
                .block(xx + 1, y, z)
                .is_solid_render(self.options.fancy_graphics)
        {
            center = self.neighbor_light_color(block, xx + 1, y, z);
        }
        let face_ao = self.neighbor_ao(xx + 1, y, z);
        let corners_ao = [
            (bottom.1 + bse.1 + face_ao + south.1) / 4.0,
            (bne.1 + bottom.1 + north.1 + face_ao) / 4.0,
            (north.1 + face_ao + tne.1 + top.1) / 4.0,
            (face_ao + south.1 + top.1 + tse.1) / 4.0,
        ];
        let corners_color = [
            Self::interpolate_light_color(bottom.0, bse.0, south.0, center),
            Self::interpolate_light_color(bne.0, bottom.0, north.0, center),
            Self::interpolate_light_color(north.0, tne.0, top.0, center),
            Self::interpolate_light_color(south.0, tse.0, top.0, center),
        ];
        // Vertex order: bottom-SE, bottom-NE, top-NE, top-SE.
        let weights = [
            [
                (1.0 - self.min_y) * self.max_z,
                (1.0 - self.min_y) * (1.0 - self.max_z),
                self.min_y * (1.0 - self.max_z),
                self.min_y * self.max_z,
            ],
            [
                (1.0 - self.min_y) * self.min_z,
                (1.0 - self.min_y) * (1.0 - self.min_z),
                self.min_y * (1.0 - self.min_z),
                self.min_y * self.min_z,
            ],
            [
                (1.0 - self.max_y) * self.min_z,
                (1.0 - self.max_y) * (1.0 - self.min_z),
                self.max_y * (1.0 - self.min_z),
                self.max_y * self.min_z,
            ],
            [
                (1.0 - self.max_y) * self.max_z,
                (1.0 - self.max_y) * (1.0 - self.max_z),
                self.max_y * (1.0 - self.max_z),
                self.max_y * self.max_z,
            ],
        ];
        self.set_side_vertex_light(
            &corners_ao,
            &corners_color,
            &weights,
            [0, 1, 2, 3],
            max,
            tinted,
            r,
            g,
            b,
            0.6,
        );
        let sprite = self.sprite_for(block, x, y, z, FACE_EAST);
        self.tesselate_east_face(x as f64, y as f64, z as f64, sprite);
        if self.options.fancy_graphics && sprite.grass_side && !self.has_forced_sprite() {
            for vertex in &mut self.vertex_color {
                vertex[0] *= r;
                vertex[1] *= g;
                vertex[2] *= b;
            }
            if let Some(overlay) = self.overlay_sprite(x, y, z) {
                self.tesselate_east_face(x as f64, y as f64, z as f64, overlay);
            }
        }
    }

    /// Per-vertex AO multipliers and brightness of one side face.
    ///
    /// `weights[v]` is aligned with the canonical corner order, so the same
    /// tuple drives the AO mix and the `getVertexBrightness` mix. In the min
    /// variant vertex v takes corner `permutation[v]` directly.
    fn set_side_vertex_light(
        &mut self,
        corners_ao: &[f32; 4],
        corners_color: &[i32; 4],
        weights: &[[f64; 4]; 4],
        permutation: [usize; 4],
        max: bool,
        tinted: bool,
        r: f32,
        g: f32,
        b: f32,
        base: f32,
    ) {
        let (ao_mult, brightness) = if max {
            let mut ao_mult = [0.0_f32; 4];
            let mut brightness = [0_i32; 4];
            for vertex in 0..4 {
                let w = &weights[vertex];
                ao_mult[vertex] = corners_ao[0] * w[0] as f32
                    + corners_ao[1] * w[1] as f32
                    + corners_ao[2] * w[2] as f32
                    + corners_ao[3] * w[3] as f32;
                brightness[vertex] = Self::get_vertex_brightness(
                    corners_color[0],
                    corners_color[1],
                    corners_color[2],
                    corners_color[3],
                    w[0],
                    w[1],
                    w[2],
                    w[3],
                );
            }
            (ao_mult, brightness)
        } else {
            let ao_mult = [
                corners_ao[permutation[0]],
                corners_ao[permutation[1]],
                corners_ao[permutation[2]],
                corners_ao[permutation[3]],
            ];
            let brightness = [
                corners_color[permutation[0]],
                corners_color[permutation[1]],
                corners_color[permutation[2]],
                corners_color[permutation[3]],
            ];
            (ao_mult, brightness)
        };

        let base_r = if tinted { r * base } else { base };
        let base_g = if tinted { g * base } else { base };
        let base_b = if tinted { b * base } else { base };
        for vertex in 0..4 {
            self.vertex_color[vertex] = [
                base_r * ao_mult[vertex],
                base_g * ao_mult[vertex],
                base_b * ao_mult[vertex],
            ];
            self.vertex_brightness[vertex] = brightness[vertex];
        }
    }

    // Face tessellators -------------------------------------------------
    //
    // The UV dance mirrors vanilla's variable names: (u_a, u_b, v_a, v_b)
    // are var10/12/14/16 and (u_c, u_d, v_c, v_d) are var18/20/22/24.

    fn tesselate_bottom_face(&mut self, x: f64, y: f64, z: f64, mut sprite: Sprite) {
        if let Some(forced) = self.forced_sprite {
            sprite = forced;
        }
        self.tesselator.layer(sprite.layer);
        let mut u_a = sprite.get_u(self.min_x * 16.0);
        let mut u_b = sprite.get_u(self.max_x * 16.0);
        let mut v_a = sprite.get_v(self.min_z * 16.0);
        let mut v_b = sprite.get_v(self.max_z * 16.0);
        if self.min_x < 0.0 || self.max_x > 1.0 {
            u_a = sprite.u_min();
            u_b = sprite.u_max();
        }
        if self.min_z < 0.0 || self.max_z > 1.0 {
            v_a = sprite.v_min();
            v_b = sprite.v_max();
        }
        let (mut u_c, mut u_d, mut v_c, mut v_d) = (u_b, u_a, v_a, v_b);
        match self.face_rotations[FACE_BOTTOM as usize] {
            2 => {
                u_a = sprite.get_u(self.min_z * 16.0);
                v_a = sprite.get_v(16.0 - self.max_x * 16.0);
                u_b = sprite.get_u(self.max_z * 16.0);
                v_b = sprite.get_v(16.0 - self.min_x * 16.0);
                v_c = v_a;
                v_d = v_b;
                u_c = u_a;
                u_d = u_b;
                v_a = v_b;
                v_b = v_c;
            }
            1 => {
                u_a = sprite.get_u(16.0 - self.max_z * 16.0);
                v_a = sprite.get_v(self.min_x * 16.0);
                u_b = sprite.get_u(16.0 - self.min_z * 16.0);
                v_b = sprite.get_v(self.max_x * 16.0);
                u_c = u_b;
                u_d = u_a;
                std::mem::swap(&mut u_a, &mut u_b);
                v_c = v_b;
                v_d = v_a;
            }
            3 => {
                u_a = sprite.get_u(16.0 - self.min_x * 16.0);
                u_b = sprite.get_u(16.0 - self.max_x * 16.0);
                v_a = sprite.get_v(16.0 - self.min_z * 16.0);
                v_b = sprite.get_v(16.0 - self.max_z * 16.0);
                u_c = u_b;
                u_d = u_a;
                v_c = v_a;
                v_d = v_b;
            }
            _ => {}
        }

        let mut x0 = x + self.min_x;
        let mut x1 = x + self.max_x;
        let y0 = y + self.min_y;
        let z0 = z + self.min_z;
        let z1 = z + self.max_z;
        if self.flip_depth {
            x0 = x + self.max_x;
            x1 = x + self.min_x;
        }

        let t = &mut self.tesselator;
        if self.ambient_occlusion {
            t.color(
                self.vertex_color[0][0],
                self.vertex_color[0][1],
                self.vertex_color[0][2],
            );
            t.brightness(self.vertex_brightness[0]);
            t.vertex(x0, y0, z1, u_d, v_d);
            t.color(
                self.vertex_color[1][0],
                self.vertex_color[1][1],
                self.vertex_color[1][2],
            );
            t.brightness(self.vertex_brightness[1]);
            t.vertex(x0, y0, z0, u_a, v_a);
            t.color(
                self.vertex_color[2][0],
                self.vertex_color[2][1],
                self.vertex_color[2][2],
            );
            t.brightness(self.vertex_brightness[2]);
            t.vertex(x1, y0, z0, u_c, v_c);
            t.color(
                self.vertex_color[3][0],
                self.vertex_color[3][1],
                self.vertex_color[3][2],
            );
            t.brightness(self.vertex_brightness[3]);
            t.vertex(x1, y0, z1, u_b, v_b);
        } else {
            t.vertex(x0, y0, z1, u_d, v_d);
            t.vertex(x0, y0, z0, u_a, v_a);
            t.vertex(x1, y0, z0, u_c, v_c);
            t.vertex(x1, y0, z1, u_b, v_b);
        }
    }

    fn tesselate_top_face(&mut self, x: f64, y: f64, z: f64, mut sprite: Sprite) {
        if let Some(forced) = self.forced_sprite {
            sprite = forced;
        }
        self.tesselator.layer(sprite.layer);
        let mut u_a = sprite.get_u(self.min_x * 16.0);
        let mut u_b = sprite.get_u(self.max_x * 16.0);
        let mut v_a = sprite.get_v(self.min_z * 16.0);
        let mut v_b = sprite.get_v(self.max_z * 16.0);
        if self.min_x < 0.0 || self.max_x > 1.0 {
            u_a = sprite.u_min();
            u_b = sprite.u_max();
        }
        if self.min_z < 0.0 || self.max_z > 1.0 {
            v_a = sprite.v_min();
            v_b = sprite.v_max();
        }
        let (mut u_c, mut u_d, mut v_c, mut v_d) = (u_b, u_a, v_a, v_b);
        match self.face_rotations[FACE_TOP as usize] {
            1 => {
                u_a = sprite.get_u(self.min_z * 16.0);
                v_a = sprite.get_v(16.0 - self.max_x * 16.0);
                u_b = sprite.get_u(self.max_z * 16.0);
                v_b = sprite.get_v(16.0 - self.min_x * 16.0);
                v_c = v_a;
                v_d = v_b;
                u_c = u_a;
                u_d = u_b;
                v_a = v_b;
                v_b = v_c;
            }
            2 => {
                u_a = sprite.get_u(16.0 - self.max_z * 16.0);
                v_a = sprite.get_v(self.min_x * 16.0);
                u_b = sprite.get_u(16.0 - self.min_z * 16.0);
                v_b = sprite.get_v(self.max_x * 16.0);
                u_c = u_b;
                u_d = u_a;
                std::mem::swap(&mut u_a, &mut u_b);
                v_c = v_b;
                v_d = v_a;
            }
            3 => {
                u_a = sprite.get_u(16.0 - self.min_x * 16.0);
                u_b = sprite.get_u(16.0 - self.max_x * 16.0);
                v_a = sprite.get_v(16.0 - self.min_z * 16.0);
                v_b = sprite.get_v(16.0 - self.max_z * 16.0);
                u_c = u_b;
                u_d = u_a;
                v_c = v_a;
                v_d = v_b;
            }
            _ => {}
        }

        let mut x0 = x + self.min_x;
        let mut x1 = x + self.max_x;
        let y0 = y + self.max_y;
        let z0 = z + self.min_z;
        let z1 = z + self.max_z;
        if self.flip_depth {
            x0 = x + self.max_x;
            x1 = x + self.min_x;
        }

        let t = &mut self.tesselator;
        if self.ambient_occlusion {
            t.color(
                self.vertex_color[0][0],
                self.vertex_color[0][1],
                self.vertex_color[0][2],
            );
            t.brightness(self.vertex_brightness[0]);
            t.vertex(x1, y0, z1, u_b, v_b);
            t.color(
                self.vertex_color[1][0],
                self.vertex_color[1][1],
                self.vertex_color[1][2],
            );
            t.brightness(self.vertex_brightness[1]);
            t.vertex(x1, y0, z0, u_c, v_c);
            t.color(
                self.vertex_color[2][0],
                self.vertex_color[2][1],
                self.vertex_color[2][2],
            );
            t.brightness(self.vertex_brightness[2]);
            t.vertex(x0, y0, z0, u_a, v_a);
            t.color(
                self.vertex_color[3][0],
                self.vertex_color[3][1],
                self.vertex_color[3][2],
            );
            t.brightness(self.vertex_brightness[3]);
            t.vertex(x0, y0, z1, u_d, v_d);
        } else {
            t.vertex(x1, y0, z1, u_b, v_b);
            t.vertex(x1, y0, z0, u_c, v_c);
            t.vertex(x0, y0, z0, u_a, v_a);
            t.vertex(x0, y0, z1, u_d, v_d);
        }
    }

    fn tesselate_north_face(&mut self, x: f64, y: f64, z: f64, mut sprite: Sprite) {
        if let Some(forced) = self.forced_sprite {
            sprite = forced;
        }
        self.tesselator.layer(sprite.layer);
        let mut u_a = sprite.get_u(self.min_x * 16.0);
        let mut u_b = sprite.get_u(self.max_x * 16.0);
        if self.mirror_uv {
            u_b = sprite.get_u((1.0 - self.min_x) * 16.0);
            u_a = sprite.get_u((1.0 - self.max_x) * 16.0);
        }
        let mut v_a = sprite.get_v(16.0 - self.max_y * 16.0);
        let mut v_b = sprite.get_v(16.0 - self.min_y * 16.0);
        if self.render_flipped {
            std::mem::swap(&mut u_a, &mut u_b);
        }
        if self.min_x < 0.0 || self.max_x > 1.0 {
            u_a = sprite.u_min();
            u_b = sprite.u_max();
        }
        if self.min_y < 0.0 || self.max_y > 1.0 {
            v_a = sprite.v_min();
            v_b = sprite.v_max();
        }
        let (mut u_c, mut u_d, mut v_c, mut v_d) = (u_b, u_a, v_a, v_b);
        match self.face_rotations[FACE_NORTH as usize] {
            2 => {
                u_a = sprite.get_u(self.min_y * 16.0);
                u_b = sprite.get_u(self.max_y * 16.0);
                v_a = sprite.get_v(16.0 - self.min_x * 16.0);
                v_b = sprite.get_v(16.0 - self.max_x * 16.0);
                v_c = v_a;
                v_d = v_b;
                u_c = u_a;
                u_d = u_b;
                v_a = v_b;
                v_b = v_c;
            }
            1 => {
                u_a = sprite.get_u(16.0 - self.max_y * 16.0);
                u_b = sprite.get_u(16.0 - self.min_y * 16.0);
                v_a = sprite.get_v(self.max_x * 16.0);
                v_b = sprite.get_v(self.min_x * 16.0);
                u_c = u_b;
                u_d = u_a;
                std::mem::swap(&mut u_a, &mut u_b);
                v_c = v_b;
                v_d = v_a;
            }
            3 => {
                u_a = sprite.get_u(16.0 - self.min_x * 16.0);
                u_b = sprite.get_u(16.0 - self.max_x * 16.0);
                v_a = sprite.get_v(self.max_y * 16.0);
                v_b = sprite.get_v(self.min_y * 16.0);
                u_c = u_b;
                u_d = u_a;
                v_c = v_a;
                v_d = v_b;
            }
            _ => {}
        }

        let mut x0 = x + self.min_x;
        let mut x1 = x + self.max_x;
        let y0 = y + self.min_y;
        let y1 = y + self.max_y;
        let z0 = z + self.min_z;
        if self.flip_depth {
            x0 = x + self.max_x;
            x1 = x + self.min_x;
        }

        let t = &mut self.tesselator;
        if self.ambient_occlusion {
            t.color(
                self.vertex_color[0][0],
                self.vertex_color[0][1],
                self.vertex_color[0][2],
            );
            t.brightness(self.vertex_brightness[0]);
            t.vertex(x0, y1, z0, u_c, v_c);
            t.color(
                self.vertex_color[1][0],
                self.vertex_color[1][1],
                self.vertex_color[1][2],
            );
            t.brightness(self.vertex_brightness[1]);
            t.vertex(x1, y1, z0, u_a, v_a);
            t.color(
                self.vertex_color[2][0],
                self.vertex_color[2][1],
                self.vertex_color[2][2],
            );
            t.brightness(self.vertex_brightness[2]);
            t.vertex(x1, y0, z0, u_d, v_d);
            t.color(
                self.vertex_color[3][0],
                self.vertex_color[3][1],
                self.vertex_color[3][2],
            );
            t.brightness(self.vertex_brightness[3]);
            t.vertex(x0, y0, z0, u_b, v_b);
        } else {
            t.vertex(x0, y1, z0, u_c, v_c);
            t.vertex(x1, y1, z0, u_a, v_a);
            t.vertex(x1, y0, z0, u_d, v_d);
            t.vertex(x0, y0, z0, u_b, v_b);
        }
    }

    fn tesselate_south_face(&mut self, x: f64, y: f64, z: f64, mut sprite: Sprite) {
        if let Some(forced) = self.forced_sprite {
            sprite = forced;
        }
        self.tesselator.layer(sprite.layer);
        let mut u_a = sprite.get_u(self.min_x * 16.0);
        let mut u_b = sprite.get_u(self.max_x * 16.0);
        let mut v_a = sprite.get_v(16.0 - self.max_y * 16.0);
        let mut v_b = sprite.get_v(16.0 - self.min_y * 16.0);
        if self.render_flipped {
            std::mem::swap(&mut u_a, &mut u_b);
        }
        if self.min_x < 0.0 || self.max_x > 1.0 {
            u_a = sprite.u_min();
            u_b = sprite.u_max();
        }
        if self.min_y < 0.0 || self.max_y > 1.0 {
            v_a = sprite.v_min();
            v_b = sprite.v_max();
        }
        let (mut u_c, mut u_d, mut v_c, mut v_d) = (u_b, u_a, v_a, v_b);
        match self.face_rotations[FACE_SOUTH as usize] {
            1 => {
                u_a = sprite.get_u(self.min_y * 16.0);
                v_b = sprite.get_v(16.0 - self.min_x * 16.0);
                u_b = sprite.get_u(self.max_y * 16.0);
                v_a = sprite.get_v(16.0 - self.max_x * 16.0);
                v_c = v_a;
                v_d = v_b;
                u_c = u_a;
                u_d = u_b;
                v_a = v_b;
                v_b = v_c;
            }
            2 => {
                u_a = sprite.get_u(16.0 - self.max_y * 16.0);
                v_a = sprite.get_v(self.min_x * 16.0);
                u_b = sprite.get_u(16.0 - self.min_y * 16.0);
                v_b = sprite.get_v(self.max_x * 16.0);
                u_c = u_b;
                u_d = u_a;
                std::mem::swap(&mut u_a, &mut u_b);
                v_c = v_b;
                v_d = v_a;
            }
            3 => {
                u_a = sprite.get_u(16.0 - self.min_x * 16.0);
                u_b = sprite.get_u(16.0 - self.max_x * 16.0);
                v_a = sprite.get_v(self.max_y * 16.0);
                v_b = sprite.get_v(self.min_y * 16.0);
                u_c = u_b;
                u_d = u_a;
                v_c = v_a;
                v_d = v_b;
            }
            _ => {}
        }

        let mut x0 = x + self.min_x;
        let mut x1 = x + self.max_x;
        let y0 = y + self.min_y;
        let y1 = y + self.max_y;
        let z0 = z + self.max_z;
        if self.flip_depth {
            x0 = x + self.max_x;
            x1 = x + self.min_x;
        }

        let t = &mut self.tesselator;
        if self.ambient_occlusion {
            t.color(
                self.vertex_color[0][0],
                self.vertex_color[0][1],
                self.vertex_color[0][2],
            );
            t.brightness(self.vertex_brightness[0]);
            t.vertex(x0, y1, z0, u_a, v_a);
            t.color(
                self.vertex_color[1][0],
                self.vertex_color[1][1],
                self.vertex_color[1][2],
            );
            t.brightness(self.vertex_brightness[1]);
            t.vertex(x0, y0, z0, u_d, v_d);
            t.color(
                self.vertex_color[2][0],
                self.vertex_color[2][1],
                self.vertex_color[2][2],
            );
            t.brightness(self.vertex_brightness[2]);
            t.vertex(x1, y0, z0, u_b, v_b);
            t.color(
                self.vertex_color[3][0],
                self.vertex_color[3][1],
                self.vertex_color[3][2],
            );
            t.brightness(self.vertex_brightness[3]);
            t.vertex(x1, y1, z0, u_c, v_c);
        } else {
            t.vertex(x0, y1, z0, u_a, v_a);
            t.vertex(x0, y0, z0, u_d, v_d);
            t.vertex(x1, y0, z0, u_b, v_b);
            t.vertex(x1, y1, z0, u_c, v_c);
        }
    }

    fn tesselate_west_face(&mut self, x: f64, y: f64, z: f64, mut sprite: Sprite) {
        if let Some(forced) = self.forced_sprite {
            sprite = forced;
        }
        self.tesselator.layer(sprite.layer);
        let mut u_a = sprite.get_u(self.min_z * 16.0);
        let mut u_b = sprite.get_u(self.max_z * 16.0);
        let mut v_a = sprite.get_v(16.0 - self.max_y * 16.0);
        let mut v_b = sprite.get_v(16.0 - self.min_y * 16.0);
        if self.render_flipped {
            std::mem::swap(&mut u_a, &mut u_b);
        }
        if self.min_z < 0.0 || self.max_z > 1.0 {
            u_a = sprite.u_min();
            u_b = sprite.u_max();
        }
        if self.min_y < 0.0 || self.max_y > 1.0 {
            v_a = sprite.v_min();
            v_b = sprite.v_max();
        }
        let (mut u_c, mut u_d, mut v_c, mut v_d) = (u_b, u_a, v_a, v_b);
        // The west face reads the east rotation, the east face the west
        // rotation: a vanilla quirk, kept.
        match self.face_rotations[FACE_EAST as usize] {
            1 => {
                u_a = sprite.get_u(self.min_y * 16.0);
                v_a = sprite.get_v(16.0 - self.max_z * 16.0);
                u_b = sprite.get_u(self.max_y * 16.0);
                v_b = sprite.get_v(16.0 - self.min_z * 16.0);
                v_c = v_a;
                v_d = v_b;
                u_c = u_a;
                u_d = u_b;
                v_a = v_b;
                v_b = v_c;
            }
            2 => {
                u_a = sprite.get_u(16.0 - self.max_y * 16.0);
                v_a = sprite.get_v(self.min_z * 16.0);
                u_b = sprite.get_u(16.0 - self.min_y * 16.0);
                v_b = sprite.get_v(self.max_z * 16.0);
                u_c = u_b;
                u_d = u_a;
                std::mem::swap(&mut u_a, &mut u_b);
                v_c = v_b;
                v_d = v_a;
            }
            3 => {
                u_a = sprite.get_u(16.0 - self.min_z * 16.0);
                u_b = sprite.get_u(16.0 - self.max_z * 16.0);
                v_a = sprite.get_v(self.max_y * 16.0);
                v_b = sprite.get_v(self.min_y * 16.0);
                u_c = u_b;
                u_d = u_a;
                v_c = v_a;
                v_d = v_b;
            }
            _ => {}
        }

        let x0 = x + self.min_x;
        let y0 = y + self.min_y;
        let y1 = y + self.max_y;
        let mut z0 = z + self.min_z;
        let mut z1 = z + self.max_z;
        if self.flip_depth {
            z0 = z + self.max_z;
            z1 = z + self.min_z;
        }

        let t = &mut self.tesselator;
        if self.ambient_occlusion {
            t.color(
                self.vertex_color[0][0],
                self.vertex_color[0][1],
                self.vertex_color[0][2],
            );
            t.brightness(self.vertex_brightness[0]);
            t.vertex(x0, y1, z1, u_c, v_c);
            t.color(
                self.vertex_color[1][0],
                self.vertex_color[1][1],
                self.vertex_color[1][2],
            );
            t.brightness(self.vertex_brightness[1]);
            t.vertex(x0, y1, z0, u_a, v_a);
            t.color(
                self.vertex_color[2][0],
                self.vertex_color[2][1],
                self.vertex_color[2][2],
            );
            t.brightness(self.vertex_brightness[2]);
            t.vertex(x0, y0, z0, u_d, v_d);
            t.color(
                self.vertex_color[3][0],
                self.vertex_color[3][1],
                self.vertex_color[3][2],
            );
            t.brightness(self.vertex_brightness[3]);
            t.vertex(x0, y0, z1, u_b, v_b);
        } else {
            t.vertex(x0, y1, z1, u_c, v_c);
            t.vertex(x0, y1, z0, u_a, v_a);
            t.vertex(x0, y0, z0, u_d, v_d);
            t.vertex(x0, y0, z1, u_b, v_b);
        }
    }

    fn tesselate_east_face(&mut self, x: f64, y: f64, z: f64, mut sprite: Sprite) {
        if let Some(forced) = self.forced_sprite {
            sprite = forced;
        }
        self.tesselator.layer(sprite.layer);
        let mut u_a = sprite.get_u(self.min_z * 16.0);
        let mut u_b = sprite.get_u(self.max_z * 16.0);
        if self.mirror_uv {
            u_b = sprite.get_u((1.0 - self.min_z) * 16.0);
            u_a = sprite.get_u((1.0 - self.max_z) * 16.0);
        }
        let mut v_a = sprite.get_v(16.0 - self.max_y * 16.0);
        let mut v_b = sprite.get_v(16.0 - self.min_y * 16.0);
        if self.render_flipped {
            std::mem::swap(&mut u_a, &mut u_b);
        }
        if self.min_z < 0.0 || self.max_z > 1.0 {
            u_a = sprite.u_min();
            u_b = sprite.u_max();
        }
        if self.min_y < 0.0 || self.max_y > 1.0 {
            v_a = sprite.v_min();
            v_b = sprite.v_max();
        }
        let (mut u_c, mut u_d, mut v_c, mut v_d) = (u_b, u_a, v_a, v_b);
        match self.face_rotations[FACE_WEST as usize] {
            2 => {
                u_a = sprite.get_u(self.min_y * 16.0);
                v_a = sprite.get_v(16.0 - self.min_z * 16.0);
                u_b = sprite.get_u(self.max_y * 16.0);
                v_b = sprite.get_v(16.0 - self.max_z * 16.0);
                v_c = v_a;
                v_d = v_b;
                u_c = u_a;
                u_d = u_b;
                v_a = v_b;
                v_b = v_c;
            }
            1 => {
                u_a = sprite.get_u(16.0 - self.max_y * 16.0);
                v_a = sprite.get_v(self.max_z * 16.0);
                u_b = sprite.get_u(16.0 - self.min_y * 16.0);
                v_b = sprite.get_v(self.min_z * 16.0);
                u_c = u_b;
                u_d = u_a;
                std::mem::swap(&mut u_a, &mut u_b);
                v_c = v_b;
                v_d = v_a;
            }
            3 => {
                u_a = sprite.get_u(16.0 - self.min_z * 16.0);
                u_b = sprite.get_u(16.0 - self.max_z * 16.0);
                v_a = sprite.get_v(self.max_y * 16.0);
                v_b = sprite.get_v(self.min_y * 16.0);
                u_c = u_b;
                u_d = u_a;
                v_c = v_a;
                v_d = v_b;
            }
            _ => {}
        }

        let x0 = x + self.max_x;
        let y0 = y + self.min_y;
        let y1 = y + self.max_y;
        let mut z0 = z + self.min_z;
        let mut z1 = z + self.max_z;
        if self.flip_depth {
            z0 = z + self.max_z;
            z1 = z + self.min_z;
        }

        let t = &mut self.tesselator;
        if self.ambient_occlusion {
            t.color(
                self.vertex_color[0][0],
                self.vertex_color[0][1],
                self.vertex_color[0][2],
            );
            t.brightness(self.vertex_brightness[0]);
            t.vertex(x0, y0, z1, u_d, v_d);
            t.color(
                self.vertex_color[1][0],
                self.vertex_color[1][1],
                self.vertex_color[1][2],
            );
            t.brightness(self.vertex_brightness[1]);
            t.vertex(x0, y0, z0, u_b, v_b);
            t.color(
                self.vertex_color[2][0],
                self.vertex_color[2][1],
                self.vertex_color[2][2],
            );
            t.brightness(self.vertex_brightness[2]);
            t.vertex(x0, y1, z0, u_c, v_c);
            t.color(
                self.vertex_color[3][0],
                self.vertex_color[3][1],
                self.vertex_color[3][2],
            );
            t.brightness(self.vertex_brightness[3]);
            t.vertex(x0, y1, z1, u_a, v_a);
        } else {
            t.vertex(x0, y0, z1, u_d, v_d);
            t.vertex(x0, y0, z0, u_b, v_b);
            t.vertex(x0, y1, z0, u_c, v_c);
            t.vertex(x0, y1, z1, u_a, v_a);
        }
    }

    // Crossed squares ---------------------------------------------------

    /// The 3129871/116129781/42317861 hash of `tesselateCross`: the same
    /// formula vanilla applies to tallgrass, flowers and double plants.
    pub fn cross_offset(x: i32, y: i32, z: i32) -> i64 {
        // `x * 3129871` is an int multiply in vanilla, then widened to long.
        let mut hash =
            (x.wrapping_mul(3129871) as i64) ^ (z as i64).wrapping_mul(116129781) ^ y as i64;
        hash = hash
            .wrapping_mul(hash)
            .wrapping_mul(42317861)
            .wrapping_add(hash.wrapping_mul(11));
        hash
    }

    /// `tesselateCross(Block, x, y, z)`.
    fn tesselate_cross(&mut self, block: &BlockDef, x: i32, y: i32, z: i32) -> bool {
        self.tesselator
            .brightness(block.light_color(self.world, x, y, z));
        let color = block.color(self.world.metadata(x, y, z) as u8, self.world, x, y, z);
        let mut r = (color >> 16 & 0xFF) as f32 / 255.0;
        let mut g = (color >> 8 & 0xFF) as f32 / 255.0;
        let mut b = (color & 0xFF) as f32 / 255.0;
        if self.options.anaglyph {
            let gray = (r * 30.0 + g * 59.0 + b * 11.0) / 100.0;
            let red = (r * 30.0 + g * 70.0) / 100.0;
            let blue = (r * 30.0 + b * 70.0) / 100.0;
            r = gray;
            g = red;
            b = blue;
        }
        self.tesselator.color(r, g, b);

        let mut px = x as f64;
        let mut py = y as f64;
        let mut pz = z as f64;
        match block.kind {
            Kind::Cross(CrossKind::TallGrass) => {
                let hash = Self::cross_offset(x, y, z);
                px += (((hash >> 16 & 15) as f32 / 15.0 - 0.5) * 0.5) as f64;
                py += (((hash >> 20 & 15) as f32 / 15.0 - 1.0) * 0.2) as f64;
                pz += (((hash >> 24 & 15) as f32 / 15.0 - 0.5) * 0.5) as f64;
            }
            Kind::Cross(CrossKind::Flower) => {
                let hash = Self::cross_offset(x, y, z);
                px += (((hash >> 16 & 15) as f32 / 15.0 - 0.5) * 0.3) as f64;
                pz += (((hash >> 24 & 15) as f32 / 15.0 - 0.5) * 0.3) as f64;
            }
            _ => {}
        }

        let sprite = self.sprite_for(block, x, y, z, 0);
        self.tesselate_cross_sprite(sprite, px, py, pz, 1.0);
        true
    }

    /// `tesselateCross(Sprite, x, y, z, size)`.
    fn tesselate_cross_sprite(&mut self, mut sprite: Sprite, x: f64, y: f64, z: f64, size: f32) {
        if let Some(forced) = self.forced_sprite {
            sprite = forced;
        }
        self.tesselator.layer(sprite.layer);
        let u_min = sprite.u_min();
        let v_min = sprite.v_min();
        let u_max = sprite.u_max();
        let v_max = sprite.v_max();
        let half = 0.45 * size as f64;
        let x0 = x + 0.5 - half;
        let x1 = x + 0.5 + half;
        let z0 = z + 0.5 - half;
        let z1 = z + 0.5 + half;
        let t = &mut self.tesselator;
        t.vertex(x0, y + size as f64, z0, u_min, v_min);
        t.vertex(x0, y, z0, u_min, v_max);
        t.vertex(x1, y, z1, u_max, v_max);
        t.vertex(x1, y + size as f64, z1, u_max, v_min);
        t.vertex(x1, y + size as f64, z1, u_min, v_min);
        t.vertex(x1, y, z1, u_min, v_max);
        t.vertex(x0, y, z0, u_max, v_max);
        t.vertex(x0, y + size as f64, z0, u_max, v_min);
        t.vertex(x0, y + size as f64, z1, u_min, v_min);
        t.vertex(x0, y, z1, u_min, v_max);
        t.vertex(x1, y, z0, u_max, v_max);
        t.vertex(x1, y + size as f64, z0, u_max, v_min);
        t.vertex(x1, y + size as f64, z0, u_min, v_min);
        t.vertex(x1, y, z0, u_min, v_max);
        t.vertex(x0, y, z1, u_max, v_max);
        t.vertex(x0, y + size as f64, z1, u_max, v_min);
    }

    // Crops -------------------------------------------------------------

    /// `tesselatePlant(Block, x, y, z)` (render type 6): the block is anchored
    /// 1/16 below its cell and crosses at the centre, untinted.
    fn tesselate_plant(&mut self, block: &BlockDef, x: i32, y: i32, z: i32) -> bool {
        self.tesselator
            .brightness(block.light_color(self.world, x, y, z));
        self.tesselator.color(1.0, 1.0, 1.0);
        let sprite = self.sprite_for(block, x, y, z, 0);
        self.tesselate_plant_sprite(sprite, x as f64, y as f64 - 0.0625, z as f64);
        true
    }

    /// `tesselatePlant(Sprite, double, double, double)`: four crossed pairs of
    /// quads at the crops' 0.25 offsets.
    fn tesselate_plant_sprite(&mut self, mut sprite: Sprite, x: f64, y: f64, z: f64) {
        if let Some(forced) = self.forced_sprite {
            sprite = forced;
        }
        self.tesselator.layer(sprite.layer);
        let u_min = sprite.u_min();
        let v_min = sprite.v_min();
        let u_max = sprite.u_max();
        let v_max = sprite.v_max();
        let t = &mut self.tesselator;
        let (x0, x1) = (x + 0.5 - 0.25, x + 0.5 + 0.25);
        let (z0, z1) = (z + 0.5 - 0.5, z + 0.5 + 0.5);
        for a in [x0, x1] {
            t.vertex(a, y + 1.0, z0, u_min, v_min);
            t.vertex(a, y, z0, u_min, v_max);
            t.vertex(a, y, z1, u_max, v_max);
            t.vertex(a, y + 1.0, z1, u_max, v_min);
            t.vertex(a, y + 1.0, z1, u_min, v_min);
            t.vertex(a, y, z1, u_min, v_max);
            t.vertex(a, y, z0, u_max, v_max);
            t.vertex(a, y + 1.0, z0, u_max, v_min);
        }
        let (x0, x1) = (x + 0.5 - 0.5, x + 0.5 + 0.5);
        let (z0, z1) = (z + 0.5 - 0.25, z + 0.5 + 0.25);
        for a in [z0, z1] {
            t.vertex(x0, y + 1.0, a, u_min, v_min);
            t.vertex(x0, y, a, u_min, v_max);
            t.vertex(x1, y, a, u_max, v_max);
            t.vertex(x1, y + 1.0, a, u_max, v_min);
            t.vertex(x1, y + 1.0, a, u_min, v_min);
            t.vertex(x1, y, a, u_min, v_max);
            t.vertex(x0, y, a, u_max, v_max);
            t.vertex(x0, y + 1.0, a, u_max, v_min);
        }
    }

    // Lily pads ---------------------------------------------------------

    /// `tesselateLilyPad` (render type 23): one rotated 1/16 quad and its
    /// underside, darkened by `(color & 0xFEFEFE) >> 1`.
    fn tesselate_lily_pad(&mut self, block: &BlockDef, x: i32, y: i32, z: i32) -> bool {
        let sprite = self.sprite_for(block, x, y, z, 1);
        self.tesselator.layer(sprite.layer);
        let (u_min, v_min, u_max, v_max) = (
            sprite.u_min(),
            sprite.v_min(),
            sprite.u_max(),
            sprite.v_max(),
        );
        let quarter = (Self::cross_offset(x, y, z) >> 16 & 3) as i32;
        let color = block.color(self.world.metadata(x, y, z) as u8, self.world, x, y, z);
        self.tesselator
            .brightness(block.light_color(self.world, x, y, z));
        // `y + 0.015625F` is float arithmetic in vanilla.
        let py = f64::from(y as f32 + 0.015_625);
        let cx = x as f32 + 0.5;
        let cz = z as f32 + 0.5;
        let a = (quarter & 1) as f32 * 0.5 * (1 - (quarter / 2 % 2) * 2) as f32;
        let b = ((quarter + 1) & 1) as f32 * 0.5 * (1 - ((quarter + 1) / 2 % 2) * 2) as f32;
        let corners = [
            (cx + a - b, cz + a + b, u_min, v_min),
            (cx + a + b, cz - a + b, u_max, v_min),
            (cx - a + b, cz - a - b, u_max, v_max),
            (cx - a - b, cz + a - b, u_min, v_max),
        ];
        self.tesselator.color_rgb(color);
        for (px, pz, u, v) in corners {
            self.tesselator
                .vertex(f64::from(px), py, f64::from(pz), u, v);
        }
        self.tesselator.color_rgb((color & 16_711_422) >> 1);
        for (px, pz, u, v) in corners.iter().rev() {
            self.tesselator
                .vertex(f64::from(*px), py, f64::from(*pz), *u, *v);
        }
        true
    }

    // Cactus ------------------------------------------------------------

    /// `tesselateCactus` (render type 13): the flat-lit faces, their sides
    /// pulled 1/16 inward by the tesselator's offset.
    fn tesselate_cactus(&mut self, block: &BlockDef, x: i32, y: i32, z: i32) -> bool {
        let color = block.color(self.world.metadata(x, y, z) as u8, self.world, x, y, z);
        let r = (color >> 16 & 0xFF) as f32 / 255.0;
        let g = (color >> 8 & 0xFF) as f32 / 255.0;
        let b = (color & 0xFF) as f32 / 255.0;
        let light = block.light_color(self.world, x, y, z);
        if self.render_face_always || self.should_render_face(block, x, y - 1, z, FACE_BOTTOM) {
            let bottom = if self.min_y > 0.0 {
                light
            } else {
                block.light_color(self.world, x, y - 1, z)
            };
            self.tesselator.brightness(bottom);
            self.tesselator.color(0.5 * r, 0.5 * g, 0.5 * b);
            let sprite = self.sprite_for(block, x, y, z, FACE_BOTTOM);
            self.tesselate_bottom_face(x as f64, y as f64, z as f64, sprite);
        }
        if self.render_face_always || self.should_render_face(block, x, y + 1, z, FACE_TOP) {
            let top = if self.max_y < 1.0 {
                light
            } else {
                block.light_color(self.world, x, y + 1, z)
            };
            self.tesselator.brightness(top);
            self.tesselator.color(r, g, b);
            let sprite = self.sprite_for(block, x, y, z, FACE_TOP);
            self.tesselate_top_face(x as f64, y as f64, z as f64, sprite);
        }
        self.tesselator.brightness(light);
        let inset = 0.0625_f64;
        self.tesselator.color(0.8 * r, 0.8 * g, 0.8 * b);
        self.tesselator.add_offset(0.0, 0.0, inset);
        let sprite = self.sprite_for(block, x, y, z, FACE_NORTH);
        self.tesselate_north_face(x as f64, y as f64, z as f64, sprite);
        self.tesselator.add_offset(0.0, 0.0, -inset);
        self.tesselator.add_offset(0.0, 0.0, -inset);
        let sprite = self.sprite_for(block, x, y, z, FACE_SOUTH);
        self.tesselate_south_face(x as f64, y as f64, z as f64, sprite);
        self.tesselator.add_offset(0.0, 0.0, inset);
        self.tesselator.color(0.6 * r, 0.6 * g, 0.6 * b);
        self.tesselator.add_offset(inset, 0.0, 0.0);
        let sprite = self.sprite_for(block, x, y, z, FACE_WEST);
        self.tesselate_west_face(x as f64, y as f64, z as f64, sprite);
        self.tesselator.add_offset(-inset, 0.0, 0.0);
        self.tesselator.add_offset(-inset, 0.0, 0.0);
        let sprite = self.sprite_for(block, x, y, z, FACE_EAST);
        self.tesselate_east_face(x as f64, y as f64, z as f64, sprite);
        self.tesselator.add_offset(inset, 0.0, 0.0);
        true
    }

    // Torches -----------------------------------------------------------

    /// `tesselateTorch(Block, x, y, z)`: pick the wall offset from metadata.
    fn tesselate_torch(&mut self, block: &BlockDef, x: i32, y: i32, z: i32) -> bool {
        let metadata = self.world.metadata(x, y, z);
        self.tesselator
            .brightness(block.light_color(self.world, x, y, z));
        self.tesselator.color(1.0, 1.0, 1.0);
        let offset = 0.4_f32 as f64;
        let shift = 0.5 - offset;
        let lift = 0.2_f32 as f64;
        let (px, py, pz, ox, oz) = match metadata {
            1 => (x as f64 - shift, y as f64 + lift, z as f64, -offset, 0.0),
            2 => (x as f64 + shift, y as f64 + lift, z as f64, offset, 0.0),
            3 => (x as f64, y as f64 + lift, z as f64 - shift, 0.0, -offset),
            4 => (x as f64, y as f64 + lift, z as f64 + shift, 0.0, offset),
            _ => (x as f64, y as f64, z as f64, 0.0, 0.0),
        };
        let sprite = self.sprite_for(block, x, y, z, 0);
        self.tesselate_torch_at(sprite, px, py, pz, ox, oz);
        true
    }

    /// `tesselateTorch(Block, double, double, double, double, double, int)`:
    /// the 24 torch vertices.
    fn tesselate_torch_at(
        &mut self,
        sprite: Sprite,
        x: f64,
        y: f64,
        z: f64,
        offset_x: f64,
        offset_z: f64,
    ) {
        let mut sprite = sprite;
        if let Some(forced) = self.forced_sprite {
            sprite = forced;
        }
        self.tesselator.layer(sprite.layer);
        let u_min = sprite.u_min();
        let v_min = sprite.v_min();
        let u_max = sprite.u_max();
        let v_max = sprite.v_max();
        let u7 = sprite.get_u(7.0);
        let v6 = sprite.get_v(6.0);
        let u9 = sprite.get_u(9.0);
        let v8 = sprite.get_v(8.0);
        let v13 = sprite.get_v(13.0);
        let v15 = sprite.get_v(15.0);
        let x = x + 0.5;
        let z = z + 0.5;
        let x0 = x - 0.5;
        let x1 = x + 0.5;
        let z0 = z - 0.5;
        let z1 = z + 0.5;
        let half = 0.0625_f64;
        let top = 0.625_f64;
        let t = &mut self.tesselator;
        t.vertex(
            x + offset_x * (1.0 - top) - half,
            y + top,
            z + offset_z * (1.0 - top) - half,
            u7,
            v6,
        );
        t.vertex(
            x + offset_x * (1.0 - top) - half,
            y + top,
            z + offset_z * (1.0 - top) + half,
            u7,
            v8,
        );
        t.vertex(
            x + offset_x * (1.0 - top) + half,
            y + top,
            z + offset_z * (1.0 - top) + half,
            u9,
            v8,
        );
        t.vertex(
            x + offset_x * (1.0 - top) + half,
            y + top,
            z + offset_z * (1.0 - top) - half,
            u9,
            v6,
        );
        t.vertex(x + half + offset_x, y, z - half + offset_z, u9, v13);
        t.vertex(x + half + offset_x, y, z + half + offset_z, u9, v15);
        t.vertex(x - half + offset_x, y, z + half + offset_z, u7, v15);
        t.vertex(x - half + offset_x, y, z - half + offset_z, u7, v13);
        t.vertex(x - half, y + 1.0, z0, u_min, v_min);
        t.vertex(x - half + offset_x, y, z0 + offset_z, u_min, v_max);
        t.vertex(x - half + offset_x, y, z1 + offset_z, u_max, v_max);
        t.vertex(x - half, y + 1.0, z1, u_max, v_min);
        t.vertex(x + half, y + 1.0, z1, u_min, v_min);
        t.vertex(x + offset_x + half, y, z1 + offset_z, u_min, v_max);
        t.vertex(x + offset_x + half, y, z0 + offset_z, u_max, v_max);
        t.vertex(x + half, y + 1.0, z0, u_max, v_min);
        t.vertex(x0, y + 1.0, z + half, u_min, v_min);
        t.vertex(x0 + offset_x, y, z + half + offset_z, u_min, v_max);
        t.vertex(x1 + offset_x, y, z + half + offset_z, u_max, v_max);
        t.vertex(x1, y + 1.0, z + half, u_max, v_min);
        t.vertex(x1, y + 1.0, z - half, u_min, v_min);
        t.vertex(x1 + offset_x, y, z - half + offset_z, u_min, v_max);
        t.vertex(x0 + offset_x, y, z - half + offset_z, u_max, v_max);
        t.vertex(x0, y + 1.0, z - half, u_max, v_min);
    }

    // Liquids -----------------------------------------------------------

    /// `LiquidBlock.getHeightLoss(state)`: water column height per level.
    pub fn liquid_height_loss(mut state: i32) -> f32 {
        if state >= 8 {
            state = 0;
        }
        (state + 1) as f32 / 9.0
    }

    /// `LiquidBlock.getFlowAngle(world, x, y, z, material)`: the flow
    /// direction of the liquid column, or -1000 when it does not flow.
    pub fn liquid_flow_angle(world: &W, x: i32, y: i32, z: i32, material: Material) -> f32 {
        let flow = Self::liquid_flow(world, x, y, z, material);
        if flow[0] == 0.0 && flow[2] == 0.0 {
            -1000.0
        } else {
            (flow[2].atan2(flow[0]) - (std::f64::consts::PI / 2.0)) as f32
        }
    }

    /// `LiquidBlock.getFlow`: the normalized flow vector of the column.
    pub fn liquid_flow(world: &W, x: i32, y: i32, z: i32, material: Material) -> [f64; 3] {
        let mut flow = [0.0_f64; 3];
        let here = Self::liquid_depth(world, x, y, z, material);
        for neighbor in 0..4_i32 {
            let (mut nx, mut nz) = (x, z);
            if neighbor == 0 {
                nx -= 1;
            }
            if neighbor == 1 {
                nz -= 1;
            }
            if neighbor == 2 {
                nx += 1;
            }
            if neighbor == 3 {
                nz += 1;
            }
            let depth = Self::liquid_depth(world, nx, y, nz, material);
            if depth < 0 {
                if !world.block(nx, y, nz).material.blocks_movement {
                    let below = Self::liquid_depth(world, nx, y - 1, nz, material);
                    if below >= 0 {
                        let delta = below - (here - 8);
                        flow[0] += ((nx - x) * delta) as f64;
                        flow[1] += 0.0;
                        flow[2] += ((nz - z) * delta) as f64;
                    }
                }
            } else {
                let delta = depth - here;
                flow[0] += ((nx - x) * delta) as f64;
                flow[1] += 0.0;
                flow[2] += ((nz - z) * delta) as f64;
            }
        }

        // Falling liquid pulls down when boxed in by solid faces.
        if world.metadata(x, y, z) >= 8 {
            let mut boxed = false;
            for &(dx, dy, dz) in &[
                (0, 0, -1),
                (0, 0, 1),
                (-1, 0, 0),
                (1, 0, 0),
                (0, 1, -1),
                (0, 1, 1),
                (-1, 1, 0),
                (1, 1, 0),
            ] {
                if !boxed && world.block(x + dx, y + dy, z + dz).material.is_solid() {
                    boxed = true;
                }
            }
            if boxed {
                let length = (flow[0] * flow[0] + flow[1] * flow[1] + flow[2] * flow[2]).sqrt();
                if length > 0.0 {
                    flow[0] /= length;
                    flow[1] /= length;
                    flow[2] /= length;
                }
                flow[1] -= 6.0;
            }
        }

        let length = (flow[0] * flow[0] + flow[1] * flow[1] + flow[2] * flow[2]).sqrt();
        if length > 0.0 {
            flow[0] /= length;
            flow[1] /= length;
            flow[2] /= length;
        }
        flow
    }

    /// `LiquidBlock.getLiquidDepth`.
    fn liquid_depth(world: &W, x: i32, y: i32, z: i32, material: Material) -> i32 {
        if world.block(x, y, z).material != material {
            return -1;
        }
        let mut depth = world.metadata(x, y, z);
        if depth >= 8 {
            depth = 0;
        }
        depth
    }

    /// `getLiquidHeight`: the smoothed height of the liquid surface corner.
    fn liquid_height(&self, x: i32, y: i32, z: i32, material: Material) -> f32 {
        let mut weight = 0;
        let mut total = 0.0_f32;
        for corner in 0..4_i32 {
            let cx = x - (corner & 1);
            let cz = z - (corner >> 1 & 1);
            if self.world.block(cx, y + 1, cz).material == material {
                return 1.0;
            }
            let material_here = self.world.block(cx, y, cz).material;
            if material_here == material {
                let metadata = self.world.metadata(cx, y, cz);
                if metadata >= 8 || metadata == 0 {
                    total += Self::liquid_height_loss(metadata) * 10.0;
                    weight += 10;
                }
                total += Self::liquid_height_loss(metadata);
                weight += 1;
            } else if !material_here.is_solid() {
                total += 1.0;
                weight += 1;
            }
        }
        1.0 - total / weight as f32
    }

    /// `tesselateLiquid`: the water/lava renderer (render type 4).
    fn tesselate_liquid(&mut self, block: &BlockDef, x: i32, y: i32, z: i32) -> bool {
        let color = block.color(self.world.metadata(x, y, z) as u8, self.world, x, y, z);
        let r = (color >> 16 & 0xFF) as f32 / 255.0;
        let g = (color >> 8 & 0xFF) as f32 / 255.0;
        let b = (color & 0xFF) as f32 / 255.0;
        let render_top = self.should_render_face(block, x, y + 1, z, FACE_TOP);
        let render_bottom = self.should_render_face(block, x, y - 1, z, FACE_BOTTOM);
        let side_face = [
            self.should_render_face(block, x, y, z - 1, FACE_NORTH),
            self.should_render_face(block, x, y, z + 1, FACE_SOUTH),
            self.should_render_face(block, x - 1, y, z, FACE_WEST),
            self.should_render_face(block, x + 1, y, z, FACE_EAST),
        ];
        if !render_top
            && !render_bottom
            && !side_face[0]
            && !side_face[1]
            && !side_face[2]
            && !side_face[3]
        {
            return false;
        }

        let mut rendered = false;
        let side = 0.5_f32;
        let top = 1.0_f32;
        let z_side = 0.8_f32;
        let x_side = 0.6_f32;
        let material = block.material;
        let _metadata = self.world.metadata(x, y, z);
        let mut height_x0z0 = self.liquid_height(x, y, z, material) as f64;
        let mut height_x0z1 = self.liquid_height(x, y, z + 1, material) as f64;
        let mut height_x1z1 = self.liquid_height(x + 1, y, z + 1, material) as f64;
        let mut height_x1z0 = self.liquid_height(x + 1, y, z, material) as f64;
        let epsilon = 0.001_f64;

        if self.render_face_always || render_top {
            rendered = true;
            let mut sprite = self.sprite_for(block, x, y, z, 1);
            let flow_angle = Self::liquid_flow_angle(self.world, x, y, z, material);
            if flow_angle > -999.0 {
                sprite = self.sprite_for(block, x, y, z, 2);
            }
            self.tesselator.layer(sprite.layer);
            height_x0z0 -= epsilon;
            height_x0z1 -= epsilon;
            height_x1z1 -= epsilon;
            height_x1z0 -= epsilon;
            let (u0, v0, u1, v1, u2, v2, u3, v3);
            if flow_angle < -999.0 {
                u0 = sprite.get_u(0.0);
                v0 = sprite.get_v(0.0);
                u1 = u0;
                v1 = sprite.get_v(16.0);
                u2 = sprite.get_u(16.0);
                v2 = v1;
                u3 = u2;
                v3 = v0;
            } else {
                let sin = flow_angle.sin() * 0.25;
                let cos = flow_angle.cos() * 0.25;
                u0 = sprite.get_u((8.0 + (-cos - sin) * 16.0) as f64);
                v0 = sprite.get_v((8.0 + (-cos + sin) * 16.0) as f64);
                u1 = sprite.get_u((8.0 + (-cos + sin) * 16.0) as f64);
                v1 = sprite.get_v((8.0 + (cos + sin) * 16.0) as f64);
                u2 = sprite.get_u((8.0 + (cos + sin) * 16.0) as f64);
                v2 = sprite.get_v((8.0 + (cos - sin) * 16.0) as f64);
                u3 = sprite.get_u((8.0 + (cos - sin) * 16.0) as f64);
                v3 = sprite.get_v((8.0 + (-cos - sin) * 16.0) as f64);
            }

            self.tesselator
                .brightness(block.light_color(self.world, x, y, z));
            self.tesselator.color(top * r, top * g, top * b);
            let t = &mut self.tesselator;
            let (xf, zf) = (x as f64, z as f64);
            let (y0, y1, y2, y3) = (
                y as f64 + height_x0z0,
                y as f64 + height_x0z1,
                y as f64 + height_x1z1,
                y as f64 + height_x1z0,
            );
            t.vertex(xf, y0, zf, u0, v0);
            t.vertex(xf, y1, zf + 1.0, u1, v1);
            t.vertex(xf + 1.0, y2, zf + 1.0, u2, v2);
            t.vertex(xf + 1.0, y3, zf, u3, v3);
            t.vertex(xf, y0, zf, u0, v0);
            t.vertex(xf + 1.0, y3, zf, u3, v3);
            t.vertex(xf + 1.0, y2, zf + 1.0, u2, v2);
            t.vertex(xf, y1, zf + 1.0, u1, v1);
        }

        if self.render_face_always || render_bottom {
            self.tesselator
                .brightness(block.light_color(self.world, x, y - 1, z));
            self.tesselator.color(side, side, side);
            self.tesselate_bottom_face(
                x as f64,
                y as f64 + epsilon,
                z as f64,
                self.sprite_for(block, x, y, z, 0),
            );
            rendered = true;
        }

        for side_index in 0..4_usize {
            let (nx, nz) = match side_index {
                0 => (x, z - 1),
                1 => (x, z + 1),
                2 => (x - 1, z),
                _ => (x + 1, z),
            };
            let sprite = self.sprite_for(block, x, y, z, side_index as i32 + 2);
            self.tesselator.layer(sprite.layer);
            if self.render_face_always || side_face[side_index] {
                let (h0, h1, vx0, vx1, vz0, vz1);
                match side_index {
                    0 => {
                        h0 = height_x0z0;
                        h1 = height_x1z0;
                        vx0 = x as f64;
                        vx1 = x as f64 + 1.0;
                        vz0 = z as f64 + epsilon;
                        vz1 = z as f64 + epsilon;
                    }
                    1 => {
                        h0 = height_x1z1;
                        h1 = height_x0z1;
                        vx0 = x as f64 + 1.0;
                        vx1 = x as f64;
                        vz0 = z as f64 + 1.0 - epsilon;
                        vz1 = z as f64 + 1.0 - epsilon;
                    }
                    2 => {
                        h0 = height_x0z1;
                        h1 = height_x0z0;
                        vx0 = x as f64 + epsilon;
                        vx1 = x as f64 + epsilon;
                        vz0 = z as f64 + 1.0;
                        vz1 = z as f64;
                    }
                    _ => {
                        h0 = height_x1z0;
                        h1 = height_x1z1;
                        vx0 = x as f64 + 1.0 - epsilon;
                        vx1 = x as f64 + 1.0 - epsilon;
                        vz0 = z as f64;
                        vz1 = z as f64 + 1.0;
                    }
                }

                rendered = true;
                let u0 = sprite.get_u(0.0);
                let u1 = sprite.get_u(8.0);
                let v0 = sprite.get_v((1.0 - h0) * 16.0 * 0.5);
                let v1 = sprite.get_v((1.0 - h1) * 16.0 * 0.5);
                let v_bottom = sprite.get_v(8.0);
                self.tesselator
                    .brightness(block.light_color(self.world, nx, y, nz));
                let mut face_color = 1.0_f32;
                face_color *= if side_index < 2 { z_side } else { x_side };
                self.tesselator.color(
                    top * face_color * r,
                    top * face_color * g,
                    top * face_color * b,
                );
                let t = &mut self.tesselator;
                let (yf0, yf1) = (y as f64 + h0, y as f64 + h1);
                t.vertex(vx0, yf0, vz0, u0, v0);
                t.vertex(vx1, yf1, vz1, u1, v1);
                t.vertex(vx1, y as f64, vz1, u1, v_bottom);
                t.vertex(vx0, y as f64, vz0, u0, v_bottom);
                t.vertex(vx0, y as f64, vz0, u0, v_bottom);
                t.vertex(vx1, y as f64, vz1, u1, v_bottom);
                t.vertex(vx1, yf1, vz1, u1, v1);
                t.vertex(vx0, yf0, vz0, u0, v0);
            }
        }

        // The liquid render leaves the bounds at the full cube, exactly
        // like vanilla's `this.minY = 0.0; this.maxY = 1.0`.
        self.prepare_bounds(Bounds::FULL);
        rendered
    }

    // Stairs and fences -------------------------------------------------

    /// `tesselateStairs`: the base slab, the corner shape and, for inner
    /// corners, the step shape.
    fn tesselate_stairs(&mut self, block: &BlockDef, x: i32, y: i32, z: i32) -> bool {
        let base = stairs::base_shape(self.world, x, y, z);
        self.prepare_bounds(base);
        self.tesselate_block(block, x, y, z);
        self.mirror_uv = true;
        let (inner, is_inner) = stairs::inner_shape(self.world, x, y, z);
        self.prepare_bounds(inner);
        self.tesselate_block(block, x, y, z);
        if is_inner && let Some(step) = stairs::step_shape(self.world, x, y, z) {
            self.prepare_bounds(step);
            self.tesselate_block(block, x, y, z);
        }
        self.mirror_uv = false;
        true
    }

    /// `tesselateFence`: the post plus the rails of each connected side.
    fn tesselate_fence(&mut self, block: &BlockDef, x: i32, y: i32, z: i32) -> bool {
        let mut rendered;
        let post_x = 0.375_f64;
        let post_max = 0.625_f64;
        self.prepare_bounds(Bounds::new(post_x, 0.0, post_x, post_max, 1.0, post_max));
        self.tesselate_block(block, x, y, z);
        rendered = true;

        let connect_west = Self::fence_connects(self.world, x - 1, y, z);
        let connect_east = Self::fence_connects(self.world, x + 1, y, z);
        let connect_north = Self::fence_connects(self.world, x, y, z - 1);
        let connect_south = Self::fence_connects(self.world, x, y, z + 1);
        let mut along_x = connect_west || connect_east;
        let along_z = connect_north || connect_south;
        if !along_x && !along_z {
            along_x = true;
        }

        let rail_min = 0.4375_f64;
        let rail_max = 0.5625_f64;
        let west_edge = if connect_west { 0.0 } else { rail_min };
        let east_edge = if connect_east { 1.0 } else { rail_max };
        let north_edge = if connect_north { 0.0 } else { rail_min };
        let south_edge = if connect_south { 1.0 } else { rail_max };

        self.mirror_uv = true;
        // The two upper rails.
        let upper_min = 0.75_f64;
        let upper_max = 0.9375_f64;
        if along_x {
            self.prepare_bounds(Bounds::new(
                west_edge, upper_min, rail_min, east_edge, upper_max, rail_max,
            ));
            self.tesselate_block(block, x, y, z);
            rendered = true;
        }
        if along_z {
            self.prepare_bounds(Bounds::new(
                rail_min, upper_min, north_edge, rail_max, upper_max, south_edge,
            ));
            self.tesselate_block(block, x, y, z);
            rendered = true;
        }

        // The two lower rails.
        let lower_min = 0.375_f64;
        let lower_max = 0.5625_f64;
        if along_x {
            self.prepare_bounds(Bounds::new(
                west_edge, lower_min, rail_min, east_edge, lower_max, rail_max,
            ));
            self.tesselate_block(block, x, y, z);
            rendered = true;
        }
        if along_z {
            self.prepare_bounds(Bounds::new(
                rail_min, lower_min, north_edge, rail_max, lower_max, south_edge,
            ));
            self.tesselate_block(block, x, y, z);
            rendered = true;
        }
        self.mirror_uv = false;

        // Restore the collision shape like `block.updateShape` does.
        self.prepare_bounds(Bounds::FULL);
        rendered
    }

    /// `FenceBlock.shouldConnectTo`.
    pub fn fence_connects(world: &W, x: i32, y: i32, z: i32) -> bool {
        let block = world.block(x, y, z);
        if block.connects_to_fence {
            true
        } else {
            block.material.is_solid_blocking()
                && block.is_cube()
                && block.material != Material::PUMPKIN
        }
    }
}

/// `Directions.X/Y/Z_OFFSET` of a face.
fn face_offset(face: i32) -> (i32, i32, i32) {
    match face {
        FACE_BOTTOM => (0, -1, 0),
        FACE_TOP => (0, 1, 0),
        FACE_NORTH => (0, 0, -1),
        FACE_SOUTH => (0, 0, 1),
        FACE_WEST => (-1, 0, 0),
        _ => (1, 0, 0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesher::sunset17::fixture::{self, TestWorld};

    fn render(world: &TestWorld, at: [i32; 3], fancy: bool) -> usize {
        let [x, y, z] = at;
        let def = world.state(x, y, z).block;
        let mut renderer = RenderBlocks::new(
            world,
            RenderOptions {
                ao: 0,
                fancy_graphics: fancy,
                anaglyph: false,
            },
        );
        assert!(renderer.tesselate_in_world(&def, x, y, z));
        renderer.tesselator.vertices.len()
    }

    #[test]
    fn glass_hides_only_the_faces_its_own_kind_shares() {
        let mut world = TestWorld::new();
        world.set(4, 4, 4, fixture::id("glass") as u8, 0);
        // Six faces of one glass block.
        assert_eq!(render(&world, [4, 4, 4], false), 6 * 4);
        // Same-metadata glass hides the shared face.
        world.set(5, 4, 4, fixture::id("glass") as u8, 0);
        assert_eq!(render(&world, [4, 4, 4], false), 5 * 4);
        // A different transparent block is not gold, so the face stays.
        world.set(5, 4, 4, fixture::id("stained_glass") as u8, 0);
        assert_eq!(render(&world, [4, 4, 4], false), 6 * 4);
        // An opaque neighbour hides the face like any solid block.
        world.set(5, 4, 4, fixture::id("stone") as u8, 0);
        assert_eq!(render(&world, [4, 4, 4], false), 5 * 4);
    }

    #[test]
    fn fast_graphics_hides_the_face_between_two_leaf_blocks() {
        let mut world = TestWorld::new();
        world.set(4, 4, 4, fixture::id("leaves") as u8, 0);
        world.set(5, 4, 4, fixture::id("leaves") as u8, 0);
        // Fancy graphics draws every leaf face; fast graphics hides the
        // shared one.
        assert_eq!(render(&world, [4, 4, 4], true), 6 * 4);
        assert_eq!(render(&world, [4, 4, 4], false), 5 * 4);
    }

    #[test]
    fn ice_hides_the_face_between_two_ice_blocks() {
        let mut world = TestWorld::new();
        world.set(4, 4, 4, fixture::id("ice") as u8, 0);
        world.set(5, 4, 4, fixture::id("ice") as u8, 0);
        assert_eq!(render(&world, [4, 4, 4], false), 5 * 4);
    }

    #[test]
    fn a_cactus_insets_its_side_faces() {
        let mut world = TestWorld::new();
        world.set(4, 4, 4, fixture::id("cactus") as u8, 0);
        let mut renderer = RenderBlocks::new(
            &world,
            RenderOptions {
                ao: 0,
                fancy_graphics: false,
                anaglyph: false,
            },
        );
        let def = world.state(4, 4, 4).block;
        assert!(renderer.tesselate_in_world(&def, 4, 4, 4));
        // The south face of the cactus sits 1/16 inside the cell.
        let zs: Vec<f32> = renderer
            .tesselator
            .vertices
            .iter()
            .map(|vertex| vertex.z)
            .collect();
        assert!(zs.contains(&4.9375), "{zs:?}");
    }
}
