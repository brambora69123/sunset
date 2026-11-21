//! The old HUD layout, ported from `net.minecraft.client.gui.GameGui`
//! (vanilla 1.7.10 `GuiIngame`): hearts, hunger, armor, air, the experience
//! bar and level, the hotbar, the crosshair and the status-bar jitters.
//!
//! Vanilla draws icon rectangles from `icons.png`; this returns the draw
//! calls so a caller can paint them or a parity harness can compare them.
//! The RNG is `java.util.Random` seeded exactly like vanilla
//! (`updateCounter * 312871`), so the low-health and hunger jitters match.

use sunset_1_7_core::{LegacyRandom, RandomSource};

/// One HUD icon draw: a 9x9 icon rectangle of `icons.png` at a screen spot.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct IconDraw {
    pub x: i32,
    pub y: i32,
    /// Icon atlas position in pixels (vanilla's `drawTexture` u/v).
    pub u: i32,
    pub v: i32,
    pub width: i32,
    pub height: i32,
}

/// What the HUD needs from the player this frame.
#[derive(Clone, Copy, Debug)]
pub struct HudPlayer {
    pub health: f32,
    /// `prevHealth`, the flash value.
    pub prev_health: i32,
    pub max_health: f32,
    pub absorption: f32,
    pub armor: i32,
    pub food: i32,
    pub saturation: f32,
    /// `getBreath()`, 300 = full.
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
    pub invulnerable_timer: i32,
}

impl Default for HudPlayer {
    fn default() -> HudPlayer {
        HudPlayer {
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

/// The laid-out status bars of one frame.
#[derive(Clone, Debug, Default)]
pub struct StatusBars {
    pub armor: Vec<IconDraw>,
    pub hearts: Vec<IconDraw>,
    pub food: Vec<IconDraw>,
    pub air: Vec<IconDraw>,
    /// Left edge and top row of the bars (`width / 2 - 91`, `height - 39`).
    pub left: i32,
    pub top: i32,
}

/// The laid-out hotbar of one frame.
#[derive(Clone, Debug, Default)]
pub struct Hotbar {
    /// The container texture (`widgets.png`) at `width / 2 - 91, height - 22`.
    pub container: IconDraw,
    /// The selection frame over the selected slot.
    pub selection: IconDraw,
    /// One entry per slot: the slot rectangle in screen coordinates.
    pub slots: [(i32, i32); 9],
}

/// The laid-out experience bar of one frame.
#[derive(Clone, Debug, Default)]
pub struct ExperienceBar {
    /// The empty bar, 182x5 at `width / 2 - 91, height - 32 + 3`.
    pub background: IconDraw,
    /// The filled part, `xpProgress * 183` wide.
    pub fill: Option<IconDraw>,
    /// The level text position (centered, `height - 31 - 4`).
    pub level: Option<(i32, i32)>,
}

/// `GameGui` (GuiIngame). `width`/`height` are the scaled GUI size.
pub struct Hud {
    pub width: i32,
    pub height: i32,
    /// `updateCounter`.
    pub ticks: i32,
    random: LegacyRandom,
}

impl Hud {
    pub fn new(width: i32, height: i32) -> Hud {
        Hud {
            width,
            height,
            ticks: 0,
            random: LegacyRandom::new(0),
        }
    }

    /// `renderStatusBars` (GuiIngame.renderStatusBars): armor, hearts, food
    /// and air in the vanilla layout, with the vanilla jitter RNG.
    pub fn status_bars(&mut self, player: &HudPlayer) -> StatusBars {
        let mut bars = StatusBars {
            left: self.width / 2 - 91,
            top: self.height - 39,
            ..StatusBars::default()
        };
        let left = bars.left;
        let right = self.width / 2 + 91;
        let top = self.height - 39;

        // The low-health shake: the hurt-timer flag, computed before the RNG
        // seed exactly like vanilla.
        let mut low_health = player.invulnerable_timer / 3 % 2 == 1;
        if player.invulnerable_timer < 10 {
            low_health = false;
        }
        let health = player.health.ceil() as i32;
        let prev_health = player.prev_health;

        self.random.set_seed((self.ticks * 312871) as i64);
        let food = player.food;

        let rows = ((player.max_health + player.absorption) / 2.0 / 10.0).ceil() as i32;
        let row_spacing = (10 - (rows - 2)).max(3);
        let heart_top = top - (rows - 1) * row_spacing - 10;
        let mut absorption_left = player.absorption;
        let armor = player.armor;
        let mut regeneration_row = -1_i32;
        if player.regeneration {
            regeneration_row = self.ticks % (player.max_health + 5.0).ceil() as i32;
        }

        // Armor, one row above the hearts.
        for slot in 0..10 {
            if armor > 0 {
                let x = left + slot * 8;
                if slot * 2 + 1 < armor {
                    bars.armor.push(IconDraw {
                        x,
                        y: heart_top,
                        u: 34,
                        v: 9,
                        width: 9,
                        height: 9,
                    });
                }
                if slot * 2 + 1 == armor {
                    bars.armor.push(IconDraw {
                        x,
                        y: heart_top,
                        u: 25,
                        v: 9,
                        width: 9,
                        height: 9,
                    });
                }
                if slot * 2 + 1 > armor {
                    bars.armor.push(IconDraw {
                        x,
                        y: heart_top,
                        u: 16,
                        v: 9,
                        width: 9,
                        height: 9,
                    });
                }
            }
        }

        // Hearts, from the top-right heart down.
        let total_hearts = ((player.max_health + player.absorption) / 2.0).ceil() as i32;
        for heart in (0..total_hearts).rev() {
            let mut icon = 16_i32;
            if player.poisoned {
                icon += 36;
            } else if player.withered {
                icon += 72;
            }

            let background = if low_health { 1 } else { 0 };
            let x = left + heart % 10 * 8;
            let mut y = top - ((heart + 1) as f32 / 10.0).ceil() as i32 * row_spacing + row_spacing;
            if health <= 4 {
                y += self.random.next_int_bound(2).unwrap_or(0);
            }
            if heart == regeneration_row {
                y -= 2;
            }

            let v_row = if player.hardcore { 5 * 9 } else { 0 };
            bars.hearts.push(IconDraw {
                x,
                y,
                u: 16 + background * 9,
                v: v_row,
                width: 9,
                height: 9,
            });
            if low_health {
                if heart * 2 + 1 < prev_health {
                    bars.hearts.push(IconDraw {
                        x,
                        y,
                        u: icon + 54,
                        v: v_row + 9,
                        width: 9,
                        height: 9,
                    });
                }
                if heart * 2 + 1 == prev_health {
                    bars.hearts.push(IconDraw {
                        x,
                        y,
                        u: icon + 63,
                        v: v_row + 9,
                        width: 9,
                        height: 9,
                    });
                }
            }

            if absorption_left > 0.0 {
                if absorption_left == player.absorption && player.absorption % 2.0 == 1.0 {
                    bars.hearts.push(IconDraw {
                        x,
                        y,
                        u: icon + 153,
                        v: v_row + 9,
                        width: 9,
                        height: 9,
                    });
                } else {
                    bars.hearts.push(IconDraw {
                        x,
                        y,
                        u: icon + 144,
                        v: v_row + 9,
                        width: 9,
                        height: 9,
                    });
                }
                absorption_left -= 2.0;
            } else {
                if heart * 2 + 1 < health {
                    bars.hearts.push(IconDraw {
                        x,
                        y,
                        u: icon + 36,
                        v: v_row + 9,
                        width: 9,
                        height: 9,
                    });
                }
                if heart * 2 + 1 == health {
                    bars.hearts.push(IconDraw {
                        x,
                        y,
                        u: icon + 45,
                        v: v_row + 9,
                        width: 9,
                        height: 9,
                    });
                }
            }
        }

        // Food, or the mount health while riding.
        if !player.riding {
            for slot in 0..10 {
                let mut y = top;
                let mut icon = 16_i32;
                let mut background = 0_i32;
                if player.hungry {
                    icon += 36;
                    background = 13;
                }
                if player.saturation <= 0.0 && self.ticks % (food * 3 + 1) == 0 {
                    y += self.random.next_int_bound(3).unwrap_or(0) - 1;
                }

                let x = right - slot * 8 - 9;
                bars.food.push(IconDraw {
                    x,
                    y,
                    u: 16 + background * 9,
                    v: 27,
                    width: 9,
                    height: 9,
                });
                if slot * 2 + 1 < food {
                    bars.food.push(IconDraw {
                        x,
                        y,
                        u: icon + 36,
                        v: 27,
                        width: 9,
                        height: 9,
                    });
                }
                if slot * 2 + 1 == food {
                    bars.food.push(IconDraw {
                        x,
                        y,
                        u: icon + 45,
                        v: 27,
                        width: 9,
                        height: 9,
                    });
                }
            }
        }

        // Air bubbles above the food, right to left.
        if player.submerged {
            let breath = player.breath;
            let full = ((breath - 2) as f64 * 10.0 / 300.0).ceil() as i32;
            let partial = (breath as f64 * 10.0 / 300.0).ceil() as i32 - full;
            for bubble in 0..full + partial {
                let x = right - bubble * 8 - 9;
                if bubble < full {
                    bars.air.push(IconDraw {
                        x,
                        y: heart_top,
                        u: 16,
                        v: 18,
                        width: 9,
                        height: 9,
                    });
                } else {
                    bars.air.push(IconDraw {
                        x,
                        y: heart_top,
                        u: 25,
                        v: 18,
                        width: 9,
                        height: 9,
                    });
                }
            }
        }

        bars
    }

    /// The experience bar (`hasXpBar` path).
    pub fn experience_bar(&self, player: &HudPlayer) -> ExperienceBar {
        let left = self.width / 2 - 91;
        let top = self.height - 32 + 3;
        let mut bar = ExperienceBar {
            background: IconDraw {
                x: left,
                y: top,
                u: 0,
                v: 64,
                width: 182,
                height: 5,
            },
            fill: None,
            level: None,
        };
        let progress = (player.xp_progress * 183.0) as i32;
        if progress > 0 {
            bar.fill = Some(IconDraw {
                x: left,
                y: top,
                u: 0,
                v: 69,
                width: progress,
                height: 5,
            });
        }
        if player.xp_level > 0 {
            bar.level = Some((self.width / 2, self.height - 31 - 4));
        }
        bar
    }

    /// The hotbar and its selection frame.
    pub fn hotbar(&self, selected_slot: i32) -> Hotbar {
        let left = self.width / 2 - 91;
        let top = self.height - 22;
        let mut slots = [(0, 0); 9];
        for slot in 0..9 {
            slots[slot as usize] = (left + 1 + slot * 20, top + 3);
        }
        Hotbar {
            container: IconDraw {
                x: left,
                y: top,
                u: 0,
                v: 0,
                width: 182,
                height: 22,
            },
            selection: IconDraw {
                x: left - 1 + selected_slot * 20,
                y: top - 1,
                u: 0,
                v: 22,
                width: 24,
                height: 22,
            },
            slots,
        }
    }

    /// The crosshair draw over `icons.png` at the screen centre.
    pub fn crosshair(&self) -> IconDraw {
        IconDraw {
            x: self.width / 2 - 7,
            y: self.height / 2 - 7,
            u: 0,
            v: 0,
            width: 16,
            height: 16,
        }
    }

    /// `getNextLevelExperience` of `PlayerEntity`.
    pub fn next_level_experience(level: i32) -> i32 {
        if level >= 30 {
            62 + (level - 30) * 7
        } else if level >= 15 {
            17 + (level - 15) * 3
        } else {
            17
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hud() -> Hud {
        Hud::new(427, 240)
    }

    fn player() -> HudPlayer {
        HudPlayer::default()
    }

    #[test]
    fn full_health_lays_out_ten_hearts_on_one_row() {
        let mut hud = hud();
        let bars = hud.status_bars(&player());
        // 10 background hearts plus 10 full hearts, no armor on a bare player.
        assert_eq!(bars.hearts.len(), 20);
        assert!(bars.armor.is_empty());
        assert_eq!(bars.left, 427 / 2 - 91);
        assert_eq!(bars.top, 240 - 39);
        // Vanilla draws from the top-right heart down: the first draw is the
        // rightmost heart of the row, the last one at the left edge.
        assert_eq!(bars.hearts[0].x, bars.left + 9 * 8);
        assert_eq!(bars.hearts.last().unwrap().x, bars.left);
    }

    #[test]
    fn armor_icons_follow_half_armor_points() {
        let mut hud = hud();
        let mut player = player();
        player.armor = 7; // 3 full icons, 1 half icon, 6 empty
        let bars = hud.status_bars(&player);
        assert_eq!(bars.armor.len(), 10);
        let full = bars.armor.iter().filter(|d| d.u == 34).count();
        let half = bars.armor.iter().filter(|d| d.u == 25).count();
        let empty = bars.armor.iter().filter(|d| d.u == 16).count();
        assert_eq!(full, 3);
        assert_eq!(half, 1);
        assert_eq!(empty, 6);
    }

    #[test]
    fn low_health_shake_draws_from_the_seeded_random() {
        let mut hud = hud();
        let mut player = player();
        player.health = 2.0; // one heart left
        hud.ticks = 7;
        let bars = hud.status_bars(&player);

        let mut reference = LegacyRandom::new(0);
        reference.set_seed(7 * 312871);
        let jitter = reference.next_int_bound(2).unwrap();
        // The heart background rows jitter by exactly that draw.
        let heart_y = bars.top + jitter;
        let shaken = bars.hearts.iter().filter(|d| d.y == heart_y).count();
        assert!(shaken > 0);
    }

    #[test]
    fn food_bar_mirrors_right_to_left() {
        let mut hud = hud();
        let bars = hud.status_bars(&player());
        // 10 background shanks plus 10 full shanks.
        assert_eq!(bars.food.len(), 20);
        assert_eq!(bars.food[0].x, hud.width / 2 + 91 - 9);
        let last = bars.food.last().unwrap();
        assert_eq!(last.x, hud.width / 2 + 91 - 9 * 8 - 9);
    }

    #[test]
    fn hunger_effect_shifts_the_food_icons() {
        let mut hud = hud();
        let mut player = player();
        player.hungry = true;
        let bars = hud.status_bars(&player);
        let backgrounds = bars.food.iter().filter(|d| d.u == 16 + 13 * 9).count();
        assert_eq!(backgrounds, 10);
    }

    #[test]
    fn air_bubbles_appear_only_submerged() {
        let mut hud = hud();
        let dry = hud.status_bars(&player());
        assert!(dry.air.is_empty());

        let mut player = player();
        player.submerged = true;
        player.breath = 300;
        let wet = hud.status_bars(&player);
        assert_eq!(wet.air.len(), 10);
        // Full bubbles are the 16,18 icon.
        assert!(wet.air.iter().all(|d| d.u == 16));
    }

    #[test]
    fn partial_air_mixes_full_and_pop_icons() {
        let mut hud = hud();
        let mut player = player();
        player.submerged = true;
        player.breath = 150;
        let bars = hud.status_bars(&player);
        let full = bars.air.iter().filter(|d| d.u == 16).count();
        let popped = bars.air.iter().filter(|d| d.u == 25).count();
        assert_eq!(full + popped, bars.air.len());
        assert_eq!(full, 5); // ceil((150 - 2) * 10 / 300) = ceil(4.93) = 5
    }

    #[test]
    fn experience_bar_fills_with_progress() {
        let hud = hud();
        let mut player = player();
        player.xp_progress = 0.5;
        let bar = hud.experience_bar(&player);
        assert_eq!(bar.fill.as_ref().unwrap().width, 91);
        assert!(bar.level.is_none());

        player.xp_level = 5;
        let bar = hud.experience_bar(&player);
        assert_eq!(bar.level.unwrap().1, 240 - 35);
    }

    #[test]
    fn hotbar_slots_sit_20_pixels_apart() {
        let hud = hud();
        let bar = hud.hotbar(3);
        assert_eq!(bar.container.x, 427 / 2 - 91);
        assert_eq!(bar.container.y, 240 - 22);
        assert_eq!(bar.selection.x, 427 / 2 - 91 - 1 + 3 * 20);
        assert_eq!(bar.selection.y, 240 - 22 - 1);
        assert_eq!(bar.slots[0].0, 427 / 2 - 90);
        assert_eq!(bar.slots[1].0, 427 / 2 - 90 + 20);
    }

    #[test]
    fn crosshair_sits_at_the_center() {
        let hud = hud();
        let crosshair = hud.crosshair();
        assert_eq!(crosshair.x, 427 / 2 - 7);
        assert_eq!(crosshair.y, 240 / 2 - 7);
        assert_eq!(crosshair.width, 16);
    }

    #[test]
    fn next_level_experience_matches_vanilla_table() {
        // Levels 0..15 cost a flat 17, 16..29 grow by 3, 30 and above by 7.
        assert_eq!(Hud::next_level_experience(0), 17);
        assert_eq!(Hud::next_level_experience(5), 17);
        assert_eq!(Hud::next_level_experience(14), 17);
        assert_eq!(Hud::next_level_experience(15), 17);
        assert_eq!(Hud::next_level_experience(16), 20);
        assert_eq!(Hud::next_level_experience(29), 59);
        assert_eq!(Hud::next_level_experience(30), 62);
        assert_eq!(Hud::next_level_experience(35), 97);
    }
}
