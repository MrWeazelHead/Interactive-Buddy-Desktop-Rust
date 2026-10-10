//! Desktop overlay input/capture system.

use bevy::prelude::{Commands, Entity, Query, ResMut, Vec2, With};
use bevy::window::{CursorOptions, PrimaryWindow, RawHandleWrapper};

use crate::desktop;
use crate::{Action, Game};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct CaptureUpdate {
    captured: bool,
    pressed: bool,
    released: bool,
}

fn capture_update(captured: bool, previous_down: bool, down: bool, hit: bool) -> CaptureUpdate {
    let pressed = down && !previous_down && hit;
    let released = captured && !down;
    CaptureUpdate {
        captured: if released { false } else { captured || pressed },
        pressed,
        released,
    }
}

/// Poll desktop-level input before the regular game input system. Desktop
/// mouse edges are accepted only when a press starts over a Buddy/UI hit region
/// or while Play Anywhere (`desktop_interact`) is armed.
pub fn update(
    _main_thread: bevy::ecs::system::NonSendMarker,
    mut commands: Commands,
    mut g: ResMut<Game>,
    mut query: Query<(Entity, Option<&RawHandleWrapper>, &mut CursorOptions), With<PrimaryWindow>>,
) {
    trace_window_test("host:update-enter");
    let Ok((entity, wrapper, mut cursor)) = query.single_mut() else {
        return;
    };

    if !g.desktop {
        desktop::set_tool_capture(false);
        cursor.hit_test = true;
        if g.desktop_keyboard {
            if let Some(raw) = g.desktop_handle {
                desktop::set_keyboard_input(raw as desktop::Hwnd, false);
            }
            g.desktop_keyboard = false;
        }
        if g.desktop_capture {
            g.released = true;
        }
        g.desktop_capture = false;
        g.desktop_raw_down = false;
        return;
    }

    if g.desktop_area.is_none() {
        let Some(area) = desktop::work_area() else {
            cursor.hit_test = true;
            return;
        };
        g.stage = Vec2::new(area.width() as f32 / 1.5, area.height() as f32 / 1.5);
        let bounds = g.stage + Vec2::new(0.0, 8.0);
        g.buddy.set_bounds(bounds);
        g.desktop_area = Some(area);
    }

    if g.desktop_handle.is_none() {
        if let Some(hwnd) = wrapper
            .and_then(|raw| desktop::hwnd_from_primary_wrapper(raw, "Interactive Buddy Desktop"))
        {
            trace_window_test("host:before-configure-overlay");
            desktop::configure_overlay(hwnd);
            trace_window_test("host:after-configure-overlay");
            g.desktop_handle = Some(hwnd as usize);
        }
    }
    if g.desktop_handle.is_some() && wrapper.is_some() {
        // The visible Winit HWND remains alive, but the render app now targets
        // an Image and UpdateLayeredWindow presents it. Removing the wrapper
        // prevents Bevy's extractor from creating/presenting a GPU surface over
        // the native layered bitmap.
        commands.entity(entity).remove::<RawHandleWrapper>();
    }

    desktop::set_tool_capture(g.desktop_interact && !g.quit);
    let Some(pointer) = desktop::pointer() else {
        cursor.hit_test = false;
        g.desktop_raw_down = false;
        g.desktop_capture = false;
        g.down = false;
        if let Some(raw) = g.desktop_handle {
            desktop::update_overlay_input_style(raw as desktop::Hwnd, true);
        }
        return;
    };
    if pointer.ctrl_down && pointer.alt_down && pointer.quit_down {
        g.action(Action::Quit);
        return;
    }
    let area = g.desktop_area.expect("desktop area initialized above");
    // The desktop stage is 2/3 of physical pixels; map the Win32 cursor into
    // that same game-space before Buddy/UI hit testing.
    let local = desktop::screen_to_window(pointer.screen, area.origin()).as_vec2() / 1.5;
    g.mouse = local;
    g.auto_aim = pointer.shift_down;
    g.head_aim = pointer.ctrl_down;

    if pointer.escape_down {
        let had_capture = g.desktop_capture;
        g.desktop_interact = false;
        g.panel.clear();
        g.menu.clear();
        g.category.clear();
        g.desktop_capture = false;
        g.pressed = false;
        g.released = had_capture;
        g.down = false;
        g.desktop_raw_down = pointer.left_down;
    } else {
        let update = capture_update(
            g.desktop_capture,
            g.desktop_raw_down,
            pointer.left_down,
            g.desktop_hit(local),
        );
        g.desktop_capture = update.captured;
        g.pressed |= update.pressed;
        g.released |= update.released;
        g.down = update.captured && pointer.left_down;
        g.desktop_raw_down = pointer.left_down;
    }

    // Bevy/Winit implements this on Windows with the correct transparent and
    // layered styles; retain event capture while a press is being dragged.
    let native_hit = g.desktop_hit(local) || g.desktop_capture;
    desktop::set_tool_capture(g.desktop_interact && !g.quit);
    // Keep Winit's cached hit-test state stable; desktop input is polled
    // globally, and the native layered style below owns click-through toggles.
    cursor.hit_test = false;
    if let Some(raw) = g.desktop_handle {
        desktop::update_overlay_input_style(raw as desktop::Hwnd, !native_hit);
    }

    if let Some(raw) = g.desktop_handle {
        let wants_keyboard = g.panel == "Script";
        if wants_keyboard != g.desktop_keyboard {
            desktop::set_keyboard_input(raw as desktop::Hwnd, wants_keyboard);
            // Track the requested state even if Windows refuses foreground
            // activation; this avoids repeatedly stealing focus every frame.
            g.desktop_keyboard = wants_keyboard;
        }
    }
}

fn trace_window_test(event: &str) {
    if !std::env::args().any(|arg| arg == "--window-effects-test") {
        return;
    }
    use std::io::Write;
    let _ = std::fs::create_dir_all("screenshots");
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("screenshots/window-effects-test.log")
    {
        let _ = writeln!(file, "{event}");
    }
}

#[cfg(test)]
mod tests {
    use super::capture_update;

    #[test]
    fn capture_starts_only_on_a_hit_tested_down_edge() {
        assert_eq!(
            capture_update(false, false, true, false),
            Default::default()
        );
        let started = capture_update(false, false, true, true);
        assert!(started.captured && started.pressed && !started.released);
        assert!(capture_update(false, true, true, true).pressed == false);
    }

    #[test]
    fn capture_latches_until_release_even_after_pointer_leaves() {
        let held = capture_update(true, true, true, false);
        assert!(held.captured && !held.pressed && !held.released);
        let released = capture_update(true, true, false, false);
        assert!(!released.captured && released.released && !released.pressed);
    }
}
