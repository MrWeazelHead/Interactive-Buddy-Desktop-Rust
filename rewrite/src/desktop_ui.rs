//! Compact desktop overlay menus anchored immediately above the taskbar.
use bevy::prelude::{Color, Vec2};

use crate::{Action, Game, Painter};

const STRIP_W: f32 = 76.0;
const STRIP_H: f32 = 259.0;
const PANEL_W: f32 = 250.0;
const GAP: f32 = 7.0;
const MARGIN: f32 = 8.0;

fn strip_rect(g: &Game) -> (Vec2, Vec2) {
    let size = Vec2::new(STRIP_W, STRIP_H);
    let center = Vec2::new(
        g.stage.x - MARGIN - STRIP_W * 0.5,
        g.stage.y - MARGIN - STRIP_H * 0.5,
    );
    (center, size)
}
fn options_rect(g: &Game) -> (Vec2, Vec2) {
    let requested_height = match g.desktop_section.as_str() {
        "File" => 180.0,
        "Items" => 250.0,
        "Skins" | "Modes" => 248.0,
        "Settings" => 222.0,
        "Help" => 130.0,
        _ => 150.0,
    };
    let bottom = g.stage.y - 115.0;
    let top = (bottom - requested_height).max(8.0);
    let height = bottom - top;
    let size = Vec2::new(PANEL_W, height);
    let center = Vec2::new(
        g.stage.x - MARGIN - STRIP_W - GAP - PANEL_W * 0.5,
        top + height * 0.5,
    );
    (center, size)
}

/// Number of pages for the currently selected owned list. Keep in sync with
/// the item-category and skin/mode page sizes used by the options panel.
pub(crate) fn page_count(g: &Game) -> usize {
    match g.desktop_section.as_str() {
        "Items" => {
            let category = match g.category.as_str() {
                "Explosives" | "God Powers" | "Guns" | "Hand" | "Miscellaneous" | "Objects"
                | "Special" => g.category.as_str(),
                _ => "Hand",
            };
            let count = g
                .catalog
                .items
                .iter()
                .filter(|e| e.category == category && g.save.owned.contains(&e.name))
                .count();
            count.div_ceil(5).max(1)
        }
        "Skins" => g
            .catalog
            .skins
            .iter()
            .filter(|e| g.save.owned.contains(&e.name))
            .count()
            .div_ceil(7)
            .max(1),
        "Modes" => g
            .catalog
            .modes
            .iter()
            .filter(|e| g.save.owned.contains(&e.name))
            .count()
            .div_ceil(7)
            .max(1),
        _ => 1,
    }
}

fn tab_rect(g: &Game) -> (Vec2, Vec2) {
    (
        Vec2::new(g.stage.x - 29.0, g.stage.y - 25.0),
        Vec2::new(42.0, 34.0),
    )
}

fn collapsed_exit_rect(g: &Game) -> (Vec2, Vec2) {
    // Keep Exit adjacent to the collapsed tab, inside the primary work area.
    (
        Vec2::new(g.stage.x - 79.0, g.stage.y - 25.0),
        Vec2::new(48.0, 34.0),
    )
}

/// Precise hit test for the dock's disjoint menu and options rectangles.
fn lock_rect(g: &Game) -> (Vec2, Vec2) {
    (
        Vec2::new(g.stage.x - 56.0, g.stage.y - 65.0),
        Vec2::new(100.0, 24.0),
    )
}

pub(crate) fn hit(g: &Game, point: Vec2) -> bool {
    if !g.desktop_open {
        let (center, size) = tab_rect(g);
        let (exit_center, exit_size) = collapsed_exit_rect(g);
        let (lock_center, lock_size) = lock_rect(g);
        return (point - center).abs().cmple(size * 0.5).all()
            || (point - exit_center).abs().cmple(exit_size * 0.5).all()
            || (point - lock_center).abs().cmple(lock_size * 0.5).all();
    }
    let (strip_center, strip_size) = strip_rect(g);
    if (point - strip_center).abs().cmple(strip_size * 0.5).all() {
        return true;
    }
    // Clicking the selected menu a second time collapses its options. Leave
    // the strip itself interactive, but make the now-empty options area
    // transparent to desktop clicks.
    !g.desktop_section.is_empty() && {
        let (center, size) = options_rect(g);
        (point - center).abs().cmple(size * 0.5).all()
    }
}

fn dark_button(
    d: &mut Painter<'_, '_, '_>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    label: &str,
    action: Action,
) {
    dark_control(d, x, y, w, h, label, action, false);
}
fn dark_selected(
    d: &mut Painter<'_, '_, '_>,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    action: Action,
    selected: bool,
) {
    dark_control(d, x, y, w, 20.0, label, action, selected);
}
fn dark_control(
    d: &mut Painter<'_, '_, '_>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    label: &str,
    action: Action,
    selected: bool,
) {
    let hover = (d.mouse - Vec2::new(x + w * 0.5, y + h * 0.5))
        .abs()
        .cmple(Vec2::new(w, h) * 0.5)
        .all();
    let color = if selected {
        Color::srgba(0.22, 0.34, 0.46, 0.98)
    } else if hover {
        Color::srgba(0.20, 0.24, 0.29, 0.98)
    } else {
        Color::srgba(0.105, 0.125, 0.15, 0.98)
    };
    d.rect(x, y, w, h, color);
    d.text(
        x + 6.0,
        y + 3.0,
        label,
        10.0,
        Color::srgba(0.90, 0.92, 0.95, 1.0),
    );
    d.hits.push(crate::Hit {
        pos: Vec2::new(x + w * 0.5, y + h * 0.5),
        size: Vec2::new(w, h),
        action,
    });
}

pub(crate) fn draw(d: &mut Painter<'_, '_, '_>, g: &Game) {
    d.z += 500.0;
    if !g.desktop_open {
        let (center, size) = tab_rect(g);
        d.rect(
            center.x - size.x * 0.5,
            center.y - size.y * 0.5,
            size.x,
            size.y,
            Color::srgba(0.055, 0.065, 0.08, 0.94),
        );
        dark_button(
            d,
            center.x - size.x * 0.5 + 2.0,
            center.y - size.y * 0.5 + 2.0,
            size.x - 4.0,
            size.y - 4.0,
            "IB  ▴",
            Action::DesktopToggle,
        );
        let (exit_center, exit_size) = collapsed_exit_rect(g);
        dark_button(
            d,
            exit_center.x - exit_size.x * 0.5,
            exit_center.y - exit_size.y * 0.5,
            exit_size.x,
            exit_size.y,
            "Exit",
            Action::Quit,
        );
        let (lock_center, lock_size) = lock_rect(g);
        dark_button(
            d,
            lock_center.x - lock_size.x * 0.5,
            lock_center.y - lock_size.y * 0.5,
            lock_size.x,
            lock_size.y,
            "Lock to window",
            Action::ToggleSurface,
        );
        return;
    }

    let (strip_center, strip_size) = strip_rect(g);
    let strip_x = strip_center.x - strip_size.x * 0.5;
    let strip_y = strip_center.y - strip_size.y * 0.5;
    d.rect(
        strip_x,
        strip_y,
        strip_size.x,
        strip_size.y,
        Color::srgba(0.045, 0.055, 0.07, 0.94),
    );
    let (panel_x, panel_y) = if g.desktop_section.is_empty() {
        (0.0, 0.0)
    } else {
        let (panel_center, panel_size) = options_rect(g);
        let x = panel_center.x - panel_size.x * 0.5;
        let y = panel_center.y - panel_size.y * 0.5;
        d.rect(
            x,
            y,
            panel_size.x,
            panel_size.y,
            Color::srgba(0.045, 0.055, 0.07, 0.94),
        );
        (x, y)
    };

    let menus = ["File", "Skins", "Items", "Modes", "Settings", "Help"];
    for (i, menu) in menus.iter().enumerate() {
        let y = strip_y + 5.0 + i as f32 * 29.0;
        dark_selected(
            d,
            strip_x + 4.0,
            y,
            strip_size.x - 8.0,
            menu,
            Action::DesktopSection((*menu).into()),
            g.desktop_section == *menu,
        );
    }
    dark_button(
        d,
        strip_x + 4.0,
        strip_y + strip_size.y - 49.0,
        strip_size.x - 8.0,
        20.0,
        "Hide",
        Action::DesktopToggle,
    );
    dark_button(
        d,
        strip_x + 4.0,
        strip_y + strip_size.y - 75.0,
        strip_size.x - 8.0,
        20.0,
        "Lock window",
        Action::ToggleSurface,
    );
    dark_button(
        d,
        strip_x + 4.0,
        strip_y + strip_size.y - 25.0,
        strip_size.x - 8.0,
        20.0,
        "Exit",
        Action::Quit,
    );

    match g.desktop_section.as_str() {
        "" => {}
        "File" => draw_file(d, g, panel_x, panel_y),
        "Items" => draw_items(d, g, panel_x, panel_y),
        "Skins" => draw_catalog(d, g, panel_x, panel_y, "Skins"),
        "Modes" => draw_catalog(d, g, panel_x, panel_y, "Modes"),
        "Settings" => draw_settings(d, g, panel_x, panel_y),
        "Help" => draw_help(d, panel_x, panel_y),
        _ => draw_file(d, g, panel_x, panel_y),
    }
}

fn draw_file(d: &mut Painter<'_, '_, '_>, g: &Game, x: f32, y: f32) {
    d.text(
        x + 12.0,
        y + 10.0,
        format!("Buddy   ${:.2}", g.save.cash),
        13.0,
        Color::WHITE,
    );
    for (i, (label, action)) in [
        ("Stats", Action::Panel("Stats".into())),
        ("Reset Buddy", Action::ResetBuddy),
        (if g.paused { "Resume" } else { "Pause" }, Action::Pause),
        ("Clear Progress...", Action::Panel("Clear File".into())),
        ("Restore Windows", Action::RestoreWindows),
        ("Exit", Action::Quit),
    ]
    .into_iter()
    .enumerate()
    {
        dark_button(
            d,
            x + 9.0,
            y + 38.0 + i as f32 * 23.0,
            232.0,
            21.0,
            label,
            action,
        );
    }
}

fn draw_items(d: &mut Painter<'_, '_, '_>, g: &Game, x: f32, y: f32) {
    const CATS: [&str; 7] = [
        "Explosives",
        "God Powers",
        "Guns",
        "Hand",
        "Miscellaneous",
        "Objects",
        "Special",
    ];
    d.text(x + 9.0, y + 7.0, "Owned tools", 12.0, Color::WHITE);
    for (i, category) in CATS.iter().enumerate() {
        let col = i % 2;
        let row = i / 2;
        dark_button(
            d,
            x + 8.0 + col as f32 * 117.0,
            y + 27.0 + row as f32 * 19.0,
            112.0,
            18.0,
            category,
            Action::Category((*category).into()),
        );
    }
    let category = if CATS.contains(&g.category.as_str()) {
        g.category.as_str()
    } else {
        "Hand"
    };
    let entries = g
        .catalog
        .items
        .iter()
        .filter(|e| e.category == category && g.save.owned.contains(&e.name))
        .collect::<Vec<_>>();
    let pages = entries.len().div_ceil(5).max(1);
    let page = g.page.min(pages - 1);
    for (offset, entry) in entries.iter().skip(page * 5).take(5).enumerate() {
        dark_selected(
            d,
            x + 8.0,
            y + 106.0 + offset as f32 * 20.0,
            234.0,
            &entry.name,
            Action::Select("Items".into(), entry.name.clone()),
            g.save.item == entry.name,
        );
    }
    dark_button(d, x + 8.0, y + 210.0, 27.0, 18.0, "<", Action::Page(-1));
    d.text(
        x + 43.0,
        y + 214.0,
        format!("{} / {}", page + 1, pages),
        10.0,
        Color::WHITE,
    );
    dark_button(d, x + 84.0, y + 210.0, 27.0, 18.0, ">", Action::Page(1));
    dark_button(
        d,
        x + 130.0,
        y + 210.0,
        112.0,
        18.0,
        "Shop tools",
        Action::Store("Items".into()),
    );
}

fn draw_catalog(d: &mut Painter<'_, '_, '_>, g: &Game, x: f32, y: f32, kind: &str) {
    let entries = if kind == "Skins" {
        &g.catalog.skins
    } else {
        &g.catalog.modes
    };
    let owned = entries
        .iter()
        .filter(|e| g.save.owned.contains(&e.name))
        .collect::<Vec<_>>();
    let pages = owned.len().div_ceil(7).max(1);
    let page = g.page.min(pages - 1);
    d.text(
        x + 10.0,
        y + 8.0,
        format!("Owned {kind}"),
        12.0,
        Color::WHITE,
    );
    for (offset, entry) in owned.iter().enumerate().skip(page * 7).take(7) {
        let active = if kind == "Skins" {
            g.save.skin == entry.id
        } else {
            g.save.modes.contains(&entry.id)
        };
        dark_selected(
            d,
            x + 8.0,
            y + 32.0 + (offset - page * 7) as f32 * 21.0,
            234.0,
            &entry.name,
            Action::Select(kind.into(), entry.name.clone()),
            active,
        );
    }
    dark_button(d, x + 8.0, y + 184.0, 28.0, 20.0, "<", Action::Page(-1));
    d.text(
        x + 44.0,
        y + 188.0,
        format!("{} / {}", page + 1, pages),
        10.0,
        Color::WHITE,
    );
    dark_button(d, x + 85.0, y + 184.0, 28.0, 20.0, ">", Action::Page(1));
    dark_button(
        d,
        x + 125.0,
        y + 184.0,
        117.0,
        20.0,
        &format!("Shop {kind}"),
        Action::Store(kind.into()),
    );
}

fn draw_settings(d: &mut Painter<'_, '_, '_>, g: &Game, x: f32, y: f32) {
    d.text(x + 10.0, y + 8.0, "Settings", 12.0, Color::WHITE);
    dark_button(
        d,
        x + 8.0,
        y + 31.0,
        234.0,
        22.0,
        if g.save.sound {
            "Sound: On"
        } else {
            "Sound: Off"
        },
        Action::ToggleSound,
    );
    dark_selected(
        d,
        x + 8.0,
        y + 57.0,
        234.0,
        if g.desktop_interact {
            "Play anywhere: Armed"
        } else {
            "Play anywhere: Off"
        },
        Action::DesktopInteract,
        g.desktop_interact,
    );
    dark_button(
        d,
        x + 8.0,
        y + 81.0,
        234.0,
        22.0,
        "Physics...",
        Action::Panel("Physics".into()),
    );
    for (i, key) in ["Anti-Aliasing", "Effects", "Motion Blur"]
        .iter()
        .enumerate()
    {
        let value = match *key {
            "Anti-Aliasing" => &g.save.anti_aliasing,
            "Effects" => &g.save.effects,
            _ => &g.save.motion_blur,
        };
        let options = if *key == "Motion Blur" {
            ["None", "Weak", "Strong"]
        } else {
            ["Low", "Medium", "High"]
        };
        let next = options
            .iter()
            .position(|v| *v == value)
            .map_or(options[0], |index| options[(index + 1) % options.len()]);
        dark_button(
            d,
            x + 8.0,
            y + 109.0 + i as f32 * 25.0,
            234.0,
            22.0,
            &format!("{key}: {value}  →"),
            Action::Setting((*key).into(), next.into()),
        );
    }
}

fn draw_help(d: &mut Painter<'_, '_, '_>, x: f32, y: f32) {
    d.text(x + 10.0, y + 8.0, "Help", 12.0, Color::WHITE);
    for (i, label) in ["Help", "What's New?", "About"].iter().enumerate() {
        dark_button(
            d,
            x + 9.0,
            y + 35.0 + i as f32 * 25.0,
            232.0,
            22.0,
            label,
            Action::Panel((*label).into()),
        );
    }
}
