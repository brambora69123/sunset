//! The render-side block descriptor, ported from the render-relevant half of
//! `net.minecraft.block.Block` (shape, face culling, light and sprites).
//!
//! `tesselateInWorld` calls `block.updateShape` before every render; vanilla
//! mutates the singleton block in place, here the shape comes back from
//! `shape` instead. `getSprite`, `getColor` and the light color are ports of
//! the block getters the renderer reads.

use crate::material::Material;
use crate::sprite::Sprite;
use crate::world::WorldView;

/// Face order of vanilla: 0 bottom (-Y), 1 top (+Y), 2 north (-Z),
/// 3 south (+Z), 4 west (-X), 5 east (+X).
pub const FACE_BOTTOM: i32 = 0;
pub const FACE_TOP: i32 = 1;
pub const FACE_NORTH: i32 = 2;
pub const FACE_SOUTH: i32 = 3;
pub const FACE_WEST: i32 = 4;
pub const FACE_EAST: i32 = 5;

/// Render types of `Block.getRenderType()` this crate renders.
pub const RENDER_TYPE_STANDARD: i32 = 0;
pub const RENDER_TYPE_CROSS: i32 = 1;
pub const RENDER_TYPE_TORCH: i32 = 2;
pub const RENDER_TYPE_LIQUID: i32 = 4;
pub const RENDER_TYPE_STAIRS: i32 = 10;
pub const RENDER_TYPE_FENCE: i32 = 11;
pub const RENDER_TYPE_AXIS: i32 = 31;

/// Which block shape the renderer is drawing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// A full cube (`renderStandardBlock`).
    Cube,
    /// Grass: a cube whose side overlay pass carries the biome tint.
    Grass,
    /// A log: `tesselateAxisBlock`, the top sprite on the axis faces.
    Log,
    /// Glass: `TransparentBlock`, culled against same-metadata glass.
    Glass,
    /// A single or double slab; metadata bit 8 picks the top half.
    Slab,
    /// A snow layer; metadata 0..7 picks the height.
    Snow,
    Stairs,
    Fence,
    /// Crossed squares (render types 1 and 6) with vanilla's random offset.
    Cross(CrossKind),
    /// Water and lava (render type 4).
    Liquid,
    /// A torch (render type 2).
    Torch,
    /// Air: nothing renders and it lets light through.
    Air,
}

impl Kind {
    /// `Block.isCube()`: the flag that decides ambient occlusion darkness.
    pub fn is_cube(self) -> bool {
        matches!(self, Kind::Cube | Kind::Grass | Kind::Log)
    }

    /// `Block.isSolidRender()`: whether a neighbouring face is culled.
    pub fn is_solid_render(self) -> bool {
        matches!(self, Kind::Cube | Kind::Grass | Kind::Log)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CrossKind {
    /// Tallgrass: the 0.5 x/z and 0.2 y offset hash.
    TallGrass,
    /// Flowers: the 0.3 x/z offset hash.
    Flower,
    /// Everything else crosses at the block centre.
    Plain,
}

/// The prepared bounds of one render pass.
#[derive(Clone, Copy, Debug)]
pub struct Bounds {
    pub min_x: f64,
    pub min_y: f64,
    pub min_z: f64,
    pub max_x: f64,
    pub max_y: f64,
    pub max_z: f64,
}

impl Bounds {
    pub const FULL: Bounds = Bounds {
        min_x: 0.0,
        min_y: 0.0,
        min_z: 0.0,
        max_x: 1.0,
        max_y: 1.0,
        max_z: 1.0,
    };

    pub const fn new(
        min_x: f64,
        min_y: f64,
        min_z: f64,
        max_x: f64,
        max_y: f64,
        max_z: f64,
    ) -> Bounds {
        Bounds {
            min_x,
            min_y,
            min_z,
            max_x,
            max_y,
            max_z,
        }
    }

    /// `Minecraft.isAmbientOcclusionEnabled()`'s full-cube check in
    /// `RenderBlocks.prepare`: whether any face is inside the block cell.
    pub fn is_full_cube(&self) -> bool {
        self.min_x == 0.0
            && self.max_x == 1.0
            && self.min_y == 0.0
            && self.max_y == 1.0
            && self.min_z == 0.0
            && self.max_z == 1.0
    }
}

/// Per-face sprite selection, ported from the `getSprite` overrides.
#[derive(Clone, Copy, Debug)]
pub enum Sprites {
    Single(Sprite),
    /// Grass: dirt bottom and sides, grass top, tinted side overlay.
    Grass {
        top: Sprite,
        side: Sprite,
        side_overlay: Sprite,
        bottom: Sprite,
    },
    /// One sprite per face (grass without an overlay pass, ...).
    PerFace([Sprite; 6]),
    /// A log: the top sprite on the axis faces, the side sprite elsewhere.
    Log {
        side: Sprite,
        top: Sprite,
    },
    /// A stone slab: the top sprite on the horizontal faces.
    Slab {
        top: Sprite,
        side: Sprite,
    },
    /// A liquid: the still sprite on the horizontal faces, the flow sprite
    /// on the sides (`LiquidBlock.getSprite`).
    Liquid {
        still: Sprite,
        flow: Sprite,
    },
}

impl Sprites {
    /// `AxisBlock.getSprite(face, metadata)`'s sprite choice.
    fn log_sprite(side: Sprite, top: Sprite, metadata: i32, face: i32) -> Sprite {
        let axis = metadata & 12;
        let top_face = (axis == 0 && (face == 0 || face == 1))
            || (axis == 4 && (face == 4 || face == 5))
            || (axis == 8 && (face == 2 || face == 3));
        if top_face { top } else { side }
    }

    pub fn sprite(&self, metadata: i32, face: i32) -> Sprite {
        match self {
            Sprites::Single(sprite) => *sprite,
            Sprites::Grass {
                top, side, bottom, ..
            } => match face {
                FACE_TOP => *top,
                FACE_BOTTOM => *bottom,
                _ => *side,
            },
            Sprites::PerFace(faces) => faces[face as usize],
            Sprites::Log { side, top } => Self::log_sprite(*side, *top, metadata, face),
            Sprites::Slab { top, side } => {
                if face == FACE_TOP || face == FACE_BOTTOM {
                    *top
                } else {
                    *side
                }
            }
            Sprites::Liquid { still, flow } => {
                if face == FACE_TOP || face == FACE_BOTTOM {
                    *still
                } else {
                    *flow
                }
            }
        }
    }

    pub fn top(&self) -> Sprite {
        self.sprite(0, FACE_TOP)
    }

    pub fn side_overlay(&self) -> Option<Sprite> {
        match self {
            Sprites::Grass { side_overlay, .. } => Some(*side_overlay),
            _ => None,
        }
    }
}

/// One renderable block.
#[derive(Clone, Debug)]
pub struct BlockDef {
    pub name: &'static str,
    pub kind: Kind,
    pub material: Material,
    /// `Block.getLight()`: the light this block emits, 0..15.
    pub light: i32,
    /// `Block.isTranslucent`: `!material.isOpaque()`, which the smooth
    /// lighting uses to decide whether a corner sample is halved.
    pub translucent: bool,
    /// `Block.usesNeighborLight`: the light sampler takes the maximum of the
    /// block above and the four horizontal neighbours. `Block.init` sets it
    /// for stairs, slabs, tall grass and every block of opacity 0.
    pub use_neighbor_light: bool,
    pub sprites: Sprites,
    /// A fixed color multiplier from `Block.getColor`, 0xRRGGBB.
    pub tint: i32,
    /// The bounds a `Cube` renders with (`Block.setShape`). `FULL` unless
    /// the block is not a full cube.
    pub bounds: Bounds,
}

impl BlockDef {
    pub fn new(name: &'static str, kind: Kind, material: Material, sprites: Sprites) -> BlockDef {
        // `Block.init`'s `useNeighborLight`: stairs, slabs, tall grass and
        // every block whose opacity ended up 0. Liquids set opacity 3.
        let opacity_zero = matches!(
            kind,
            Kind::Glass | Kind::Torch | Kind::Fence | Kind::Cross(_) | Kind::Snow
        );
        let translucent = !material.is_opaque();
        BlockDef {
            name,
            kind,
            material,
            light: 0,
            translucent,
            use_neighbor_light: !material.air
                && (matches!(kind, Kind::Stairs | Kind::Slab) || translucent || opacity_zero),
            sprites,
            tint: 0xFFFFFF,
            bounds: Bounds::FULL,
        }
    }

    pub fn with_light(mut self, light: i32) -> BlockDef {
        self.light = light;
        self
    }

    pub fn with_tint(mut self, tint: i32) -> BlockDef {
        self.tint = tint;
        self
    }

    pub fn with_translucent(mut self, translucent: bool) -> BlockDef {
        self.translucent = translucent;
        if translucent {
            self.use_neighbor_light = true;
        }
        self
    }

    pub fn render_type(&self) -> i32 {
        match self.kind {
            Kind::Cube | Kind::Grass | Kind::Glass | Kind::Slab | Kind::Snow => {
                RENDER_TYPE_STANDARD
            }
            Kind::Log => RENDER_TYPE_AXIS,
            Kind::Stairs => RENDER_TYPE_STAIRS,
            Kind::Fence => RENDER_TYPE_FENCE,
            Kind::Cross(_) => RENDER_TYPE_CROSS,
            Kind::Liquid => RENDER_TYPE_LIQUID,
            Kind::Torch => RENDER_TYPE_TORCH,
            Kind::Air => -1,
        }
    }

    /// `Block.isCube()`: a full cube of an opaque material. Slabs, stairs and
    /// fences render as normal blocks but are not normal cubes.
    pub fn is_cube(&self) -> bool {
        self.kind.is_cube()
    }

    /// `Block.isSolidRender()`.
    pub fn is_solid_render(&self) -> bool {
        self.kind.is_solid_render()
    }

    /// `Block.blocksAmbientLight()`: `material.blocksMovement() && isCube()`.
    pub fn blocks_ambient_light(&self) -> bool {
        self.material.blocks_movement && self.is_cube()
    }

    /// `Block.getAmbientOcclusionLight()`.
    pub fn ambient_occlusion_light(&self) -> f32 {
        if self.blocks_ambient_light() {
            0.2
        } else {
            1.0
        }
    }

    /// `Block.getLightColor(world, x, y, z)`, including the single-slab
    /// light leak down a block when the slab sits in darkness, and the
    /// liquid override that takes the brighter of the block and the block
    /// above.
    pub fn light_color(&self, world: &dyn WorldView, x: i32, y: i32, z: i32) -> i32 {
        if self.kind == Kind::Liquid {
            // `LiquidBlock.getLightColor`: per byte, the brighter of this
            // block and the one above.
            let below = world.light_color(x, y, z, 0);
            let above = world.light_color(x, y + 1, z, 0);
            let block = (below & 0xFF).max(above & 0xFF);
            let sky = (below >> 16 & 0xFF).max(above >> 16 & 0xFF);
            return block | sky << 16;
        }
        let color = world.light_color(x, y, z, self.light);
        if color == 0 && self.kind == Kind::Slab {
            let below = world.block(x, y - 1, z);
            world.light_color(x, y - 1, z, below.light)
        } else {
            color
        }
    }

    /// `Block.getColor(world, x, y, z)`.
    pub fn color(&self, world: &dyn WorldView, x: i32, _y: i32, z: i32) -> i32 {
        if self.kind == Kind::Grass {
            world.biome_grass_tint(x, z)
        } else {
            self.tint
        }
    }

    /// `Block.updateShape` + the bounds `RenderBlocks.prepare` reads.
    pub fn shape(&self, world: &dyn WorldView, x: i32, y: i32, z: i32) -> Bounds {
        match self.kind {
            Kind::Slab => {
                if world.metadata(x, y, z) & 8 != 0 {
                    Bounds::new(0.0, 0.5, 0.0, 1.0, 1.0, 1.0)
                } else {
                    Bounds::new(0.0, 0.0, 0.0, 1.0, 0.5, 1.0)
                }
            }
            Kind::Snow => {
                let layers = world.metadata(x, y, z) & 7;
                let height = (2 * (1 + layers)) as f32 / 16.0;
                Bounds::new(0.0, 0.0, 0.0, 1.0, height as f64, 1.0)
            }
            _ => self.bounds,
        }
    }

    /// `Block.shouldRenderFace(world, x, y, z, face)` for the neighbour at
    /// `(x, y, z)`; the glass and liquid overrides live in the renderer,
    /// which knows the block's own position.
    pub fn should_render_face(
        &self,
        world: &dyn WorldView,
        x: i32,
        y: i32,
        z: i32,
        face: i32,
        bounds: &Bounds,
    ) -> bool {
        if matches!(self.kind, Kind::Fence) || (self.kind == Kind::Snow && face == FACE_TOP) {
            return true;
        }
        let bounds_force = match face {
            FACE_BOTTOM => bounds.min_y > 0.0,
            FACE_TOP => bounds.max_y < 1.0,
            FACE_NORTH => bounds.min_z > 0.0,
            FACE_SOUTH => bounds.max_z < 1.0,
            FACE_WEST => bounds.min_x > 0.0,
            _ => bounds.max_x < 1.0,
        };
        bounds_force || !world.block(x, y, z).is_solid_render()
    }

    /// `SlabBlock.shouldRenderFace`. The caller passes the position this
    /// face looks at (the neighbour), like `tesselateInWorld` does.
    pub fn slab_should_render_face(
        &self,
        world: &dyn WorldView,
        x: i32,
        y: i32,
        z: i32,
        face: i32,
    ) -> bool {
        // The slab's own position and shape, reached from the neighbour by
        // the face's opposite offset.
        let (ox, oy, oz) = opposite_offset(face);
        let own = self.shape(world, x + ox, y + oy, z + oz);
        let own_top = world.metadata(x + ox, y + oy, z + oz) & 8 != 0;
        let neighbour = world.block(x, y, z);
        let neighbour_top = world.metadata(x, y, z) & 8 != 0;
        if face != FACE_TOP
            && face != FACE_BOTTOM
            && !self.should_render_face(world, x, y, z, face, &own)
        {
            return false;
        }
        if own_top {
            if face == FACE_BOTTOM {
                return true;
            }
            if face == FACE_TOP && self.should_render_face(world, x, y, z, face, &own) {
                return true;
            }
            return !(neighbour.kind == Kind::Slab) || !neighbour_top;
        }
        if face == FACE_TOP {
            return true;
        }
        if face == FACE_BOTTOM && self.should_render_face(world, x, y, z, face, &own) {
            return true;
        }
        !(neighbour.kind == Kind::Slab) || neighbour_top
    }
}

/// `Directions.X/Y/Z_OFFSET` of the opposite face.
pub fn opposite_offset(face: i32) -> (i32, i32, i32) {
    match face {
        FACE_BOTTOM => (0, -1, 0),
        FACE_TOP => (0, 1, 0),
        FACE_NORTH => (0, 0, 1),
        FACE_SOUTH => (0, 0, -1),
        FACE_WEST => (1, 0, 0),
        _ => (-1, 0, 0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(name: &'static str, kind: Kind, material: Material) -> BlockDef {
        BlockDef::new(name, kind, material, Sprites::Single(Sprite::unit("test")))
    }

    #[test]
    fn air_renders_nothing() {
        let air = block("air", Kind::Air, Material::AIR);
        assert_eq!(air.render_type(), -1);
        assert!(!air.is_solid_render());
        assert!(!air.is_cube());
    }

    #[test]
    fn ambient_occlusion_light_follows_material_and_cube() {
        let stone = block("stone", Kind::Cube, Material::STONE);
        assert_eq!(stone.ambient_occlusion_light(), 0.2);

        let air = block("air", Kind::Air, Material::AIR);
        assert_eq!(air.ambient_occlusion_light(), 1.0);

        // A non-cube of a movement-blocking material still lets light through
        let slab = block("slab", Kind::Slab, Material::WOOD);
        assert!(!slab.is_cube());
        assert_eq!(slab.ambient_occlusion_light(), 1.0);
    }

    #[test]
    fn slab_shape_follows_metadata_bit_8() {
        let slab = block("slab", Kind::Slab, Material::WOOD);
        let mut world = crate::fixture::TestWorld::new();
        world.set(0, 0, 0, 1, 0); // metadata 0
        assert_eq!(slab.shape(&world, 0, 0, 0).max_y, 0.5);
        world.set(0, 0, 0, 1, 8); // top slab
        assert_eq!(slab.shape(&world, 0, 0, 0).min_y, 0.5);
    }

    #[test]
    fn snow_shape_follows_the_layer_count() {
        let snow = block("snow", Kind::Snow, Material::SNOW_LAYER);
        let mut world = crate::fixture::TestWorld::new();
        world.set(0, 0, 0, 1, 3);
        assert_eq!(snow.shape(&world, 0, 0, 0).max_y, 0.5);
        world.set(0, 0, 0, 1, 7);
        assert_eq!(snow.shape(&world, 0, 0, 0).max_y, 1.0);
    }

    #[test]
    fn face_culling_checks_the_solid_render_of_the_neighbour() {
        let stone = block("stone", Kind::Cube, Material::STONE);
        let mut world = crate::fixture::TestWorld::new();
        world.set(1, 0, 0, 1, 0); // stone east
        // The caller passes the position the face looks at, like vanilla's
        // `tesselateInWorld` does.
        assert!(!stone.should_render_face(&world, 1, 0, 0, FACE_EAST, &Bounds::FULL));
        assert!(stone.should_render_face(&world, -1, 0, 0, FACE_WEST, &Bounds::FULL));
    }

    #[test]
    fn log_sprite_follows_the_axis_metadata() {
        let side = Sprite::atlas("log_side", 0, 0, 256, 256);
        let top = Sprite::atlas("log_top", 16, 0, 256, 256);
        let sprites = Sprites::Log { side, top };
        // Vertical: top on the horizontal faces.
        assert_eq!(sprites.sprite(0, FACE_TOP).name, "log_top");
        assert_eq!(sprites.sprite(0, FACE_NORTH).name, "log_side");
        // East-west: top on the west/east faces.
        assert_eq!(sprites.sprite(4, FACE_WEST).name, "log_top");
        assert_eq!(sprites.sprite(4, FACE_TOP).name, "log_side");
        // North-south: top on the north/south faces.
        assert_eq!(sprites.sprite(8, FACE_NORTH).name, "log_top");
        assert_eq!(sprites.sprite(8, FACE_EAST).name, "log_side");
    }
}
