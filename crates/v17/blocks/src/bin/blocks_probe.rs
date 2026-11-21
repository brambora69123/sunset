//! Probe: prints the 1.7.10 block registry deterministically, one CSV row per
//! block, for later parity checks against the vanilla sources.

use sunset_1_7_blocks::{BLOCKS, MAX_ID, by_id};

fn main() {
    println!(
        "id,name,key,material,hardness,resistance,slipperiness,opacity,light,tool,harvest_level"
    );
    for id in 0..=MAX_ID {
        let Some(block) = by_id(id) else {
            continue;
        };
        println!(
            "{},{},{},{},{},{},{},{},{},{},{}",
            block.id,
            block.name,
            block.key.unwrap_or("<null>"),
            format!("{:?}", block.material).to_lowercase(),
            block.hardness,
            block.resistance,
            block.slipperiness,
            block.opacity,
            block.light,
            match block.tool {
                Some(tool) => format!("{tool:?}").to_lowercase(),
                None => "none".to_owned(),
            },
            block.harvest_level,
        );
    }
    eprintln!("{} blocks", BLOCKS.len());
}
