//! Version probe: deterministic output for parity checks. Prints the 1.7.10
//! version constants, the fresh options.txt a 64-bit client writes, the
//! default key bindings and the full translation key inventory.

use sunset_1_7_version::{keys, lang, options, version};

fn main() {
    println!("version={}", version::GAME_VERSION);
    println!("protocol={}", version::PROTOCOL_VERSION);
    println!("resourcePackFormat={}", version::RESOURCE_PACK_FORMAT);
    match version::DATA_VERSION {
        Some(data_version) => println!("dataVersion={data_version}"),
        None => println!("dataVersion=none"),
    }

    println!("[options.txt]");
    print!("{}", options::Options::default().save());

    println!("[keys]");
    let bindings = keys::KeyBindings::default();
    for binding in bindings.all() {
        let default = if binding.code == binding.default_code {
            "default"
        } else {
            "custom"
        };
        println!(
            "{}={}:{}:{default}",
            binding.name, binding.category, binding.code
        );
    }

    println!("[translation keys]");
    for (prefix, keys) in lang::KEY_GROUPS {
        println!("--{prefix}--");
        for key in keys.iter() {
            println!("{key}");
        }
    }
}
