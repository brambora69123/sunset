//! Minecraft 1.7.10's HUD and its three container screens, wired to the live
//! game: the state the render crate's painters read
//! ([`sunrise_client::hud_1_7_10`], [`sunrise_client::screens_1_7_10`]), the
//! engine's menu slot each of the era's slots stands for, and the painters'
//! call sites.
//!
//! The content root decides, once per load ([`UiEra`]): a root the era asset
//! loader recognises draws 1.7.10's HUD and screens, every other root keeps
//! 26.3's. The painters' tables are the era's own layout, so the hit
//! rectangles are exactly the drawn slots; only the adapter between an era
//! slot's role and the engine's menu index lives here.

use super::*;
use std::sync::OnceLock;
use sunrise_client::command::args::GameMode;
use sunrise_client::hud_1_7_10::{self, HotbarItem17, Hud17, HudLayout17, HudPlayer17, HudState17};
use sunrise_client::resource_source::ResourceSource;
use sunrise_client::screens_1_7_10::{
    self as era_screens, FurnaceProgress17, MenuView17, Screen17, Slot17, SlotRole17, SlotSource17,
};

/// Whether a frame draws 1.7.10's UI or 26.3's, chosen from the loaded content
/// root by the same marker as the era asset loader.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum UiEra {
    Modern,
    OneSevenTen,
}

/// The UI a content root asks for.
pub(super) fn ui_era(source: &ResourceSource) -> UiEra {
    if hud_1_7_10::era_ui(source) {
        UiEra::OneSevenTen
    } else {
        UiEra::Modern
    }
}

/// `GameGui.prevHealth` and the health the last tick saw: the white hurt
/// flash draws the health the hit took the player from
/// (`LivingEntity.attackEntityFrom` copies it into `prevHealth` before the
/// damage lands).
#[derive(Clone, Copy, Debug)]
pub(super) struct EraHud {
    seen_health: Option<f32>,
    prev_health: i32,
}

impl Default for EraHud {
    fn default() -> Self {
        Self {
            seen_health: None,
            prev_health: 20,
        }
    }
}

impl Game<'_> {
    /// Whether this game's content root is the era's.
    pub(super) fn era_ui(&self) -> bool {
        self.pack.era_ui == UiEra::OneSevenTen
    }

    /// `Hud.tick`'s health memory for the era HUD's hurt flash.
    pub(super) fn tick_era_hud(&mut self) {
        let Some(p) = &self.player else {
            return;
        };
        let health = p.health.health();
        if let Some(seen) = self.era_hud.seen_health
            && health < seen
        {
            self.era_hud.prev_health = seen.ceil() as i32;
        }
        self.era_hud.seen_health = Some(health);
    }

    /// The era container screen the open inventory is, when the content root
    /// is the era's and the menu is one of its three.
    pub(super) fn era_container(&self) -> Option<Screen17> {
        if self.screen != Screen::Inventory
            || !self.era_ui()
            || self.mount_active()
            || self.station.is_some()
            // A mod's own container screen draws and places its slots; the
            // era's panel does not replace it.
            || self.native_screen_shown()
        {
            return None;
        }
        match self.container_kind() {
            Some(BlockEntity::Furnace(f)) => (f.kind
                == sunrise_client::containers::FurnaceKind::Furnace)
                .then_some(Screen17::Furnace),
            Some(_) => None,
            None if self.craft_size == 3 => Some(Screen17::CraftingTable),
            None if self.craft_size == 2 => Some(Screen17::PlayerInventory),
            None => None,
        }
    }

    /// `GameGui.render` over the render crate's era painter: the hotbar with
    /// its items, the crosshair, the status bars, the experience bar and the
    /// held item's name, from the player this frame.
    pub(super) fn draw_era_hud(&mut self, batch: &mut HudBatch) {
        let Some(p) = self.player.as_mut() else {
            return;
        };
        let effects = &self.potions.effects;
        let can_hurt = matches!(self.mode, GameMode::Survival | GameMode::Adventure);
        let riding = self.riding.is_some();
        // Hud.extractAirBubbles: the eyes below the surface, not merely in a
        // water block.
        let submerged = p.is_eye_in_fluid(sunrise_client::motion::WATER);
        let max_health = p.attribute("minecraft:max_health").unwrap_or(20.0) as f32;
        let armor = p
            .attribute("minecraft:armor")
            .or_else(|_| p.attribute("armor"))
            .unwrap_or(0.0) as i32;
        let player = HudPlayer17 {
            health: p.health.health(),
            prev_health: self.era_hud.prev_health,
            max_health,
            absorption: p.health.absorption(),
            armor,
            food: p.health.food.food,
            saturation: p.health.food.saturation,
            breath: p.air,
            xp_progress: self.experience.progress,
            xp_level: self.experience.level,
            riding,
            submerged,
            poisoned: effects.get("poison").is_some(),
            withered: effects.get("wither").is_some(),
            hungry: effects.get("hunger").is_some(),
            regeneration: effects.get("regeneration").is_some(),
            hardcore: self.level.hardcore,
            invulnerable_timer: p.health.hurt.cooldown,
        };
        let hotbar_items: [Option<HotbarItem17>; 9] =
            std::array::from_fn(|slot| self.era_hotbar_item(slot));
        let held_name = self
            .tool_highlight
            .last
            .as_ref()
            .map(|held| held.hover_name.resolve_legacy().plain());
        let state = HudState17 {
            ticks: self.gui_ticks,
            selected_slot: self.selected as i32,
            show_status_bars: can_hurt,
            // GameGui.render: a rideable mount shows its jump bar where the
            // experience bar would be; the era painter has no jump bar.
            show_xp_bar: can_hurt && !riding,
            // GameOptions.itemInHandTooltips, 1.7.10's default.
            show_item_name: true,
            held_item: held_name.as_deref(),
            held_item_timer: self.tool_highlight.timer.clamp(0, 40),
            hotbar_items,
        };
        let hud = Hud17::new(batch.width as i32, batch.height as i32);
        let text_width = |text: &str| self.gui.atlas.font.width(text) as i32;
        let layout = hud.layout(&player, &state, &text_width);
        if let Some(top) = layout
            .elements
            .iter()
            .filter(|element| era_status_element(&element.name))
            .map(|element| element.y)
            .min()
        {
            self.keep_status_top(top, batch.width as i32, batch.height as i32);
        }
        // GameGui.render stacks the hotbar's items over the status bars and
        // under their durability bars, and the experience bar after both.
        let mut over_items = HudLayout17::default();
        let mut sprites = layout.clone();
        sprites
            .elements
            .retain(|element| !element.name.starts_with("durability_"));
        sprites.texts.clear();
        over_items.elements = layout
            .elements
            .iter()
            .filter(|element| element.name.starts_with("durability_"))
            .cloned()
            .collect();
        over_items.texts = layout.texts;
        hud_1_7_10::paint(batch, &self.gui.atlas, &sprites);
        for (slot, (x, y)) in layout.hotbar_slots.iter().enumerate() {
            let Some(stack) = self.inventory[slot].as_ref() else {
                continue;
            };
            self.draw_item_model(batch, stack, *x as f32, *y as f32);
            if stack.count > 1 {
                // ItemRenderer.renderGuiItemDecoration: the count at
                // (x + 19 - 2 - width, y + 6 + 3), with a shadow.
                let count = stack.count.to_string();
                let width = self.gui.atlas.font.width(&count);
                batch.text(
                    &self.gui.atlas,
                    &count,
                    *x as f32 + 17.0 - width,
                    *y as f32 + 9.0,
                    u32::MAX,
                    true,
                );
            }
        }
        hud_1_7_10::paint(batch, &self.gui.atlas, &over_items);
    }

    /// `ItemRenderer.renderGuiItemDecoration`'s damage reading for one hotbar
    /// slot: `isDamaged()` is a damageable stack that has taken damage.
    fn era_hotbar_item(&self, slot: usize) -> Option<HotbarItem17> {
        let stack = self.inventory.get(slot)?.as_ref()?;
        if !sunrise_client::stations::is_damageable(self.items(), stack) {
            return None;
        }
        let info = self.items().get(&stack.item);
        let damage = stack.effective_damage(info);
        (damage > 0).then(|| HotbarItem17 {
            damage,
            max_damage: stack.max_damage(info),
        })
    }

    /// The era panel, its labels and a furnace's flame and arrow, at the
    /// era's own origin (`(width - 176) / 2, (height - 166) / 2`).
    pub(super) fn draw_era_container(&self, batch: &mut HudBatch, screen: Screen17) {
        let mut view = MenuView17::new(batch.width as i32, batch.height as i32);
        let title = (screen == Screen17::Furnace).then(|| {
            self.container_menu_title("container.furnace", "Furnace")
                .plain()
        });
        view.furnace_title = title.as_deref();
        if screen == Screen17::Furnace
            && let Some(BlockEntity::Furnace(f)) = self.container_kind()
        {
            view.furnace =
                FurnaceProgress17::from_ticks(f.lit_remaining, f.lit_total, f.cooking_timer);
        }
        let menu = screen.layout(&view);
        screen_info::name_container(|| {
            title
                .clone()
                .unwrap_or_else(|| self.translate_with_fallback("container.crafting", "Crafting"))
        });
        let resolve = |key: &str| self.translate_with_fallback(key, era_label_fallback(key));
        let text_width = |text: &str| self.gui.atlas.font.width(text) as i32;
        era_screens::paint(batch, &self.gui.atlas, &menu, &resolve, &text_width);
    }

    /// `InventoryMenuScreen.render`'s hovered slot: a half-alpha white fill
    /// over the slot, drawn after its item.
    pub(super) fn draw_era_slot_highlight(&self, batch: &mut HudBatch, x: f32, y: f32) {
        batch.fill(&self.gui.atlas, x, y, 16.0, 16.0, rgba(255, 255, 255, 128));
    }

    /// The era container screen's contents: the slot items at the era's
    /// rectangles, the hovered slot's fill, then the carried stack and the
    /// tooltips. The era has no recipe book, sort button or effect panel next
    /// to these three menus, so 26.3's are not drawn here.
    pub(super) fn draw_era_inventory(&mut self, batch: &mut HudBatch, ox: f32, oy: f32) {
        self.draw_inventory_slots(batch, ox, oy);
        if let Some(slot) = self.slot_at(batch, self.cursor.0, self.cursor.1) {
            let (sx, sy) = self.slot_position(slot);
            self.draw_era_slot_highlight(batch, ox + sx, oy + sy);
        }
        let browser_tip = self.draw_mod_browser(batch);
        self.draw_screen_phase(batch, sunrise_client::screen_draw::RenderPhase::Foreground);
        self.draw_carried(batch);
        self.draw_slot_tooltip(batch);
        self.draw_native_screen_tooltip(batch);
        if let Some(tooltip) = browser_tip {
            self.draw_mod_browser_tooltip(batch, tooltip);
        }
    }

    /// A container screen's title: a server's menu title, else the block's
    /// custom name, else `key`'s text.
    pub(super) fn container_menu_title(
        &self,
        key: &str,
        fallback: &str,
    ) -> sunrise_client::text::Text {
        self.remote_menu_title().unwrap_or_else(|| {
            sunrise_client::text::Text::literal(
                self.container_title(self.translate_with_fallback(key, fallback)),
            )
            .resolve_legacy()
        })
    }
}

/// The era label keys' English text, for a language that lacks them.
fn era_label_fallback(key: &str) -> &'static str {
    match key {
        "container.inventory" => "Inventory",
        "container.furnace" => "Furnace",
        _ => "Crafting",
    }
}

/// The HUD elements that stand for the status rows above the hotbar, which
/// Sunrise's own overlays keep clear of.
fn era_status_element(name: &str) -> bool {
    name.starts_with("heart_")
        || name.starts_with("armor_")
        || name.starts_with("food_")
        || name.starts_with("air_")
}

/// The engine's menu slot an era slot stands for, `Slot17::menu_index` being
/// the era's own container order. The engine keeps the player's hotbar at
/// 0..9, the main rows at 9..36, the crafting grid at 100.., the result at
/// 200, an open container's slots at 300.., and the armour at 500..504; the
/// era's inventory index agrees with the engine's on the hotbar and the main
/// rows.
pub(super) fn era_engine_slot(slot: &Slot17) -> Option<usize> {
    Some(match (slot.role, slot.source) {
        (SlotRole17::CraftingResult, _) => 200,
        (SlotRole17::CraftingGrid, SlotSource17::Crafting(index)) => 100 + index,
        (SlotRole17::Armor(armor), _) => 500 + armor as usize,
        (SlotRole17::MainInventory | SlotRole17::Hotbar, SlotSource17::Player(index)) => index,
        (SlotRole17::FurnaceInput, _) => 300,
        (SlotRole17::FurnaceFuel, _) => 301,
        (SlotRole17::FurnaceResult, _) => 302,
        _ => return None,
    })
}

/// The era slot table as `(engine menu slot, position)` pairs, built once:
/// the painter's table is static, and `slot_position` asks per slot.
fn era_slot_pixels(screen: Screen17) -> &'static [(usize, (f32, f32))] {
    type Tables = [Vec<(usize, (f32, f32))>; 3];
    static TABLES: OnceLock<Tables> = OnceLock::new();
    let tables = TABLES.get_or_init(|| {
        [
            Screen17::PlayerInventory,
            Screen17::CraftingTable,
            Screen17::Furnace,
        ]
        .map(|screen| {
            screen
                .slots()
                .iter()
                .filter_map(|slot| Some((era_engine_slot(slot)?, (slot.x as f32, slot.y as f32))))
                .collect()
        })
    });
    let index = match screen {
        Screen17::PlayerInventory => 0,
        Screen17::CraftingTable => 1,
        Screen17::Furnace => 2,
    };
    &tables[index]
}

/// One menu slot's era position, if the era screen has a slot for it.
pub(super) fn era_slot_position(screen: Screen17, menu_slot: usize) -> Option<(f32, f32)> {
    era_slot_pixels(screen)
        .iter()
        .find_map(|&(slot, at)| (slot == menu_slot).then_some(at))
}
