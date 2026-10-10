#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod behavior;
mod desktop;
mod desktop_host;
mod desktop_present;
mod desktop_smoke;
mod desktop_ui;
mod desktop_windows;
mod objects;
mod physics;
mod script;

use bevy::{audio::Volume, camera::ScalingMode, prelude::*, sprite::Anchor};
use physics::Buddy;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

const W: f32 = 550.;
const H: f32 = 400.;
fn blast(delta: Vec2, power: f32) -> (f32, Vec2) {
    let distance = delta.length() / power + 20. / power - power * 20.;
    let magnitude = (1_000_000. / (distance * distance) - 1.).clamp(0., 150.);
    // atan2(0, 0) in Flash points right, unlike normalize_or_zero().
    let angle = delta.y.atan2(delta.x);
    (distance, Vec2::new(angle.cos(), angle.sin()) * magnitude)
}

/// Find the first window crossed by a swept circular object and return a point
/// on that window's rectangle. Expanding the rectangle by the object's radius
/// approximates circle/AABB contact without missing fast projectiles.
fn swept_window_contact(
    start: Vec2,
    end: Vec2,
    radius: f32,
    windows: &[desktop_windows::WindowRect],
) -> Option<Vec2> {
    let delta = end - start;
    for window in windows {
        let min = window.min - Vec2::splat(radius.max(0.0));
        let max = window.max + Vec2::splat(radius.max(0.0));
        let mut enter: f32 = 0.0;
        let mut leave: f32 = 1.0;
        let mut intersects = true;
        for axis in 0..2 {
            let origin = if axis == 0 { start.x } else { start.y };
            let direction = if axis == 0 { delta.x } else { delta.y };
            let lo = if axis == 0 { min.x } else { min.y };
            let hi = if axis == 0 { max.x } else { max.y };
            if direction.abs() < 1.0e-6 {
                if origin < lo || origin > hi {
                    intersects = false;
                    break;
                }
            } else {
                let a = (lo - origin) / direction;
                let b = (hi - origin) / direction;
                enter = enter.max(a.min(b));
                leave = leave.min(a.max(b));
                if enter > leave {
                    intersects = false;
                    break;
                }
            }
        }
        if intersects && leave >= 0.0 && enter <= 1.0 {
            let center_at_contact = start + delta * enter.clamp(0.0, 1.0);
            return Some(center_at_contact.clamp(window.min, window.max));
        }
    }
    None
}
#[derive(Clone, Deserialize)]
struct Entry {
    name: String,
    #[serde(default)]
    category: String,
    #[serde(default, rename = "type")]
    id: String,
    price: f64,
    #[serde(rename = "initiallyOwned")]
    initially_owned: bool,
}
#[derive(Deserialize)]
struct Catalog {
    items: Vec<Entry>,
    skins: Vec<Entry>,
    modes: Vec<Entry>,
}
#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
struct Save {
    cash: f64,
    earned: f64,
    hits: u64,
    seconds: f64,
    owned: HashSet<String>,
    modes: HashSet<String>,
    item: String,
    skin: String,
    sound: bool,
    gravity: f32,
    #[serde(default = "default_quality")]
    anti_aliasing: String,
    #[serde(default = "default_quality")]
    effects: String,
    #[serde(default = "default_blur")]
    motion_blur: String,
    emotion: f32,
    script_code: String,
}
fn default_quality() -> String {
    "High".into()
}
fn default_blur() -> String {
    "None".into()
}
impl Default for Save {
    fn default() -> Self {
        Self {
            cash: 0.,
            earned: 0.,
            hits: 0,
            seconds: 0.,
            owned: HashSet::new(),
            modes: HashSet::new(),
            item: "Open Hand".into(),
            skin: "default".into(),
            sound: true,
            gravity: 1.,
            anti_aliasing: "High".into(),
            effects: "High".into(),
            motion_blur: "None".into(),
            emotion: 0.,
            script_code: "addBuddyVel((275-getBuddyX())*0.075,(200-getBuddyY())*0.075)".into(),
        }
    }
}
#[derive(Clone)]
enum Action {
    ToggleSurface,
    DesktopToggle,
    DesktopSection(String),
    DesktopInteract,
    Quit,
    RestoreWindows,
    Menu(String),
    Category(String),
    Select(String, String),
    Store(String),
    Row(usize),
    Buy,
    Page(i32),
    Close,
    Reset,
    ResetBuddy,
    ToggleSound,
    Setting(String, String),
    Pause,
    Panel(String),
    Gravity(f32),
    ScriptRun(bool),
    ScriptStop,
    ScriptExample(String),
}
#[derive(Clone)]
struct Hit {
    pos: Vec2,
    size: Vec2,
    action: Action,
}
struct Object {
    pos: Vec2,
    prev: Vec2,
    r: f32,
    kind: String,
    life: f32,
    held: bool,
    hit_timer: i32,
    window_hit_timer: i32,
}
struct Fx {
    pos: Vec2,
    vel: Vec2,
    life: f32,
    max: f32,
    r: f32,
    color: Color,
}
struct Bullet {
    pos: Vec2,
    velocity: Vec2,
    strength: f32,
    ricochets: i32,
    shotgun: bool,
    age: u16,
    window_hit: bool,
}
#[derive(Resource)]
struct Game {
    desktop: bool,
    stage: Vec2,
    desktop_open: bool,
    desktop_section: String,
    desktop_interact: bool,
    desktop_capture: bool,
    desktop_raw_down: bool,
    desktop_area: Option<desktop::WorkArea>,
    desktop_handle: Option<usize>,
    desktop_keyboard: bool,
    quit: bool,
    desktop_restore: bool,
    desktop_windows: Vec<desktop_windows::WindowRect>,
    desktop_impacts: Vec<desktop_windows::Impact>,
    desktop_hit_cooldown: u8,
    buddy: Buddy,
    catalog: Catalog,
    save: Save,
    menu: String,
    category: String,
    panel: String,
    page: usize,
    row: Option<usize>,
    hits: Vec<Hit>,
    mouse: Vec2,
    old_mouse: Vec2,
    down: bool,
    pressed: bool,
    released: bool,
    drag: Option<usize>,
    objects: Vec<Object>,
    fx: Vec<Fx>,
    bullets: Vec<Bullet>,
    bullet_lines: Vec<(Vec2, Vec2)>,
    rng: u64,
    frame: u64,
    cooldown: i32,
    emotion: f32,
    burn: f32,
    message: String,
    message_time: f32,
    paused: bool,
    sounds: Vec<String>,
    engine: script::Engine,
    code: String,
    every_frame: bool,
    save_path: PathBuf,
    save_error: Option<String>,
    demo: bool,
    camera_offset: Vec2,
    camera_scale: f32,
    throw_origin: Vec2,
    direction: Vec2,
    auto_aim: bool,
    head_aim: bool,
    behavior: behavior::Behavior,
    cheat_keys: String,
    flail_pos: Vec2,
    flail_prev: Vec2,
    fist_delay: i32,
    health: f32,
    unconscious: u16,
    shocked: bool,
    render_fps: f32,
}
impl Game {
    fn new() -> Self {
        let catalog: Catalog =
            serde_json::from_str(include_str!("../assets/catalog.json")).expect("catalog");
        let save_path = std::env::var_os("IB_SAVE_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("buddy-save.json"));
        let mut save: Save = std::fs::read_to_string(&save_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        for e in catalog
            .items
            .iter()
            .chain(&catalog.skins)
            .chain(&catalog.modes)
        {
            if e.initially_owned {
                save.owned.insert(e.name.clone());
            }
        }
        let demo = std::env::args()
            .any(|a| a == "--demo" || a == "--smoke-test" || a == "--desktop-smoke-test");
        if demo {
            for e in catalog
                .items
                .iter()
                .chain(&catalog.skins)
                .chain(&catalog.modes)
            {
                save.owned.insert(e.name.clone());
            }
            save.cash = 1000.;
        }
        let emotion = save.emotion;
        let code = save.script_code.clone();
        Self {
            desktop: false,
            stage: Vec2::new(W, H),
            desktop_open: false,
            desktop_section: "Tools".into(),
            desktop_interact: false,
            desktop_capture: false,
            desktop_raw_down: false,
            desktop_area: None,
            desktop_handle: None,
            desktop_keyboard: false,
            quit: false,
            desktop_restore: false,
            desktop_windows: vec![],
            desktop_impacts: vec![],
            desktop_hit_cooldown: 0,
            buddy: Buddy::new(),
            catalog,
            save,
            menu: String::new(),
            category: String::new(),
            panel: String::new(),
            page: 0,
            row: None,
            hits: vec![],
            mouse: Vec2::ZERO,
            old_mouse: Vec2::ZERO,
            down: false,
            pressed: false,
            released: false,
            drag: None,
            objects: vec![],
            fx: vec![],
            bullets: vec![],
            bullet_lines: vec![],
            rng: 0x49b84f29d271a305,
            frame: 0,
            cooldown: 0,
            emotion,
            burn: 0.,
            message: String::new(),
            message_time: 0.,
            paused: false,
            sounds: vec![],
            engine: script::Engine::default(),
            code,
            every_frame: false,
            save_path,
            save_error: None,
            demo,
            camera_offset: Vec2::ZERO,
            camera_scale: 1.,
            flail_pos: Vec2::new(275., 150.),
            flail_prev: Vec2::new(275., 150.),
            fist_delay: 0,
            health: 100.,
            unconscious: 0,
            shocked: false,
            render_fps: 36.,
            throw_origin: Vec2::ZERO,
            direction: Vec2::X,
            auto_aim: false,
            head_aim: false,
            behavior: behavior::Behavior::default(),
            cheat_keys: String::new(),
        }
    }
    fn mode(&self, id: &str) -> bool {
        self.save.modes.contains(id)
    }
    fn reset_buddy(&mut self) {
        self.buddy = Buddy::new();
        self.buddy.set_bounds(
            self.stage
                + if self.desktop {
                    Vec2::new(0., 8.)
                } else {
                    Vec2::ZERO
                },
        );
        if self.desktop {
            let target = Vec2::new(self.stage.x - 140., self.stage.y - 55.);
            let delta = target - self.buddy.parts[5].pos;
            for p in &mut self.buddy.parts {
                p.pos += delta;
                p.prev += delta;
            }
        }
        self.health = 100.;
        self.unconscious = 0;
    }
    fn desktop_hit(&self, point: Vec2) -> bool {
        if point.x < 0. || point.y < 0. || point.x > self.stage.x || point.y > self.stage.y {
            return false;
        }
        if desktop_ui::hit(self, point) {
            return true;
        }
        if !self.panel.is_empty() {
            let origin = self.panel_origin();
            if (point - origin - Vec2::new(275., 203.))
                .abs()
                .cmple(Vec2::new(190., 151.))
                .all()
            {
                return true;
            }
        }
        self.desktop_interact
            || self
                .buddy
                .parts
                .iter()
                .any(|p| p.pos.distance(point) < p.radius + 12.)
            || self
                .objects
                .iter()
                .any(|o| o.pos.distance(point) < o.r + 8.)
    }
    fn panel_origin(&self) -> Vec2 {
        if self.desktop {
            Vec2::new((self.stage.x - 550.).max(0.), (self.stage.y - 450.).max(0.))
        } else {
            Vec2::ZERO
        }
    }
    fn entries(&self, kind: &str) -> Vec<Entry> {
        match kind {
            "Skins" => self.catalog.skins.clone(),
            "Modes" => self.catalog.modes.clone(),
            _ => self.catalog.items.clone(),
        }
    }
    fn notify(&mut self, s: impl Into<String>) {
        self.message = s.into();
        self.message_time = 3.;
    }
    fn cash(&mut self, n: f64) {
        if n.is_finite() && n > 0. {
            self.save.cash += n;
            self.save.earned += n;
        }
    }
    fn sound(&mut self, s: &str) {
        if self.save.sound && self.sounds.len() < 8 {
            self.sounds.push(s.into());
        }
    }
    fn random(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng >> 40) as f32 / 16_777_216.
    }
    fn hit(&mut self, p: Vec2, f: Vec2, r: f32) {
        let strength = self.buddy.impulse(p, f, r);
        if strength > 0.01 {
            self.save.hits += 1;
            self.emotion = (self.emotion - 0.4).max(-100.);
            if self.frame % 120 == 0 {
                match self.save.skin.as_str() {
                    "nap" => self.sound("nd-ugh"),
                    "strawberry" => self.sound("sc-whydoyoudothis"),
                    _ => {}
                }
            }
            if self.mode("gore") && strength > 2. {
                for i in 0..5 {
                    let a = i as f32 * 2.399 + self.frame as f32;
                    self.fx.push(Fx {
                        pos: p,
                        vel: Vec2::new(a.cos(), a.sin()) * 2.,
                        life: 1.3,
                        max: 1.3,
                        r: 1.5,
                        color: Color::srgb(0.55, 0.02, 0.01),
                    });
                }
            }
        }
    }
    fn window_hit(&mut self, point: Vec2, power: f32) {
        if !self.desktop
            || self.desktop_hit_cooldown > 0
            || !point.is_finite()
            || !power.is_finite()
            || power <= 0.0
            || !self
                .desktop_windows
                .iter()
                .any(|window| point.cmpge(window.min).all() && point.cmple(window.max).all())
        {
            return;
        }
        self.desktop_impacts.push(desktop_windows::Impact {
            point,
            power: power.clamp(0.1, 3.0),
            kind: desktop_windows::ImpactKind::Hit,
        });
        self.desktop_hit_cooldown = 7;
    }
    fn window_hit_swept(&mut self, start: Vec2, end: Vec2, radius: f32, power: f32) {
        if !self.desktop || self.desktop_hit_cooldown > 0 {
            return;
        }
        if let Some(point) = swept_window_contact(start, end, radius, &self.desktop_windows) {
            self.window_hit(point, power);
        }
    }
    fn punch(&mut self, cursor: Vec2, delta: Vec2) {
        if delta.length() > 5. {
            self.window_hit(cursor, delta.length() / 30.);
        }
        let swept = |pos: Vec2, radius: f32| {
            (0..=8).any(|i| (cursor - delta * (1. - i as f32 / 8.)).distance(pos) < radius)
        };
        let body_hit = swept(self.buddy.parts[5].pos, 40.);
        let head_hit = swept(self.buddy.parts[0].pos, 20.);
        self.fist_delay -= 1;
        if self.down && body_hit {
            self.drag = Some(5);
        }
        if self.drag == Some(5) {
            if self.down {
                self.buddy.parts[5].pos = cursor;
                self.buddy.parts[5].prev = cursor;
                self.fist_delay = 5;
            } else {
                for part in &mut self.buddy.parts {
                    part.prev = part.pos - delta * 2.;
                }
                self.drag = None;
            }
            return;
        }
        if self.fist_delay > 0 {
            return;
        }
        let speed = delta.length();
        if body_hit && !self.down {
            for part in &mut self.buddy.parts {
                part.prev -= delta * 0.5;
            }
            if speed > 25. {
                self.cash((speed as f64 * 0.002).min(0.5));
                self.emotion = (self.emotion - (speed * 0.002).min(0.35)).max(-100.);
                self.save.hits += 1;
                self.sound("punch1");
            }
        }
        if head_hit {
            self.buddy.parts[0].prev -= delta * 1.2;
            let d = cursor - self.buddy.parts[5].pos;
            let angle = (d + delta).y.atan2((d + delta).x) - d.y.atan2(d.x);
            self.buddy
                .add_rotation_velocity(-angle.abs().mul_add(0.5, 0.).min(0.4) * angle.signum());
            if speed > 20. {
                self.cash((speed as f64 * 0.002).min(0.5));
                self.emotion = (self.emotion - (speed * 0.002).min(0.5)).max(-100.);
                self.save.hits += 1;
                self.sound("punch1");
            }
        }
    }
    fn explode(&mut self, p: Vec2, power: f32) {
        if !p.is_finite() || !power.is_finite() || power <= 0. {
            return;
        }
        if self.desktop {
            self.desktop_impacts.push(desktop_windows::Impact {
                point: p,
                power,
                kind: desktop_windows::ImpactKind::Explosion,
            });
        }
        // frame_25 explode(): the torso determines the shared Buddy velocity
        // and reward. A blast does not independently punch all six circles.
        let (distance, velocity) = blast(self.buddy.parts[5].pos - p, power);
        for part in &mut self.buddy.parts {
            part.prev -= velocity;
        }
        let blast_dx = self.buddy.parts[5].pos.x - p.x;
        let blast_side = if blast_dx > 0.0 {
            1.0
        } else if blast_dx < 0.0 {
            -1.0
        } else {
            0.0
        };
        let spin = -(20. / distance).min(0.75) * blast_side;
        self.buddy.add_rotation_velocity(spin);
        if distance <= 200. {
            self.cash((20. - distance * 0.15) as f64);
            self.emotion = (self.emotion - if distance <= 100. { 5. } else { 2. }).max(-100.);
            self.save.hits += 1;
            if distance <= 100. {
                self.unconscious = 300;
            }
        }
        for object in &mut self.objects {
            let (distance, velocity) = blast(object.pos - p, power);
            if object.kind == "Mines" {
                if distance < power * 100. {
                    object.life = object.life.min(1. / 36.);
                }
            } else {
                let magnitude = velocity.length();
                object.prev -= velocity - Vec2::new(0., magnitude * 0.075);
            }
        }
        let quality = match self.save.effects.as_str() {
            "Low" => 12,
            "Medium" => 24,
            _ => 36,
        };
        for i in 0..if self.mode("pyroMode") {
            quality * 2
        } else {
            quality
        } {
            let a = i as f32 * 2.39996 + self.frame as f32;
            let v = Vec2::new(a.cos(), a.sin()) * (2. + (i % 7) as f32) * power;
            self.fx.push(Fx {
                pos: p,
                vel: v,
                life: 0.6 + (i % 6) as f32 * 0.1,
                max: 1.2,
                r: 3. + (i % 5) as f32,
                color: Color::srgb(1., 0.3 + (i % 3) as f32 * 0.2, 0.02),
            });
        }
        self.sound("explosion");
    }
    fn fire_bullet(&mut self, origin: Vec2, direction: Vec2, weapon: &str) {
        let shotgun = weapon == "Shotgun";
        let speed = if shotgun {
            125. + self.random() * 150.
        } else {
            250.
        };
        let spread = if shotgun {
            0.35
        } else if weapon == "Machine Gun" {
            0.05
        } else {
            0.025
        };
        let angle = (self.random() - 0.5) * spread;
        let velocity = Vec2::new(
            direction.x * angle.cos() - direction.y * angle.sin(),
            direction.x * angle.sin() + direction.y * angle.cos(),
        ) * speed;
        let ricochets = if shotgun {
            (self.random().powi(4) * 4.).floor()
        } else {
            (self.random() * 1.5).floor()
        } as i32;
        if self.bullets.len() < 256 {
            self.bullets.push(Bullet {
                pos: origin,
                velocity,
                strength: if shotgun {
                    0.65
                } else if weapon == "Pistol" {
                    1.25
                } else {
                    1.
                },
                ricochets,
                shotgun,
                age: 0,
                window_hit: false,
            });
        }
    }
    fn advance_bullets(&mut self, gravity: f32) {
        self.bullet_lines.clear();
        // Original doBullets uses sixteen collision samples per game frame.
        for bullet in &mut self.bullets {
            bullet.age += 1;
            for _ in 0..16 {
                let old = bullet.pos;
                bullet.pos += bullet.velocity / 16.;
                bullet.velocity.y += gravity / 16.;
                let part = self
                    .buddy
                    .parts
                    .iter()
                    .position(|p| p.pos.distance_squared(bullet.pos) < p.radius * p.radius);
                if let Some(index) = part {
                    let shared = Vec2::new(bullet.velocity.x * 0.015, bullet.velocity.y * 0.02);
                    for p in &mut self.buddy.parts {
                        p.prev -= shared;
                    }
                    let p = &mut self.buddy.parts[index];
                    p.prev -= bullet.velocity / p.mass * 0.05 * bullet.strength;
                    self.emotion = (self.emotion
                        - if index == 0 {
                            if bullet.shotgun { 1. } else { 4. }
                        } else if index == 5 && bullet.shotgun {
                            0.65
                        } else if bullet.shotgun {
                            0.3
                        } else {
                            1.
                        })
                    .max(-100.);
                    self.save.hits += 1;
                    self.health -= if index == 0 {
                        if bullet.shotgun { 40. } else { 120. }
                    } else if index == 5 {
                        if bullet.shotgun { 10. } else { 20. }
                    } else if index >= 3 {
                        if bullet.shotgun { 5. } else { 10. }
                    } else {
                        0.
                    };
                    bullet.ricochets = -1;
                } else if let Some(object) = self
                    .objects
                    .iter_mut()
                    .find(|o| o.pos.distance_squared(bullet.pos) < o.r * o.r)
                {
                    if matches!(
                        object.kind.as_str(),
                        "Mines" | "Grenades" | "Molotov Cocktails"
                    ) {
                        object.life = object.life.min(1. / 36.);
                    } else {
                        let mass = objects::properties(&object.kind).mass;
                        object.prev -= bullet.velocity / mass * 0.1 * bullet.strength;
                    }
                    bullet.ricochets = -1;
                }
                if !bullet.window_hit
                    && self.desktop
                    && self
                        .desktop_windows
                        .iter()
                        .any(|w| bullet.pos.cmpge(w.min).all() && bullet.pos.cmple(w.max).all())
                {
                    self.desktop_impacts.push(desktop_windows::Impact {
                        point: bullet.pos,
                        power: bullet.strength,
                        kind: desktop_windows::ImpactKind::Hit,
                    });
                    // A desktop window is behind the Buddy, not an opaque
                    // obstacle that absorbs the shot before it reaches him.
                    bullet.window_hit = true;
                }
                if bullet.ricochets >= 0 {
                    if bullet.pos.x > self.stage.x - 4. || bullet.pos.x < 4. {
                        bullet.ricochets -= 1;
                        bullet.velocity.x *= -1.;
                        bullet.pos.x = bullet.pos.x.clamp(4., self.stage.x - 4.);
                    }
                    if bullet.pos.y > self.stage.y - 4. || bullet.pos.y < 30. {
                        bullet.ricochets -= 1;
                        bullet.velocity.y *= -1.;
                        if bullet.pos.y < 30. {
                            bullet.strength *= 0.5;
                        }
                        bullet.pos.y = bullet.pos.y.clamp(30., self.stage.y - 4.);
                    }
                }
                self.bullet_lines.push((old, bullet.pos));
                if bullet.ricochets < 0 {
                    break;
                }
            }
        }
        self.bullets.retain(|b| b.ricochets >= 0 && b.age < 360);
    }
    fn collide_objects(&mut self) {
        let mut reward = 0.;
        let mut emotion = 0.;
        let mut rewarded = false;
        for object in &mut self.objects {
            if object.held || object.kind == "Missiles" || object.life <= 0. {
                continue;
            }
            let properties = objects::properties(&object.kind);
            for (index, part) in self.buddy.parts.iter_mut().enumerate() {
                let incoming = object.pos - object.prev;
                if let Some(c) = objects::collide_with_bounce(
                    object.pos,
                    incoming,
                    properties.mass,
                    object.r,
                    part.pos,
                    part.pos - part.prev,
                    part.mass,
                    part.radius,
                    properties.bounce,
                    0.5,
                ) {
                    object.pos = c.a_pos;
                    object.prev = c.a_pos - c.a_velocity;
                    part.pos = c.b_pos;
                    let transfer = incoming * properties.mass / part.mass;
                    part.prev = part.pos - c.b_velocity - transfer;
                    let speed = transfer.length();
                    if !rewarded
                        && object.hit_timer <= 0
                        && ((index == 0 && speed > 25.) || (index == 5 && speed > 5.))
                    {
                        reward += if index == 0 {
                            (speed as f64 * 0.02).max(3.)
                        } else {
                            (speed as f64 * 0.02).min(2.)
                        };
                        emotion += (-speed * 0.05).min(-2.);
                        object.hit_timer = 60;
                        rewarded = true;
                    }
                    if index == 0 && speed > 50. {
                        self.unconscious = 300;
                    }
                }
            }
        }
        for i in 0..self.objects.len() {
            let (left, right) = self.objects.split_at_mut(i + 1);
            let a = &mut left[i];
            if a.held || a.kind == "Missiles" || a.life <= 0. {
                continue;
            }
            let ap = objects::properties(&a.kind);
            for b in right {
                if b.held || b.kind == "Missiles" || b.life <= 0. {
                    continue;
                }
                let bp = objects::properties(&b.kind);
                if let Some(c) = objects::collide_with_bounce(
                    a.pos,
                    a.pos - a.prev,
                    ap.mass,
                    a.r,
                    b.pos,
                    b.pos - b.prev,
                    bp.mass,
                    b.r,
                    ap.bounce,
                    bp.bounce,
                ) {
                    a.pos = c.a_pos;
                    a.prev = c.a_pos - c.a_velocity;
                    b.pos = c.b_pos;
                    b.prev = c.b_pos - c.b_velocity;
                }
            }
        }
        self.cash(reward);
        self.emotion = (self.emotion + emotion).clamp(-100., 100.);
        if rewarded {
            self.sound("punch1");
            self.save.hits += 1;
        }
    }
    fn gravitate(&mut self, point: Vec2, power: f32, constant: bool) {
        for object in &mut self.objects {
            if matches!(object.kind.as_str(), "Mines" | "Gravity Shifter") {
                continue;
            }
            let d = point - object.pos;
            let distance = d.length().max(0.001);
            let force = if constant {
                power
            } else {
                (3. * (power / distance).powf(1.5)).min(35.)
                    / objects::properties(&object.kind).mass.max(0.2)
            };
            object.prev -= d / distance * force;
        }
        let d = point - self.buddy.parts[5].pos;
        let distance = d.length().max(0.001);
        let force = if constant {
            power
        } else {
            (10. * (power / distance).powf(1.5)).clamp(0., 15.) * (distance * 0.015).clamp(0., 1.)
        };
        for part in &mut self.buddy.parts {
            part.prev -= d / distance * force;
        }
        let body = &mut self.buddy.parts[5];
        let velocity = (body.pos - body.prev)
            * (distance / 35.).min(1.).powi(2)
            * if constant { 0.9 } else { 0.8 };
        body.prev = body.pos - velocity;
    }
    fn shock(&mut self, point: Vec2) {
        let index = [0, 5, 1, 2, 3, 4]
            .into_iter()
            .find(|&i| self.buddy.parts[i].pos.distance(point) < self.buddy.parts[i].radius);
        if let Some(i) = index {
            let part = &mut self.buddy.parts[i];
            let direction = (part.pos - point).normalize_or_zero();
            part.prev -= direction
                * match i {
                    0 => 40.,
                    5 => 20.,
                    1 | 2 => 50.,
                    _ => 30.,
                };
            self.unconscious = 0;
            if !self.shocked {
                self.cash(match i {
                    0 => 0.6,
                    5 => 0.2,
                    1 | 2 => 0.05,
                    _ => 0.1,
                });
                self.emotion = (self.emotion - 1.).max(-100.);
                self.sound("shock");
                self.save.hits += 1;
            }
        }
        self.shocked = index.is_some();
    }
    fn persist(&mut self) {
        if self.demo {
            return;
        }
        self.save.emotion = self.emotion;
        self.save.script_code = self.code.clone();
        match serde_json::to_string_pretty(&self.save)
            .map_err(|e| e.to_string())
            .and_then(|data| {
                let tmp = self.save_path.with_extension("tmp");
                std::fs::write(&tmp, data).map_err(|e| e.to_string())?;
                std::fs::rename(tmp, &self.save_path).map_err(|e| e.to_string())
            }) {
            Ok(_) => self.save_error = None,
            Err(e) => self.save_error = Some(e),
        }
    }
    fn action(&mut self, a: Action) {
        match a {
            Action::ToggleSurface => {
                self.persist();
                let result = std::env::current_exe().and_then(|executable| {
                    let mut child = std::process::Command::new(executable);
                    if self.desktop {
                        child.args(["--classic", "--floating"]);
                    } else {
                        child.arg("--desktop");
                    }
                    if self.demo {
                        child.arg("--demo");
                    }
                    child.spawn()
                });
                match result {
                    Ok(_) => self.quit = true,
                    Err(error) => self.notify(&format!("Could not switch mode: {error}")),
                }
            }
            Action::DesktopToggle => {
                self.desktop_open = !self.desktop_open;
                self.panel.clear();
                self.page = 0;
            }
            Action::DesktopSection(section) => {
                self.desktop_section = if self.desktop_section == section {
                    String::new()
                } else {
                    section
                };
                self.page = 0;
                self.category.clear();
                self.panel.clear();
            }
            Action::DesktopInteract => self.desktop_interact = !self.desktop_interact,
            Action::Quit => {
                self.persist();
                self.quit = true;
            }
            Action::RestoreWindows => self.desktop_restore = true,
            Action::Menu(s) => {
                if self.menu == s {
                    self.menu.clear()
                } else {
                    self.menu = s;
                }
                self.category.clear();
            }
            Action::Category(s) => self.category = s,
            Action::Store(s) => {
                self.panel = s;
                self.page = 0;
                self.row = None;
                self.menu.clear();
            }
            Action::Row(n) => self.row = Some(n),
            Action::Page(n) => {
                if self.desktop && self.panel.is_empty() {
                    let max = desktop_ui::page_count(self).saturating_sub(1);
                    self.page = ((self.page as i32 + n).max(0) as usize).min(max);
                    return;
                }
                let max = self
                    .entries(&self.panel)
                    .len()
                    .div_ceil(10)
                    .saturating_sub(1);
                self.page = ((self.page as i32 + n).max(0) as usize).min(max);
                self.row = None;
            }
            Action::Buy => {
                let entries = self.entries(&self.panel);
                if let Some(i) = self.row {
                    if let Some(e) = entries.get(i) {
                        if !self.save.owned.contains(&e.name) && self.save.cash >= e.price {
                            self.save.cash -= e.price;
                            self.save.owned.insert(e.name.clone());
                            self.sound("register");
                            self.notify(format!("Purchased {}", e.name));
                            self.persist();
                        } else {
                            self.notify("Not enough money, or already owned.");
                        }
                    }
                }
            }
            Action::Select(kind, name) => {
                if kind == "Items" {
                    if self.desktop {
                        self.desktop_interact =
                            !matches!(name.as_str(), "None" | "Open Hand" | "Tickle");
                    }
                    self.save.item = name;
                } else if kind == "Skins" {
                    if let Some(e) = self.catalog.skins.iter().find(|e| e.name == name) {
                        self.save.skin = e.id.clone();
                    }
                } else if let Some(e) = self.catalog.modes.iter().find(|e| e.name == name) {
                    let id = e.id.clone();
                    if id == "script" {
                        self.panel = "Script".into();
                    } else if !self.save.modes.remove(&id) {
                        self.save.modes.insert(id);
                    }
                }
                self.menu.clear();
                self.persist();
            }
            Action::Close => {
                self.panel.clear();
                self.menu.clear();
                self.persist();
            }
            Action::Reset => {
                let initial = Self::new();
                self.save = Save::default();
                self.save.owned = initial
                    .catalog
                    .items
                    .iter()
                    .chain(&initial.catalog.skins)
                    .chain(&initial.catalog.modes)
                    .filter(|e| e.initially_owned)
                    .map(|e| e.name.clone())
                    .collect();
                self.reset_buddy();
                self.objects.clear();
                self.bullets.clear();
                self.panel.clear();
                self.persist();
            }
            Action::ResetBuddy => {
                self.reset_buddy();
                self.objects.clear();
                self.bullets.clear();
                self.burn = 0.;
                self.health = 100.;
                self.unconscious = 0;
                self.menu.clear();
            }
            Action::ToggleSound => {
                self.save.sound = !self.save.sound;
                self.menu.clear();
                self.persist();
            }
            Action::Setting(key, value) => {
                match key.as_str() {
                    "Anti-Aliasing" => self.save.anti_aliasing = value,
                    "Effects" => self.save.effects = value,
                    "Motion Blur" => self.save.motion_blur = value,
                    _ => {}
                }
                self.persist();
            }
            Action::Pause => {
                self.paused = !self.paused;
                self.menu.clear();
            }
            Action::Panel(s) => {
                self.panel = s;
                self.menu.clear();
            }
            Action::Gravity(g) => {
                self.save.gravity = g;
                self.persist();
            }
            Action::ScriptRun(repeat) => {
                self.engine.reset_first_run();
                self.every_frame = repeat;
                if !repeat {
                    self.run_script();
                }
            }
            Action::ScriptStop => self.every_frame = false,
            Action::ScriptExample(code) => self.code = code,
        }
    }
    fn run_script(&mut self) {
        let p = self.buddy.parts[5].pos;
        let ctx = script::Context {
            buddy_x: p.x as f64,
            buddy_y: p.y as f64,
            buddy_rotation: self.buddy.rotation() as f64,
            emotion: self.emotion as f64,
            mouse_x: self.mouse.x as f64,
            mouse_y: self.mouse.y as f64,
            mouse_down: self.down,
            time: self.save.seconds,
        };
        match self.engine.run(&self.code, &ctx) {
            Ok(effects) => {
                for effect in effects {
                    match effect {
                        script::Effect::Cash(v) => {
                            if v.is_finite() {
                                self.save.cash = (self.save.cash + v).max(0.);
                                if v > 0. {
                                    self.save.earned += v;
                                }
                            }
                        }
                        script::Effect::Explode(x, y, p) => {
                            self.explode(Vec2::new(x as f32, y as f32), p as f32)
                        }
                        script::Effect::Velocity(x, y) => {
                            for p in &mut self.buddy.parts {
                                p.prev -= Vec2::new(x as f32, y as f32).clamp_length_max(50.);
                            }
                        }
                        script::Effect::Say(s) => self.notify(s),
                        script::Effect::Create(kind, x, y, vx, vy) => {
                            if self.objects.len() < 100 {
                                self.objects.push(Object {
                                    pos: Vec2::new(x as f32, y as f32),
                                    prev: Vec2::new((x - vx) as f32, (y - vy) as f32),
                                    r: objects::properties(&kind).radius,
                                    kind: match kind.as_str() {
                                        "grenade" => "Grenades",
                                        "molotov" => "Molotov Cocktails",
                                        "mine" => "Mines",
                                        "baseball" => "Baseballs",
                                        "bowlball" => "Bowling Balls",
                                        "rubberball" => "Rubber Balls",
                                        "radio" => "Radio",
                                        "orb" => "Magical Orb",
                                        "vortex" => "Gravity Shifter",
                                        _ => &kind,
                                    }
                                    .into(),
                                    life: if kind == "grenade" {
                                        140. / 36.
                                    } else {
                                        f32::INFINITY
                                    },
                                    held: false,
                                    hit_timer: 0,
                                    window_hit_timer: 0,
                                });
                            }
                        }
                        script::Effect::Gravity(g) => self.save.gravity = (g as f32).clamp(-5., 5.),
                        script::Effect::SetVelocity(x, y) => {
                            let v = Vec2::new(x as f32, y as f32);
                            if v.is_finite() {
                                for p in &mut self.buddy.parts {
                                    p.prev = p.pos - v;
                                }
                            }
                        }
                        script::Effect::SetPosition(x, y) => {
                            let target = Vec2::new(x as f32, y as f32);
                            if target.is_finite() {
                                let delta = target - self.buddy.parts[5].pos;
                                for p in &mut self.buddy.parts {
                                    p.pos += delta;
                                    p.prev += delta;
                                }
                            }
                        }
                        script::Effect::Rotation(r, add) => {
                            if r.is_finite() {
                                let value = r as f32 + if add { self.buddy.rotation() } else { 0. };
                                self.buddy.set_rotation(value);
                            }
                        }
                        script::Effect::Emotion(v) => {
                            if v.is_finite() {
                                self.emotion = (self.emotion + v as f32).clamp(-100., 100.);
                            }
                        }
                        script::Effect::UnlockAll => {
                            for e in self
                                .catalog
                                .items
                                .iter()
                                .chain(&self.catalog.skins)
                                .chain(&self.catalog.modes)
                            {
                                self.save.owned.insert(e.name.clone());
                            }
                            self.persist();
                        }
                        script::Effect::Bullet(x, y, vx, vy, strength, ricochets) => {
                            if [x, y, vx, vy, strength, ricochets]
                                .iter()
                                .all(|v| v.is_finite())
                                && self.bullets.len() < 256
                            {
                                self.bullets.push(Bullet {
                                    pos: Vec2::new(x as f32, y as f32),
                                    velocity: Vec2::new(vx as f32, vy as f32),
                                    strength: strength as f32,
                                    ricochets: ricochets.clamp(0., 100.) as i32,
                                    shotgun: false,
                                    age: 0,
                                    window_hit: false,
                                });
                            }
                        }
                        script::Effect::Sound(name) => self.sound(&name),
                    }
                }
            }
            Err(e) => {
                self.every_frame = false;
                self.notify(e);
            }
        }
    }
}
#[derive(Resource)]
struct Art {
    font: Handle<Font>,
    room: Handle<Image>,
    circle: Handle<Image>,
    skins: HashMap<String, Handle<Image>>,
    sounds: HashMap<String, Handle<AudioSource>>,
    tools: HashMap<String, Handle<Image>>,
    mood: Vec<Handle<Image>>,
}
#[derive(Component)]
struct Drawn;
#[derive(Component)]
struct RadioMusic;
fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    let requested = !args.iter().any(|s| s == "--classic" || s == "--smoke-test")
        || args.iter().any(|s| s == "--desktop");
    if requested {
        desktop::enable_dpi_awareness();
    }
    let mut game = Game::new();
    let mut window = Window {
        title: "Interactive Buddy • Rust / Bevy".into(),
        resolution: (1100, 800).into(),
        window_level: if args.iter().any(|arg| arg == "--floating") {
            bevy::window::WindowLevel::AlwaysOnTop
        } else {
            bevy::window::WindowLevel::Normal
        },
        ..default()
    };
    if requested {
        if let Some(area) = desktop::work_area() {
            game.desktop = true;
            game.desktop_area = Some(area);
            game.stage = Vec2::new(area.width() as f32, area.height() as f32) / 1.5;
            game.desktop_section.clear();
            game.reset_buddy();
            window = Window {
                title: "Interactive Buddy Desktop".into(),
                resolution: bevy::window::WindowResolution::new(area.width(), area.height())
                    .with_scale_factor_override(1.),
                position: bevy::window::WindowPosition::At(area.origin()),
                transparent: true,
                visible: false,
                decorations: false,
                resizable: false,
                focused: false,
                window_level: bevy::window::WindowLevel::AlwaysOnTop,
                skip_taskbar: true,
                ..default()
            };
        }
    }
    let clear = if game.desktop {
        Color::NONE
    } else {
        Color::BLACK
    };
    let executable_assets = std::env::current_exe()
        .ok()
        .and_then(|executable| executable.parent().map(|parent| parent.join("assets")))
        .filter(|path| path.is_dir());
    let assets_path = executable_assets
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets"));
    let desktop_mode = game.desktop;
    App::new()
        .init_resource::<desktop_present::DesktopFrame>()
        .insert_resource(ClearColor(clear))
        .insert_resource(game)
        .insert_resource(Time::<Fixed>::from_hz(36.))
        .insert_resource(if desktop_mode {
            let mode =
                bevy::winit::UpdateMode::reactive_low_power(std::time::Duration::from_millis(28));
            bevy::winit::WinitSettings {
                focused_mode: mode,
                unfocused_mode: mode,
            }
        } else {
            bevy::winit::WinitSettings::continuous()
        })
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(window),
                    ..default()
                })
                .set(AssetPlugin {
                    file_path: assets_path.to_string_lossy().into_owned(),
                    ..default()
                }),
        )
        .add_plugins(desktop_windows::DesktopWindowsPlugin)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                desktop_host::update,
                input,
                smoke_test,
                desktop_smoke::run,
                sampling,
                draw,
                desktop_present::update,
                play_sounds,
                capture,
            )
                .chain(),
        )
        .add_systems(FixedUpdate, simulate)
        .run();
}
fn setup(
    mut c: Commands,
    server: Res<AssetServer>,
    g: Res<Game>,
    mut images: ResMut<Assets<Image>>,
    mut frame: ResMut<desktop_present::DesktopFrame>,
) {
    let camera = c
        .spawn((
            Camera2d,
            Projection::Orthographic(OrthographicProjection {
                scaling_mode: ScalingMode::Fixed {
                    width: g.stage.x,
                    height: g.stage.y,
                },
                ..OrthographicProjection::default_2d()
            }),
            Transform::from_xyz(g.stage.x * 0.5, -g.stage.y * 0.5, 0.),
        ))
        .id();
    if g.desktop {
        use bevy::{
            asset::RenderAssetUsages,
            camera::RenderTarget,
            render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
        };
        let area = g.desktop_area.unwrap();
        let mut image = Image::new_fill(
            Extent3d {
                width: area.width(),
                height: area.height(),
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[0, 0, 0, 0],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );
        image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
            | TextureUsages::COPY_SRC
            | TextureUsages::COPY_DST
            | TextureUsages::RENDER_ATTACHMENT;
        let target = images.add(image);
        frame.target = Some(target.clone());
        c.entity(camera).insert(RenderTarget::Image(target.into()));
    }
    let mut skins = HashMap::new();
    for s in [
        "default",
        "teletubby",
        "pirate",
        "strawberry",
        "goth",
        "rep",
        "dem",
        "baby",
        "gates",
        "tom",
        "nap",
    ] {
        for p in ["head", "rArm", "lArm", "rLeg", "lLeg", "body"] {
            let name = format!("{s}_{p}");
            skins.insert(name.clone(), server.load(format!("skins/{name}.png")));
        }
    }
    let mut sounds = HashMap::new();
    for s in [
        "punch1",
        "punch2",
        "punch3",
        "punch4",
        "explosion",
        "shock",
        "shotgunFire",
        "register",
        "beeper",
        "burnball",
        "pistolFire",
        "machFire",
        "radioMusic",
        "water",
    ] {
        if std::path::Path::new(&format!(
            "{}/assets/sounds/{s}.wav",
            env!("CARGO_MANIFEST_DIR")
        ))
        .exists()
        {
            sounds.insert(s.into(), server.load(format!("sounds/{s}.wav")));
        }
    }
    let mut tool_art = HashMap::new();
    if let Ok(files) = std::fs::read_dir(format!("{}/assets/sounds", env!("CARGO_MANIFEST_DIR"))) {
        for file in files.flatten() {
            let path = file.path();
            if path.extension().is_some_and(|s| s == "wav") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    sounds.insert(name.into(), server.load(format!("sounds/{name}.wav")));
                }
            }
        }
    }
    for name in [
        "fist",
        "grenade",
        "molotov",
        "mine",
        "baseball",
        "bowlball",
        "rubberball",
        "infant",
        "radio",
        "orb",
        "vortex",
        "pistol",
        "machineGun",
        "shotgun",
        "stunGun",
        "flail_head",
        "flail_chain",
        "missile",
        "fireball",
    ] {
        tool_art.insert(name.into(), server.load(format!("tools/{name}.png")));
    }
    c.insert_resource(Art {
        font: server.load("arial.ttf"),
        room: server.load("room.png"),
        circle: server.load("circle.png"),
        skins,
        sounds,
        tools: tool_art,
        mood: (1..=20)
            .map(|n| server.load(format!("mood/{n}.png")))
            .collect(),
    });
}
fn input(
    mut g: ResMut<Game>,
    windows: Query<&Window>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut chars: MessageReader<bevy::input::keyboard::KeyboardInput>,
    mut exit: MessageWriter<AppExit>,
) {
    let Ok(w) = windows.single() else { return };
    if !g.desktop {
        if let Some(p) = w.cursor_position() {
            g.mouse = Vec2::new(p.x / w.width() * g.stage.x, p.y / w.height() * g.stage.y);
        }
        g.down = mouse.pressed(MouseButton::Left);
        g.auto_aim = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
        g.head_aim = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
        g.pressed |= mouse.just_pressed(MouseButton::Left);
        g.released |= mouse.just_released(MouseButton::Left);
    }
    if g.panel != "Script" {
        for (key, ch) in [
            (KeyCode::Space, 's'),
            (KeyCode::ArrowUp, 'u'),
            (KeyCode::ArrowDown, 'd'),
            (KeyCode::ArrowLeft, 'l'),
            (KeyCode::ArrowRight, 'r'),
        ] {
            if keys.just_pressed(key) {
                g.cheat_keys.push(ch);
                if g.cheat_keys.len() > 8 {
                    g.cheat_keys.remove(0);
                }
                match g.cheat_keys.as_str() {
                    "susulrlr" => {
                        g.cash(1000.);
                        g.notify("Cheat accepted!");
                    }
                    "srdldrul" => {
                        let names: Vec<String> = g
                            .catalog
                            .items
                            .iter()
                            .chain(&g.catalog.skins)
                            .chain(&g.catalog.modes)
                            .map(|e| e.name.clone())
                            .collect();
                        g.save.owned.extend(names);
                        g.notify("Cheat accepted!");
                    }
                    "uuusslru" => g.emotion = 100.,
                    "uuusslrd" => g.emotion = -100.,
                    "uuusslrs" => g.emotion = 0.,
                    _ => {}
                }
            }
        }
    }
    if keys.just_pressed(KeyCode::Backspace) && g.panel != "Script" {
        let item = g.save.item.clone();
        g.objects.retain(|o| o.kind != item);
    }
    if keys.just_pressed(KeyCode::Escape) {
        g.panel.clear();
        g.menu.clear();
        g.desktop_interact = false;
    }
    if keys.just_pressed(KeyCode::KeyP) && g.panel != "Script" {
        g.paused = !g.paused;
    }
    if keys.just_pressed(KeyCode::KeyR) && g.panel != "Script" {
        g.reset_buddy();
    }
    if g.panel == "Script" {
        for event in chars.read() {
            if event.state.is_pressed() {
                if event.key_code == KeyCode::Backspace {
                    g.code.pop();
                } else if event.key_code == KeyCode::Enter {
                    g.code.push('\n');
                } else if let Some(t) = &event.text {
                    if g.code.len() < 8000 {
                        g.code.push_str(t);
                    }
                }
            }
        }
    } else {
        chars.clear();
    }
    let clicked = if g.desktop {
        g.pressed
    } else {
        mouse.just_pressed(MouseButton::Left)
    };
    if clicked {
        let action = g
            .hits
            .iter()
            .rev()
            .find(|h| (g.mouse - h.pos).abs().cmple(h.size * 0.5).all())
            .map(|h| h.action.clone());
        if let Some(a) = action {
            g.action(a);
            g.pressed = false;
            g.down = false;
            g.released = false;
            g.drag = None;
        } else if !g.menu.is_empty() {
            g.menu.clear();
            g.category.clear();
            g.pressed = false;
        }
    }
    if g.quit {
        exit.write(AppExit::Success);
    }
    if !g.menu.is_empty() {
        if let Some(a) = g
            .hits
            .iter()
            .rev()
            .find(|h| (g.mouse - h.pos).abs().cmple(h.size * 0.5).all())
            .map(|h| h.action.clone())
        {
            if let Action::Category(s) = a {
                g.category = s;
            }
        }
    }
}
fn simulate(mut g: ResMut<Game>) {
    let stage = g.stage;
    if g.paused || !g.panel.is_empty() || !g.menu.is_empty() {
        g.pressed = false;
        g.released = false;
        return;
    }
    g.frame += 1;
    g.desktop_hit_cooldown = g.desktop_hit_cooldown.saturating_sub(1);
    g.unconscious = g.unconscious.saturating_sub(1);
    if g.health <= 0. {
        g.unconscious = 200;
    }
    g.health = (g.health + 2.).clamp(0., 100.);
    g.buddy.awake = g.unconscious == 0;
    g.buddy.is_clock = g.save.skin == "strawberry";
    g.emotion *= 0.99995;
    g.save.seconds += 1. / 36.;
    g.message_time = (g.message_time - 1. / 36.).max(0.);
    g.cooldown -= 1;
    let grav = if g.objects.iter().any(|o| o.kind == "Gravity Shifter") {
        0.
    } else if g.mode("lowGrav") {
        g.save.gravity * 0.35
    } else {
        g.save.gravity
    };
    let elastic = g.mode("realPhysics");
    g.buddy.open_ceiling = g.mode("openCeil");
    {
        let interaction =
            g.down && g.save.item == "Tickle" && g.buddy.parts[5].pos.distance(g.mouse) < 30.;
        let emotion = g.emotion;
        let burn = g.burn;
        let radio = g.objects.iter().find(|o| o.kind == "Radio").map(|o| o.pos);
        let grenade = g
            .objects
            .iter()
            .filter(|o| o.kind == "Grenades" && !o.held)
            .min_by(|a, b| a.life.total_cmp(&b.life))
            .map(|o| o.pos);
        let mouse = g.mouse;
        let game = &mut *g;
        if game.buddy.awake {
            game.behavior.step(
                &mut game.buddy,
                emotion,
                burn,
                interaction,
                radio,
                grenade,
                mouse,
            );
            let reward = game.behavior.reward;
            game.cash(reward);
        }
    }
    g.buddy.step(grav, elastic);
    let impact_cash = g.buddy.impact_cash;
    g.cash(impact_cash);
    if g.mode("earthquake") {
        let grounded =
            g.buddy.parts[3].pos.y > stage.y - 32. || g.buddy.parts[4].pos.y > stage.y - 32.;
        if grounded {
            for i in 0..g.buddy.parts.len() {
                let velocity = Vec2::new(g.random() * 6. - 3., g.random() * 10. - 5.);
                g.buddy.parts[i].prev -= velocity;
            }
        }
        for i in 0..g.objects.len() {
            if g.objects[i].pos.y > stage.y - 15. - g.objects[i].r {
                let velocity = Vec2::new(g.random() * 6. - 3., g.random() * 10. - 5.);
                g.objects[i].prev -= velocity;
            }
        }
    }
    let center = Vec2::new(stage.x * 0.5, stage.y * 0.5);
    if g.mode("dynCam") && !g.desktop {
        let target = center - g.buddy.parts[5].pos;
        g.camera_offset = g.camera_offset.lerp(target, 0.05);
        g.camera_scale = 1.15;
    } else {
        g.camera_offset = Vec2::ZERO;
        g.camera_scale = 1.;
    }
    let p = (g.mouse - center) / g.camera_scale + center - g.camera_offset;
    if g.pressed {
        g.throw_origin = p;
    }
    let delta = (p - g.old_mouse).clamp_length_max(1000.);
    if g.auto_aim {
        g.direction = (g.buddy.parts[if g.head_aim { 0 } else { 5 }].pos - p).normalize_or_zero();
    } else if delta.length() > 0.5 {
        g.direction = g
            .direction
            .lerp(delta.normalize_or_zero(), 0.325)
            .normalize_or_zero();
    }
    let item = g.save.item.clone();
    if item != "Stun Gun" || !g.down {
        g.shocked = false;
    }
    let in_arena = if g.desktop {
        g.mouse.x >= 0.
            && g.mouse.x <= stage.x
            && g.mouse.y >= 0.
            && g.mouse.y <= stage.y
            && !desktop_ui::hit(&g, g.mouse)
    } else {
        g.mouse.y > 23. && g.mouse.y < 383.
    };
    let active = (g.down || g.pressed || matches!(item.as_str(), "Fist" | "Medieval Flail"))
        && in_arena
        && (!g.desktop
            || g.desktop_interact
            || g.desktop_capture
            || g.buddy.nearest(g.mouse).is_some());
    if active {
        match item.as_str() {
            "None" => {}
            "Open Hand" => {
                if g.pressed {
                    g.drag = g.buddy.nearest(p);
                }
                if let Some(i) = g.drag {
                    g.buddy.drag(i, p);
                    g.buddy.parts[i].prev = p - delta * 0.6;
                }
            }
            "Tickle" => {
                if g.buddy.parts[5].pos.distance(p) < 30. {
                    g.emotion = (g.emotion + 0.05).min(100.);
                    if g.frame % 72 == 0 {
                        g.notify("Hee hee!");
                    }
                }
            }
            "Fist" => g.punch(p, delta),
            "Medieval Flail" => {
                let (hit_pos, velocity) = {
                    let old = g.flail_pos;
                    let velocity = (g.flail_pos - g.flail_prev) * 0.98;
                    g.flail_prev = old;
                    g.flail_pos += velocity + Vec2::new(0., grav);
                    let link = g.flail_pos - p;
                    if link.length() > 75. {
                        g.flail_pos = p + link.normalize_or_zero() * 75.;
                    }
                    (g.flail_pos, g.flail_pos - old)
                };
                g.hit(hit_pos, velocity * 1.5 + Vec2::new(0., -0.7), 40.);
                g.window_hit_swept(hit_pos, hit_pos, 40.0, (velocity.length() / 12.0).max(0.2));
                if delta.length() > 4. && g.buddy.nearest(p).is_some() && g.cooldown <= 0 {
                    g.sound("punch1");
                    g.cooldown = 6;
                }
            }
            "Weak Gravity Vortex" | "Strong Gravity Vortex" => {
                let strong = item.starts_with("Strong");
                g.gravitate(p, if strong { 20. } else { 50. }, strong);
            }
            "Explode At Mouse" => {
                if g.pressed {
                    g.explode(p, 0.85);
                }
            }
            "Missiles" => {
                if g.pressed {
                    let origin = Vec2::new(25. + ((g.frame * 137) % 500) as f32, -100.);
                    let velocity = (p - origin).normalize_or_zero() * 120.;
                    g.objects.push(Object {
                        pos: origin,
                        prev: origin - velocity,
                        r: 4.,
                        kind: "Missiles".into(),
                        life: 8.,
                        held: false,
                        hit_timer: 0,
                        window_hit_timer: 0,
                    });
                }
            }
            "Pistol" | "Shotgun" | "Machine Gun" => {
                if (item == "Machine Gun" && g.cooldown <= 0)
                    || (item != "Machine Gun" && g.pressed)
                {
                    let d = g.direction;
                    for _ in 0..if item == "Shotgun" { 8 } else { 1 } {
                        g.fire_bullet(p, d, &item);
                    }
                    g.sound(if item == "Shotgun" {
                        "shotgunFire"
                    } else if item == "Machine Gun" {
                        "machFire"
                    } else {
                        "pistolFire"
                    });
                    g.cooldown = 5;
                }
            }
            "Stun Gun" => {
                g.shock(p);
                g.window_hit(p, 0.75);
            }
            "Wide Nozzle Hose" | "Narrow Nozzle Hose" | "Fire Hose" | "Flamethrower" => {
                let fire = item == "Flamethrower";
                let shock = item == "Stun Gun";
                let direction = if shock {
                    (g.buddy.parts[5].pos - p).normalize_or_zero()
                } else {
                    g.direction
                };
                g.hit(
                    p,
                    direction
                        * if fire {
                            1.
                        } else if shock {
                            3.
                        } else {
                            2.
                        },
                    if item == "Fire Hose" { 130. } else { 90. },
                );
                let ray = g.buddy.parts[5].pos - p;
                let near_ray = ray.dot(direction) > 0.
                    && (ray - direction * ray.dot(direction)).length() < 40.
                    && ray.length() < 240.;
                if fire && near_ray {
                    if g.burn <= 0. {
                        g.cash(18.);
                    }
                    g.burn = 5.;
                } else if !shock && !fire {
                    g.burn = (g.burn - 0.15).max(0.);
                    g.emotion = (g.emotion + 0.03).min(100.);
                }
                let t = g.frame as f32;
                for i in 0..4 {
                    let v = direction * 8.
                        + Vec2::new((t + i as f32).sin(), (t * 3. + i as f32).cos()) * 2.;
                    g.fx.push(Fx {
                        pos: p,
                        vel: v,
                        life: 0.35,
                        max: 0.35,
                        r: if shock { 1.5 } else { 2.5 },
                        color: if fire {
                            Color::srgb(1., 0.5, 0.05)
                        } else if shock {
                            Color::srgb(0.8, 0.9, 1.)
                        } else {
                            Color::srgb(0.45, 0.7, 0.9)
                        },
                    });
                }
                if shock && g.frame % 12 == 0 {
                    g.sound("shock");
                }
            }
            _ => {
                let special = matches!(item.as_str(), "Radio" | "Gravity Shifter" | "Magical Orb");
                if g.pressed && special {
                    if let Some(o) = g.objects.iter_mut().find(|o| o.kind == item) {
                        o.held = true;
                    }
                }
                if g.pressed
                    && g.objects.len() < 40
                    && !(special && g.objects.iter().any(|o| o.kind == item))
                {
                    if !special
                        && g.objects
                            .iter()
                            .filter(|o| {
                                !matches!(
                                    o.kind.as_str(),
                                    "Radio" | "Gravity Shifter" | "Magical Orb"
                                )
                            })
                            .count()
                            >= 6
                    {
                        if let Some(i) = g.objects.iter().position(|o| {
                            !matches!(o.kind.as_str(), "Radio" | "Gravity Shifter" | "Magical Orb")
                        }) {
                            g.objects.remove(i);
                        }
                    }
                    let r = objects::properties(&item).radius;
                    g.objects.push(Object {
                        pos: p,
                        prev: p,
                        r,
                        kind: item.clone(),
                        life: if item == "Grenades" {
                            140. / 36.
                        } else {
                            f32::INFINITY
                        },
                        held: true,
                        hit_timer: 0,
                        window_hit_timer: 0,
                    });
                }
            }
        }
    }
    if g.released {
        g.drag = None;
        if item == "Machine Gun" {
            g.cooldown = 0;
        }
    }
    if !g.down
        && item == "Open Hand"
        && p.y < stage.y - 40.
        && g.buddy.parts[1].pos.distance(p) < 60.
    {
        let part = &mut g.buddy.parts[1];
        let velocity = part.pos - part.prev + (p - part.pos) * 0.9;
        part.pos = part.pos.lerp(p, 0.2);
        part.prev = part.pos - velocity;
        if g.frame % 3 == 0 {
            g.cash(0.075);
            g.emotion = (g.emotion + 0.15).min(100.);
        }
    }
    g.advance_bullets(grav);
    let mut explosions = vec![];
    let mut ignite = false;
    let body_positions: Vec<(Vec2, f32)> =
        g.buddy.parts.iter().map(|b| (b.pos, b.radius)).collect();
    let open_ceiling = g.mode("openCeil");
    let throw_velocity = (p - g.throw_origin) * 0.225;
    let desktop = g.desktop;
    let window_rects = if desktop {
        g.desktop_windows.clone()
    } else {
        Vec::new()
    };
    let mut window_hits = Vec::new();
    for o in &mut g.objects {
        o.hit_timer = (o.hit_timer - 1).max(0);
        o.window_hit_timer = (o.window_hit_timer - 1).max(0);
        if o.held {
            if active {
                o.pos = p;
                o.prev = p - throw_velocity;
            } else {
                o.held = false;
            }
        } else {
            if o.kind == "Missiles" {
                let v = o.pos - o.prev;
                let mut collided = false;
                for _ in 0..8 {
                    o.pos += v / 8.;
                    if o.pos.y > stage.y - 10.
                        || o.pos.x < 10.
                        || o.pos.x > stage.x - 10.
                        || body_positions
                            .iter()
                            .any(|(b, r)| b.distance(o.pos) < *r + 4.)
                    {
                        collided = true;
                        break;
                    }
                }
                o.prev = o.pos - v;
                if collided {
                    o.life = -1.;
                    explosions.push((o.pos, 0.85));
                }
                continue;
            }
            let properties = objects::properties(&o.kind);
            let (bounce, friction) = (properties.bounce, properties.friction);
            let old_pos = o.pos;
            let v = o.pos - o.prev;
            o.prev = o.pos;
            o.pos += v + Vec2::new(
                0.,
                if matches!(o.kind.as_str(), "Magical Orb" | "Gravity Shifter") {
                    0.
                } else {
                    grav
                },
            );
            if o.pos.y > stage.y - 10. - o.r {
                o.pos.y = stage.y - 11. - o.r;
                o.prev.y = o.pos.y + v.y * bounce;
                o.prev.x = o.pos.x - v.x * friction;
                if o.kind == "Molotov Cocktails" {
                    o.life = -1.;
                }
            }
            if o.pos.x < o.r + 10. || o.pos.x > stage.x - o.r - 10. {
                o.pos.x = o.pos.x.clamp(o.r + 11., stage.x - o.r - 11.);
                o.prev.x = o.pos.x + v.x * bounce;
                o.prev.y = o.pos.y - v.y * friction;
                if o.kind == "Molotov Cocktails" {
                    o.life = -1.;
                }
            }
            if !open_ceiling && o.pos.y < o.r + 10. {
                o.pos.y = o.r + 11.;
                o.prev.y = o.pos.y + v.y * bounce;
                o.prev.x = o.pos.x - v.x * friction;
                if o.kind == "Molotov Cocktails" {
                    o.life = -1.;
                }
            }
            o.life -= 1. / 36.;
            for (bp, br) in &body_positions {
                let d = o.pos - *bp;
                let distance = d.length();
                if distance < o.r + br {
                    if matches!(
                        o.kind.as_str(),
                        "Fireballs" | "fireball" | "Molotov Cocktails"
                    ) {
                        ignite = true;
                    }
                    if matches!(o.kind.as_str(), "Molotov Cocktails" | "Missiles" | "Mines") {
                        o.life = -1.;
                    }
                }
            }
            if matches!(o.kind.as_str(), "Grenades" | "Missiles" | "Mines") && o.life <= 0. {
                explosions.push((o.pos, if o.kind == "Grenades" { 0.7 } else { 0.75 }));
            }
            if desktop
                && o.window_hit_timer <= 0
                && v.length() >= 4.0
                && !matches!(o.kind.as_str(), "Radio" | "Gravity Shifter" | "Magical Orb")
            {
                if let Some(contact) = swept_window_contact(old_pos, o.pos, o.r, &window_rects) {
                    let power = (properties.mass * v.length() / 15.0).clamp(0.2, 3.0);
                    window_hits.push((contact, power));
                    o.window_hit_timer = 30;
                }
            }
        }
    }
    for (point, power) in window_hits {
        g.desktop_impacts.push(desktop_windows::Impact {
            point,
            power,
            kind: desktop_windows::ImpactKind::Hit,
        });
    }
    g.collide_objects();
    let shifter = g
        .objects
        .iter()
        .find(|o| o.kind == "Gravity Shifter")
        .map(|o| o.pos);
    if let Some(pos) = shifter {
        g.gravitate(pos, 1., true);
    }
    let bursts = g
        .objects
        .iter()
        .filter(|o| o.kind == "Molotov Cocktails" && o.life <= 0.)
        .map(|o| o.pos)
        .collect::<Vec<_>>();
    for pos in bursts {
        if g.buddy.parts.iter().any(|p| p.pos.distance(pos) < 65.) {
            ignite = true;
        }
        for i in 0..12 {
            let angle = i as f32 * std::f32::consts::TAU / 12.;
            g.fx.push(Fx {
                pos,
                vel: Vec2::new(angle.cos(), angle.sin()) * 8.,
                life: 0.6,
                max: 0.6,
                r: 5.,
                color: Color::srgb(1., 0.4, 0.05),
            });
        }
        g.sound("burnball");
    }
    if ignite {
        if g.burn <= 0. {
            g.cash(18.);
        }
        g.burn = 5.;
    }
    for (p, s) in explosions {
        g.explode(p, s);
    }
    g.objects.retain(|o| o.life > 0.);
    if g.burn > 0. {
        g.burn -= 1. / 36.;
        let bp = g.buddy.parts[5].pos;
        let t = g.frame as f32;
        g.fx.push(Fx {
            pos: bp + Vec2::new(t.sin() * 20., t.cos() * 20.),
            vel: Vec2::new(0., -2.),
            life: 0.5,
            max: 0.5,
            r: 5.,
            color: Color::srgb(1., 0.4, 0.05),
        });
        g.cash(0.012);
        g.buddy.impulse(bp, Vec2::new(t.sin(), -0.2), 50.);
    }
    for f in &mut g.fx {
        f.pos += f.vel;
        f.vel *= 0.96;
        f.life -= 1. / 36.;
    }
    g.fx.retain(|f| f.life > 0.);
    if g.fx.len() > 600 {
        let n = g.fx.len() - 600;
        g.fx.drain(..n);
    }
    if g.every_frame {
        g.run_script();
    }
    if g.frame % 72 == 0 {
        g.persist();
    }
    g.old_mouse = p;
    g.pressed = false;
    g.released = false;
}

struct Painter<'a, 'w, 's> {
    c: &'a mut Commands<'w, 's>,
    art: &'a Art,
    z: f32,
    hits: Vec<Hit>,
    mouse: Vec2,
    offset: Vec2,
    scale: f32,
    quantize: bool,
    stage: Vec2,
}
fn world(p: Vec2, z: f32) -> Vec3 {
    Vec3::new(p.x, -p.y, z)
}
impl Painter<'_, '_, '_> {
    fn line(&mut self, from: Vec2, to: Vec2, width: f32, color: Color) {
        let delta = to - from;
        self.c.spawn((
            Drawn,
            Sprite::from_color(
                color,
                Vec2::new(delta.length() * self.scale, width * self.scale),
            ),
            Transform::from_translation(world(self.view((from + to) * 0.5), self.z))
                .with_rotation(Quat::from_rotation_z(-delta.y.atan2(delta.x))),
        ));
        self.z += 0.001;
    }
    fn native(&mut self, p: Vec2, image: Handle<Image>, angle: f32) {
        self.c.spawn((
            Drawn,
            Sprite::from_image(image),
            Transform::from_translation(world(self.view(p), self.z))
                .with_scale(Vec3::splat(self.scale))
                .with_rotation(Quat::from_rotation_z(-angle)),
        ));
        self.z += 0.001;
    }
    fn view(&self, p: Vec2) -> Vec2 {
        let p = if self.quantize {
            (p / 12.).floor() * 12. + Vec2::splat(6.)
        } else {
            p
        };
        (p + self.offset - self.stage * 0.5) * self.scale + self.stage * 0.5
    }
    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: Color) {
        self.c.spawn((
            Drawn,
            Sprite::from_color(color, Vec2::new(w, h) * self.scale),
            Transform::from_translation(world(
                self.view(Vec2::new(x + w * 0.5, y + h * 0.5)),
                self.z,
            )),
        ));
        self.z += 0.001;
    }
    fn image(&mut self, p: Vec2, size: Vec2, image: Handle<Image>, color: Color) {
        self.c.spawn((
            Drawn,
            Sprite {
                image,
                custom_size: Some(size * self.scale),
                color,
                ..default()
            },
            Transform::from_translation(world(self.view(p), self.z)),
        ));
        self.z += 0.001;
    }
    fn circle(&mut self, p: Vec2, r: f32, color: Color) {
        self.image(p, Vec2::splat(r * 2.), self.art.circle.clone(), color);
    }
    fn text(&mut self, x: f32, y: f32, s: impl Into<String>, size: f32, color: Color) {
        self.c.spawn((
            Drawn,
            Text2d::new(s),
            TextFont {
                font: self.art.font.clone().into(),
                font_size: bevy::text::FontSize::Px(size),
                ..default()
            },
            TextColor(color),
            Anchor::TOP_LEFT,
            Transform::from_translation(world(self.view(Vec2::new(x, y)), self.z))
                .with_scale(Vec3::splat(self.scale)),
        ));
        self.z += 0.001;
    }
    fn button(&mut self, x: f32, y: f32, w: f32, h: f32, label: &str, a: Action) {
        let hover = (self.mouse - self.view(Vec2::new(x + w * 0.5, y + h * 0.5)))
            .abs()
            .cmple(Vec2::new(w, h) * 0.5)
            .all();
        self.rect(x, y, w, h, Color::srgb(0.25, 0.27, 0.25));
        self.rect(
            x + 1.,
            y + 1.,
            w - 2.,
            h - 2.,
            if hover {
                Color::srgb(0.87, 0.89, 0.87)
            } else {
                Color::srgb(0.78, 0.79, 0.78)
            },
        );
        self.text(x + 5., y + 4., label, 11., Color::BLACK);
        self.hits.push(Hit {
            pos: self.view(Vec2::new(x + w * 0.5, y + h * 0.5)),
            size: Vec2::new(w, h) * self.scale,
            action: a,
        });
    }
    fn row(&mut self, x: f32, y: f32, w: f32, label: &str, a: Action, selected: bool) {
        let hover = (self.mouse - self.view(Vec2::new(x + w * 0.5, y + 10.)))
            .abs()
            .cmple(Vec2::new(w * 0.5, 10.))
            .all();
        self.rect(
            x,
            y,
            w,
            20.,
            if hover || selected {
                Color::srgb(0.25, 0.36, 0.49)
            } else {
                Color::srgb(0.91, 0.92, 0.91)
            },
        );
        self.text(
            x + 8.,
            y + 3.,
            label,
            11.,
            if hover || selected {
                Color::WHITE
            } else {
                Color::BLACK
            },
        );
        self.hits.push(Hit {
            pos: self.view(Vec2::new(x + w * 0.5, y + 10.)),
            size: Vec2::new(w, 20.) * self.scale,
            action: a,
        });
    }
}
fn draw_panel(d: &mut Painter<'_, '_, '_>, g: &Game) {
    match g.panel.as_str() {
        "Items" | "Skins" | "Modes" => {
            let entries = g.entries(&g.panel);
            let pages = entries.len().div_ceil(10);
            let page = g.page.min(pages.saturating_sub(1));
            for (i, e) in entries.iter().enumerate().skip(page * 10).take(10) {
                let label = format!(
                    "{}   {}",
                    if g.save.owned.contains(&e.name) { "[owned]".into() } else { format!("${:0.0}", e.price) },
                    e.name
                );
                d.row(96., 85. + (i - page * 10) as f32 * 21., 358., &label, Action::Row(i), g.row == Some(i));
            }
            d.text(100., 300., format!("Cash: ${:0.2}", g.save.cash), 11., Color::BLACK);
            d.button(96., 323., 28., 21., "<", Action::Page(-1));
            d.text(132., 328., format!("{} / {}", page + 1, pages), 11., Color::BLACK);
            d.button(185., 323., 28., 21., ">", Action::Page(1));
            d.button(307., 323., 147., 21., "Buy Selected Item", Action::Buy);
        }
        "Stats" => d.text(105., 95., format!("Money earned: ${:0.2}\n\nCurrent money: ${:0.2}\n\nInteractions: {}\n\nTime playing: {:0.0} seconds\n\nItems owned: {}", g.save.earned, g.save.cash, g.save.hits, g.save.seconds, g.save.owned.len()), 13., Color::BLACK),
        "Clear File" => {
            d.text(105., 100., "Reset everything?\n\nMoney and purchased items will be cleared.", 13., Color::BLACK);
            d.button(200., 270., 80., 24., "Reset", Action::Reset);
            d.button(300., 270., 80., 24., "Cancel", Action::Close);
        }
        "Physics" => {
            d.text(105., 100., format!("Gravity: {:0.2}\n\nOriginal simulation rate: 36 frames / second", g.save.gravity), 13., Color::BLACK);
            d.button(110., 180., 85., 24., "Low", Action::Gravity(0.15));
            d.button(220., 180., 85., 24., "Normal", Action::Gravity(1.));
            d.button(330., 180., 85., 24., "Reverse", Action::Gravity(-1.));
        }
        "Script" => {
            d.text(100., 85., "ShockScript — type to edit; Backspace deletes", 11., Color::BLACK);
            d.rect(97., 105., 357., 166., Color::srgb(0.96, 0.97, 0.95));
            let text = g.code.lines().rev().take(10).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n");
            d.text(102., 110., text, 11., Color::BLACK);
            if g.message_time > 0. {
                let message: String = g.message.chars().take(55).collect();
                d.text(100., 304., message, 10., Color::srgb(0.65, 0.08, 0.08));
            }
            d.button(98., 280., 105., 22., "Run Once", Action::ScriptRun(false));
            d.button(208., 280., 145., 22., if g.every_frame { "Stop Every Frame" } else { "Run Every Frame" }, if g.every_frame { Action::ScriptStop } else { Action::ScriptRun(true) });
            d.button(98., 315., 105., 22., "Levitate", Action::ScriptExample("addBuddyVel((275-getBuddyX())*0.075,(200-getBuddyY())*0.075)".into()));
            d.button(208., 315., 105., 22., "Explosion", Action::ScriptExample("if(getMouseDown(),explode(getXMouse(),getYMouse(),0.5),0)".into()));
        }
        "About" => d.text(105., 100., "Interactive Buddy\nOriginal game by Shock Value\n\nRust / Bevy native recreation\nBased on the supplied v1.02 SWF\n\nOriginal artwork and sounds remain credited\nto their original creators.", 13., Color::BLACK),
        "What's New?" => d.text(105., 100., "Native Rust / Bevy version\n\n36 Hz physics, original item catalog,\nshops, skins, persistent saves and scripting.\n\nSee PARITY.md for remaining differences.", 13., Color::BLACK),
        _ => d.text(105., 95., "Choose a tool from Items, then hold the mouse\nto interact with your buddy.\n\nOpen Hand: grab and throw.\nTickle: make him happy. Fist: strike him.\nObjects: hold, move, then release to throw.\nGuns and powers: hold over the target.\n\nEarn money to buy items, skins and modes.\nP: pause     R: reset buddy     Esc: close\n\nProgress saves automatically.\nRun with --demo to try everything.", 12., Color::BLACK),
    }
}
fn draw_modal(d: &mut Painter<'_, '_, '_>, g: &Game) {
    if g.panel.is_empty() {
        return;
    }
    d.z = if g.desktop { 800. } else { 200. };
    d.offset = g.panel_origin();
    d.rect(85., 52., 380., 302., Color::srgb(0.19, 0.22, 0.2));
    d.rect(87., 54., 376., 298., Color::srgb(0.83, 0.84, 0.82));
    d.rect(88., 55., 374., 23., Color::srgb(0.3, 0.4, 0.35));
    d.text(
        95.,
        60.,
        if matches!(g.panel.as_str(), "Items" | "Skins" | "Modes") {
            format!("Buy New {}", g.panel)
        } else {
            g.panel.clone()
        },
        12.,
        Color::WHITE,
    );
    d.button(440., 57., 19., 18., "x", Action::Close);
    draw_panel(d, g);
    d.offset = Vec2::ZERO;
}
fn draw(
    mut c: Commands,
    old: Query<Entity, With<Drawn>>,
    art: Res<Art>,
    mut g: ResMut<Game>,
    time: Res<Time>,
) {
    if time.delta_secs() > 0. {
        g.render_fps = g.render_fps * 0.95 + time.delta_secs().recip() * 0.05;
    }
    for e in &old {
        c.entity(e).despawn();
    }
    let mut d = Painter {
        c: &mut c,
        art: &art,
        z: 0.,
        hits: vec![],
        mouse: g.mouse,
        offset: Vec2::ZERO,
        scale: 1.,
        quantize: false,
        stage: g.stage,
    };
    if !g.desktop {
        d.image(
            Vec2::new(275., 200.),
            Vec2::new(W, H),
            art.room.clone(),
            Color::WHITE,
        );
    }
    d.z = 2.;
    d.offset = g.camera_offset;
    d.scale = g.camera_scale;
    d.quantize = g.mode("lag");
    for o in &g.objects {
        d.circle(
            Vec2::new(o.pos.x + 4., g.stage.y - if g.desktop { 1. } else { 12. }),
            o.r * 0.7,
            Color::srgba(0.08, 0.12, 0.09, 0.25),
        );
    }
    for p in &g.buddy.parts {
        d.image(
            Vec2::new(p.pos.x + 10., g.stage.y - if g.desktop { 1. } else { 12. }),
            Vec2::new(p.radius * 2., 4.),
            art.circle.clone(),
            Color::srgba(0.05, 0.08, 0.05, 0.2),
        );
    }
    d.z = 5.;
    for i in [5, 3, 4, 1, 2, 0] {
        let p = &g.buddy.parts[i];
        let name = ["head", "rArm", "lArm", "rLeg", "lLeg", "body"][i];
        if let Some(handle) = art.skins.get(&format!("{}_{}", g.save.skin, name)) {
            let head = g.buddy.parts[0].pos - g.buddy.parts[5].pos;
            let angle = head.x.atan2(-head.y);
            let trails = match g.save.motion_blur.as_str() {
                "Strong" => 3,
                "Weak" => 1,
                _ => 0,
            };
            for j in (1..=trails).rev() {
                let ghost = p.pos - (p.pos - p.prev) * j as f32 * 0.5;
                d.c.spawn((
                    Drawn,
                    Sprite {
                        image: handle.clone(),
                        color: Color::WHITE.with_alpha(0.13),
                        ..default()
                    },
                    Transform::from_translation(world(d.view(ghost), d.z))
                        .with_scale(Vec3::splat(d.scale))
                        .with_rotation(Quat::from_rotation_z(-angle)),
                ));
                d.z += 0.001;
            }
            d.native(p.pos, handle.clone(), angle);
        } else {
            d.circle(p.pos, p.radius, Color::srgb(0.85, 0.87, 0.85));
        }
    }
    for o in &g.objects {
        let key = match o.kind.as_str() {
            "Grenades" => "grenade",
            "Molotov Cocktails" => "molotov",
            "Mines" => "mine",
            "Baseballs" | "baseball" => "baseball",
            "Bowling Balls" | "bowlball" => "bowlball",
            "Rubber Balls" => "rubberball",
            "Infants" => "infant",
            "Radio" => "radio",
            "Magical Orb" => "orb",
            "Gravity Shifter" => "vortex",
            "Missiles" => "missile",
            "Fireballs" | "fireball" => "fireball",
            _ => "",
        };
        if let Some(h) = art.tools.get(key) {
            d.native(
                o.pos,
                h.clone(),
                if key == "missile" {
                    let v = o.pos - o.prev;
                    v.y.atan2(v.x) + std::f32::consts::FRAC_PI_2
                } else {
                    0.
                },
            );
            continue;
        }
        let color = match o.kind.as_str() {
            "Grenades" | "Mines" => Color::srgb(0.15, 0.24, 0.1),
            "Bowling Balls" | "bowlball" => Color::srgb(0.09, 0.09, 0.1),
            "Rubber Balls" => Color::srgb(0.7, 0.15, 0.1),
            "Molotov Cocktails" => Color::srgb(0.35, 0.25, 0.1),
            "Missiles" => Color::srgb(0.4, 0.42, 0.4),
            "Fireballs" | "fireball" => Color::srgb(1., 0.4, 0.02),
            "Magical Orb" => Color::srgb(0.4, 0.7, 1.),
            "Gravity Shifter" => Color::srgb(0.4, 0.1, 0.6),
            "Radio" => Color::srgb(0.22, 0.23, 0.25),
            "Infants" => Color::srgb(0.85, 0.7, 0.55),
            _ => Color::srgb(0.9, 0.9, 0.85),
        };
        d.circle(o.pos, o.r + 1., Color::srgb(0.15, 0.17, 0.14));
        d.circle(o.pos, o.r, color);
        if o.kind == "Grenades" {
            d.rect(
                o.pos.x - 2.,
                o.pos.y - o.r - 4.,
                4.,
                5.,
                Color::srgb(0.65, 0.65, 0.5),
            );
        }
        if o.kind == "Radio" {
            d.rect(o.pos.x - 15., o.pos.y - 10., 30., 20., color);
            d.circle(o.pos + Vec2::new(-7., 0.), 6., Color::BLACK);
            d.rect(
                o.pos.x + 3.,
                o.pos.y - 6.,
                9.,
                3.,
                Color::srgb(0.6, 0.8, 0.5),
            );
        }
    }
    d.z = 10.;
    for &(from, to) in &g.bullet_lines {
        d.line(from, to, 2., Color::WHITE);
    }
    for f in &g.fx {
        d.circle(
            f.pos,
            f.r * (0.5 + f.life / f.max * 0.5),
            f.color.with_alpha((f.life / f.max).clamp(0., 1.)),
        );
    }
    if (if g.desktop {
        g.desktop_interact || g.desktop_capture || g.buddy.nearest(g.mouse).is_some()
    } else {
        g.mouse.y > 23. && g.mouse.y < 383.
    }) && g.panel.is_empty()
        && g.menu.is_empty()
        && (!g.desktop || !desktop_ui::hit(&g, g.mouse))
    {
        let center = g.stage * 0.5;
        let cursor = (g.mouse - center) / g.camera_scale + center - g.camera_offset;
        match g.save.item.as_str() {
            "Fist" => {
                if let Some(h) = art.tools.get("fist") {
                    d.native(cursor, h.clone(), 0.);
                }
            }
            "Pistol" | "Shotgun" | "Machine Gun" | "Stun Gun" => {
                let key = match g.save.item.as_str() {
                    "Pistol" => "pistol",
                    "Shotgun" => "shotgun",
                    "Machine Gun" => "machineGun",
                    _ => "stunGun",
                };
                let v = g.direction;
                if g.save.item != "Stun Gun" {
                    d.line(cursor, cursor + v * 600., 1., Color::srgba(1., 0., 0., 0.2));
                }
                if let Some(h) = art.tools.get(key) {
                    d.native(cursor, h.clone(), v.y.atan2(v.x));
                }
            }
            "Medieval Flail" => {
                if let Some(h) = art.tools.get("flail_head") {
                    d.line(cursor, g.flail_pos, 1.5, Color::srgb(0.2, 0.22, 0.2));
                    d.native(g.flail_pos, h.clone(), 0.);
                }
            }
            "Tickle" => {
                d.text(g.mouse.x + 10., g.mouse.y - 8., "~", 18., Color::WHITE);
            }
            "Weak Gravity Vortex" | "Strong Gravity Vortex" => {
                for i in 0..12 {
                    let t = g.frame as f32 * 0.15 + i as f32 * 0.6;
                    d.circle(
                        cursor + Vec2::new(t.cos(), t.sin()) * (i as f32 * 2.),
                        2.,
                        Color::srgba(0.7, 0.8, 1., 0.7),
                    );
                }
            }
            _ => {
                d.circle(cursor, 2., Color::srgba(1., 1., 1., 0.7));
            }
        }
    }
    d.z = 100.;
    d.offset = Vec2::ZERO;
    d.scale = 1.;
    d.quantize = false;
    if g.desktop {
        if std::env::args().any(|arg| arg == "--window-effects-test") {
            d.rect(130., 130., 20., 20., Color::srgb(1., 0., 0.));
        }
        desktop_ui::draw(&mut d, &g);
        draw_modal(&mut d, &g);
        g.hits = d.hits;
        return;
    }
    if !art.mood.is_empty() {
        let n = ((g.emotion * 0.1 + 10.).ceil() as i32).clamp(1, 20) as usize - 1;
        d.c.spawn((
            Drawn,
            Sprite {
                image: art.mood[n].clone(),
                color: Color::WHITE.with_alpha(0.5),
                ..default()
            },
            Transform::from_translation(world(Vec2::new(30., 49.), 100.))
                .with_scale(Vec3::splat(1.8333)),
        ));
    }
    d.rect(0., 0., W, 22., Color::srgb(0.8, 0.82, 0.8));
    d.rect(0., 21., W, 1., Color::srgb(0.25, 0.28, 0.25));
    let menus = [
        ("File", 0., 36.),
        ("Skins", 36., 42.),
        ("Items", 78., 43.),
        ("Modes", 121., 48.),
        ("Settings", 169., 59.),
        ("Help", 228., 37.),
    ];
    for (name, x, w) in menus {
        d.button(x, 0., w, 21., name, Action::Menu(name.into()));
    }
    d.button(350., 0., 120., 21., "Desktop mode", Action::ToggleSurface);
    d.button(476., 0., 70., 21., "Exit", Action::Quit);
    d.text(
        13.5,
        370.,
        format!("${:0.2}  -  {}", g.save.cash, g.save.item),
        11.,
        Color::WHITE,
    );
    if g.mode("fps") {
        d.text(
            485.,
            383.,
            format!("{:.0} fps", g.render_fps),
            10.,
            Color::WHITE,
        );
    }
    if g.paused {
        d.text(244., 180., "PAUSED", 16., Color::WHITE);
    }
    if g.message_time > 0. {
        d.rect(120., 40., 310., 30., Color::srgba(0.12, 0.17, 0.12, 0.9));
        d.text(130., 48., &g.message, 11., Color::WHITE);
    }
    if g.save_error.is_some() {
        d.text(
            290.,
            383.,
            "Save failed (see file permissions)",
            10.,
            Color::srgb(1., 0.6, 0.3),
        );
    }
    if !g.menu.is_empty() {
        let x = menus
            .iter()
            .find(|(n, _, _)| *n == g.menu)
            .map_or(0., |(_, x, _)| *x);
        let kind = g.menu.clone();
        let mut y = 23.;
        match kind.as_str() {
            "Items" => {
                let cats = [
                    "Explosives",
                    "God Powers",
                    "Guns",
                    "Hand",
                    "Miscellaneous",
                    "Objects",
                    "Special",
                ];
                let mut cy = 23.;
                for cat in cats {
                    if g.catalog
                        .items
                        .iter()
                        .any(|e| e.category == cat && g.save.owned.contains(&e.name))
                    {
                        d.row(
                            x,
                            cy,
                            160.,
                            &format!("{cat}                 >"),
                            Action::Category(cat.into()),
                            g.category == cat,
                        );
                        if g.category == cat {
                            let mut sy = cy;
                            for e in g
                                .catalog
                                .items
                                .iter()
                                .filter(|e| e.category == cat && g.save.owned.contains(&e.name))
                            {
                                d.row(
                                    x + 161.,
                                    sy,
                                    177.,
                                    &e.name,
                                    Action::Select("Items".into(), e.name.clone()),
                                    g.save.item == e.name,
                                );
                                sy += 20.;
                            }
                        }
                        cy += 20.;
                    }
                }
                y = cy;
                d.row(
                    x,
                    y,
                    160.,
                    "Buy New Items...",
                    Action::Store("Items".into()),
                    false,
                );
            }
            "Skins" | "Modes" => {
                for e in g
                    .entries(&kind)
                    .iter()
                    .filter(|e| g.save.owned.contains(&e.name))
                {
                    let selected = if kind == "Skins" {
                        g.save.skin == e.id
                    } else {
                        g.save.modes.contains(&e.id)
                    };
                    d.row(
                        x,
                        y,
                        205.,
                        &e.name,
                        Action::Select(kind.clone(), e.name.clone()),
                        selected,
                    );
                    y += 20.;
                }
                d.row(
                    x,
                    y,
                    205.,
                    &format!("Buy New {kind}..."),
                    Action::Store(kind.clone()),
                    false,
                );
            }
            "File" => {
                for (label, action) in [
                    ("Stats...", Action::Panel("Stats".into())),
                    ("Clear File...", Action::Panel("Clear File".into())),
                    ("Reset Buddy", Action::ResetBuddy),
                    ("Pause / Resume    P", Action::Pause),
                ] {
                    d.row(x, y, 170., label, action, false);
                    y += 20.;
                }
            }
            "Settings" => {
                d.row(
                    x,
                    y,
                    200.,
                    "Physics...",
                    Action::Panel("Physics".into()),
                    false,
                );
                y += 20.;
                for key in ["Anti-Aliasing", "Effects", "Motion Blur"] {
                    let value = match key {
                        "Anti-Aliasing" => &g.save.anti_aliasing,
                        "Effects" => &g.save.effects,
                        _ => &g.save.motion_blur,
                    };
                    let selected = g.category == key;
                    d.row(
                        x,
                        y,
                        200.,
                        &format!("{key}: {value}   >"),
                        Action::Category(key.into()),
                        selected,
                    );
                    if selected {
                        for (i, option) in (if key == "Motion Blur" {
                            ["None", "Weak", "Strong"]
                        } else {
                            ["Low", "Medium", "High"]
                        })
                        .iter()
                        .enumerate()
                        {
                            d.row(
                                x + 201.,
                                y + i as f32 * 20.,
                                120.,
                                option,
                                Action::Setting(key.into(), (*option).into()),
                                value.as_str() == *option,
                            );
                        }
                    }
                    y += 20.;
                }
                d.row(
                    x,
                    y,
                    200.,
                    if g.save.sound {
                        "[x] Sound Enabled"
                    } else {
                        "[ ] Sound Enabled"
                    },
                    Action::ToggleSound,
                    false,
                );
            }
            "Help" => {
                for label in ["Help", "What's New?", "About"] {
                    d.row(
                        x,
                        y,
                        190.,
                        &format!("{label}..."),
                        Action::Panel(label.into()),
                        false,
                    );
                    y += 20.;
                }
            }
            _ => {}
        }
    }
    draw_modal(&mut d, &g);
    g.hits = d.hits;
}
fn play_sounds(
    mut c: Commands,
    art: Res<Art>,
    mut g: ResMut<Game>,
    music: Query<Entity, With<RadioMusic>>,
) {
    let radio = g.save.sound && g.objects.iter().any(|o| o.kind == "Radio");
    if radio && music.is_empty() {
        if let Some(h) = art.sounds.get("radioMusic") {
            c.spawn((
                RadioMusic,
                AudioPlayer::new(h.clone()),
                PlaybackSettings::LOOP.with_volume(Volume::Linear(0.2)),
            ));
        }
    }
    if !radio {
        for e in &music {
            c.entity(e).despawn();
        }
    }
    for s in g.sounds.drain(..) {
        if let Some(handle) = art.sounds.get(&s) {
            c.spawn((
                AudioPlayer::new(handle.clone()),
                PlaybackSettings::DESPAWN.with_volume(Volume::Linear(0.35)),
            ));
        }
    }
}

fn capture(
    mut c: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    frame: Res<desktop_present::DesktopFrame>,
) {
    if keys.just_pressed(KeyCode::F12) {
        std::fs::create_dir_all("screenshots").ok();
        let screenshot = frame
            .target
            .clone()
            .map(bevy::render::view::screenshot::Screenshot::image)
            .unwrap_or_else(bevy::render::view::screenshot::Screenshot::primary_window);
        c.spawn(screenshot)
            .observe(bevy::render::view::screenshot::save_to_disk(
                "screenshots/manual.png",
            ));
    }
}

fn sampling(g: Res<Game>, mut images: ResMut<Assets<Image>>, mut previous: Local<String>) {
    if *previous != g.save.anti_aliasing {
        for (_, im) in images.iter_mut() {
            im.sampler = if g.save.anti_aliasing == "Low" {
                bevy::image::ImageSampler::nearest()
            } else {
                bevy::image::ImageSampler::linear()
            };
        }
        *previous = g.save.anti_aliasing.clone();
    }
}

/// Deterministic native rendering and action smoke test. Uses temporary demo
/// state, captures review images, and exits without touching the player's save.
fn smoke_test(
    mut c: Commands,
    mut g: ResMut<Game>,
    mut frame: Local<u32>,
    mut exit: MessageWriter<AppExit>,
) {
    if !std::env::args().any(|a| a == "--smoke-test") {
        return;
    }
    *frame += 1;
    let shot = match *frame {
        120 => Some("01-room"),
        150 => {
            g.save.owned.remove("Molotov Cocktails");
            g.action(Action::Store("Items".into()));
            g.action(Action::Row(5));
            Some("02-shop")
        }
        180 => {
            let before = g.save.cash;
            g.action(Action::Buy);
            assert_eq!(g.save.cash, before - 60.);
            g.action(Action::Close);
            g.action(Action::Menu("Items".into()));
            g.action(Action::Category("Guns".into()));
            Some("03-menu")
        }
        210 => {
            g.action(Action::Close);
            g.action(Action::Select("Skins".into(), "Teletubby".into()));
            Some("04-skin")
        }
        240 => {
            g.action(Action::Select("Skins".into(), "Buddy".into()));
            let p = g.buddy.parts[5].pos;
            g.explode(p + Vec2::new(-30., 20.), 1.);
            Some("05-explosion")
        }
        270 => {
            g.action(Action::Panel("Script".into()));
            g.code = "say(\"Hello from Rust!\"); create(\"grenade\",275,100,2,0);".into();
            g.run_script();
            assert!(g.objects.iter().any(|o| o.kind == "Grenades"));
            Some("06-script")
        }
        300 => {
            g.action(Action::Close);
            g.action(Action::Store("Skins".into()));
            Some("07-skins")
        }
        340 => {
            println!(
                "SMOKE TEST PASSED: catalog, menus, skin switching, explosion, scripting and native rendering"
            );
            exit.write(AppExit::Success);
            None
        }
        _ => None,
    };
    if let Some(name) = shot {
        std::fs::create_dir_all(format!("{}/screenshots", env!("CARGO_MANIFEST_DIR")))
            .expect("screenshot directory");
        c.spawn(bevy::render::view::screenshot::Screenshot::primary_window())
            .observe(bevy::render::view::screenshot::save_to_disk(format!(
                "{}/screenshots/{name}.png",
                env!("CARGO_MANIFEST_DIR")
            )));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn game() -> Game {
        let mut g = Game::new();
        g.demo = true;
        g.save = Save::default();
        g.emotion = 0.;
        for e in g
            .catalog
            .items
            .iter()
            .chain(&g.catalog.skins)
            .chain(&g.catalog.modes)
        {
            if e.initially_owned {
                g.save.owned.insert(e.name.clone());
            }
        }
        g
    }
    #[test]
    fn catalog_matches_extracted_inventory() {
        let g = game();
        assert_eq!(
            (
                g.catalog.items.len(),
                g.catalog.skins.len(),
                g.catalog.modes.len()
            ),
            (28, 11, 10)
        );
        assert!(g.save.owned.contains("Grenades"));
        assert!(!g.save.owned.contains("Shotgun"));
        assert_eq!(
            g.catalog
                .items
                .iter()
                .find(|e| e.name == "Radio")
                .unwrap()
                .price,
            320.
        );
    }
    #[test]
    fn purchases_require_cash_and_cannot_charge_twice() {
        let mut g = game();
        g.action(Action::Store("Items".into()));
        g.action(Action::Row(5));
        g.action(Action::Buy);
        assert!(!g.save.owned.contains("Molotov Cocktails"));
        g.save.cash = 100.;
        g.action(Action::Buy);
        assert_eq!(g.save.cash, 40.);
        assert!(g.save.owned.contains("Molotov Cocktails"));
        g.action(Action::Buy);
        assert_eq!(g.save.cash, 40.);
    }
    #[test]
    fn skin_ids_and_mode_ids_are_resolved() {
        let mut g = game();
        g.action(Action::Select("Skins".into(), "Maddox".into()));
        assert_eq!(g.save.skin, "pirate");
        g.action(Action::Select("Modes".into(), "Low Gravity".into()));
        assert!(g.mode("lowGrav"));
        g.action(Action::Select("Modes".into(), "Low Gravity".into()));
        assert!(!g.mode("lowGrav"));
    }
    #[test]
    fn scripted_grenades_and_signed_cash_work() {
        let mut g = game();
        g.save.cash = 100.;
        g.code = "secret_addCash(-25); create(\"grenade\",100,100,2,3)".into();
        g.run_script();
        assert_eq!(g.save.cash, 75.);
        assert_eq!(g.objects[0].kind, "Grenades");
        assert_eq!(g.objects[0].life, 140. / 36.);
    }
    #[test]
    fn explosion_earns_only_near_buddy_and_moves_it() {
        let mut g = game();
        let p = g.buddy.parts[5].pos;
        g.explode(Vec2::new(-1000., -1000.), 1.);
        assert_eq!(g.save.cash, 0.);
        g.explode(p - Vec2::new(30., 30.), 1.);
        assert!(g.save.cash > 0.);
        assert!(g.buddy.parts.iter().any(|p| p.pos.distance(p.prev) > 0.));
    }
    #[test]
    fn blast_matches_source_reward_and_shared_velocity() {
        let mut g = game();
        let body = g.buddy.parts[5].pos;
        g.explode(body - Vec2::new(100., 0.), 1.);
        assert!((g.save.cash - 5.).abs() < 0.0001);
        let velocity = g.buddy.parts[5].pos - g.buddy.parts[5].prev;
        assert!((velocity.x - 99.).abs() < 0.001);
        for part in &g.buddy.parts {
            assert!((part.pos - part.prev - velocity).length() < 0.001);
        }
    }
    #[test]
    fn blast_directly_above_buddy_does_not_create_spin_from_zero_dx() {
        let mut g = game();
        let body = g.buddy.parts[5].pos;
        g.explode(body + Vec2::new(0.0, 80.0), 1.0);
        assert_eq!(g.buddy.rotation(), 0.0);
    }
    #[test]
    fn swept_throw_contact_finds_fast_crossings_without_nearby_false_hits() {
        let windows = [desktop_windows::WindowRect {
            id: 1,
            min: Vec2::new(100.0, 100.0),
            max: Vec2::new(200.0, 200.0),
        }];
        assert_eq!(
            swept_window_contact(
                Vec2::new(0.0, 150.0),
                Vec2::new(300.0, 150.0),
                10.0,
                &windows,
            ),
            Some(Vec2::new(100.0, 150.0))
        );
        assert_eq!(
            swept_window_contact(
                Vec2::new(0.0, 230.0),
                Vec2::new(300.0, 230.0),
                10.0,
                &windows,
            ),
            None
        );
    }
    #[test]
    fn bullets_travel_ricochet_and_hit_between_frames() {
        let mut g = game();
        g.bullets.push(Bullet {
            pos: Vec2::new(100., 200.),
            velocity: Vec2::new(20., 0.),
            strength: 1.,
            ricochets: 1,
            shotgun: false,
            age: 0,
            window_hit: false,
        });
        g.advance_bullets(0.);
        assert_eq!(g.bullets[0].pos, Vec2::new(120., 200.));
        g.bullets[0].pos.x = 540.;
        g.advance_bullets(0.);
        assert!(g.bullets[0].velocity.x < 0.);
        assert_eq!(g.bullets[0].ricochets, 0);
        g.bullets.clear();
        let target = g.buddy.parts[0].pos;
        g.bullets.push(Bullet {
            pos: target - Vec2::new(100., 0.),
            velocity: Vec2::new(250., 0.),
            strength: 1.25,
            ricochets: 0,
            shotgun: false,
            age: 0,
            window_hit: false,
        });
        g.advance_bullets(0.);
        assert!(g.bullets.is_empty());
        assert!(g.health < 0.);
        assert_eq!(g.save.hits, 1);
    }
    #[test]
    fn desktop_windows_do_not_absorb_bullets_before_buddy() {
        let mut g = game();
        g.desktop = true;
        g.desktop_windows.push(desktop_windows::WindowRect {
            id: 1,
            min: Vec2::ZERO,
            max: Vec2::new(W, H),
        });
        let target = g.buddy.parts[0].pos;
        g.bullets.push(Bullet {
            pos: target - Vec2::new(100., 0.),
            velocity: Vec2::new(250., 0.),
            strength: 1.25,
            ricochets: 0,
            shotgun: false,
            age: 0,
            window_hit: false,
        });
        g.advance_bullets(0.);
        assert_eq!(g.save.hits, 1);
        assert_eq!(g.desktop_impacts.len(), 1);
        assert!(g.bullets.is_empty());
    }

    #[test]
    fn selecting_desktop_tools_arms_use_and_hand_disarms_it() {
        let mut g = game();
        g.desktop = true;
        g.save.owned.insert("Pistol".into());
        g.action(Action::Select("Items".into(), "Pistol".into()));
        assert!(g.desktop_interact);
        g.action(Action::Select("Items".into(), "Open Hand".into()));
        assert!(!g.desktop_interact);
    }

    #[test]
    fn desktop_missile_reaches_buddy_in_front_of_a_window() {
        let mut g = game();
        g.desktop = true;
        g.desktop_windows.push(desktop_windows::WindowRect {
            id: 1,
            min: Vec2::ZERO,
            max: Vec2::new(W, H),
        });
        let body = g.buddy.parts[5].pos;
        g.objects.push(Object {
            pos: body - Vec2::new(175., 0.),
            prev: body - Vec2::new(295., 0.),
            r: 4.,
            kind: "Missiles".into(),
            life: 8.,
            held: false,
            hit_timer: 0,
            window_hit_timer: 0,
        });
        let mut app = App::new();
        app.insert_resource(g).add_systems(Update, simulate);
        app.update();
        assert_eq!(app.world().resource::<Game>().objects.len(), 1);
        assert!(app.world().resource::<Game>().desktop_impacts.is_empty());
        app.update();
        let g = app.world().resource::<Game>();
        assert!(g.objects.is_empty());
        assert!(
            g.desktop_impacts
                .iter()
                .any(|impact| impact.kind == desktop_windows::ImpactKind::Explosion)
        );
    }
    #[test]
    fn script_controls_position_velocity_rotation_and_projectiles() {
        let mut g = game();
        g.code = "setBuddyPos(200,100);setBuddyVel(2,3);setBuddyRot(0.5);secret_addEmotion(10);bullet(50,100,250,0,1,0)".into();
        g.run_script();
        assert_eq!(g.buddy.parts[5].pos, Vec2::new(200., 100.));
        assert_eq!(
            g.buddy.parts[5].pos - g.buddy.parts[5].prev,
            Vec2::new(2., 3.)
        );
        assert!((g.buddy.rotation() - 0.5).abs() < 0.001);
        assert_eq!(g.emotion, 10.);
        assert_eq!(g.bullets.len(), 1);
    }
    #[test]
    fn every_tool_survives_use_and_release() {
        let names = game()
            .catalog
            .items
            .into_iter()
            .map(|e| e.name)
            .collect::<Vec<_>>();
        for name in names {
            let mut g = game();
            g.save.item = name.clone();
            let mut app = App::new();
            app.insert_resource(g).add_systems(Update, simulate);
            for i in 0..200 {
                let mut g = app.world_mut().resource_mut::<Game>();
                g.down = i < 30;
                g.pressed = i == 0;
                g.released = i == 30;
                g.mouse = g.buddy.parts[5].pos + Vec2::new((i as f32 * 0.2).sin() * 20., -10.);
                drop(g);
                app.update();
            }
            let g = app.world().resource::<Game>();
            assert!(
                g.buddy
                    .parts
                    .iter()
                    .all(|p| p.pos.is_finite() && p.prev.is_finite()),
                "{name}"
            );
            assert!(
                g.objects
                    .iter()
                    .all(|o| o.pos.is_finite() && o.prev.is_finite()),
                "{name}"
            );
            assert!(g.save.cash.is_finite() && g.save.cash >= 0., "{name}");
            assert!(g.fx.len() <= 600, "{name}");
        }
    }
    #[test]
    fn progress_replaces_existing_save_and_round_trips() {
        let mut g = game();
        g.demo = false;
        g.save_path =
            std::env::temp_dir().join(format!("ib-save-test-{}.json", std::process::id()));
        g.save.cash = 42.;
        g.emotion = 31.;
        g.code = "say(\"saved\")".into();
        g.persist();
        assert!(g.save_error.is_none());
        g.save.cash = 43.;
        g.persist();
        assert!(g.save_error.is_none());
        let saved: Save =
            serde_json::from_str(&std::fs::read_to_string(&g.save_path).unwrap()).unwrap();
        assert_eq!(saved.cash, 43.);
        assert_eq!(saved.emotion, 31.);
        assert_eq!(saved.script_code, g.code);
        std::fs::remove_file(&g.save_path).unwrap();
    }
}
