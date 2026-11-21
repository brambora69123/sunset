//! 1.7.10 client options (`GameOptions`): the options.txt set and defaults,
//! the file format (load/save line-exact), and the GUI option enum
//! (`GameOptions$Option`) with its ranges.

use std::collections::HashMap;

use crate::keys::{KeyBindings, lwjgl_key_name, mouse_button_number};
use crate::sound::SoundCategory;

/// 1.7.10's client options (`GameOptions`' fields), with the era's defaults.
/// The constructor resets the fields to these defaults; load() applies an
/// options.txt over them.
#[derive(Clone, Debug)]
pub struct Options {
    pub invert_mouse_y: bool,
    pub mouse_sensitivity: f32,
    /// The full field of view in degrees (options.txt stores it as
    /// `(fov - 70) / 40`, the normalized slider value).
    pub fov: f32,
    pub gamma: f32,
    pub saturation: f32,
    pub view_distance: i32,
    pub gui_scale: i32,
    pub particles: i32,
    pub view_bobbing: bool,
    pub anaglyph: bool,
    pub advanced_opengl: bool,
    pub fps_limit: i32,
    pub fbo_enable: bool,
    /// 0 peaceful, 1 easy, 2 normal, 3 hard (`Difficulty.byId`).
    pub difficulty: i32,
    pub fancy_graphics: bool,
    /// 0 off, 1 min, 2 max.
    pub ambient_occlusion: i32,
    pub render_clouds: bool,
    pub resource_packs: Vec<String>,
    pub last_server: String,
    pub language: String,
    /// 0 full, 1 system, 2 hidden (`PlayerEntity$ChatVisibility`).
    pub chat_visibility: i32,
    pub chat_colors: bool,
    pub chat_links: bool,
    pub chat_links_prompt: bool,
    pub chat_opacity: f32,
    pub snooper_enabled: bool,
    pub fullscreen: bool,
    pub vsync: bool,
    pub hide_server_address: bool,
    pub advanced_item_tooltips: bool,
    pub pause_on_unfocus: bool,
    pub show_cape: bool,
    pub touchscreen: bool,
    pub override_width: i32,
    pub override_height: i32,
    pub item_in_hand_tooltips: bool,
    pub chat_scale: f32,
    pub chat_width: f32,
    pub unfocused_chat_height: f32,
    pub focused_chat_height: f32,
    pub show_inventory_achievement_hint: bool,
    pub mipmap_levels: i32,
    pub anisotropic_filtering: i32,
    pub stream_bytes_per_pixel: f32,
    pub stream_mic_volume: f32,
    pub stream_system_volume: f32,
    pub stream_kbps: f32,
    pub stream_fps: f32,
    pub stream_compression: i32,
    pub stream_send_metadata: bool,
    pub stream_preferred_server: String,
    pub stream_chat_enabled: i32,
    pub stream_chat_user_filter: i32,
    pub stream_mic_toggle_behavior: i32,
    pub force_unicode_font: bool,
    /// Per-category volumes; `sound_category_volume` reads 1.0 when unset
    /// (`soundCategoryVolumes`).
    sound_category_volumes: HashMap<SoundCategory, f32>,
    /// The key bindings, in the `keyBindings` array order options.txt saves.
    pub keys: KeyBindings,
}

impl Default for Options {
    fn default() -> Options {
        Options::new(true)
    }
}

impl Options {
    /// `GameOptions(minecraft, dir)`: view distance 12 on a 64-bit machine, 8
    /// otherwise, every field at the era's default, no options.txt applied.
    pub fn new(is_64_bit: bool) -> Options {
        Options {
            invert_mouse_y: false,
            mouse_sensitivity: 0.5,
            fov: 70.0,
            gamma: 0.0,
            saturation: 0.0,
            view_distance: if is_64_bit { 12 } else { 8 },
            gui_scale: 0,
            particles: 0,
            view_bobbing: true,
            anaglyph: false,
            advanced_opengl: false,
            fps_limit: 120,
            fbo_enable: true,
            difficulty: 2,
            fancy_graphics: true,
            ambient_occlusion: 2,
            render_clouds: true,
            resource_packs: Vec::new(),
            last_server: String::new(),
            language: "en_US".to_string(),
            chat_visibility: 0,
            chat_colors: true,
            chat_links: true,
            chat_links_prompt: true,
            chat_opacity: 1.0,
            snooper_enabled: true,
            fullscreen: false,
            vsync: true,
            hide_server_address: false,
            advanced_item_tooltips: false,
            pause_on_unfocus: true,
            show_cape: true,
            touchscreen: false,
            override_width: 0,
            override_height: 0,
            item_in_hand_tooltips: true,
            chat_scale: 1.0,
            chat_width: 1.0,
            unfocused_chat_height: 0.44366196,
            focused_chat_height: 1.0,
            show_inventory_achievement_hint: true,
            mipmap_levels: 4,
            anisotropic_filtering: 1,
            stream_bytes_per_pixel: 0.5,
            stream_mic_volume: 1.0,
            stream_system_volume: 1.0,
            stream_kbps: 0.5412844,
            stream_fps: 0.31690142,
            stream_compression: 1,
            stream_send_metadata: true,
            stream_preferred_server: String::new(),
            stream_chat_enabled: 0,
            stream_chat_user_filter: 0,
            stream_mic_toggle_behavior: 0,
            force_unicode_font: false,
            sound_category_volumes: HashMap::new(),
            keys: KeyBindings::default(),
        }
    }

    /// `getSoundCategoryVolume`: 1.0 when options.txt sets none.
    pub fn sound_category_volume(&self, category: SoundCategory) -> f32 {
        self.sound_category_volumes
            .get(&category)
            .copied()
            .unwrap_or(1.0)
    }

    /// `setSoundCategoryVolume`.
    pub fn set_sound_category_volume(&mut self, category: SoundCategory, volume: f32) {
        self.sound_category_volumes.insert(category, volume);
    }

    /// The fields `ClientSettingsC2SPacket` carries, as
    /// `GameOptions.syncClientSettings` sends them.
    pub fn client_settings(&self) -> ClientSettings {
        ClientSettings {
            language: self.language.clone(),
            view_distance: self.view_distance,
            chat_visibility: self.chat_visibility,
            chat_colors: self.chat_colors,
            difficulty: self.difficulty,
            show_cape: self.show_cape,
        }
    }

    /// `load`: apply an options.txt over the defaults, line by line. Bad
    /// lines are skipped, as vanilla skips them with a warning; unknown lines
    /// are ignored.
    pub fn load(&mut self, text: &str) {
        for line in text.lines() {
            self.load_line(line);
        }
    }

    /// A parse failure or a missing value leaves the field at its current
    /// value and the line is skipped (vanilla warns "Skipping bad option").
    fn load_line(&mut self, line: &str) {
        let _ = self.try_load_line(line);
    }

    fn try_load_line(&mut self, line: &str) -> Result<(), ()> {
        let (key, rest) = line.split_once(':').ok_or(())?;
        // Java splits every scalar value on ":" and takes the second field;
        // the string values below keep everything after the first colon.
        let scalar = rest.split(':').next().unwrap_or("");
        match key {
            "invertYMouse" => self.invert_mouse_y = parse_bool(scalar)?,
            "mouseSensitivity" => self.mouse_sensitivity = parse_option_float(scalar)?,
            // options.txt stores the normalized slider value; load scales it
            // back (vanilla loads "fov" twice, the second load wins).
            "fov" => self.fov = parse_option_float(scalar)? * 40.0 + 70.0,
            "gamma" => self.gamma = parse_option_float(scalar)?,
            "saturation" => self.saturation = parse_option_float(scalar)?,
            "renderDistance" => self.view_distance = parse_int(scalar)?,
            "guiScale" => self.gui_scale = parse_int(scalar)?,
            "particles" => self.particles = parse_int(scalar)?,
            "bobView" => self.view_bobbing = parse_bool(scalar)?,
            "anaglyph3d" => self.anaglyph = parse_bool(scalar)?,
            "advancedOpengl" => self.advanced_opengl = parse_bool(scalar)?,
            "maxFps" => self.fps_limit = parse_int(scalar)?,
            "fboEnable" => self.fbo_enable = parse_bool(scalar)?,
            "difficulty" => self.difficulty = parse_int(scalar)?,
            "fancyGraphics" => self.fancy_graphics = parse_bool(scalar)?,
            "ao" => {
                self.ambient_occlusion = if scalar == "true" {
                    2
                } else if scalar == "false" {
                    0
                } else {
                    parse_int(scalar)?
                }
            }
            "clouds" => self.render_clouds = parse_bool(scalar)?,
            "resourcePacks" => self.resource_packs = parse_string_list(rest)?,
            "lastServer" => self.last_server = rest.to_string(),
            "lang" => self.language = rest.to_string(),
            "chatVisibility" => self.chat_visibility = parse_int(scalar)?,
            "chatColors" => self.chat_colors = parse_bool(scalar)?,
            "chatLinks" => self.chat_links = parse_bool(scalar)?,
            "chatLinksPrompt" => self.chat_links_prompt = parse_bool(scalar)?,
            "chatOpacity" => self.chat_opacity = parse_option_float(scalar)?,
            "snooperEnabled" => self.snooper_enabled = parse_bool(scalar)?,
            "fullscreen" => self.fullscreen = parse_bool(scalar)?,
            "enableVsync" => self.vsync = parse_bool(scalar)?,
            "hideServerAddress" => self.hide_server_address = parse_bool(scalar)?,
            "advancedItemTooltips" => self.advanced_item_tooltips = parse_bool(scalar)?,
            "pauseOnLostFocus" => self.pause_on_unfocus = parse_bool(scalar)?,
            "showCape" => self.show_cape = parse_bool(scalar)?,
            "touchscreen" => self.touchscreen = parse_bool(scalar)?,
            "overrideWidth" => self.override_width = parse_int(scalar)?,
            "overrideHeight" => self.override_height = parse_int(scalar)?,
            "heldItemTooltips" => self.item_in_hand_tooltips = parse_bool(scalar)?,
            "chatHeightFocused" => self.focused_chat_height = parse_option_float(scalar)?,
            "chatHeightUnfocused" => self.unfocused_chat_height = parse_option_float(scalar)?,
            "chatScale" => self.chat_scale = parse_option_float(scalar)?,
            "chatWidth" => self.chat_width = parse_option_float(scalar)?,
            "showInventoryAchievementHint" => {
                self.show_inventory_achievement_hint = parse_bool(scalar)?
            }
            "mipmapLevels" => self.mipmap_levels = parse_int(scalar)?,
            "anisotropicFiltering" => self.anisotropic_filtering = parse_int(scalar)?,
            "streamBytesPerPixel" => self.stream_bytes_per_pixel = parse_option_float(scalar)?,
            "streamMicVolume" => self.stream_mic_volume = parse_option_float(scalar)?,
            "streamSystemVolume" => self.stream_system_volume = parse_option_float(scalar)?,
            "streamKbps" => self.stream_kbps = parse_option_float(scalar)?,
            "streamFps" => self.stream_fps = parse_option_float(scalar)?,
            "streamCompression" => self.stream_compression = parse_int(scalar)?,
            "streamSendMetadata" => self.stream_send_metadata = parse_bool(scalar)?,
            "streamPreferredServer" => self.stream_preferred_server = rest.to_string(),
            "streamChatEnabled" => self.stream_chat_enabled = parse_int(scalar)?,
            "streamChatUserFilter" => self.stream_chat_user_filter = parse_int(scalar)?,
            "streamMicToggleBehavior" => self.stream_mic_toggle_behavior = parse_int(scalar)?,
            "forceUnicodeFont" => self.force_unicode_font = parse_bool(scalar)?,
            "soundCategory_master" => {
                self.set_sound_category_volume(SoundCategory::Master, parse_option_float(scalar)?)
            }
            "soundCategory_music" => {
                self.set_sound_category_volume(SoundCategory::Music, parse_option_float(scalar)?)
            }
            "soundCategory_record" => {
                self.set_sound_category_volume(SoundCategory::Records, parse_option_float(scalar)?)
            }
            "soundCategory_weather" => {
                self.set_sound_category_volume(SoundCategory::Weather, parse_option_float(scalar)?)
            }
            "soundCategory_block" => {
                self.set_sound_category_volume(SoundCategory::Blocks, parse_option_float(scalar)?)
            }
            "soundCategory_hostile" => {
                self.set_sound_category_volume(SoundCategory::Mobs, parse_option_float(scalar)?)
            }
            "soundCategory_neutral" => {
                self.set_sound_category_volume(SoundCategory::Animals, parse_option_float(scalar)?)
            }
            "soundCategory_player" => {
                self.set_sound_category_volume(SoundCategory::Players, parse_option_float(scalar)?)
            }
            "soundCategory_ambient" => {
                self.set_sound_category_volume(SoundCategory::Ambient, parse_option_float(scalar)?)
            }
            other => {
                // Key lines are "key_" + KeyBinding.getName().
                let name = other.strip_prefix("key_").ok_or(())?;
                let binding = self
                    .keys
                    .all_mut()
                    .into_iter()
                    .find(|binding| binding.name == name)
                    .ok_or(())?;
                binding.code = parse_int(scalar)?;
            }
        }
        Ok(())
    }

    /// `save`: the full options.txt text, in vanilla's line order, with the
    /// era's float formatting (`Float.toString` keeps a trailing ".0").
    pub fn save(&self) -> String {
        let mut out = String::new();
        macro_rules! line {
            ($($content:tt)*) => {
                out.push_str(&format!($($content)*));
                out.push('\n');
            };
        }
        let boolean = |value: bool| if value { "true" } else { "false" };
        line!("invertYMouse:{}", boolean(self.invert_mouse_y));
        line!("mouseSensitivity:{}", java_float(self.mouse_sensitivity));
        line!("fov:{}", java_float((self.fov - 70.0) / 40.0));
        line!("gamma:{}", java_float(self.gamma));
        line!("saturation:{}", java_float(self.saturation));
        line!("renderDistance:{}", self.view_distance);
        line!("guiScale:{}", self.gui_scale);
        line!("particles:{}", self.particles);
        line!("bobView:{}", boolean(self.view_bobbing));
        line!("anaglyph3d:{}", boolean(self.anaglyph));
        line!("advancedOpengl:{}", boolean(self.advanced_opengl));
        line!("maxFps:{}", self.fps_limit);
        line!("fboEnable:{}", boolean(self.fbo_enable));
        line!("difficulty:{}", self.difficulty);
        line!("fancyGraphics:{}", boolean(self.fancy_graphics));
        line!("ao:{}", self.ambient_occlusion);
        line!("clouds:{}", boolean(self.render_clouds));
        line!("resourcePacks:{}", write_string_list(&self.resource_packs));
        line!("lastServer:{}", self.last_server);
        line!("lang:{}", self.language);
        line!("chatVisibility:{}", self.chat_visibility);
        line!("chatColors:{}", boolean(self.chat_colors));
        line!("chatLinks:{}", boolean(self.chat_links));
        line!("chatLinksPrompt:{}", boolean(self.chat_links_prompt));
        line!("chatOpacity:{}", java_float(self.chat_opacity));
        line!("snooperEnabled:{}", boolean(self.snooper_enabled));
        line!("fullscreen:{}", boolean(self.fullscreen));
        line!("enableVsync:{}", boolean(self.vsync));
        line!("hideServerAddress:{}", boolean(self.hide_server_address));
        line!(
            "advancedItemTooltips:{}",
            boolean(self.advanced_item_tooltips)
        );
        line!("pauseOnLostFocus:{}", boolean(self.pause_on_unfocus));
        line!("showCape:{}", boolean(self.show_cape));
        line!("touchscreen:{}", boolean(self.touchscreen));
        line!("overrideWidth:{}", self.override_width);
        line!("overrideHeight:{}", self.override_height);
        line!("heldItemTooltips:{}", boolean(self.item_in_hand_tooltips));
        line!("chatHeightFocused:{}", java_float(self.focused_chat_height));
        line!(
            "chatHeightUnfocused:{}",
            java_float(self.unfocused_chat_height)
        );
        line!("chatScale:{}", java_float(self.chat_scale));
        line!("chatWidth:{}", java_float(self.chat_width));
        line!(
            "showInventoryAchievementHint:{}",
            boolean(self.show_inventory_achievement_hint)
        );
        line!("mipmapLevels:{}", self.mipmap_levels);
        line!("anisotropicFiltering:{}", self.anisotropic_filtering);
        line!(
            "streamBytesPerPixel:{}",
            java_float(self.stream_bytes_per_pixel)
        );
        line!("streamMicVolume:{}", java_float(self.stream_mic_volume));
        line!(
            "streamSystemVolume:{}",
            java_float(self.stream_system_volume)
        );
        line!("streamKbps:{}", java_float(self.stream_kbps));
        line!("streamFps:{}", java_float(self.stream_fps));
        line!("streamCompression:{}", self.stream_compression);
        line!("streamSendMetadata:{}", boolean(self.stream_send_metadata));
        line!("streamPreferredServer:{}", self.stream_preferred_server);
        line!("streamChatEnabled:{}", self.stream_chat_enabled);
        line!("streamChatUserFilter:{}", self.stream_chat_user_filter);
        line!(
            "streamMicToggleBehavior:{}",
            self.stream_mic_toggle_behavior
        );
        line!("forceUnicodeFont:{}", boolean(self.force_unicode_font));
        for binding in self.keys.all() {
            line!("key_{}:{}", binding.name, binding.code);
        }
        for category in SoundCategory::ALL {
            line!(
                "soundCategory_{}:{}",
                category.name(),
                java_float(self.sound_category_volume(category))
            );
        }
        out
    }
}

/// The `ClientSettingsC2SPacket` fields (`GameOptions.syncClientSettings`).
/// The packet encoding itself lives in the protocol crate; the payload is the
/// language, view distance, chat visibility, chat colours, difficulty and
/// showCape.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientSettings {
    pub language: String,
    pub view_distance: i32,
    /// 0 full, 1 system, 2 hidden (`PlayerEntity$ChatVisibility`).
    pub chat_visibility: i32,
    pub chat_colors: bool,
    /// 0 peaceful, 1 easy, 2 normal, 3 hard (`Difficulty.byId`).
    pub difficulty: i32,
    pub show_cape: bool,
}

/// What a key code reads as in the controls menu (`GameOptions.getKeyName`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyName {
    /// A keyboard key: its LWJGL 2 name (`Keyboard.getKeyName`).
    Keyboard(&'static str),
    /// A mouse button: `key.mouseButton` ([`KEY_MOUSE_BUTTON`]) with the
    /// button number; the vanilla translation reads "Button %d".
    Mouse(i32),
}

/// The name a code reads as in the controls menu (`GameOptions.getKeyName`):
/// mouse buttons read `key.mouseButton` with the button number, keyboard keys
/// their LWJGL 2 name, `None` outside LWJGL 2's named set.
pub fn key_display_name(key_code: i32) -> Option<KeyName> {
    if key_code < 0 {
        Some(KeyName::Mouse(mouse_button_number(key_code)))
    } else {
        lwjgl_key_name(key_code).map(KeyName::Keyboard)
    }
}

/// Java's `Float.toString`: always a decimal point ("1.0", "-0.0"), shortest
/// round-trip digits otherwise.
///
/// ponytail: Rust's shortest round-trip plus the ".0" fix matches Java for
/// every realistic option value; the scientific notation Java switches to
/// outside 1e-3..1e7 is not ported — add it if an options.txt with such
/// values must round-trip.
pub fn java_float(value: f32) -> String {
    let text = format!("{value}");
    if text.contains('.') || text.contains('e') || text.contains("inf") || text.contains("NaN") {
        text
    } else {
        format!("{text}.0")
    }
}

/// `GameOptions.parseFloat`: "true"/"false" read as 1.0/0.0.
fn parse_option_float(value: &str) -> Result<f32, ()> {
    if value == "true" {
        Ok(1.0)
    } else if value == "false" {
        Ok(0.0)
    } else {
        value.parse().map_err(|_| ())
    }
}

fn parse_bool(value: &str) -> Result<bool, ()> {
    if value == "true" {
        Ok(true)
    } else if value == "false" {
        Ok(false)
    } else {
        Err(())
    }
}

fn parse_int(value: &str) -> Result<i32, ()> {
    value.parse().map_err(|_| ())
}

/// Gson's string list form: `[]`, `["a","b"]`.
fn write_string_list(list: &[String]) -> String {
    let mut out = String::from("[");
    for (index, value) in list.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('"');
        for c in value.chars() {
            if c == '"' || c == '\\' {
                out.push('\\');
            }
            out.push(c);
        }
        out.push('"');
    }
    out.push(']');
    out
}

/// Parse Gson's string list form (`[]`, `["a","b"]`), quotes and escapes only.
fn parse_string_list(text: &str) -> Result<Vec<String>, ()> {
    let text = text.strip_prefix('[').ok_or(())?;
    let text = text.strip_suffix(']').ok_or(())?;
    let mut list = Vec::new();
    let mut chars = text.chars().peekable();
    while chars.peek().is_some() {
        if chars.next() != Some('"') {
            return Err(());
        }
        let mut value = String::new();
        loop {
            match chars.next() {
                Some('"') => break,
                Some('\\') => value.push(chars.next().ok_or(())?),
                Some(c) => value.push(c),
                None => return Err(()),
            }
        }
        list.push(value);
        match chars.peek() {
            Some(',') => {
                chars.next();
            }
            Some(_) => return Err(()),
            None => break,
        }
    }
    Ok(list)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The fresh options.txt a vanilla 1.7.10 64-bit client writes before any
    /// change (from `GameOptions.save` and the field defaults).
    const FRESH_SAVE: &str = "\
invertYMouse:false
mouseSensitivity:0.5
fov:0.0
gamma:0.0
saturation:0.0
renderDistance:12
guiScale:0
particles:0
bobView:true
anaglyph3d:false
advancedOpengl:false
maxFps:120
fboEnable:true
difficulty:2
fancyGraphics:true
ao:2
clouds:true
resourcePacks:[]
lastServer:
lang:en_US
chatVisibility:0
chatColors:true
chatLinks:true
chatLinksPrompt:true
chatOpacity:1.0
snooperEnabled:true
fullscreen:false
enableVsync:true
hideServerAddress:false
advancedItemTooltips:false
pauseOnLostFocus:true
showCape:true
touchscreen:false
overrideWidth:0
overrideHeight:0
heldItemTooltips:true
chatHeightFocused:1.0
chatHeightUnfocused:0.44366196
chatScale:1.0
chatWidth:1.0
showInventoryAchievementHint:true
mipmapLevels:4
anisotropicFiltering:1
streamBytesPerPixel:0.5
streamMicVolume:1.0
streamSystemVolume:1.0
streamKbps:0.5412844
streamFps:0.31690142
streamCompression:1
streamSendMetadata:true
streamPreferredServer:
streamChatEnabled:0
streamChatUserFilter:0
streamMicToggleBehavior:0
forceUnicodeFont:false
key_key.attack:-100
key_key.use:-99
key_key.forward:17
key_key.left:30
key_key.back:31
key_key.right:32
key_key.jump:57
key_key.sneak:42
key_key.drop:16
key_key.inventory:18
key_key.chat:20
key_key.playerlist:15
key_key.pickItem:-98
key_key.command:53
key_key.screenshot:60
key_key.togglePerspective:63
key_key.smoothCamera:0
key_key.sprint:29
key_key.streamStartStop:64
key_key.streamPauseUnpause:65
key_key.streamCommercial:0
key_key.streamToggleMic:0
key_key.fullscreen:87
key_key.hotbar.1:2
key_key.hotbar.2:3
key_key.hotbar.3:4
key_key.hotbar.4:5
key_key.hotbar.5:6
key_key.hotbar.6:7
key_key.hotbar.7:8
key_key.hotbar.8:9
key_key.hotbar.9:10
soundCategory_master:1.0
soundCategory_music:1.0
soundCategory_record:1.0
soundCategory_weather:1.0
soundCategory_block:1.0
soundCategory_hostile:1.0
soundCategory_neutral:1.0
soundCategory_player:1.0
soundCategory_ambient:1.0
";

    #[test]
    fn fresh_save_matches_vanilla() {
        assert_eq!(Options::default().save(), FRESH_SAVE);
    }

    #[test]
    fn eight_bit_machine_sees_fewer_chunks() {
        assert_eq!(Options::new(false).view_distance, 8);
        assert_eq!(Options::new(true).view_distance, 12);
    }

    #[test]
    fn load_save_round_trips() {
        let mut options = Options::default();
        options.load(FRESH_SAVE);
        assert_eq!(options.save(), FRESH_SAVE);
    }

    #[test]
    fn load_applies_changed_values() {
        let mut options = Options::new(true);
        options.load(FRESH_SAVE);
        options.set_sound_category_volume(SoundCategory::Music, 0.25);
        options.keys.forward.code = 200;
        let text = options.save();
        assert!(text.contains("soundCategory_music:0.25"));
        assert!(text.contains("key_key.forward:200"));

        let mut reloaded = Options::default();
        reloaded.load(&text);
        assert_eq!(reloaded.sound_category_volume(SoundCategory::Music), 0.25);
        assert_eq!(reloaded.keys.forward.code, 200);
        assert_eq!(
            reloaded.keys.by_code(200).map(|b| b.name),
            Some("key.forward")
        );
    }

    #[test]
    fn bad_lines_are_skipped() {
        let mut options = Options::default();
        options.load("gamma:notanumber\nfov:0.5\nmaxFps:\n");
        // gamma stays at its default, fov applies, maxFps (no value) skips.
        assert_eq!(options.gamma, 0.0);
        assert_eq!(options.fov, 0.5 * 40.0 + 70.0);
        assert_eq!(options.fps_limit, 120);
    }

    #[test]
    fn ao_reads_old_bool_form() {
        let mut options = Options::default();
        options.load("ao:true\n");
        assert_eq!(options.ambient_occlusion, 2);
        options.load("ao:false\n");
        assert_eq!(options.ambient_occlusion, 0);
        options.load("ao:1\n");
        assert_eq!(options.ambient_occlusion, 1);
    }

    #[test]
    fn fov_saves_normalized_slider_value() {
        let mut options = Options::new(true);
        options.fov = 110.0;
        let saved = options.save();
        assert!(saved.contains("fov:1.0"));
        let mut reloaded = Options::default();
        reloaded.load(&saved);
        assert_eq!(reloaded.fov, 110.0);
    }

    #[test]
    fn resource_packs_round_trip() {
        let mut options = Options::new(true);
        options.resource_packs = vec!["a.zip".to_string(), "b\"x\\y".to_string()];
        let saved = options.save();
        assert!(saved.contains("resourcePacks:[\"a.zip\",\"b\\\"x\\\\y\"]"));
        let mut reloaded = Options::default();
        reloaded.load(&saved);
        assert_eq!(reloaded.resource_packs, options.resource_packs);
    }

    #[test]
    fn last_server_keeps_colons() {
        let mut options = Options::default();
        options.load("lastServer:mc.example.com:25565\n");
        assert_eq!(options.last_server, "mc.example.com:25565");
    }

    #[test]
    fn java_float_keeps_trailing_zero() {
        assert_eq!(java_float(1.0), "1.0");
        assert_eq!(java_float(0.5), "0.5");
        assert_eq!(java_float(-0.0), "-0.0");
        assert_eq!(java_float(0.44366196), "0.44366196");
        assert_eq!(java_float(100.0), "100.0");
    }

    #[test]
    fn client_settings_match_sync() {
        let options = Options::default();
        assert_eq!(
            options.client_settings(),
            ClientSettings {
                language: "en_US".to_string(),
                view_distance: 12,
                chat_visibility: 0,
                chat_colors: true,
                difficulty: 2,
                show_cape: true,
            }
        );
    }

    #[test]
    fn key_display_names_read_like_vanilla() {
        assert_eq!(key_display_name(17), Some(KeyName::Keyboard("W")));
        assert_eq!(key_display_name(60), Some(KeyName::Keyboard("F2")));
        assert_eq!(key_display_name(-100), Some(KeyName::Mouse(1)));
        assert_eq!(key_display_name(-98), Some(KeyName::Mouse(3)));
        assert_eq!(key_display_name(12345), None);
    }
}
