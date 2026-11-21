//! Dumps the 1.7.10 item registry, one item per line, for parity checks.

use sunset_1_7_items as items;

fn main() {
    let registry: Vec<_> = items::items().collect();
    println!("count {}", registry.len());
    for item in registry {
        println!(
            "{}|{}|{}|stack={}|damage={}|tool={}|tab={}|block={}",
            item.id,
            item.name,
            item.translation_key(),
            item.max_stack_size,
            item.max_damage,
            item.tool_class().unwrap_or("-"),
            item.tab.map(|tab| tab.key()).unwrap_or("-"),
            item.block
                .map(|id| id.to_string())
                .unwrap_or("-".to_owned()),
        );
    }
}
