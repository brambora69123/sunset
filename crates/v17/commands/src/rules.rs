//! `net.minecraft.world.GameRules`: the nine 1.7.10 rules over a `TreeMap`
//! (sorted keys), exact string semantics — unknown names are added, not
//! refused, and `getBoolean` is Java's `Boolean.parseBoolean`.

use std::collections::BTreeMap;

/// `GameRules$Value.getBoolean` = `Boolean.parseBoolean(value)`: only the
/// exact (case-sensitive) string `"true"` is true.
pub fn parse_boolean(value: &str) -> bool {
    value == "true"
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GameRules {
    values: BTreeMap<String, String>,
}

impl GameRules {
    /// The 1.7.10 defaults, in `GameRules()`'s registration order (the tree
    /// stores them sorted; `/gamerule` with no args lists them sorted).
    pub fn new() -> Self {
        let mut rules = Self::default();
        for (name, value) in Self::defaults() {
            rules.values.insert(name.to_string(), value.to_string());
        }
        rules
    }

    pub fn defaults() -> &'static [(&'static str, &'static str)] {
        &[
            ("doFireTick", "true"),
            ("mobGriefing", "true"),
            ("keepInventory", "false"),
            ("doMobSpawning", "true"),
            ("doMobLoot", "true"),
            ("doTileDrops", "true"),
            ("commandBlockOutput", "true"),
            ("naturalRegeneration", "true"),
            ("doDaylightCycle", "true"),
        ]
    }

    /// `add(name, value)`.
    pub fn add(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.values.insert(name.into(), value.into());
    }

    /// `set`: overwrite a known rule, add an unknown one.
    pub fn set(&mut self, name: &str, value: &str) {
        self.values.insert(name.to_string(), value.to_string());
    }

    /// `get`: the value, or `""` for an unknown rule.
    pub fn get(&self, name: &str) -> String {
        self.values.get(name).cloned().unwrap_or_default()
    }

    /// `getBoolean`: `false` for an unknown rule, `Boolean.parseBoolean` else.
    pub fn get_boolean(&self, name: &str) -> bool {
        self.values
            .get(name)
            .map(|v| parse_boolean(v))
            .unwrap_or(false)
    }

    /// `contains`.
    pub fn contains(&self, name: &str) -> bool {
        self.values.contains_key(name)
    }

    /// `getAll`: every rule name, sorted (`TreeMap.keySet`).
    pub fn get_all(&self) -> Vec<String> {
        self.values.keys().cloned().collect()
    }

    /// `toNbt`: every rule as a string tag, sorted keys.
    pub fn to_nbt(&self) -> sunrise_core::nbt::Tag {
        sunrise_core::nbt::Tag::Compound(
            self.values
                .iter()
                .map(|(k, v)| {
                    (
                        k.as_str().into(),
                        sunrise_core::nbt::Tag::String(v.as_str().into()),
                    )
                })
                .collect(),
        )
    }

    /// `readNbt`: every compound entry becomes a rule (added or overwritten).
    pub fn read_nbt(&mut self, nbt: &sunrise_core::nbt::Tag) {
        if let sunrise_core::nbt::Tag::Compound(entries) = nbt {
            for (key, value) in entries.iter() {
                if let sunrise_core::nbt::Tag::String(text) = value {
                    let name = String::from_utf16_lossy(&key.0);
                    let text = String::from_utf16_lossy(&text.0);
                    self.values.insert(name, text);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_sorted_list() {
        let rules = GameRules::new();
        assert!(rules.contains("doDaylightCycle"));
        assert!(rules.get_boolean("doFireTick"));
        assert!(!rules.get_boolean("keepInventory"));
        assert!(!rules.get_boolean("missing"));
        assert_eq!(rules.get("missing"), "");
        let all = rules.get_all();
        assert_eq!(all.len(), 9);
        let mut sorted = all.clone();
        sorted.sort();
        assert_eq!(all, sorted, "TreeMap lists its keys sorted");
    }

    #[test]
    fn set_unknown_rule_adds_it() {
        let mut rules = GameRules::new();
        rules.set("customRule", "hello");
        assert!(rules.contains("customRule"));
        assert_eq!(rules.get("customRule"), "hello");
        assert!(!rules.get_boolean("customRule"));
        rules.set("customRule", "true");
        assert!(rules.get_boolean("customRule"));
    }

    #[test]
    fn boolean_parses_exact_true_only() {
        assert!(!parse_boolean("True"));
        assert!(!parse_boolean("1"));
        assert!(!parse_boolean("yes"));
        assert!(parse_boolean("true"));
    }

    #[test]
    fn nbt_round_trip() {
        let mut rules = GameRules::new();
        rules.set("keepInventory", "true");
        let nbt = rules.to_nbt();
        let mut loaded = GameRules::default();
        loaded.read_nbt(&nbt);
        assert_eq!(loaded, rules);
    }
}
