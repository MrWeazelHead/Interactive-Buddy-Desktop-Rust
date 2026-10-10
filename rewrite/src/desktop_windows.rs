//! Native top-level-window interactions for desktop mode.
//!
//! Explosions capture a window only into memory, render that snapshot as
//! falling fragments, then minimize the target. Restore brings minimized
//! windows back to their captured rectangles. Other impacts briefly shake a
//! window. No window is closed, killed, or written to disk.

use bevy::{
    app::AppExit,
    asset::RenderAssetUsages,
    color::Alpha,
    prelude::{
        App, Assets, Color, Commands, Component, Entity, Image, MessageReader, MessageWriter,
        Plugin, Query, Res, ResMut, Resource, Sprite, Time, Transform, Vec2, Vec3,
    },
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
#[cfg(windows)]
use std::mem::size_of;

use crate::{Game, desktop};

/// Screen-window rectangle converted to the desktop simulation's stage units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowRect {
    pub id: usize,
    pub min: Vec2,
    pub max: Vec2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImpactKind {
    Explosion,
    Hit,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Impact {
    pub point: Vec2,
    pub power: f32,
    pub kind: ImpactKind,
}

/// Add this plugin once to run window refresh, impact effects and debris.
pub struct DesktopWindowsPlugin;

impl Plugin for DesktopWindowsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NativeState>();
        app.add_systems(bevy::prelude::Update, update_windows);
        app.add_systems(bevy::prelude::Last, restore_on_exit);
    }
}

#[derive(Component)]
struct Debris {
    velocity: Vec2,
    angular_velocity: f32,
    life: f32,
    max_life: f32,
}

#[derive(Clone, Copy)]
struct ScreenRect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[derive(Clone)]
struct ScreenWindow {
    id: usize,
    rect: ScreenRect,
    stage: WindowRect,
}

#[derive(Clone, Copy)]
struct RestoreWindow {
    id: usize,
    rect: ScreenRect,
    maximized: bool,
}

struct ShakeWindow {
    id: usize,
    rect: ScreenRect,
    remaining: f32,
    amplitude: f32,
    phase: f32,
}

#[derive(Resource, Default)]
struct NativeState {
    refresh_elapsed: f32,
    enumerated: bool,
    smoke_mode: Option<SmokeMode>,
    windows: Vec<ScreenWindow>,
    restore: Vec<RestoreWindow>,
    shakes: Vec<ShakeWindow>,
    test: EffectsTest,
}

#[derive(Default)]
struct EffectsTest {
    phase: TestPhase,
    fixture: usize,
    original: Option<ScreenRect>,
    elapsed: f32,
    snapshot_expected: bool,
    debris_before: usize,
    diagnostic_elapsed: f32,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum TestPhase {
    #[default]
    Waiting,
    HitQueued,
    HitWait,
    ExplosionWait,
    RestoreWait,
    DebrisWait,
    Finished,
    Failed,
}

impl Drop for NativeState {
    fn drop(&mut self) {
        restore_windows(&mut self.restore);
        restore_shakes(&mut self.shakes);
        destroy_fixture(&mut self.test.fixture);
    }
}

fn update_windows(
    _main_thread: bevy::ecs::system::NonSendMarker,
    mut commands: Commands,
    mut g: ResMut<Game>,
    mut images: ResMut<Assets<Image>>,
    time: Res<Time>,
    mut state: ResMut<NativeState>,
    mut debris: Query<(Entity, &mut Transform, &mut Debris, &mut Sprite)>,
    mut exit: MessageWriter<AppExit>,
) {
    tick_debris(&mut commands, time.delta_secs(), &mut debris);

    if g.desktop_restore {
        restore_windows(&mut state.restore);
        g.desktop_restore = false;
    }

    if !g.desktop {
        restore_windows(&mut state.restore);
        restore_shakes(&mut state.shakes);
        state.windows.clear();
        state.refresh_elapsed = 0.0;
        state.enumerated = false;
        g.desktop_windows.clear();
        g.desktop_impacts.clear();
        return;
    }

    let Some(area) = g.desktop_area else {
        g.desktop_impacts.clear();
        return;
    };
    let smoke = *state.smoke_mode.get_or_insert_with(smoke_effect_mode);

    if smoke == SmokeMode::FixtureOnly && state.test.fixture == 0 {
        if g.desktop_handle.is_some() {
            log_test_line("windows:before-create-fixture");
            match create_fixture(area) {
                Some((hwnd, rect)) => {
                    state.test.fixture = hwnd;
                    state.test.original = Some(rect);
                    #[cfg(windows)]
                    unsafe {
                        // Keep the known white fixture immediately behind the
                        // overlay so GetPixel tests actual desktop composition.
                        SetWindowPos(
                            hwnd as Hwnd,
                            g.desktop_handle.unwrap() as Hwnd,
                            0,
                            0,
                            0,
                            0,
                            SWP_NOSIZE | 0x0002 | SWP_NOACTIVATE,
                        );
                    }
                    log_test_line(&format!("windows:after-create-fixture hwnd={hwnd:#x}"));
                }
                None => {
                    log_test_line("windows:create-fixture-failed");
                    fail_effects_test(&mut state, &mut exit)
                }
            }
        }
    }
    state.test.elapsed += time.delta_secs();
    if smoke == SmokeMode::FixtureOnly
        && state.test.elapsed > 20.0
        && !matches!(state.test.phase, TestPhase::Finished | TestPhase::Failed)
    {
        fail_effects_test(&mut state, &mut exit);
    }

    state.refresh_elapsed += time.delta_secs();
    if !state.enumerated || state.refresh_elapsed >= 0.5 {
        state.refresh_elapsed = 0.0;
        state.enumerated = true;
        if smoke == SmokeMode::FixtureOnly {
            log_test_line("windows:before-enumerate");
        }
        state.windows = enumerate_windows(
            g.desktop_handle.unwrap_or_default(),
            area,
            g.stage,
            smoke == SmokeMode::FixtureOnly,
        );
        if smoke == SmokeMode::FixtureOnly {
            log_test_line(&format!(
                "windows:after-enumerate count={}",
                state.windows.len()
            ));
        }
        g.desktop_windows = state.windows.iter().map(|w| w.stage).collect();
    }

    if smoke == SmokeMode::FixtureOnly {
        state.test.diagnostic_elapsed += time.delta_secs();
        if state.test.diagnostic_elapsed >= 0.5 {
            state.test.diagnostic_elapsed = 0.0;
            let shake = state.shakes.iter().any(|s| s.id == state.test.fixture);
            log_effects_test(
                &state.test,
                state.windows.len(),
                g.desktop_handle.unwrap_or_default(),
                area,
                shake,
            );
        }
    }

    tick_shakes(&mut state.shakes, time.delta_secs());
    if smoke == SmokeMode::FixtureOnly {
        let state = &mut *state;
        drive_effects_test(&mut state.test, &state.windows, &state.shakes, area, &mut g);
    }
    let impacts = std::mem::take(&mut g.desktop_impacts);
    for impact in impacts {
        if !impact.point.is_finite() || !impact.power.is_finite() || impact.power <= 0.0 {
            continue;
        }
        if smoke == SmokeMode::Disabled {
            continue;
        }
        let Some(target) = choose_target(&state.windows, impact).cloned() else {
            continue;
        };
        match impact.kind {
            ImpactKind::Explosion => {
                let snapshot = capture_window(target.id, target.rect);
                if smoke == SmokeMode::FixtureOnly && target.id == state.test.fixture {
                    state.test.snapshot_expected = snapshot.is_some();
                    state.test.debris_before = debris.iter().count();
                }
                if let Some(shot) = snapshot {
                    spawn_snapshot_debris(&mut commands, &mut images, &g, &target, impact, shot);
                } else {
                    spawn_glass_debris(&mut commands, &target, impact);
                }
                let restore_rect = state
                    .shakes
                    .iter()
                    .find(|shake| shake.id == target.id)
                    .map_or_else(
                        || live_rect(target.id).unwrap_or(target.rect),
                        |shake| shake.rect,
                    );
                minimize_window(target.id, restore_rect, &mut state.restore);
                state.shakes.retain(|shake| shake.id != target.id);
                state.windows.retain(|w| w.id != target.id);
                g.desktop_windows.retain(|w| w.id != target.id);
            }
            ImpactKind::Hit => {
                shake_window(target.id, target.rect, impact.power, &mut state.shakes);
                if smoke == SmokeMode::FixtureOnly && target.id == state.test.fixture {
                    state.test.phase = TestPhase::HitWait;
                }
            }
        }
    }
    if smoke == SmokeMode::FixtureOnly {
        finish_effects_test(&mut state.test, area, &mut g, &mut debris, &mut exit);
    }
}

fn drive_effects_test(
    test: &mut EffectsTest,
    windows: &[ScreenWindow],
    shakes: &[ShakeWindow],
    area: desktop::WorkArea,
    g: &mut Game,
) {
    if test.phase == TestPhase::Waiting {
        if test.elapsed < 3.0 {
            return;
        }
        let Some(rect) = test.original else {
            return;
        };
        #[cfg(windows)]
        unsafe {
            DwmFlush();
            let dc = GetDC(core::ptr::null_mut());
            let pixel = GetPixel(dc, rect.left + 100, rect.top + 100);
            ReleaseDC(core::ptr::null_mut(), dc);
            log_test_line(&format!(
                "NATIVE COMPOSITION: background pixel={pixel:#010x}, expected=0x00ffffff"
            ));
            assert_eq!(
                pixel, 0x00ffffff,
                "desktop overlay hides its white background fixture"
            );
            let dc = GetDC(core::ptr::null_mut());
            let marker = GetPixel(dc, area.left + 210, area.top + 210);
            ReleaseDC(core::ptr::null_mut(), dc);
            log_test_line(&format!(
                "NATIVE COMPOSITION: overlay marker={marker:#010x}, expected=0x000000ff"
            ));
            assert_eq!(marker, 0x000000ff, "desktop overlay content is invisible");
        }
        let Some(window) = windows.iter().find(|w| w.id == test.fixture) else {
            return;
        };
        let center = (window.stage.min + window.stage.max) * 0.5;
        g.desktop_impacts.push(Impact {
            point: center,
            power: 1.0,
            kind: ImpactKind::Hit,
        });
        test.phase = TestPhase::HitQueued;
    } else if test.phase == TestPhase::HitWait {
        if test.fixture != 0
            && test.original.is_some()
            && !shakes.iter().any(|s| s.id == test.fixture)
            && rect_matches(test.fixture, test.original.unwrap())
        {
            let original = test.original.unwrap();
            let center = screen_to_stage(
                (original.left + original.right) as f32 * 0.5,
                (original.top + original.bottom) as f32 * 0.5,
                area,
            );
            g.desktop_impacts.push(Impact {
                point: center,
                power: 1.0,
                kind: ImpactKind::Explosion,
            });
            test.phase = TestPhase::ExplosionWait;
        }
    } else if test.phase == TestPhase::ExplosionWait && is_iconic(test.fixture) {
        g.desktop_restore = true;
        test.phase = TestPhase::RestoreWait;
    }
    if test.phase == TestPhase::Finished || test.phase == TestPhase::Failed {
        return;
    }
}

fn finish_effects_test(
    test: &mut EffectsTest,
    _area: desktop::WorkArea,
    _g: &mut Game,
    debris: &mut Query<(Entity, &mut Transform, &mut Debris, &mut Sprite)>,
    exit: &mut MessageWriter<AppExit>,
) {
    if test.phase == TestPhase::RestoreWait
        && test.fixture != 0
        && !is_iconic(test.fixture)
        && test
            .original
            .is_some_and(|rect| rect_matches(test.fixture, rect))
    {
        if test.snapshot_expected {
            test.phase = TestPhase::DebrisWait;
        } else {
            pass_effects_test(test, exit);
        }
    } else if test.phase == TestPhase::DebrisWait && debris.iter().count() > test.debris_before {
        pass_effects_test(test, exit);
    }
}

fn pass_effects_test(test: &mut EffectsTest, exit: &mut MessageWriter<AppExit>) {
    log_test_line("WINDOW EFFECTS TEST PASSED");
    destroy_fixture(&mut test.fixture);
    test.phase = TestPhase::Finished;
    println!("WINDOW EFFECTS TEST PASSED");
    exit.write(AppExit::Success);
}

fn fail_effects_test(test: &mut NativeState, exit: &mut MessageWriter<AppExit>) {
    restore_windows(&mut test.restore);
    restore_shakes(&mut test.shakes);
    destroy_fixture(&mut test.test.fixture);
    test.test.phase = TestPhase::Failed;
    log_test_line("WINDOW EFFECTS TEST FAILED");
    eprintln!("WINDOW EFFECTS TEST FAILED");
    exit.write(AppExit::Error(std::num::NonZeroU8::new(1).unwrap()));
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SmokeMode {
    Disabled,
    FixtureOnly,
    Normal,
}

fn log_effects_test(
    test: &EffectsTest,
    windows: usize,
    overlay: usize,
    area: desktop::WorkArea,
    shake: bool,
) {
    log_test_line(&format!(
        "elapsed={:.1} phase={:?} fixture={:#x} overlay={:#x} windows={} area=({},{}..{}, {}) shake={} iconic={} original={:?}",
        test.elapsed,
        test.phase,
        test.fixture,
        overlay,
        windows,
        area.left,
        area.top,
        area.right,
        area.bottom,
        shake,
        test.fixture != 0 && is_iconic(test.fixture),
        test.original.map(|r| (r.left, r.top, r.right, r.bottom)),
    ));
}

fn log_test_line(line: &str) {
    use std::io::Write;
    let _ = std::fs::create_dir_all("screenshots");
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("screenshots/window-effects-test.log")
    {
        let _ = writeln!(file, "{line}");
    }
}

fn smoke_effect_mode() -> SmokeMode {
    let mut smoke = false;
    let mut fixture = false;
    for arg in std::env::args() {
        match arg.as_str() {
            "--smoke-test" | "--desktop-smoke-test" => smoke = true,
            "--window-effects-test" => fixture = true,
            _ => {}
        }
    }
    if fixture {
        SmokeMode::FixtureOnly
    } else if smoke {
        SmokeMode::Disabled
    } else {
        SmokeMode::Normal
    }
}

#[cfg(windows)]
fn create_fixture(area: desktop::WorkArea) -> Option<(usize, ScreenRect)> {
    const WIDTH: i32 = 400;
    const HEIGHT: i32 = 280;
    let max_x = (area.right - WIDTH).max(area.left);
    let max_y = (area.bottom - HEIGHT).max(area.top);
    let x = (area.left + 48).min(max_x);
    let y = (area.top + 48).min(max_y);
    let title: Vec<u16> = "Interactive Buddy Window Effects Test Fixture"
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let class: Vec<u16> = "STATIC".encode_utf16().chain(Some(0)).collect();
    let instance = unsafe { GetModuleHandleW(core::ptr::null()) };
    let hwnd = unsafe {
        CreateWindowExW(
            0,
            class.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPEDWINDOW | 0x0006, // SS_WHITERECT: known background for composition test
            x,
            y,
            WIDTH,
            HEIGHT,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            instance,
            core::ptr::null_mut(),
        )
    };
    if hwnd.is_null() {
        return None;
    }
    unsafe {
        ShowWindow(hwnd, SW_SHOWNORMAL);
        UpdateWindow(hwnd);
    }
    let mut rect = WinRect::default();
    if unsafe { GetWindowRect(hwnd, &mut rect) } == 0 {
        unsafe {
            DestroyWindow(hwnd);
        }
        return None;
    }
    Some((
        hwnd as usize,
        ScreenRect {
            left: rect.left,
            top: rect.top,
            right: rect.right,
            bottom: rect.bottom,
        },
    ))
}

#[cfg(not(windows))]
fn create_fixture(_area: desktop::WorkArea) -> Option<(usize, ScreenRect)> {
    None
}

fn destroy_fixture(fixture: &mut usize) {
    #[cfg(windows)]
    unsafe {
        if *fixture != 0 {
            DestroyWindow(*fixture as Hwnd);
        }
    }
    *fixture = 0;
}

#[cfg(windows)]
fn live_rect(id: usize) -> Option<ScreenRect> {
    let mut rect = WinRect::default();
    if unsafe { GetWindowRect(id as Hwnd, &mut rect) } == 0 {
        return None;
    }
    Some(ScreenRect {
        left: rect.left,
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
    })
}
#[cfg(not(windows))]
fn live_rect(_id: usize) -> Option<ScreenRect> {
    None
}

#[cfg(windows)]
fn rect_matches(id: usize, expected: ScreenRect) -> bool {
    let mut rect = WinRect::default();
    unsafe {
        GetWindowRect(id as Hwnd, &mut rect) != 0
            && rect.left == expected.left
            && rect.top == expected.top
            && rect.right == expected.right
            && rect.bottom == expected.bottom
    }
}

#[cfg(not(windows))]
fn rect_matches(_id: usize, _expected: ScreenRect) -> bool {
    false
}

#[cfg(windows)]
fn is_iconic(id: usize) -> bool {
    unsafe { id != 0 && IsIconic(id as Hwnd) != 0 }
}
#[cfg(not(windows))]
fn is_iconic(_id: usize) -> bool {
    false
}

fn choose_target(windows: &[ScreenWindow], impact: Impact) -> Option<&ScreenWindow> {
    let radius = if impact.kind == ImpactKind::Explosion {
        (140.0 * impact.power.clamp(0.05, 3.0)).max(18.0)
    } else {
        (45.0 * impact.power.clamp(0.1, 3.0)).max(16.0)
    };
    // EnumWindows returns topmost windows first; the first intersection is the
    // visible window the Buddy actually struck.
    windows.iter().find(|w| {
        if impact.kind == ImpactKind::Hit {
            point_inside(impact.point, w.stage)
        } else {
            distance_to_rect(impact.point, w.stage) <= radius
        }
    })
}

fn point_inside(point: Vec2, rect: WindowRect) -> bool {
    point.x >= rect.min.x && point.x <= rect.max.x && point.y >= rect.min.y && point.y <= rect.max.y
}

fn distance_to_rect(point: Vec2, rect: WindowRect) -> f32 {
    let nearest = point.clamp(rect.min, rect.max);
    nearest.distance(point)
}

fn minimize_window(id: usize, rect: ScreenRect, restore: &mut Vec<RestoreWindow>) {
    #[cfg(windows)]
    unsafe {
        if IsWindow(id as Hwnd) != 0 && IsIconic(id as Hwnd) == 0 {
            if !restore.iter().any(|w| w.id == id) {
                restore.push(RestoreWindow {
                    id,
                    rect,
                    maximized: IsZoomed(id as Hwnd) != 0,
                });
            }
            ShowWindow(id as Hwnd, SW_MINIMIZE);
        }
    }
    #[cfg(not(windows))]
    let _ = (id, rect, restore);
}

fn restore_windows(restore: &mut Vec<RestoreWindow>) {
    #[cfg(windows)]
    unsafe {
        for saved in restore.drain(..) {
            let hwnd = saved.id as Hwnd;
            if IsWindow(hwnd) == 0 {
                continue;
            }
            ShowWindow(hwnd, SW_RESTORE);
            SetWindowPos(
                hwnd,
                core::ptr::null_mut(),
                saved.rect.left,
                saved.rect.top,
                saved.rect.right - saved.rect.left,
                saved.rect.bottom - saved.rect.top,
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
            if saved.maximized {
                ShowWindow(hwnd, SW_SHOWMAXIMIZED);
            }
        }
    }
    #[cfg(not(windows))]
    restore.clear();
}

fn shake_window(id: usize, rect: ScreenRect, power: f32, shakes: &mut Vec<ShakeWindow>) {
    let amplitude = (3.0 + 10.0 * power.clamp(0.1, 2.0)).min(18.0);
    if let Some(existing) = shakes.iter_mut().find(|s| s.id == id) {
        // Keep the first pre-shake rectangle so repeated hits cannot shift the
        // saved origin and accumulate positional drift.
        existing.remaining = 0.20;
        existing.amplitude = amplitude;
        existing.phase = 0.0;
    } else {
        shakes.push(ShakeWindow {
            id,
            rect,
            remaining: 0.20,
            amplitude,
            phase: 0.0,
        });
    }
}

fn tick_shakes(shakes: &mut Vec<ShakeWindow>, dt: f32) {
    #[cfg(windows)]
    unsafe {
        let mut i = 0;
        while i < shakes.len() {
            let s = &mut shakes[i];
            s.remaining -= dt;
            s.phase += dt * 110.0;
            let hwnd = s.id as Hwnd;
            if IsWindow(hwnd) == 0 || IsIconic(hwnd) != 0 {
                shakes.swap_remove(i);
                continue;
            }
            if s.remaining <= 0.0 {
                SetWindowPos(
                    hwnd,
                    core::ptr::null_mut(),
                    s.rect.left,
                    s.rect.top,
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                );
                shakes.swap_remove(i);
            } else {
                let dx = (s.phase.sin() * s.amplitude) as i32;
                let dy = ((s.phase * 1.37).cos() * s.amplitude * 0.45) as i32;
                SetWindowPos(
                    hwnd,
                    core::ptr::null_mut(),
                    s.rect.left + dx,
                    s.rect.top + dy,
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                );
                i += 1;
            }
        }
    }
    #[cfg(not(windows))]
    let _ = (shakes, dt);
}

fn restore_on_exit(
    _main_thread: bevy::ecs::system::NonSendMarker,
    mut exits: MessageReader<AppExit>,
    mut state: ResMut<NativeState>,
) {
    if exits.read().next().is_some() {
        restore_windows(&mut state.restore);
        restore_shakes(&mut state.shakes);
    }
}

fn restore_shakes(shakes: &mut Vec<ShakeWindow>) {
    #[cfg(windows)]
    unsafe {
        for saved in shakes.drain(..) {
            let hwnd = saved.id as Hwnd;
            if IsWindow(hwnd) != 0 && IsIconic(hwnd) == 0 {
                SetWindowPos(
                    hwnd,
                    core::ptr::null_mut(),
                    saved.rect.left,
                    saved.rect.top,
                    saved.rect.right - saved.rect.left,
                    saved.rect.bottom - saved.rect.top,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
            }
        }
    }
    #[cfg(not(windows))]
    shakes.clear();
}

fn spawn_snapshot_debris(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    g: &Game,
    target: &ScreenWindow,
    impact: Impact,
    shot: CapturedWindow,
) {
    let image = Image::new(
        Extent3d {
            width: shot.width,
            height: shot.height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        shot.rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    let handle = images.add(image);
    let area_px = shot.width.saturating_mul(shot.height);
    let tile = 42u32.max(((area_px as f32 / 120.0).sqrt().ceil() as u32).max(1));
    for y in (0..shot.height).step_by(tile as usize) {
        for x in (0..shot.width).step_by(tile as usize) {
            let w = tile.min(shot.width - x);
            let h = tile.min(shot.height - y);
            let screen_x = target.rect.left as f32 + x as f32 + w as f32 * 0.5;
            let screen_y = target.rect.top as f32 + y as f32 + h as f32 * 0.5;
            let stage = screen_to_stage(screen_x, screen_y, g.desktop_area.unwrap());
            let size = Vec2::new(w as f32, h as f32) / 1.5;
            let radial = (stage - impact.point).normalize_or_zero();
            let seed = (x.wrapping_mul(31).wrapping_add(y.wrapping_mul(17)) % 23) as f32 - 11.0;
            let velocity_stage = radial * (55.0 + 32.0 * impact.power.clamp(0.1, 3.0))
                + Vec2::new(seed, -seed * 0.3);
            let world = stage_to_world(stage);
            let sprite = Sprite {
                image: handle.clone(),
                rect: Some(bevy::math::Rect::new(
                    x as f32,
                    y as f32,
                    (x + w) as f32,
                    (y + h) as f32,
                )),
                custom_size: Some(size),
                ..Default::default()
            };
            commands.spawn((
                sprite,
                Transform::from_translation(Vec3::new(world.x, world.y, 80.0)),
                Debris {
                    velocity: Vec2::new(velocity_stage.x, -velocity_stage.y),
                    angular_velocity: seed * 0.12,
                    life: 2.5,
                    max_life: 2.5,
                },
            ));
        }
    }
}

fn spawn_glass_debris(commands: &mut Commands, target: &ScreenWindow, impact: Impact) {
    const COLORS: [Color; 5] = [
        Color::srgba(0.38, 0.68, 0.9, 0.85),
        Color::srgba(0.65, 0.82, 0.96, 0.86),
        Color::srgba(0.27, 0.52, 0.78, 0.88),
        Color::srgba(0.78, 0.9, 0.98, 0.82),
        Color::srgba(0.34, 0.6, 0.8, 0.84),
    ];
    let cols = 6;
    let rows = 4;
    let rect = target.stage;
    let size = Vec2::new(
        (rect.max.x - rect.min.x) / cols as f32,
        (rect.max.y - rect.min.y) / rows as f32,
    );
    for y in 0..rows {
        for x in 0..cols {
            let stage = rect.min + Vec2::new((x as f32 + 0.5) * size.x, (y as f32 + 0.5) * size.y);
            let seed = (x * 13 + y * 7) as f32;
            let outward = (stage - impact.point).normalize_or_zero();
            let vel_stage = outward * (55.0 + impact.power.clamp(0.1, 3.0) * 35.0)
                + Vec2::new((seed % 9.0) - 4.0, -((seed % 7.0) - 3.0));
            let world = stage_to_world(stage);
            commands.spawn((
                Sprite::from_color(COLORS[(x + y * cols) as usize % COLORS.len()], size * 0.97),
                Transform::from_translation(Vec3::new(world.x, world.y, 80.0)),
                Debris {
                    velocity: Vec2::new(vel_stage.x, -vel_stage.y),
                    angular_velocity: (seed % 9.0 - 4.0) * 0.1,
                    life: 2.4,
                    max_life: 2.4,
                },
            ));
        }
    }
}

fn tick_debris(
    commands: &mut Commands,
    dt: f32,
    debris: &mut Query<(Entity, &mut Transform, &mut Debris, &mut Sprite)>,
) {
    for (entity, mut transform, mut part, mut sprite) in debris.iter_mut() {
        part.life -= dt;
        if part.life <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        part.velocity.y -= 480.0 * dt;
        transform.translation.x += part.velocity.x * dt;
        transform.translation.y += part.velocity.y * dt;
        transform.rotate_z(part.angular_velocity * dt);
        sprite.color = sprite
            .color
            .with_alpha((part.life / part.max_life).clamp(0.0, 1.0));
    }
}

fn stage_to_world(stage: Vec2) -> Vec2 {
    Vec2::new(stage.x, -stage.y)
}

fn screen_to_stage(x: f32, y: f32, area: desktop::WorkArea) -> Vec2 {
    Vec2::new((x - area.left as f32) / 1.5, (y - area.top as f32) / 1.5)
}

// Enumeration callback data is deliberately local to the frame; screenshots
// and their pixels are never persisted.
struct EnumContext {
    self_hwnd: Hwnd,
    own_pid: u32,
    area: desktop::WorkArea,
    stage: Vec2,
    fixture_only: bool,
    windows: Vec<ScreenWindow>,
}

#[cfg(windows)]
fn enumerate_windows(
    own: usize,
    area: desktop::WorkArea,
    stage: Vec2,
    fixture_only: bool,
) -> Vec<ScreenWindow> {
    let mut context = EnumContext {
        self_hwnd: own as Hwnd,
        own_pid: unsafe { GetCurrentProcessId() },
        area,
        stage,
        fixture_only,
        windows: Vec::new(),
    };
    unsafe {
        EnumWindows(Some(enum_window_callback), &mut context as *mut _ as isize);
    }
    context.windows
}

#[cfg(not(windows))]
fn enumerate_windows(
    _own: usize,
    _area: desktop::WorkArea,
    _stage: Vec2,
    _fixture_only: bool,
) -> Vec<ScreenWindow> {
    Vec::new()
}

#[cfg(windows)]
unsafe extern "system" fn enum_window_callback(hwnd: Hwnd, param: isize) -> i32 {
    let context = unsafe { &mut *(param as *mut EnumContext) };
    if hwnd == context.self_hwnd
        || unsafe { IsWindowVisible(hwnd) } == 0
        || unsafe { IsIconic(hwnd) } != 0
    {
        return 1;
    }
    let ex = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } as u32;
    if ex & WS_EX_TOOLWINDOW as u32 != 0 {
        return 1;
    }
    let mut class = [0u16; 128];
    let class_len =
        unsafe { GetClassNameW(hwnd, class.as_mut_ptr(), class.len() as i32) }.max(0) as usize;
    let class_name = String::from_utf16_lossy(&class[..class_len]);
    if matches!(
        class_name.as_str(),
        "Progman"
            | "WorkerW"
            | "Shell_TrayWnd"
            | "Shell_SecondaryTrayWnd"
            | "Windows.UI.Core.CoreWindow"
    ) {
        return 1;
    }
    let mut cloaked = 0u32;
    if unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            &mut cloaked as *mut _ as *mut _,
            size_of::<u32>() as u32,
        )
    } == 0
        && cloaked != 0
    {
        return 1;
    }
    let mut title = [0u16; 512];
    let title_len =
        unsafe { GetWindowTextW(hwnd, title.as_mut_ptr(), title.len() as i32) }.max(0) as usize;
    let title = String::from_utf16_lossy(&title[..title_len]);
    if context.fixture_only {
        let mut pid = 0u32;
        unsafe {
            GetWindowThreadProcessId(hwnd, &mut pid);
        }
        if pid != context.own_pid
            || !title.starts_with("Interactive Buddy Window Effects Test Fixture")
        {
            return 1;
        }
    }
    let mut rect = WinRect::default();
    if unsafe { GetWindowRect(hwnd, &mut rect) } == 0
        || rect.right <= rect.left
        || rect.bottom <= rect.top
    {
        return 1;
    }
    let screen = ScreenRect {
        left: rect.left,
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
    };
    let stage_rect = WindowRect {
        id: hwnd as usize,
        min: screen_to_stage(rect.left as f32, rect.top as f32, context.area),
        max: screen_to_stage(rect.right as f32, rect.bottom as f32, context.area),
    };
    // Ignore windows far outside the simulated desktop bounds.
    if stage_rect.max.x < 0.0
        || stage_rect.max.y < 0.0
        || stage_rect.min.x > context.stage.x
        || stage_rect.min.y > context.stage.y
    {
        return 1;
    }
    context.windows.push(ScreenWindow {
        id: hwnd as usize,
        rect: screen,
        stage: stage_rect,
    });
    1
}

struct CapturedWindow {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

#[cfg(windows)]
fn capture_window(hwnd_id: usize, rect: ScreenRect) -> Option<CapturedWindow> {
    let width = u32::try_from(rect.right.checked_sub(rect.left)?).ok()?;
    let height = u32::try_from(rect.bottom.checked_sub(rect.top)?).ok()?;
    if width == 0
        || height == 0
        || width > 4096
        || height > 4096
        || width as u64 * height as u64 > 16_000_000
    {
        return None;
    }
    let hwnd = hwnd_id as Hwnd;
    unsafe {
        if IsWindow(hwnd) == 0 {
            return None;
        }
        let screen_dc = GetDC(core::ptr::null_mut());
        if screen_dc.is_null() {
            return None;
        }
        let memory_dc = CreateCompatibleDC(screen_dc);
        let bitmap = CreateCompatibleBitmap(screen_dc, width as i32, height as i32);
        if memory_dc.is_null() || bitmap.is_null() {
            if !memory_dc.is_null() {
                DeleteDC(memory_dc);
            }
            ReleaseDC(core::ptr::null_mut(), screen_dc);
            return None;
        }
        let old_bitmap = SelectObject(memory_dc, bitmap);
        // Use visible screen pixels only. PrintWindow can block on a hung
        // target UI thread, while BitBlt does not depend on its message loop.
        let ok = BitBlt(
            memory_dc,
            0,
            0,
            width as i32,
            height as i32,
            screen_dc,
            rect.left,
            rect.top,
            SRCCOPY | CAPTUREBLT,
        ) != 0;
        let mut bgra = vec![0u8; width as usize * height as usize * 4];
        let mut info = BitmapInfo::top_down(width as i32, height as i32);
        SelectObject(memory_dc, old_bitmap);
        let lines = if ok {
            GetDIBits(
                screen_dc,
                bitmap,
                0,
                height,
                bgra.as_mut_ptr() as *mut _,
                &mut info,
                DIB_RGB_COLORS,
            )
        } else {
            0
        };
        DeleteObject(bitmap);
        DeleteDC(memory_dc);
        ReleaseDC(core::ptr::null_mut(), screen_dc);
        if lines == 0 {
            return None;
        }
        let mut rgba = bgra;
        for px in rgba.chunks_exact_mut(4) {
            px.swap(0, 2);
            if px[3] == 0 {
                px[3] = 255;
            }
        }
        Some(CapturedWindow {
            width,
            height,
            rgba,
        })
    }
}

#[cfg(not(windows))]
fn capture_window(_hwnd_id: usize, _rect: ScreenRect) -> Option<CapturedWindow> {
    None
}

type Hwnd = *mut core::ffi::c_void;

#[cfg(windows)]
#[repr(C)]
#[derive(Default, Clone, Copy)]
struct WinRect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[cfg(windows)]
#[repr(C)]
struct BitmapInfoHeader {
    size: u32,
    width: i32,
    height: i32,
    planes: u16,
    bit_count: u16,
    compression: u32,
    size_image: u32,
    x_pels_per_meter: i32,
    y_pels_per_meter: i32,
    clr_used: u32,
    clr_important: u32,
}

#[cfg(windows)]
#[repr(C)]
struct BitmapInfo {
    header: BitmapInfoHeader,
    colors: [u32; 1],
}

#[cfg(windows)]
impl BitmapInfo {
    fn top_down(width: i32, height: i32) -> Self {
        Self {
            header: BitmapInfoHeader {
                size: size_of::<BitmapInfoHeader>() as u32,
                width,
                height: -height,
                planes: 1,
                bit_count: 32,
                compression: BI_RGB,
                size_image: 0,
                x_pels_per_meter: 0,
                y_pels_per_meter: 0,
                clr_used: 0,
                clr_important: 0,
            },
            colors: [0],
        }
    }
}

#[cfg(windows)]
const GWL_EXSTYLE: i32 = -20;
#[cfg(windows)]
const WS_EX_TOOLWINDOW: isize = 0x80;
#[cfg(windows)]
const DWMWA_CLOAKED: u32 = 14;
#[cfg(windows)]
const SW_MINIMIZE: i32 = 6;
#[cfg(windows)]
const SW_RESTORE: i32 = 9;
#[cfg(windows)]
const SW_SHOWNORMAL: i32 = 1;
#[cfg(windows)]
const SW_SHOWMAXIMIZED: i32 = 3;
#[cfg(windows)]
const WS_OVERLAPPEDWINDOW: u32 = 0x00CF0000;
#[cfg(windows)]
const SWP_NOSIZE: u32 = 0x0001;
#[cfg(windows)]
const SWP_NOZORDER: u32 = 0x0004;
#[cfg(windows)]
const SWP_NOACTIVATE: u32 = 0x0010;
#[cfg(windows)]
const SRCCOPY: u32 = 0x00CC0020;
#[cfg(windows)]
const CAPTUREBLT: u32 = 0x40000000;
#[cfg(windows)]
#[cfg(windows)]
const DIB_RGB_COLORS: u32 = 0;
#[cfg(windows)]
const BI_RGB: u32 = 0;

#[cfg(windows)]
#[link(name = "user32")]
unsafe extern "system" {
    fn EnumWindows(
        callback: Option<unsafe extern "system" fn(Hwnd, isize) -> i32>,
        lparam: isize,
    ) -> i32;
    fn IsWindow(hwnd: Hwnd) -> i32;
    fn IsWindowVisible(hwnd: Hwnd) -> i32;
    fn IsIconic(hwnd: Hwnd) -> i32;
    fn IsZoomed(hwnd: Hwnd) -> i32;
    fn GetWindowLongPtrW(hwnd: Hwnd, index: i32) -> isize;
    fn GetWindowThreadProcessId(hwnd: Hwnd, process_id: *mut u32) -> u32;
    fn GetClassNameW(hwnd: Hwnd, class_name: *mut u16, max: i32) -> i32;
    fn GetWindowTextW(hwnd: Hwnd, text: *mut u16, max: i32) -> i32;
    fn GetWindowRect(hwnd: Hwnd, rect: *mut WinRect) -> i32;
    fn ShowWindow(hwnd: Hwnd, command: i32) -> i32;
    fn CreateWindowExW(
        ex_style: u32,
        class: *const u16,
        title: *const u16,
        style: u32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        parent: Hwnd,
        menu: Hwnd,
        instance: Hwnd,
        param: *mut core::ffi::c_void,
    ) -> Hwnd;
    fn UpdateWindow(hwnd: Hwnd) -> i32;
    fn DestroyWindow(hwnd: Hwnd) -> i32;
    fn SetWindowPos(hwnd: Hwnd, after: Hwnd, x: i32, y: i32, cx: i32, cy: i32, flags: u32) -> i32;
    fn GetDC(hwnd: Hwnd) -> Hwnd;
    fn ReleaseDC(hwnd: Hwnd, dc: Hwnd) -> i32;
}

#[cfg(windows)]
#[link(name = "gdi32")]
unsafe extern "system" {
    fn GetPixel(dc: Hwnd, x: i32, y: i32) -> u32;
    fn CreateCompatibleDC(dc: Hwnd) -> Hwnd;
    fn CreateCompatibleBitmap(dc: Hwnd, width: i32, height: i32) -> Hwnd;
    fn SelectObject(dc: Hwnd, object: Hwnd) -> Hwnd;
    fn DeleteObject(object: Hwnd) -> i32;
    fn DeleteDC(dc: Hwnd) -> i32;
    fn BitBlt(
        dst: Hwnd,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        src: Hwnd,
        sx: i32,
        sy: i32,
        rop: u32,
    ) -> i32;
    fn GetDIBits(
        dc: Hwnd,
        bitmap: Hwnd,
        start: u32,
        lines: u32,
        bits: *mut core::ffi::c_void,
        info: *mut BitmapInfo,
        usage: u32,
    ) -> i32;
}

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetCurrentProcessId() -> u32;
    fn GetModuleHandleW(module_name: *const u16) -> Hwnd;
}

#[cfg(windows)]
#[link(name = "dwmapi")]
unsafe extern "system" {
    fn DwmFlush() -> i32;
    fn DwmGetWindowAttribute(
        hwnd: Hwnd,
        attribute: u32,
        value: *mut core::ffi::c_void,
        size: u32,
    ) -> i32;
}
