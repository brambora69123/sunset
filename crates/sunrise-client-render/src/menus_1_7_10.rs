//! Minecraft 1.7.10's menu screens: the pause menu, Options and its Video
//! Settings, Music & Sounds, Controls and Language screens.
//!
//! The layouts are the era's own: `GameMenuScreen.init` (the pause menu's
//! buttons), `OptionsScreen.init` (the two-column 150px grid and its
//! `gui.done`), `VideoOptionsScreen.init` (`OptionListWidget` pairs in a
//! 25px-row list), `SoundsScreen.init` (a 310px master slider over the
//! categories), `ControlsOptionsScreen.init` (`ControlsListWidget` from y 63
//! to the footer) and `LanguageOptionsScreen.init` (an 18px-row list). The
//! widgets are the era's `ButtonWidget`, `OptionButtonWidget`,
//! `OptionSliderWidget` and `ListWidget`, drawn from `textures/gui/widgets.png`
//! and `textures/gui/options_background.png`.
//!
//! Text is the era's `en_US.lang`: [`text`] answers a key the loaded language
//! has and `None` for one it lacks, and a widget whose label the language
//! cannot give is left out rather than drawn as its key. [`assert_translated`]
//! guards every visible label in debug builds.

use crate::{
    hud::{HudAtlas, HudBatch, Rect, rgba},
    hud_1_7_10::Icon,
    lang::{self, LanguageInfo},
    settings::{SOUND_SOURCES, Settings},
    text::Text,
    widgets::{self, Narratable},
};

/// The era's widget sheet (`ButtonWidget.WIDGETS_LOCATION`).
pub const WIDGETS: &str = "gui/widgets";
/// The tiled menu background (`GuiElement.BACKGROUND_LOCATION`).
pub const OPTIONS_BACKGROUND: &str = "gui/options_background";

/// `ButtonWidget`'s label colours.
pub const BUTTON_TEXT: u32 = rgba(0xE0, 0xE0, 0xE0, 0xff);
/// `ButtonWidget`'s hovered label colour.
pub const BUTTON_HOVER_TEXT: u32 = rgba(0xFF, 0xFF, 0xA0, 0xff);
/// `ButtonWidget`'s inactive label colour.
pub const BUTTON_INACTIVE_TEXT: u32 = rgba(0xA0, 0xA0, 0xA0, 0xff);
/// `Screen.drawCenteredString(..., 16777215)`.
pub const TITLE_TEXT: u32 = rgba(0xFF, 0xFF, 0xFF, 0xff);
/// `LanguageOptionsScreen.render`'s warning colour (8421504).
pub const WARNING_TEXT: u32 = rgba(0x80, 0x80, 0x80, 0xff);

/// `Screen.renderBackground`: the in-world dim over the world.
const DIM_TOP: u32 = rgba(0x10, 0x10, 0x10, 0xC0);
const DIM_BOTTOM: u32 = rgba(0x10, 0x10, 0x10, 0xD0);
/// `Screen.drawBackgroundTexture`/`ListWidget.render`'s tint of the tiled dirt.
const MENU_TINT: u32 = rgba(0x40, 0x40, 0x40, 0xff);
const LIST_TINT: u32 = rgba(0x20, 0x20, 0x20, 0xff);

/// `ButtonWidget.getYImage`'s rows in `widgets.png`: inactive, normal, hovered.
const BUTTON_INACTIVE_V: i32 = 46;
const BUTTON_NORMAL_V: i32 = 66;
const BUTTON_HOVER_V: i32 = 86;
/// `OptionSliderWidget.renderBackground`'s handle halves.
const SLIDER_HANDLE_V: i32 = 66;

/// `Font.getHeight`, for vertical positions the era centred by hand.
const FONT_HEIGHT: i32 = 9;

/// `GameOptions$Option.RENDER_DISTANCE`'s end: 1.7.10's slider stops at 16
/// chunks, whatever the engine behind it can stream.
const ERA_RENDER_DISTANCE: i32 = 16;

/// The era's screens this module lays out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Menu17 {
    Pause,
    Options,
    Video,
    Sounds,
    Controls,
    Language,
}

impl Menu17 {
    /// The title's translation key, drawn by `drawCenteredString`.
    fn title_key(self) -> &'static str {
        match self {
            Self::Pause => "menu.game",
            Self::Options => "options.title",
            Self::Video => "options.videoTitle",
            Self::Sounds => "options.sounds.title",
            Self::Controls => "controls.title",
            Self::Language => "options.language",
        }
    }

    /// The screen a child of Options returns to.
    pub fn parent(self) -> Option<Self> {
        match self {
            Self::Options | Self::Pause => None,
            Self::Video | Self::Sounds | Self::Controls | Self::Language => Some(Self::Options),
        }
    }
}

/// What pressing a widget does, as the era's button ids did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action17 {
    /// `GameMenuScreen` id 4: back to the game.
    BackToGame,
    /// `GameMenuScreen` id 0: the Options screen.
    OpenOptions,
    /// `GameMenuScreen` id 6.
    Statistics,
    /// `GameMenuScreen` id 1.
    QuitWorld,
    /// `OptionsScreen` id 101.
    OpenVideo,
    /// `OptionsScreen` id 106.
    OpenSounds,
    /// `OptionsScreen` id 100.
    OpenControls,
    /// `OptionsScreen` id 102.
    OpenLanguage,
    /// `gui.done`: one level back.
    Done,
    /// `ControlsOptionsScreen` id 201.
    ResetAllKeys,
    /// `ControlsOptionsScreen`'s `GameOptions$Option.INVERT_MOUSE`.
    InvertMouse,
    /// `ControlsOptionsScreen`'s `GameOptions$Option.SENSITIVITY` slider.
    Sensitivity,
    /// A key binding button: `KeyBindingEntry.keyBindingButton`.
    BindKey(&'static str),
    /// A key binding's `controls.reset` button.
    ResetKey(&'static str),
    /// A language row: its index in `MenuView17.languages`.
    Language(usize),
    /// A Video Settings option (`OptionListWidget`'s buttons and sliders).
    Video(VideoOption17),
    /// A sound category's slider: its index in [`SOUND_SOURCES`].
    Sound(usize),
    /// `LanguageOptionsScreen`'s `options.forceUnicodeFont` button.
    ForceUnicode,
}

/// `VideoOptionsScreen.VIDEO_OPTIONS`, without the entries this port has no
/// setting for (3D Anaglyph and Advanced OpenGL) and anisotropic filtering,
/// whose era values this port's sampler does not take.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VideoOption17 {
    Graphics,
    RenderDistance,
    SmoothLighting,
    MaxFramerate,
    ViewBobbing,
    GuiScale,
    Brightness,
    Clouds,
    Particles,
    Fullscreen,
    Vsync,
    Mipmaps,
}

impl VideoOption17 {
    /// The pairs `OptionListWidget` puts two to a row, in `VIDEO_OPTIONS` order.
    const ROWS: [[Option<Self>; 2]; 6] = [
        [Some(Self::Graphics), Some(Self::RenderDistance)],
        [Some(Self::SmoothLighting), Some(Self::MaxFramerate)],
        [Some(Self::ViewBobbing), Some(Self::GuiScale)],
        [Some(Self::Brightness), Some(Self::Clouds)],
        [Some(Self::Particles), Some(Self::Fullscreen)],
        [Some(Self::Vsync), Some(Self::Mipmaps)],
    ];

    /// `GameOptions$Option.getName`.
    fn caption(self) -> &'static str {
        match self {
            Self::Graphics => "options.graphics",
            Self::RenderDistance => "options.renderDistance",
            Self::SmoothLighting => "options.ao",
            Self::MaxFramerate => "options.framerateLimit",
            Self::ViewBobbing => "options.viewBobbing",
            Self::GuiScale => "options.guiScale",
            Self::Brightness => "options.gamma",
            Self::Clouds => "options.renderClouds",
            Self::Particles => "options.particles",
            Self::Fullscreen => "options.fullscreen",
            Self::Vsync => "options.vsync",
            Self::Mipmaps => "options.mipmapLevels",
        }
    }

    /// Whether `OptionListWidget.createWidget` makes a slider for it.
    fn is_slider(self) -> bool {
        matches!(
            self,
            Self::RenderDistance | Self::MaxFramerate | Self::Brightness | Self::Mipmaps
        )
    }

    /// A slider's position (`GameOptions$Option.normalize`).
    pub fn position(self, s: &Settings) -> Option<f32> {
        Some(
            match self {
                Self::RenderDistance => {
                    (s.render_distance - 2) as f32 / (ERA_RENDER_DISTANCE - 2) as f32
                }
                Self::MaxFramerate => (s.max_fps as f32 - 10.0) / (260.0 - 10.0),
                Self::Brightness => s.gamma as f32,
                Self::Mipmaps => s.mipmap_levels as f32 / 4.0,
                _ => return None,
            }
            .clamp(0.0, 1.0),
        )
    }

    /// `OptionSliderWidget`'s value for a position along the track
    /// (`GameOptions$Option.denormalize`).
    pub fn slide(self, s: &mut Settings, position: f32) {
        let v = position.clamp(0.0, 1.0);
        match self {
            Self::RenderDistance => {
                s.render_distance = 2 + (v * (ERA_RENDER_DISTANCE - 2) as f32).round() as i32;
            }
            // The era's fps step is 10, from 10 to 260.
            Self::MaxFramerate => s.max_fps = (10.0 + (v * 25.0).round() * 10.0) as u32,
            Self::Brightness => s.gamma = f64::from(v),
            Self::Mipmaps => s.mipmap_levels = (v * 4.0).round() as i32,
            _ => {}
        }
    }

    /// `OptionListWidget$Entry.mouseClicked`'s `options.set(option, 1)`: a
    /// cycle button's next value.
    pub fn cycle(self, s: &mut Settings, max_scale: u32) {
        match self {
            Self::Graphics => {
                s.graphics_preset = if s.graphics_preset == "fast" {
                    "fancy".into()
                } else {
                    "fast".into()
                };
                crate::options_ui::apply_preset(s);
            }
            Self::SmoothLighting => s.smooth_lighting = !s.smooth_lighting,
            Self::ViewBobbing => s.view_bobbing = !s.view_bobbing,
            Self::GuiScale => {
                let max = max_scale.max(1);
                s.gui_scale = if s.gui_scale >= max {
                    0
                } else {
                    s.gui_scale + 1
                };
            }
            Self::Clouds => {
                s.clouds = if s.clouds == "false" {
                    "true".into()
                } else {
                    "false".into()
                };
            }
            Self::Particles => s.particles = (s.particles + 1) % 3,
            Self::Fullscreen => s.fullscreen = !s.fullscreen,
            Self::Vsync => s.vsync = !s.vsync,
            _ => {}
        }
    }

    /// `GameOptions.getAsString(option)`, or `None` when the language lacks a
    /// piece of it (the widget is then left out).
    fn label(self, s: &Settings) -> Option<String> {
        let caption = text(self.caption())?;
        let value = match self {
            Self::Graphics => text(if s.graphics_preset == "fast" {
                "options.graphics.fast"
            } else {
                "options.graphics.fancy"
            })?,
            // The era spells these with literal units, as 1.7.10 did.
            Self::RenderDistance => format!("{} chunks", s.render_distance),
            Self::SmoothLighting => text(if s.smooth_lighting {
                "options.ao.max"
            } else {
                "options.ao.off"
            })?,
            Self::MaxFramerate => {
                if s.max_fps >= 260 {
                    text("options.framerateLimit.max")?
                } else {
                    format!("{} fps", s.max_fps)
                }
            }
            Self::ViewBobbing => on_off(s.view_bobbing)?,
            Self::GuiScale => match s.gui_scale {
                0 => text("options.guiScale.auto")?,
                1 => text("options.guiScale.small")?,
                2 => text("options.guiScale.normal")?,
                3 => text("options.guiScale.large")?,
                n => n.to_string(),
            },
            Self::Brightness => match (s.gamma * 100.0).round() as i32 {
                0 => text("options.gamma.min")?,
                100 => text("options.gamma.max")?,
                n => format!("+{n}%"),
            },
            Self::Clouds => on_off(s.clouds != "false")?,
            Self::Particles => text(match s.particles.min(2) {
                0 => "options.particles.all",
                1 => "options.particles.decreased",
                _ => "options.particles.minimal",
            })?,
            Self::Fullscreen => on_off(s.fullscreen)?,
            Self::Vsync => on_off(s.vsync)?,
            Self::Mipmaps => {
                if s.mipmap_levels == 0 {
                    text("options.off")?
                } else {
                    s.mipmap_levels.min(4).to_string()
                }
            }
        };
        Some(format!("{caption}: {value}"))
    }
}

/// A widget's kind, as the era's classes drew and hit it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind17 {
    Button,
    /// `OptionSliderWidget`: its handle's position 0..1 and the track a click
    /// spans (`x + 4`, `width - 8`), for the control socket's `set`.
    Slider {
        value: f32,
        track: [f32; 2],
    },
    /// `ListWidget`'s selection highlight around a language row.
    Row {
        selected: bool,
    },
}

/// One clickable widget of a menu, in GUI pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct Widget17 {
    pub rect: [i32; 4],
    pub label: String,
    pub action: Action17,
    pub kind: Kind17,
    pub active: bool,
    /// Scrolled with the screen's list: drawn and hit only inside it.
    pub in_list: bool,
}

impl Widget17 {
    fn button(x: i32, y: i32, w: i32, action: Action17, label: String, active: bool) -> Self {
        Self {
            rect: [x, y, w, 20],
            label,
            action,
            kind: Kind17::Button,
            active,
            in_list: false,
        }
    }

    fn slider(x: i32, y: i32, w: i32, action: Action17, label: String, value: f32) -> Self {
        Self {
            rect: [x, y, w, 20],
            label,
            action,
            kind: Kind17::Slider {
                value,
                track: [x as f32 + 4.0, w as f32 - 8.0],
            },
            active: true,
            in_list: false,
        }
    }

    /// The same widget inside a scrolling list.
    fn listed(mut self) -> Self {
        self.in_list = true;
        self
    }

    /// A widget whose height is not the 20 a button defaults to.
    fn with_height(mut self, height: i32) -> Self {
        self.rect[3] = height;
        self
    }

    fn contains(&self, x: f32, y: f32) -> bool {
        let [wx, wy, ww, wh] = self.rect;
        x >= wx as f32 && x < (wx + ww) as f32 && y >= wy as f32 && y < (wy + wh) as f32
    }
}

/// A string `TextRenderer.draw`/`drawCenteredString` draws.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Label17 {
    pub text: String,
    pub x: i32,
    pub y: i32,
    pub color: u32,
    /// Centred on `x` rather than starting there.
    pub centered: bool,
    /// `TextRenderer.drawWithShadow`.
    pub shadow: bool,
    /// A row of a scrolling list: clipped to the list like its widgets.
    pub in_list: bool,
}

impl Label17 {
    fn centered(text: String, x: i32, y: i32, color: u32) -> Self {
        Self {
            text,
            x,
            y,
            color,
            centered: true,
            shadow: true,
            in_list: false,
        }
    }
}

/// A scrolling list's geometry (`ListWidget`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct List17 {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    /// The rows' full height (`ListWidget.getHeight`).
    pub content: i32,
    /// The scrollbar's left edge (`getScrollbarPosition`).
    pub scrollbar_x: i32,
    pub scroll: i32,
}

impl List17 {
    /// `ListWidget.getMaxScroll`, with the era's `centerAlongY = false` list
    /// never scrolling while the rows fit.
    pub fn max_scroll(self) -> i32 {
        (self.content - (self.h - 4)).max(0)
    }

    fn holds(self, y: f32) -> bool {
        y >= self.y as f32 && y < (self.y + self.h) as f32
    }
}

/// Everything a screen draws, in the era's paint order.
#[derive(Clone, Debug, PartialEq)]
pub struct MenuLayout17 {
    pub menu: Menu17,
    pub title: Option<Label17>,
    pub labels: Vec<Label17>,
    pub widgets: Vec<Widget17>,
    pub list: Option<List17>,
    /// The action the keyboard or a controller focused.
    pub focus: Option<Action17>,
}

impl MenuLayout17 {
    /// The widget under a point, top-most first; widgets scrolled out of a
    /// list are not reachable.
    pub fn widget_at(&self, x: f32, y: f32) -> Option<&Widget17> {
        self.widgets.iter().rev().find(|w| {
            w.active
                && (!w.in_list || self.list.is_some_and(|list| list.holds(y)))
                && w.contains(x, y)
        })
    }

    /// The widgets a focus step can reach, in the order they were laid out:
    /// the list's own widgets scroll with it and are left out.
    pub fn focusable(&self) -> Vec<&Widget17> {
        self.widgets
            .iter()
            .filter(|w| w.active && !w.in_list)
            .collect()
    }
}

/// The era screens' inputs: the GUI-scaled size and the live state their
/// widgets read.
#[derive(Clone, Copy, Debug)]
pub struct MenuView17<'a> {
    pub width: i32,
    pub height: i32,
    /// `GameMenuScreen`: the world is a server's (`menu.disconnect`).
    pub remote: bool,
    pub scroll: i32,
    /// `Window.calculateScale`, for the GUI Scale button's cycle.
    pub max_scale: u32,
    pub languages: &'a [LanguageInfo],
    pub selected_language: &'a str,
    /// The `KeyBindingEntry` waiting for a key, if any.
    pub binding: Option<&'a str>,
    pub focus: Option<Action17>,
}

impl MenuView17<'_> {
    pub fn new(width: i32, height: i32, max_scale: u32) -> Self {
        Self {
            width,
            height,
            remote: false,
            scroll: 0,
            max_scale,
            languages: &[],
            selected_language: lang::DEFAULT,
            binding: None,
            focus: None,
        }
    }
}

/// `I18n.translate(key)`: the loaded language's text, or `None` when it has
/// no such key (the era's `en_US.lang` has no 26.3 keys).
pub fn text(key: &str) -> Option<String> {
    lang::has(key)
        .then(|| Text::translate(key, &[]).plain())
        .filter(|text| !text.is_empty())
}

/// `options.on`/`options.off`.
fn on_off(on: bool) -> Option<String> {
    text(if on { "options.on" } else { "options.off" })
}

/// The era's `options.generic_value` is a 26.3 key: `getAsString` joins the
/// caption and value with a colon itself.
fn caption_value(caption: &str, value: Option<String>) -> Option<String> {
    Some(format!("{}: {}", text(caption)?, value?))
}

/// The era's key name on a binding button (`GameOptions.getKeyName`):
/// `Keyboard.getKeyName`'s words, and `key.mouseButton`'s number.
pub fn key_name(key: &str) -> String {
    if key == "key.keyboard.unknown" {
        return "NONE".into();
    }
    if let Some(number) = key.strip_prefix("key.mouse.") {
        let number = match number {
            "left" => "1",
            "right" => "2",
            "middle" => "3",
            other => other,
        };
        return if lang::has("key.mouseButton") {
            Text::translate("key.mouseButton", &[Text::literal(number)]).plain()
        } else {
            format!("Button {number}")
        };
    }
    let Some(name) = key.strip_prefix("key.keyboard.") else {
        return "NONE".into();
    };
    let named = match name {
        "space" => "SPACE",
        "left.shift" => "LSHIFT",
        "right.shift" => "RSHIFT",
        "left.control" => "LCONTROL",
        "right.control" => "RCONTROL",
        "left.alt" => "LMENU",
        "right.alt" => "RMENU",
        "tab" => "TAB",
        "enter" => "RETURN",
        "escape" => "ESCAPE",
        "backspace" => "BACK",
        "up" => "UP",
        "down" => "DOWN",
        "left" => "LEFT",
        "right" => "RIGHT",
        "insert" => "INSERT",
        "delete" => "DELETE",
        "home" => "HOME",
        "end" => "END",
        "page.up" => "PAGEUP",
        "page.down" => "PAGEDOWN",
        "caps.lock" => "CAPITAL",
        "num.lock" => "NUMLOCK",
        "scroll.lock" => "SCROLL",
        "pause" => "PAUSE",
        "print.screen" => "SYSRQ",
        "keypad.divide" => "DIVIDE",
        "keypad.multiply" => "MULTIPLY",
        "keypad.subtract" => "SUBTRACT",
        "keypad.add" => "ADD",
        "keypad.enter" => "ENTER",
        "keypad.period" => "DECIMAL",
        "keypad.0" => "NUMPAD0",
        "keypad.1" => "NUMPAD1",
        "keypad.2" => "NUMPAD2",
        "keypad.3" => "NUMPAD3",
        "keypad.4" => "NUMPAD4",
        "keypad.5" => "NUMPAD5",
        "keypad.6" => "NUMPAD6",
        "keypad.7" => "NUMPAD7",
        "keypad.8" => "NUMPAD8",
        "keypad.9" => "NUMPAD9",
        "minus" => "-",
        "equal" => "=",
        "left.bracket" => "[",
        "right.bracket" => "]",
        "backslash" => "\\",
        "semicolon" => ";",
        "apostrophe" => "'",
        "grave.accent" => "`",
        "comma" => ",",
        "period" => ".",
        "slash" => "/",
        _ => {
            if let Some(letter) = name.strip_prefix('f')
                && !letter.is_empty()
                && letter.bytes().all(|b| b.is_ascii_digit())
            {
                return format!("F{letter}");
            }
            // LWJGL names a key it knows; a key it does not reads as its name
            // in upper case rather than as a translation key.
            return name.to_ascii_uppercase();
        }
    };
    named.into()
}

/// The era's key bindings (`GameOptions.keyBindings`) and the category each
/// button lists under. The Twitch keys this build has no streaming for are
/// left out.
pub const KEY_BINDINGS: [(&str, &str); 28] = [
    ("key.attack", "gameplay"),
    ("key.use", "gameplay"),
    ("key.forward", "movement"),
    ("key.left", "movement"),
    ("key.back", "movement"),
    ("key.right", "movement"),
    ("key.jump", "movement"),
    ("key.sneak", "movement"),
    ("key.drop", "inventory"),
    ("key.inventory", "inventory"),
    ("key.chat", "multiplayer"),
    ("key.playerlist", "multiplayer"),
    ("key.pickItem", "gameplay"),
    ("key.command", "multiplayer"),
    ("key.screenshot", "misc"),
    ("key.togglePerspective", "misc"),
    ("key.smoothCamera", "misc"),
    ("key.sprint", "gameplay"),
    ("key.hotbar.1", "inventory"),
    ("key.hotbar.2", "inventory"),
    ("key.hotbar.3", "inventory"),
    ("key.hotbar.4", "inventory"),
    ("key.hotbar.5", "inventory"),
    ("key.hotbar.6", "inventory"),
    ("key.hotbar.7", "inventory"),
    ("key.hotbar.8", "inventory"),
    ("key.hotbar.9", "inventory"),
    ("key.fullscreen", "misc"),
];

/// One row of a `ControlsListWidget`: a category header or a binding.
#[derive(Clone, Debug, PartialEq)]
enum ControlRow17 {
    Category { name: String },
    Binding { name: &'static str, label: String },
}

/// `ControlsListWidget`'s entries: the era's bindings sorted by their
/// category's text then their own text, with a header where the category
/// changes.
fn control_rows() -> Vec<ControlRow17> {
    let mut rows: Vec<(String, String, &'static str)> = Vec::new();
    for &(name, category) in &KEY_BINDINGS {
        let Some(label) = text(name) else {
            continue;
        };
        let Some(category_text) = text(&format!("key.categories.{category}")) else {
            continue;
        };
        rows.push((category_text, label, name));
    }
    rows.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    let mut out = Vec::with_capacity(rows.len() + 5);
    let mut category = String::new();
    for (category_text, label, name) in rows {
        if category_text != category {
            category = category_text.clone();
            out.push(ControlRow17::Category {
                name: category_text,
            });
        }
        out.push(ControlRow17::Binding { name, label });
    }
    out
}

/// The maximum key name's width, which `KeyBindingEntry` right-aligns the
/// descriptions against.
fn max_control_label_width(rows: &[ControlRow17], width: &dyn Fn(&str) -> i32) -> i32 {
    rows.iter()
        .filter_map(|row| match row {
            ControlRow17::Binding { label, .. } => Some(width(label)),
            ControlRow17::Category { .. } => None,
        })
        .max()
        .unwrap_or(0)
}

/// `width / 2` in integer GUI pixels, as the era's `Screen` fields.
fn half(width: i32) -> i32 {
    width / 2
}

/// `VideoOptionsScreen`'s list: `OptionListWidget` over y 32 to `height - 32`
/// in 25px rows, its scrollbar 32 pixels right of the era list's own.
fn video_list(view: &MenuView17<'_>) -> List17 {
    let content = VideoOption17::ROWS.len() as i32 * 25;
    let list = List17 {
        x: 0,
        y: 32,
        w: view.width,
        h: (view.height - 32 - 32).max(0),
        content,
        scrollbar_x: half(view.width) + 124 + 32,
        scroll: view.scroll.max(0),
    };
    List17 {
        scroll: list.scroll.min(list.max_scroll()),
        ..list
    }
}

/// The pause menu (`GameMenuScreen.init`), at a GUI-scaled size.
pub fn pause_layout(view: &MenuView17<'_>) -> MenuLayout17 {
    let (w, h) = (view.width, view.height);
    let (x0, y0, right) = (half(w) - 100, h / 4, half(w) + 2);
    let mut widgets = Vec::with_capacity(5);
    // id 4: Back to Game.
    if let Some(label) = text("menu.returnToGame") {
        widgets.push(Widget17::button(
            x0,
            y0 + 8,
            200,
            Action17::BackToGame,
            label,
            true,
        ));
    }
    // id 6: Statistics, in the era's right slot. The left slot is id 5,
    // Achievements: the only achievements screen this port has is 26.3's,
    // which draws 26.3's keys on an era root, so the button stays out.
    if let Some(label) = text("gui.stats") {
        widgets.push(Widget17::button(
            right,
            y0 + 32,
            98,
            Action17::Statistics,
            label,
            true,
        ));
    }
    // id 0: Options..., in the era's left slot. The right slot is Share to
    // LAN, which this port has no OpenToLanScreen for.
    if let Some(label) = text("menu.options") {
        widgets.push(Widget17::button(
            x0,
            y0 + 80,
            98,
            Action17::OpenOptions,
            label,
            true,
        ));
    }
    // id 1: Save and Quit to Title, or Disconnect from a server.
    let quit = if view.remote {
        "menu.disconnect"
    } else {
        "menu.returnToMenu"
    };
    if let Some(label) = text(quit) {
        widgets.push(Widget17::button(
            x0,
            y0 + 104,
            200,
            Action17::QuitWorld,
            label,
            true,
        ));
    }
    MenuLayout17 {
        menu: Menu17::Pause,
        title: text("menu.game").map(|label| Label17::centered(label, half(w), 40, TITLE_TEXT)),
        labels: Vec::new(),
        widgets,
        list: None,
        focus: view.focus,
    }
}

/// The Options screen: the era's two-column 150px grid with its `gui.done`.
/// `Super Secret Settings`, Broadcast Settings, Multiplayer Settings,
/// Resource Packs and Snooper Settings have no ported screen in this build,
/// so their slots stay empty rather than opening the wrong thing.
pub fn options_layout(view: &MenuView17<'_>) -> MenuLayout17 {
    let (w, h) = (view.width, view.height);
    let (left, right) = (half(w) - 155, half(w) + 5);
    let mut widgets = Vec::with_capacity(5);
    let mut add = |y: i32, x: i32, action: Action17, key: &str| {
        if let Some(label) = text(key) {
            widgets.push(Widget17::button(x, y, 150, action, label, true));
        }
    };
    add(h / 6 + 66, left, Action17::OpenSounds, "options.sounds");
    add(h / 6 + 90, left, Action17::OpenVideo, "options.video");
    add(
        h / 6 + 90,
        right,
        Action17::OpenControls,
        "options.controls",
    );
    add(
        h / 6 + 114,
        left,
        Action17::OpenLanguage,
        "options.language",
    );
    if let Some(label) = text("gui.done") {
        widgets.push(Widget17::button(
            half(w) - 100,
            h / 6 + 168,
            200,
            Action17::Done,
            label,
            true,
        ));
    }
    MenuLayout17 {
        menu: Menu17::Options,
        title: text("options.title").map(|label| Label17::centered(label, half(w), 15, TITLE_TEXT)),
        labels: Vec::new(),
        widgets,
        list: None,
        focus: view.focus,
    }
}

/// Video Settings (`VideoOptionsScreen.init` + `OptionListWidget`): the
/// option pairs from y 32 in a 25px-row list, with `gui.done` in the footer.
pub fn video_layout(settings: &Settings, view: &MenuView17<'_>) -> MenuLayout17 {
    let (w, h) = (view.width, view.height);
    let list = video_list(view);
    let top = list.y + 4 - list.scroll;
    let mut widgets = Vec::with_capacity(13);
    for (row, pair) in VideoOption17::ROWS.iter().enumerate() {
        let y = top + row as i32 * 25;
        for (column, option) in pair.iter().enumerate() {
            let Some(option) = option else { continue };
            let x = if column == 0 {
                half(w) - 155
            } else {
                half(w) + 5
            };
            let Some(label) = option.label(settings) else {
                continue;
            };
            let widget = if option.is_slider() {
                Widget17::slider(
                    x,
                    y,
                    150,
                    Action17::Video(*option),
                    label,
                    option.position(settings).unwrap_or(0.0),
                )
            } else {
                Widget17::button(x, y, 150, Action17::Video(*option), label, true)
            };
            widgets.push(widget.listed());
        }
    }
    if let Some(label) = text("gui.done") {
        widgets.push(Widget17::button(
            half(w) - 100,
            h - 27,
            200,
            Action17::Done,
            label,
            true,
        ));
    }
    MenuLayout17 {
        menu: Menu17::Video,
        title: text("options.videoTitle")
            .map(|label| Label17::centered(label, half(w), 5, TITLE_TEXT)),
        labels: Vec::new(),
        widgets,
        list: Some(list),
        focus: view.focus,
    }
}

/// A sound category's slider message (`SoundsScreen.getVolume`):
/// `soundCategory.<name>: <n>%`, or `options.off`.
fn volume_label(source: usize, volume: f64) -> Option<String> {
    let caption = text(&format!("soundCategory.{}", SOUND_SOURCES[source]))?;
    let value = if volume == 0.0 {
        text("options.off")?
    } else {
        format!("{}%", (volume * 100.0) as i32)
    };
    Some(format!("{caption}: {value}"))
}

/// Music & Sounds (`SoundsScreen.init`): the master slider across the screen,
/// then the categories two to a row from `height / 6 - 12`.
pub fn sounds_layout(settings: &Settings, view: &MenuView17<'_>) -> MenuLayout17 {
    let (w, h) = (view.width, view.height);
    let (left, right) = (half(w) - 155, half(w) + 5);
    let mut widgets = Vec::with_capacity(10);
    if let Some(label) = volume_label(0, settings.master_volume) {
        widgets.push(Widget17::slider(
            left,
            h / 6 - 12,
            310,
            Action17::Sound(0),
            label,
            settings.master_volume as f32,
        ));
    }
    // SoundCategory.MUSIC is var1 = 2: the row after master, left column.
    for source in 1usize..9 {
        let slot = source + 1;
        let x = if slot % 2 == 0 { left } else { right };
        let y = h / 6 - 12 + 24 * (slot / 2) as i32;
        if let Some(label) = volume_label(source, settings.volumes[source]) {
            widgets.push(Widget17::slider(
                x,
                y,
                150,
                Action17::Sound(source),
                label,
                settings.volumes[source] as f32,
            ));
        }
    }
    if let Some(label) = text("gui.done") {
        widgets.push(Widget17::button(
            half(w) - 100,
            h / 6 + 168,
            200,
            Action17::Done,
            label,
            true,
        ));
    }
    MenuLayout17 {
        menu: Menu17::Sounds,
        title: text("options.sounds.title")
            .map(|label| Label17::centered(label, half(w), 15, TITLE_TEXT)),
        labels: Vec::new(),
        widgets,
        list: None,
        focus: view.focus,
    }
}

/// Controls (`ControlsOptionsScreen.init` and `ControlsListWidget`): the
/// mouse option and sensitivity over the scrolling binding list, `gui.done`
/// and `controls.resetAll` in the footer.
pub fn controls_layout(
    settings: &Settings,
    view: &MenuView17<'_>,
    text_width: &dyn Fn(&str) -> i32,
) -> MenuLayout17 {
    let (w, h) = (view.width, view.height);
    let rows = control_rows();
    let list = List17 {
        x: 0,
        y: 63,
        w,
        h: (h - 32 - 63).max(0),
        content: rows.len() as i32 * 20,
        scrollbar_x: half(w) + 124 + 15,
        scroll: view.scroll.max(0),
    };
    let list = List17 {
        scroll: list.scroll.min(list.max_scroll()),
        ..list
    };
    let row_x = half(w) - 126 + 2;
    let top = list.y + 4 - list.scroll;
    let max_label = max_control_label_width(&rows, text_width);
    let mut widgets = Vec::with_capacity(6 + rows.len() * 2);
    let mut labels = Vec::new();
    // ControlsOptionsScreen's own OPTIONS: Invert Mouse and Sensitivity.
    if let Some(label) = caption_value("options.invertMouse", on_off(settings.invert_y)) {
        widgets.push(Widget17::button(
            half(w) - 155,
            18,
            150,
            Action17::InvertMouse,
            label,
            true,
        ));
    }
    let sensitivity = match (settings.sensitivity * 200.0) as i32 {
        0 => text("options.sensitivity.min"),
        200 => text("options.sensitivity.max"),
        n => Some(format!("{n}%")),
    };
    if let Some(label) = caption_value("options.sensitivity", sensitivity) {
        widgets.push(Widget17::slider(
            half(w) + 5,
            18,
            150,
            Action17::Sensitivity,
            label,
            settings.sensitivity as f32,
        ));
    }
    for (index, row) in rows.iter().enumerate() {
        let y = top + index as i32 * 20;
        match row {
            ControlRow17::Category { name } => labels.push(Label17 {
                text: name.clone(),
                x: half(w),
                y: y + 16 - FONT_HEIGHT - 1,
                color: TITLE_TEXT,
                centered: true,
                shadow: false,
                in_list: true,
            }),
            ControlRow17::Binding { name, label } => {
                let key = key_name(settings.keys.get(name));
                let shown = if view.binding == Some(*name) {
                    format!("§f> §e{key}§f <")
                } else if !settings.keys.conflicts(name).is_empty() {
                    format!("§c{key}")
                } else {
                    key
                };
                labels.push(Label17 {
                    text: label.clone(),
                    x: row_x + 90 - max_label,
                    y: y + 4,
                    color: TITLE_TEXT,
                    centered: false,
                    shadow: false,
                    in_list: true,
                });
                widgets.push(
                    Widget17::button(row_x + 105, y, 75, Action17::BindKey(name), shown, true)
                        .with_height(18)
                        .listed(),
                );
                if let Some(label) = text("controls.reset") {
                    widgets.push(
                        Widget17::button(
                            row_x + 190,
                            y,
                            50,
                            Action17::ResetKey(name),
                            label,
                            !settings.keys.is_default(name),
                        )
                        .with_height(18)
                        .listed(),
                    );
                }
            }
        }
    }
    if let Some(label) = text("gui.done") {
        widgets.push(Widget17::button(
            half(w) - 155,
            h - 29,
            150,
            Action17::Done,
            label,
            true,
        ));
    }
    if let Some(label) = text("controls.resetAll") {
        widgets.push(Widget17::button(
            half(w) - 155 + 160,
            h - 29,
            150,
            Action17::ResetAllKeys,
            label,
            settings.keys.changed_from_defaults(),
        ));
    }
    MenuLayout17 {
        menu: Menu17::Controls,
        title: text("controls.title").map(|label| Label17::centered(label, half(w), 8, TITLE_TEXT)),
        labels,
        widgets,
        list: Some(list),
        focus: view.focus,
    }
}

/// Language (`LanguageOptionsScreen.init`): the language list from y 32 in
/// 18px rows, Force Unicode Font and `gui.done` in the footer, the warning
/// text above them.
pub fn language_layout(view: &MenuView17<'_>, force_unicode: bool) -> MenuLayout17 {
    let (w, h) = (view.width, view.height);
    let list = List17 {
        x: 0,
        y: 32,
        w,
        h: (h - 65 + 4 - 32).max(0),
        content: view.languages.len() as i32 * 18,
        scrollbar_x: half(w) + 124,
        scroll: view.scroll.max(0),
    };
    let list = List17 {
        scroll: list.scroll.min(list.max_scroll()),
        ..list
    };
    let top = list.y + 4 - list.scroll;
    let mut widgets = Vec::with_capacity(view.languages.len() + 2);
    for (index, language) in view.languages.iter().enumerate() {
        // `Language.toString`: "Region (Name)".
        let label = format!("{} ({})", language.region, language.name);
        let y = top + index as i32 * 18;
        widgets.push(
            Widget17 {
                rect: [half(w) - 110, y, 220, 18],
                label,
                action: Action17::Language(index),
                kind: Kind17::Row {
                    selected: language.code == view.selected_language,
                },
                active: true,
                in_list: true,
            }
            .listed(),
        );
    }
    if let Some(label) = caption_value("options.forceUnicodeFont", on_off(force_unicode)) {
        widgets.push(Widget17::button(
            half(w) - 155,
            h - 38,
            150,
            Action17::ForceUnicode,
            label,
            true,
        ));
    }
    if let Some(label) = text("gui.done") {
        widgets.push(Widget17::button(
            half(w) + 5,
            h - 38,
            150,
            Action17::Done,
            label,
            true,
        ));
    }
    let mut labels = Vec::new();
    if let Some(warning) = text("options.languageWarning") {
        labels.push(Label17::centered(
            format!("({warning})"),
            half(w),
            h - 56,
            WARNING_TEXT,
        ));
    }
    MenuLayout17 {
        menu: Menu17::Language,
        title: text("options.language")
            .map(|label| Label17::centered(label, half(w), 16, TITLE_TEXT)),
        labels,
        widgets,
        list: Some(list),
        focus: view.focus,
    }
}

/// The layout of one screen.
pub fn layout(
    menu: Menu17,
    settings: &Settings,
    view: &MenuView17<'_>,
    text_width: &dyn Fn(&str) -> i32,
) -> MenuLayout17 {
    match menu {
        Menu17::Pause => pause_layout(view),
        Menu17::Options => options_layout(view),
        Menu17::Video => video_layout(settings, view),
        Menu17::Sounds => sounds_layout(settings, view),
        Menu17::Controls => controls_layout(settings, view, text_width),
        Menu17::Language => language_layout(view, settings.force_unicode),
    }
}

/// The title's key, for tests and the screen's narration.
pub fn title_key(menu: Menu17) -> &'static str {
    menu.title_key()
}

/// `Screen.renderBackground`: the in-world dim, or the era's tiled dirt.
pub fn paint_background(batch: &mut HudBatch, atlas: &HudAtlas, in_world: bool) {
    let (w, h) = (batch.width, batch.height);
    if in_world {
        batch.fill_gradient(atlas, (0.0, 0.0, w, h), DIM_TOP, DIM_BOTTOM);
    } else {
        tiled(batch, atlas, OPTIONS_BACKGROUND, 0.0, 0.0, w, h, MENU_TINT);
    }
}

/// `drawTexture` at 32 GUI pixels per texture repeat: the era's dirt
/// background, tinted.
fn tiled(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    name: &str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: u32,
) {
    let Some(source) = atlas.blit_rect(name).copied() else {
        return;
    };
    let mut ty = y;
    while ty < y + h {
        let mut tx = x;
        while tx < x + w {
            let tw = (x + w - tx).min(32.0);
            let th = (y + h - ty).min(32.0);
            let rect = Rect {
                u0: source.u0,
                v0: source.v0,
                u1: source.u0 + (source.u1 - source.u0) * tw / 32.0,
                v1: source.v0 + (source.v1 - source.v0) * th / 32.0,
                w: tw as u32,
                h: th as u32,
            };
            batch.rect_uv(tx, ty, tw, th, rect, color);
            tx += 32.0;
        }
        ty += 32.0;
    }
}

/// `ButtonWidget.render`'s two halves of `widgets.png` at row `v`.
fn paint_button_shell(batch: &mut HudBatch, atlas: &HudAtlas, [x, y, w, h]: [i32; 4], v: i32) {
    // The left half from u 0, the right from 200 - w/2.
    let left = Icon::at(WIDGETS, 0, v, w / 2, h);
    let right = Icon::at(WIDGETS, 200 - w / 2, v, w / 2, h);
    for (icon, dx) in [(left, 0), (right, w / 2)] {
        for part in icon.atlas_parts(atlas) {
            batch.rect_uv(
                (x + dx + part.dx) as f32,
                (y + part.dy) as f32,
                part.w as f32,
                part.h as f32,
                part.rect,
                u32::MAX,
            );
        }
    }
}

/// One draw of a widget's background and label (`ButtonWidget.render`).
fn paint_button(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    [x, y, w, h]: [i32; 4],
    label: &str,
    active: bool,
    hover: bool,
) {
    let v = if !active {
        BUTTON_INACTIVE_V
    } else if hover {
        BUTTON_HOVER_V
    } else {
        BUTTON_NORMAL_V
    };
    paint_button_shell(batch, atlas, [x, y, w, h], v);
    let color = if !active {
        BUTTON_INACTIVE_TEXT
    } else if hover {
        BUTTON_HOVER_TEXT
    } else {
        BUTTON_TEXT
    };
    draw_centered(batch, atlas, label, x + w / 2, y + (h - 8) / 2, color);
}

/// `OptionSliderWidget`: the button's inactive background (`getYImage` is 0),
/// then its 8px handle at `value` along the track.
fn paint_slider(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    [x, y, w, h]: [i32; 4],
    label: &str,
    value: f32,
    hover: bool,
) {
    paint_button_shell(batch, atlas, [x, y, w, h], BUTTON_INACTIVE_V);
    let handle_x = x + (value.clamp(0.0, 1.0) * (w - 8) as f32) as i32;
    for (icon, dx) in [
        (Icon::at(WIDGETS, 0, SLIDER_HANDLE_V, 4, h), 0),
        (Icon::at(WIDGETS, 196, SLIDER_HANDLE_V, 4, h), 4),
    ] {
        for part in icon.atlas_parts(atlas) {
            batch.rect_uv(
                (handle_x + dx + part.dx) as f32,
                (y + part.dy) as f32,
                part.w as f32,
                part.h as f32,
                part.rect,
                u32::MAX,
            );
        }
    }
    draw_centered(
        batch,
        atlas,
        label,
        x + w / 2,
        y + (h - 8) / 2,
        if hover {
            BUTTON_HOVER_TEXT
        } else {
            BUTTON_TEXT
        },
    );
}

/// `ListWidget.renderList`'s selection highlight around a language row and
/// `LanguageOptionsScreen`'s centred `toString` at its top.
fn paint_row(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    [x, y, w, h]: [i32; 4],
    label: &str,
    selected: bool,
    _hover: bool,
) {
    // `var9` is `entryHeight - 4`: the outer band from y - 2 to y + var9 + 2,
    // the black field one pixel inside it.
    if selected {
        batch.fill(
            atlas,
            x as f32,
            (y - 2) as f32,
            w as f32,
            h as f32,
            rgba(0x80, 0x80, 0x80, 0xff),
        );
        batch.fill(
            atlas,
            (x + 1) as f32,
            (y - 1) as f32,
            (w - 2) as f32,
            (h - 2) as f32,
            rgba(0, 0, 0, 0xff),
        );
    }
    draw_centered(batch, atlas, label, x + w / 2, y + 1, TITLE_TEXT);
}

/// `drawCenteredString`: half the text's width back from `center_x`, with a
/// shadow.
fn draw_centered(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    text: &str,
    center_x: i32,
    y: i32,
    color: u32,
) {
    assert_translated(text);
    if text.is_empty() {
        return;
    }
    let x = center_x - atlas.font.width(text) as i32 / 2;
    batch.text(atlas, text, x as f32, y as f32, color, true);
}

/// One `Label17`.
fn paint_label(batch: &mut HudBatch, atlas: &HudAtlas, label: &Label17) {
    assert_translated(&label.text);
    let x = if label.centered {
        label.x - atlas.font.width(&label.text) as i32 / 2
    } else {
        label.x
    };
    batch.text(
        atlas,
        &label.text,
        x as f32,
        label.y as f32,
        label.color,
        label.shadow,
    );
}

/// The prefixes that mean a string is an unresolved translation key, not
/// text a person reads.
const KEY_PREFIXES: [&str; 12] = [
    "gui.",
    "menu.",
    "options.",
    "key.",
    "controls.",
    "soundCategory.",
    "container.",
    "deathScreen.",
    "stat.",
    "multiplayer.",
    "advancements.",
    "sunrise.",
];

/// A visible label that starts with one of [`KEY_PREFIXES`] and that the
/// loaded language does not translate: the era screens must never draw one.
pub fn is_untranslated_key(label: &str) -> bool {
    KEY_PREFIXES.iter().any(|prefix| label.starts_with(prefix)) && !lang::has(label)
}

/// Debug builds refuse to draw an unresolved key on an era screen.
#[track_caller]
pub fn assert_translated(label: &str) {
    debug_assert!(
        !is_untranslated_key(label),
        "era screen drew an untranslated key: {label}"
    );
}

/// The scrollbar `ListWidget` draws over its list: a fading black track and
/// the light thumb.
fn paint_scrollbar(batch: &mut HudBatch, atlas: &HudAtlas, list: List17) {
    let max = list.max_scroll();
    if max <= 0 {
        return;
    }
    let x = list.scrollbar_x as f32;
    batch.fill(
        atlas,
        x,
        list.y as f32,
        6.0,
        list.h as f32,
        rgba(0, 0, 0, 255),
    );
    let area = list.h;
    let thumb = (area * area / list.content.max(1)).clamp(32, (area - 8).max(1));
    let y = list.scroll * (area - thumb) / max + list.y;
    batch.fill(
        atlas,
        x,
        y as f32,
        6.0,
        thumb as f32,
        rgba(0x80, 0x80, 0x80, 255),
    );
    batch.fill(
        atlas,
        x,
        y as f32,
        5.0,
        (thumb - 1).max(1) as f32,
        rgba(0xC0, 0xC0, 0xC0, 255),
    );
}

/// Draw a menu's list background, labels, widgets, title and scrollbar.
/// `cursor` is the pointer in GUI pixels, for the widgets' hover state.
pub fn paint(batch: &mut HudBatch, atlas: &HudAtlas, layout: &MenuLayout17, cursor: (f32, f32)) {
    if let Some(list) = layout.list {
        // ListWidget.renderHoleBackground: the tiled dirt above and below the
        // list, at the screen background's tint.
        for (y, h) in [
            (0, list.y),
            (list.y + list.h, batch.height as i32 - (list.y + list.h)),
        ] {
            if h > 0 {
                tiled(
                    batch,
                    atlas,
                    OPTIONS_BACKGROUND,
                    0.0,
                    y as f32,
                    batch.width,
                    h as f32,
                    MENU_TINT,
                );
            }
        }
        tiled(
            batch,
            atlas,
            OPTIONS_BACKGROUND,
            list.x as f32,
            list.y as f32,
            list.w as f32,
            list.h as f32,
            LIST_TINT,
        );
        // ListWidget's 4px dark gradients at the list's edges.
        batch.fill_gradient(
            atlas,
            (list.x as f32, list.y as f32, list.w as f32, 4.0),
            rgba(0, 0, 0, 255),
            rgba(0, 0, 0, 0),
        );
        batch.fill_gradient(
            atlas,
            (
                list.x as f32,
                (list.y + list.h - 4) as f32,
                list.w as f32,
                4.0,
            ),
            rgba(0, 0, 0, 0),
            rgba(0, 0, 0, 255),
        );
    }
    let list = layout.list;
    for label in &layout.labels {
        // A list's rows scroll with it: a row out of sight is neither drawn
        // nor clicks, and one halfway out is cut at the list's edge.
        if label.in_list
            && let Some(list) = list
            && (label.y + FONT_HEIGHT <= list.y || label.y >= list.y + list.h)
        {
            continue;
        }
        let start = batch.vertices.len();
        paint_label(batch, atlas, label);
        if label.in_list
            && let Some(list) = list
        {
            batch.clip_since(
                start,
                list.x as f32,
                list.y as f32,
                (list.x + list.w) as f32,
                (list.y + list.h) as f32,
            );
        }
    }
    for widget in &layout.widgets {
        let [x, y, w, h] = widget.rect;
        let visible = !widget.in_list || list.is_some_and(|l| y + h > l.y && y < l.y + l.h);
        if !visible {
            continue;
        }
        let focused = layout.focus == Some(widget.action);
        let hover = focused || widget.contains(cursor.0, cursor.1);
        let start = batch.vertices.len();
        let kind = match widget.kind {
            Kind17::Slider { value, .. } => {
                paint_slider(batch, atlas, widget.rect, &widget.label, value, hover);
                crate::widgets::Kind::Slider {
                    value,
                    track: [x as f32 + 4.0, w as f32 - 8.0],
                }
            }
            Kind17::Button => {
                paint_button(
                    batch,
                    atlas,
                    widget.rect,
                    &widget.label,
                    widget.active,
                    hover,
                );
                crate::widgets::Kind::Button
            }
            Kind17::Row { selected } => {
                paint_row(batch, atlas, widget.rect, &widget.label, selected, hover);
                crate::widgets::Kind::Row
            }
        };
        if let Some(list) = list
            && widget.in_list
        {
            batch.clip_since(
                start,
                list.x as f32,
                list.y as f32,
                (list.x + list.w) as f32,
                (list.y + list.h) as f32,
            );
        }
        // The control socket's widget tree (`sunrise-ctl ui`): its label,
        // bounds and, for a slider, its value.
        widgets::record_as(
            &widget.label,
            [x as f32, y as f32, w as f32, h as f32],
            widget.active,
            kind,
        );
        widgets::target_as([x as f32, y as f32, w as f32, h as f32], kind);
        widgets::narratable(|| {
            let narration = match kind {
                crate::widgets::Kind::Slider { .. } => Narratable::slider(
                    widget.label.clone(),
                    [x as f32, y as f32, w as f32, h as f32],
                    widget.active,
                ),
                crate::widgets::Kind::Row => Narratable::text(
                    widget.label.clone(),
                    [x as f32, y as f32, w as f32, h as f32],
                ),
                _ => Narratable::button(
                    widget.label.clone(),
                    [x as f32, y as f32, w as f32, h as f32],
                    widget.active,
                ),
            };
            narration.hovered(hover)
        });
    }
    if let Some(title) = &layout.title {
        paint_label(batch, atlas, title);
    }
    if let Some(list) = layout.list {
        paint_scrollbar(batch, atlas, list);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn language(entries: &[(&str, &str)]) -> lang::Language {
        lang::Language::of(
            "en_us",
            entries
                .iter()
                .map(|(key, value)| ((*key).to_owned(), (*value).to_owned())),
        )
    }

    fn en() -> lang::Language {
        language(&[
            ("menu.game", "Game menu"),
            ("menu.returnToGame", "Back to Game"),
            ("menu.returnToMenu", "Save and Quit to Title"),
            ("menu.disconnect", "Disconnect"),
            ("menu.options", "Options..."),
            ("gui.achievements", "Achievements"),
            ("gui.stats", "Statistics"),
            ("gui.done", "Done"),
            ("options.title", "Options"),
            ("options.sounds", "Music & Sounds..."),
            ("options.sounds.title", "Music & Sound Options"),
            ("options.video", "Video Settings..."),
            ("options.videoTitle", "Video Settings"),
            ("options.controls", "Controls..."),
            ("controls.title", "Controls"),
            ("options.language", "Language..."),
            (
                "options.languageWarning",
                "Language translations may not be 100%% accurate",
            ),
            ("options.forceUnicodeFont", "Force Unicode Font"),
            ("options.on", "ON"),
            ("options.off", "OFF"),
            ("options.invertMouse", "Invert Mouse"),
            ("options.sensitivity", "Sensitivity"),
            ("options.sensitivity.min", "*yawn*"),
            ("options.sensitivity.max", "HYPERSPEED!!!"),
            ("options.graphics", "Graphics"),
            ("options.graphics.fancy", "Fancy"),
            ("options.graphics.fast", "Fast"),
            ("options.renderDistance", "Render Distance"),
            ("options.ao", "Smooth Lighting"),
            ("options.ao.max", "Maximum"),
            ("options.ao.off", "OFF"),
            ("options.framerateLimit", "Max Framerate"),
            ("options.framerateLimit.max", "Unlimited"),
            ("options.viewBobbing", "View Bobbing"),
            ("options.guiScale", "GUI Scale"),
            ("options.guiScale.auto", "Auto"),
            ("options.guiScale.small", "Small"),
            ("options.guiScale.normal", "Normal"),
            ("options.guiScale.large", "Large"),
            ("options.gamma", "Brightness"),
            ("options.gamma.min", "Moody"),
            ("options.gamma.max", "Bright"),
            ("options.renderClouds", "Clouds"),
            ("options.particles", "Particles"),
            ("options.particles.all", "All"),
            ("options.particles.decreased", "Decreased"),
            ("options.particles.minimal", "Minimal"),
            ("options.fullscreen", "Fullscreen"),
            ("options.vsync", "VSync"),
            ("options.mipmapLevels", "Mipmap Levels"),
            ("controls.reset", "Reset"),
            ("controls.resetAll", "Reset Keys"),
            ("soundCategory.master", "Master Volume"),
            ("soundCategory.music", "Music"),
            ("soundCategory.record", "Jukebox/Noteblocks"),
            ("soundCategory.weather", "Weather"),
            ("soundCategory.block", "Blocks"),
            ("soundCategory.hostile", "Hostile Creatures"),
            ("soundCategory.neutral", "Friendly Creatures"),
            ("soundCategory.player", "Players"),
            ("soundCategory.ambient", "Ambient/Environment"),
            ("key.categories.gameplay", "Gameplay"),
            ("key.categories.movement", "Movement"),
            ("key.categories.inventory", "Inventory"),
            ("key.categories.multiplayer", "Multiplayer"),
            ("key.categories.misc", "Miscellaneous"),
            ("key.attack", "Attack/Destroy"),
            ("key.use", "Use Item/Place Block"),
            ("key.forward", "Walk Forwards"),
            ("key.left", "Strafe Left"),
            ("key.back", "Walk Backwards"),
            ("key.right", "Strafe Right"),
            ("key.jump", "Jump"),
            ("key.sneak", "Sneak"),
            ("key.drop", "Drop Item"),
            ("key.inventory", "Inventory"),
            ("key.chat", "Open Chat"),
            ("key.playerlist", "List Players"),
            ("key.pickItem", "Pick Block"),
            ("key.command", "Open Command"),
            ("key.screenshot", "Take Screenshot"),
            ("key.togglePerspective", "Toggle Perspective"),
            ("key.smoothCamera", "Toggle Cinematic Camera"),
            ("key.sprint", "Sprint"),
            ("key.hotbar.1", "Hotbar Slot 1"),
            ("key.hotbar.2", "Hotbar Slot 2"),
            ("key.hotbar.3", "Hotbar Slot 3"),
            ("key.hotbar.4", "Hotbar Slot 4"),
            ("key.hotbar.5", "Hotbar Slot 5"),
            ("key.hotbar.6", "Hotbar Slot 6"),
            ("key.hotbar.7", "Hotbar Slot 7"),
            ("key.hotbar.8", "Hotbar Slot 8"),
            ("key.hotbar.9", "Hotbar Slot 9"),
            ("key.fullscreen", "Toggle Fullscreen"),
            ("key.mouseButton", "Button %1$s"),
        ])
    }

    fn with_en<R>(f: impl FnOnce() -> R) -> R {
        lang::scoped(en(), f).0
    }

    #[test]
    fn the_pause_menu_is_game_menu_screens_layout() {
        with_en(|| {
            let view = MenuView17::new(427, 240, 4);
            let layout = pause_layout(&view);
            let rect = |action| {
                layout
                    .widgets
                    .iter()
                    .find(|w| w.action == action)
                    .map(|w| w.rect)
            };
            // GameMenuScreen.init's `var1 = -16`, in integer GUI pixels.
            assert_eq!(rect(Action17::BackToGame), Some([113, 68, 200, 20]));
            assert_eq!(rect(Action17::Statistics), Some([215, 92, 98, 20]));
            assert_eq!(rect(Action17::OpenOptions), Some([113, 140, 98, 20]));
            assert_eq!(rect(Action17::QuitWorld), Some([113, 164, 200, 20]));
            let title = layout.title.as_ref().unwrap();
            assert_eq!((title.text.as_str(), title.y), ("Game menu", 40));
            let labels: Vec<&str> = layout
                .widgets
                .iter()
                .map(|widget| widget.label.as_str())
                .collect();
            assert_eq!(
                labels,
                [
                    "Back to Game",
                    "Statistics",
                    "Options...",
                    "Save and Quit to Title"
                ]
            );
        });
    }

    #[test]
    fn a_server_pause_menu_disconnects_instead_of_saving() {
        with_en(|| {
            let mut view = MenuView17::new(427, 240, 4);
            view.remote = true;
            let layout = pause_layout(&view);
            let quit = layout
                .widgets
                .iter()
                .find(|w| w.action == Action17::QuitWorld)
                .unwrap();
            assert_eq!(quit.label, "Disconnect");
        });
    }

    #[test]
    fn options_screen_is_the_eras_two_column_grid() {
        with_en(|| {
            let view = MenuView17::new(427, 240, 4);
            let layout = options_layout(&view);
            let rect = |action| {
                layout
                    .widgets
                    .iter()
                    .find(|w| w.action == action)
                    .map(|w| w.rect)
            };
            assert_eq!(rect(Action17::OpenSounds), Some([58, 106, 150, 20]));
            assert_eq!(rect(Action17::OpenVideo), Some([58, 130, 150, 20]));
            assert_eq!(rect(Action17::OpenControls), Some([218, 130, 150, 20]));
            assert_eq!(rect(Action17::OpenLanguage), Some([58, 154, 150, 20]));
            assert_eq!(rect(Action17::Done), Some([113, 208, 200, 20]));
            assert_eq!(layout.title.as_ref().unwrap().text, "Options");
        });
    }

    #[test]
    fn video_settings_pairs_its_options_in_eras_order() {
        with_en(|| {
            let view = MenuView17::new(427, 240, 4);
            let layout = video_layout(&Settings::default(), &view);
            let widgets: Vec<(Action17, [i32; 4])> = layout
                .widgets
                .iter()
                .filter(|w| matches!(w.action, Action17::Video(_)))
                .map(|w| (w.action, w.rect))
                .collect();
            assert_eq!(
                widgets[0],
                (Action17::Video(VideoOption17::Graphics), [58, 36, 150, 20])
            );
            assert_eq!(
                widgets[1],
                (
                    Action17::Video(VideoOption17::RenderDistance),
                    [218, 36, 150, 20]
                )
            );
            // OptionListWidget's 25px rows: left column then right.
            assert_eq!(widgets[2].1, [58, 61, 150, 20]);
            assert_eq!(widgets[3].1, [218, 61, 150, 20]);
            assert_eq!(
                widgets.last().copied(),
                Some((Action17::Video(VideoOption17::Mipmaps), [218, 161, 150, 20]))
            );
            // The era's value text, from getAsString.
            let label = |option: VideoOption17| {
                &layout
                    .widgets
                    .iter()
                    .find(|w| w.action == Action17::Video(option))
                    .unwrap()
                    .label
            };
            assert_eq!(label(VideoOption17::Graphics), "Graphics: Fancy");
            assert_eq!(
                label(VideoOption17::RenderDistance),
                "Render Distance: 12 chunks"
            );
            assert_eq!(
                label(VideoOption17::SmoothLighting),
                "Smooth Lighting: Maximum"
            );
            assert_eq!(label(VideoOption17::MaxFramerate), "Max Framerate: 120 fps");
            assert_eq!(label(VideoOption17::Brightness), "Brightness: +50%");
            assert_eq!(layout.list.unwrap().scrollbar_x, half(427) + 156);
        });
    }

    #[test]
    fn a_video_slider_takes_the_value_the_track_stands_for() {
        let mut settings = Settings::default();
        VideoOption17::RenderDistance.slide(&mut settings, 1.0);
        assert_eq!(settings.render_distance, ERA_RENDER_DISTANCE);
        VideoOption17::MaxFramerate.slide(&mut settings, 0.0);
        assert_eq!(settings.max_fps, 10);
        VideoOption17::MaxFramerate.slide(&mut settings, 1.0);
        assert_eq!(settings.max_fps, 260);
        VideoOption17::Mipmaps.slide(&mut settings, 0.5);
        assert_eq!(settings.mipmap_levels, 2);
        VideoOption17::Brightness.slide(&mut settings, 0.25);
        assert!((settings.gamma - 0.25).abs() < 1e-6);
        assert_eq!(VideoOption17::Brightness.position(&settings), Some(0.25));
    }

    #[test]
    fn video_cycle_buttons_step_the_eras_values() {
        let mut settings = Settings {
            clouds: "true".into(),
            ..Settings::default()
        };
        VideoOption17::Clouds.cycle(&mut settings, 4);
        assert_eq!(settings.clouds, "false");
        VideoOption17::Clouds.cycle(&mut settings, 4);
        assert_eq!(settings.clouds, "true");
        settings.particles = 2;
        VideoOption17::Particles.cycle(&mut settings, 4);
        assert_eq!(settings.particles, 0);
        settings.gui_scale = 3;
        VideoOption17::GuiScale.cycle(&mut settings, 3);
        assert_eq!(settings.gui_scale, 0, "the era wraps to Auto");
        settings.graphics_preset = "fancy".into();
        VideoOption17::Graphics.cycle(&mut settings, 4);
        assert_eq!(settings.graphics_preset, "fast");
    }

    #[test]
    fn sounds_puts_the_master_slider_over_the_categories() {
        with_en(|| {
            let view = MenuView17::new(427, 240, 4);
            let layout = sounds_layout(&Settings::default(), &view);
            let at = |action| {
                layout
                    .widgets
                    .iter()
                    .find(|w| w.action == action)
                    .map(|w| w.rect)
            };
            // SoundsScreen.init: master is 310 wide, then var1 starts at 2.
            assert_eq!(at(Action17::Sound(0)), Some([58, 28, 310, 20]));
            assert_eq!(at(Action17::Sound(1)), Some([58, 52, 150, 20]));
            assert_eq!(at(Action17::Sound(2)), Some([218, 52, 150, 20]));
            assert_eq!(at(Action17::Sound(8)), Some([218, 124, 150, 20]));
            let music = layout
                .widgets
                .iter()
                .find(|w| w.action == Action17::Sound(1))
                .unwrap();
            assert_eq!(music.label, "Music: 100%");
        });
    }

    #[test]
    fn controls_lists_eras_bindings_with_their_category_headers() {
        with_en(|| {
            let view = MenuView17::new(427, 240, 4);
            let layout = controls_layout(&Settings::default(), &view, &|t| t.len() as i32 * 6);
            let first = layout
                .widgets
                .iter()
                .find(|w| matches!(w.action, Action17::BindKey(_)))
                .unwrap();
            // ControlsListWidget: the row starts at width / 2 - 124, and its
            // key button 105 pixels in, 75 by 18 (`KeyBindingEntry`).
            // The Gameplay header takes the list's first row, so Attack is
            // the second: ControlsListWidget's 20px rows from y 63 + 4.
            assert_eq!(first.rect, [194, 87, 75, 18]);
            assert_eq!(
                first.action,
                Action17::BindKey("key.attack"),
                "Gameplay sorts before the other categories"
            );
            assert!(
                layout
                    .labels
                    .iter()
                    .any(|l| l.text == "Gameplay" && l.centered),
                "the category header is centred"
            );
            assert_eq!(layout.list.unwrap().scrollbar_x, half(427) + 124 + 15);
            // ControlsOptionsScreen's OPTIONS: Invert Mouse and Sensitivity.
            assert!(
                layout
                    .widgets
                    .iter()
                    .any(|w| w.action == Action17::InvertMouse && w.label == "Invert Mouse: OFF")
            );
            assert!(
                layout
                    .widgets
                    .iter()
                    .any(|w| w.action == Action17::Sensitivity && w.label == "Sensitivity: 100%")
            );
        });
    }

    #[test]
    fn controls_shows_the_key_its_binding_holds_and_waits_with_brackets() {
        with_en(|| {
            let mut settings = Settings::default();
            settings.keys.set("key.jump", "key.keyboard.z");
            let mut view = MenuView17::new(427, 240, 4);
            view.binding = Some("key.jump");
            let layout = controls_layout(&settings, &view, &|t| t.len() as i32 * 6);
            let jump = layout
                .widgets
                .iter()
                .find(|w| w.action == Action17::BindKey("key.jump"))
                .unwrap();
            assert_eq!(jump.label, "§f> §eZ§f <");
            let reset = layout
                .widgets
                .iter()
                .find(|w| w.action == Action17::ResetKey("key.jump"))
                .unwrap();
            assert!(reset.active, "a moved binding can be reset");
        });
    }

    #[test]
    fn language_rows_are_the_eras_region_name_pairs() {
        with_en(|| {
            let languages = [LanguageInfo {
                code: "en_us".into(),
                name: "English".into(),
                region: "US".into(),
                bidirectional: false,
            }];
            let mut view = MenuView17::new(427, 240, 4);
            view.languages = &languages;
            let layout = language_layout(&view, false);
            let row = layout
                .widgets
                .iter()
                .find(|w| matches!(w.action, Action17::Language(0)))
                .unwrap();
            assert_eq!(row.label, "US (English)");
            assert_eq!(row.rect, [103, 36, 220, 18]);
            assert!(matches!(row.kind, Kind17::Row { selected: true }));
            let warning = layout.labels.first().unwrap();
            assert_eq!(
                warning.text,
                "(Language translations may not be 100% accurate)"
            );
            let unicode = layout
                .widgets
                .iter()
                .find(|w| w.action == Action17::ForceUnicode)
                .unwrap();
            let done = layout
                .widgets
                .iter()
                .find(|w| w.action == Action17::Done)
                .unwrap();
            assert_eq!((unicode.rect[1], done.rect[1]), (202, 202));
        });
    }

    #[test]
    fn key_names_are_the_eras_keyboard_names() {
        assert_eq!(key_name("key.keyboard.w"), "W");
        assert_eq!(key_name("key.keyboard.left.shift"), "LSHIFT");
        assert_eq!(key_name("key.keyboard.space"), "SPACE");
        assert_eq!(key_name("key.keyboard.f5"), "F5");
        assert_eq!(key_name("key.mouse.left"), "Button 1");
        assert_eq!(key_name("key.mouse.right"), "Button 2");
        assert_eq!(key_name("key.mouse.middle"), "Button 3");
        assert_eq!(key_name("key.keyboard.unknown"), "NONE");
    }

    #[test]
    fn a_raw_key_is_never_drawn() {
        lang::scoped(lang::Language::of("en_us", []), || {
            // A key the language lacks is caught, and real text is not.
            assert!(is_untranslated_key("options.generic_value"));
            assert!(!is_untranslated_key("Options"));
            assert!(!is_untranslated_key("US (English)"));
        });
        lang::scoped(en(), || {
            // A key the language has is a translation, not a leaked key.
            assert!(!is_untranslated_key("options.video"));
        });
    }

    #[test]
    fn era_text_is_the_translation_and_a_missing_key_is_none() {
        let language = language(&[("options.on", "ON"), ("test.percent", "100%% accurate")]);
        lang::scoped(language, || {
            assert_eq!(text("options.on").as_deref(), Some("ON"));
            // I18n.translate runs the format, so `%%` is one percent sign.
            assert_eq!(text("test.percent").as_deref(), Some("100% accurate"));
            assert_eq!(text("options.generic_value"), None);
            // A label that is real text is not mistaken for a key.
            assert_translated("Options");
        });
    }

    #[test]
    fn no_menu_draws_a_label_the_language_lacks() {
        // Every visible string of every era menu is text, never the key it
        // was looked up with: a widget whose label the language cannot give
        // is left out by the layouts, and nothing here is a bare key.
        with_en(|| {
            let languages = [LanguageInfo {
                code: "en_us".into(),
                name: "English".into(),
                region: "US".into(),
                bidirectional: false,
            }];
            let mut view = MenuView17::new(427, 240, 4);
            view.languages = &languages;
            let settings = Settings::default();
            let text_width = |t: &str| t.len() as i32 * 6;
            for menu in [
                Menu17::Pause,
                Menu17::Options,
                Menu17::Video,
                Menu17::Sounds,
                Menu17::Controls,
                Menu17::Language,
            ] {
                let layout = layout(menu, &settings, &view, &text_width);
                let labels = layout
                    .title
                    .iter()
                    .map(|title| &title.text)
                    .chain(layout.labels.iter().map(|label| &label.text))
                    .chain(layout.widgets.iter().map(|widget| &widget.label));
                for label in labels {
                    assert!(
                        !is_untranslated_key(label),
                        "{menu:?} draws the key {label:?}"
                    );
                }
            }
        });
    }

    #[test]
    fn a_missing_language_leaves_the_widgets_out() {
        lang::scoped(lang::Language::of("en_us", []), || {
            let view = MenuView17::new(427, 240, 4);
            let pause = pause_layout(&view);
            assert!(pause.widgets.is_empty(), "no key, no button");
            assert!(pause.title.is_none());
            let options = options_layout(&view);
            assert!(options.widgets.is_empty());
            assert!(options.title.is_none());
            let video = video_layout(&Settings::default(), &view);
            assert!(video.widgets.is_empty());
        });
    }

    #[test]
    fn every_child_screen_returns_to_options() {
        for menu in [
            Menu17::Video,
            Menu17::Sounds,
            Menu17::Controls,
            Menu17::Language,
        ] {
            assert_eq!(menu.parent(), Some(Menu17::Options));
        }
        assert_eq!(Menu17::Options.parent(), None);
    }

    #[test]
    fn a_widget_under_the_pointer_is_the_one_clicked() {
        with_en(|| {
            let view = MenuView17::new(427, 240, 4);
            let layout = pause_layout(&view);
            assert_eq!(
                layout.widget_at(150.0, 75.0).map(|w| w.action),
                Some(Action17::BackToGame)
            );
            assert_eq!(
                layout.widget_at(260.0, 98.0).map(|w| w.action),
                Some(Action17::Statistics)
            );
            assert_eq!(layout.widget_at(400.0, 98.0).map(|w| w.action), None);
        });
    }
}
