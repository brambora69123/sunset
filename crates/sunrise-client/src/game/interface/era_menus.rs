//! Minecraft 1.7.10's own menus over the live game: the pause menu
//! (`GameMenuScreen`), Options and its Music & Sounds, Video Settings,
//! Controls and Language screens, drawn by
//! [`sunrise_client::menus_1_7_10`] from the era's language and its
//! `gui/widgets` sheet.
//!
//! The engine's [`Screen`] still says when the game is paused and which
//! screen is open; this module is the era's picture, hit rectangles and
//! keyboard focus for the screens it owns. A content root whose profile is
//! [`Profile::V1_7_10`](sunrise_core::profile::Profile::V1_7_10) draws them,
//! every other root keeps 26.3's, and the era's pages are only reached from
//! the era's pause menu.

use super::*;
use sunrise_client::{
    hud::HudBatch,
    keybinds::{self, KeyBinds},
    lang::{self, LanguageInfo},
    menus_1_7_10 as era,
    options_ui::Event,
    widgets::{self, ScreenInfo},
};
use winit::keyboard::KeyCode;

/// The era menu state of one game: which Options page is open, where its
/// list is scrolled, which widget the keyboard focused and which key is
/// waiting to be bound.
#[derive(Default)]
pub(super) struct EraMenus {
    /// The Options page on screen. `None` while the pause menu, or while
    /// `Screen::Options` shows 26.3's pages (a title-screen Options).
    page: Option<era::Menu17>,
    /// The widget Tab and the arrow keys focus.
    focus: Option<era::Action17>,
    /// The open page's list scroll, in GUI pixels.
    scroll: i32,
    /// The key binding's button waiting for a key (`selectedKeyBinding`).
    binding: Option<&'static str>,
    /// The slider the left button holds, and its track.
    dragging: Option<(era::Action17, [f32; 2])>,
    /// `LanguageManager.getLanguages`, read when the Language page opens.
    languages: Vec<LanguageInfo>,
}

impl EraMenus {
    /// A key binding's button waits for a key, so the next press binds it
    /// rather than driving the game.
    pub(super) fn waiting_for_key(&self) -> bool {
        self.binding.is_some()
    }

    /// Open one of the era's Options pages at its top.
    fn show(&mut self, page: era::Menu17) {
        self.page = Some(page);
        self.focus = None;
        self.scroll = 0;
        self.binding = None;
        self.dragging = None;
    }

    /// The pages are gone: the pause menu is back, or another screen opened.
    pub(super) fn close(&mut self) {
        self.page = None;
        self.focus = None;
        self.scroll = 0;
        self.binding = None;
        self.dragging = None;
        self.languages.clear();
    }
}

impl Game<'_> {
    /// Whether the era's pause menu and Options are the open screen's.
    pub(super) fn era_menu_open(&self) -> bool {
        self.era_ui() && self.era_menus.page.is_some()
    }

    /// `Window.calculateScale`'s largest scale, which the GUI Scale button
    /// cycles up to.
    fn era_max_scale(&self) -> u32 {
        let framebuffer = self
            .renderer
            .as_ref()
            .map_or([1280, 720], |r| [r.config.width, r.config.height]);
        sunrise_client::window::calculate_scale(
            framebuffer.map(|v| v as i32),
            0,
            self.settings.force_unicode,
        ) as u32
    }

    /// The GUI-scaled size the layouts are built at.
    fn era_menu_size(&self) -> (i32, i32) {
        let (w, h) = self.gui_size();
        (w as i32, h as i32)
    }

    /// The state the era painter reads.
    fn era_menu_view(&self, width: i32, height: i32) -> era::MenuView17<'_> {
        let mut view = era::MenuView17::new(width, height, self.era_max_scale());
        view.remote = self.external_server();
        view.scroll = self.era_menus.scroll;
        view.languages = &self.era_menus.languages;
        view.selected_language = &self.settings.language;
        view.binding = self.era_menus.binding;
        view.focus = self.era_menus.focus;
        view
    }

    /// One screen's layout at `width` by `height` GUI pixels.
    fn era_menu_layout(&self, menu: era::Menu17, width: i32, height: i32) -> era::MenuLayout17 {
        let view = self.era_menu_view(width, height);
        let atlas = &self.gui.atlas;
        let text_width = |text: &str| atlas.font.width(text) as i32;
        era::layout(menu, &self.settings, &view, &text_width)
    }

    /// The era pause menu: the world dimmed, `menu.game` at the top and the
    /// era's four buttons.
    pub(super) fn draw_era_pause_menu(&mut self, batch: &mut HudBatch) {
        let (w, h) = (batch.width as i32, batch.height as i32);
        let layout = self.era_pause_layout(w, h);
        let title = layout
            .title
            .as_ref()
            .map(|title| title.text.clone())
            .unwrap_or_default();
        widgets::begin_screen(|| ScreenInfo::new("paused", title));
        era::paint_background(batch, &self.gui.atlas, true);
        self.draw_screen_phase(batch, sunrise_client::screen_draw::RenderPhase::Background);
        era::paint(batch, &self.gui.atlas, &layout, self.cursor);
        self.draw_screen_phase(batch, sunrise_client::screen_draw::RenderPhase::Foreground);
    }

    fn era_pause_layout(&self, width: i32, height: i32) -> era::MenuLayout17 {
        let view = self.era_menu_view(width, height);
        era::pause_layout(&view)
    }

    /// The era Options screen (or one of its children): the world dimmed, or
    /// the tiled dirt on a title-screen page, and the page's widgets.
    pub(super) fn draw_era_options_menu(&mut self, batch: &mut HudBatch) {
        let Some(page) = self.era_menus.page else {
            return;
        };
        let (w, h) = (batch.width as i32, batch.height as i32);
        let layout = self.era_menu_layout(page, w, h);
        let title = layout
            .title
            .as_ref()
            .map(|title| title.text.clone())
            .unwrap_or_default();
        let id = match page {
            era::Menu17::Options => "options",
            era::Menu17::Video => "options/video",
            era::Menu17::Sounds => "options/sounds",
            era::Menu17::Controls => "options/controls",
            era::Menu17::Language => "options/language",
            era::Menu17::Pause => "options/pause",
        };
        widgets::begin_screen(|| ScreenInfo::new(id, title));
        era::paint_background(batch, &self.gui.atlas, self.title.is_none());
        self.draw_screen_phase(batch, sunrise_client::screen_draw::RenderPhase::Background);
        era::paint(batch, &self.gui.atlas, &layout, self.cursor);
        self.draw_screen_phase(batch, sunrise_client::screen_draw::RenderPhase::Foreground);
    }

    /// A press of the left button on the era pause menu.
    pub(super) fn era_pause_click(&mut self) -> Result<()> {
        let (w, h) = self.era_menu_size();
        let layout = self.era_pause_layout(w, h);
        let Some(action) = layout
            .widget_at(self.cursor.0, self.cursor.1)
            .map(|widget| widget.action)
        else {
            return Ok(());
        };
        self.era_menus.focus = Some(action);
        self.audio.play_button_click();
        self.era_menu_activate(action)
    }

    /// A press of the keyboard on the era pause menu.
    pub(super) fn era_pause_key(&mut self, key: KeyCode) -> Result<()> {
        let (w, h) = self.era_menu_size();
        let layout = self.era_pause_layout(w, h);
        let focusable: Vec<era::Action17> = layout
            .focusable()
            .into_iter()
            .map(|widget| widget.action)
            .collect();
        match key {
            KeyCode::Tab => self.era_step_focus(&focusable, !self.has_shift_down()),
            KeyCode::ArrowUp | KeyCode::ArrowLeft => self.era_step_focus(&focusable, false),
            KeyCode::ArrowDown | KeyCode::ArrowRight => self.era_step_focus(&focusable, true),
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                if let Some(action) = self.era_menus.focus {
                    self.audio.play_button_click();
                    self.era_menu_activate(action)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// `Screen.keyPressed`'s focus step: Tab and the arrows walk the page's
    /// widgets, and Enter presses the one focused.
    fn era_step_focus(&mut self, widgets: &[era::Action17], forward: bool) {
        let Some(next) = era_focus_step(widgets, self.era_menus.focus, forward) else {
            self.era_menus.focus = None;
            return;
        };
        self.era_menus.focus = Some(next);
    }

    /// A press of the left button on the era Options page: a slider takes the
    /// value the pointer's column stands for and follows the pointer until it
    /// is let go, any other widget does its job.
    pub(super) fn era_options_mouse(&mut self, pressed: bool, right: bool) -> Result<()> {
        if right {
            return Ok(());
        }
        if !pressed {
            if self.era_menus.dragging.take().is_some() {
                self.dragging = false;
                return self.era_settings_saved();
            }
            return Ok(());
        }
        let Some(page) = self.era_menus.page else {
            return Ok(());
        };
        let (w, h) = self.era_menu_size();
        let layout = self.era_menu_layout(page, w, h);
        let Some(widget) = layout.widget_at(self.cursor.0, self.cursor.1) else {
            return Ok(());
        };
        let action = widget.action;
        match widget.kind {
            era::Kind17::Slider { track, .. } => {
                self.era_menus.focus = Some(action);
                self.era_menus.dragging = Some((action, track));
                self.dragging = true;
                self.era_slide_to(action, self.cursor.0, track)
            }
            era::Kind17::Button | era::Kind17::Row { .. } => {
                self.era_menus.focus = Some(action);
                self.era_menu_activate(action)
            }
        }
    }

    /// The pointer moved while a slider is held.
    pub(super) fn era_options_drag(&mut self) -> Result<()> {
        let Some((action, track)) = self.era_menus.dragging else {
            return Ok(());
        };
        self.era_slide_to(action, self.cursor.0, track)
    }

    /// A press of the keyboard on the era Options page: a binding waiting for
    /// a key takes it, Escape closes to the game (`Screen.keyPressed`), and
    /// Tab, the arrows and Enter walk and press the page's widgets.
    pub(super) fn era_options_key(&mut self, key: KeyCode) -> Result<()> {
        if let Some(binding) = self.era_menus.binding {
            // KeyBinding.setKeyCode: Escape clears the binding, any other key
            // or button is the new one.
            let bound = if key == KeyCode::Escape {
                "key.keyboard.unknown"
            } else {
                keybinds::key_name(key).unwrap_or("key.keyboard.unknown")
            };
            self.settings.keys.set(binding, bound);
            self.era_menus.binding = None;
            return self.era_settings_saved();
        }
        match key {
            KeyCode::Escape => {
                self.close_options_to_game();
                Ok(())
            }
            KeyCode::Tab => {
                let focusable = self.era_options_focusable();
                self.era_step_focus(&focusable, !self.has_shift_down());
                Ok(())
            }
            KeyCode::ArrowUp | KeyCode::ArrowLeft => {
                if self.era_focused_slider_step(-0.05)? {
                    return Ok(());
                }
                let focusable = self.era_options_focusable();
                self.era_step_focus(&focusable, false);
                Ok(())
            }
            KeyCode::ArrowDown | KeyCode::ArrowRight => {
                if self.era_focused_slider_step(0.05)? {
                    return Ok(());
                }
                let focusable = self.era_options_focusable();
                self.era_step_focus(&focusable, true);
                Ok(())
            }
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                if let Some(action) = self.era_menus.focus
                    && self.era_focused_slider(action).is_none()
                {
                    self.audio.play_button_click();
                    self.era_menu_activate(action)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// The focused widget when the keyboard steps a slider, moved by `step`
    /// along its track; `false` when the focus is not a slider.
    fn era_focused_slider_step(&mut self, step: f32) -> Result<bool> {
        let Some(action) = self.era_menus.focus else {
            return Ok(false);
        };
        if self.era_focused_slider(action).is_none() {
            return Ok(false);
        }
        let value = (self.era_slider_position(action) + step).clamp(0.0, 1.0);
        self.era_set_slider(action, value)?;
        Ok(true)
    }

    /// The track of a slider among the open page's widgets.
    fn era_focused_slider(&self, action: era::Action17) -> Option<[f32; 2]> {
        let page = self.era_menus.page?;
        let (w, h) = self.era_menu_size();
        let layout = self.era_menu_layout(page, w, h);
        layout
            .widgets
            .iter()
            .find(|widget| widget.action == action)
            .and_then(|widget| match widget.kind {
                era::Kind17::Slider { track, .. } => Some(track),
                _ => None,
            })
    }

    /// A slider's value as drawn (0..1).
    fn era_slider_position(&self, action: era::Action17) -> f32 {
        match action {
            era::Action17::Video(option) => option.position(&self.settings).unwrap_or(0.0),
            era::Action17::Sound(0) => self.settings.master_volume as f32,
            era::Action17::Sound(source) => self.settings.volumes[source] as f32,
            era::Action17::Sensitivity => self.settings.sensitivity as f32,
            _ => 0.0,
        }
    }

    /// The actions Tab and the arrows walk on the open page.
    fn era_options_focusable(&self) -> Vec<era::Action17> {
        let Some(page) = self.era_menus.page else {
            return Vec::new();
        };
        let (w, h) = self.era_menu_size();
        self.era_menu_layout(page, w, h)
            .focusable()
            .into_iter()
            .map(|widget| widget.action)
            .collect()
    }

    /// The wheel over a page's list (`ListWidget.render`'s `Mouse.getEventDWheel`):
    /// half a row a notch.
    pub(super) fn era_options_scroll(&mut self, amount: f32) -> Result<()> {
        let Some(page) = self.era_menus.page else {
            return Ok(());
        };
        let row = match page {
            era::Menu17::Video => 25,
            era::Menu17::Controls => 20,
            era::Menu17::Language => 18,
            _ => return Ok(()),
        };
        let (w, h) = self.era_menu_size();
        let layout = self.era_menu_layout(page, w, h);
        let Some(list) = layout.list else {
            return Ok(());
        };
        // ListWidget.render reads the wheel only over its own rectangle.
        if self.cursor.1 < list.y as f32
            || self.cursor.1 > (list.y + list.h) as f32
            || self.cursor.0 < list.x as f32
            || self.cursor.0 > (list.x + list.w) as f32
        {
            return Ok(());
        }
        // The era's wheel steps the list by half a row, up for a notch up.
        let step = amount.round() as i32 * (row / 2);
        self.era_menus.scroll = (self.era_menus.scroll - step).clamp(0, list.max_scroll());
        Ok(())
    }

    /// Give a slider the value the track at `x` stands for.
    fn era_slide_to(&mut self, action: era::Action17, x: f32, track: [f32; 2]) -> Result<()> {
        let value = ((x - track[0]) / track[1]).clamp(0.0, 1.0);
        self.era_set_slider(action, value)
    }

    /// One slider's setting, 0..1 along its track.
    fn era_set_slider(&mut self, action: era::Action17, value: f32) -> Result<()> {
        let source = match action {
            era::Action17::Video(option) => {
                option.slide(&mut self.settings, value);
                None
            }
            era::Action17::Sound(0) => {
                self.settings.master_volume = f64::from(value);
                Some(0)
            }
            era::Action17::Sound(source) => {
                self.settings.volumes[source] = f64::from(value);
                Some(source)
            }
            era::Action17::Sensitivity => {
                self.settings.sensitivity = f64::from(value);
                None
            }
            _ => return Ok(()),
        };
        // OptionSliderWidget.render: the setting follows the handle while it
        // is held, and only the release saves it.
        let mut events = vec![Event::SliderDrag];
        if let Some(source) = source {
            events.push(Event::Volume(source, value));
        }
        self.handle_option_events(events)
    }

    /// A press of the left button on the era Controls page while a binding
    /// waits takes the button.
    pub(super) fn era_bind_mouse(&mut self, button: winit::event::MouseButton) -> Result<()> {
        let Some(binding) = self.era_menus.binding.take() else {
            return Ok(());
        };
        let bound = keybinds::mouse_name(button);
        self.settings.keys.set(binding, &bound);
        self.era_settings_saved()
    }

    /// One widget of an era menu was pressed.
    fn era_menu_activate(&mut self, action: era::Action17) -> Result<()> {
        match action {
            era::Action17::BackToGame => {
                self.era_menus.focus = None;
                self.pause_focus = None;
                self.pause_menu = true;
                self.set_screen(Screen::Playing);
                self.capture(true);
            }
            era::Action17::OpenOptions => {
                self.open_options(false);
                self.era_menus.show(era::Menu17::Options);
            }
            era::Action17::Statistics => self.open_stats(),
            era::Action17::QuitWorld => self.close_world()?,
            era::Action17::OpenVideo => self.era_open_options_page(era::Menu17::Video),
            era::Action17::OpenSounds => self.era_open_options_page(era::Menu17::Sounds),
            era::Action17::OpenControls => self.era_open_options_page(era::Menu17::Controls),
            era::Action17::OpenLanguage => self.era_open_options_page(era::Menu17::Language),
            era::Action17::Done => self.era_options_back()?,
            era::Action17::ResetAllKeys => {
                self.settings.keys.reset_all();
                return self.era_settings_saved();
            }
            era::Action17::InvertMouse => {
                self.settings.invert_y = !self.settings.invert_y;
                return self.era_settings_saved();
            }
            era::Action17::Sensitivity => {}
            era::Action17::BindKey(name) => self.era_menus.binding = Some(name),
            era::Action17::ResetKey(name) => {
                self.settings.keys.set(name, KeyBinds::default_of(name));
                return self.era_settings_saved();
            }
            era::Action17::Language(index) => {
                if let Some(language) = self.era_menus.languages.get(index)
                    && language.code != self.settings.language
                {
                    let code = language.code.clone();
                    self.settings.language = code.clone();
                    self.load_language(&code);
                    return self.era_settings_saved();
                }
            }
            era::Action17::ForceUnicode => {
                self.settings.force_unicode = !self.settings.force_unicode;
                return self.era_settings_saved();
            }
            // Only a cycle button reaches here: a slider's press is the
            // pointer's column, taken in `era_options_mouse`.
            era::Action17::Video(option) => {
                let max_scale = self.era_max_scale();
                option.cycle(&mut self.settings, max_scale);
                return self.era_settings_saved();
            }
            era::Action17::Sound(_) => {}
        }
        Ok(())
    }

    /// Open one of the era's Options pages, reading the languages the
    /// Language page lists.
    fn era_open_options_page(&mut self, page: era::Menu17) {
        if page == era::Menu17::Language {
            self.era_menus.languages = lang::available(&[&self.pack.source]);
        }
        self.era_menus.show(page);
    }

    /// `gui.done`: a child page goes back to Options, Options back to the
    /// pause menu.
    fn era_options_back(&mut self) -> Result<()> {
        match self.era_menus.page.and_then(era::Menu17::parent) {
            Some(parent) => self.era_menus.show(parent),
            None => {
                self.leave_options();
                self.pause_focus = None;
            }
        }
        Ok(())
    }

    /// Settings changed: apply and save them, as the 26.3 Options pages do.
    fn era_settings_saved(&mut self) -> Result<()> {
        self.handle_option_events(vec![Event::Changed])
    }
}

/// The next widget a focus step lands on, wrapping at both ends.
pub(super) fn era_focus_step(
    widgets: &[era::Action17],
    focus: Option<era::Action17>,
    forward: bool,
) -> Option<era::Action17> {
    let position = focus.and_then(|focus| widgets.iter().position(|action| *action == focus));
    match position {
        Some(at) if forward => widgets.get((at + 1) % widgets.len()).copied(),
        Some(at) => widgets
            .get((at + widgets.len() - 1) % widgets.len())
            .copied(),
        None if forward => widgets.first().copied(),
        None => widgets.last().copied(),
    }
}
