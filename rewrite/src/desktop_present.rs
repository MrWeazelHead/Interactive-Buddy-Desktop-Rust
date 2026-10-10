//! Present offscreen Bevy frames through Windows' per-pixel compositor.
use bevy::{
    prelude::*,
    render::view::screenshot::{Screenshot, ScreenshotCaptured},
};

#[derive(Resource, Default)]
pub struct DesktopFrame {
    pub target: Option<Handle<Image>>,
    pub pixels: Option<Vec<u8>>,
    pub width: u32,
    pub height: u32,
    pending: bool,
}

impl Drop for DesktopFrame {
    fn drop(&mut self) {
        crate::desktop::release_tool_capture();
    }
}

pub fn update(
    _main_thread: bevy::ecs::system::NonSendMarker,
    mut commands: Commands,
    game: Res<crate::Game>,
    mut frame: ResMut<DesktopFrame>,
) {
    if !game.desktop || game.quit {
        return;
    }
    let Some(hwnd) = game.desktop_handle else {
        return;
    };
    if let Some(pixels) = frame.pixels.take() {
        if std::env::args().any(|arg| arg == "--window-effects-test") {
            use std::io::Write;
            let offset = ((210 * frame.width + 210) * 4) as usize;
            if let Ok(mut log) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("screenshots/window-effects-test.log")
            {
                let _ = writeln!(
                    log,
                    "PRESENT {}x{} marker={:?}",
                    frame.width,
                    frame.height,
                    pixels.get(offset..offset + 4)
                );
            }
        }
        assert!(
            crate::desktop::present_rgba(
                hwnd as crate::desktop::Hwnd,
                frame.width,
                frame.height,
                &pixels
            ),
            "Windows layered-window presentation failed"
        );
    }
    if frame.pending {
        return;
    }
    let Some(target) = frame.target.clone() else {
        return;
    };
    frame.pending = true;
    commands.spawn(Screenshot::image(target)).observe(
        |captured: On<ScreenshotCaptured>, mut frame: ResMut<DesktopFrame>| {
            let rgba = captured
                .image
                .clone()
                .try_into_dynamic()
                .expect("read desktop frame")
                .to_rgba8();
            frame.width = rgba.width();
            frame.height = rgba.height();
            frame.pixels = Some(rgba.into_raw());
            frame.pending = false;
        },
    );
}
