//! Minecraft 1.7.10's HUD, laid out as draw calls from the era's own
//! `textures/gui/icons.png` and `textures/gui/widgets.png`.
//!
//! The era draws its HUD in a fixed 256x256 texture coordinate space
//! (`GuiElement.drawTexture` scales every `u`/`v` by `1/256`,
//! `net/minecraft/client/gui/GuiElement.java:105`), whatever resolution a
//! resource pack replaces the file with. [`Icon`] keeps that coordinate space
//! and [`Icon::atlas_parts`] maps it onto the packed [`HudAtlas`] region.
//!
//! The layout is read from `net/minecraft/client/gui/GameGui.java`
//! (`GuiIngame`): `render` draws the hotbar, selection frame and crosshair
//! (lines 95-103), `renderStatusBars` the armour, hearts, food and air
//! (lines 458-662), the experience bar and level (lines 161-191) and the held
//! item's name (lines 193-219). The low-health and hunger jitters use the
//! `java.util.Random` seeded with `ticks * 312871` (line 466), drawn in
//! vanilla's order.
//!
//! The era's UI is selected by the content root's profile
//! ([`sunrise_core::profile::Profile::V1_7_10`]); 26.3 keeps
//! [`crate::hud_layout`].

use crate::hud::{HudAtlas, HudBatch, Rect, rgba};
use smallvec::SmallVec;
use sunrise_core::random::{LegacyRandom, RandomSource};

/// The era's HUD icon sheet.
pub const ICONS: &str = "gui/icons";
/// The era's hotbar sheet.
pub const WIDGETS: &str = "gui/widgets";
/// The GUI textures' fixed coordinate space, `drawTexture`'s 1/256.
pub const GUI_TEXTURE_SIZE: i32 = 256;

/// One cell of an era GUI texture, in the fixed 256x256 coordinate space.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Icon {
    /// The texture under `assets/minecraft/textures/`, without `.png`.
    pub texture: &'static str,
    pub u: i32,
    pub v: i32,
    pub w: i32,
    pub h: i32,
}

/// One draw of an [`Icon`]: its atlas region and its offset inside the cell.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IconPart {
    pub rect: Rect,
    pub dx: i32,
    pub dy: i32,
    pub w: i32,
    pub h: i32,
}

impl Icon {
    pub const fn at(texture: &'static str, u: i32, v: i32, w: i32, h: i32) -> Self {
        Self {
            texture,
            u,
            v,
            w,
            h,
        }
    }

    /// The cell's atlas draws. The era samples GUI textures with `GL_REPEAT`
    /// (`TextureUtil.setTextureClamp(false)` at upload), so a cell that runs
    /// past the 256x256 space wraps to the same texture's other edge: the
    /// furnace's full flame starts at `v = -1` and reads the sheet's last row
    /// first. Most cells are one draw; none here needs more than two.
    pub fn atlas_parts(self, atlas: &HudAtlas) -> SmallVec<[IconPart; 4]> {
        let mut parts = SmallVec::new();
        let Some(source) = atlas.blit_rect(self.texture).copied() else {
            return parts;
        };
        let (u, nu) = axis_parts(self.u, self.w);
        let (v, nv) = axis_parts(self.v, self.h);
        for &(u0, w, dx) in &u[..nu] {
            for &(v0, h, dy) in &v[..nv] {
                parts.push(IconPart {
                    rect: cell_rect(source, u0, v0, w, h),
                    dx,
                    dy,
                    w,
                    h,
                });
            }
        }
        parts
    }
}

/// `(source start, length, destination offset)` for one axis of a cell: two
/// pieces when the cell crosses the 256 edge, else one.
fn axis_parts(start: i32, len: i32) -> ([(i32, i32, i32); 2], usize) {
    if start < 0 {
        (
            [
                (GUI_TEXTURE_SIZE + start, -start, 0),
                (0, start + len, -start),
            ],
            2,
        )
    } else if start + len > GUI_TEXTURE_SIZE {
        (
            [
                (start, GUI_TEXTURE_SIZE - start, 0),
                (0, start + len - GUI_TEXTURE_SIZE, GUI_TEXTURE_SIZE - start),
            ],
            2,
        )
    } else {
        ([(start, len, 0), (0, 0, 0)], 1)
    }
}

/// A cell of the texture's region, in the era's 1/256 coordinate space.
fn cell_rect(source: Rect, u: i32, v: i32, w: i32, h: i32) -> Rect {
    let size = GUI_TEXTURE_SIZE as f32;
    let (du, dv) = (source.u1 - source.u0, source.v1 - source.v0);
    let u0 = source.u0 + du * u as f32 / size;
    let v0 = source.v0 + dv * v as f32 / size;
    Rect {
        u0,
        v0,
        u1: u0 + du * w as f32 / size,
        v1: v0 + dv * h as f32 / size,
        w: w as u32,
        h: h as u32,
    }
}

/// The crosshair, `GameGui.render:103`.
pub const CROSSHAIR: Icon = Icon::at(ICONS, 0, 0, 16, 16);
/// The hotbar's 182x22 well, `GameGui.render:98`.
pub const HOTBAR: Icon = Icon::at(WIDGETS, 0, 0, 182, 22);
/// The 24x22 selection frame, `GameGui.render:99`.
pub const HOTBAR_SELECTION: Icon = Icon::at(WIDGETS, 0, 22, 24, 22);
/// A full armour point, `GameGui.renderStatusBars:493`.
pub const ARMOR_FULL: Icon = Icon::at(ICONS, 34, 9, 9, 9);
/// A half armour point, `GameGui.renderStatusBars:497`.
pub const ARMOR_HALF: Icon = Icon::at(ICONS, 25, 9, 9, 9);
/// An empty armour point, `GameGui.renderStatusBars:501`.
pub const ARMOR_EMPTY: Icon = Icon::at(ICONS, 16, 9, 9, 9);
/// A full air bubble, `GameGui.renderStatusBars:654`.
pub const AIR_FULL: Icon = Icon::at(ICONS, 16, 18, 9, 9);
/// The popping air bubble, `GameGui.renderStatusBars:656`.
pub const AIR_PARTIAL: Icon = Icon::at(ICONS, 25, 18, 9, 9);
/// The experience bar's empty 182x5 track, `GameGui.render:169`.
pub const XP_BACKGROUND: Icon = Icon::at(ICONS, 0, 64, 182, 5);
/// The experience bar's fill, `GameGui.render:171`; its width is the progress.
pub const XP_FILL: Icon = Icon::at(ICONS, 0, 69, 182, 5);
/// A normal food shank, `GameGui.renderStatusBars:589`.
pub const FOOD_CONTAINER: Icon = Icon::at(ICONS, 16, 27, 9, 9);
/// The hunger effect's shank, `GameGui.renderStatusBars:575-589` (`var46 = 13`).
pub const FOOD_CONTAINER_HUNGER: Icon = Icon::at(ICONS, 133, 27, 9, 9);
/// The uneaten-food flash shank; `var6` is always false in this build, so the
/// branch at `GameGui.renderStatusBars:584` never runs.
pub const FOOD_CONTAINER_LOW: Icon = Icon::at(ICONS, 25, 27, 9, 9);

/// Which status effect tints a heart, `GameGui.renderStatusBars:510-514`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeartEffect {
    Normal,
    Poison,
    Wither,
}

impl HeartEffect {
    fn base(self) -> i32 {
        match self {
            Self::Normal => 16,
            Self::Poison => 52,
            Self::Wither => 88,
        }
    }
}

/// The six content cells of one heart, `GameGui.renderStatusBars:540-562`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeartKind {
    Full,
    Half,
    FullBlink,
    HalfBlink,
    AbsorptionHalf,
    AbsorptionFull,
}

impl HeartKind {
    fn offset(self) -> i32 {
        match self {
            Self::Full => 36,
            Self::Half => 45,
            Self::FullBlink => 54,
            Self::HalfBlink => 63,
            Self::AbsorptionHalf => 144,
            Self::AbsorptionFull => 153,
        }
    }
}

/// A heart's container, `GameGui.renderStatusBars:537`: `16 + var24 * 9`, where
/// `var24` is the low-health blink, at `9 * var28` where `var28 = 5` is hardcore.
pub fn heart_container(blink: bool, hardcore: bool) -> Icon {
    Icon::at(
        ICONS,
        if blink { 25 } else { 16 },
        if hardcore { 45 } else { 0 },
        9,
        9,
    )
}

/// A heart's content cell: the effect's base (`var36`), the kind's offset and
/// the hardcore row (`9 * var28`), `GameGui.renderStatusBars:537-562`.
pub fn heart_icon(effect: HeartEffect, hardcore: bool, kind: HeartKind) -> Icon {
    Icon::at(
        ICONS,
        effect.base() + kind.offset(),
        if hardcore { 45 } else { 0 },
        9,
        9,
    )
}

/// The four food content cells, `GameGui.renderStatusBars:601-605`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FoodKind {
    Full,
    Half,
    FullBlink,
    HalfBlink,
}

impl FoodKind {
    fn offset(self) -> i32 {
        match self {
            Self::Full => 36,
            Self::Half => 45,
            Self::FullBlink => 54,
            Self::HalfBlink => 63,
        }
    }
}

/// A food content cell: `var43 + offset`, where the hunger effect sets
/// `var43 = 52` (`GameGui.renderStatusBars:575-605`).
pub fn food_icon(hungry: bool, kind: FoodKind) -> Icon {
    Icon::at(
        ICONS,
        (if hungry { 52 } else { 16 }) + kind.offset(),
        27,
        9,
        9,
    )
}

/// What the HUD needs from the player this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HudPlayer17 {
    pub health: f32,
    /// `prevHealth`, drawn by the hurt flash.
    pub prev_health: i32,
    pub max_health: f32,
    pub absorption: f32,
    /// `getArmorProtection()`, armour points (0-20).
    pub armor: i32,
    pub food: i32,
    pub saturation: f32,
    /// `getBreath()`, 300 at the surface.
    pub breath: i32,
    pub xp_progress: f32,
    pub xp_level: i32,
    pub riding: bool,
    pub submerged: bool,
    pub poisoned: bool,
    pub withered: bool,
    pub hungry: bool,
    pub regeneration: bool,
    pub hardcore: bool,
    /// `invulnerableTimer`, in ticks.
    pub invulnerable_timer: i32,
}

impl Default for HudPlayer17 {
    fn default() -> Self {
        Self {
            health: 20.0,
            prev_health: 20,
            max_health: 20.0,
            absorption: 0.0,
            armor: 0,
            food: 20,
            saturation: 5.0,
            breath: 300,
            xp_progress: 0.0,
            xp_level: 0,
            riding: false,
            submerged: false,
            poisoned: false,
            withered: false,
            hungry: false,
            regeneration: false,
            hardcore: false,
            invulnerable_timer: 0,
        }
    }
}

/// One hotbar stack's damage, for the durability bar
/// (`ItemRenderer.renderGuiItemDecoration`, `ItemRenderer.java:454-467`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HotbarItem17 {
    pub damage: i32,
    pub max_damage: i32,
}

/// The rest of the HUD's frame state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HudState17<'a> {
    /// `GameGui.ticks`.
    pub ticks: i32,
    pub selected_slot: i32,
    /// `interactionManager.hasStatusBars()`.
    pub show_status_bars: bool,
    /// `interactionManager.hasXpBar()`.
    pub show_xp_bar: bool,
    /// `options.itemInHandTooltips`.
    pub show_item_name: bool,
    pub held_item: Option<&'a str>,
    /// `mainHandMessageTimer`, 0-40; 40 is fully opaque, 0 hidden.
    pub held_item_timer: i32,
    pub hotbar_items: [Option<HotbarItem17>; 9],
}

impl Default for HudState17<'_> {
    fn default() -> Self {
        Self {
            ticks: 0,
            selected_slot: 0,
            show_status_bars: true,
            show_xp_bar: true,
            show_item_name: true,
            held_item: None,
            held_item_timer: 0,
            hotbar_items: [None; 9],
        }
    }
}

/// How a draw blends, as `GameGui` binds it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Blend17 {
    /// `GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA`.
    Alpha,
    /// `GL_ONE_MINUS_DST_COLOR, GL_ONE_MINUS_SRC_COLOR` (`GameGui.render:102`).
    Invert,
}

/// One `drawTexture`/`fillRect` of the HUD.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HudElement {
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    /// The sprite cell; `None` for a coloured fill (the durability bar).
    pub icon: Option<Icon>,
    /// RGBA, the HUD batch's packing; white for a sprite.
    pub color: u32,
    pub blend: Blend17,
}

/// A `TextRenderer` draw of the HUD.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextShadow17 {
    None,
    /// `drawWithShadow`.
    Drop,
    /// The experience level's four-offset black outline
    /// (`GameGui.render:183-187`).
    Outline,
}

/// One text draw of the HUD.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HudText {
    pub name: String,
    pub text: String,
    pub x: i32,
    pub y: i32,
    pub color: u32,
    pub shadow: TextShadow17,
}

/// The HUD's frame: its sprite/fill draws, its text and the nine hotbar item
/// origins (`GameGui.render:118`, `width / 2 - 90 + slot * 20 + 2`,
/// `height - 16 - 3`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HudLayout17 {
    pub elements: Vec<HudElement>,
    pub texts: Vec<HudText>,
    pub hotbar_slots: [(i32, i32); 9],
}

/// The era's HUD in GUI pixels (`Window.getWidth`/`getHeight`, not the
/// framebuffer's), exactly as `GameGui` lays it out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hud17 {
    pub width: i32,
    pub height: i32,
}

impl Hud17 {
    pub const fn new(width: i32, height: i32) -> Self {
        Self { width, height }
    }

    /// The GUI-scaled size of a frame batch (`HudBatch::width`/`height`, which
    /// `Window.calculateScale` produced).
    pub fn from_batch(batch: &HudBatch) -> Self {
        Self {
            width: batch.width as i32,
            height: batch.height as i32,
        }
    }

    /// The whole HUD, in vanilla's draw order: hotbar and selection, crosshair,
    /// status bars, the hotbar's durability bars, experience, then the held
    /// item's name. `text_width` is `TextRenderer.getWidth`.
    pub fn layout(
        &self,
        player: &HudPlayer17,
        state: &HudState17<'_>,
        text_width: &dyn Fn(&str) -> i32,
    ) -> HudLayout17 {
        let mut out = HudLayout17::default();
        self.hotbar(&mut out, state.selected_slot);
        out.elements.push(HudElement {
            name: "crosshair".to_owned(),
            x: self.width / 2 - 7,
            y: self.height / 2 - 7,
            w: 16,
            h: 16,
            icon: Some(CROSSHAIR),
            color: u32::MAX,
            blend: Blend17::Invert,
        });
        if state.show_status_bars {
            self.status_bars(&mut out, player, state.ticks);
        }
        self.durability(&mut out, state);
        if state.show_xp_bar {
            self.experience(&mut out, player, text_width);
        }
        self.item_name(&mut out, state, text_width);
        out
    }

    fn hotbar(&self, out: &mut HudLayout17, selected_slot: i32) {
        let left = self.width / 2 - 91;
        let top = self.height - 22;
        sprite(out, "hotbar", left, top, 182, 22, HOTBAR);
        sprite(
            out,
            "hotbar_selection",
            left - 1 + selected_slot * 20,
            top - 1,
            24,
            22,
            HOTBAR_SELECTION,
        );
        for (slot, origin) in out.hotbar_slots.iter_mut().enumerate() {
            *origin = (
                self.width / 2 - 90 + slot as i32 * 20 + 2,
                self.height - 16 - 3,
            );
        }
    }

    fn status_bars(&self, out: &mut HudLayout17, player: &HudPlayer17, ticks: i32) {
        let left = self.width / 2 - 91;
        let right = self.width / 2 + 91;
        let top = self.height - 39;

        let health = player.health.ceil() as i32;
        let rows = ((player.max_health + player.absorption) / 2.0 / 10.0).ceil() as i32;
        let spacing = (10 - (rows - 2)).max(3);
        // `GameGui.renderStatusBars:479`, where the armour and bubbles sit.
        let heart_top = top - (rows - 1) * spacing - 10;
        let blink = player.invulnerable_timer / 3 % 2 == 1 && player.invulnerable_timer >= 10;

        // Armour, one row above the hearts (`GameGui.renderStatusBars:489-504`).
        if player.armor > 0 {
            for slot in 0..10 {
                let icon = if slot * 2 + 1 < player.armor {
                    ARMOR_FULL
                } else if slot * 2 + 1 == player.armor {
                    ARMOR_HALF
                } else {
                    ARMOR_EMPTY
                };
                sprite(
                    out,
                    &format!("armor_{slot}"),
                    left + slot * 8,
                    heart_top,
                    9,
                    9,
                    icon,
                );
            }
        }

        // Hearts, from the top-right one down (`GameGui.renderStatusBars:508-565`).
        let effect = if player.poisoned {
            HeartEffect::Poison
        } else if player.withered {
            HeartEffect::Wither
        } else {
            HeartEffect::Normal
        };
        let regen_span = (player.max_health + 5.0).ceil() as i32;
        let regeneration = player.regeneration.then(|| ticks % regen_span.max(1));
        let mut random = LegacyRandom::new(i64::from(ticks.wrapping_mul(312_871)));
        let total_hearts = ((player.max_health + player.absorption) / 2.0).ceil() as i32;
        let mut absorption_left = player.absorption;
        for heart in (0..total_hearts).rev() {
            let row = (f64::from(heart + 1) / 10.0).ceil() as i32 - 1;
            let mut y = top - row * spacing;
            if health <= 4 {
                y += random.next_int_bound(2).unwrap_or(0);
            }
            if regeneration == Some(heart) {
                y -= 2;
            }
            let x = left + heart % 10 * 8;
            sprite(
                out,
                &format!("heart_{heart}_container"),
                x,
                y,
                9,
                9,
                heart_container(blink, player.hardcore),
            );
            if blink {
                if heart * 2 + 1 < player.prev_health {
                    sprite(
                        out,
                        &format!("heart_{heart}_flash_full"),
                        x,
                        y,
                        9,
                        9,
                        heart_icon(effect, player.hardcore, HeartKind::FullBlink),
                    );
                }
                if heart * 2 + 1 == player.prev_health {
                    sprite(
                        out,
                        &format!("heart_{heart}_flash_half"),
                        x,
                        y,
                        9,
                        9,
                        heart_icon(effect, player.hardcore, HeartKind::HalfBlink),
                    );
                }
            }
            if absorption_left > 0.0 {
                let kind = if absorption_left == player.absorption && player.absorption % 2.0 == 1.0
                {
                    HeartKind::AbsorptionFull
                } else {
                    HeartKind::AbsorptionHalf
                };
                sprite(
                    out,
                    &format!("heart_{heart}_absorption"),
                    x,
                    y,
                    9,
                    9,
                    heart_icon(effect, player.hardcore, kind),
                );
                absorption_left -= 2.0;
            } else {
                if heart * 2 + 1 < health {
                    sprite(
                        out,
                        &format!("heart_{heart}_full"),
                        x,
                        y,
                        9,
                        9,
                        heart_icon(effect, player.hardcore, HeartKind::Full),
                    );
                }
                if heart * 2 + 1 == health {
                    sprite(
                        out,
                        &format!("heart_{heart}_half"),
                        x,
                        y,
                        9,
                        9,
                        heart_icon(effect, player.hardcore, HeartKind::Half),
                    );
                }
            }
        }

        // Food, right to left (`GameGui.renderStatusBars:571-607`).
        if !player.riding {
            let container = if player.hungry {
                FOOD_CONTAINER_HUNGER
            } else {
                FOOD_CONTAINER
            };
            let jitter_period = player.food * 3 + 1;
            for slot in 0..10 {
                let mut y = top;
                if player.saturation <= 0.0 && ticks % jitter_period == 0 {
                    y += random.next_int_bound(3).unwrap_or(0) - 1;
                }
                let x = right - slot * 8 - 9;
                sprite(
                    out,
                    &format!("food_{slot}_container"),
                    x,
                    y,
                    9,
                    9,
                    container,
                );
                if slot * 2 + 1 < player.food {
                    sprite(
                        out,
                        &format!("food_{slot}_full"),
                        x,
                        y,
                        9,
                        9,
                        food_icon(player.hungry, FoodKind::Full),
                    );
                }
                if slot * 2 + 1 == player.food {
                    sprite(
                        out,
                        &format!("food_{slot}_half"),
                        x,
                        y,
                        9,
                        9,
                        food_icon(player.hungry, FoodKind::Half),
                    );
                }
            }
        }

        // Air bubbles above the food, right to left
        // (`GameGui.renderStatusBars:647-659`).
        if player.submerged {
            let full = (f64::from(player.breath - 2) * 10.0 / 300.0).ceil() as i32;
            let partial = (f64::from(player.breath) * 10.0 / 300.0).ceil() as i32 - full;
            for bubble in 0..full + partial {
                let icon = if bubble < full { AIR_FULL } else { AIR_PARTIAL };
                sprite(
                    out,
                    &format!(
                        "air_{bubble}_{}",
                        if bubble < full { "full" } else { "partial" }
                    ),
                    right - bubble * 8 - 9,
                    heart_top,
                    9,
                    9,
                    icon,
                );
            }
        }
    }

    fn experience(
        &self,
        out: &mut HudLayout17,
        player: &HudPlayer17,
        text_width: &dyn Fn(&str) -> i32,
    ) {
        let left = self.width / 2 - 91;
        let y = self.height - 32 + 3;
        sprite(out, "xp_background", left, y, 182, 5, XP_BACKGROUND);
        let progress = (player.xp_progress * 183.0) as i32;
        if progress > 0 {
            // `GameGui.render:171` draws the first `progress` columns of the
            // same sheet at (0, 69), 183 at a full bar.
            sprite(
                out,
                "xp_fill",
                left,
                y,
                progress,
                5,
                Icon::at(ICONS, 0, 69, progress, 5),
            );
        }
        if player.xp_level > 0 {
            let number = player.xp_level.to_string();
            let x = (self.width - text_width(&number)) / 2;
            out.texts.push(HudText {
                name: "xp_level".to_owned(),
                text: number,
                x,
                y: self.height - 31 - 4,
                color: rgba(0x80, 0xff, 0x20, 0xff),
                shadow: TextShadow17::Outline,
            });
        }
    }

    fn item_name(
        &self,
        out: &mut HudLayout17,
        state: &HudState17<'_>,
        text_width: &dyn Fn(&str) -> i32,
    ) {
        if !state.show_item_name || state.held_item_timer <= 0 {
            return;
        }
        let Some(name) = state.held_item else {
            return;
        };
        let alpha = ((state.held_item_timer as f32 * 256.0 / 10.0) as i32).min(255);
        if alpha <= 0 {
            return;
        }
        let x = (self.width - text_width(name)) / 2;
        let y = self.height - 59 + if state.show_status_bars { 0 } else { 14 };
        out.texts.push(HudText {
            name: "held_item_name".to_owned(),
            text: name.to_owned(),
            x,
            y,
            color: rgba(0xff, 0xff, 0xff, alpha as u8),
            shadow: TextShadow17::Drop,
        });
    }

    fn durability(&self, out: &mut HudLayout17, state: &HudState17<'_>) {
        for (slot, item) in state.hotbar_items.iter().enumerate() {
            let Some(item) = item else {
                continue;
            };
            if item.damage <= 0 || item.max_damage <= 0 {
                continue;
            }
            let (x, y) = out.hotbar_slots[slot];
            let (x, y) = (x + 2, y + 13);
            let width =
                (13.0 - f64::from(item.damage) * 13.0 / f64::from(item.max_damage)).round() as i32;
            let shade = (255.0 - f64::from(item.damage) * 255.0 / f64::from(item.max_damage))
                .round() as i32;
            // `ItemRenderer.renderGuiItemDecoration`: the 13x2 black backing,
            // then the 12x1 track and the damage-coloured 1-pixel fill.
            fill(
                out,
                &format!("durability_{slot}_backing"),
                x,
                y,
                13,
                2,
                rgba(0, 0, 0, 0xff),
            );
            fill(
                out,
                &format!("durability_{slot}_track"),
                x,
                y,
                12,
                1,
                rgba(((255 - shade) / 4) as u8, 0x3f, 0, 0xff),
            );
            fill(
                out,
                &format!("durability_{slot}_fill"),
                x,
                y,
                width,
                1,
                rgba((255 - shade) as u8, shade as u8, 0, 0xff),
            );
        }
    }
}

/// Draws the HUD's sprites, fills and text with the frame's batch.
pub fn paint(batch: &mut HudBatch, atlas: &HudAtlas, layout: &HudLayout17) {
    for element in &layout.elements {
        let start = batch.vertices.len();
        match element.icon {
            Some(icon) => {
                for part in icon.atlas_parts(atlas) {
                    batch.rect_uv(
                        (element.x + part.dx) as f32,
                        (element.y + part.dy) as f32,
                        part.w as f32,
                        part.h as f32,
                        part.rect,
                        element.color,
                    );
                }
            }
            None => batch.fill(
                atlas,
                element.x as f32,
                element.y as f32,
                element.w as f32,
                element.h as f32,
                element.color,
            ),
        }
        if element.blend == Blend17::Invert {
            batch.invert_since(start);
        }
    }
    for text in &layout.texts {
        draw_text(batch, atlas, text);
    }
}

fn draw_text(batch: &mut HudBatch, atlas: &HudAtlas, text: &HudText) {
    let (x, y) = (text.x as f32, text.y as f32);
    match text.shadow {
        TextShadow17::None => {
            batch.text(atlas, &text.text, x, y, text.color, false);
        }
        TextShadow17::Drop => {
            batch.text(atlas, &text.text, x, y, text.color, true);
        }
        TextShadow17::Outline => {
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                batch.text(
                    atlas,
                    &text.text,
                    x + dx as f32,
                    y + dy as f32,
                    rgba(0, 0, 0, 0xff),
                    false,
                );
            }
            batch.text(atlas, &text.text, x, y, text.color, false);
        }
    }
}

fn sprite(out: &mut HudLayout17, name: &str, x: i32, y: i32, w: i32, h: i32, icon: Icon) {
    out.elements.push(HudElement {
        name: name.to_owned(),
        x,
        y,
        w,
        h,
        icon: Some(icon),
        color: u32::MAX,
        blend: Blend17::Alpha,
    });
}

fn fill(out: &mut HudLayout17, name: &str, x: i32, y: i32, w: i32, h: i32, color: u32) {
    out.elements.push(HudElement {
        name: name.to_owned(),
        x,
        y,
        w,
        h,
        icon: None,
        color,
        blend: Blend17::Alpha,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn element<'a>(layout: &'a HudLayout17, name: &str) -> &'a HudElement {
        layout
            .elements
            .iter()
            .find(|element| element.name == name)
            .unwrap_or_else(|| panic!("no HUD element {name}"))
    }

    fn text<'a>(layout: &'a HudLayout17, name: &str) -> &'a HudText {
        layout
            .texts
            .iter()
            .find(|text| text.name == name)
            .unwrap_or_else(|| panic!("no HUD text {name}"))
    }

    fn six_per_character(text: &str) -> i32 {
        text.chars().count() as i32 * 6
    }

    fn layout(player: &HudPlayer17, state: &HudState17<'_>) -> HudLayout17 {
        Hud17::new(427, 240).layout(player, state, &six_per_character)
    }

    #[test]
    fn heart_cells_match_ingame_hud() {
        // GameGui.renderStatusBars: containers at 537, content at 540-562;
        // var36 is 16/52/88 for normal/poison/wither, and var28 is 5 (v=45)
        // on a hardcore world.
        for hardcore in [false, true] {
            let v = if hardcore { 45 } else { 0 };
            let rows = [
                (HeartEffect::Normal, 52, 61, 70, 79, 160, 169),
                (HeartEffect::Poison, 88, 97, 106, 115, 196, 205),
                (HeartEffect::Wither, 124, 133, 142, 151, 232, 241),
            ];
            for (effect, full, half, full_blink, half_blink, absorption_half, absorption_full) in
                rows
            {
                let cases = [
                    (HeartKind::Full, full),
                    (HeartKind::Half, half),
                    (HeartKind::FullBlink, full_blink),
                    (HeartKind::HalfBlink, half_blink),
                    (HeartKind::AbsorptionHalf, absorption_half),
                    (HeartKind::AbsorptionFull, absorption_full),
                ];
                for (kind, u) in cases {
                    let icon = heart_icon(effect, hardcore, kind);
                    assert_eq!(
                        (icon.texture, icon.u, icon.v, icon.w, icon.h),
                        (ICONS, u, v, 9, 9)
                    );
                }
            }
            assert_eq!(
                heart_container(false, hardcore),
                Icon::at(ICONS, 16, v, 9, 9)
            );
            assert_eq!(
                heart_container(true, hardcore),
                Icon::at(ICONS, 25, v, 9, 9)
            );
        }
    }

    #[test]
    fn food_armor_air_and_bar_cells_match_ingame_hud() {
        // GameGui.renderStatusBars: food container at 589, content at
        // 601-605, hunger sets var43 = 52 and var46 = 13; armour at 493-501;
        // air at 654-656.
        for hungry in [false, true] {
            let (container, full, half, full_blink, half_blink) = if hungry {
                (133, 88, 97, 106, 115)
            } else {
                (16, 52, 61, 70, 79)
            };
            assert_eq!(
                (if hungry {
                    FOOD_CONTAINER_HUNGER
                } else {
                    FOOD_CONTAINER
                })
                .u,
                container
            );
            for (kind, u) in [
                (FoodKind::Full, full),
                (FoodKind::Half, half),
                (FoodKind::FullBlink, full_blink),
                (FoodKind::HalfBlink, half_blink),
            ] {
                let icon = food_icon(hungry, kind);
                assert_eq!(
                    (icon.texture, icon.u, icon.v, icon.w, icon.h),
                    (ICONS, u, 27, 9, 9)
                );
            }
        }
        assert_eq!(FOOD_CONTAINER_LOW, Icon::at(ICONS, 25, 27, 9, 9));
        assert_eq!(ARMOR_FULL, Icon::at(ICONS, 34, 9, 9, 9));
        assert_eq!(ARMOR_HALF, Icon::at(ICONS, 25, 9, 9, 9));
        assert_eq!(ARMOR_EMPTY, Icon::at(ICONS, 16, 9, 9, 9));
        assert_eq!(AIR_FULL, Icon::at(ICONS, 16, 18, 9, 9));
        assert_eq!(AIR_PARTIAL, Icon::at(ICONS, 25, 18, 9, 9));
        // GameGui.render:169-171 and 103, 98-99.
        assert_eq!(XP_BACKGROUND, Icon::at(ICONS, 0, 64, 182, 5));
        assert_eq!(XP_FILL, Icon::at(ICONS, 0, 69, 182, 5));
        assert_eq!(CROSSHAIR, Icon::at(ICONS, 0, 0, 16, 16));
        assert_eq!(HOTBAR, Icon::at(WIDGETS, 0, 0, 182, 22));
        assert_eq!(HOTBAR_SELECTION, Icon::at(WIDGETS, 0, 22, 24, 22));
    }

    #[test]
    fn cells_map_into_the_fixed_256_coordinate_space() {
        // The era draws every GUI cell over a 256x256 space whatever the
        // texture's real size (GuiElement.java:105-114).
        let source = Rect {
            u0: 0.5,
            v0: 0.25,
            u1: 0.75,
            v1: 0.75,
            w: 256,
            h: 256,
        };
        let cell = cell_rect(source, 0, 64, 182, 5);
        assert_eq!((cell.u0, cell.v0), (0.5, 0.25 + 0.5 * 64.0 / 256.0));
        assert_eq!(
            (cell.u1, cell.v1),
            (
                0.5 + 0.25 * 182.0 / 256.0,
                0.25 + 0.5 * 64.0 / 256.0 + 0.5 * 5.0 / 256.0
            )
        );
        assert_eq!((cell.w, cell.h), (182, 5));
    }

    #[test]
    fn a_cell_that_crosses_the_sheet_edge_wraps_inside_its_texture() {
        // The era uploads GUI textures with GL_REPEAT, so the furnace's full
        // flame (v = -1) reads the sheet's last row before row 0.
        let (parts, count) = axis_parts(-1, 14);
        assert_eq!(count, 2);
        assert_eq!(&parts[..count], &[(255, 1, 0), (0, 13, 1)]);
        let (parts, count) = axis_parts(250, 10);
        assert_eq!(count, 2);
        assert_eq!(&parts[..count], &[(250, 6, 0), (0, 4, 6)]);
        let (parts, count) = axis_parts(176, 14);
        assert_eq!(count, 1);
        assert_eq!(parts[0], (176, 14, 0));
    }

    #[test]
    fn hotbar_matches_ingame_hud() {
        let state = HudState17 {
            selected_slot: 3,
            ..HudState17::default()
        };
        let layout = layout(&HudPlayer17::default(), &state);
        let bar = element(&layout, "hotbar");
        assert_eq!((bar.x, bar.y, bar.w, bar.h), (122, 218, 182, 22));
        assert_eq!(bar.icon, Some(HOTBAR));
        let selection = element(&layout, "hotbar_selection");
        assert_eq!(
            (selection.x, selection.y, selection.w, selection.h),
            (181, 217, 24, 22)
        );
        assert_eq!(selection.icon, Some(HOTBAR_SELECTION));
        // GameGui.render:118: width / 2 - 90 + slot * 20 + 2, height - 16 - 3.
        assert_eq!(layout.hotbar_slots[0], (125, 221));
        assert_eq!(layout.hotbar_slots[3], (185, 221));
        assert_eq!(layout.hotbar_slots[8], (285, 221));
    }

    #[test]
    fn full_health_bars_match_ingame_hud() {
        let player = HudPlayer17 {
            armor: 15,
            food: 10,
            ..HudPlayer17::default()
        };
        let layout = layout(&player, &HudState17::default());
        // The hearts start at the right edge and walk left; the first one is
        // heart 9 at left + 9 * 8, y = height - 39.
        assert_eq!(
            (
                element(&layout, "heart_9_container").x,
                element(&layout, "heart_9_container").y
            ),
            (194, 201)
        );
        assert_eq!(
            element(&layout, "heart_9_full").icon,
            Some(heart_icon(HeartEffect::Normal, false, HeartKind::Full))
        );
        assert_eq!(element(&layout, "heart_0_full").x, 122);
        // Armour 15: seven full, one half, two empty.
        for slot in 0..7 {
            assert_eq!(
                element(&layout, &format!("armor_{slot}")).icon,
                Some(ARMOR_FULL)
            );
        }
        assert_eq!(element(&layout, "armor_7").icon, Some(ARMOR_HALF));
        assert_eq!(element(&layout, "armor_8").icon, Some(ARMOR_EMPTY));
        assert_eq!(element(&layout, "armor_9").icon, Some(ARMOR_EMPTY));
        assert_eq!(
            (element(&layout, "armor_0").x, element(&layout, "armor_0").y),
            (122, 191)
        );
        // Hunger 10: five full shanks, no halves, drawn from the right.
        assert_eq!(
            (
                element(&layout, "food_0_container").x,
                element(&layout, "food_0_container").y
            ),
            (295, 201)
        );
        assert_eq!(
            element(&layout, "food_0_full").icon,
            Some(food_icon(false, FoodKind::Full))
        );
        assert_eq!(
            element(&layout, "food_4_full").icon,
            Some(food_icon(false, FoodKind::Full))
        );
        assert!(
            layout
                .elements
                .iter()
                .all(|element| element.name != "food_5_full")
        );
    }

    #[test]
    fn half_heart_matches_ingame_hud() {
        let player = HudPlayer17 {
            health: 17.0,
            prev_health: 17,
            armor: 15,
            ..HudPlayer17::default()
        };
        let layout = layout(&player, &HudState17::default());
        // 17 health: hearts 0-7 full (2h+1 < 17), heart 8 half, heart 9 empty.
        for heart in 0..8 {
            assert_eq!(
                element(&layout, &format!("heart_{heart}_full")).icon,
                Some(heart_icon(HeartEffect::Normal, false, HeartKind::Full))
            );
        }
        assert_eq!(
            element(&layout, "heart_8_half").icon,
            Some(heart_icon(HeartEffect::Normal, false, HeartKind::Half))
        );
        assert!(
            layout
                .elements
                .iter()
                .all(|element| element.name != "heart_9_full")
        );
        assert!(
            layout
                .elements
                .iter()
                .all(|element| element.name != "heart_9_half")
        );
    }

    #[test]
    fn critical_health_flashes_and_jitters_like_java_random() {
        let player = HudPlayer17 {
            health: 3.0,
            prev_health: 20,
            invulnerable_timer: 15,
            ..HudPlayer17::default()
        };
        let state = HudState17 {
            ticks: 7,
            ..HudState17::default()
        };
        let layout = layout(&player, &state);
        // Seed 7 * 312871, ten nextInt(2) draws: 1 1 1 0 0 1 1 0 1 1,
        // consumed from heart 9 down to heart 0.
        assert_eq!(
            (
                element(&layout, "heart_9_container").x,
                element(&layout, "heart_9_container").y
            ),
            (194, 202)
        );
        assert_eq!(
            (
                element(&layout, "heart_6_container").x,
                element(&layout, "heart_6_container").y
            ),
            (170, 201)
        );
        assert_eq!(
            (
                element(&layout, "heart_0_container").x,
                element(&layout, "heart_0_container").y
            ),
            (122, 202)
        );
        // The hurt flash draws the white-backed container and the previous
        // health's content (20/20 is ten full flashing hearts).
        assert_eq!(
            element(&layout, "heart_9_container").icon,
            Some(heart_container(true, false))
        );
        assert_eq!(
            element(&layout, "heart_9_flash_full").icon,
            Some(heart_icon(HeartEffect::Normal, false, HeartKind::FullBlink))
        );
        assert_eq!(
            element(&layout, "heart_1_flash_full").icon,
            Some(heart_icon(HeartEffect::Normal, false, HeartKind::FullBlink))
        );
        // 3 health: heart 0 full, heart 1 half, hearts 2-9 empty hurt flashes.
        assert_eq!(
            element(&layout, "heart_0_full").icon,
            Some(heart_icon(HeartEffect::Normal, false, HeartKind::Full))
        );
        assert_eq!(
            element(&layout, "heart_1_half").icon,
            Some(heart_icon(HeartEffect::Normal, false, HeartKind::Half))
        );
        assert!(
            layout
                .elements
                .iter()
                .all(|element| element.name != "heart_2_full")
        );
        assert!(
            layout
                .elements
                .iter()
                .all(|element| element.name != "heart_2_half")
        );
    }

    #[test]
    fn absorption_spans_two_rows_and_uses_the_golden_cells() {
        let player = HudPlayer17 {
            absorption: 3.0,
            armor: 15,
            ..HudPlayer17::default()
        };
        let layout = layout(&player, &HudState17::default());
        // ceil((20 + 3) / 2) = 12 hearts, two rows of spacing 10; the armour
        // row and the first bubble row sit 10 pixels above the top heart row.
        assert_eq!(element(&layout, "armor_0").y, 181);
        assert_eq!(element(&layout, "heart_11_container").y, 191);
        assert_eq!(element(&layout, "heart_10_container").y, 191);
        assert_eq!(element(&layout, "heart_9_container").y, 201);
        // An odd absorption draws the top heart full and the next one half.
        assert_eq!(
            element(&layout, "heart_11_absorption").icon,
            Some(heart_icon(
                HeartEffect::Normal,
                false,
                HeartKind::AbsorptionFull
            ))
        );
        assert_eq!(
            element(&layout, "heart_10_absorption").icon,
            Some(heart_icon(
                HeartEffect::Normal,
                false,
                HeartKind::AbsorptionHalf
            ))
        );
        assert!(
            layout
                .elements
                .iter()
                .all(|element| element.name != "heart_11_full")
        );
        assert!(
            layout
                .elements
                .iter()
                .all(|element| element.name != "heart_11_half")
        );
    }

    #[test]
    fn hunger_effect_tints_the_food_and_regeneration_lifts_one_heart() {
        let player = HudPlayer17 {
            food: 10,
            hungry: true,
            regeneration: true,
            ..HudPlayer17::default()
        };
        let state = HudState17 {
            ticks: 5,
            ..HudState17::default()
        };
        let layout = layout(&player, &state);
        assert_eq!(
            element(&layout, "food_0_container").icon,
            Some(FOOD_CONTAINER_HUNGER)
        );
        assert_eq!(
            element(&layout, "food_0_full").icon,
            Some(food_icon(true, FoodKind::Full))
        );
        // ticks % ceil(20 + 5) = 5 lifts heart 5 by two pixels.
        assert_eq!(element(&layout, "heart_5_container").y, 199);
        assert_eq!(element(&layout, "heart_4_container").y, 201);
    }

    #[test]
    fn hardcore_hearts_use_the_second_icon_row() {
        let player = HudPlayer17 {
            hardcore: true,
            ..HudPlayer17::default()
        };
        let layout = layout(&player, &HudState17::default());
        assert_eq!(
            element(&layout, "heart_0_container").icon,
            Some(heart_container(false, true))
        );
        assert_eq!(
            element(&layout, "heart_0_full").icon,
            Some(heart_icon(HeartEffect::Normal, true, HeartKind::Full))
        );
    }

    #[test]
    fn air_and_experience_match_ingame_hud() {
        let player = HudPlayer17 {
            submerged: true,
            breath: 210,
            xp_progress: 0.5,
            xp_level: 27,
            ..HudPlayer17::default()
        };
        let state = HudState17 {
            held_item: Some("Diamond Pickaxe"),
            held_item_timer: 40,
            ..HudState17::default()
        };
        let layout = layout(&player, &state);
        // 7 bubbles for 210 breath: ceil(208 * 10 / 300) = 7 full.
        for bubble in 0..7 {
            assert_eq!(
                element(&layout, &format!("air_{bubble}_full")).icon,
                Some(AIR_FULL)
            );
        }
        assert!(
            layout
                .elements
                .iter()
                .all(|element| element.name != "air_7_full")
        );
        // GameGui.render:168-171: 182x5 at width / 2 - 91, height - 32 + 3.
        let background = element(&layout, "xp_background");
        assert_eq!(
            (background.x, background.y, background.w, background.h),
            (122, 211, 182, 5)
        );
        assert_eq!(background.icon, Some(XP_BACKGROUND));
        let fill = element(&layout, "xp_fill");
        assert_eq!((fill.x, fill.y, fill.w, fill.h), (122, 211, 91, 5));
        assert_eq!(fill.icon, Some(Icon::at(ICONS, 0, 69, 91, 5)));
        // GameGui.render:180-187: the level is centered at height - 31 - 4,
        // in the 0x80FF20 green with a black four-offset outline.
        let level = text(&layout, "xp_level");
        assert_eq!((level.x, level.y), ((427 - 12) / 2, 205));
        assert_eq!(level.color, rgba(0x80, 0xff, 0x20, 0xff));
        assert_eq!(level.shadow, TextShadow17::Outline);
        // GameGui.render:195-212: the held item's name at height - 59, fully
        // opaque at timer 40.
        let name = text(&layout, "held_item_name");
        assert_eq!((name.x, name.y), ((427 - 90) / 2, 181));
        assert_eq!(name.color, rgba(0xff, 0xff, 0xff, 0xff));
        assert_eq!(name.shadow, TextShadow17::Drop);
    }

    #[test]
    fn held_item_name_fades_and_moves_without_status_bars() {
        let state = HudState17 {
            show_status_bars: false,
            held_item: Some("Diamond Pickaxe"),
            held_item_timer: 5,
            ..HudState17::default()
        };
        let layout = layout(&HudPlayer17::default(), &state);
        let name = text(&layout, "held_item_name");
        // (int)(5 * 256 / 10) = 128, and the popup sits 14 pixels lower
        // without status bars (GameGui.render:199-200).
        assert_eq!(name.color, rgba(0xff, 0xff, 0xff, 128));
        assert_eq!(name.y, 195);
    }

    #[test]
    fn durability_bar_matches_item_renderer() {
        let state = HudState17 {
            hotbar_items: [
                Some(HotbarItem17 {
                    damage: 1200,
                    max_damage: 1561,
                }),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            ],
            ..HudState17::default()
        };
        let layout = layout(&HudPlayer17::default(), &state);
        // ItemRenderer.renderGuiItemDecoration: at the item origin + (2, 13),
        // a 13x2 black backing, a 12x1 track and a fill of round(13 - damage *
        // 13 / maxDamage) with round(255 - damage * 255 / maxDamage) green.
        let backing = element(&layout, "durability_0_backing");
        assert_eq!(
            (backing.x, backing.y, backing.w, backing.h),
            (127, 234, 13, 2)
        );
        assert_eq!(backing.color, rgba(0, 0, 0, 0xff));
        let track = element(&layout, "durability_0_track");
        assert_eq!((track.x, track.y, track.w, track.h), (127, 234, 12, 1));
        assert_eq!(track.color, rgba(49, 0x3f, 0, 0xff));
        let fill = element(&layout, "durability_0_fill");
        assert_eq!((fill.x, fill.y, fill.w, fill.h), (127, 234, 3, 1));
        assert_eq!(fill.color, rgba(196, 59, 0, 0xff));
    }

    #[test]
    fn empty_saturation_jitters_the_food_after_the_hearts() {
        // Full health makes no heart draw, so the first nextInt(3) is the
        // first food slot's (seed 0: -1 0 0 1 1 1 1 -1 -1 1).
        let player = HudPlayer17 {
            food: 10,
            saturation: 0.0,
            ..HudPlayer17::default()
        };
        let layout = layout(&player, &HudState17::default());
        assert_eq!(element(&layout, "food_0_container").y, 200);
        assert_eq!(element(&layout, "food_3_container").y, 202);
    }

    #[test]
    fn layout_uses_the_gui_scaled_size() {
        let batch = HudBatch::scratch([854, 480]);
        assert_eq!((batch.width, batch.height), (427.0, 240.0));
        let hud = Hud17::from_batch(&batch);
        assert_eq!(hud, Hud17::new(427, 240));
        // 320x240 is GUI scale 1: the bars stay the same 182 pixels wide but
        // move with the smaller scaled view.
        let narrow = Hud17::new(320, 240);
        let layout = narrow.layout(
            &HudPlayer17::default(),
            &HudState17::default(),
            &six_per_character,
        );
        assert_eq!(element(&layout, "hotbar").x, 69);
        assert_eq!(element(&layout, "food_0_container").x, 251 - 9);
        assert_eq!(element(&layout, "crosshair").x, 153);
    }
}
