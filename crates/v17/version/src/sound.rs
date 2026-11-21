//! 1.7.10 sound categories (`SoundCategory`): the names options.txt's
//! `soundCategory_*` lines and the sound menu use.

/// A sound category: its file name and id (`SoundCategory.getName`/`getId`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SoundCategory {
    Master,
    Music,
    Records,
    Weather,
    Blocks,
    Mobs,
    Animals,
    Players,
    Ambient,
}

impl SoundCategory {
    /// All categories in vanilla's enum order (`SoundCategory.values()`),
    /// the order options.txt saves them in.
    pub const ALL: [SoundCategory; 9] = [
        SoundCategory::Master,
        SoundCategory::Music,
        SoundCategory::Records,
        SoundCategory::Weather,
        SoundCategory::Blocks,
        SoundCategory::Mobs,
        SoundCategory::Animals,
        SoundCategory::Players,
        SoundCategory::Ambient,
    ];

    /// `SoundCategory.getName`: the options.txt / settings name.
    pub fn name(self) -> &'static str {
        match self {
            SoundCategory::Master => "master",
            SoundCategory::Music => "music",
            SoundCategory::Records => "record",
            SoundCategory::Weather => "weather",
            SoundCategory::Blocks => "block",
            SoundCategory::Mobs => "hostile",
            SoundCategory::Animals => "neutral",
            SoundCategory::Players => "player",
            SoundCategory::Ambient => "ambient",
        }
    }

    /// `SoundCategory.getId`: the packet id.
    pub fn id(self) -> i32 {
        match self {
            SoundCategory::Master => 0,
            SoundCategory::Music => 1,
            SoundCategory::Records => 2,
            SoundCategory::Weather => 3,
            SoundCategory::Blocks => 4,
            SoundCategory::Mobs => 5,
            SoundCategory::Animals => 6,
            SoundCategory::Players => 7,
            SoundCategory::Ambient => 8,
        }
    }

    /// `SoundCategory.byName`.
    pub fn by_name(name: &str) -> Option<SoundCategory> {
        SoundCategory::ALL.into_iter().find(|c| c.name() == name)
    }

    /// `SoundCategory.byId` (by id).
    pub fn by_id(id: i32) -> Option<SoundCategory> {
        SoundCategory::ALL.into_iter().find(|c| c.id() == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_ids_match_vanilla() {
        for (index, category) in SoundCategory::ALL.iter().enumerate() {
            assert_eq!(category.id(), index as i32);
        }
        assert_eq!(SoundCategory::by_name("block"), Some(SoundCategory::Blocks));
        assert_eq!(SoundCategory::by_name("hostile"), Some(SoundCategory::Mobs));
        assert_eq!(SoundCategory::by_name("blocks"), None);
    }
}
