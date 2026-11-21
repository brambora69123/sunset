//! Generates 1.7.10 chunks through the engine's `ChunkGenerator`.
//!
//! Usage: `sunset17_probe [SEED] [CHUNK_X] [CHUNK_Z] [RADIUS] [OUT_DIR]`
//! (defaults: 1234 0 0 1). The centre chunk's blocks are printed as
//! `id:meta` in save order (`y << 8 | z << 4 | x`), the format
//! `target/release/world_dump SEED CHUNK_X CHUNK_Z RADIUS` compares against
//! for ids, and `region_dump` for both.
//!
//! `OUT_DIR` writes every populated chunk of the region to
//! `<chunkX>_<chunkZ>.txt`, the format `tools/terrain_region_1_7_10.py` reads.
//!
//! The 1.7.10 content export is read from `SUNSET_1_7_CONTENT` or
//! `.local/sunrise-client/1.7.10`, and written there if it is missing.
use std::{
    collections::HashMap,
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};
use sunrise_client_lib::{
    chunk::ColumnLayout,
    streaming::{ChunkGenerator, Proto},
    sunset17gen::{Sunset17Proto, Sunset17Terrain},
};
use sunrise_world::registry::BlockRegistry;

fn content_root() -> PathBuf {
    std::env::var_os("SUNSET_1_7_CONTENT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(sunset_1_7_content::DEFAULT_DIR))
}

fn registry(root: &Path) -> &'static BlockRegistry {
    if !root.join("facts.json").is_file() {
        sunset_1_7_content::export(root).expect("write the 1.7.10 content export");
    }
    let file = File::open(root.join("facts.json")).expect("open facts.json");
    Box::leak(Box::new(
        BlockRegistry::from_facts_json(file).expect("read the 1.7.10 block registry"),
    ))
}

/// The nine protos around `(cx, cz)`, taken out of the map.
fn take_region(protos: &mut HashMap<(i32, i32), Proto>, cx: i32, cz: i32) -> [Proto; 9] {
    std::array::from_fn(|index| {
        let (dx, dz) = (index % 3 - 1, index / 3 - 1);
        protos
            .remove(&(cx + dx as i32, cz + dz as i32))
            .unwrap_or_else(|| Box::new(()))
    })
}

fn put_region(protos: &mut HashMap<(i32, i32), Proto>, cx: i32, cz: i32, region: [Proto; 9]) {
    for (index, proto) in region.into_iter().enumerate() {
        if proto.is::<Sunset17Proto>() {
            let (dx, dz) = (index % 3 - 1, index / 3 - 1);
            protos.insert((cx + dx as i32, cz + dz as i32), proto);
        }
    }
}

/// The 1.7.10 dump format: `y << 8 | z << 4 | x`, `id:meta`.
fn dump(chunk: &Sunset17Proto) -> String {
    let blocks = chunk.blocks();
    let mut out = String::with_capacity(65536 * 4);
    for y in 0..256usize {
        for z in 0..16usize {
            for x in 0..16usize {
                let (id, meta) = blocks[(x << 12) | (z << 8) | y];
                if !out.is_empty() {
                    out.push(',');
                }
                out.push_str(&format!("{id}:{meta}"));
            }
        }
    }
    out
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let seed: i64 = args.get(1).map_or(1234, |v| v.parse().expect("seed"));
    let cx: i32 = args.get(2).map_or(0, |v| v.parse().expect("chunk x"));
    let cz: i32 = args.get(3).map_or(0, |v| v.parse().expect("chunk z"));
    let radius: i32 = args.get(4).map_or(1, |v| v.parse().expect("radius"));
    let out_dir = args.get(5).map(PathBuf::from);

    let registry = registry(&content_root());
    let terrain =
        Sunset17Terrain::from_registry(seed, registry, [0; 256], ColumnLayout::of_level(0, 256))
            .expect("1.7.10 terrain");
    let mut worker = terrain.worker();

    // One ring of terrain beyond the populated area, so reads across a
    // chunk's border see real blocks (world_dump's `radius + 1`).
    let mut protos: HashMap<(i32, i32), Proto> = HashMap::new();
    for dz in -(radius + 1)..=radius + 1 {
        for dx in -(radius + 1)..=radius + 1 {
            protos.insert(
                (cx + dx, cz + dz),
                terrain.proto(&mut worker, cx + dx, cz + dz),
            );
        }
    }

    // Vanilla populates chunks as they load, around spawn centre first and
    // then ring by ring; each population writes into its neighbours.
    let mut order: Vec<(i32, i32)> = Vec::new();
    for dz in -radius..=radius {
        for dx in -radius..=radius {
            order.push((dx, dz));
        }
    }
    order.sort_by_key(|&(dx, dz)| (dx.abs().max(dz.abs()), dz, dx));
    for (dx, dz) in order {
        let mut region = take_region(&mut protos, cx + dx, cz + dz);
        terrain.decorate(&mut worker, &mut region);
        put_region(&mut protos, cx + dx, cz + dz, region);
    }

    if let Some(dir) = out_dir {
        std::fs::create_dir_all(&dir).expect("create the dump folder");
        for dz in -radius..=radius {
            for dx in -radius..=radius {
                let (x, z) = (cx + dx, cz + dz);
                let chunk = protos
                    .get(&(x, z))
                    .and_then(|proto| proto.downcast_ref::<Sunset17Proto>())
                    .expect("a populated chunk");
                let mut file =
                    File::create(dir.join(format!("{x}_{z}.txt"))).expect("create a chunk dump");
                file.write_all(dump(chunk).as_bytes())
                    .expect("write a chunk dump");
            }
        }
        eprintln!(
            "dumped {} chunks to {}",
            (2 * radius + 1) * (2 * radius + 1),
            dir.display()
        );
    }

    let proto = protos
        .remove(&(cx, cz))
        .and_then(|proto| proto.downcast::<Sunset17Proto>().ok())
        .expect("the centre chunk");
    let proto = *proto;
    let pairs = proto.blocks().to_vec();
    println!("{}", dump(&proto));

    // Everything above ran through the trait's `proto` and `decorate`; finish
    // the same chunk as the engine will and check the column it builds.
    let column = terrain.finish(Box::new(proto));
    let mut placed = 0usize;
    for y in 0..256i32 {
        for z in 0..16usize {
            for x in 0..16usize {
                let (id, meta) = pairs[(x << 12) | (z << 8) | y as usize];
                if column.get(x as i32, y, z as i32) == terrain.state_id(id, meta) {
                    placed += 1;
                }
            }
        }
    }
    eprintln!("finish: {placed}/65536 states placed");
    assert_eq!(placed, 65536, "the engine column holds every dumped block");
}
