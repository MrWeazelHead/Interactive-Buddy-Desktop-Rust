//! Desktop companion rail: an unobtrusive icon strip plus one compact panel.
//!
//! The overlay shares the desktop with everything else the user is doing, so
//! the chrome stays out of the way: the menu button pinned to the bottom-right
//! corner with the close button beside it, eighteen-pixel section dots
//! stacking upward from the menu button, a panel that only exists while a
//! section is open, and an optional one-line status readout. Nothing here is
//! required to play.
use bevy::prelude::{Color, Handle, Image, Vec2};

use crate::{Action, Game, Painter};

/// Visual diameter of a rail dot. Matches the eighteen-pixel source art.
const DOT: f32 = 18.0;
/// Hit diameter. Larger than the art so the dots stay easy to click, but
/// smaller than `PITCH` so a real gutter of dead desktop survives between
/// them: at twenty-two the dots leave four clickable-through pixels, where a
/// touch box as wide as the pitch would swallow the gap entirely.
const TOUCH: f32 = 22.0;
/// Clear space between neighbouring dots.
const DOT_GAP: f32 = 8.0;
/// Centre-to-centre distance between neighbouring dots.
const PITCH: f32 = DOT + DOT_GAP;
/// Clear space between the menu button and the close button beside it.
const CLOSE_GAP: f32 = 8.0;
/// Gaps from the right and bottom edges of the desktop work area.
const RAIL_RIGHT: f32 = 14.0;
const RAIL_BOTTOM: f32 = 14.0;
/// Panel metrics.
const PANEL_W: f32 = 208.0;
const PANEL_GAP: f32 = 10.0;
const PANEL_PAD: f32 = 8.0;
const ROW_H: f32 = 17.0;
const ROW_PITCH: f32 = 20.0;
const HEAD_H: f32 = 22.0;

/// Icon drawn on a section dot. The project ships one reusable circle of art,
/// so the sections are drawn rather than imported; each gets its own colour so
/// the rail is readable at a glance without hovering.
#[derive(Clone, Copy)]
pub(crate) enum Icon {
    File,
    Skins,
    Items,
    Modes,
    Settings,
    Help,
    Menu,
    Close,
    /// Drawn on the menu button while clicks pass through to the desktop:
    /// a pointer with a bar across it.
    Passthrough,
}

/// Section names, bottom to top: index zero is the dot nearest the menu button.
/// The colours are muted on purpose: they have to stay readable against an
/// arbitrary desktop without competing with it for attention.
pub(crate) const SECTIONS: [(&str, Icon, Color); 6] = [
    ("File", Icon::File, Color::srgba(0.42, 0.56, 0.74, 0.95)),
    ("Skins", Icon::Skins, Color::srgba(0.82, 0.44, 0.52, 0.95)),
    ("Items", Icon::Items, Color::srgba(0.78, 0.61, 0.26, 0.95)),
    ("Modes", Icon::Modes, Color::srgba(0.56, 0.42, 0.76, 0.95)),
    (
        "Settings",
        Icon::Settings,
        Color::srgba(0.30, 0.62, 0.58, 0.95),
    ),
    ("Help", Icon::Help, Color::srgba(0.52, 0.57, 0.66, 0.95)),
];
/// Owned-item page sizes, kept in sync with `page_count`.
const ITEM_PAGE: usize = 5;
const CATALOG_PAGE: usize = 7;

/// Centre of the menu button, pinned to the bottom-right corner of the work
/// area. The section dots stack upward from it and the close button sits
/// beside it, so the two things that are always needed stay in one place.
pub(crate) fn menu_center(g: &Game) -> Vec2 {
    Vec2::new(
        g.stage.x - RAIL_RIGHT - DOT * 0.5,
        g.stage.y - RAIL_BOTTOM - DOT * 0.5,
    )
}

/// Centre of the close button: immediately left of the menu button, and
/// deliberately outside the rail so quitting is never a mis-click away from a
/// section.
pub(crate) fn close_center(g: &Game) -> Vec2 {
    let menu = menu_center(g);
    Vec2::new(menu.x - (DOT + CLOSE_GAP), menu.y)
}

/// Centre of section dot `index`, counting upward from the menu button.
pub(crate) fn rail_center(g: &Game, index: usize) -> Vec2 {
    let menu = menu_center(g);
    Vec2::new(menu.x, menu.y - PITCH * (index as f32 + 1.0))
}

/// Name, icon, tint and action for a section dot, or `None` past the end.
fn rail_entry(index: usize) -> Option<(&'static str, Icon, Color, Action)> {
    let (name, icon, tint) = *SECTIONS.get(index)?;
    Some((name, icon, tint, Action::DesktopSection(name.into())))
}

/// Label shown in the hover caption for a hovered dot.
fn rail_label(index: usize) -> &'static str {
    SECTIONS[index].0
}

/// Bounding box of the corner controls: the close button on the left through
/// to the top of the section column. Used to keep the panel clear of them.
fn rail_extent(g: &Game) -> (Vec2, Vec2) {
    let menu = menu_center(g);
    let left = close_center(g).x - DOT * 0.5;
    let right = menu.x + DOT * 0.5;
    let top = rail_center(g, SECTIONS.len() - 1).y - DOT * 0.5;
    let bottom = menu.y + DOT * 0.5;
    (
        Vec2::new((left + right) * 0.5, (top + bottom) * 0.5),
        Vec2::new(right - left, bottom - top),
    )
}

/// Panel height for the open section, in stage units.
fn panel_height(g: &Game) -> f32 {
    let content = match g.desktop_section.as_str() {
        // Seven rows, plus room for a failed save's message.
        "File" => HEAD_H + 9.0 * ROW_PITCH + (if g.save_error.is_some() { 16.0 } else { 0.0 }),
        "Items" => {
            HEAD_H
                + (if g.save.pet.enabled { 5.0 } else { 4.0 }) * 17.0
                + 8.0
                + ITEM_PAGE as f32 * ROW_PITCH
                + ROW_PITCH
        }
        "Skins" => HEAD_H + 4.0 + CATALOG_PAGE as f32 * ROW_PITCH + 2.0 * ROW_PITCH,
        "Modes" => HEAD_H + 4.0 + CATALOG_PAGE as f32 * ROW_PITCH + ROW_PITCH,
        // Five rows, a divider, then the three one-line cycles.
        "Settings" => HEAD_H + 8.0 * ROW_PITCH + 4.0 + 3.0 * ROW_PITCH + 4.0,
        // Three rows, then the two-line note about the H keybind.
        "Help" => HEAD_H + 3.0 * ROW_PITCH + 4.0 + 6.0 + 22.0,
        _ => HEAD_H,
    };
    content + PANEL_PAD * 2.0
}

/// Panel rectangle, bottom-aligned with the rail and clamped to the work area.
fn panel_rect(g: &Game) -> (Vec2, Vec2) {
    let (rail, rail_size) = rail_extent(g);
    // Anchor to the visible dot column, not the larger touch box, so the panel
    // can never cover a dot.
    let right = rail.x - DOT * 0.5 - PANEL_GAP;
    // Prefer sitting on the rail's bottom edge, but fall back to hanging from
    // its top when the section is too tall. Clamping the height instead would
    // silently push the last rows outside the plate.
    let rail_top = rail.y - rail_size.y * 0.5;
    let rail_bottom = rail.y + rail_size.y * 0.5;
    let height = panel_height(g);
    let top = if rail_bottom - height >= 6.0 {
        rail_bottom - height
    } else {
        rail_top.max(6.0)
    };
    let height = height.min(g.stage.y - 6.0 - top);
    (
        Vec2::new(right - PANEL_W * 0.5, top + height * 0.5),
        Vec2::new(PANEL_W, height),
    )
}

/// Number of pages for the currently selected owned list. Keep in sync with
/// the item-category and skin/mode page sizes used by the options panel.
pub(crate) fn page_count(g: &Game) -> usize {
    match g.desktop_section.as_str() {
        "Items" => {
            let category = match g.category.as_str() {
                "Pet Items" if g.save.pet.enabled => "Pet Items",
                "Explosives" | "God Powers" | "Guns" | "Hand" | "Miscellaneous" | "Objects"
                | "Special" | "Favorites" => g.category.as_str(),
                _ => "Hand",
            };
            let count = g
                .catalog
                .items
                .iter()
                .filter(|e| g.in_category(e, category) && g.save.owned.contains(&e.name))
                .count();
            count.div_ceil(ITEM_PAGE).max(1)
        }
        "Skins" => g
            .catalog
            .skins
            .iter()
            .filter(|e| g.save.owned.contains(&e.name))
            .count()
            .div_ceil(CATALOG_PAGE)
            .max(1),
        "Modes" => g
            .catalog
            .modes
            .iter()
            .filter(|e| g.save.owned.contains(&e.name))
            .count()
            .div_ceil(CATALOG_PAGE)
            .max(1),
        _ => 1,
    }
}

/// Precise hit test: the menu and close buttons are always live, the section
/// dots only while the rail is open, and the panel only while a section is
/// open. Everything else falls through to the desktop.
pub(crate) fn hit(g: &Game, point: Vec2) -> bool {
    // The corner buttons are tested before the passthrough check on purpose.
    // Click-through is about the *room* letting clicks reach whatever is
    // underneath it, not about locking the player out of the menu: with these
    // two dead as well, passthrough became a one-way door whose only exit was
    // the Ctrl+Alt+P hotkey. They are also the two things a player reaches for
    // deliberately, so honouring them costs nothing on the desktop below.
    if corner_hit(g, point) {
        return true;
    }
    if g.save.mouse_passthrough {
        return false;
    }
    if !g.desktop_open {
        return false;
    }
    // Each dot is tested on its own so the gaps between them stay
    // click-through: a few pixels of dead desktop between the dots is
    // cheaper than a misdirected click.
    if rail_index_at(g, point).is_some() {
        return true;
    }
    // Re-selecting the open section closes it again.
    !g.desktop_section.is_empty() && {
        let (center, size) = panel_rect(g);
        (point - center).abs().cmple(size * 0.5).all()
    }
}

/// The menu and close buttons, which stay clickable in every mode.
fn corner_hit(g: &Game, point: Vec2) -> bool {
    [menu_center(g), close_center(g)]
        .into_iter()
        .any(|center| (point - center).abs().cmple(Vec2::splat(TOUCH * 0.5)).all())
}

/// Index of the section dot under `point`, if any.
fn rail_index_at(g: &Game, point: Vec2) -> Option<usize> {
    (0..SECTIONS.len()).find(|&i| {
        (point - rail_center(g, i))
            .abs()
            .cmple(Vec2::splat(TOUCH * 0.5))
            .all()
    })
}

fn under(d: &Painter<'_, '_, '_>, center: Vec2, size: Vec2) -> bool {
    (d.mouse - center).abs().cmple(size * 0.5).all()
}

/// Translucent plate behind a block of text so it stays readable over any
/// desktop wallpaper.
fn plate(d: &mut Painter<'_, '_, '_>, x: f32, y: f32, w: f32, h: f32) {
    d.rect(x, y, w, h, Color::srgba(0.035, 0.045, 0.06, 0.82));
}

/// Section heading with a hairline underline.
fn heading(d: &mut Painter<'_, '_, '_>, x: f32, y: f32, title: &str, right: &str) {
    d.text(x, y, title, 11.0, Color::srgba(0.95, 0.97, 1.0, 1.0));
    if !right.is_empty() {
        d.text(
            x + PANEL_W - PANEL_PAD * 2.0 - right.len() as f32 * 5.2,
            y + 1.0,
            right,
            9.0,
            Color::srgba(0.62, 0.70, 0.78, 1.0),
        );
    }
    d.rect(
        x,
        y + 15.0,
        PANEL_W - PANEL_PAD * 2.0,
        1.0,
        Color::srgba(0.35, 0.42, 0.50, 0.55),
    );
}

fn row(
    d: &mut Painter<'_, '_, '_>,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    action: Action,
    selected: bool,
) {
    let hover = under(
        d,
        Vec2::new(x + w * 0.5, y + ROW_H * 0.5),
        Vec2::new(w, ROW_H),
    );
    d.rect(
        x,
        y,
        w,
        ROW_H,
        if selected {
            Color::srgba(0.20, 0.33, 0.45, 0.96)
        } else if hover {
            Color::srgba(0.19, 0.23, 0.28, 0.96)
        } else {
            Color::srgba(0.085, 0.10, 0.125, 0.92)
        },
    );
    d.text(
        x + 6.0,
        y + 3.0,
        label,
        10.0,
        if selected {
            Color::srgba(1.0, 0.95, 0.75, 1.0)
        } else {
            Color::srgba(0.88, 0.91, 0.94, 1.0)
        },
    );
    d.hits.push(crate::Hit {
        pos: Vec2::new(x + w * 0.5, y + ROW_H * 0.5),
        size: Vec2::new(w, ROW_H),
        action,
    });
}

/// Left/right pager plus an optional trailing action.
fn pager(
    d: &mut Painter<'_, '_, '_>,
    x: f32,
    y: f32,
    w: f32,
    page: usize,
    pages: usize,
    trailing: Option<(&str, Action)>,
) {
    const ARROW: f32 = 20.0;
    row(d, x, y, ARROW, "<", Action::Page(-1), false);
    d.text(
        x + ARROW + 6.0,
        y + 4.0,
        format!("{}/{pages}", page + 1),
        9.0,
        Color::srgba(0.70, 0.76, 0.82, 1.0),
    );
    row(d, x + ARROW + 36.0, y, ARROW, ">", Action::Page(1), false);
    if let Some((label, action)) = trailing {
        let rest = w - (ARROW + 36.0 + ARROW + 6.0);
        row(d, x + w - rest, y, rest, label, action, false);
    }
}

/// Draw one corner or section dot and register its click target.
///
/// Everything clickable in the rail goes through here, so there is exactly
/// one place that can forget to make a dot clickable.
fn dot(
    d: &mut Painter<'_, '_, '_>,
    center: Vec2,
    image: Handle<Image>,
    tint: Color,
    icon: Option<(Icon, Color)>,
    action: Action,
    selected: bool,
    hovered: bool,
) {
    if selected {
        d.circle(center, DOT * 0.95, Color::srgba(0.45, 0.78, 1.0, 0.30));
    } else if hovered {
        d.circle(center, DOT * 0.75, Color::srgba(1.0, 1.0, 1.0, 0.20));
    }
    d.image(
        center,
        Vec2::splat(DOT),
        image,
        // The menu and close buttons carry their own art and pass white, so
        // they are never recoloured; the section dots pass their own tint and
        // keep it when selected, because the halo above already says which one
        // is open.
        if hovered { Color::WHITE } else { tint },
    );
    if let Some((icon, ink)) = icon {
        draw_icon(d, center, icon, tint, ink);
    }
    d.hits.push(crate::Hit {
        pos: center,
        size: Vec2::splat(TOUCH),
        action,
    });
}

/// Draw a section glyph inside a dot.
///
/// `tint` is the dot's own colour: a few shapes need it to punch a hole, and
/// the icon must stay readable without a hover tooltip.
fn draw_icon(d: &mut Painter<'_, '_, '_>, center: Vec2, icon: Icon, tint: Color, ink: Color) {
    let (x, y) = (center.x, center.y);
    let bar = |d: &mut Painter<'_, '_, '_>, x: f32, y: f32, w: f32, h: f32| {
        d.rect(x, y, w, h, ink);
    };
    match icon {
        // A page with two lines of text.
        Icon::File => {
            let (w, h, t) = (7.0, 9.0, 1.2);
            bar(d, x - w * 0.5, y - h * 0.5, w, t);
            bar(d, x - w * 0.5, y + h * 0.5 - t, w, t);
            bar(d, x - w * 0.5, y - h * 0.5, t, h);
            bar(d, x + w * 0.5 - t, y - h * 0.5, t, h);
            bar(d, x - 2.0, y - 1.6, 4.0, 1.0);
            bar(d, x - 2.0, y + 1.4, 4.0, 1.0);
        }
        // A painter's palette: a ring with two dabs of colour.
        Icon::Skins => {
            d.circle(center, 5.0, ink);
            d.circle(center, 3.1, tint);
            d.circle(Vec2::new(x - 1.4, y - 1.6), 1.0, ink);
            d.circle(Vec2::new(x + 1.5, y + 1.3), 1.0, ink);
        }
        // Four items on a shelf.
        Icon::Items => {
            for (dx, dy) in [(-2.0, -2.0), (2.0, -2.0), (-2.0, 2.0), (2.0, 2.0)] {
                bar(d, x + dx - 1.5, y + dy - 1.5, 3.0, 3.0);
            }
        }
        // A spark: two offset bars of different heights.
        Icon::Modes => {
            bar(d, x - 2.2, y - 4.5, 1.8, 9.0);
            bar(d, x + 0.4, y - 2.5, 1.8, 5.0);
        }
        // Three sliders, each with its knob somewhere different.
        Icon::Settings => {
            for (row, knob) in [(-3.0, -1.5), (0.0, 1.5), (3.0, 0.0)] {
                bar(d, x - 3.5, y + row - 0.5, 7.0, 1.0);
                d.circle(Vec2::new(x + knob, y + row), 1.3, ink);
            }
        }
        Icon::Help => {
            d.text_centered(x, y - 0.5, "?".to_string(), 11.0, ink);
        }
        // Three stacked bars: the menu.
        Icon::Menu => {
            for row in -1..=1 {
                bar(d, x - 3.5, y + row as f32 * 3.2 - 0.5, 7.0, 1.1);
            }
        }
        // Two crossing strokes: the close button.
        Icon::Close => {
            for step in -2..=2 {
                let o = step as f32 * 1.3;
                d.rect(x + o - 0.6, y + o - 0.6, 1.2, 1.2, ink);
                d.rect(x + o - 0.6, y - o - 0.6, 1.2, 1.2, ink);
            }
        }
        Icon::Passthrough => {
            // Pointer arrow, then a bar across it: "clicks go through here".
            d.rect(x - 2.4, y - 1.8, 1.2, 4.2, ink);
            d.rect(x - 2.4, y + 2.4, 2.4, 1.2, ink);
            d.rect(x - 0.6, y + 0.6, 1.2, 1.8, ink);
            for step in 0..5 {
                let o = step as f32 * 1.1;
                d.rect(x - 3.0 + o, y - 3.0 + o, 1.0, 1.0, ink);
            }
        }
    }
}

pub(crate) fn draw(d: &mut Painter<'_, '_, '_>, g: &Game) {
    d.z += 500.0;

    // Panel first, so the corner buttons and their captions stay on top of it.
    if g.desktop_open && !g.desktop_section.is_empty() {
        let (center, size) = panel_rect(g);
        let x = center.x - size.x * 0.5;
        let y = center.y - size.y * 0.5;
        d.rect(x, y, size.x, size.y, Color::srgba(0.035, 0.045, 0.06, 0.90));
        match g.desktop_section.as_str() {
            "Items" => draw_items(d, g, x, y),
            "Skins" | "Modes" => draw_catalog(d, g, x, y, &g.desktop_section.clone()),
            "Settings" => draw_settings(d, g, x, y),
            "Help" => draw_help(d, x, y),
            _ => draw_file(d, g, x, y),
        }
    }

    if g.desktop_open {
        for index in 0..SECTIONS.len() {
            let Some((label, icon, tint, action)) = rail_entry(index) else {
                break;
            };
            let center = rail_center(g, index);
            let hovered = under(d, center, Vec2::splat(TOUCH));
            dot(
                d,
                center,
                d.art.circle.clone(),
                tint,
                Some((icon, Color::srgba(0.97, 0.98, 1.0, 0.95))),
                action,
                rail_label(index) == g.desktop_section,
                hovered,
            );
            if hovered {
                caption(d, g, center, label.to_string(), false);
            }
        }
    }

    // The two buttons that are always there, side by side in the corner. The
    // close button lives outside the rail so it is never a mis-click away.
    // Both are drawn in the same idiom as the section dots: a tinted disc with
    // a drawn glyph, rather than a piece of button art.
    for (center, tint, icon, action, label, danger) in [
        (
            menu_center(g),
            // Amber, not green, while clicks pass straight through. The colour
            // is the whole signal: this button is the only way back into a
            // menu that passthrough has hidden, so its state has to be legible
            // without clicking anything.
            if g.save.mouse_passthrough {
                Color::srgba(0.80, 0.60, 0.20, 0.95)
            } else {
                Color::srgba(0.34, 0.58, 0.42, 0.95)
            },
            if g.save.mouse_passthrough {
                Icon::Passthrough
            } else {
                Icon::Menu
            },
            Action::DesktopToggle,
            if g.save.mouse_passthrough {
                "Click-through is on"
            } else {
                ""
            },
            false,
        ),
        (
            close_center(g),
            Color::srgba(0.72, 0.26, 0.28, 0.95),
            Icon::Close,
            Action::Quit,
            "Exit",
            true,
        ),
    ] {
        let hovered = under(d, center, Vec2::splat(TOUCH));
        dot(
            d,
            center,
            d.art.circle.clone(),
            tint,
            Some((icon, Color::srgba(0.97, 0.98, 1.0, 0.95))),
            action,
            false,
            hovered,
        );
        if hovered {
            let text = if label.is_empty() && g.desktop_open {
                "Close menu"
            } else if label.is_empty() {
                "Menu"
            } else {
                label
            };
            caption(d, g, center, text.to_string(), danger);
        }
    }

    if g.save.show_overlay {
        status_line(d, g);
    }
}

/// Small right-aligned label beside a rail dot, so the icons stay discoverable
/// without permanent text. Non-interactive.
fn caption(d: &mut Painter<'_, '_, '_>, g: &Game, center: Vec2, label: String, danger: bool) {
    let width = label.len() as f32 * 5.4 + 12.0;
    let height = 15.0;
    // Sit the caption to the left of the whole corner cluster.
    let right = close_center(g).x - DOT * 0.5 - 4.0;
    let x = (right - width).max(2.0);
    let y = center.y - height * 0.5;
    plate(d, x, y, width, height);
    d.text(
        x + 6.0,
        y + 3.0,
        label,
        9.0,
        if danger {
            Color::srgb(1.0, 0.72, 0.68)
        } else {
            Color::srgba(0.92, 0.95, 0.98, 1.0)
        },
    );
}

/// One-line readout at the bottom-left of the work area: what Buddy is, what
/// you have, and whether desktop play is armed. Non-interactive, so it never
/// intercepts a click meant for the desktop.
fn status_line(d: &mut Painter<'_, '_, '_>, g: &Game) {
    let tool = if g.save.item.is_empty() {
        "None"
    } else {
        g.save.item.as_str()
    };
    let mode = if !g.desktop {
        "windowed"
    } else if g.save.mouse_passthrough {
        "passthrough (Ctrl+Alt+P)"
    } else if crate::desktop_blocked_by(g).is_some() {
        // "armed" told the player nothing. This says what broke and what to
        // press, and it sits still on screen rather than appearing once and
        // fading before they notice.
        "right-click puts tool away"
    } else if g.desktop_interact {
        "armed"
    } else {
        "desktop"
    };
    let text = format!(
        "Buddy  ${:.2}  ·  {tool}  ·  {mode}{}",
        g.save.cash,
        if g.paused { "  ·  paused" } else { "" }
    );
    let width = text.chars().count() as f32 * 5.3 + 16.0;
    let top = g.stage.y - 21.0;
    plate(d, 6.0, top, width, 16.0);
    d.text(
        14.0,
        top + 3.0,
        text,
        9.5,
        Color::srgba(0.80, 0.86, 0.92, 1.0),
    );
}

fn draw_file(d: &mut Painter<'_, '_, '_>, g: &Game, x: f32, y: f32) {
    let inner = PANEL_W - PANEL_PAD * 2.0;
    let mut top = y + PANEL_PAD;
    heading(
        d,
        x + PANEL_PAD,
        top,
        "Buddy",
        &format!("${:.2}", g.save.cash),
    );
    top += HEAD_H;
    for (label, action) in [
        (if g.paused { "Resume" } else { "Pause" }, Action::Pause),
        ("Reset Buddy", Action::ResetBuddy),
        ("Clear toys  ·  C", Action::ClearToys),
        ("Loadout / Favorites...", Action::Panel("Loadout".into())),
        ("Stats", Action::Panel("Stats".into())),
        ("Clear Progress...", Action::Panel("Clear File".into())),
        (
            if g.desktop {
                "Switch to window"
            } else {
                "Switch to desktop"
            },
            Action::ToggleSurface,
        ),
        ("Restore Windows", Action::RestoreWindows),
        // The rail's close button is for the corner; this is the in-panel way
        // to quit for anyone who never looks that far.
        ("Quit Buddy", Action::Quit),
    ] {
        row(d, x + PANEL_PAD, top, inner, label, action, false);
        top += ROW_PITCH;
    }
    if g.save_error.is_some() {
        d.text(
            x + PANEL_PAD,
            top + 2.0,
            "Save failed - check permissions",
            9.0,
            Color::srgb(1.0, 0.62, 0.35),
        );
    }
}

fn draw_items(d: &mut Painter<'_, '_, '_>, g: &Game, x: f32, y: f32) {
    const CATS: [&str; 9] = [
        "Pet Items",
        "Explosives",
        "God Powers",
        "Guns",
        "Hand",
        "Miscellaneous",
        "Objects",
        "Special",
        "Favorites",
    ];
    let inner = PANEL_W - PANEL_PAD * 2.0;
    let left = x + PANEL_PAD;
    let mut top = y + PANEL_PAD;
    heading(
        d,
        left,
        top,
        "Tools",
        &format!("{} owned", g.save.owned.len()),
    );
    top += HEAD_H;

    let chip_w = (inner - 4.0) * 0.5;
    let cats: Vec<_> = CATS
        .iter()
        .copied()
        .filter(|c| *c != "Pet Items" || g.save.pet.enabled)
        .collect();
    for (i, category) in cats.iter().enumerate() {
        let cx = left + (i % 2) as f32 * (chip_w + 4.0);
        let cy = top + (i / 2) as f32 * 17.0;
        let selected = g.category == *category;
        let hover = under(
            d,
            Vec2::new(cx + chip_w * 0.5, cy + 8.0),
            Vec2::new(chip_w, 15.0),
        );
        d.rect(
            cx,
            cy,
            chip_w,
            15.0,
            if selected {
                Color::srgba(0.20, 0.33, 0.45, 0.96)
            } else if hover {
                Color::srgba(0.19, 0.23, 0.28, 0.96)
            } else {
                Color::srgba(0.085, 0.10, 0.125, 0.92)
            },
        );
        d.text(
            cx + 5.0,
            cy + 3.0,
            *category,
            9.0,
            Color::srgba(0.85, 0.89, 0.93, 1.0),
        );
        d.hits.push(crate::Hit {
            pos: Vec2::new(cx + chip_w * 0.5, cy + 7.5),
            size: Vec2::new(chip_w, 15.0),
            action: Action::Category((*category).into()),
        });
    }
    top += cats.len().div_ceil(2) as f32 * 17.0 + 8.0;

    let category = if cats.contains(&g.category.as_str()) {
        g.category.as_str()
    } else {
        "Hand"
    };
    let entries = g
        .catalog
        .items
        .iter()
        .filter(|e| g.in_category(e, category) && g.save.owned.contains(&e.name))
        .collect::<Vec<_>>();
    let pages = entries.len().div_ceil(ITEM_PAGE).max(1);
    let page = g.page.min(pages - 1);
    for (offset, entry) in entries
        .iter()
        .skip(page * ITEM_PAGE)
        .take(ITEM_PAGE)
        .enumerate()
    {
        row(
            d,
            left,
            top + offset as f32 * ROW_PITCH,
            inner,
            &entry.name,
            Action::Select("Items".into(), entry.name.clone()),
            g.save.item == entry.name,
        );
    }
    top += ITEM_PAGE as f32 * ROW_PITCH;
    pager(
        d,
        left,
        top,
        inner,
        page,
        pages,
        Some(("Shop", Action::Store("Items".into()))),
    );
}

fn draw_catalog(d: &mut Painter<'_, '_, '_>, g: &Game, x: f32, y: f32, kind: &str) {
    let inner = PANEL_W - PANEL_PAD * 2.0;
    let left = x + PANEL_PAD;
    let mut top = y + PANEL_PAD;
    let entries = if kind == "Skins" {
        &g.catalog.skins
    } else {
        &g.catalog.modes
    };
    let owned = entries
        .iter()
        .filter(|e| g.save.owned.contains(&e.name))
        .collect::<Vec<_>>();
    let pages = owned.len().div_ceil(CATALOG_PAGE).max(1);
    let page = g.page.min(pages - 1);
    heading(d, left, top, kind, &format!("{} owned", owned.len()));
    top += HEAD_H + 4.0;
    for (offset, entry) in owned
        .iter()
        .enumerate()
        .skip(page * CATALOG_PAGE)
        .take(CATALOG_PAGE)
    {
        let active = if kind == "Skins" {
            g.save.skin == entry.id
        } else {
            g.save.modes.contains(&entry.id)
        };
        row(
            d,
            left,
            top + (offset - page * CATALOG_PAGE) as f32 * ROW_PITCH,
            inner,
            &entry.name,
            Action::Select(kind.into(), entry.name.clone()),
            active,
        );
    }
    top += CATALOG_PAGE as f32 * ROW_PITCH;
    pager(
        d,
        left,
        top,
        inner,
        page,
        pages,
        Some((&format!("Shop {kind}"), Action::Store(kind.into()))),
    );
    if kind == "Skins" {
        row(
            d,
            left,
            top + ROW_PITCH,
            inner,
            "Skin Maker...",
            Action::Panel("Skin Maker".into()),
            false,
        );
    }
}

fn draw_settings(d: &mut Painter<'_, '_, '_>, g: &Game, x: f32, y: f32) {
    let inner = PANEL_W - PANEL_PAD * 2.0;
    let left = x + PANEL_PAD;
    let mut top = y + PANEL_PAD;
    heading(d, left, top, "Settings", "");
    top += HEAD_H;
    row(
        d,
        left,
        top,
        inner,
        if g.save.sound {
            "Sound: On"
        } else {
            "Sound: Off"
        },
        Action::ToggleSound,
        false,
    );
    top += ROW_PITCH;
    row(
        d,
        left,
        top,
        inner,
        if g.desktop_interact {
            "Screen interactions: On"
        } else {
            "Screen interactions: Off"
        },
        Action::DesktopInteract,
        g.desktop_interact,
    );
    top += ROW_PITCH;
    row(
        d,
        left,
        top,
        inner,
        if g.save.show_overlay {
            "Bottom-left overlay: On"
        } else {
            "Bottom-left overlay: Off"
        },
        Action::ToggleOverlay,
        g.save.show_overlay,
    );
    top += ROW_PITCH;
    row(
        d,
        left,
        top,
        inner,
        if g.save.mouse_passthrough {
            "Mouse passthrough: On"
        } else {
            "Mouse passthrough: Off"
        },
        Action::TogglePassthrough,
        g.save.mouse_passthrough,
    );
    top += ROW_PITCH;
    row(
        d,
        left,
        top,
        inner,
        "Physics...",
        Action::Panel("Physics".into()),
        false,
    );
    top += ROW_PITCH;
    row(
        d,
        left,
        top,
        inner,
        "Dialogue options...",
        Action::Panel("Dialogue".into()),
        false,
    );
    top += ROW_PITCH;
    for (label, panel) in [
        ("Passive Pet...", "Passive Pet"),
        ("Skin Maker...", "Skin Maker"),
    ] {
        row(
            d,
            left,
            top,
            inner,
            label,
            Action::Panel(panel.into()),
            false,
        );
        top += ROW_PITCH;
    }
    top += 4.0;
    for key in ["Anti-Aliasing", "Effects", "Motion Blur"] {
        let value = match key {
            "Anti-Aliasing" => &g.save.anti_aliasing,
            "Effects" => &g.save.effects,
            _ => &g.save.motion_blur,
        };
        let options = if key == "Motion Blur" {
            ["None", "Weak", "Strong"]
        } else {
            ["Low", "Medium", "High"]
        };
        let next = options
            .iter()
            .position(|v| *v == value)
            .map_or(options[0], |index| options[(index + 1) % options.len()]);
        row(
            d,
            left,
            top,
            inner,
            &format!("{key}: {value}  >"),
            Action::Setting(key.into(), next.into()),
            false,
        );
        top += ROW_PITCH;
    }
}

fn draw_help(d: &mut Painter<'_, '_, '_>, x: f32, y: f32) {
    let inner = PANEL_W - PANEL_PAD * 2.0;
    let left = x + PANEL_PAD;
    let mut top = y + PANEL_PAD;
    heading(d, left, top, "Help", "");
    top += HEAD_H + 4.0;
    for label in ["Help", "What's New?", "About"] {
        row(
            d,
            left,
            top,
            inner,
            label,
            Action::Panel(label.into()),
            false,
        );
        top += ROW_PITCH;
    }
    // The keybind is the only escape from a weapon swallowing every click, so
    // it has to be written down somewhere a player can actually read.
    top += 6.0;
    d.text(
        left,
        top,
        "Right-click puts the tool away;",
        9.0,
        Color::srgba(0.565, 0.612, 0.678, 1.0),
    );
    d.text(
        left,
        top + 11.0,
        "your desktop its clicks back.",
        9.0,
        Color::srgba(0.565, 0.612, 0.678, 1.0),
    );
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::Save;

    fn game() -> Game {
        let mut g = Game::new();
        // `Game::new` reads the player's save file, so without this the tests
        // would inherit whatever the last real session left behind.
        g.save = Save::default();
        g.stage = Vec2::new(1280., 688.);
        g.desktop = true;
        g.desktop_open = true;
        g.desktop_section = String::new();
        g
    }

    /// The rail must never reach into the desktop gutter or the taskbar, and
    /// the corner buttons must sit in the bottom-right, not the top-right.
    #[test]
    fn rail_stays_inside_the_work_area() {
        let g = game();
        for center in [menu_center(&g), close_center(&g)]
            .into_iter()
            .chain((0..SECTIONS.len()).map(|i| rail_center(&g, i)))
        {
            assert!(
                center.x + DOT * 0.5 <= g.stage.x && center.x - DOT * 0.5 >= 0.,
                "a dot left the work area horizontally at {center:?}"
            );
            assert!(
                center.y + DOT * 0.5 <= g.stage.y && center.y - DOT * 0.5 >= 0.,
                "a dot left the work area vertically at {center:?}"
            );
        }
        let menu = menu_center(&g);
        assert!(
            menu.y > g.stage.y * 0.5,
            "the menu belongs in the bottom-right corner, not the top"
        );
        assert!(
            menu.x > g.stage.x * 0.5,
            "the menu belongs on the right, not the left"
        );
    }

    /// The panel belongs directly left of the rail, on screen, never on top of
    /// it and never wrapped to the opposite edge.
    #[test]
    fn panel_sits_left_of_the_rail_and_inside_the_work_area() {
        let g = game();
        let rail_left = g.stage.x - RAIL_RIGHT - DOT;
        for section in ["File", "Items", "Skins", "Modes", "Settings", "Help"] {
            let mut g = game();
            g.desktop_section = section.into();
            let (center, size) = panel_rect(&g);
            let left = center.x - size.x * 0.5;
            let right = center.x + size.x * 0.5;
            let top = center.y - size.y * 0.5;
            let bottom = center.y + size.y * 0.5;
            assert!(
                right <= rail_left,
                "{section} panel overlaps the rail: right={right} rail_left={rail_left}"
            );
            assert!(left >= 0., "{section} panel wrapped off screen at {left}");
            assert!(
                bottom <= g.stage.y + 0.5,
                "{section} panel runs past the taskbar at {bottom}"
            );
            assert!(
                top >= 0.,
                "{section} panel runs past the top of the work area at {top}"
            );
            assert!(
                (size.y - panel_height(&g)).abs() < 0.5,
                "{section} panel is {} tall but its content needs {}; rows would spill outside the plate",
                size.y,
                panel_height(&g)
            );
        }
    }

    /// Every dot must be clickable, and nothing around it may be.
    #[test]
    fn only_the_dots_and_open_panel_take_clicks() {
        let mut g = game();
        for center in [menu_center(&g), close_center(&g)]
            .into_iter()
            .chain((0..SECTIONS.len()).map(|i| rail_center(&g, i)))
        {
            assert!(hit(&g, center), "a dot at {center:?} does not take clicks");
            // Probe away from the cluster: the close button's right-hand side
            // is the menu button, which is meant to take clicks.
            let away = if (center - close_center(&g)).abs().x < 0.01 {
                Vec2::new(-TOUCH * 0.6, 0.)
            } else {
                Vec2::new(TOUCH * 0.6, 0.)
            };
            assert!(
                !hit(&g, center + away),
                "space beside {center:?} eats clicks"
            );
        }
        let above = rail_center(&g, 1) - Vec2::new(0., TOUCH * 0.6);
        assert!(!hit(&g, above), "the gap between dots eats clicks");
        let (center, _) = panel_rect(&g);
        assert!(!hit(&g, center), "empty panel area eats clicks");
        g.desktop_section = "Help".into();
        let (center, size) = panel_rect(&g);
        assert!(hit(&g, center), "open panel must take clicks");
        assert!(
            !hit(&g, center + Vec2::new(0., size.y * 0.5 + 4.)),
            "panel is taller than it draws"
        );
        assert!(
            !hit(&g, center - Vec2::new(size.x * 0.5 + 4., 0.)),
            "panel is wider than it draws"
        );
    }

    /// Collapsed state: only the two corner buttons are live, and the rail
    /// dots above them must be dead desktop.
    #[test]
    fn collapsed_menu_shows_only_the_corner_buttons() {
        let mut g = game();
        g.desktop_open = false;
        assert!(hit(&g, menu_center(&g)), "the menu button must take clicks");
        assert!(
            hit(&g, close_center(&g)),
            "the close button must take clicks"
        );
        assert!(
            !hit(&g, rail_center(&g, 0)),
            "a section dot is still live with the menu collapsed"
        );
        assert!(
            !hit(&g, menu_center(&g) + Vec2::new(0., TOUCH * 0.6 + 4.)),
            "the corner buttons eat the desktop"
        );
    }

    /// The close button sits beside the menu button, not in the rail.
    #[test]
    fn close_button_sits_beside_the_menu_button() {
        let g = game();
        let menu = menu_center(&g);
        let close = close_center(&g);
        assert!(
            (close.y - menu.y).abs() < 0.01,
            "the close button must line up with the menu button"
        );
        assert!(
            menu.x - close.x >= DOT,
            "the close button overlaps the menu button"
        );
        assert!(
            !hit(&g, (menu + close) * 0.5),
            "the gap between the corner buttons must stay click-through"
        );
        // It is outside the section column, which is why it is not one of them:
        // every rail dot shares the menu button's x, and the close button does
        // not.
        assert!(
            (rail_center(&g, 0).x - menu.x).abs() < 0.01,
            "the section column must line up with the menu button"
        );
        assert!(
            (close.x - menu.x).abs() > 0.01,
            "the close button belongs beside the menu, not in the rail"
        );
    }

    /// Passthrough hands the whole overlay back to the desktop.
    #[test]
    fn passthrough_hides_every_control() {
        let mut g = game();
        g.save.mouse_passthrough = true;
        // The room, the rail and the panel all let clicks through...
        for center in (0..SECTIONS.len())
            .map(|i| rail_center(&g, i))
            .chain([panel_rect(&g).0])
        {
            assert!(
                !hit(&g, center),
                "{center:?} still takes clicks while passthrough is on"
            );
        }
        // ...but the corner buttons do not. They are the only way back into the
        // menu, and passthrough once had no way out at all: it locked these
        // too, while the Ctrl+Alt+P hotkey its own status line advertised had
        // never been implemented. The player was stuck.
        for center in [menu_center(&g), close_center(&g)] {
            assert!(
                hit(&g, center),
                "{center:?} must stay clickable, or passthrough is a one-way door"
            );
        }
    }

    /// Whatever passthrough is set to, the player must always be able to get
    /// back into the menu.
    #[test]
    fn the_menu_is_reachable_in_every_mode() {
        for passthrough in [false, true] {
            for open in [false, true] {
                for section in ["", "Items", "Settings"] {
                    let mut g = game();
                    g.save.mouse_passthrough = passthrough;
                    g.desktop_open = open;
                    g.desktop_section = section.into();
                    assert!(
                        hit(&g, menu_center(&g)),
                        "menu unreachable with passthrough={passthrough} \
                         open={open} section={section:?}"
                    );
                }
            }
        }
    }
}
