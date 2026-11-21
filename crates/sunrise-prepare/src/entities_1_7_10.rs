//! Minecraft 1.7.10's entity models and entity types, read from the era's own `Model*`
//! classes and baked into the engine's `entity-models.json` shape.
//!
//! The engine renders every living entity from a baked part tree: a root whose children are
//! the model's bones, each with a pose and its boxes as textured quads. 26.3 exports that tree
//! from `LayerDefinitions`; the era has no such table, so [`models`] holds the same numbers
//! hand-transcribed from the era's sources (`net.minecraft.client.render.model.entity.*` and
//! `Box`, whose constructor is the box-to-quad mapping) and [`bake_models`] turns them into the
//! engine's document. [`entity_types`] holds the dimensions and base attributes the era's mob
//! classes give (`setSize`, `initAttributes`).
//!
//! The bones use the engine's names (`right_hind_leg`, `right_arm`, ...), not the era's field
//! names, so the animation the engine already plays drives the same parts. A model keeps the
//! era's texture size, which sets the quads' UVs: a 1.7.10 skin is 64x32, while a zombie's
//! sheet is 64x64, and the same box means different pixels on each.
//!
//! [`write`] puts both documents in a prepared 1.7.10 folder, beside `assets/`.

use crate::pyjson::Value;
use crate::{write_if_changed, Log};
use std::path::Path;

/// The baked part trees, in the engine's shape (the document the 26.3 exporter writes).
pub const MODELS: &str = "entity-models.json";
/// The entity types' dimensions and base attributes.
pub const TYPES: &str = "entity-types.json";

/// One box of a part (`Box`, `ModelPart.addBox`): the corner, the size, where its unwrapped
/// texture starts, and the inflation the era's `addBox` calls `increase`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cube {
    /// `addBox`'s `x`, `y`, `z`: the corner at the part's texture offset.
    pub origin: [f32; 3],
    /// `addBox`'s `sizeX`, `sizeY`, `sizeZ`.
    pub size: [u32; 3],
    /// `setTextureCoords`' `u`, `v`: the box's top-left corner in the texture.
    pub uv: [i32; 2],
    /// The `increase` every face grows by (the hat layer's `0.5`).
    pub inflate: f32,
    /// `ModelPart.flipped`: the box mirrors around the part's X and its quads reverse.
    pub mirror: bool,
}

/// One part of a model: `ModelPart`, with its pivot (`setPos`) and initial rotation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bone {
    /// The engine's part name (`head`, `right_hind_leg`, `hat`, ...).
    pub name: &'static str,
    /// The part this one hangs from; `None` for a direct child of the model's root.
    pub parent: Option<&'static str>,
    /// `setPos`: the rotation point, relative to the parent part.
    pub pivot: [f32; 3],
    /// The pose the era's `setupAnimation` leaves at rest (`body` lies a quadruped's way).
    pub rotation: [f32; 3],
    pub cubes: &'static [Cube],
}

/// One era entity model, with the sheet it draws from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Model {
    /// The engine's layer key (`minecraft:zombie#main`).
    pub name: &'static str,
    /// The era's texture file, relative to `assets/minecraft` (`textures/entity/steve.png`).
    pub texture: &'static str,
    /// The model's `textureWidth` and `textureHeight`, which scale every UV.
    pub texture_size: [u32; 2],
    pub bones: &'static [Bone],
}

/// A texture the engine's species table asks for under a 26.3 name, and the era file that
/// holds it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextureName {
    /// The name the engine looks up in its atlas (`entity/pig/pig_temperate`).
    pub name: &'static str,
    /// The era's file, relative to `assets/minecraft`.
    pub path: &'static str,
}

/// One entity type's row: `EntityType.getDimensions` and the class's `setSize`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EntityType {
    pub id: &'static str,
    pub width: f32,
    pub height: f32,
    pub eye_height: f32,
    /// The era's `EnumCreatureType` name.
    pub category: &'static str,
    pub fire_immune: bool,
    /// The class's `initAttributes` values, by the engine's attribute names.
    pub attributes: &'static [(&'static str, f32)],
}

const fn cube(origin: [f32; 3], size: [u32; 3], uv: [i32; 2], inflate: f32, mirror: bool) -> Cube {
    Cube {
        origin,
        size,
        uv,
        inflate,
        mirror,
    }
}

const fn bone(
    name: &'static str,
    parent: Option<&'static str>,
    pivot: [f32; 3],
    rotation: [f32; 3],
    cubes: &'static [Cube],
) -> Bone {
    Bone {
        name,
        parent,
        pivot,
        rotation,
        cubes,
    }
}

const ZERO: [f32; 3] = [0.0, 0.0, 0.0];
const LIE_DOWN: [f32; 3] = [std::f32::consts::FRAC_PI_2, 0.0, 0.0];
/// `ModelSpider.setupAnimation`'s splay `float var8 = PI / 4`.
const SPIDER_SPLAY: f32 = std::f32::consts::FRAC_PI_4;

// `ModelBiped`/`HumanoidModel` (player, zombie, skeleton share it).
const HEAD: Bone = bone(
    "head",
    None,
    ZERO,
    ZERO,
    &[cube([-4.0, -8.0, -4.0], [8, 8, 8], [0, 0], 0.0, false)],
);
const BODY: Bone = bone(
    "body",
    None,
    ZERO,
    ZERO,
    &[cube([-4.0, 0.0, -2.0], [8, 12, 4], [16, 16], 0.0, false)],
);
const RIGHT_ARM: Bone = bone(
    "right_arm",
    None,
    [-5.0, 2.0, 0.0],
    ZERO,
    &[cube([-3.0, -2.0, -2.0], [4, 12, 4], [40, 16], 0.0, false)],
);
const LEFT_ARM: Bone = bone(
    "left_arm",
    None,
    [5.0, 2.0, 0.0],
    ZERO,
    &[cube([-1.0, -2.0, -2.0], [4, 12, 4], [40, 16], 0.0, true)],
);
const RIGHT_LEG: Bone = bone(
    "right_leg",
    None,
    [-1.9, 12.0, 0.1],
    ZERO,
    &[cube([-2.0, 0.0, -2.0], [4, 12, 4], [0, 16], 0.0, false)],
);
const LEFT_LEG: Bone = bone(
    "left_leg",
    None,
    [1.9, 12.0, 0.1],
    ZERO,
    &[cube([-2.0, 0.0, -2.0], [4, 12, 4], [0, 16], 0.0, true)],
);
const HAT: Bone = bone(
    "hat",
    Some("head"),
    ZERO,
    ZERO,
    &[cube([-4.0, -8.0, -4.0], [8, 8, 8], [32, 0], 0.5, false)],
);

/// The era's living models, by the layer keys the engine's species table asks for.
pub const MODELS_TABLE: &[Model] = &[
    // `PlayerRenderer`'s `new HumanoidModel(0.0F)`: `ModelBiped` at 64x32, one skin layout.
    Model {
        name: "minecraft:player#main",
        texture: "textures/entity/steve.png",
        texture_size: [64, 32],
        bones: &[HEAD, HAT, BODY, RIGHT_ARM, LEFT_ARM, RIGHT_LEG, LEFT_LEG],
    },
    // 1.7.10 has no slim arms; a slim skin of that era is the same 64x32 biped.
    Model {
        name: "minecraft:player_slim#main",
        texture: "textures/entity/steve.png",
        texture_size: [64, 32],
        bones: &[HEAD, HAT, BODY, RIGHT_ARM, LEFT_ARM, RIGHT_LEG, LEFT_LEG],
    },
    // `new ZombieModel()`: `ModelBiped` with `pivot=false`, a 64x64 sheet.
    Model {
        name: "minecraft:zombie#main",
        texture: "textures/entity/zombie/zombie.png",
        texture_size: [64, 64],
        bones: &[HEAD, HAT, BODY, RIGHT_ARM, LEFT_ARM, RIGHT_LEG, LEFT_LEG],
    },
    // `new SkeletonModel()`: `ModelBiped` at 64x32 with 2x12x2 limbs.
    Model {
        name: "minecraft:skeleton#main",
        texture: "textures/entity/skeleton/skeleton.png",
        texture_size: [64, 32],
        bones: &[
            HEAD,
            HAT,
            BODY,
            bone(
                "right_arm",
                None,
                [-5.0, 2.0, 0.0],
                ZERO,
                &[cube([-1.0, -2.0, -1.0], [2, 12, 2], [40, 16], 0.0, false)],
            ),
            bone(
                "left_arm",
                None,
                [5.0, 2.0, 0.0],
                ZERO,
                &[cube([-1.0, -2.0, -1.0], [2, 12, 2], [40, 16], 0.0, true)],
            ),
            bone(
                "right_leg",
                None,
                [-2.0, 12.0, 0.0],
                ZERO,
                &[cube([-1.0, 0.0, -1.0], [2, 12, 2], [0, 16], 0.0, false)],
            ),
            bone(
                "left_leg",
                None,
                [2.0, 12.0, 0.0],
                ZERO,
                &[cube([-1.0, 0.0, -1.0], [2, 12, 2], [0, 16], 0.0, true)],
            ),
        ],
    },
    // `CreeperModel`: the hat is the 0.5-px outer layer; the legs end at y=16.
    Model {
        name: "minecraft:creeper#main",
        texture: "textures/entity/creeper/creeper.png",
        texture_size: [64, 32],
        bones: &[
            bone(
                "head",
                None,
                [0.0, 4.0, 0.0],
                ZERO,
                &[cube([-4.0, -8.0, -4.0], [8, 8, 8], [0, 0], 0.0, false)],
            ),
            bone(
                "hat",
                Some("head"),
                ZERO,
                ZERO,
                &[cube([-4.0, -8.0, -4.0], [8, 8, 8], [32, 0], 0.5, false)],
            ),
            bone(
                "body",
                None,
                [0.0, 4.0, 0.0],
                ZERO,
                &[cube([-4.0, 0.0, -2.0], [8, 12, 4], [16, 16], 0.0, false)],
            ),
            // `CreeperModel` binds its left/right fields to the opposite side (`anim.rs`
            // notes it), so the era's right legs take the engine's left names.
            bone(
                "left_hind_leg",
                None,
                [-2.0, 16.0, 4.0],
                ZERO,
                &[cube([-2.0, 0.0, -2.0], [4, 6, 4], [0, 16], 0.0, false)],
            ),
            bone(
                "right_hind_leg",
                None,
                [2.0, 16.0, 4.0],
                ZERO,
                &[cube([-2.0, 0.0, -2.0], [4, 6, 4], [0, 16], 0.0, false)],
            ),
            bone(
                "left_front_leg",
                None,
                [-2.0, 16.0, -4.0],
                ZERO,
                &[cube([-2.0, 0.0, -2.0], [4, 6, 4], [0, 16], 0.0, false)],
            ),
            bone(
                "right_front_leg",
                None,
                [2.0, 16.0, -4.0],
                ZERO,
                &[cube([-2.0, 0.0, -2.0], [4, 6, 4], [0, 16], 0.0, false)],
            ),
        ],
    },
    // `CowModel extends QuadrupedModel(12)`: its own head and body, horns and udder.
    Model {
        name: "minecraft:cow#main",
        texture: "textures/entity/cow/cow.png",
        texture_size: [64, 32],
        bones: &[
            bone(
                "head",
                None,
                [0.0, 4.0, -8.0],
                ZERO,
                &[
                    cube([-4.0, -4.0, -6.0], [8, 8, 6], [0, 0], 0.0, false),
                    cube([-5.0, -5.0, -4.0], [1, 3, 1], [22, 0], 0.0, false),
                    cube([4.0, -5.0, -4.0], [1, 3, 1], [22, 0], 0.0, false),
                ],
            ),
            bone(
                "body",
                None,
                [0.0, 5.0, 2.0],
                LIE_DOWN,
                &[
                    cube([-6.0, -10.0, -7.0], [12, 18, 10], [18, 4], 0.0, false),
                    cube([-2.0, 2.0, -8.0], [4, 6, 1], [52, 0], 0.0, false),
                ],
            ),
            bone(
                "right_hind_leg",
                None,
                [-4.0, 12.0, 7.0],
                ZERO,
                &[cube([-2.0, 0.0, -2.0], [4, 12, 4], [0, 16], 0.0, false)],
            ),
            bone(
                "left_hind_leg",
                None,
                [4.0, 12.0, 7.0],
                ZERO,
                &[cube([-2.0, 0.0, -2.0], [4, 12, 4], [0, 16], 0.0, false)],
            ),
            bone(
                "right_front_leg",
                None,
                [-4.0, 12.0, -6.0],
                ZERO,
                &[cube([-2.0, 0.0, -2.0], [4, 12, 4], [0, 16], 0.0, false)],
            ),
            bone(
                "left_front_leg",
                None,
                [4.0, 12.0, -6.0],
                ZERO,
                &[cube([-2.0, 0.0, -2.0], [4, 12, 4], [0, 16], 0.0, false)],
            ),
        ],
    },
    // `PigModel extends QuadrupedModel(6)`: the snout is a second box on the head.
    Model {
        name: "minecraft:pig#main",
        texture: "textures/entity/pig/pig.png",
        texture_size: [64, 32],
        bones: &[
            bone(
                "head",
                None,
                [0.0, 12.0, -6.0],
                ZERO,
                &[
                    cube([-4.0, -4.0, -8.0], [8, 8, 8], [0, 0], 0.0, false),
                    cube([-2.0, 0.0, -9.0], [4, 3, 1], [16, 16], 0.0, false),
                ],
            ),
            bone(
                "body",
                None,
                [0.0, 11.0, 2.0],
                LIE_DOWN,
                &[cube([-5.0, -10.0, -7.0], [10, 16, 8], [28, 8], 0.0, false)],
            ),
            bone(
                "right_hind_leg",
                None,
                [-3.0, 18.0, 7.0],
                ZERO,
                &[cube([-2.0, 0.0, -2.0], [4, 6, 4], [0, 16], 0.0, false)],
            ),
            bone(
                "left_hind_leg",
                None,
                [3.0, 18.0, 7.0],
                ZERO,
                &[cube([-2.0, 0.0, -2.0], [4, 6, 4], [0, 16], 0.0, false)],
            ),
            bone(
                "right_front_leg",
                None,
                [-3.0, 18.0, -5.0],
                ZERO,
                &[cube([-2.0, 0.0, -2.0], [4, 6, 4], [0, 16], 0.0, false)],
            ),
            bone(
                "left_front_leg",
                None,
                [3.0, 18.0, -5.0],
                ZERO,
                &[cube([-2.0, 0.0, -2.0], [4, 6, 4], [0, 16], 0.0, false)],
            ),
        ],
    },
    // `SheepModel extends QuadrupedModel(12)`: its own head and body.
    Model {
        name: "minecraft:sheep#main",
        texture: "textures/entity/sheep/sheep.png",
        texture_size: [64, 32],
        bones: &[
            bone(
                "head",
                None,
                [0.0, 6.0, -8.0],
                ZERO,
                &[cube([-3.0, -4.0, -6.0], [6, 6, 8], [0, 0], 0.0, false)],
            ),
            bone(
                "body",
                None,
                [0.0, 5.0, 2.0],
                LIE_DOWN,
                &[cube([-4.0, -10.0, -7.0], [8, 16, 6], [28, 8], 0.0, false)],
            ),
            bone(
                "right_hind_leg",
                None,
                [-3.0, 12.0, 7.0],
                ZERO,
                &[cube([-2.0, 0.0, -2.0], [4, 12, 4], [0, 16], 0.0, false)],
            ),
            bone(
                "left_hind_leg",
                None,
                [3.0, 12.0, 7.0],
                ZERO,
                &[cube([-2.0, 0.0, -2.0], [4, 12, 4], [0, 16], 0.0, false)],
            ),
            bone(
                "right_front_leg",
                None,
                [-3.0, 12.0, -5.0],
                ZERO,
                &[cube([-2.0, 0.0, -2.0], [4, 12, 4], [0, 16], 0.0, false)],
            ),
            bone(
                "left_front_leg",
                None,
                [3.0, 12.0, -5.0],
                ZERO,
                &[cube([-2.0, 0.0, -2.0], [4, 12, 4], [0, 16], 0.0, false)],
            ),
        ],
    },
    // `SheepFurModel`: the same quadruped, its boxes grown so the wool sits over the sheep.
    Model {
        name: "minecraft:sheep#wool",
        texture: "textures/entity/sheep/sheep_fur.png",
        texture_size: [64, 32],
        bones: &[
            bone(
                "head",
                None,
                [0.0, 6.0, -8.0],
                ZERO,
                &[cube([-3.0, -4.0, -4.0], [6, 6, 6], [0, 0], 0.6, false)],
            ),
            bone(
                "body",
                None,
                [0.0, 5.0, 2.0],
                LIE_DOWN,
                &[cube([-4.0, -10.0, -7.0], [8, 16, 6], [28, 8], 1.75, false)],
            ),
            bone(
                "right_hind_leg",
                None,
                [-3.0, 12.0, 7.0],
                ZERO,
                &[cube([-2.0, 0.0, -2.0], [4, 6, 4], [0, 16], 0.5, false)],
            ),
            bone(
                "left_hind_leg",
                None,
                [3.0, 12.0, 7.0],
                ZERO,
                &[cube([-2.0, 0.0, -2.0], [4, 6, 4], [0, 16], 0.5, false)],
            ),
            bone(
                "right_front_leg",
                None,
                [-3.0, 12.0, -5.0],
                ZERO,
                &[cube([-2.0, 0.0, -2.0], [4, 6, 4], [0, 16], 0.5, false)],
            ),
            bone(
                "left_front_leg",
                None,
                [3.0, 12.0, -5.0],
                ZERO,
                &[cube([-2.0, 0.0, -2.0], [4, 6, 4], [0, 16], 0.5, false)],
            ),
        ],
    },
    // `ChickenModel`: the 16-px body pivot leaves the legs at y=19.
    Model {
        name: "minecraft:chicken#main",
        texture: "textures/entity/chicken.png",
        texture_size: [64, 32],
        bones: &[
            bone(
                "head",
                None,
                [0.0, 15.0, -4.0],
                ZERO,
                &[cube([-2.0, -6.0, -2.0], [4, 6, 3], [0, 0], 0.0, false)],
            ),
            bone(
                "beak",
                None,
                [0.0, 15.0, -4.0],
                ZERO,
                &[cube([-2.0, -4.0, -4.0], [4, 2, 2], [14, 0], 0.0, false)],
            ),
            bone(
                "waddle",
                None,
                [0.0, 15.0, -4.0],
                ZERO,
                &[cube([-1.0, -2.0, -3.0], [2, 2, 2], [14, 4], 0.0, false)],
            ),
            bone(
                "body",
                None,
                [0.0, 16.0, 0.0],
                LIE_DOWN,
                &[cube([-3.0, -4.0, -3.0], [6, 8, 6], [0, 9], 0.0, false)],
            ),
            bone(
                "right_leg",
                None,
                [-2.0, 19.0, 1.0],
                ZERO,
                &[cube([-1.0, 0.0, -3.0], [3, 5, 3], [26, 0], 0.0, false)],
            ),
            bone(
                "left_leg",
                None,
                [1.0, 19.0, 1.0],
                ZERO,
                &[cube([-1.0, 0.0, -3.0], [3, 5, 3], [26, 0], 0.0, false)],
            ),
            bone(
                "right_wing",
                None,
                [-4.0, 13.0, 0.0],
                ZERO,
                &[cube([0.0, 0.0, -3.0], [1, 4, 6], [24, 13], 0.0, false)],
            ),
            bone(
                "left_wing",
                None,
                [4.0, 13.0, 0.0],
                ZERO,
                &[cube([-1.0, 0.0, -3.0], [1, 4, 6], [24, 13], 0.0, false)],
            ),
        ],
    },
    // `SpiderModel`: eight 16x2x2 legs, splayed by the era's fixed y/z rotations.
    Model {
        name: "minecraft:spider#main",
        texture: "textures/entity/spider/spider.png",
        texture_size: [64, 32],
        bones: &[
            bone(
                "head",
                None,
                [0.0, 15.0, -3.0],
                ZERO,
                &[cube([-4.0, -4.0, -8.0], [8, 8, 8], [32, 4], 0.0, false)],
            ),
            bone(
                "neck",
                None,
                [0.0, 15.0, 0.0],
                ZERO,
                &[cube([-3.0, -3.0, -3.0], [6, 6, 6], [0, 0], 0.0, false)],
            ),
            bone(
                "body",
                None,
                [0.0, 15.0, 9.0],
                ZERO,
                &[cube([-5.0, -4.0, -6.0], [10, 8, 12], [0, 12], 0.0, false)],
            ),
            bone(
                "right_hind_leg",
                None,
                [-4.0, 15.0, 2.0],
                [0.0, SPIDER_SPLAY * 2.0, -SPIDER_SPLAY],
                &[cube([-15.0, -1.0, -1.0], [16, 2, 2], [18, 0], 0.0, false)],
            ),
            bone(
                "left_hind_leg",
                None,
                [4.0, 15.0, 2.0],
                [0.0, -SPIDER_SPLAY * 2.0, SPIDER_SPLAY],
                &[cube([-1.0, -1.0, -1.0], [16, 2, 2], [18, 0], 0.0, false)],
            ),
            bone(
                "right_middle_hind_leg",
                None,
                [-4.0, 15.0, 1.0],
                [0.0, SPIDER_SPLAY, -SPIDER_SPLAY * 0.74],
                &[cube([-15.0, -1.0, -1.0], [16, 2, 2], [18, 0], 0.0, false)],
            ),
            bone(
                "left_middle_hind_leg",
                None,
                [4.0, 15.0, 1.0],
                [0.0, -SPIDER_SPLAY, SPIDER_SPLAY * 0.74],
                &[cube([-1.0, -1.0, -1.0], [16, 2, 2], [18, 0], 0.0, false)],
            ),
            bone(
                "right_middle_front_leg",
                None,
                [-4.0, 15.0, 0.0],
                [0.0, -SPIDER_SPLAY, -SPIDER_SPLAY * 0.74],
                &[cube([-15.0, -1.0, -1.0], [16, 2, 2], [18, 0], 0.0, false)],
            ),
            bone(
                "left_middle_front_leg",
                None,
                [4.0, 15.0, 0.0],
                [0.0, SPIDER_SPLAY, SPIDER_SPLAY * 0.74],
                &[cube([-1.0, -1.0, -1.0], [16, 2, 2], [18, 0], 0.0, false)],
            ),
            bone(
                "right_front_leg",
                None,
                [-4.0, 15.0, -1.0],
                [0.0, -SPIDER_SPLAY * 2.0, -SPIDER_SPLAY],
                &[cube([-15.0, -1.0, -1.0], [16, 2, 2], [18, 0], 0.0, false)],
            ),
            bone(
                "left_front_leg",
                None,
                [4.0, 15.0, -1.0],
                [0.0, SPIDER_SPLAY * 2.0, SPIDER_SPLAY],
                &[cube([-1.0, -1.0, -1.0], [16, 2, 2], [18, 0], 0.0, false)],
            ),
        ],
    },
];

/// The era's entity sheet under each name the engine's species table asks for.
///
/// A 1.7.10 skin is at `textures/entity/steve.png` and its pig at `textures/entity/pig/pig.png`;
/// 26.3 names them `entity/player/wide/steve` and `entity/pig/pig_temperate`. The HUD atlas
/// packs the era's tree under its own names, so this is the alias list that lets the engine's
/// table find them. Names that already match (sheep, zombie, ...) are listed too, so a table
/// change shows here.
pub const TEXTURES_TABLE: &[TextureName] = &[
    TextureName {
        name: "entity/player/wide/steve",
        path: "textures/entity/steve.png",
    },
    TextureName {
        name: "entity/player/slim/steve",
        path: "textures/entity/steve.png",
    },
    TextureName {
        name: "entity/zombie/zombie",
        path: "textures/entity/zombie/zombie.png",
    },
    TextureName {
        name: "entity/skeleton/skeleton",
        path: "textures/entity/skeleton/skeleton.png",
    },
    TextureName {
        name: "entity/creeper/creeper",
        path: "textures/entity/creeper/creeper.png",
    },
    TextureName {
        name: "entity/cow/cow_temperate",
        path: "textures/entity/cow/cow.png",
    },
    TextureName {
        name: "entity/pig/pig_temperate",
        path: "textures/entity/pig/pig.png",
    },
    TextureName {
        name: "entity/sheep/sheep",
        path: "textures/entity/sheep/sheep.png",
    },
    TextureName {
        name: "entity/sheep/sheep_wool",
        path: "textures/entity/sheep/sheep_fur.png",
    },
    TextureName {
        name: "entity/chicken/chicken_temperate",
        path: "textures/entity/chicken.png",
    },
    TextureName {
        name: "entity/spider/spider",
        path: "textures/entity/spider/spider.png",
    },
];

/// The era's entity types the covered models draw: `EntityType.getDimensions` and each class's
/// `setSize`/`initAttributes`. A living entity's eye height is `height * 0.85`
/// (`LivingEntity.getEyeHeight`).
pub const TYPES_TABLE: &[EntityType] = &[
    EntityType {
        id: "minecraft:pig",
        width: 0.9,
        height: 0.9,
        eye_height: 0.765,
        category: "creature",
        fire_immune: false,
        attributes: &[
            ("minecraft:max_health", 10.0),
            ("minecraft:movement_speed", 0.25),
        ],
    },
    EntityType {
        id: "minecraft:cow",
        width: 0.9,
        height: 1.3,
        eye_height: 1.105,
        category: "creature",
        fire_immune: false,
        attributes: &[
            ("minecraft:max_health", 10.0),
            ("minecraft:movement_speed", 0.2),
        ],
    },
    EntityType {
        id: "minecraft:sheep",
        width: 0.9,
        height: 1.3,
        eye_height: 1.105,
        category: "creature",
        fire_immune: false,
        attributes: &[
            ("minecraft:max_health", 8.0),
            ("minecraft:movement_speed", 0.23),
        ],
    },
    EntityType {
        id: "minecraft:chicken",
        width: 0.3,
        height: 0.7,
        eye_height: 0.595,
        category: "creature",
        fire_immune: false,
        attributes: &[
            ("minecraft:max_health", 4.0),
            ("minecraft:movement_speed", 0.25),
        ],
    },
    EntityType {
        id: "minecraft:zombie",
        width: 0.6,
        height: 1.8,
        eye_height: 1.53,
        category: "monster",
        fire_immune: false,
        attributes: &[
            ("minecraft:max_health", 20.0),
            ("minecraft:movement_speed", 0.23),
            ("minecraft:attack_damage", 3.0),
            ("minecraft:follow_range", 40.0),
        ],
    },
    EntityType {
        id: "minecraft:skeleton",
        width: 0.6,
        height: 1.8,
        eye_height: 1.53,
        category: "monster",
        fire_immune: false,
        attributes: &[
            ("minecraft:max_health", 20.0),
            ("minecraft:movement_speed", 0.25),
            ("minecraft:attack_damage", 2.0),
        ],
    },
    EntityType {
        id: "minecraft:creeper",
        width: 0.6,
        height: 1.8,
        eye_height: 1.53,
        category: "monster",
        fire_immune: false,
        attributes: &[
            ("minecraft:max_health", 20.0),
            ("minecraft:movement_speed", 0.25),
        ],
    },
    EntityType {
        id: "minecraft:spider",
        width: 1.4,
        height: 0.9,
        eye_height: 0.765,
        category: "monster",
        fire_immune: false,
        attributes: &[
            ("minecraft:max_health", 16.0),
            ("minecraft:movement_speed", 0.8),
            ("minecraft:attack_damage", 2.0),
        ],
    },
];

/// Every covered era model, in table order.
pub fn models() -> &'static [Model] {
    MODELS_TABLE
}

/// Every texture name the engine's species table asks for, with the era's file.
pub fn textures() -> &'static [TextureName] {
    TEXTURES_TABLE
}

/// Every covered era entity type.
pub fn entity_types() -> &'static [EntityType] {
    TYPES_TABLE
}

/// The models as the engine's `entity-models.json`: one part tree per layer key, each part's
/// boxes baked to the six quads `Box` builds (`Box`'s constructor is the box-to-quad mapping).
pub fn bake_models() -> Value {
    let mut document = Value::Object(Vec::new());
    for model in models() {
        document.insert(model.name, bake_model(model));
    }
    document
}

/// The entity types as the engine's `entity-types.json`.
pub fn bake_types() -> Value {
    let mut document = Value::Object(Vec::new());
    for kind in entity_types() {
        let mut attributes = Value::Object(Vec::new());
        for (name, value) in kind.attributes {
            attributes.insert(name, Value::Float(f64::from(*value)));
        }
        document.insert(
            kind.id,
            Value::Object(vec![
                ("width".into(), Value::Float(f64::from(kind.width))),
                ("height".into(), Value::Float(f64::from(kind.height))),
                (
                    "eye_height".into(),
                    Value::Float(f64::from(kind.eye_height)),
                ),
                ("category".into(), Value::Str(kind.category.into())),
                ("fire_immune".into(), Value::Bool(kind.fire_immune)),
                ("attributes".into(), attributes),
            ]),
        );
    }
    document
}

/// Write [`MODELS`] and [`TYPES`] into a prepared 1.7.10 folder, rewriting a file only when its
/// bytes changed.
pub fn write(dir: &Path, log: Log) -> Result<(), String> {
    let baked = crate::pyjson::dumps(&bake_models(), &crate::pyjson::INDENT2) + "\n";
    write_if_changed(&dir.join(MODELS), baked.as_bytes())?;
    let types = crate::pyjson::dumps(&bake_types(), &crate::pyjson::INDENT2) + "\n";
    write_if_changed(&dir.join(TYPES), types.as_bytes())?;
    log(&format!(
        "{} entity models and {} entity types from the era's own classes",
        models().len(),
        entity_types().len()
    ));
    Ok(())
}

fn bake_model(model: &Model) -> Value {
    Value::Object(vec![
        (
            "pose".into(),
            pose([0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0]),
        ),
        ("polygons".into(), Value::Array(Vec::new())),
        ("normals".into(), Value::Array(Vec::new())),
        ("children".into(), Value::Object(bake_children(model, None))),
    ])
}

fn bake_children(model: &Model, parent: Option<&str>) -> Vec<(String, Value)> {
    model
        .bones
        .iter()
        .filter(|bone| bone.parent == parent)
        .map(|bone| (bone.name.to_owned(), bake_bone(model, bone)))
        .collect()
}

fn bake_bone(model: &Model, bone: &Bone) -> Value {
    let mut polygons = Vec::new();
    let mut normals = Vec::new();
    for cube in bone.cubes {
        bake_cube(model, cube, &mut polygons, &mut normals);
    }
    Value::Object(vec![
        (
            "pose".into(),
            pose([
                bone.pivot[0],
                bone.pivot[1],
                bone.pivot[2],
                bone.rotation[0],
                bone.rotation[1],
                bone.rotation[2],
                1.0,
                1.0,
                1.0,
            ]),
        ),
        ("polygons".into(), Value::Array(polygons)),
        ("normals".into(), Value::Array(normals)),
        (
            "children".into(),
            Value::Object(bake_children(model, Some(bone.name))),
        ),
    ])
}

fn pose(values: [f32; 9]) -> Value {
    Value::Array(
        values
            .into_iter()
            .map(|value| Value::Float(f64::from(value)))
            .collect(),
    )
}

/// `Box`'s constructor: the eight corners of the (inflated, possibly mirrored) box, the six
/// quads with their texture rectangles, and each quad's outward normal (`Polygon.compile`).
fn bake_cube(model: &Model, cube: &Cube, polygons: &mut Vec<Value>, normals: &mut Vec<Value>) {
    let width = model.texture_size[0] as f32;
    let height = model.texture_size[1] as f32;
    let grow = cube.inflate;
    let mut min = [
        cube.origin[0] - grow,
        cube.origin[1] - grow,
        cube.origin[2] - grow,
    ];
    let mut max = [
        cube.origin[0] + cube.size[0] as f32 + grow,
        cube.origin[1] + cube.size[1] as f32 + grow,
        cube.origin[2] + cube.size[2] as f32 + grow,
    ];
    if cube.mirror {
        std::mem::swap(&mut min[0], &mut max[0]);
    }
    let corners = [
        [min[0], min[1], min[2]],
        [max[0], min[1], min[2]],
        [max[0], max[1], min[2]],
        [min[0], max[1], min[2]],
        [min[0], min[1], max[2]],
        [max[0], min[1], max[2]],
        [max[0], max[1], max[2]],
        [min[0], max[1], max[2]],
    ];
    let u = cube.uv[0];
    let v = cube.uv[1];
    let (size_x, size_y, size_z) = (
        cube.size[0] as i32,
        cube.size[1] as i32,
        cube.size[2] as i32,
    );
    // (corner indices, u1, v1, u2, v2) in `Box`'s order.
    let faces: [([usize; 4], [i32; 4]); 6] = [
        (
            [5, 1, 2, 6],
            [
                u + size_z + size_x,
                v + size_z,
                u + size_z + size_x + size_z,
                v + size_z + size_y,
            ],
        ),
        (
            [0, 4, 7, 3],
            [u, v + size_z, u + size_z, v + size_z + size_y],
        ),
        (
            [5, 4, 0, 1],
            [u + size_z, v, u + size_z + size_x, v + size_z],
        ),
        (
            [2, 3, 7, 6],
            [
                u + size_z + size_x,
                v + size_z,
                u + size_z + size_x + size_x,
                v,
            ],
        ),
        (
            [1, 0, 3, 2],
            [
                u + size_z,
                v + size_z,
                u + size_z + size_x,
                v + size_z + size_y,
            ],
        ),
        (
            [4, 5, 6, 7],
            [
                u + size_z + size_x + size_z,
                v + size_z,
                u + size_z + size_x + size_z + size_x,
                v + size_z + size_y,
            ],
        ),
    ];
    for (indices, [u1, v1, u2, v2]) in faces {
        let mut quad: Vec<([f32; 3], [f32; 2])> = indices
            .iter()
            .zip([
                [u2 as f32 / width, v1 as f32 / height],
                [u1 as f32 / width, v1 as f32 / height],
                [u1 as f32 / width, v2 as f32 / height],
                [u2 as f32 / width, v2 as f32 / height],
            ])
            .map(|(index, uv)| (corners[*index], uv))
            .collect();
        if cube.mirror {
            quad.reverse();
        }
        let normal = quad_normal(&quad);
        polygons.push(Value::Array(
            quad.iter()
                .map(|([x, y, z], [u, v])| {
                    Value::Array(vec![
                        Value::Float(f64::from(*x)),
                        Value::Float(f64::from(*y)),
                        Value::Float(f64::from(*z)),
                        Value::Float(f64::from(*u)),
                        Value::Float(f64::from(*v)),
                    ])
                })
                .collect(),
        ));
        normals.push(Value::Array(
            normal
                .into_iter()
                .map(|value| Value::Float(f64::from(value)))
                .collect(),
        ));
    }
}

/// `Polygon.compile`: `(v2 - v1) x (v0 - v1)`, normalized.
fn quad_normal(quad: &[([f32; 3], [f32; 2])]) -> [f32; 3] {
    let [v0, v1, v2] = [quad[0].0, quad[1].0, quad[2].0];
    let a = [v2[0] - v1[0], v2[1] - v1[1], v2[2] - v1[2]];
    let b = [v0[0] - v1[0], v0[1] - v1[1], v0[2] - v1[2]];
    let cross = [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ];
    let length = (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt();
    if length < 1.0e-4 {
        [0.0; 3]
    } else {
        [cross[0] / length, cross[1] / length, cross[2] / length]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(name: &str) -> &'static Model {
        models()
            .iter()
            .find(|model| model.name == name)
            .unwrap_or_else(|| panic!("{name} is not in the era's table"))
    }

    fn bone<'a>(model: &'a Model, name: &str) -> &'a Bone {
        model
            .bones
            .iter()
            .find(|bone| bone.name == name)
            .unwrap_or_else(|| panic!("{} has no {name}", model.name))
    }

    /// `ModelBiped`/`HumanoidModel`: the head, body and hat boxes, and the hat's 0.5-px
    /// inflation, straight from the era's constructor.
    #[test]
    fn the_player_is_the_eras_model_biped() {
        let player = model("minecraft:player#main");
        assert_eq!(player.texture, "textures/entity/steve.png");
        assert_eq!(player.texture_size, [64, 32]);
        let head = bone(player, "head");
        assert_eq!(head.pivot, [0.0, 0.0, 0.0]);
        assert_eq!(
            head.cubes,
            &[cube([-4.0, -8.0, -4.0], [8, 8, 8], [0, 0], 0.0, false)]
        );
        let body = bone(player, "body");
        assert_eq!(
            body.cubes,
            &[cube([-4.0, 0.0, -2.0], [8, 12, 4], [16, 16], 0.0, false)]
        );
        let hat = bone(player, "hat");
        assert_eq!(hat.parent, Some("head"));
        assert_eq!(
            hat.cubes[0].inflate, 0.5,
            "the outer hat layer is inflated 0.5"
        );
        assert_eq!(hat.cubes[0].uv, [32, 0]);
        // Both arms and both legs are there; the left ones mirror the right ones' texture.
        assert_eq!(bone(player, "right_arm").cubes[0].size, [4, 12, 4]);
        assert_eq!(bone(player, "right_arm").cubes[0].uv, [40, 16]);
        assert!(!bone(player, "right_arm").cubes[0].mirror);
        assert!(bone(player, "left_arm").cubes[0].mirror);
        assert_eq!(bone(player, "left_leg").pivot, [1.9, 12.0, 0.1]);
    }

    /// A zombie's sheet is 64x64, so the same head box lands on different pixels than a
    /// player's 64x32 one.
    #[test]
    fn the_zombie_is_the_same_biped_on_a_taller_sheet() {
        let zombie = model("minecraft:zombie#main");
        assert_eq!(zombie.texture_size, [64, 64]);
        assert_eq!(bone(zombie, "head").cubes[0].uv, [0, 0]);
        assert_eq!(bone(zombie, "hat").cubes[0].inflate, 0.5);
    }

    /// `SkeletonModel`'s own arms and legs: 2x12x2 at texture offset 40,16 and 0,16.
    #[test]
    fn the_skeleton_has_the_eras_thin_limbs() {
        let skeleton = model("minecraft:skeleton#main");
        assert_eq!(bone(skeleton, "right_arm").cubes[0].size, [2, 12, 2]);
        assert_eq!(bone(skeleton, "right_arm").cubes[0].uv, [40, 16]);
        assert_eq!(bone(skeleton, "right_leg").cubes[0].size, [2, 12, 2]);
        assert_eq!(bone(skeleton, "right_leg").pivot, [-2.0, 12.0, 0.0]);
        assert_eq!(skeleton.texture_size, [64, 32]);
    }

    /// `CreeperModel`'s 4x6x4 legs sit at y=16, and its hat is the same 8x8x8 box grown 0.5.
    #[test]
    fn the_creeper_has_four_short_legs_at_the_era_offsets() {
        let creeper = model("minecraft:creeper#main");
        assert_eq!(bone(creeper, "head").pivot, [0.0, 4.0, 0.0]);
        assert_eq!(bone(creeper, "head").cubes[0].size, [8, 8, 8]);
        assert_eq!(bone(creeper, "hat").cubes[0].inflate, 0.5);
        assert_eq!(bone(creeper, "body").cubes[0].size, [8, 12, 4]);
        assert_eq!(bone(creeper, "left_hind_leg").pivot, [-2.0, 16.0, 4.0]);
        assert_eq!(bone(creeper, "right_front_leg").pivot, [2.0, 16.0, -4.0]);
        assert_eq!(bone(creeper, "left_hind_leg").cubes[0].size, [4, 6, 4]);
    }

    /// `QuadrupedModel` and the pig/cow/sheep overrides: the era's pivots, body angle and size.
    #[test]
    fn the_quadrupeds_keep_the_eras_proportions() {
        let pig = model("minecraft:pig#main");
        assert_eq!(bone(pig, "head").pivot, [0.0, 12.0, -6.0]);
        assert_eq!(bone(pig, "head").cubes[0].size, [8, 8, 8]);
        assert_eq!(bone(pig, "head").cubes[1].uv, [16, 16], "the snout");
        assert_eq!(bone(pig, "head").cubes[1].origin, [-2.0, 0.0, -9.0]);
        assert_eq!(bone(pig, "body").rotation, LIE_DOWN);
        assert_eq!(bone(pig, "body").pivot, [0.0, 11.0, 2.0]);

        let cow = model("minecraft:cow#main");
        assert_eq!(bone(cow, "head").pivot, [0.0, 4.0, -8.0]);
        assert_eq!(bone(cow, "body").cubes[0].size, [12, 18, 10]);
        assert_eq!(bone(cow, "right_front_leg").pivot, [-4.0, 12.0, -6.0]);

        let sheep = model("minecraft:sheep#main");
        assert_eq!(bone(sheep, "head").cubes[0].size, [6, 6, 8]);
        assert_eq!(bone(sheep, "body").cubes[0].size, [8, 16, 6]);

        let wool = model("minecraft:sheep#wool");
        assert_eq!(bone(wool, "body").cubes[0].inflate, 1.75);
    }

    /// `ChickenModel`: the body is a 6x8x6 box lying down and the legs hang from y=19.
    #[test]
    fn the_chicken_is_the_eras_eight_part_model() {
        let chicken = model("minecraft:chicken#main");
        assert_eq!(chicken.bones.len(), 8);
        assert_eq!(bone(chicken, "body").cubes[0].size, [6, 8, 6]);
        assert_eq!(bone(chicken, "body").rotation, LIE_DOWN);
        assert_eq!(bone(chicken, "right_leg").pivot, [-2.0, 19.0, 1.0]);
        assert_eq!(bone(chicken, "left_wing").pivot, [4.0, 13.0, 0.0]);
    }

    /// `SpiderModel`: eight 16x2x2 legs, the outer pair splayed PI/4 and the middle PI/8.
    #[test]
    fn the_spider_has_eight_legs_with_the_eras_splay() {
        let spider = model("minecraft:spider#main");
        assert_eq!(spider.bones.len(), 11);
        let leg = bone(spider, "right_front_leg");
        assert_eq!(leg.cubes[0].size, [16, 2, 2]);
        assert_eq!(leg.pivot, [-4.0, 15.0, -1.0]);
        assert_eq!(leg.rotation[1], -SPIDER_SPLAY * 2.0);
        assert_eq!(leg.rotation[2], -SPIDER_SPLAY);
        let middle = bone(spider, "left_middle_hind_leg");
        assert_eq!(middle.rotation[2], SPIDER_SPLAY * 0.74);
    }

    /// `Box`'s texture unwrap: a ModelBiped head (uv 0,0, 8x8x8) on a 64x32 sheet has its
    /// -Z (front) quad between (8/64, 8/32) and (16/64, 16/32), and its +Y (model-down) quad
    /// in the texture's second band, which the era flips along U.
    #[test]
    fn the_baked_quads_are_the_eras_texture_unwrap() {
        let player = model("minecraft:player#main");
        let mut polygons = Vec::new();
        let mut normals = Vec::new();
        bake_cube(
            player,
            &bone(player, "head").cubes[0],
            &mut polygons,
            &mut normals,
        );
        assert_eq!(polygons.len(), 6);
        assert_eq!(normals.len(), 6);
        let quad = |index: usize| -> Vec<[f32; 5]> {
            polygons[index]
                .as_array()
                .unwrap()
                .iter()
                .map(|vertex| {
                    let values = vertex.as_array().unwrap();
                    std::array::from_fn(|i| match &values[i] {
                        Value::Float(value) => *value as f32,
                        _ => panic!("not a float"),
                    })
                })
                .collect()
        };
        // The -Z face (`faces[4]`) starts at (u + sizeZ, v + sizeZ) = (8, 8) and spans
        // 8x8 texels: (8/64, 8/32) .. (16/64, 16/32).
        let north = quad(4);
        assert_eq!(
            [north[0][3], north[0][4]],
            [16.0 / 64.0, 8.0 / 32.0],
            "the front face's first corner is (u2, v1)"
        );
        assert_eq!([north[1][3], north[1][4]], [8.0 / 64.0, 8.0 / 32.0]);
        assert_eq!([north[3][3], north[3][4]], [16.0 / 64.0, 16.0 / 32.0]);
        // The +Y face (`faces[3]`, model-down) sits at u = 8 + 8 = 16 .. 24 and runs from
        // v = 8 up to v = 0, so its first vertex takes the region's right edge.
        let down = quad(3);
        assert_eq!(down[0][3], 24.0 / 64.0);
        assert_eq!(down[0][4], 8.0 / 32.0);
        assert_eq!(down[2][3], 16.0 / 64.0);
        assert_eq!(down[2][4], 0.0);
        // Its normal points +Y.
        let normal = normals[3].as_array().unwrap();
        assert_eq!(normal[1], Value::Float(1.0));
        // The corners carry the box's own coordinates: the head spans y -8..0.
        assert_eq!(&north[1][0..3], &[-4.0, -8.0, -4.0]);
        assert_eq!(&north[3][0..3], &[4.0, 0.0, -4.0]);
    }

    /// The baked document is the engine's: a pose of nine numbers, four-vertex quads with
    /// one normal each, and the hat under the head.
    #[test]
    fn the_baked_document_has_the_engines_shape() {
        let document = bake_models();
        for model in models() {
            let part = document
                .get(model.name)
                .expect("every model is in the document");
            assert_eq!(part.get("pose").and_then(Value::as_array).unwrap().len(), 9);
            walk(part);
        }
        fn walk(part: &Value) {
            let pose = part.get("pose").and_then(Value::as_array).unwrap();
            assert_eq!(pose.len(), 9);
            // A part with a zero scale is not drawn (nor are its children): the root's
            // pose must keep the identity scale, as the 26.3 exporter's does.
            for scale in &pose[6..9] {
                assert_eq!(scale, &Value::Float(1.0));
            }
            let polygons = part.get("polygons").and_then(Value::as_array).unwrap();
            let normals = part.get("normals").and_then(Value::as_array).unwrap();
            assert_eq!(polygons.len(), normals.len());
            for polygon in polygons {
                let vertices = polygon.as_array().unwrap();
                assert_eq!(vertices.len(), 4);
                for vertex in vertices {
                    assert_eq!(vertex.as_array().unwrap().len(), 5);
                }
            }
            for (name, child) in part.get("children").and_then(Value::as_object).unwrap() {
                assert_eq!(
                    child.get("pose").and_then(Value::as_array).unwrap().len(),
                    9
                );
                assert!(!name.is_empty());
                walk(child);
            }
        }
        let player = document.get("minecraft:player#main").unwrap();
        let head = player
            .get("children")
            .and_then(|children| children.get("head"))
            .unwrap();
        assert!(head
            .get("children")
            .and_then(|children| children.get("hat"))
            .is_some());
    }

    /// The era's own dimensions and base attributes, from the entity classes.
    #[test]
    fn the_era_entity_types_carry_the_eras_numbers() {
        let types = bake_types();
        let row = |id: &str| types.get(id).unwrap();
        let attr = |id: &str, name: &str| {
            row(id)
                .get("attributes")
                .and_then(|attrs| attrs.get(name))
                .unwrap()
        };
        assert_eq!(
            row("minecraft:chicken").get("width"),
            Some(&Value::Float(f64::from(0.3f32)))
        );
        assert_eq!(
            row("minecraft:chicken").get("eye_height"),
            Some(&Value::Float(f64::from(0.595f32)))
        );
        assert_eq!(
            row("minecraft:zombie").get("category"),
            Some(&Value::Str("monster".into()))
        );
        assert_eq!(
            attr("minecraft:zombie", "minecraft:follow_range"),
            &Value::Float(40.0)
        );
        assert_eq!(
            attr("minecraft:zombie", "minecraft:attack_damage"),
            &Value::Float(3.0)
        );
        assert_eq!(
            attr("minecraft:spider", "minecraft:movement_speed"),
            &Value::Float(f64::from(0.8f32))
        );
        assert_eq!(
            attr("minecraft:sheep", "minecraft:max_health"),
            &Value::Float(8.0)
        );
        assert_eq!(
            row("minecraft:spider").get("width"),
            Some(&Value::Float(f64::from(1.4f32)))
        );
    }

    /// Every covered model has the sheet its type renders with, and every alias names a file
    /// the extraction copies.
    #[test]
    fn every_model_and_alias_names_a_file_the_extraction_has() {
        let extracted =
            |path: &str| crate::assets_1_7_10::is_asset(&format!("assets/minecraft/{path}"));
        for model in models() {
            assert!(
                model.texture.starts_with("textures/entity/"),
                "{} names {}",
                model.name,
                model.texture
            );
            assert!(
                extracted(model.texture),
                "{} is not extracted",
                model.texture
            );
        }
        for texture in textures() {
            assert!(texture.path.starts_with("textures/entity/"));
            assert!(extracted(texture.path), "{} is not extracted", texture.path);
        }
    }
}
