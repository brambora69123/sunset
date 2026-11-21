//! The render-side half of `net.minecraft.block.Block` (1.7.10): the shape a block
//! draws with, its face culling, the light it needs and the tint it multiplies.
//!
//! The facts are keyed by 1.7.10's registry name (`stone`, `grass`, `log2`), the
//! name the engine's 1.7.10 content export gives every block state. They are read
//! from the era's own classes: `Block.getRenderType`'s switch, `Block.isCube`,
//! `Block.isSolidRender`, `Material.isOpaque`/`blocksMovement` and the
//! `getRenderLayer` overrides. A name this table does not know (a block a mod
//! added to the era registry) is a full opaque cube.

use super::material::Material;
use super::world::WorldView;

/// Face order of vanilla: 0 bottom (-Y), 1 top (+Y), 2 north (-Z),
/// 3 south (+Z), 4 west (-X), 5 east (+X).
pub const FACE_BOTTOM: i32 = 0;
pub const FACE_TOP: i32 = 1;
pub const FACE_NORTH: i32 = 2;
pub const FACE_SOUTH: i32 = 3;
pub const FACE_WEST: i32 = 4;
pub const FACE_EAST: i32 = 5;

/// Which 1.7.10 block shape the renderer is drawing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// Nothing is drawn (air, fire, the end portal).
    Air,
    /// A standard block (`renderStandardBlock`), full bounds unless `bounds` says
    /// otherwise.
    Cube,
    /// Grass: a cube whose side overlay pass carries the biome tint.
    Grass,
    /// An axis block (`tesselateAxisBlock`): the top sprite on the axis faces.
    Log,
    /// Quartz (`tesselateQuartz`): the same UV rotations on metadata 3 and 4.
    Quartz,
    /// Glass and stained glass: faces between same-metadata glass never draw.
    Glass,
    /// Ice: the same face rule as glass, with the face id mirrored.
    Ice,
    /// Leaves: `isSolidRender` follows the fancy-graphics option, and with fast
    /// graphics two leaves hide their shared face.
    Leaves,
    /// A single or double slab; metadata bit 8 picks the top half.
    Slab {
        double: bool,
    },
    /// A snow layer; metadata 0..7 picks the height.
    Snow,
    Stairs,
    Fence,
    /// Crossed squares (render types 1 and 6).
    Cross(CrossKind),
    /// Crops (render type 6): crossed squares anchored at the block centre.
    Plant,
    /// Water and lava (render type 4).
    Liquid(LiquidKind),
    /// A torch (render type 2).
    Torch,
    /// A cactus (render type 13).
    Cactus,
    /// A lily pad (render type 23): one flat quad at 1/16.
    LilyPad,
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

/// The two liquids, told apart by `LiquidBlock.getRenderLayer` and by the
/// `Material` the face culling compares.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LiquidKind {
    Water,
    Lava,
}

/// `Block.getColor(world, x, y, z)`'s tint source.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tint {
    /// `0xFFFFFF`: the texture's own colours.
    None,
    /// `Biome.getGrassColor` (`GrassColors`, the era's `colormap/grass.png`).
    Grass,
    /// `Biome.getFoliageColor` (`FoliageColors`, `colormap/foliage.png`).
    Foliage,
    /// `AbstractLeavesBlock.getColor`: spruce and birch are fixed, the rest is
    /// the biome's foliage colour.
    Leaves,
    /// `TallPlantBlock.getColor`: white for metadata 0, else the biome's grass.
    TallGrass,
    /// `DoublePlantBlock.getColor`: the biome's grass for variants 2 and 3.
    DoublePlant,
    /// A fixed `0xRRGGBB` multiplier.
    Fixed(u32),
    /// `StemBlock.getColor(metadata)`: the age's darkening.
    Stem,
}

/// The bounds one render pass draws within (`RenderBlocks.minX..maxZ`).
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

    /// `RenderBlocks.prepare`'s full-cube check: whether the pass fills the cell.
    pub fn is_full_cube(&self) -> bool {
        self.min_x == 0.0
            && self.max_x == 1.0
            && self.min_y == 0.0
            && self.max_y == 1.0
            && self.min_z == 0.0
            && self.max_z == 1.0
    }
}

/// One 1.7.10 block's render facts.
#[derive(Clone, Copy, Debug)]
pub struct BlockDef {
    /// `Block.getIdFromBlock`: the era's numeric id, the identity vanilla's
    /// glass and leaf face rules compare. Assigned by the state table (and the
    /// probe's fixture).
    pub key: u16,
    pub kind: Kind,
    pub material: Material,
    /// `Block.isTranslucent`: `!material.isOpaque()`, which the smooth lighting
    /// reads to decide whether a corner's light is halved.
    pub translucent: bool,
    /// `Block.usesNeighborLight`: the light sampler takes the maximum of the
    /// block above and the four horizontal neighbours (`Block.init`).
    pub use_neighbor_light: bool,
    pub tint: Tint,
    /// The bounds a standard block renders with; `FULL` unless its shape is
    /// smaller.
    pub bounds: Bounds,
    /// `getTopSprite(...).getName().equals("grass_top")`, the smooth-lighting
    /// path's `tinted` flag.
    pub top_is_grass_top: bool,
    /// `shouldRenderFace(face 1)` is always true (snow layers, carpets).
    pub top_always: bool,
    /// `FenceBlock.shouldConnectTo`'s same-kind test (`fence`, `fence_gate`,
    /// `nether_brick_fence`).
    pub connects_to_fence: bool,
    /// `getRenderLayer() == 1`: drawn after the solid pass, blended.
    pub translucent_pass: bool,
    /// `Block.isCube()`, which decides ambient-occlusion darkness and the
    /// fence's connection test.
    pub cube: bool,
    /// `Block.isSolidRender()` of a non-leaf, non-slab block.
    pub solid_render: bool,
}

impl BlockDef {
    /// The facts of the era block registered as `name`.
    pub fn new(name: &str) -> BlockDef {
        let kind = kind_of(name);
        let material = material_of(name, kind);
        let translucent = !material.is_opaque();
        // `Block.init`'s `opacity == 0`: the constructor's `isSolidRender`
        // value, except where a class calls `setOpacity` (water and lava 3,
        // leaves and webs 1, ice 3, slabs and stairs 255, farmland 255).
        let opacity_zero = match name {
            "anvil" | "snow_layer" | "carpet" | "enchanting_table" => true,
            "water" | "flowing_water" | "lava" | "flowing_lava" | "leaves" | "leaves2" | "web"
            | "ice" | "stone_slab" | "wooden_slab" | "double_stone_slab" | "double_wooden_slab"
            | "farmland" => false,
            _ if name.ends_with("_stairs") => false,
            _ => !solid_render_of(name, kind),
        };
        BlockDef {
            key: 0,
            kind,
            material,
            translucent,
            use_neighbor_light: !material.air
                && (matches!(kind, Kind::Stairs | Kind::Slab { .. })
                    || name == "tallgrass"
                    || translucent
                    || opacity_zero),
            tint: tint_of(name),
            bounds: bounds_of(name),
            top_is_grass_top: name == "grass",
            top_always: name == "snow_layer" || name == "carpet",
            connects_to_fence: matches!(name, "fence" | "nether_brick_fence" | "fence_gate"),
            translucent_pass: matches!(kind, Kind::Liquid(LiquidKind::Water))
                || matches!(
                    name,
                    "ice" | "stained_glass" | "stained_glass_pane" | "portal"
                ),
            cube: match kind {
                Kind::Slab { double } => double,
                Kind::Cube | Kind::Grass | Kind::Log | Kind::Quartz => !non_cube(name),
                _ => false,
            },
            solid_render: solid_render_of(name, kind),
        }
    }

    /// This block with its era numeric id.
    pub fn with_key(mut self, key: u16) -> BlockDef {
        self.key = key;
        self
    }

    /// `Block.isCube()`.
    pub fn is_cube(&self) -> bool {
        self.cube
    }

    /// `Block.isSolidRender()`, with the fancy-graphics flag leaves read.
    pub fn is_solid_render(&self, fancy: bool) -> bool {
        match self.kind {
            Kind::Air | Kind::Glass | Kind::Ice => false,
            Kind::Leaves => !fancy,
            Kind::Slab { double } => double,
            _ => self.solid_render,
        }
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

    /// `Block.getLightColor(world, x, y, z)`, including the single-slab light
    /// leak down a block when the slab sits in darkness and the liquid override
    /// that takes the brighter of the block and the block above.
    pub fn light_color(&self, world: &dyn WorldView, x: i32, y: i32, z: i32) -> i32 {
        // `Block.getLightColor` reads the block at the sampled position for its
        // emitted light, so a torch lights the samples around it.
        let light = |x: i32, y: i32, z: i32| i32::from(world.state(x, y, z).light);
        if matches!(self.kind, Kind::Liquid(_)) {
            let below = world.light_color(x, y, z, 0);
            let above = world.light_color(x, y + 1, z, 0);
            let block = (below & 0xFF).max(above & 0xFF);
            let sky = (below >> 16 & 0xFF).max(above >> 16 & 0xFF);
            return block | sky << 16;
        }
        let color = world.light_color(x, y, z, light(x, y, z));
        if color == 0 && matches!(self.kind, Kind::Slab { double: false }) {
            world.light_color(x, y - 1, z, light(x, y - 1, z))
        } else {
            color
        }
    }

    /// `Block.getColor(world, x, y, z)` at metadata `meta`. The biome colours
    /// are the era's: a 3×3 average of the biome colour at each position.
    pub fn color(&self, meta: u8, world: &dyn WorldView, x: i32, y: i32, z: i32) -> i32 {
        match self.tint {
            Tint::None => 0xFF_FFFF,
            Tint::Grass => world.biome_tint(Tint::Grass, x, y, z),
            Tint::Foliage => world.biome_tint(Tint::Foliage, x, y, z),
            Tint::Leaves => match meta & 3 {
                1 => 0x61_9961,
                2 => 0x80_A755,
                _ => world.biome_tint(Tint::Foliage, x, y, z),
            },
            Tint::TallGrass => {
                if meta == 0 {
                    0xFF_FFFF
                } else {
                    world.biome_tint(Tint::Grass, x, y, z)
                }
            }
            Tint::DoublePlant => {
                // `getVariant`: the upper half reads the bottom half's
                // metadata below it.
                let meta = if meta & 8 == 0 {
                    meta
                } else {
                    world.metadata(x, y - 1, z) as u8
                };
                if matches!(meta & 7, 2 | 3) {
                    world.biome_tint(Tint::Grass, x, y, z)
                } else {
                    0xFF_FFFF
                }
            }
            Tint::Fixed(rgb) => rgb as i32,
            Tint::Stem => {
                let age = i32::from(meta);
                (age * 32) << 16 | (255 - age * 8) << 8 | (age * 4)
            }
        }
    }

    /// `Block.updateShape` + the bounds `RenderBlocks.prepare` reads.
    pub fn shape(&self, meta: u8, world: &dyn WorldView, x: i32, y: i32, z: i32) -> Bounds {
        match self.kind {
            Kind::Slab { double } => {
                if double {
                    Bounds::FULL
                } else if meta & 8 != 0 {
                    Bounds::new(0.0, 0.5, 0.0, 1.0, 1.0, 1.0)
                } else {
                    Bounds::new(0.0, 0.0, 0.0, 1.0, 0.5, 1.0)
                }
            }
            Kind::Snow => {
                let layers = meta & 7;
                let height = (2 * (1 + layers)) as f32 / 16.0;
                Bounds::new(0.0, 0.0, 0.0, 1.0, height as f64, 1.0)
            }
            Kind::LilyPad => Bounds::new(0.0, 0.0, 0.0, 1.0, 1.0 / 16.0, 1.0),
            Kind::Torch => torch_bounds(meta),
            _ => {
                // A bounded shape that follows its neighbours is read where it
                // is drawn (`render_blocks`), so the static bounds suffice.
                let _ = (world, x, y, z);
                self.bounds
            }
        }
    }

    /// `Block.shouldRenderFace(world, x, y, z, face)` for the neighbour at
    /// `(x, y, z)`; the glass, leaves, ice and liquid overrides live in the
    /// renderer, which knows the block's own position.
    pub fn should_render_face(
        &self,
        world: &dyn WorldView,
        x: i32,
        y: i32,
        z: i32,
        face: i32,
        bounds: &Bounds,
        fancy: bool,
    ) -> bool {
        if (self.top_always && face == FACE_TOP) || matches!(self.kind, Kind::Fence) {
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
        bounds_force || !world.block(x, y, z).is_solid_render(fancy)
    }

    /// `SlabBlock.shouldRenderFace`. The caller passes the position this face
    /// looks at (the neighbour), like `tesselateInWorld` does.
    pub fn slab_should_render_face(
        &self,
        world: &dyn WorldView,
        x: i32,
        y: i32,
        z: i32,
        face: i32,
    ) -> bool {
        let (ox, oy, oz) = opposite_offset(face);
        let own = self.shape(
            world.metadata(x + ox, y + oy, z + oz) as u8,
            world,
            x + ox,
            y + oy,
            z + oz,
        );
        let own_top = world.metadata(x + ox, y + oy, z + oz) & 8 != 0;
        let neighbour = world.block(x, y, z);
        let neighbour_top = world.metadata(x, y, z) & 8 != 0;
        if face != FACE_TOP
            && face != FACE_BOTTOM
            && !self.should_render_face(world, x, y, z, face, &own, false)
        {
            return false;
        }
        if own_top {
            if face == FACE_BOTTOM {
                return true;
            }
            if face == FACE_TOP && self.should_render_face(world, x, y, z, face, &own, false) {
                return true;
            }
            return !is_single_slab(neighbour) || !neighbour_top;
        }
        if face == FACE_TOP {
            return true;
        }
        if face == FACE_BOTTOM && self.should_render_face(world, x, y, z, face, &own, false) {
            return true;
        }
        !is_single_slab(neighbour) || neighbour_top
    }
}

/// `SlabBlock.isSingleSlab`: a single (not double) slab of either family.
fn is_single_slab(block: &BlockDef) -> bool {
    matches!(block.kind, Kind::Slab { double: false })
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

impl Kind {
    /// The shape of the era block registered as `name`.
    pub fn of(name: &str) -> Kind {
        kind_of(name)
    }
}

fn kind_of(name: &str) -> Kind {
    match name {
        "air" | "fire" | "end_portal" => Kind::Air,
        "grass" => Kind::Grass,
        "log" | "log2" | "hay_block" => Kind::Log,
        "quartz_block" => Kind::Quartz,
        "glass" | "stained_glass" => Kind::Glass,
        "ice" => Kind::Ice,
        "leaves" | "leaves2" => Kind::Leaves,
        "double_stone_slab" | "double_wooden_slab" => Kind::Slab { double: true },
        "stone_slab" | "wooden_slab" => Kind::Slab { double: false },
        "snow_layer" => Kind::Snow,
        "fence" | "nether_brick_fence" => Kind::Fence,
        "tallgrass" => Kind::Cross(CrossKind::TallGrass),
        "yellow_flower" | "red_flower" => Kind::Cross(CrossKind::Flower),
        "brown_mushroom" | "red_mushroom" | "deadbush" | "web" | "reeds" | "vine" => {
            Kind::Cross(CrossKind::Plain)
        }
        "water" | "flowing_water" => Kind::Liquid(LiquidKind::Water),
        "lava" | "flowing_lava" => Kind::Liquid(LiquidKind::Lava),
        "torch" | "redstone_torch" | "unlit_redstone_torch" => Kind::Torch,
        "cactus" => Kind::Cactus,
        "wheat" | "carrots" | "potatoes" | "nether_wart" | "pumpkin_stem" | "melon_stem" => {
            Kind::Plant
        }
        "waterlily" => Kind::LilyPad,
        // A double plant crosses at the same 0.3 offset as a flower; its
        // upper half and the sunflower's second cross are not ported.
        "sapling" => Kind::Cross(CrossKind::Plain),
        "double_plant" => Kind::Cross(CrossKind::Flower),
        _ if name.ends_with("_stairs") => Kind::Stairs,
        _ => Kind::Cube,
    }
}

fn material_of(name: &str, kind: Kind) -> Material {
    match name {
        "web" => return Material::WEB,
        "portal" => return Material::PORTAL,
        "carpet" => return Material::CARPET,
        "tnt" => return Material::TNT,
        "snow" => return Material::SNOW,
        "snow_layer" => return Material::SNOW_LAYER,
        "pumpkin" | "lit_pumpkin" => return Material::PUMPKIN,
        "cocoa" => return Material::PLANT,
        _ => {}
    }
    match kind {
        Kind::Air => Material::AIR,
        Kind::Liquid(LiquidKind::Water) => Material::WATER,
        Kind::Liquid(LiquidKind::Lava) => Material::LAVA,
        Kind::Glass => Material::GLASS,
        Kind::Ice => Material::ICE,
        Kind::Leaves => Material::LEAVES,
        Kind::Cross(_) | Kind::Plant => Material::PLANT,
        Kind::Snow => Material::SNOW_LAYER,
        Kind::Torch => Material::PLANT,
        Kind::Cactus => Material::CACTUS,
        Kind::Fence | Kind::Stairs | Kind::Log | Kind::Quartz | Kind::Slab { .. } => Material::WOOD,
        Kind::LilyPad => Material::PLANT,
        Kind::Cube | Kind::Grass => match name {
            "snow" | "snow_layer" => Material::SNOW,
            "tnt" => Material::TNT,
            "pumpkin" | "lit_pumpkin" => Material::PUMPKIN,
            "web" => Material::WEB,
            "portal" => Material::PORTAL,
            "carpet" => Material::CARPET,
            _ => Material::STONE,
        },
    }
}

/// Blocks whose shape is not the full cell (`Block.isCube()`, and the render
/// type's own bounds).
fn non_cube(name: &str) -> bool {
    matches!(
        name,
        "anvil"
            | "bed"
            | "brewing_stand"
            | "cake"
            | "cauldron"
            | "chest"
            | "trapped_chest"
            | "ender_chest"
            | "cocoa"
            | "daylight_detector"
            | "dragon_egg"
            | "enchanting_table"
            | "end_portal_frame"
            | "farmland"
            | "flower_pot"
            | "hopper"
            | "ladder"
            | "lever"
            | "piston"
            | "sticky_piston"
            | "piston_head"
            | "piston_extension"
            | "redstone_wire"
            | "unpowered_repeater"
            | "powered_repeater"
            | "unpowered_comparator"
            | "powered_comparator"
            | "standing_sign"
            | "wall_sign"
            | "skull"
            | "stone_button"
            | "wooden_button"
            | "stone_pressure_plate"
            | "wooden_pressure_plate"
            | "light_weighted_pressure_plate"
            | "heavy_weighted_pressure_plate"
            | "trapdoor"
            | "tripwire"
            | "tripwire_hook"
            | "wooden_door"
            | "iron_door"
            | "fence_gate"
            | "rail"
            | "golden_rail"
            | "detector_rail"
            | "activator_rail"
            | "portal"
            | "beacon"
            | "glass_pane"
            | "iron_bars"
            | "stained_glass_pane"
            | "cobblestone_wall"
    )
}

/// `Block.isSolidRender()` of the era block, before the fancy-graphics flag.
fn solid_render_of(name: &str, kind: Kind) -> bool {
    match kind {
        Kind::Air | Kind::Leaves | Kind::Glass | Kind::Ice => false,
        Kind::Slab { double } => double,
        Kind::Cube => !matches!(
            name,
            "anvil"
                | "bed"
                | "brewing_stand"
                | "cake"
                | "cauldron"
                | "chest"
                | "trapped_chest"
                | "ender_chest"
                | "cocoa"
                | "daylight_detector"
                | "dragon_egg"
                | "enchanting_table"
                | "end_portal_frame"
                | "farmland"
                | "flower_pot"
                | "hopper"
                | "ladder"
                | "lever"
                | "mob_spawner"
                | "piston"
                | "sticky_piston"
                | "piston_head"
                | "piston_extension"
                | "redstone_wire"
                | "unpowered_repeater"
                | "powered_repeater"
                | "unpowered_comparator"
                | "powered_comparator"
                | "standing_sign"
                | "wall_sign"
                | "skull"
                | "stone_button"
                | "wooden_button"
                | "stone_pressure_plate"
                | "wooden_pressure_plate"
                | "light_weighted_pressure_plate"
                | "heavy_weighted_pressure_plate"
                | "trapdoor"
                | "tripwire"
                | "tripwire_hook"
                | "wooden_door"
                | "iron_door"
                | "fence_gate"
                | "golden_rail"
                | "detector_rail"
                | "activator_rail"
                | "rail"
                | "portal"
                | "beacon"
                | "pane"
                | "glass_pane"
                | "iron_bars"
                | "stained_glass_pane"
        ),
        _ => false,
    }
}

fn tint_of(name: &str) -> Tint {
    match name {
        "grass" => Tint::Grass,
        "tallgrass" => Tint::TallGrass,
        "double_plant" => Tint::DoublePlant,
        "leaves" | "leaves2" => Tint::Leaves,
        "vine" => Tint::Foliage,
        "reeds" => Tint::Grass,
        "waterlily" => Tint::Fixed(2_129_968),
        "redstone_wire" => Tint::Fixed(0x80_0000),
        "pumpkin_stem" | "melon_stem" => Tint::Stem,
        _ => Tint::None,
    }
}

fn bounds_of(name: &str) -> Bounds {
    match name {
        "soul_sand" => Bounds::new(0.0, 0.0, 0.0, 1.0, 0.875, 1.0),
        "farmland" => Bounds::new(0.0, 0.0, 0.0, 1.0, 0.9375, 1.0),
        "end_portal_frame" => Bounds::new(0.0, 0.0, 0.0, 1.0, 0.8125, 1.0),
        "carpet" => Bounds::new(0.0, 0.0, 0.0, 1.0, 1.0 / 16.0, 1.0),
        "chest" | "trapped_chest" | "ender_chest" => {
            Bounds::new(0.0625, 0.0, 0.0625, 0.9375, 0.875, 0.9375)
        }
        "bed" => Bounds::new(0.0, 0.0, 0.0, 1.0, 0.5625, 1.0),
        "hopper" => Bounds::new(0.0, 0.0, 0.0, 1.0, 0.625, 1.0),
        "brewing_stand" => Bounds::new(0.4375, 0.0, 0.4375, 0.5625, 0.875, 0.5625),
        "flower_pot" => Bounds::new(0.3125, 0.0, 0.3125, 0.6875, 0.375, 0.6875),
        "enchanting_table" => Bounds::new(0.0, 0.0, 0.0, 1.0, 0.75, 1.0),
        "dragon_egg" => Bounds::new(0.0625, 0.0, 0.0625, 0.9375, 1.0, 0.9375),
        "anvil" => Bounds::new(0.125, 0.0, 0.125, 0.875, 1.0, 0.875),
        "cake" => Bounds::new(0.0625, 0.0, 0.0625, 0.9375, 0.5, 0.9375),
        "cauldron" => Bounds::new(0.0, 0.0, 0.0, 1.0, 1.0, 1.0),
        "ladder" => Bounds::new(0.0, 0.0, 0.0, 1.0, 1.0, 0.125),
        "glass_pane" | "stained_glass_pane" | "iron_bars" => {
            Bounds::new(0.4375, 0.0, 0.0, 0.5625, 1.0, 1.0)
        }
        "cobblestone_wall" => Bounds::new(0.25, 0.0, 0.25, 0.75, 1.0, 0.75),
        // CactusBlock keeps the full shape and insets its side faces by 1/16
        // as it draws them (`tesselateCactus`).
        "cactus" => Bounds::FULL,
        // The flat shapes: rails, wire, tripwire, plates and buttons lie on
        // the floor, signs stand as a thin plate. Their real models are not
        // ported yet; the box is the vanilla collision height.
        "rail"
        | "golden_rail"
        | "detector_rail"
        | "activator_rail"
        | "redstone_wire"
        | "tripwire"
        | "stone_pressure_plate"
        | "wooden_pressure_plate"
        | "light_weighted_pressure_plate"
        | "heavy_weighted_pressure_plate"
        | "stone_button"
        | "wooden_button"
        | "lever" => Bounds::new(0.0, 0.0, 0.0, 1.0, 1.0 / 16.0, 1.0),
        "standing_sign" | "wall_sign" => Bounds::new(0.0, 0.0, 7.0 / 16.0, 1.0, 1.0, 9.0 / 16.0),
        "skull" => Bounds::new(0.25, 0.0, 0.25, 0.75, 0.5, 0.75),
        "trapdoor" => Bounds::new(0.0, 0.0, 0.0, 1.0, 0.1875, 1.0),
        "wooden_door" | "iron_door" => Bounds::new(0.0, 0.0, 0.0, 0.1875, 1.0, 1.0),
        "fence_gate" => Bounds::new(0.0, 0.0, 0.375, 1.0, 1.0, 0.625),
        "beacon" => Bounds::new(0.0, 0.0, 0.0, 1.0, 1.0, 1.0),
        "piston" | "sticky_piston" => Bounds::new(0.0, 0.0, 0.0, 1.0, 1.0, 1.0),
        _ => Bounds::FULL,
    }
}

/// `TorchBlock.updateShape`: the 0.4..0.6 post, leaning for the wall metadata.
fn torch_bounds(meta: u8) -> Bounds {
    let (min, max) = (0.4, 0.6);
    match meta {
        1 => Bounds::new(0.0, 0.2, min, 0.2, 0.8, max),
        2 => Bounds::new(1.0 - 0.2, 0.2, min, 1.0, 0.8, max),
        3 => Bounds::new(min, 0.2, 0.0, max, 0.8, 0.2),
        4 => Bounds::new(min, 0.2, 1.0 - 0.2, max, 0.8, 1.0),
        _ => Bounds::new(min, 0.0, min, max, 0.6, max),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_eras_shapes_are_recognised_by_name() {
        assert_eq!(Kind::of("stone"), Kind::Cube);
        assert_eq!(Kind::of("grass"), Kind::Grass);
        assert_eq!(Kind::of("log2"), Kind::Log);
        assert_eq!(Kind::of("oak_stairs"), Kind::Stairs);
        assert_eq!(Kind::of("stone_slab"), Kind::Slab { double: false });
        assert_eq!(Kind::of("double_wooden_slab"), Kind::Slab { double: true });
        assert_eq!(Kind::of("tallgrass"), Kind::Cross(CrossKind::TallGrass));
        assert_eq!(Kind::of("water"), Kind::Liquid(LiquidKind::Water));
        assert_eq!(Kind::of("wheat"), Kind::Plant);
        assert_eq!(Kind::of("snow_layer"), Kind::Snow);
        assert_eq!(Kind::of("cactus"), Kind::Cactus);
    }

    #[test]
    fn air_draws_nothing_and_glass_is_not_solid() {
        assert_eq!(BlockDef::new("air").kind, Kind::Air);
        assert!(!BlockDef::new("air").is_solid_render(false));
        assert!(!BlockDef::new("glass").is_solid_render(false));
        assert!(BlockDef::new("stone").is_solid_render(false));
        // Leaves follow the fancy-graphics option.
        assert!(!BlockDef::new("leaves").is_solid_render(true));
        assert!(BlockDef::new("leaves").is_solid_render(false));
        // A double slab is a full cube.
        assert!(BlockDef::new("double_stone_slab").is_solid_render(false));
        assert!(!BlockDef::new("stone_slab").is_solid_render(false));
    }

    #[test]
    fn neighbour_light_follows_the_eras_init_rules() {
        // Stairs, slabs, tallgrass and every opacity-0 block smooth their light.
        assert!(BlockDef::new("oak_stairs").use_neighbor_light);
        assert!(BlockDef::new("stone_slab").use_neighbor_light);
        assert!(BlockDef::new("tallgrass").use_neighbor_light);
        assert!(BlockDef::new("glass").use_neighbor_light);
        assert!(BlockDef::new("sapling").use_neighbor_light);
        // Stone, water (opacity 3) and web (opacity 1) do not.
        assert!(!BlockDef::new("stone").use_neighbor_light);
        assert!(!BlockDef::new("water").use_neighbor_light);
        assert!(!BlockDef::new("web").use_neighbor_light);
    }

    #[test]
    fn ambient_occlusion_light_reads_material_and_shape() {
        assert_eq!(BlockDef::new("stone").ambient_occlusion_light(), 0.2);
        // Glass is not a cube, so it lets light through for the AO pass.
        assert_eq!(BlockDef::new("glass").ambient_occlusion_light(), 1.0);
        // Water blocks no movement.
        assert_eq!(BlockDef::new("water").ambient_occlusion_light(), 1.0);
        assert_eq!(BlockDef::new("web").ambient_occlusion_light(), 1.0);
    }

    #[test]
    fn the_biome_tint_families_match_the_era() {
        assert_eq!(BlockDef::new("grass").tint, Tint::Grass);
        assert_eq!(BlockDef::new("tallgrass").tint, Tint::TallGrass);
        assert_eq!(BlockDef::new("leaves").tint, Tint::Leaves);
        assert_eq!(BlockDef::new("vine").tint, Tint::Foliage);
        // Water has no tint of its own in 1.7.10: `LiquidBlock.getColor` is the
        // biome's white water-fog colour.
        assert_eq!(BlockDef::new("water").tint, Tint::None);
        assert_eq!(BlockDef::new("waterlily").tint, Tint::Fixed(2_129_968));
    }

    #[test]
    fn snow_layer_height_follows_its_metadata() {
        let snow = BlockDef::new("snow_layer");
        let world = super::super::fixture::TestWorld::new();
        assert_eq!(snow.shape(3, &world, 0, 0, 0).max_y, 0.5);
        assert_eq!(snow.shape(7, &world, 0, 0, 0).max_y, 1.0);
    }
}
