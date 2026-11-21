//! Minecraft 1.7.10's three container screens: the player inventory, the
//! crafting table and the furnace.
//!
//! The panels, labels, slot boxes and furnace progress are read from the era's
//! client classes: `InventoryMenuScreen` (176x166, the inventory texture,
//! `net/minecraft/client/gui/screen/inventory/menu/InventoryMenuScreen.java:22-24`),
//! `SurvivalInventoryScreen` (title at 86,16, line 42),
//! `CraftingTableScreen` (title at 28,6 and the inventory label at 8,72, lines
//! 19-20) and `FurnaceScreen` (title centered at 88,6, the flame and arrow at
//! 56,36/79,34, lines 21-37). The slot coordinates are the era's container
//! classes, `PlayerMenu`/`CraftingTableMenu`/`FurnaceMenu` in the server tree.
//!
//! The panel's `x`/`y` is `(width - 176) / 2, (height - 166) / 2` in GUI
//! pixels, as `InventoryMenuScreen.init:60-61`; every label and slot below is
//! relative to that origin, the way the era's container classes add slots.
//! Items, tooltips and the player preview are the caller's (out of scope here:
//! item rendering and the inventory's entity model).

use crate::{
    hud::{HudAtlas, HudBatch, rgba},
    hud_1_7_10::Icon,
};

/// `InventoryMenuScreen.backgroundWidth`.
pub const PANEL_WIDTH: i32 = 176;
/// `InventoryMenuScreen.backgroundHeight`.
pub const PANEL_HEIGHT: i32 = 166;
/// The 16x16 box a slot occupies (`InventoryMenuScreen.render`'s hover fill,
/// line 96).
pub const SLOT_SIZE: i32 = 16;
/// `TextRenderer.draw(..., 4210752)` for a container's labels.
pub const LABEL_COLOR: u32 = rgba(0x40, 0x40, 0x40, 0xff);

/// Which of the era's three container screens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen17 {
    PlayerInventory,
    CraftingTable,
    Furnace,
}

impl Screen17 {
    /// The panel sprite and the era's file under `textures/gui/container/`.
    pub fn background(self) -> Icon {
        let texture = match self {
            Self::PlayerInventory => "gui/container/inventory",
            Self::CraftingTable => "gui/container/crafting_table",
            Self::Furnace => "gui/container/furnace",
        };
        Icon::at(texture, 0, 0, PANEL_WIDTH, PANEL_HEIGHT)
    }

    /// `(width - backgroundWidth) / 2, (height - backgroundHeight) / 2` in
    /// GUI pixels.
    pub fn origin(self, width: i32, height: i32) -> (i32, i32) {
        ((width - PANEL_WIDTH) / 2, (height - PANEL_HEIGHT) / 2)
    }

    /// The screen's slots in container order, with the coordinates the era's
    /// `addSlot` calls give.
    pub fn slots(self) -> Vec<Slot17> {
        match self {
            Self::PlayerInventory => player_inventory_slots(),
            Self::CraftingTable => crafting_table_slots(),
            Self::Furnace => furnace_slots(),
        }
    }

    /// The panel, labels, furnace progress and slot table at a GUI-scaled
    /// view size.
    pub fn layout(self, view: &MenuView17<'_>) -> MenuLayout17 {
        let origin = self.origin(view.width, view.height);
        let labels = match self {
            Self::PlayerInventory => vec![MenuLabel17 {
                name: "title",
                text: LabelText17::Key("container.crafting"),
                x: 86,
                y: 16,
                align: Align17::Left,
                color: LABEL_COLOR,
            }],
            Self::CraftingTable => vec![
                MenuLabel17 {
                    name: "title",
                    text: LabelText17::Key("container.crafting"),
                    x: 28,
                    y: 6,
                    align: Align17::Left,
                    color: LABEL_COLOR,
                },
                inventory_label(),
            ],
            Self::Furnace => vec![
                MenuLabel17 {
                    name: "title",
                    text: match view.furnace_title {
                        Some(title) => LabelText17::Literal(title.to_owned()),
                        None => LabelText17::Key("container.furnace"),
                    },
                    x: PANEL_WIDTH / 2,
                    y: 6,
                    align: Align17::Center,
                    color: LABEL_COLOR,
                },
                inventory_label(),
            ],
        };
        let mut progress = Vec::new();
        if self == Self::Furnace && view.furnace.burning {
            // `FurnaceScreen.renderMenuBackground:33-38`:
            // (x + 56, y + 36 + 12 - lit) from (176, 12 - lit), 14 x (lit + 1),
            // then (x + 79, y + 34) from (176, 14), (cook + 1) x 16.
            progress.push(MenuProgress17 {
                name: "flame",
                icon: Icon::at(
                    "gui/container/furnace",
                    176,
                    12 - view.furnace.lit,
                    14,
                    view.furnace.lit + 1,
                ),
                x: 56,
                y: 36 + 12 - view.furnace.lit,
            });
            progress.push(MenuProgress17 {
                name: "arrow",
                icon: Icon::at("gui/container/furnace", 176, 14, view.furnace.cook + 1, 16),
                x: 79,
                y: 34,
            });
        }
        MenuLayout17 {
            screen: self,
            origin,
            panel: self.background(),
            labels,
            progress,
            slots: self.slots(),
        }
    }
}

fn inventory_label() -> MenuLabel17 {
    MenuLabel17 {
        name: "inventory",
        text: LabelText17::Key("container.inventory"),
        x: 8,
        y: 72,
        align: Align17::Left,
        color: LABEL_COLOR,
    }
}

/// What a slot reads from, the era's `InventorySlot`'s inventory and index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotSource17 {
    /// `PlayerInventory`; the index the era's slot was constructed with.
    Player(usize),
    /// The screen's `CraftingInventory`.
    Crafting(usize),
    /// The screen's `ResultInventory`.
    Result,
    /// The furnace block entity's inventory.
    Furnace(usize),
}

/// The slot's role, including the armour slot's 0=head..3=feet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotRole17 {
    CraftingResult,
    CraftingGrid,
    Armor(u8),
    MainInventory,
    Hotbar,
    FurnaceInput,
    FurnaceFuel,
    FurnaceResult,
}

/// One container slot: the item's 16x16 origin relative to the panel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Slot17 {
    pub name: String,
    /// The slot's position in `InventoryMenu.slots`.
    pub menu_index: usize,
    pub source: SlotSource17,
    pub role: SlotRole17,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Slot17 {
    fn new(
        name: String,
        menu_index: usize,
        source: SlotSource17,
        role: SlotRole17,
        x: i32,
        y: i32,
    ) -> Self {
        Self {
            name,
            menu_index,
            source,
            role,
            x,
            y,
            w: SLOT_SIZE,
            h: SLOT_SIZE,
        }
    }

    fn push(
        slots: &mut Vec<Self>,
        name: String,
        source: SlotSource17,
        role: SlotRole17,
        x: i32,
        y: i32,
    ) {
        let menu_index = slots.len();
        slots.push(Self::new(name, menu_index, source, role, x, y));
    }
}

/// `PlayerMenu`: result at (144,36), a 2x2 grid at (88,26), four armour slots
/// at (8,8), three rows at (8,84) and the hotbar at (8,142).
fn player_inventory_slots() -> Vec<Slot17> {
    let mut slots = Vec::with_capacity(45);
    Slot17::push(
        &mut slots,
        "result".to_owned(),
        SlotSource17::Result,
        SlotRole17::CraftingResult,
        144,
        36,
    );
    for row in 0..2 {
        for column in 0..2 {
            let index = column + row * 2;
            Slot17::push(
                &mut slots,
                format!("crafting_{index}"),
                SlotSource17::Crafting(index),
                SlotRole17::CraftingGrid,
                88 + column as i32 * 18,
                26 + row as i32 * 18,
            );
        }
    }
    for armor in 0..4 {
        Slot17::push(
            &mut slots,
            ["armor_head", "armor_chest", "armor_legs", "armor_feet"][armor].to_owned(),
            SlotSource17::Player(39 - armor),
            SlotRole17::Armor(armor as u8),
            8,
            8 + armor as i32 * 18,
        );
    }
    push_player_rows(&mut slots);
    slots
}

/// `CraftingTableMenu`: result at (124,35), a 3x3 grid at (30,17), then the
/// player's rows and hotbar.
fn crafting_table_slots() -> Vec<Slot17> {
    let mut slots = Vec::with_capacity(46);
    Slot17::push(
        &mut slots,
        "result".to_owned(),
        SlotSource17::Result,
        SlotRole17::CraftingResult,
        124,
        35,
    );
    for row in 0..3 {
        for column in 0..3 {
            let index = column + row * 3;
            Slot17::push(
                &mut slots,
                format!("crafting_{index}"),
                SlotSource17::Crafting(index),
                SlotRole17::CraftingGrid,
                30 + column as i32 * 18,
                17 + row as i32 * 18,
            );
        }
    }
    push_player_rows(&mut slots);
    slots
}

/// `FurnaceMenu`: input (56,17), fuel (56,53), output (116,35), then the
/// player's rows and hotbar.
fn furnace_slots() -> Vec<Slot17> {
    let mut slots = Vec::with_capacity(39);
    for (name, source, role, x, y) in [
        ("input", 0, SlotRole17::FurnaceInput, 56, 17),
        ("fuel", 1, SlotRole17::FurnaceFuel, 56, 53),
        ("output", 2, SlotRole17::FurnaceResult, 116, 35),
    ] {
        Slot17::push(
            &mut slots,
            name.to_owned(),
            SlotSource17::Furnace(source),
            role,
            x,
            y,
        );
    }
    push_player_rows(&mut slots);
    slots
}

/// The player's 27 main slots and nine hotbar slots, as every container adds
/// them: `PlayerMenu.java:36-45`, `CraftingTableMenu.java:36-45`,
/// `FurnaceMenu.java:23-31`.
fn push_player_rows(slots: &mut Vec<Slot17>) {
    for row in 0..3 {
        for column in 0..9 {
            let inventory = column + (row + 1) * 9;
            Slot17::push(
                slots,
                format!("inventory_{}", column + row * 9),
                SlotSource17::Player(inventory),
                SlotRole17::MainInventory,
                8 + column as i32 * 18,
                84 + row as i32 * 18,
            );
        }
    }
    for slot in 0..9 {
        Slot17::push(
            slots,
            format!("hotbar_{slot}"),
            SlotSource17::Player(slot),
            SlotRole17::Hotbar,
            8 + slot as i32 * 18,
            142,
        );
    }
}

/// The furnace's three `FurnaceBlockEntity` data values, as
/// `FurnaceMenu.updateListeners` tracks them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FurnaceProgress17 {
    /// `FurnaceBlockEntity.hasFuel()`: `fuelTime > 0`.
    pub burning: bool,
    /// `getLitProgress(13)`: `fuelTime * 13 / totalFuelTime`.
    pub lit: i32,
    /// `getCookProgress(24)`: `cookTime * 24 / 200`.
    pub cook: i32,
}

impl FurnaceProgress17 {
    /// The progress `getLitProgress`/`getCookProgress` compute from the block
    /// entity's ticks (`FurnaceBlockEntity.java:142-152`): a zero total fuel
    /// time reads as 200.
    pub fn from_ticks(fuel_time: i32, total_fuel_time: i32, cook_time: i32) -> Self {
        let total = if total_fuel_time == 0 {
            200
        } else {
            total_fuel_time
        };
        Self {
            burning: fuel_time > 0,
            lit: fuel_time * 13 / total,
            cook: cook_time * 24 / 200,
        }
    }
}

/// The GUI-scaled view a screen is laid out in, and the furnace's state and
/// name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MenuView17<'a> {
    pub width: i32,
    pub height: i32,
    pub furnace: FurnaceProgress17,
    pub furnace_title: Option<&'a str>,
}

impl<'a> MenuView17<'a> {
    pub const fn new(width: i32, height: i32) -> Self {
        Self {
            width,
            height,
            furnace: FurnaceProgress17 {
                burning: false,
                lit: 0,
                cook: 0,
            },
            furnace_title: None,
        }
    }
}

/// A label's text: a translation key or a custom (furnace) name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LabelText17 {
    Key(&'static str),
    Literal(String),
}

/// A label's horizontal anchor: its left edge or its horizontal centre.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align17 {
    Left,
    Center,
}

/// One `TextRenderer.draw` of the panel, relative to its origin.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuLabel17 {
    pub name: &'static str,
    pub text: LabelText17,
    pub x: i32,
    pub y: i32,
    pub align: Align17,
    pub color: u32,
}

/// The furnace's flame or arrow, relative to the panel origin.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuProgress17 {
    pub name: &'static str,
    pub icon: Icon,
    pub x: i32,
    pub y: i32,
}

/// One era container screen at a GUI-scaled size.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuLayout17 {
    pub screen: Screen17,
    pub origin: (i32, i32),
    pub panel: Icon,
    pub labels: Vec<MenuLabel17>,
    pub progress: Vec<MenuProgress17>,
    pub slots: Vec<Slot17>,
}

/// Draws the panel, the furnace progress and the labels; slot items and the
/// player preview are the caller's. `resolve` turns a translation key into the
/// era's text, `text_width` is `TextRenderer.getWidth` for centered labels.
pub fn paint(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    menu: &MenuLayout17,
    resolve: &dyn Fn(&str) -> String,
    text_width: &dyn Fn(&str) -> i32,
) {
    let (ox, oy) = menu.origin;
    for part in menu.panel.atlas_parts(atlas) {
        batch.rect_uv(
            (ox + part.dx) as f32,
            (oy + part.dy) as f32,
            part.w as f32,
            part.h as f32,
            part.rect,
            u32::MAX,
        );
    }
    for progress in &menu.progress {
        for part in progress.icon.atlas_parts(atlas) {
            batch.rect_uv(
                (ox + progress.x + part.dx) as f32,
                (oy + progress.y + part.dy) as f32,
                part.w as f32,
                part.h as f32,
                part.rect,
                u32::MAX,
            );
        }
    }
    for label in &menu.labels {
        let text = match &label.text {
            LabelText17::Key(key) => resolve(key),
            LabelText17::Literal(text) => text.clone(),
        };
        let x = match label.align {
            Align17::Left => label.x,
            Align17::Center => label.x - text_width(&text) / 2,
        };
        batch.text(
            atlas,
            &text,
            (ox + x) as f32,
            (oy + label.y) as f32,
            label.color,
            false,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAYER_INVENTORY_SLOTS: [(i32, i32); 45] = [
        (144, 36),
        (88, 26),
        (106, 26),
        (88, 44),
        (106, 44),
        (8, 8),
        (8, 26),
        (8, 44),
        (8, 62),
        (8, 84),
        (26, 84),
        (44, 84),
        (62, 84),
        (80, 84),
        (98, 84),
        (116, 84),
        (134, 84),
        (152, 84),
        (8, 102),
        (26, 102),
        (44, 102),
        (62, 102),
        (80, 102),
        (98, 102),
        (116, 102),
        (134, 102),
        (152, 102),
        (8, 120),
        (26, 120),
        (44, 120),
        (62, 120),
        (80, 120),
        (98, 120),
        (116, 120),
        (134, 120),
        (152, 120),
        (8, 142),
        (26, 142),
        (44, 142),
        (62, 142),
        (80, 142),
        (98, 142),
        (116, 142),
        (134, 142),
        (152, 142),
    ];
    const CRAFTING_TABLE_SLOTS: [(i32, i32); 46] = [
        (124, 35),
        (30, 17),
        (48, 17),
        (66, 17),
        (30, 35),
        (48, 35),
        (66, 35),
        (30, 53),
        (48, 53),
        (66, 53),
        (8, 84),
        (26, 84),
        (44, 84),
        (62, 84),
        (80, 84),
        (98, 84),
        (116, 84),
        (134, 84),
        (152, 84),
        (8, 102),
        (26, 102),
        (44, 102),
        (62, 102),
        (80, 102),
        (98, 102),
        (116, 102),
        (134, 102),
        (152, 102),
        (8, 120),
        (26, 120),
        (44, 120),
        (62, 120),
        (80, 120),
        (98, 120),
        (116, 120),
        (134, 120),
        (152, 120),
        (8, 142),
        (26, 142),
        (44, 142),
        (62, 142),
        (80, 142),
        (98, 142),
        (116, 142),
        (134, 142),
        (152, 142),
    ];
    const FURNACE_SLOTS: [(i32, i32); 39] = [
        (56, 17),
        (56, 53),
        (116, 35),
        (8, 84),
        (26, 84),
        (44, 84),
        (62, 84),
        (80, 84),
        (98, 84),
        (116, 84),
        (134, 84),
        (152, 84),
        (8, 102),
        (26, 102),
        (44, 102),
        (62, 102),
        (80, 102),
        (98, 102),
        (116, 102),
        (134, 102),
        (152, 102),
        (8, 120),
        (26, 120),
        (44, 120),
        (62, 120),
        (80, 120),
        (98, 120),
        (116, 120),
        (134, 120),
        (152, 120),
        (8, 142),
        (26, 142),
        (44, 142),
        (62, 142),
        (80, 142),
        (98, 142),
        (116, 142),
        (134, 142),
        (152, 142),
    ];

    fn layout(screen: Screen17) -> MenuLayout17 {
        screen.layout(&MenuView17::new(427, 240))
    }

    fn slot<'a>(menu: &'a MenuLayout17, name: &str) -> &'a Slot17 {
        menu.slots
            .iter()
            .find(|slot| slot.name == name)
            .unwrap_or_else(|| panic!("no slot {name}"))
    }

    #[test]
    fn player_inventory_slots_match_container_player() {
        // PlayerMenu.java:23-45: result (144,36), 2x2 grid (88/106, 26/44),
        // armour (8, 8..62), rows (8+18c, 84+18r), hotbar (8+18s, 142).
        let menu = layout(Screen17::PlayerInventory);
        let coordinates: Vec<(i32, i32)> = menu.slots.iter().map(|slot| (slot.x, slot.y)).collect();
        assert_eq!(coordinates, PLAYER_INVENTORY_SLOTS);
        assert_eq!(menu.slots.len(), 45);
        assert_eq!(menu.origin, (125, 37));
        assert_eq!(
            menu.panel,
            Icon::at("gui/container/inventory", 0, 0, 176, 166)
        );
        assert_eq!(slot(&menu, "result").menu_index, 0);
        assert_eq!(slot(&menu, "result").source, SlotSource17::Result);
        assert_eq!(slot(&menu, "result").role, SlotRole17::CraftingResult);
        assert_eq!(slot(&menu, "crafting_3").source, SlotSource17::Crafting(3));
        assert_eq!(slot(&menu, "armor_head").source, SlotSource17::Player(39));
        assert_eq!(slot(&menu, "armor_head").role, SlotRole17::Armor(0));
        assert_eq!(slot(&menu, "armor_feet").source, SlotSource17::Player(36));
        assert_eq!(slot(&menu, "armor_feet").role, SlotRole17::Armor(3));
        assert_eq!(slot(&menu, "inventory_0").source, SlotSource17::Player(9));
        assert_eq!(slot(&menu, "inventory_26").source, SlotSource17::Player(35));
        assert_eq!(slot(&menu, "hotbar_8").source, SlotSource17::Player(8));
        assert!(menu.slots.iter().all(|slot| slot.w == 16 && slot.h == 16));
    }

    #[test]
    fn crafting_table_slots_match_container_workbench() {
        // CraftingTableMenu.java:28-45: result (124,35), 3x3 grid at
        // (30/48/66, 17/35/53), the player's rows and hotbar.
        let menu = layout(Screen17::CraftingTable);
        let coordinates: Vec<(i32, i32)> = menu.slots.iter().map(|slot| (slot.x, slot.y)).collect();
        assert_eq!(coordinates, CRAFTING_TABLE_SLOTS);
        assert_eq!(menu.slots.len(), 46);
        assert_eq!(
            menu.panel,
            Icon::at("gui/container/crafting_table", 0, 0, 176, 166)
        );
        assert_eq!(slot(&menu, "result").source, SlotSource17::Result);
        assert_eq!(slot(&menu, "crafting_8").source, SlotSource17::Crafting(8));
        assert_eq!(slot(&menu, "inventory_0").source, SlotSource17::Player(9));
        assert_eq!(slot(&menu, "hotbar_0").source, SlotSource17::Player(0));
    }

    #[test]
    fn furnace_slots_match_container_furnace() {
        // FurnaceMenu.java:19-30: input (56,17), fuel (56,53), output
        // (116,35), then the player's rows and hotbar.
        let menu = layout(Screen17::Furnace);
        let coordinates: Vec<(i32, i32)> = menu.slots.iter().map(|slot| (slot.x, slot.y)).collect();
        assert_eq!(coordinates, FURNACE_SLOTS);
        assert_eq!(menu.slots.len(), 39);
        assert_eq!(
            menu.panel,
            Icon::at("gui/container/furnace", 0, 0, 176, 166)
        );
        assert_eq!(slot(&menu, "input").source, SlotSource17::Furnace(0));
        assert_eq!(slot(&menu, "fuel").source, SlotSource17::Furnace(1));
        assert_eq!(slot(&menu, "output").source, SlotSource17::Furnace(2));
        assert_eq!(slot(&menu, "output").role, SlotRole17::FurnaceResult);
    }

    #[test]
    fn panels_and_labels_match_the_era_screens() {
        let inventory = layout(Screen17::PlayerInventory);
        assert_eq!(inventory.labels.len(), 1);
        assert_eq!(inventory.labels[0].name, "title");
        assert_eq!(
            inventory.labels[0].text,
            LabelText17::Key("container.crafting")
        );
        assert_eq!((inventory.labels[0].x, inventory.labels[0].y), (86, 16));
        assert_eq!(inventory.labels[0].align, Align17::Left);
        assert_eq!(inventory.labels[0].color, LABEL_COLOR);

        let crafting = layout(Screen17::CraftingTable);
        assert_eq!(
            crafting.labels[0].text,
            LabelText17::Key("container.crafting")
        );
        assert_eq!((crafting.labels[0].x, crafting.labels[0].y), (28, 6));
        assert_eq!(
            crafting.labels[1].text,
            LabelText17::Key("container.inventory")
        );
        assert_eq!((crafting.labels[1].x, crafting.labels[1].y), (8, 72));

        let furnace = layout(Screen17::Furnace);
        assert_eq!(
            furnace.labels[0].text,
            LabelText17::Key("container.furnace")
        );
        assert_eq!((furnace.labels[0].x, furnace.labels[0].y), (88, 6));
        assert_eq!(furnace.labels[0].align, Align17::Center);
        assert_eq!(
            furnace.labels[1].text,
            LabelText17::Key("container.inventory")
        );
        assert_eq!((furnace.labels[1].x, furnace.labels[1].y), (8, 72));
    }

    #[test]
    fn furnace_uses_a_custom_name_when_it_has_one() {
        let mut view = MenuView17::new(427, 240);
        view.furnace_title = Some("My Furnace");
        let menu = Screen17::Furnace.layout(&view);
        assert_eq!(
            menu.labels[0].text,
            LabelText17::Literal("My Furnace".to_owned())
        );
    }

    #[test]
    fn furnace_flame_and_arrow_match_furnace_screen() {
        // A full lit flame: getLitProgress(13) = 13, drawn from v = -1 with
        // height 14 (FurnaceScreen.java:34-35). A half-cooked arrow:
        // getCookProgress(24) = 12, 13 x 16 from (176,14).
        let mut view = MenuView17::new(427, 240);
        view.furnace = FurnaceProgress17 {
            burning: true,
            lit: 13,
            cook: 12,
        };
        let menu = Screen17::Furnace.layout(&view);
        assert_eq!(menu.progress.len(), 2);
        assert_eq!(menu.progress[0].name, "flame");
        assert_eq!((menu.progress[0].x, menu.progress[0].y), (56, 35));
        assert_eq!(
            menu.progress[0].icon,
            Icon::at("gui/container/furnace", 176, -1, 14, 14)
        );
        assert_eq!(menu.progress[1].name, "arrow");
        assert_eq!((menu.progress[1].x, menu.progress[1].y), (79, 34));
        assert_eq!(
            menu.progress[1].icon,
            Icon::at("gui/container/furnace", 176, 14, 13, 16)
        );

        // No fuel: neither the flame nor the arrow is drawn.
        let idle = Screen17::Furnace.layout(&MenuView17::new(427, 240));
        assert!(idle.progress.is_empty());
    }

    #[test]
    fn furnace_progress_from_ticks_matches_the_block_entity() {
        // FurnaceBlockEntity.java:142-152: 100/200 fuel is lit 6, 40/200 cook
        // is 4; a zero total fuel time reads as 200.
        let half = FurnaceProgress17::from_ticks(100, 200, 40);
        assert_eq!(
            half,
            FurnaceProgress17 {
                burning: true,
                lit: 6,
                cook: 4
            }
        );
        let rest = FurnaceProgress17::from_ticks(0, 0, 0);
        assert!(!rest.burning);
        assert_eq!((rest.lit, rest.cook), (0, 0));
    }

    #[test]
    fn panels_are_centered_in_gui_pixels() {
        // InventoryMenuScreen.init:60-61 uses the GUI-scaled width and height.
        assert_eq!(Screen17::PlayerInventory.origin(427, 240), (125, 37));
        assert_eq!(Screen17::CraftingTable.origin(427, 240), (125, 37));
        assert_eq!(Screen17::Furnace.origin(320, 240), (72, 37));
    }
}
