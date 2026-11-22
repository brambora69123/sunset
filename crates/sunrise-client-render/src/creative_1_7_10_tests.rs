//! The era's creative screen: the tab list, the grid's geometry and a sample of
//! every tab's contents, against the 1.7.10 classes.

use super::*;

/// The tab list, `CreativeModeTab.ALL` without the search and inventory tabs:
/// id, name and row.
#[test]
fn the_ten_item_tabs_keep_vanillas_order_and_cells() {
    let expected = [
        (0, "buildingBlocks", true, 0),
        (1, "decorations", true, 1),
        (2, "redstone", true, 2),
        (3, "transportation", true, 3),
        (4, "misc", true, 4),
        (6, "food", false, 0),
        (7, "tools", false, 1),
        (8, "combat", false, 2),
        (9, "brewing", false, 3),
        (10, "materials", false, 4),
    ];
    assert_eq!(
        Tab17::ALL
            .map(|tab| (tab.id(), tab.name(), tab.top_row(), tab.column()))
            .to_vec(),
        expected
    );
    // `getDisplayName` is `itemGroup.<name>`.
    for tab in Tab17::ALL {
        assert_eq!(tab.title_key(), format!("itemGroup.{}", tab.name()));
        assert!(tab.has_scrollbar());
    }
}

/// `renderTabIcon`'s cells and the strip's hit rows (`isInRow`).
#[test]
fn tab_cells_and_hit_rows_sit_where_the_era_puts_them() {
    let building = Tab17::BuildingBlocks.tab_cell(false);
    assert_eq!(
        (building.x, building.y, building.u, building.v),
        (0, -28, 0, 0)
    );
    assert_eq!((building.width, building.height), (28, 32));
    // Column 1 and up shift by their column as well: `x = 28 * column + column`.
    let decorations = Tab17::Decorations.tab_cell(false);
    assert_eq!((decorations.x, decorations.u), (29, 28));
    let misc = Tab17::Misc.tab_cell(false);
    assert_eq!((misc.x, misc.u), (116, 112));
    // The bottom row hangs below the panel at `backgroundHeight - 4`, its cells
    // two rows down the sheet.
    let food = Tab17::Food.tab_cell(false);
    assert_eq!((food.x, food.y, food.u, food.v), (0, 132, 0, 64));
    let materials = Tab17::Materials.tab_cell(false);
    assert_eq!(
        (materials.x, materials.y, materials.u, materials.v),
        (116, 132, 112, 64)
    );
    // A selected cell is the next row of the sheet: `v += 32`.
    assert_eq!(Tab17::BuildingBlocks.tab_cell(true).v, 32);
    assert_eq!(Tab17::Food.tab_cell(true).v, 96);
    // The icon sits six pixels in, eight down, one further on the top row and
    // one back up on the bottom row.
    assert_eq!(Tab17::BuildingBlocks.icon_position(false), (6, -19));
    assert_eq!(Tab17::Food.icon_position(false), (6, 139));

    // `isInRow`'s rectangles: `-32..0` above and `136..168` below the panel,
    // four pixels past the drawn cells.
    assert_eq!(Tab17::at(4, -20), Some(Tab17::BuildingBlocks));
    assert_eq!(Tab17::at(4, -8), Some(Tab17::BuildingBlocks));
    assert_eq!(Tab17::at(4, 4), None);
    assert_eq!(Tab17::at(4, 140), Some(Tab17::Food));
    assert_eq!(Tab17::at(4, 128), None);
    assert_eq!(Tab17::at(120, 140), Some(Tab17::Materials));
    assert_eq!(Tab17::at(4, 30), None);
}

/// The display grid and the hotbar row, `CreativePlayerMenu`'s `addSlot`s.
#[test]
fn the_grid_is_five_by_nine_at_the_eras_offsets() {
    assert_eq!(GRID_SLOTS, 45);
    assert_eq!(CreativeGrid17::slot_position(0), (9, 18));
    assert_eq!(CreativeGrid17::slot_position(8), (153, 18));
    assert_eq!(CreativeGrid17::slot_position(9), (9, 36));
    assert_eq!(CreativeGrid17::slot_position(44), (153, 90));
    assert_eq!(CreativeGrid17::hotbar_position(0), (9, 112));
    assert_eq!(CreativeGrid17::hotbar_position(8), (153, 112));

    // `AbstractContainerScreen.isHovering`: a pixel around the 16x16 slot.
    assert_eq!(CreativeGrid17::slot_at(9.0, 18.0), Some(0));
    assert_eq!(CreativeGrid17::slot_at(8.0, 17.0), Some(0));
    assert_eq!(CreativeGrid17::slot_at(7.9, 17.0), None);
    // The hulls of adjacent slots meet: slot 0 reaches to x = 25, slot 1 takes 26.
    assert_eq!(CreativeGrid17::slot_at(25.0, 18.0), Some(0));
    assert_eq!(CreativeGrid17::slot_at(26.0, 18.0), Some(1));
    assert_eq!(CreativeGrid17::slot_at(9.0, 108.0), None);
    assert_eq!(CreativeGrid17::slot_at(9.0, 110.0), None);
    assert_eq!(CreativeGrid17::hotbar_slot_at(9.0, 112.0), Some(0));
    assert_eq!(CreativeGrid17::hotbar_slot_at(10.0, 114.0), Some(0));
    assert_eq!(CreativeGrid17::hotbar_slot_at(153.0, 112.0), Some(8));
}

/// A sample of every tab's list, in `addItems` order: the first stacks, one
/// later stack and the last, from `CreativeModeTab.<tab>.addItems`.
#[test]
fn every_tab_lists_the_eras_own_stacks() {
    let expected: [(Tab17, usize, [(u16, u16); 3], (u16, u16), (u16, u16)); 10] = [
        (
            Tab17::BuildingBlocks,
            141,
            [(1, 0), (2, 0), (3, 0)],
            (91, 0),
            (174, 0),
        ),
        (
            Tab17::Decorations,
            103,
            [(6, 0), (6, 1), (6, 2)],
            (145, 2),
            (397, 4),
        ),
        (
            Tab17::Redstone,
            26,
            [(23, 0), (25, 0), (29, 0)],
            (131, 0),
            (404, 0),
        ),
        (
            Tab17::Transportation,
            12,
            [(27, 0), (28, 0), (66, 0)],
            (333, 0),
            (408, 0),
        ),
        (
            Tab17::Misc,
            56,
            [(138, 0), (325, 0), (326, 0)],
            (383, 92),
            (2267, 0),
        ),
        (
            Tab17::Food,
            28,
            [(260, 0), (282, 0), (297, 0)],
            (357, 0),
            (400, 0),
        ),
        (
            Tab17::Tools,
            33,
            [(256, 0), (257, 0), (258, 0)],
            (290, 0),
            (403, 0),
        ),
        (
            Tab17::Combat,
            45,
            [(261, 0), (262, 0), (267, 0)],
            (313, 0),
            (403, 0),
        ),
        (
            Tab17::Brewing,
            65,
            [(370, 0), (373, 0), (373, 8193)],
            (373, 8232),
            (382, 0),
        ),
        (
            Tab17::Materials,
            45,
            [(263, 0), (263, 1), (264, 0)],
            (351, 3),
            (406, 0),
        ),
    ];
    for (tab, len, first, middle, last) in expected {
        let items = tab.items();
        assert_eq!(items.len(), len, "{tab:?}");
        assert_eq!(&items[..3], &first[..], "{tab:?}");
        assert_eq!(items[len / 2], middle, "{tab:?}");
        assert_eq!(items[len - 1], last, "{tab:?}");
    }
    // Metadata variants are their own stacks: the sixteen wools and the sixteen
    // dyes.
    let wools: Vec<_> = Tab17::BuildingBlocks
        .items()
        .iter()
        .filter(|&&(id, _)| id == 35)
        .map(|&(_, meta)| meta)
        .collect();
    assert_eq!(wools, (0..16).collect::<Vec<_>>());
    let dyes = Tab17::Materials
        .items()
        .iter()
        .filter(|&&(id, _)| id == 351)
        .count();
    assert_eq!(dyes, 16);
}

/// `scrollItems` and the scrollbar: the first visible row, the thumb's travel
/// and the wheel.
#[test]
fn a_long_tab_pages_five_rows_at_a_time() {
    let mut grid = CreativeGrid17::new(Tab17::BuildingBlocks, Tab17::BuildingBlocks.items());
    // 141 stacks: 141 / 9 - 5 + 1 = 11 rows of travel.
    assert_eq!(grid.scroll_rows(), 11);
    assert_eq!(grid.first_row(), 0);
    assert_eq!(grid.shown(0), Some((1, 0)));
    assert_eq!(grid.shown(44), Some((35, 15)));
    assert!(grid.scrollbar());
    assert_eq!(grid.thumb_y(), 18);
    assert!(grid.inside_thumb(180.0, 20.0));
    assert!(!grid.inside_thumb(180.0, 18.0 + 15.0));
    assert!(grid.inside_scrollbar(180.0, 129.0));

    grid.scroll = 1.0;
    // `(int)(1 * 11 + 0.5)` = 11: the last page starts at row 11, stack 99.
    assert_eq!(grid.first_row(), 11);
    assert_eq!(
        grid.shown(0),
        Tab17::BuildingBlocks.items().get(99).copied()
    );
    assert_eq!(grid.thumb_y(), 18 + 95);
    assert!(grid.inside_thumb(180.0, 114.0));

    // One wheel notch is `1 / 11` of the list; a wheel up goes toward the
    // list's start (`scrollPosition -= var1 / var2`).
    grid.wheel(1.0);
    assert!((grid.scroll - (1.0 - 1.0 / 11.0)).abs() < 1e-6);
    grid.wheel(-1.0);
    assert!((grid.scroll - 1.0).abs() < 1e-6);

    // The drag maps the track to 0..1.
    grid.drag(18.0 + 7.5);
    assert_eq!(grid.scroll, 0.0);
    grid.drag(18.0 + 112.0);
    assert_eq!(grid.scroll, 1.0);

    // A tab that fits one page never scrolls and shows no scrollbar.
    let short = CreativeGrid17::new(Tab17::Transportation, Tab17::Transportation.items());
    assert_eq!(short.scroll_rows(), -3);
    assert_eq!(short.first_row(), 0);
    assert!(!short.scrollbar());
    assert!(!short.inside_scrollbar(180.0, 30.0));
    let mut wheeled = short;
    wheeled.wheel(-1.0);
    assert_eq!(wheeled.scroll, 0.0);
}

/// The panel and the strip are the era's files, and the tabs' icons are stacks
/// of the era's items.
#[test]
fn the_screen_draws_the_eras_panel_and_icon_stacks() {
    assert_eq!(Tab17::BuildingBlocks.panel().texture, ITEMS_PANEL);
    let panel = Tab17::BuildingBlocks.panel();
    assert_eq!((panel.u, panel.v, panel.w, panel.h), (0, 0, 195, 136));
    let tab = Tab17::Combat.tab_cell(false).sprite();
    assert_eq!(tab.texture, TABS_TEXTURE);
    assert_eq!((tab.u, tab.v, tab.w, tab.h), (56, 64, 28, 32));
    for tab in Tab17::ALL {
        let (id, _) = tab.icon();
        assert!(
            crate::items_1_7_10::item_name(id).is_some(),
            "{tab:?} names its icon item {id}"
        );
    }
}
