//! Runtime-only arena mechanics. All timers count simulated 36 Hz steps.
use crate::{Game, Object, Painter, fuse_seconds};
use bevy::prelude::{Color, Vec2};

const PORTAL_RADIUS: f32 = 24.;
const STASIS_RADIUS: f32 = 65.;
const BUMPER_RADIUS: f32 = 22.;
const COOLDOWN: u8 = 18;

#[derive(Default)]
pub(crate) struct Advanced {
    portals: [Option<Vec2>; 2],
    next_portal: usize,
    buddy_cooldown: u8,
    // One gate for loose toys: a batch can cross together without persistent object IDs.
    toy_cooldown: u8,
    grapple: Option<Vec2>,
    zones: Vec<Zone>,
    pads: Vec<Pad>,
    beam: Option<Vec2>,
    bumper_cooldowns: [u8; 3],
    score: u64,
}

struct Pad {
    pos: Vec2,
    remaining: u16,
    cooldown: u8,
}
struct Zone {
    pos: Vec2,
    remaining: u16,
}

fn grapple_tool(item: &str) -> bool {
    matches!(item, "Grappling Hook" | "grapple")
}

fn safe_toy(o: &Object) -> bool {
    !o.held
        && o.life > 0.
        && !matches!(
            o.kind.as_str(),
            "Grenades"
                | "Mines"
                | "Missiles"
                | "Bazooka Rocket"
                | "Fireballs"
                | "fireball"
                | "Molotov Cocktails"
                | "Firecracker"
                | "Dynamite"
                | "Cherry Bombs"
                | "Bomb"
                | "Radio"
                | "Gravity Shifter"
                | "Magical Orb"
                | "Telekinesis"
        )
}

fn bounded(p: Vec2, stage: Vec2, margin: f32) -> Option<Vec2> {
    if !p.is_finite() || !stage.is_finite() || stage.min_element() < margin * 2. {
        return None;
    }
    Some(p.clamp(Vec2::splat(margin), stage - Vec2::splat(margin)))
}

fn bumpers(stage: Vec2) -> [Vec2; 3] {
    [
        stage * Vec2::new(0.28, 0.45),
        stage * Vec2::new(0.72, 0.45),
        stage * Vec2::new(0.50, 0.72),
    ]
}

// Translate the complete rig and its history together: never create teleport velocity.
fn move_buddy(buddy: &mut crate::physics::Buddy, target: Vec2, stage: Vec2) -> bool {
    let Some(torso) = buddy.parts.get(5) else {
        return false;
    };
    if !target.is_finite() || !stage.is_finite() {
        return false;
    }
    let desired = target - torso.pos;
    let mut low = Vec2::splat(f32::NEG_INFINITY);
    let mut high = Vec2::splat(f32::INFINITY);
    for part in &buddy.parts {
        if !part.pos.is_finite() || !part.prev.is_finite() || !part.radius.is_finite() {
            return false;
        }
        low = low.max(Vec2::splat(part.radius) - part.pos);
        high = high.min(stage - Vec2::splat(part.radius) - part.pos);
    }
    if low.x > high.x || low.y > high.y {
        return false;
    }
    let delta = desired.clamp(low, high);
    for part in &mut buddy.parts {
        part.pos += delta;
        part.prev += delta;
    }
    true
}

fn spring(delta: Vec2, velocity: Vec2, rest: f32) -> Vec2 {
    if !delta.is_finite() || !velocity.is_finite() {
        return Vec2::ZERO;
    }
    let distance = delta.length();
    if distance <= rest {
        return Vec2::ZERO;
    }
    let direction = delta.normalize_or_zero();
    (direction * ((distance - rest) * 0.018 - velocity.dot(direction) * 0.12)).clamp_length_max(2.)
}

fn kick(delta: Vec2) -> Vec2 {
    if delta.length_squared() < 0.001 {
        Vec2::new(0., -7.)
    } else {
        delta.normalize_or_zero() * 7.
    }
}

impl Game {
    pub(crate) fn advanced_cancel_pointer(&mut self) {
        self.advanced.beam = None;
        self.advanced.grapple = None;
    }
    /// Call only for active world input; `p` is already in world coordinates.
    pub(crate) fn advanced_tool(&mut self, item: &str, p: Vec2) {
        if !grapple_tool(item) {
            self.advanced.grapple = None;
        }
        match item {
            "Portal Gun" | "portalgun" if self.pressed => {
                if let Some(p) = bounded(p, self.stage, PORTAL_RADIUS) {
                    let a = &mut self.advanced;
                    a.portals[a.next_portal] = Some(p);
                    a.next_portal ^= 1;
                }
            }
            "Grappling Hook" | "grapple" => {
                if !self.down || self.released {
                    self.advanced.grapple = None;
                } else if self.pressed {
                    self.advanced.grapple = bounded(p, self.stage, 8.);
                }
            }
            "Tractor Beam" | "tractorbeam" if self.down => {
                self.advanced.beam = bounded(p, self.stage, 8.);
            }
            "Launch Pad" | "launchpad" if self.pressed => {
                if let Some(pos) = bounded(p, self.stage, 24.) {
                    if self.advanced.pads.len() == 3 {
                        self.advanced.pads.remove(0);
                    }
                    self.advanced.pads.push(Pad {
                        pos,
                        remaining: 720,
                        cooldown: 0,
                    });
                }
            }
            "Stasis Field" | "stasis" if self.pressed => {
                if let Some(p) = bounded(p, self.stage, STASIS_RADIUS) {
                    let zones = &mut self.advanced.zones;
                    if zones.len() == 3 {
                        zones.remove(0);
                    }
                    zones.push(Zone {
                        pos: p,
                        remaining: 180,
                    });
                }
            }
            _ => {}
        }
    }

    /// Run once AFTER Buddy and loose-object physics, not from a render/wall timer.
    pub(crate) fn advanced_step(&mut self) {
        if self.mode("freeze") {
            return;
        }
        let carousel = self.mode("gravityCarousel")
            && !self.mode("zerograv")
            && !self.objects.iter().any(|o| o.kind == "Gravity Shifter");
        let gravity = if self.mode("lowGrav") {
            self.save.gravity * 0.35
        } else {
            self.save.gravity
        };
        let swarm = self.mode("toySwarm");
        let pinball = self.mode("pinball");
        let tethered = self.mode("tetheredToys");
        let a = &mut self.advanced;
        a.buddy_cooldown = a.buddy_cooldown.saturating_sub(1);
        a.toy_cooldown = a.toy_cooldown.saturating_sub(1);
        for cooldown in &mut a.bumper_cooldowns {
            *cooldown = cooldown.saturating_sub(1);
        }
        if !self.down || self.released || !grapple_tool(&self.save.item) {
            a.grapple = None;
        }
        let Some(torso) = self.buddy.parts.get(5) else {
            return;
        };
        let torso_pos = torso.pos;
        let torso_velocity = torso.pos - torso.prev;
        if !self.down || self.released || self.save.item != "Tractor Beam" {
            a.beam = None;
        }
        if let Some(target) = a.beam {
            let delta = target - torso_pos;
            if delta.length() < 200. {
                let pull = spring(delta, torso_velocity, 18.);
                for part in &mut self.buddy.parts {
                    part.prev -= pull * 0.4;
                }
            }
            for o in self.objects.iter_mut().filter(|o| safe_toy(o)) {
                let delta = target - o.pos;
                if delta.length() < 200. {
                    o.prev -= spring(delta, o.pos - o.prev, 18.);
                }
            }
        }
        if carousel {
            let angle = self.frame as f32 / 36. * std::f32::consts::TAU / 20.;
            let force = Vec2::new(angle.sin(), angle.cos()) * gravity;
            for part in &mut self.buddy.parts {
                part.prev -= force;
            }
            for o in self.objects.iter_mut().filter(|o| {
                !o.held && o.life > 0. && !matches!(o.kind.as_str(), "Missiles" | "Bazooka Rocket")
            }) {
                o.prev -= force;
            }
        }
        if swarm {
            let toys: Vec<_> = self
                .objects
                .iter()
                .filter(|o| safe_toy(o))
                .map(|o| (o.pos, o.pos - o.prev))
                .collect();
            for o in self.objects.iter_mut().filter(|o| safe_toy(o)) {
                let mut force = Vec2::ZERO;
                let mut neighbors = 0;
                for &(pos, velocity) in &toys {
                    let delta = pos - o.pos;
                    let dist = delta.length();
                    if dist > 0.01 && dist < 180. {
                        force += if dist < o.r * 2. + 12. {
                            -delta.normalize_or_zero() * 0.6
                        } else {
                            delta * 0.002 + (velocity - (o.pos - o.prev)) * 0.025
                        };
                        neighbors += 1;
                    }
                }
                if neighbors > 0 {
                    o.prev -= (force / neighbors as f32).clamp_length_max(0.5);
                }
            }
        }
        for pad in &mut a.pads {
            pad.remaining = pad.remaining.saturating_sub(1);
            pad.cooldown = pad.cooldown.saturating_sub(1);
            if pad.cooldown == 0 {
                let mut launched = false;
                if self.drag.is_none()
                    && self
                        .buddy
                        .parts
                        .iter()
                        .any(|p| p.pos.distance(pad.pos) < 24. + p.radius)
                {
                    for part in &mut self.buddy.parts {
                        let mut v = part.pos - part.prev;
                        v.y = -8.;
                        part.prev = part.pos - v;
                    }
                    launched = true;
                }
                for o in self.objects.iter_mut().filter(|o| safe_toy(o)) {
                    if o.pos.distance(pad.pos) < 24. + o.r {
                        let mut v = o.pos - o.prev;
                        v.y = -8.;
                        o.prev = o.pos - v;
                        launched = true;
                    }
                }
                if launched {
                    pad.cooldown = COOLDOWN;
                }
            }
        }
        a.pads.retain(|p| p.remaining > 0);

        if let Some(anchor) = a.grapple {
            let impulse = spring(anchor - torso_pos, torso_velocity, 35.);
            self.buddy.parts[5].prev -= impulse;
        }
        if tethered {
            let mut reaction = Vec2::ZERO;
            for o in self.objects.iter_mut().filter(|o| safe_toy(o)) {
                let impulse = spring(torso_pos - o.pos, o.pos - o.prev - torso_velocity, 100.);
                o.prev -= impulse;
                reaction -= impulse * 0.15;
            }
            self.buddy.parts[5].prev -= reaction.clamp_length_max(2.);
        }

        if pinball {
            for (i, center) in bumpers(self.stage).into_iter().enumerate() {
                if a.bumper_cooldowns[i] != 0 {
                    continue;
                }
                let mut hits = 0;
                let torso = &self.buddy.parts[5];
                if torso.pos.distance(center) < BUMPER_RADIUS + torso.radius {
                    let impulse = kick(torso.pos - center);
                    for part in &mut self.buddy.parts {
                        part.prev -= impulse;
                    }
                    hits += 1;
                }
                for o in self.objects.iter_mut().filter(|o| safe_toy(o)) {
                    if o.pos.distance(center) < BUMPER_RADIUS + o.r {
                        o.prev -= kick(o.pos - center);
                        hits += 1;
                    }
                }
                if hits > 0 {
                    a.bumper_cooldowns[i] = COOLDOWN;
                    a.score = a.score.saturating_add(hits);
                }
            }
        }

        if let [Some(first), Some(second)] = a.portals {
            if a.buddy_cooldown == 0 && self.drag.is_none() {
                let pos = self.buddy.parts[5].pos;
                for (entrance, exit) in [(first, second), (second, first)] {
                    if pos.distance(entrance) <= PORTAL_RADIUS + 12.
                        && move_buddy(&mut self.buddy, exit, self.stage)
                    {
                        a.buddy_cooldown = COOLDOWN;
                        break;
                    }
                }
            }
            if a.toy_cooldown == 0 {
                let mut crossed = false;
                for o in self.objects.iter_mut().filter(|o| safe_toy(o)) {
                    for (entrance, exit) in [(first, second), (second, first)] {
                        if o.pos.distance(entrance) <= PORTAL_RADIUS + o.r {
                            if let Some(target) = bounded(exit, self.stage, o.r) {
                                let delta = target - o.pos;
                                o.pos += delta;
                                o.prev += delta;
                                crossed = true;
                            }
                            break;
                        }
                    }
                }
                if crossed {
                    a.toy_cooldown = COOLDOWN;
                }
            }
        }

        // Rewind this step's motion and damp history, rather than zeroing prev.
        if a.zones
            .iter()
            .any(|z| self.buddy.parts[5].pos.distance(z.pos) < STASIS_RADIUS)
        {
            let torso = &self.buddy.parts[5];
            let target = torso.pos - (torso.pos - torso.prev) * 0.75;
            if move_buddy(&mut self.buddy, target, self.stage) {
                for part in &mut self.buddy.parts {
                    let velocity = (part.pos - part.prev) * 0.25;
                    part.prev = part.pos - velocity;
                }
            }
        }
        for o in &mut self.objects {
            if !o.held
                && o.life > 0.
                && a.zones
                    .iter()
                    .any(|z| o.pos.distance(z.pos) < STASIS_RADIUS)
            {
                let velocity = (o.pos - o.prev) * 0.25;
                if let Some(pos) = bounded(o.prev + velocity, self.stage, o.r) {
                    o.pos = pos;
                    o.prev = pos - velocity;
                }
            }
        }
        for zone in &mut a.zones {
            zone.remaining = zone.remaining.saturating_sub(1);
        }
        a.zones.retain(|z| z.remaining > 0);
    }

    /// Prime only live, loose timed explosives; no recursive explosions here.
    pub(crate) fn advanced_explosion(&mut self, p: Vec2, r: f32) {
        if !self.mode("chainReaction") || !p.is_finite() || !r.is_finite() || r <= 0. {
            return;
        }
        let radius = r.min(200.);
        let mut primed = 0;
        for o in &mut self.objects {
            if !o.held
                && o.life > 0.
                && fuse_seconds(&o.kind).is_finite()
                && o.pos.distance(p) <= radius
            {
                // Existing short fuses are never lengthened. At most forty steps.
                o.life = o.life.min((6 + primed * 3).min(40) as f32 / 36.);
                primed += 1;
            }
        }
    }
}

/// Called inside the existing world transform; never touches native windows.
pub(crate) fn draw_world(d: &mut Painter, g: &Game) {
    let a = &g.advanced;
    for pad in &a.pads {
        d.rect(
            pad.pos.x - 24.,
            pad.pos.y - 4.,
            48.,
            8.,
            Color::srgb(0.3, 0.7, 0.6),
        );
        d.text(
            pad.pos.x - 5.,
            pad.pos.y - 19.,
            "^",
            16.,
            Color::srgb(0.6, 1., 0.7),
        );
    }
    if g.down && g.save.item == "Tractor Beam" {
        if let Some(target) = a.beam {
            d.circle(target, 18., Color::srgba(0.2, 1., 0.8, 0.25));
            if let Some(torso) = g.buddy.parts.get(5) {
                d.line(torso.pos, target, 1., Color::srgba(0.3, 1., 0.8, 0.5));
            }
        }
    }
    for (i, p) in a.portals.iter().enumerate() {
        if let Some(p) = p {
            let color = if i == 0 {
                Color::srgb(0.1, 0.6, 1.)
            } else {
                Color::srgb(1., 0.5, 0.1)
            };
            d.circle(*p, PORTAL_RADIUS, color);
            d.circle(*p, PORTAL_RADIUS - 5., Color::srgb(0.08, 0.1, 0.18));
        }
    }
    if g.down && !g.released && grapple_tool(&g.save.item) {
        if let (Some(p), Some(torso)) = (a.grapple, g.buddy.parts.get(5)) {
            d.line(torso.pos, p, 2., Color::srgb(0.7, 0.8, 0.85));
            d.circle(p, 5., Color::srgb(0.9, 0.95, 1.));
        }
    }
    for zone in &a.zones {
        d.circle(zone.pos, STASIS_RADIUS, Color::srgba(0.2, 0.75, 1., 0.16));
        d.circle(zone.pos, 4., Color::srgb(0.4, 0.9, 1.));
    }
    if g.mode("tetheredToys") {
        if let Some(torso) = g.buddy.parts.get(5) {
            for o in g.objects.iter().filter(|o| safe_toy(o)) {
                d.line(torso.pos, o.pos, 1.5, Color::srgba(0.6, 0.85, 0.7, 0.7));
            }
        }
    }
    if g.mode("pinball") {
        for (i, p) in bumpers(g.stage).into_iter().enumerate() {
            let color = if a.bumper_cooldowns[i] > 0 {
                Color::srgb(1., 0.9, 0.4)
            } else {
                Color::srgb(0.85, 0.25, 0.65)
            };
            d.circle(p, BUMPER_RADIUS, color);
            d.circle(p, 12., Color::srgb(0.25, 0.1, 0.35));
        }
        d.text(
            12.,
            if g.desktop { 12. } else { 30. },
            format!("Pinball: {}", a.score),
            16.,
            Color::srgb(1., 0.9, 0.5),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game() -> Game {
        let mut g = Game::new();
        g.demo = true;
        g.save = crate::Save::default();
        g.advanced = Advanced::default();
        g
    }

    fn toy(g: &mut Game, kind: &str, pos: Vec2) {
        g.objects.push(Object {
            pos,
            prev: pos - Vec2::X * 4.,
            r: 8.,
            kind: kind.into(),
            life: 5.,
            held: false,
            hit_timer: 0,
            window_hit_timer: 0,
        });
    }

    #[test]
    fn portals_preserve_rig_and_velocity_and_gate_return() {
        let mut g = game();
        let entrance = g.buddy.parts[5].pos;
        g.pressed = true;
        g.advanced_tool("portalgun", entrance);
        g.advanced_tool("portalgun", Vec2::splat(9999.));
        let old = g.buddy.parts.clone();
        toy(&mut g, "Baseballs", entrance);
        toy(&mut g, "Grenades", entrance);
        g.advanced_step();
        let delta = g.buddy.parts[5].pos - old[5].pos;
        assert!(delta.length() > 50.);
        for (part, before) in g.buddy.parts.iter().zip(&old) {
            assert!((part.pos - before.pos - delta).length() < 0.001);
            assert!((part.pos - part.prev - (before.pos - before.prev)).length() < 0.001);
            assert!(part.pos.x >= part.radius && part.pos.y >= part.radius);
            assert!(part.pos.x <= g.stage.x - part.radius + 0.001);
            assert!(part.pos.y <= g.stage.y - part.radius + 0.001);
        }
        assert!((g.objects[0].pos - g.objects[0].prev - Vec2::X * 4.).length() < 0.001);
        assert_eq!(g.objects[1].pos, entrance);
        let pos = g.buddy.parts[5].pos;
        let toy_pos = g.objects[0].pos;
        g.advanced_step();
        assert_eq!(g.buddy.parts[5].pos, pos);
        assert_eq!(g.objects[0].pos, toy_pos);
        assert_eq!(g.advanced.buddy_cooldown, COOLDOWN - 1);
    }

    #[test]
    fn stasis_is_bounded_capped_and_expires_in_simulation_steps() {
        let mut g = game();
        g.pressed = true;
        for _ in 0..4 {
            g.advanced_tool("stasis", Vec2::splat(-100.));
        }
        assert_eq!(g.advanced.zones.len(), 3);
        assert_eq!(g.advanced.zones[0].pos, Vec2::splat(STASIS_RADIUS));
        g.advanced.zones.clear();
        let p = g.buddy.parts[5].pos;
        g.advanced_tool("stasis", p);
        for part in &mut g.buddy.parts {
            part.prev = part.pos - Vec2::X * 4.;
        }
        toy(&mut g, "Baseballs", p);
        toy(&mut g, "Baseballs", p);
        g.objects[1].held = true;
        g.advanced_step();
        assert!((g.buddy.parts[5].pos - (p - Vec2::X * 3.)).length() < 0.001);
        assert!((g.buddy.parts[5].pos - g.buddy.parts[5].prev - Vec2::X).length() < 0.001);
        assert!((g.objects[0].pos - g.objects[0].prev - Vec2::X).length() < 0.001);
        assert_eq!(g.objects[1].pos, p);
        for _ in 1..180 {
            g.advanced_step();
        }
        assert!(g.advanced.zones.is_empty());
    }

    #[test]
    fn tractor_and_launch_pads_are_bounded_and_ignore_explosives() {
        let mut g = game();
        g.save.item = "Tractor Beam".into();
        g.down = true;
        let p = g.buddy.parts[5].pos + Vec2::new(100., 0.);
        g.advanced_tool("Tractor Beam", p);
        toy(&mut g, "Baseballs", p - Vec2::new(70., 0.));
        toy(&mut g, "Bazooka Rocket", p - Vec2::new(70., 0.));
        let before = g.objects[0].prev;
        let dangerous = g.objects[1].prev;
        g.advanced_step();
        assert!(g.objects[0].prev.x < before.x);
        assert_eq!(g.objects[1].prev, dangerous);
        g.advanced_cancel_pointer();
        assert!(g.advanced.beam.is_none());
        g.advanced_tool("Tractor Beam", p);
        g.down = false;
        g.advanced_step();
        assert!(g.advanced.beam.is_none());
        g.objects.clear();
        g.pressed = true;
        for _ in 0..4 {
            g.advanced_tool("Launch Pad", Vec2::new(40., 200.));
        }
        assert_eq!(g.advanced.pads.len(), 3);
        toy(&mut g, "Baseballs", Vec2::new(40., 200.));
        g.advanced_step();
        assert_eq!((g.objects[0].pos - g.objects[0].prev).y, -8.);
        for pad in &mut g.advanced.pads {
            pad.remaining = 1;
        }
        g.advanced_step();
        assert!(g.advanced.pads.is_empty());
    }
    #[test]
    fn beam_cancels_when_held_pointer_leaves_world_input() {
        let mut g = game();
        g.desktop = false;
        g.save.item = "Tractor Beam".into();
        g.mouse = Vec2::new(200., 200.);
        g.down = true;
        g.pressed = true;
        let mut app = bevy::prelude::App::new();
        app.insert_resource(g)
            .add_systems(bevy::prelude::Update, crate::simulate);
        app.update();
        assert!(app.world().resource::<Game>().advanced.beam.is_some());
        app.world_mut().resource_mut::<Game>().mouse = Vec2::new(0., 0.);
        app.update();
        assert!(app.world().resource::<Game>().advanced.beam.is_none());
    }
    #[test]
    fn carousel_and_swarm_apply_actual_bounded_forces() {
        let mut g = game();
        g.save.modes.insert("gravityCarousel".into());
        g.frame = 180;
        let prev = g.buddy.parts[5].prev;
        g.advanced_step();
        assert!(g.buddy.parts[5].prev.x < prev.x);
        g.save.gravity = -2.;
        g.save.modes.insert("lowGrav".into());
        let prev = g.buddy.parts[5].prev;
        g.advanced_step();
        assert!((g.buddy.parts[5].prev - prev - Vec2::X * 0.7).length() < 0.001);
        g.save.modes.insert("zerograv".into());
        let prev = g.buddy.parts[5].prev;
        g.advanced_step();
        assert_eq!(g.buddy.parts[5].prev, prev);
        g.save.modes.clear();
        g.save.modes.insert("toySwarm".into());
        toy(&mut g, "Baseballs", Vec2::new(100., 200.));
        toy(&mut g, "Baseballs", Vec2::new(200., 200.));
        let before = g.objects[0].prev;
        g.advanced_step();
        let force = before - g.objects[0].prev;
        assert!(force.x > 0. && force.length() <= 0.501);
        g.objects[1].pos = g.objects[0].pos + Vec2::X * 8.;
        g.objects[1].prev = g.objects[1].pos;
        let before = g.objects[0].prev;
        g.advanced_step();
        assert!(g.objects[0].prev.x > before.x);
        g.save.modes.insert("freeze".into());
        let before = g.objects[0].prev;
        g.advanced_step();
        assert_eq!(g.objects[0].prev, before);
    }
    #[test]
    fn live_rockets_are_not_safe_toys() {
        let mut g = game();
        for kind in ["Missiles", "Bazooka Rocket", "Grenades"] {
            toy(&mut g, kind, Vec2::new(20., 350.));
            assert!(!safe_toy(g.objects.last().unwrap()), "{kind}");
        }
        g.save.modes.insert("tetheredToys".into());
        let before: Vec<_> = g.objects.iter().map(|o| (o.pos, o.prev)).collect();
        g.advanced_step();
        assert_eq!(
            before,
            g.objects
                .iter()
                .map(|o| (o.pos, o.prev))
                .collect::<Vec<_>>()
        );
    }
    #[test]
    fn grapple_and_tethers_cap_impulses_and_release() {
        let mut g = game();
        g.save.item = "Grappling Hook".into();
        g.down = true;
        g.pressed = true;
        g.advanced_tool("grapple", Vec2::ZERO);
        let prev = g.buddy.parts[5].prev;
        g.advanced_step();
        assert!((g.buddy.parts[5].prev - prev).length() <= 2.001);
        assert_ne!(g.buddy.parts[5].prev, prev);
        g.down = false;
        g.advanced_step();
        assert!(g.advanced.grapple.is_none());
        g.save.modes.insert("tetheredToys".into());
        toy(&mut g, "Baseballs", Vec2::new(20., 350.));
        toy(&mut g, "Grenades", Vec2::new(20., 350.));
        let safe_prev = g.objects[0].prev;
        let dangerous_prev = g.objects[1].prev;
        g.advanced_step();
        let impulse = (g.objects[0].prev - safe_prev).length();
        assert!(impulse > 0. && impulse <= 2.001);
        assert_eq!(g.objects[1].prev, dangerous_prev);
        assert!(g.desktop_impacts.is_empty());
    }

    #[test]
    fn bumpers_score_once_per_cooldown_and_clear_resets_everything() {
        let mut g = game();
        g.save.modes.insert("pinball".into());
        let p = bumpers(g.stage)[0];
        move_buddy(&mut g.buddy, p, g.stage);
        g.advanced_step();
        assert_eq!(g.advanced.score, 1);
        let prev = g.buddy.parts[5].prev;
        g.advanced_step();
        assert_eq!(g.advanced.score, 1);
        assert_eq!(g.buddy.parts[5].prev, prev);
        for _ in 1..COOLDOWN {
            g.advanced_step();
        }
        assert_eq!(g.advanced.score, 2);
        g.pressed = true;
        g.advanced_tool("portalgun", p);
        g.advanced_tool("stasis", p);
        g.advanced.grapple = Some(p);
        g.advanced = Advanced::default();
        assert_eq!(g.advanced.portals, [None, None]);
        assert_eq!(g.advanced.next_portal, 0);
        assert_eq!(g.advanced.buddy_cooldown, 0);
        assert_eq!(g.advanced.toy_cooldown, 0);
        assert_eq!(g.advanced.bumper_cooldowns, [0; 3]);
        assert_eq!(g.advanced.score, 0);
        assert!(g.advanced.grapple.is_none() && g.advanced.zones.is_empty());
        assert!(g.desktop_impacts.is_empty());
    }

    #[test]
    fn chain_reaction_staggers_only_loose_timed_explosives() {
        let mut g = game();
        let p = Vec2::new(200., 200.);
        for kind in ["Grenades", "Dynamite", "Bomb", "Mines", "Baseballs"] {
            toy(&mut g, kind, p);
        }
        toy(&mut g, "Grenades", p);
        g.objects[5].held = true;
        toy(&mut g, "Grenades", p + Vec2::X * 250.);
        g.advanced_explosion(p, 999.);
        assert_eq!(g.objects[0].life, 5.);
        g.save.modes.insert("chainReaction".into());
        g.advanced_explosion(p, f32::NAN);
        assert_eq!(g.objects[0].life, 5.);
        g.advanced_explosion(p, 999.);
        for (i, frames) in [6., 9., 12.].into_iter().enumerate() {
            assert_eq!(g.objects[i].life, frames / 36.);
        }
        for o in &g.objects[3..] {
            assert_eq!(o.life, 5.);
        }
        g.objects[0].life = 1. / 36.;
        g.advanced_explosion(p, 100.);
        assert_eq!(g.objects[0].life, 1. / 36.);
    }
}
