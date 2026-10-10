//! Render-frame smoke test for the Windows desktop overlay.
//!
//! Enable with `--desktop-smoke-test`. The test drives application state
//! directly and captures screenshots; it never moves or clicks the OS pointer.

use bevy::prelude::{AppExit, Commands, Local, MessageWriter, ResMut, Vec2};
use std::io::Write;

use crate::{Action, Game};

fn assert_clickthrough_state(game: &Game, frame: u32) {
    assert!(
        !game.desktop_interact,
        "desktop smoke test unexpectedly armed Play Anywhere at frame {frame}"
    );
    assert!(
        !game.desktop_capture && !game.desktop_raw_down,
        "desktop smoke test unexpectedly captured a native pointer at frame {frame}"
    );
    assert!(
        !game.desktop_keyboard,
        "desktop smoke test unexpectedly requested native keyboard focus at frame {frame}"
    );
}

fn log_frame(frame: u32, game: &Game, passed: bool) {
    let directory = format!("{}/screenshots", env!("CARGO_MANIFEST_DIR"));
    if let Err(error) = std::fs::create_dir_all(&directory) {
        eprintln!("desktop smoke log: could not create {directory}: {error}");
        return;
    }
    let path = format!("{directory}/desktop-smoke.log");
    let result = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|mut file| {
            writeln!(
                file,
                "frame={frame} status={} desktop={} handle={:?} stage={:.0}x{:.0} work_area={:?} open={} interact={} capture={} raw_down={}",
                if passed { "passed" } else { "running" },
                game.desktop,
                game.desktop_handle,
                game.stage.x,
                game.stage.y,
                game.desktop_area,
                game.desktop_open,
                game.desktop_interact,
                game.desktop_capture,
                game.desktop_raw_down,
            )
        });
    if let Err(error) = result {
        eprintln!("desktop smoke log: could not append {path}: {error}");
    }
}

fn save_rgba_capture(
    path: String,
    log_path: String,
    frame: u32,
) -> impl FnMut(bevy::ecs::observer::On<bevy::render::view::screenshot::ScreenshotCaptured>) {
    move |captured| {
        let image = captured
            .image
            .clone()
            .try_into_dynamic()
            .expect("convert desktop smoke screenshot");
        let rgba = image.to_rgba8();
        assert!(
            rgba.width() > 0 && rgba.height() > 0,
            "empty desktop screenshot"
        );
        let corner_alpha = rgba.get_pixel(0, 0).0[3];
        rgba.save(&path)
            .expect("save RGBA desktop smoke screenshot");

        let line = format!(
            "frame={frame} screenshot={} corner_alpha={corner_alpha}\n",
            path
        );
        if let Err(error) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .and_then(|mut file| file.write_all(line.as_bytes()))
        {
            eprintln!("desktop smoke log: could not append {log_path}: {error}");
        }
        assert_eq!(
            corner_alpha, 0,
            "desktop screenshot corner should be fully transparent; saved {path}"
        );
    }
}

/// Bevy `Update` system. Chain into the app's update schedule when running
/// with `--desktop-smoke-test`.
pub fn run(
    mut commands: Commands,
    mut game: ResMut<Game>,
    mut frame: Local<u32>,
    mut exit: MessageWriter<AppExit>,
    frame_image: bevy::prelude::Res<crate::desktop_present::DesktopFrame>,
) {
    if !std::env::args().any(|arg| arg == "--desktop-smoke-test") {
        return;
    }

    *frame += 1;
    let capture = match *frame {
        90 => {
            if game.desktop_open {
                game.action(Action::DesktopToggle);
            }
            if game.desktop_interact {
                game.action(Action::DesktopInteract);
            }
            game.panel.clear();
            game.menu.clear();
            assert_clickthrough_state(&game, *frame);
            Some("desktop-01")
        }
        140 => {
            if !game.desktop_open {
                game.action(Action::DesktopToggle);
            }
            if game.desktop_section != "Items" {
                game.action(Action::DesktopSection("Items".into()));
            }
            assert_clickthrough_state(&game, *frame);
            Some("desktop-02")
        }
        200 => {
            if !game.desktop_open {
                game.action(Action::DesktopToggle);
            }
            if game.desktop_section != "Items" {
                game.action(Action::DesktopSection("Items".into()));
            }
            game.action(Action::Select("Items".into(), "Fist".into()));
            if game.desktop_interact {
                game.action(Action::DesktopInteract);
            }

            // Verlet velocity is pos - prev. Give the body and limbs a small
            // upward/rightward launch with a little differential for rotation.
            for (index, part) in game.buddy.parts.iter_mut().enumerate() {
                let velocity = Vec2::new(7.0, -11.0 + (index as f32 - 2.5) * 0.35);
                part.prev -= velocity;
            }
            assert_clickthrough_state(&game, *frame);
            Some("desktop-03")
        }
        260 => {
            game.action(Action::Store("Items".into()));
            assert_clickthrough_state(&game, *frame);
            Some("desktop-04")
        }
        320 => {
            assert!(
                game.buddy
                    .parts
                    .iter()
                    .all(|part| part.pos.is_finite() && part.prev.is_finite()),
                "desktop smoke test found a non-finite Buddy particle"
            );
            assert!(
                game.stage.is_finite() && game.stage.x >= 550.0 && game.stage.y >= 400.0,
                "desktop smoke test requires a stage at least 550x400; got {:?}",
                game.stage
            );
            assert!(
                game.desktop_handle.is_some(),
                "desktop overlay handle was not initialized"
            );
            assert!(
                game.desktop_area.is_some(),
                "desktop work area was not initialized"
            );
            assert_clickthrough_state(&game, *frame);
            let area = game.desktop_area.expect("validated desktop area");
            println!(
                "DESKTOP SMOKE TEST PASSED: stage={:.0}x{:.0}, work_area={}x{} at ({},{}), clickthrough=idle",
                game.stage.x,
                game.stage.y,
                area.width(),
                area.height(),
                area.left,
                area.top
            );
            exit.write(AppExit::Success);
            None
        }
        _ => None,
    };

    if *frame == 1 || *frame % 30 == 0 || *frame == 320 {
        log_frame(*frame, &game, *frame == 320);
    }

    if let Some(name) = capture {
        let screenshots = format!("{}/screenshots", env!("CARGO_MANIFEST_DIR"));
        std::fs::create_dir_all(&screenshots).expect("create desktop smoke screenshot directory");
        let path = format!("{screenshots}/{name}.png");
        let log_path = format!("{screenshots}/desktop-smoke.log");
        commands
            .spawn(bevy::render::view::screenshot::Screenshot::image(
                frame_image.target.clone().expect("desktop render target"),
            ))
            .observe(save_rgba_capture(path, log_path, *frame));
    }
}
