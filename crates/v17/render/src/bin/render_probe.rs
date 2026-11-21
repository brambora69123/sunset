//! Deterministic dump of the 1.7.10 block renderer's vertex stream.
//!
//! Every case mirrors `tools/javaoracle_1_7_10/RenderOracle.java` line for
//! line: same one-chunk world, same block table, same light pattern and the
//! same sprite atlas. Each vertex prints the raw 32-byte vertex words as the
//! IEEE-754 bit patterns of x, y, z, u and v, the packed colour int and the
//! packed brightness int.
//!
//! The regression harness is a line diff against the Java oracle:
//!
//! ```sh
//! sh tools/javaoracle_1_7_10/build.sh
//! sh tools/javaoracle_1_7_10/run.sh RenderOracle > /tmp/oracle.txt
//! cargo run -p sunset-1-7-render --bin render_probe > /tmp/rust.txt
//! diff /tmp/oracle.txt /tmp/rust.txt
//! ```
//!
//! Any difference is a renderer bug: the two streams must stay bit-exact.

use sunset_1_7_render::block::BlockDef;
use sunset_1_7_render::fixture::TestWorld;
use sunset_1_7_render::render_blocks::{RenderBlocks, RenderOptions};
use sunset_1_7_render::world::WorldView;

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

fn case(
    name: &str,
    ao: i32,
    fancy: bool,
    block: &'static str,
    meta: u8,
    x: i32,
    y: i32,
    z: i32,
    build: impl FnOnce(&mut TestWorld),
) {
    let mut world = prepare_world();
    build(&mut world);
    let id = world.blocks().id(block) as u8;
    world.set(x, y, z, id, meta);
    let block: &BlockDef = world.block(x, y, z);
    let options = RenderOptions {
        ao,
        fancy_graphics: fancy,
        anaglyph: false,
    };
    let mut renderer = RenderBlocks::new(&world, options);
    let rendered = renderer.tesselate_in_world(block, x, y, z);
    println!(
        "== {name} rendered={rendered} vertices={}",
        renderer.tesselator.vertices.len()
    );
    for (index, vertex) in renderer.tesselator.vertices.iter().enumerate() {
        println!(
            "v{index} x={:08X} y={:08X} z={:08X} u={:08X} v={:08X} color={:08X} light={}",
            vertex.x.to_bits(),
            vertex.y.to_bits(),
            vertex.z.to_bits(),
            vertex.u.to_bits(),
            vertex.v.to_bits(),
            vertex.color as u32,
            vertex.brightness
        );
    }
}

fn main() {
    for ao in 0..=2 {
        case(
            &format!("stone_ao{ao}"),
            ao,
            false,
            "stone",
            0,
            4,
            4,
            4,
            |world| {
                world.set(4, 3, 4, world.blocks().id("stone") as u8, 0);
            },
        );
    }
    for ao in 0..=1 {
        case(
            &format!("stone_culled_ao{ao}"),
            ao,
            false,
            "stone",
            0,
            4,
            4,
            4,
            |world| {
                let stone = world.blocks().id("stone") as u8;
                world.set(4, 3, 4, stone, 0);
                world.set(5, 4, 4, stone, 0);
                world.set(4, 4, 5, stone, 0);
            },
        );
    }
    for ao in 0..=1 {
        for (fancy, suffix) in [(true, "fancy"), (false, "plain")] {
            case(
                &format!("grass_ao{ao}_{suffix}"),
                ao,
                fancy,
                "grass",
                0,
                5,
                4,
                4,
                |world| {
                    world.set(5, 3, 4, world.blocks().id("dirt") as u8, 0);
                },
            );
        }
    }
    for (meta, axis) in [(0, "y"), (4, "x"), (8, "z")] {
        for ao in 0..=1 {
            case(
                &format!("log_{axis}_ao{ao}"),
                ao,
                false,
                "log",
                meta,
                6,
                4,
                4,
                |world| {
                    world.set(6, 3, 4, world.blocks().id("stone") as u8, 0);
                },
            );
        }
    }
    for ao in 0..=1 {
        case(
            &format!("glass_ao{ao}"),
            ao,
            false,
            "glass",
            0,
            7,
            4,
            4,
            |world| {
                let glass = world.blocks().id("glass") as u8;
                world.set(8, 4, 4, glass, 0);
                world.set(7, 4, 5, glass, 0);
                world.set(7, 3, 4, world.blocks().id("stone") as u8, 0);
            },
        );
    }
    for ao in 0..=1 {
        case(
            &format!("torch_floor_ao{ao}"),
            ao,
            false,
            "torch",
            0,
            9,
            4,
            4,
            |world| {
                world.set(9, 3, 4, world.blocks().id("stone") as u8, 0);
            },
        );
    }
    case("torch_wall_ao1", 1, false, "torch", 1, 9, 4, 6, |world| {
        world.set(10, 4, 6, world.blocks().id("stone") as u8, 0);
    });
    for ao in 0..=2 {
        case(
            &format!("stairs_east_ao{ao}"),
            ao,
            false,
            "oak_stairs",
            0,
            2,
            4,
            2,
            |world| {
                world.set(2, 3, 2, world.blocks().id("stone") as u8, 0);
            },
        );
        case(
            &format!("stairs_upside_ao{ao}"),
            ao,
            false,
            "oak_stairs",
            4,
            2,
            4,
            2,
            |world| {
                world.set(2, 3, 2, world.blocks().id("stone") as u8, 0);
            },
        );
    }
    for ao in 0..=2 {
        case(
            &format!("slab_bottom_ao{ao}"),
            ao,
            false,
            "stone_slab",
            0,
            3,
            4,
            3,
            |world| {
                world.set(3, 3, 3, world.blocks().id("stone") as u8, 0);
            },
        );
        case(
            &format!("slab_top_ao{ao}"),
            ao,
            false,
            "stone_slab",
            8,
            3,
            4,
            3,
            |world| {
                world.set(3, 3, 3, world.blocks().id("stone") as u8, 0);
            },
        );
    }
    for ao in 0..=2 {
        case(
            &format!("fence_ao{ao}"),
            ao,
            false,
            "oak_fence",
            0,
            6,
            6,
            4,
            |world| {
                let stone = world.blocks().id("stone") as u8;
                world.set(5, 6, 4, stone, 0);
                world.set(7, 6, 4, stone, 0);
            },
        );
    }
    for (meta, name) in [(1, "tallgrass"), (2, "fern")] {
        for ao in 0..=1 {
            case(
                &format!("{name}_ao{ao}"),
                ao,
                false,
                name,
                meta,
                2,
                6,
                4,
                |world| {
                    world.set(2, 5, 4, world.blocks().id("dirt") as u8, 0);
                },
            );
        }
    }
    for ao in 0..=1 {
        case(
            &format!("water_still_ao{ao}"),
            ao,
            false,
            "water",
            0,
            8,
            4,
            8,
            |world| {
                world.set(8, 3, 8, world.blocks().id("water") as u8, 0);
            },
        );
    }
    case("water_flow_ao1", 1, false, "water", 1, 8, 4, 10, |world| {
        let water = world.blocks().id("water") as u8;
        world.set(7, 4, 10, water, 0);
        world.set(8, 3, 10, world.blocks().id("stone") as u8, 0);
    });
    for ao in 0..=2 {
        case(
            &format!("snow_ao{ao}"),
            ao,
            false,
            "snow",
            3,
            5,
            6,
            4,
            |world| {
                world.set(5, 5, 4, world.blocks().id("stone") as u8, 0);
            },
        );
    }
}
