//! Deterministic dump of the engine's 1.7.10 block renderer's vertex stream.
//!
//! Every case mirrors `tools/javaoracle_1_7_10/RenderOracle.java` and the
//! reference port's `render_probe` line for line: the same one-chunk world, the
//! same block table, the same light pattern and the same sprite atlas. Each
//! vertex prints the raw 32-byte vertex words as the IEEE-754 bit patterns of
//! x, y, z, u and v, the packed colour int and the packed brightness int.
//!
//! The regression harness is a line diff against the real 1.7.10 client:
//!
//! ```sh
//! sh tools/javaoracle_1_7_10/build.sh
//! sh tools/javaoracle_1_7_10/run.sh RenderOracle > /tmp/oracle.txt
//! cargo run -p sunrise-client-render --bin sunset17_render_probe > /tmp/engine.txt
//! diff /tmp/oracle.txt /tmp/engine.txt
//! ```
//!
//! Any difference is a renderer bug: the streams must stay bit-exact. The
//! fixture also matches `sunset-1-7-render`'s `render_probe` (the port the
//! oracle was first checked against).

use sunrise_client_render::mesher::sunset17::{
    fixture::{self, TestWorld},
    render::{RenderBlocks, RenderOptions},
    tesselator::Vertex,
    world::WorldView,
};

const SIZE: i32 = 16;

/// The oracle's light pattern, see `RenderOracle.LIGHT`.
fn sky_light(x: i32, y: i32, z: i32) -> u8 {
    ((x * 5 + y * 3 + z * 7) & 15) as u8
}

fn block_light(x: i32, y: i32, z: i32) -> u8 {
    ((x * 11 + y * 2 + z * 13) & 15) as u8
}

fn prepare_world() -> TestWorld {
    let mut world = TestWorld::new();
    for y in 0..SIZE {
        for z in 0..SIZE {
            for x in 0..SIZE {
                world.set_sky_light(x, y, z, sky_light(x, y, z));
                world.set_block_light(x, y, z, block_light(x, y, z));
            }
        }
    }
    world
}

/// One probe case: the name `RenderOracle` prints, the client options, and the
/// block at a position.
struct Case<'a> {
    name: &'a str,
    ao: i32,
    fancy: bool,
    block: &'a str,
    meta: u8,
    at: [i32; 3],
}

/// The era vertex stream of one case: `(rendered, vertices)`.
fn render(case: Case<'_>, build: impl FnOnce(&mut TestWorld)) -> (bool, Vec<Vertex>) {
    let Case {
        name: _,
        ao,
        fancy,
        block,
        meta,
        at: [x, y, z],
    } = case;
    let mut world = prepare_world();
    build(&mut world);
    world.set(x, y, z, fixture::id(block) as u8, meta);
    let def = world.state(x, y, z).block;
    let options = RenderOptions {
        ao,
        fancy_graphics: fancy,
        anaglyph: false,
    };
    let mut renderer = RenderBlocks::new(&world, options);
    let rendered = renderer.tesselate_in_world(&def, x, y, z);
    (rendered, renderer.tesselator.vertices)
}

/// One line per vertex, the format `render_probe` and `RenderOracle` print.
fn line(index: usize, vertex: &Vertex) -> String {
    format!(
        "v{index} x={:08X} y={:08X} z={:08X} u={:08X} v={:08X} color={:08X} light={}",
        vertex.x.to_bits(),
        vertex.y.to_bits(),
        vertex.z.to_bits(),
        vertex.u.to_bits(),
        vertex.v.to_bits(),
        vertex.color as u32,
        vertex.brightness
    )
}

fn case(case: Case<'_>, build: impl FnOnce(&mut TestWorld)) {
    let name = case.name;
    let (rendered, vertices) = render(case, build);
    println!("== {name} rendered={rendered} vertices={}", vertices.len());
    for (index, vertex) in vertices.iter().enumerate() {
        println!("{}", line(index, vertex));
    }
}

fn main() {
    for ao in 0..=2 {
        case(
            Case {
                name: &format!("stone_ao{ao}"),
                ao,
                fancy: false,
                block: "stone",
                meta: 0,
                at: [4, 4, 4],
            },
            |world| {
                world.set(4, 3, 4, fixture::id("stone") as u8, 0);
            },
        );
    }
    for ao in 0..=1 {
        case(
            Case {
                name: &format!("stone_culled_ao{ao}"),
                ao,
                fancy: false,
                block: "stone",
                meta: 0,
                at: [4, 4, 4],
            },
            |world| {
                let stone = fixture::id("stone") as u8;
                world.set(4, 3, 4, stone, 0);
                world.set(5, 4, 4, stone, 0);
                world.set(4, 4, 5, stone, 0);
            },
        );
    }
    for ao in 0..=1 {
        for (fancy, suffix) in [(true, "fancy"), (false, "plain")] {
            case(
                Case {
                    name: &format!("grass_ao{ao}_{suffix}"),
                    ao,
                    fancy,
                    block: "grass",
                    meta: 0,
                    at: [5, 4, 4],
                },
                |world| {
                    world.set(5, 3, 4, fixture::id("dirt") as u8, 0);
                },
            );
        }
    }
    for (meta, axis) in [(0, "y"), (4, "x"), (8, "z")] {
        for ao in 0..=1 {
            case(
                Case {
                    name: &format!("log_{axis}_ao{ao}"),
                    ao,
                    fancy: false,
                    block: "log",
                    meta,
                    at: [6, 4, 4],
                },
                |world| {
                    world.set(6, 3, 4, fixture::id("stone") as u8, 0);
                },
            );
        }
    }
    for ao in 0..=1 {
        case(
            Case {
                name: &format!("glass_ao{ao}"),
                ao,
                fancy: false,
                block: "glass",
                meta: 0,
                at: [7, 4, 4],
            },
            |world| {
                let glass = fixture::id("glass") as u8;
                world.set(8, 4, 4, glass, 0);
                world.set(7, 4, 5, glass, 0);
                world.set(7, 3, 4, fixture::id("stone") as u8, 0);
            },
        );
    }
    for ao in 0..=1 {
        case(
            Case {
                name: &format!("torch_floor_ao{ao}"),
                ao,
                fancy: false,
                block: "torch",
                meta: 0,
                at: [9, 4, 4],
            },
            |world| {
                world.set(9, 3, 4, fixture::id("stone") as u8, 0);
            },
        );
    }
    case(
        Case {
            name: "torch_wall_ao1",
            ao: 1,
            fancy: false,
            block: "torch",
            meta: 1,
            at: [9, 4, 6],
        },
        |world| {
            world.set(10, 4, 6, fixture::id("stone") as u8, 0);
        },
    );
    for ao in 0..=2 {
        case(
            Case {
                name: &format!("stairs_east_ao{ao}"),
                ao,
                fancy: false,
                block: "oak_stairs",
                meta: 0,
                at: [2, 4, 2],
            },
            |world| {
                world.set(2, 3, 2, fixture::id("stone") as u8, 0);
            },
        );
        case(
            Case {
                name: &format!("stairs_upside_ao{ao}"),
                ao,
                fancy: false,
                block: "oak_stairs",
                meta: 4,
                at: [2, 4, 2],
            },
            |world| {
                world.set(2, 3, 2, fixture::id("stone") as u8, 0);
            },
        );
    }
    for ao in 0..=2 {
        case(
            Case {
                name: &format!("slab_bottom_ao{ao}"),
                ao,
                fancy: false,
                block: "stone_slab",
                meta: 0,
                at: [3, 4, 3],
            },
            |world| {
                world.set(3, 3, 3, fixture::id("stone") as u8, 0);
            },
        );
        case(
            Case {
                name: &format!("slab_top_ao{ao}"),
                ao,
                fancy: false,
                block: "stone_slab",
                meta: 8,
                at: [3, 4, 3],
            },
            |world| {
                world.set(3, 3, 3, fixture::id("stone") as u8, 0);
            },
        );
    }
    for ao in 0..=2 {
        case(
            Case {
                name: &format!("fence_ao{ao}"),
                ao,
                fancy: false,
                block: "fence",
                meta: 0,
                at: [6, 6, 4],
            },
            |world| {
                let stone = fixture::id("stone") as u8;
                world.set(5, 6, 4, stone, 0);
                world.set(7, 6, 4, stone, 0);
            },
        );
    }
    for (meta, name) in [(1, "tallgrass"), (2, "tallgrass")] {
        for ao in 0..=1 {
            case(
                Case {
                    name: &format!("{}_ao{ao}", if meta == 2 { "fern" } else { name }),
                    ao,
                    fancy: false,
                    block: name,
                    meta,
                    at: [2, 6, 4],
                },
                |world| {
                    world.set(2, 5, 4, fixture::id("dirt") as u8, 0);
                },
            );
        }
    }
    // A double plant: the lower half carries the variant, the upper half reads
    // it from the block below; both draw a one-block cross at the same x/z
    // offset (the hash has no y term), and variant 0's upper half adds the
    // sunflower's face cross.
    for ao in 0..=1 {
        case(
            Case {
                name: &format!("double_plant_grass_lower_ao{ao}"),
                ao,
                fancy: false,
                block: "double_plant",
                meta: 2,
                at: [3, 6, 4],
            },
            |world| {
                world.set(3, 5, 4, fixture::id("grass") as u8, 0);
            },
        );
        case(
            Case {
                name: &format!("double_plant_grass_upper_ao{ao}"),
                ao,
                fancy: false,
                block: "double_plant",
                meta: 8,
                at: [3, 6, 4],
            },
            |world| {
                world.set(3, 5, 4, fixture::id("double_plant") as u8, 2);
            },
        );
        case(
            Case {
                name: &format!("double_plant_fern_upper_ao{ao}"),
                ao,
                fancy: false,
                block: "double_plant",
                meta: 8,
                at: [10, 6, 4],
            },
            |world| {
                world.set(10, 5, 4, fixture::id("double_plant") as u8, 3);
            },
        );
        case(
            Case {
                name: &format!("double_plant_sunflower_lower_ao{ao}"),
                ao,
                fancy: false,
                block: "double_plant",
                meta: 0,
                at: [12, 6, 4],
            },
            |world| {
                world.set(12, 5, 4, fixture::id("grass") as u8, 0);
            },
        );
        case(
            Case {
                name: &format!("double_plant_sunflower_upper_ao{ao}"),
                ao,
                fancy: false,
                block: "double_plant",
                meta: 8,
                at: [12, 6, 4],
            },
            |world| {
                world.set(12, 5, 4, fixture::id("double_plant") as u8, 0);
            },
        );
    }
    // An upper half with no double plant below draws nothing.
    case(
        Case {
            name: "double_plant_upper_dangling_ao1",
            ao: 1,
            fancy: false,
            block: "double_plant",
            meta: 8,
            at: [6, 6, 8],
        },
        |world| {
            world.set(6, 5, 8, fixture::id("stone") as u8, 0);
        },
    );
    for ao in 0..=1 {
        case(
            Case {
                name: &format!("water_still_ao{ao}"),
                ao,
                fancy: false,
                block: "water",
                meta: 0,
                at: [8, 4, 8],
            },
            |world| {
                world.set(8, 3, 8, fixture::id("water") as u8, 0);
            },
        );
    }
    case(
        Case {
            name: "water_flow_ao1",
            ao: 1,
            fancy: false,
            block: "flowing_water",
            meta: 1,
            at: [8, 4, 10],
        },
        |world| {
            let water = fixture::id("water") as u8;
            world.set(7, 4, 10, water, 0);
            world.set(8, 3, 10, fixture::id("stone") as u8, 0);
        },
    );
    for ao in 0..=2 {
        case(
            Case {
                name: &format!("snow_ao{ao}"),
                ao,
                fancy: false,
                block: "snow_layer",
                meta: 3,
                at: [5, 6, 4],
            },
            |world| {
                world.set(5, 5, 4, fixture::id("stone") as u8, 0);
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Vertices of the real 1.7.10 client (`RenderOracle`, checked in through
    /// `tools/javaoracle_1_7_10/run.sh RenderOracle`), the values the probe's
    /// stream must keep. Three of stone's six faces pin the winding, the face
    /// shade, the sprite insets and the neighbour light.
    #[test]
    fn stone_matches_the_oracle_vert_for_vert() {
        let (rendered, vertices) = render(
            Case {
                name: "stone_ao0",
                ao: 0,
                fancy: false,
                block: "stone",
                meta: 0,
                at: [4, 4, 4],
            },
            |world| {
                world.set(4, 3, 4, fixture::id("stone") as u8, 0);
            },
        );
        assert!(rendered);
        assert_eq!(vertices.len(), 20);
        let expected = [
            "v0 x=40A00000 y=40A00000 z=40A00000 u=3D7FD70A v=3D7FD70A color=FFFFFFFF light=15728800",
            "v1 x=40A00000 y=40A00000 z=40800000 u=3D7FD70A v=3823D70A color=FFFFFFFF light=15728800",
            "v2 x=40800000 y=40A00000 z=40800000 u=3823D70A v=3823D70A color=FFFFFFFF light=15728800",
            "v3 x=40800000 y=40A00000 z=40A00000 u=3823D70A v=3D7FD70A color=FFFFFFFF light=15728800",
        ];
        for (index, expected) in expected.iter().enumerate() {
            assert_eq!(&line(index, &vertices[index]), expected);
        }
        // The stone below culls the bottom face: the next four are a 0.8 side
        // and the four after them a 0.6 side, the era's fixed face shade.
        assert_eq!(vertices[4].color as u32, 0xFFCCCCCC);
        assert_eq!(vertices[12].color as u32, 0xFF999999);
    }

    /// The flowing-water surface: the flow-angle rotated UVs and the corner
    /// heights the liquid renderer interpolates.
    #[test]
    fn flowing_water_matches_the_oracle() {
        let (rendered, vertices) = render(
            Case {
                name: "water_flow_ao1",
                ao: 1,
                fancy: false,
                block: "flowing_water",
                meta: 1,
                at: [8, 4, 10],
            },
            |world| {
                let water = fixture::id("water") as u8;
                world.set(7, 4, 10, water, 0);
                world.set(8, 3, 10, fixture::id("stone") as u8, 0);
            },
        );
        assert!(rendered);
        assert_eq!(vertices.len(), 32);
        assert_eq!(
            line(0, &vertices[0]),
            "v0 x=41000000 y=40981851 z=41200000 u=3ED7FD70 v=3E50051E color=FFFFFFFF light=13631552"
        );
        assert_eq!(
            line(3, &vertices[3]),
            "v3 x=41100000 y=408630B2 z=41200000 u=3ED7FD70 v=3E6FFAE2 color=FFFFFFFF light=13631552"
        );
    }

    /// Grass's fancy side overlay pass, tinted by the biome's 0x91BD59.
    #[test]
    fn the_grass_side_overlay_matches_the_oracle() {
        let (rendered, vertices) = render(
            Case {
                name: "grass_ao0_fancy",
                ao: 0,
                fancy: true,
                block: "grass",
                meta: 0,
                at: [5, 4, 4],
            },
            |world| {
                world.set(5, 3, 4, fixture::id("dirt") as u8, 0);
            },
        );
        assert!(rendered);
        // Five cube faces (the dirt below culls the bottom) plus four tinted
        // side overlays.
        assert_eq!(vertices.len(), 36);
        // The top face is the grass colour.
        assert_eq!(vertices[0].color as u32, 0xFF59BD91);
        // The last four are the side overlay, the tint multiplied into the
        // 0.8 side shade.
        assert_eq!(vertices[32].color as u32, 0xFF35_7157);
    }
}
