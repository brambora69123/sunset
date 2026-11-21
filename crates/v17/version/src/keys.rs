//! 1.7.10 default key bindings (`GameOptions`' KeyBinding fields, the
//! `keyBindings` array order options.txt saves them in) and the LWJGL 2 key
//! names the controls menu shows.

/// A key binding: its options.txt name, category, default code and current
/// code (`KeyBinding`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyBinding {
    /// The options.txt name, e.g. "key.forward" (the file line is
    /// `key_key.forward`, `"key_" + KeyBinding.getName()`).
    pub name: &'static str,
    /// The settings category, e.g. "key.categories.movement".
    pub category: &'static str,
    /// The code the game ships with.
    pub default_code: i32,
    /// The code currently bound (set from options.txt on load).
    pub code: i32,
}

impl KeyBinding {
    pub fn new(name: &'static str, code: i32, category: &'static str) -> KeyBinding {
        KeyBinding {
            name,
            category,
            default_code: code,
            code,
        }
    }
}

/// `key.categories.*`, the settings categories the controls menu groups by.
pub const CATEGORY_MOVEMENT: &str = "key.categories.movement";
pub const CATEGORY_INVENTORY: &str = "key.categories.inventory";
pub const CATEGORY_GAMEPLAY: &str = "key.categories.gameplay";
pub const CATEGORY_MULTIPLAYER: &str = "key.categories.multiplayer";
pub const CATEGORY_MISC: &str = "key.categories.misc";
pub const CATEGORY_STREAM: &str = "key.categories.stream";

/// The mouse button translation key: `GameOptions.getKeyName` reads it with
/// the button number (`mouse_button_number`) for a negative code.
pub const KEY_MOUSE_BUTTON: &str = "key.mouseButton";

/// `GameOptions.getKeyName`'s mouse button number: `keyCode + 101`, so the
/// attack key (-100) reads as button 1, use (-99) as 2 and pickItem (-98) as 3.
pub fn mouse_button_number(key_code: i32) -> i32 {
    key_code + 101
}

/// The default key bindings (`GameOptions`' fields), in the file order
/// options.txt saves them in. `code == 0` means unbound.
#[derive(Clone, Debug)]
pub struct KeyBindings {
    pub attack: KeyBinding,
    pub use_key: KeyBinding,
    pub forward: KeyBinding,
    pub left: KeyBinding,
    pub back: KeyBinding,
    pub right: KeyBinding,
    pub jump: KeyBinding,
    pub sneak: KeyBinding,
    pub drop: KeyBinding,
    pub inventory: KeyBinding,
    pub chat: KeyBinding,
    pub player_list: KeyBinding,
    pub pick_item: KeyBinding,
    pub command: KeyBinding,
    pub screenshot: KeyBinding,
    pub toggle_perspective: KeyBinding,
    pub smooth_camera: KeyBinding,
    pub sprint: KeyBinding,
    pub stream_start_stop: KeyBinding,
    pub stream_pause: KeyBinding,
    pub stream_commercial: KeyBinding,
    pub stream_toggle_mic: KeyBinding,
    pub fullscreen: KeyBinding,
    /// The nine hotbar slots (`hotbarKeyBindings`).
    pub hotbar: [KeyBinding; 9],
}

/// The hotbar slot key names, in slot order (`hotbarKeyBindings`).
const HOTBAR_NAMES: [&str; 9] = [
    "key.hotbar.1",
    "key.hotbar.2",
    "key.hotbar.3",
    "key.hotbar.4",
    "key.hotbar.5",
    "key.hotbar.6",
    "key.hotbar.7",
    "key.hotbar.8",
    "key.hotbar.9",
];

impl Default for KeyBindings {
    fn default() -> KeyBindings {
        KeyBindings {
            attack: KeyBinding::new("key.attack", -100, CATEGORY_GAMEPLAY),
            use_key: KeyBinding::new("key.use", -99, CATEGORY_GAMEPLAY),
            forward: KeyBinding::new("key.forward", 17, CATEGORY_MOVEMENT),
            left: KeyBinding::new("key.left", 30, CATEGORY_MOVEMENT),
            back: KeyBinding::new("key.back", 31, CATEGORY_MOVEMENT),
            right: KeyBinding::new("key.right", 32, CATEGORY_MOVEMENT),
            jump: KeyBinding::new("key.jump", 57, CATEGORY_MOVEMENT),
            sneak: KeyBinding::new("key.sneak", 42, CATEGORY_MOVEMENT),
            drop: KeyBinding::new("key.drop", 16, CATEGORY_GAMEPLAY),
            inventory: KeyBinding::new("key.inventory", 18, CATEGORY_INVENTORY),
            chat: KeyBinding::new("key.chat", 20, CATEGORY_MULTIPLAYER),
            player_list: KeyBinding::new("key.playerlist", 15, CATEGORY_MULTIPLAYER),
            pick_item: KeyBinding::new("key.pickItem", -98, CATEGORY_GAMEPLAY),
            command: KeyBinding::new("key.command", 53, CATEGORY_MULTIPLAYER),
            screenshot: KeyBinding::new("key.screenshot", 60, CATEGORY_MISC),
            toggle_perspective: KeyBinding::new("key.togglePerspective", 63, CATEGORY_MISC),
            smooth_camera: KeyBinding::new("key.smoothCamera", 0, CATEGORY_MISC),
            sprint: KeyBinding::new("key.sprint", 29, CATEGORY_GAMEPLAY),
            stream_start_stop: KeyBinding::new("key.streamStartStop", 64, CATEGORY_STREAM),
            stream_pause: KeyBinding::new("key.streamPauseUnpause", 65, CATEGORY_STREAM),
            stream_commercial: KeyBinding::new("key.streamCommercial", 0, CATEGORY_STREAM),
            stream_toggle_mic: KeyBinding::new("key.streamToggleMic", 0, CATEGORY_STREAM),
            fullscreen: KeyBinding::new("key.fullscreen", 87, CATEGORY_MISC),
            hotbar: std::array::from_fn(|slot| {
                KeyBinding::new(HOTBAR_NAMES[slot], slot as i32 + 2, CATEGORY_INVENTORY)
            }),
        }
    }
}

impl KeyBindings {
    /// The vanilla `keyBindings` array (the 23 named bindings, then the
    /// hotbar slots), the order options.txt saves them in.
    pub fn all(&self) -> [&KeyBinding; 32] {
        [
            &self.attack,
            &self.use_key,
            &self.forward,
            &self.left,
            &self.back,
            &self.right,
            &self.jump,
            &self.sneak,
            &self.drop,
            &self.inventory,
            &self.chat,
            &self.player_list,
            &self.pick_item,
            &self.command,
            &self.screenshot,
            &self.toggle_perspective,
            &self.smooth_camera,
            &self.sprint,
            &self.stream_start_stop,
            &self.stream_pause,
            &self.stream_commercial,
            &self.stream_toggle_mic,
            &self.fullscreen,
            &self.hotbar[0],
            &self.hotbar[1],
            &self.hotbar[2],
            &self.hotbar[3],
            &self.hotbar[4],
            &self.hotbar[5],
            &self.hotbar[6],
            &self.hotbar[7],
            &self.hotbar[8],
        ]
    }

    /// The same bindings, mutable (options.txt load sets codes through this).
    pub fn all_mut(&mut self) -> [&mut KeyBinding; 32] {
        let [
            slot1,
            slot2,
            slot3,
            slot4,
            slot5,
            slot6,
            slot7,
            slot8,
            slot9,
        ] = &mut self.hotbar;
        [
            &mut self.attack,
            &mut self.use_key,
            &mut self.forward,
            &mut self.left,
            &mut self.back,
            &mut self.right,
            &mut self.jump,
            &mut self.sneak,
            &mut self.drop,
            &mut self.inventory,
            &mut self.chat,
            &mut self.player_list,
            &mut self.pick_item,
            &mut self.command,
            &mut self.screenshot,
            &mut self.toggle_perspective,
            &mut self.smooth_camera,
            &mut self.sprint,
            &mut self.stream_start_stop,
            &mut self.stream_pause,
            &mut self.stream_commercial,
            &mut self.stream_toggle_mic,
            &mut self.fullscreen,
            slot1,
            slot2,
            slot3,
            slot4,
            slot5,
            slot6,
            slot7,
            slot8,
            slot9,
        ]
    }

    /// The binding a key code is currently bound to (`KeyBinding`'s
    /// BY_KEY_CODE map). Input events guard `code != 0` before the lookup,
    /// as vanilla's `KeyBinding.click`/`set` do; unbound (0) still resolves
    /// to whatever binding has it, so the guard is at the call site.
    pub fn by_code(&self, code: i32) -> Option<&KeyBinding> {
        self.all().into_iter().find(|binding| binding.code == code)
    }
}

/// The LWJGL 2 key name for a keyboard code (`Keyboard.getKeyName`), what the
/// controls menu shows. `None` for codes outside LWJGL 2's named set.
///
/// ponytail: covers LWJGL 2's named range (letters, digits, punctuation,
/// F1-F18, numpad, arrows and edit block) exactly; the unassigned gaps and
/// codes beyond 211 read `None` — extend if a parity check ever binds them.
pub fn lwjgl_key_name(key_code: i32) -> Option<&'static str> {
    Some(match key_code {
        0 => "NONE",
        1 => "ESCAPE",
        2 => "1",
        3 => "2",
        4 => "3",
        5 => "4",
        6 => "5",
        7 => "6",
        8 => "7",
        9 => "8",
        10 => "9",
        11 => "0",
        12 => "MINUS",
        13 => "EQUALS",
        14 => "BACK",
        15 => "TAB",
        16 => "Q",
        17 => "W",
        18 => "E",
        19 => "R",
        20 => "T",
        21 => "Y",
        22 => "U",
        23 => "I",
        24 => "O",
        25 => "P",
        26 => "LBRACKET",
        27 => "RBRACKET",
        28 => "RETURN",
        29 => "LCONTROL",
        30 => "A",
        31 => "S",
        32 => "D",
        33 => "F",
        34 => "G",
        35 => "H",
        36 => "J",
        37 => "K",
        38 => "L",
        39 => "SEMICOLON",
        40 => "APOSTROPHE",
        41 => "GRAVE",
        42 => "LSHIFT",
        43 => "BACKSLASH",
        44 => "Z",
        45 => "X",
        46 => "C",
        47 => "V",
        48 => "B",
        49 => "N",
        50 => "M",
        51 => "COMMA",
        52 => "PERIOD",
        53 => "SLASH",
        54 => "RSHIFT",
        55 => "MULTIPLY",
        56 => "LMENU",
        57 => "SPACE",
        58 => "CAPITAL",
        59 => "F1",
        60 => "F2",
        61 => "F3",
        62 => "F4",
        63 => "F5",
        64 => "F6",
        65 => "F7",
        66 => "F8",
        67 => "F9",
        68 => "F10",
        69 => "NUMLOCK",
        70 => "SCROLL",
        71 => "NUMPAD7",
        72 => "NUMPAD8",
        73 => "NUMPAD9",
        74 => "SUBTRACT",
        75 => "NUMPAD4",
        76 => "NUMPAD5",
        77 => "NUMPAD6",
        78 => "ADD",
        79 => "NUMPAD1",
        80 => "NUMPAD2",
        81 => "NUMPAD3",
        82 => "NUMPAD0",
        83 => "DECIMAL",
        87 => "F11",
        88 => "F12",
        89 => "F13",
        90 => "F14",
        91 => "F15",
        92 => "F16",
        93 => "F17",
        94 => "F18",
        199 => "HOME",
        200 => "UP",
        201 => "PRIOR",
        203 => "LEFT",
        205 => "RIGHT",
        207 => "END",
        208 => "DOWN",
        209 => "NEXT",
        210 => "INSERT",
        211 => "DELETE",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_codes_match_vanilla() {
        let keys = KeyBindings::default();
        let codes: Vec<(String, i32)> = keys
            .all()
            .into_iter()
            .map(|binding| (binding.name.to_string(), binding.code))
            .collect();
        let expected: Vec<(String, i32)> = [
            ("key.attack", -100),
            ("key.use", -99),
            ("key.forward", 17),
            ("key.left", 30),
            ("key.back", 31),
            ("key.right", 32),
            ("key.jump", 57),
            ("key.sneak", 42),
            ("key.drop", 16),
            ("key.inventory", 18),
            ("key.chat", 20),
            ("key.playerlist", 15),
            ("key.pickItem", -98),
            ("key.command", 53),
            ("key.screenshot", 60),
            ("key.togglePerspective", 63),
            ("key.smoothCamera", 0),
            ("key.sprint", 29),
            ("key.streamStartStop", 64),
            ("key.streamPauseUnpause", 65),
            ("key.streamCommercial", 0),
            ("key.streamToggleMic", 0),
            ("key.fullscreen", 87),
            ("key.hotbar.1", 2),
            ("key.hotbar.2", 3),
            ("key.hotbar.3", 4),
            ("key.hotbar.4", 5),
            ("key.hotbar.5", 6),
            ("key.hotbar.6", 7),
            ("key.hotbar.7", 8),
            ("key.hotbar.8", 9),
            ("key.hotbar.9", 10),
        ]
        .into_iter()
        .map(|(name, code)| (name.to_string(), code))
        .collect();
        assert_eq!(codes, expected);
    }

    #[test]
    fn hotbar_names_count_slots() {
        let keys = KeyBindings::default();
        for (slot, binding) in keys.hotbar.iter().enumerate() {
            assert_eq!(binding.name, HOTBAR_NAMES[slot]);
            assert_eq!(binding.code, slot as i32 + 2);
        }
    }

    #[test]
    fn mouse_buttons_read_as_button_numbers() {
        assert_eq!(mouse_button_number(-100), 1);
        assert_eq!(mouse_button_number(-99), 2);
        assert_eq!(mouse_button_number(-98), 3);
    }

    #[test]
    fn lwjgl_names_cover_defaults() {
        assert_eq!(lwjgl_key_name(17), Some("W"));
        assert_eq!(lwjgl_key_name(30), Some("A"));
        assert_eq!(lwjgl_key_name(57), Some("SPACE"));
        assert_eq!(lwjgl_key_name(42), Some("LSHIFT"));
        assert_eq!(lwjgl_key_name(60), Some("F2"));
        assert_eq!(lwjgl_key_name(87), Some("F11"));
        assert_eq!(lwjgl_key_name(0), Some("NONE"));
        assert_eq!(lwjgl_key_name(-100), None);
        assert_eq!(lwjgl_key_name(84), None);
    }

    #[test]
    fn by_code_finds_bound_keys() {
        let keys = KeyBindings::default();
        assert_eq!(keys.by_code(-100).map(|b| b.name), Some("key.attack"));
        assert_eq!(keys.by_code(17).map(|b| b.name), Some("key.forward"));
        // Code 0 resolves to the unbound-by-default binding; vanilla guards
        // input events with `keyCode != 0` before the map lookup.
        assert_eq!(keys.by_code(0).map(|b| b.name), Some("key.smoothCamera"));
    }
}
