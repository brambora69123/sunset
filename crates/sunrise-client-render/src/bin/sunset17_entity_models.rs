//! Prints the 1.7.10 entity model table as data, so a reviewer can compare every bone, box
//! and texture offset with the era's own `Model*` classes.
//!
//! Usage: `sunset17_entity_models [ROOT]` (`SUNSET_1_7_CONTENT` or
//! `.local/sunrise-client/1.7.10` by default). The table itself needs no files; when `ROOT`
//! is a prepared 1.7.10 folder, the probe also says whether its baked `entity-models.json`
//! carries every covered layer.

use std::path::PathBuf;
use sunrise_client_render::sunset17_assets::{self, Bone};

fn main() {
    let root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("SUNSET_1_7_CONTENT").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from(sunset17_assets::DEFAULT_DIR));

    println!("1.7.10 entity models (the era's Model* classes)");
    for model in sunset17_assets::models() {
        println!(
            "\n{} texture={} size={}x{}",
            model.name, model.texture, model.texture_size[0], model.texture_size[1]
        );
        print_bones(model.bones, 1);
    }

    println!("\n1.7.10 entity types (the era's setSize/initAttributes)");
    for kind in sunset17_assets::entity_types() {
        let attributes: Vec<String> = kind
            .attributes
            .iter()
            .map(|(name, value)| format!("{name}={value}"))
            .collect();
        println!(
            "  {:<22} {}x{} eye {} category {} fire_immune {} {}",
            kind.id,
            kind.width,
            kind.height,
            kind.eye_height,
            kind.category,
            kind.fire_immune,
            attributes.join(" ")
        );
    }

    println!("\n1.7.10 entity sheets under the engine's names");
    for texture in sunset17_assets::textures() {
        match sunset17_assets::entity_sprite(texture.path) {
            Some(era) => println!("  {:<32} <- {} ({era})", texture.name, texture.path),
            None => println!("  {:<32} <- {} (not a sheet)", texture.name, texture.path),
        }
    }

    if sunset17_assets::detect(&root) {
        let path = root.join(sunset17_assets::ENTITY_MODELS_FILE);
        match std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        {
            Some(document) => {
                println!("\nbaked layers in {}", path.display());
                for model in sunset17_assets::models() {
                    let present = document.get(model.name).is_some();
                    println!(
                        "  {:<32} {}",
                        model.name,
                        if present { "ok" } else { "MISSING" }
                    );
                }
            }
            None => println!("\n{} is missing or not JSON", path.display()),
        }
    } else {
        println!(
            "\nno prepared 1.7.10 folder at {}: the baked table was not checked",
            root.display()
        );
    }
}

fn print_bones(bones: &[Bone], depth: usize) {
    for bone in bones {
        let indent = "  ".repeat(depth);
        let parent = bone.parent.map_or("root", |parent| parent);
        println!(
            "{indent}{} parent={parent} pivot=({}, {}, {}) rot=({}, {}, {})",
            bone.name,
            bone.pivot[0],
            bone.pivot[1],
            bone.pivot[2],
            bone.rotation[0],
            bone.rotation[1],
            bone.rotation[2]
        );
        for cube in bone.cubes {
            println!(
                "{indent}  box origin=({}, {}, {}) size=({}, {}, {}) uv=({}, {}) inflate={} mirror={}",
                cube.origin[0],
                cube.origin[1],
                cube.origin[2],
                cube.size[0],
                cube.size[1],
                cube.size[2],
                cube.uv[0],
                cube.uv[1],
                cube.inflate,
                cube.mirror
            );
        }
    }
}
