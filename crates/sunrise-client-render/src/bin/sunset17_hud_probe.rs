//! Prints the 1.7.10 HUD and container-screen layouts as data, so a reviewer
//! can compare every rectangle and sprite cell with the era's own screens.
//!
//! Usage: `sunset17_hud_probe [ROOT]` (`SUNSET_1_7_CONTENT` or
//! `.local/sunrise-client/1.7.10` by default). The root's prepared texture and
//! language files measure text the way the era's `TextRenderer` does and
//! resolve the panels' translation keys; without them the probe still prints
//! the layout, measuring text as six pixels per character.

use std::path::PathBuf;
use sunrise_client_core::lang::Language;
use sunrise_client_render::{
    hud::HudAtlas,
    hud_1_7_10::{self, HotbarItem17, Hud17, HudPlayer17, HudState17},
    resource_source::ResourceSource,
    screens_1_7_10::{
        Align17, FurnaceProgress17, LabelText17, MenuLayout17, MenuView17, Screen17, SlotSource17,
    },
    sunset17_assets,
};

const GUI_WIDTH: i32 = 427;
const GUI_HEIGHT: i32 = 240;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("SUNSET_1_7_CONTENT").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from(sunset17_assets::DEFAULT_DIR));
    let source = ResourceSource::directory(root.clone());
    let atlas = (sunset17_assets::detect(&root) || sunset17_assets::is_source(&source))
        .then(|| HudAtlas::build(&source))
        .transpose()?;
    let language = sunset17_assets::Assets::load(&root)
        .ok()
        .and_then(|assets| assets.language().ok());

    println!("1.7.10 HUD and container screens (GameGui, the era's container classes)");
    println!("GUI size {GUI_WIDTH}x{GUI_HEIGHT} (a 854x480 window at GUI scale 2)");
    match &atlas {
        Some(atlas) => {
            print_sprite(atlas, "gui/icons");
            print_sprite(atlas, "gui/widgets");
            for screen in [
                Screen17::PlayerInventory,
                Screen17::CraftingTable,
                Screen17::Furnace,
            ] {
                print_sprite(atlas, screen.background().texture);
            }
            println!("text widths: HudAtlas font from {}", root.display());
        }
        None => {
            println!(
                "no 1.7.10 atlas at {}: text x falls back to 6 px per character",
                root.display()
            );
        }
    }
    let text_width = |text: &str| {
        atlas
            .as_ref()
            .map_or(text.chars().count() as i32 * 6, |atlas| {
                atlas.font.width(text) as i32
            })
    };

    let hud = Hud17::new(GUI_WIDTH, GUI_HEIGHT);
    for case in cases() {
        print_hud(&hud, &case, &text_width);
    }
    for screen in [
        Screen17::PlayerInventory,
        Screen17::CraftingTable,
        Screen17::Furnace,
    ] {
        let mut view = MenuView17::new(GUI_WIDTH, GUI_HEIGHT);
        if screen == Screen17::Furnace {
            view.furnace = FurnaceProgress17::from_ticks(200, 200, 100);
        }
        let menu = screen.layout(&view);
        print_menu(&menu, language.as_ref());
    }
    Ok(())
}

struct Case {
    name: &'static str,
    player: HudPlayer17,
    state: HudState17<'static>,
}

fn base_player() -> HudPlayer17 {
    HudPlayer17 {
        armor: 15,
        food: 10,
        saturation: 5.0,
        breath: 210,
        submerged: true,
        xp_progress: 0.5,
        xp_level: 27,
        ..HudPlayer17::default()
    }
}

fn base_state() -> HudState17<'static> {
    HudState17 {
        held_item: Some("Diamond Pickaxe"),
        held_item_timer: 40,
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
    }
}

fn cases() -> Vec<Case> {
    let requested = [
        ("health 20/20", 20.0, 20),
        ("health 17/20", 17.0, 17),
        ("health 3/20 (hurt flash)", 3.0, 20),
    ];
    let mut cases: Vec<Case> = requested
        .into_iter()
        .map(|(name, health, prev_health)| {
            let mut player = base_player();
            player.health = health;
            player.prev_health = prev_health;
            let mut state = base_state();
            if health < 4.0 {
                // A live hurt timer: the flash and the low-health jitter.
                player.invulnerable_timer = 15;
                state.ticks = 7;
            }
            Case {
                name,
                player,
                state,
            }
        })
        .collect();

    let variants: [(&'static str, fn(&mut HudPlayer17)); 6] = [
        ("absorption 3 (golden apple)", |player| {
            player.absorption = 3.0;
        }),
        ("poisoned", |player| player.poisoned = true),
        ("withered", |player| player.withered = true),
        ("hunger effect", |player| player.hungry = true),
        ("hardcore", |player| player.hardcore = true),
        ("empty hunger (jitter)", |player| {
            player.food = 0;
            player.saturation = 0.0;
        }),
    ];
    for (name, change) in variants {
        let mut player = base_player();
        change(&mut player);
        cases.push(Case {
            name,
            player,
            state: base_state(),
        });
    }
    cases
}

fn print_sprite(atlas: &HudAtlas, name: &str) {
    match atlas.sprites.get(name) {
        Some(rect) => println!("sprite {name:<26} {}x{} GUI px", rect.w, rect.h),
        None => println!("sprite {name:<26} missing from the atlas"),
    }
}

fn print_hud(hud: &Hud17, case: &Case, text_width: &dyn Fn(&str) -> i32) {
    let player = &case.player;
    println!("\n== {} ==", case.name);
    println!(
        "player health {}/{} prev {} absorption {} armour {} food {} saturation {} breath {} (air {}) xp level {} progress {:.2}",
        player.health,
        player.max_health,
        player.prev_health,
        player.absorption,
        player.armor,
        player.food,
        player.saturation,
        player.breath,
        air_bubbles(player),
        player.xp_level,
        player.xp_progress
    );
    let layout = hud.layout(player, &case.state, text_width);
    println!("  elements ({}):", layout.elements.len());
    for element in &layout.elements {
        let blend = if element.blend == hud_1_7_10::Blend17::Invert {
            " invert"
        } else {
            ""
        };
        match element.icon {
            Some(icon) => println!(
                "    {:<27} x={:4} y={:4} w={:3} h={:3}  {:<18} u={:3} v={:3} cell={}x{}{blend}",
                element.name,
                element.x,
                element.y,
                element.w,
                element.h,
                icon.texture,
                icon.u,
                icon.v,
                icon.w,
                icon.h
            ),
            None => println!(
                "    {:<27} x={:4} y={:4} w={:3} h={:3}  fill {}",
                element.name,
                element.x,
                element.y,
                element.w,
                element.h,
                colour(element.color)
            ),
        }
    }
    println!("  texts ({}):", layout.texts.len());
    for text in &layout.texts {
        println!(
            "    {:<27} {:?} at ({}, {}) colour {} {:?}",
            text.name,
            text.text,
            text.x,
            text.y,
            colour(text.color),
            text.shadow
        );
    }
    print!("  hotbar item origins:");
    for (x, y) in layout.hotbar_slots {
        print!(" ({x},{y})");
    }
    println!();
}

fn air_bubbles(player: &HudPlayer17) -> i32 {
    let full = (f64::from(player.breath - 2) * 10.0 / 300.0).ceil() as i32;
    let partial = (f64::from(player.breath) * 10.0 / 300.0).ceil() as i32 - full;
    (full + partial).max(0)
}

/// The HUD batch's packed RGBA colour, in the order a player reads it.
fn colour(value: u32) -> String {
    format!(
        "rgba({},{},{},{})",
        value & 0xff,
        (value >> 8) & 0xff,
        (value >> 16) & 0xff,
        value >> 24
    )
}

fn print_menu(menu: &MenuLayout17, language: Option<&Language>) {
    let (ox, oy) = menu.origin;
    println!(
        "\n== screen {:?} ==\n  panel {}x{} at ({},{}) sprite {}",
        menu.screen, menu.panel.w, menu.panel.h, ox, oy, menu.panel.texture
    );
    println!("  labels ({}):", menu.labels.len());
    for label in &menu.labels {
        let (source, resolved) = match &label.text {
            LabelText17::Key(key) => {
                let key: &'static str = key;
                (
                    format!("key {key}"),
                    language
                        .and_then(|language| language.get(key))
                        .unwrap_or(key)
                        .to_owned(),
                )
            }
            LabelText17::Literal(text) => ("literal".to_owned(), text.clone()),
        };
        let anchor = match label.align {
            Align17::Left => "left",
            Align17::Center => "centre",
        };
        println!(
            "    {:<10} {source:<26} = {:?} {anchor} ({},{})",
            label.name, resolved, label.x, label.y
        );
    }
    if !menu.progress.is_empty() {
        println!("  progress ({}):", menu.progress.len());
        for progress in &menu.progress {
            println!(
                "    {:<10} local=({:3},{:3}) abs=({:3},{:3})  {} u={} v={} {}x{}",
                progress.name,
                progress.x,
                progress.y,
                ox + progress.x,
                oy + progress.y,
                progress.icon.texture,
                progress.icon.u,
                progress.icon.v,
                progress.icon.w,
                progress.icon.h
            );
        }
    }
    println!("  slots ({}):", menu.slots.len());
    for slot in &menu.slots {
        let source = match slot.source {
            SlotSource17::Player(index) => format!("player[{index}]"),
            SlotSource17::Crafting(index) => format!("crafting[{index}]"),
            SlotSource17::Result => "result".to_owned(),
            SlotSource17::Furnace(index) => format!("furnace[{index}]"),
        };
        println!(
            "    [{:2}] {:<14} local=({:3},{:3}) abs=({:3},{:3}) {}x{} source={source}",
            slot.menu_index,
            slot.name,
            slot.x,
            slot.y,
            ox + slot.x,
            oy + slot.y,
            slot.w,
            slot.h
        );
    }
}
