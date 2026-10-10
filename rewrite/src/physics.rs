//! Frame-based approximation of Interactive Buddy's six-body physics.
//!
//! Coordinates use the original Flash convention (origin top-left, positive Y
//! down) on a stage whose dimensions are carried by `Buddy::bounds` (550x400
//! by default). Particle order matches `bodyParts` in the source:
//! head, right arm, left arm, right leg, left leg, torso.

use bevy::prelude::Vec2;

const DEFAULT_BOUNDS: Vec2 = Vec2::new(550.0, 400.0);
const DAMPING: f32 = 0.8;
const SPRING_DAMP: f32 = 0.225;

const HEAD: usize = 0;
const RIGHT_ARM: usize = 1;
const LEFT_ARM: usize = 2;
const RIGHT_LEG: usize = 3;
const LEFT_LEG: usize = 4;
const TORSO: usize = 5;

const OFFSETS: [Vec2; 5] = [
    Vec2::new(0.0, -39.2),
    Vec2::new(22.15, -10.0),
    Vec2::new(-22.15, -10.0),
    Vec2::new(15.05, 32.15),
    Vec2::new(-15.1, 32.15),
];

#[derive(Clone, Copy, Debug)]
pub struct Particle {
    pub pos: Vec2,
    pub prev: Vec2,
    pub radius: f32,
    pub mass: f32,
}

#[derive(Clone, Debug)]
pub struct Buddy {
    pub parts: Vec<Particle>,
    /// Stage width and height in screen coordinates, with origin at (0, 0).
    pub bounds: Vec2,
    pub awake: bool,
    pub is_clock: bool,
    pub open_ceiling: bool,
    /// Cash awarded by torso wall impacts during the latest `step`.
    pub impact_cash: f64,
    rotation: f32,
    angular_velocity: f32,
}

impl Buddy {
    /// Spawn at the root timeline's body placement (6463/20, 1931/20).
    pub fn new() -> Self {
        let torso = Vec2::new(6463.0 / 20.0, 1931.0 / 20.0);
        let radii = [13.5, 10.0, 10.0, 10.0, 10.0, 25.0];
        let masses = [0.2, 0.1, 0.1, 0.15, 0.15, 1.0];
        let parts = OFFSETS
            .iter()
            .copied()
            .chain([Vec2::ZERO])
            .zip(radii)
            .zip(masses)
            .map(|((offset, radius), mass)| {
                let pos = torso + offset;
                Particle {
                    pos,
                    prev: pos,
                    radius,
                    mass,
                }
            })
            .collect();
        Self {
            parts,
            bounds: DEFAULT_BOUNDS,
            awake: true,
            is_clock: false,
            open_ceiling: false,
            impact_cash: 0.0,
            rotation: 0.0,
            angular_velocity: 0.0,
        }
    }

    /// Set stage dimensions. Existing particle positions are not changed so
    /// callers can choose how to reposition the Buddy after a resize.
    pub fn set_bounds(&mut self, bounds: Vec2) {
        if bounds.is_finite() && bounds.x > 80.0 && bounds.y > 80.0 {
            self.bounds = bounds;
        }
    }

    /// Advance one source game frame (36 Hz). `elastic` maps to
    /// `improvedPhysics` in DoAction.as.
    pub fn step(&mut self, gravity: f32, elastic: bool) {
        self.impact_cash = 0.0;
        if self.parts.len() < 6 {
            return;
        }
        let gravity = if gravity.is_finite() { gravity } else { 0.0 };
        let bounds = if self.bounds.is_finite() && self.bounds.x > 80.0 && self.bounds.y > 80.0 {
            self.bounds
        } else {
            DEFAULT_BOUNDS
        };
        let grounded = self.parts[RIGHT_LEG].pos.y > bounds.y - 32.0
            || self.parts[LEFT_LEG].pos.y > bounds.y - 32.0;

        // bodyRot/rv integration and the awake/asleep ground response.
        self.angular_velocity *= 0.96 - if grounded { 0.1 } else { 0.0 };
        self.rotation += self.angular_velocity;
        self.rotation = (self.rotation + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
            - std::f32::consts::PI;
        if self.awake {
            if grounded {
                self.rotation *= 0.9;
            }
        } else if grounded && self.rotation < std::f32::consts::FRAC_PI_2 {
            if self.rotation > 0.0 {
                self.rotation = (self.rotation - std::f32::consts::FRAC_PI_2) * 0.95
                    + std::f32::consts::FRAC_PI_2;
            } else {
                self.rotation = (self.rotation + std::f32::consts::FRAC_PI_2) * 0.95
                    - std::f32::consts::FRAC_PI_2;
            }
        }

        // initPhysics stores each anchor as a distance and angle; these are
        // the equivalent values computed by doBodyPhysics every frame.
        let mut offsets = [Vec2::ZERO; 6];
        for (i, offset) in OFFSETS.iter().enumerate() {
            let distance = if i == HEAD && self.is_clock {
                0.0
            } else {
                offset.length()
            };
            let angle = -offset.y.atan2(offset.x) + self.rotation + std::f32::consts::FRAC_PI_2;
            offsets[i] = Vec2::new(distance * angle.sin(), distance * angle.cos());
        }
        let torso = self.parts[TORSO].pos;
        let mut velocities: [Vec2; 6] =
            std::array::from_fn(|i| self.parts[i].pos - self.parts[i].prev);

        // Source: arm springs *1.5, leg springs *0.5, head spring
        // *(2.5 - isAClock*2), with clock-head gravity disabled.
        for i in [LEFT_ARM, RIGHT_ARM] {
            velocities[i] += (torso + offsets[i] - self.parts[i].pos) * SPRING_DAMP * 1.5
                + Vec2::new(0.0, gravity);
        }
        for i in [LEFT_LEG, RIGHT_LEG] {
            velocities[i] += (torso + offsets[i] - self.parts[i].pos) * SPRING_DAMP * 0.5
                + Vec2::new(0.0, gravity);
        }
        let head_spring = if self.is_clock { 0.5 } else { 2.5 };
        velocities[HEAD] +=
            (torso + offsets[HEAD] - self.parts[HEAD].pos) * SPRING_DAMP * head_spring
                + Vec2::new(0.0, if self.is_clock { 0.0 } else { gravity });

        // rigidPhysics tests anchor displacement against 35px, then projects
        // it onto a 34px radius. This is an error-radius, not torso-distance.
        for i in 0..TORSO {
            let anchor = torso + offsets[i];
            let error = self.parts[i].pos - anchor;
            let distance = error.length();
            if distance > 35.0 && distance.is_finite() {
                self.parts[i].pos = anchor + error * (34.0 / distance);
            }
        }
        if self.is_clock {
            let delta = self.parts[HEAD].pos - torso;
            let distance = delta.length();
            if distance > 15.0 && distance.is_finite() {
                self.parts[HEAD].pos = torso + delta * (15.0 / distance);
            }
        }

        // Torso restoring acceleration from all attachment residuals.
        let mut body_x = (self.parts[TORSO].pos.x - self.parts[HEAD].pos.x + offsets[HEAD].x) * 2.0
            + (self.parts[TORSO].pos.x - self.parts[LEFT_ARM].pos.x + offsets[LEFT_ARM].x)
            + (self.parts[TORSO].pos.x - self.parts[RIGHT_ARM].pos.x + offsets[RIGHT_ARM].x);
        let left_leg_weight = if self.parts[LEFT_LEG].pos.y > bounds.y - 20.0 {
            3.0
        } else {
            1.0
        };
        let right_leg_weight = if self.parts[RIGHT_LEG].pos.y > bounds.y - 20.0 {
            3.0
        } else {
            1.0
        };
        body_x += (self.parts[TORSO].pos.x - self.parts[LEFT_LEG].pos.x + offsets[LEFT_LEG].x)
            * left_leg_weight;
        body_x += (self.parts[TORSO].pos.x - self.parts[RIGHT_LEG].pos.x + offsets[RIGHT_LEG].x)
            * right_leg_weight;
        let body_y = (self.parts[TORSO].pos.y - self.parts[HEAD].pos.y + offsets[HEAD].y) * 2.0
            + (self.parts[TORSO].pos.y - self.parts[LEFT_ARM].pos.y + offsets[LEFT_ARM].y)
            + (self.parts[TORSO].pos.y - self.parts[RIGHT_ARM].pos.y + offsets[RIGHT_ARM].y)
            + (self.parts[TORSO].pos.y - self.parts[LEFT_LEG].pos.y + offsets[LEFT_LEG].y) * 2.0
            + (self.parts[TORSO].pos.y - self.parts[RIGHT_LEG].pos.y + offsets[RIGHT_LEG].y) * 2.0;
        velocities[TORSO].x += -body_x / 5.0 * SPRING_DAMP * 1.75;
        velocities[TORSO].y += -body_y / 5.0 * SPRING_DAMP * 1.75 + gravity;

        if elastic {
            if grounded {
                velocities[TORSO].x *= DAMPING;
                velocities[LEFT_LEG].x *= DAMPING;
                velocities[RIGHT_LEG].x *= DAMPING;
            }
            let body_velocity = velocities[TORSO];
            for velocity in &mut velocities[..TORSO] {
                *velocity = (*velocity - body_velocity) * DAMPING + body_velocity;
            }
            velocities[TORSO] *= DAMPING;
        } else {
            for velocity in &mut velocities[..TORSO] {
                *velocity *= DAMPING;
            }
        }

        for (i, part) in self.parts.iter_mut().enumerate().take(6) {
            if !velocities[i].is_finite() || !part.pos.is_finite() {
                part.pos = bounds * 0.5;
                part.prev = part.pos;
                continue;
            }
            part.pos += velocities[i];
            part.prev = part.pos - velocities[i];
            self.impact_cash += Self::collide_source_bounds(part, i, self.open_ceiling, bounds);
        }
    }

    fn collide_source_bounds(
        part: &mut Particle,
        i: usize,
        open_ceiling: bool,
        bounds: Vec2,
    ) -> f64 {
        let (left, right, top, bottom) = match i {
            TORSO => (36.0, bounds.x - 31.0, 31.0, bounds.y - 31.0),
            HEAD => (26.0, bounds.x - 21.0, 21.0, bounds.y - 21.0),
            _ => (21.0, bounds.x - 16.0, 18.0, bounds.y - 18.0),
        };
        let mut cash = 0.0;
        if part.pos.x < left {
            let incoming = part.pos.x - part.prev.x;
            part.pos.x = left;
            let reflected = incoming * -0.5;
            part.prev.x = part.pos.x - reflected;
            if i == TORSO {
                cash += Self::impact_reward(reflected);
            }
        } else if part.pos.x > right {
            let incoming = part.pos.x - part.prev.x;
            part.pos.x = right;
            let reflected = incoming * -0.5;
            part.prev.x = part.pos.x - reflected;
            if i == TORSO {
                cash += Self::impact_reward(reflected);
            }
        }
        if part.pos.y > bottom {
            let incoming = part.pos.y - part.prev.y;
            part.pos.y = bottom;
            let reflected = incoming * -0.5;
            part.prev.y = part.pos.y - reflected;
            if i == TORSO {
                cash += Self::impact_reward(reflected);
            }
        } else if !open_ceiling && part.pos.y < top {
            let incoming = part.pos.y - part.prev.y;
            part.pos.y = top;
            let reflected = incoming * -0.5;
            part.prev.y = part.pos.y - reflected;
            if i == TORSO {
                cash += Self::impact_reward(reflected);
            }
        }
        cash
    }

    fn impact_reward(reflected_velocity: f32) -> f64 {
        let speed = reflected_velocity.abs();
        if speed > 25.0 {
            (speed as f64 * 0.02).min(0.5)
        } else {
            0.0
        }
    }

    pub fn impulse(&mut self, point: Vec2, force: Vec2, radius: f32) -> f32 {
        if !point.is_finite() || !force.is_finite() || !radius.is_finite() || radius <= 0.0 {
            return 0.0;
        }
        let mut total = 0.0;
        let torso = self.parts.get(TORSO).map_or(Vec2::ZERO, |part| part.pos);
        for part in &mut self.parts {
            let distance = part.pos.distance(point);
            if distance < radius {
                let falloff = 1.0 - distance / radius;
                let dv = force * (falloff / part.mass.max(0.05));
                part.prev -= dv;
                total += dv.length();
            }
        }
        let lever = point - torso;
        self.add_rotation_velocity((lever.perp_dot(force) / 1800.0).clamp(-0.35, 0.35));
        total
    }

    pub fn drag(&mut self, index: usize, target: Vec2) {
        let torso = self.parts.get(TORSO).map_or(Vec2::ZERO, |part| part.pos);
        if let Some(part) = self.parts.get_mut(index) {
            if target.is_finite() {
                let movement = target - part.pos;
                if index != TORSO {
                    self.angular_velocity += (part.pos - torso).perp_dot(movement) / 2200.0;
                }
                part.pos = target;
                part.prev = target - movement * 0.65;
            }
        }
    }

    pub fn nearest(&self, point: Vec2) -> Option<usize> {
        if !point.is_finite() {
            return None;
        }
        let (index, part) = self
            .parts
            .iter()
            .enumerate()
            .filter(|(_, part)| part.pos.is_finite())
            .min_by(|(_, a), (_, b)| {
                a.pos
                    .distance_squared(point)
                    .total_cmp(&b.pos.distance_squared(point))
            })?;
        (part.pos.distance(point) <= part.radius + 10.0).then_some(index)
    }

    pub fn recover_upright(&mut self) {
        let angle = self.rotation.rem_euclid(std::f32::consts::TAU);
        let error = if angle > std::f32::consts::PI {
            std::f32::consts::TAU - angle
        } else {
            -angle
        };
        self.angular_velocity += (error * 0.004).clamp(-0.03, 0.03);
    }

    pub fn add_rotation_velocity(&mut self, amount: f32) {
        if amount.is_finite() {
            self.angular_velocity += amount.clamp(-1.0, 1.0);
        }
    }

    pub fn rotation(&self) -> f32 {
        self.rotation
    }

    pub fn set_rotation(&mut self, radians: f32) {
        if radians.is_finite() {
            self.rotation = (radians + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
        }
    }
}

impl Default for Buddy {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remains_finite_and_within_source_bounds_under_gravity() {
        let mut buddy = Buddy::new();
        for _ in 0..2_000 {
            buddy.step(1.0, false);
        }
        let bounds = buddy.bounds;
        for (i, part) in buddy.parts.iter().enumerate() {
            assert!(part.pos.is_finite() && part.prev.is_finite());
            let (left, right, top, bottom) = match i {
                TORSO => (36.0, bounds.x - 31.0, 31.0, bounds.y - 31.0),
                HEAD => (26.0, bounds.x - 21.0, 21.0, bounds.y - 21.0),
                _ => (21.0, bounds.x - 16.0, 18.0, bounds.y - 18.0),
            };
            assert!((left..=right).contains(&part.pos.x));
            assert!((top..=bottom).contains(&part.pos.y));
        }
    }

    #[test]
    fn desktop_work_area_bounds_remain_finite_and_reset_to_default() {
        let mut buddy = Buddy::new();
        buddy.set_bounds(Vec2::new(1280.0, 700.0));
        assert_eq!(buddy.bounds, Vec2::new(1280.0, 700.0));

        for _ in 0..1_000 {
            buddy.step(1.0, false);
        }
        assert!(
            buddy
                .parts
                .iter()
                .all(|p| p.pos.is_finite() && p.prev.is_finite())
        );
        for (i, part) in buddy.parts.iter().enumerate() {
            let (left, right, top, bottom) = match i {
                TORSO => (36.0, 1280.0 - 31.0, 31.0, 700.0 - 31.0),
                HEAD => (26.0, 1280.0 - 21.0, 21.0, 700.0 - 21.0),
                _ => (21.0, 1280.0 - 16.0, 18.0, 700.0 - 18.0),
            };
            assert!((left..=right).contains(&part.pos.x));
            assert!((top..=bottom).contains(&part.pos.y));
        }

        // Buddy::new is the canonical reset path and should retain classic
        // Flash-stage dimensions regardless of a prior desktop work area.
        let reset = Buddy::new();
        assert_eq!(reset.bounds, DEFAULT_BOUNDS);
    }

    #[test]
    fn impulse_adds_velocity_and_reaches_parts_in_radius() {
        let mut buddy = Buddy::new();
        let torso = buddy.parts[TORSO].pos;
        assert!(buddy.impulse(torso, Vec2::new(0.0, -2.0), 80.0) > 0.0);
        assert!((buddy.parts[TORSO].pos - buddy.parts[TORSO].prev).y < 0.0);
    }

    #[test]
    fn drag_and_nearest_respect_target_and_pick_radius() {
        let mut buddy = Buddy::new();
        let target = Vec2::new(80.0, 90.0);
        buddy.drag(HEAD, target);
        assert_eq!(buddy.parts[HEAD].pos, target);
        assert!(buddy.parts[HEAD].prev.distance(target) > 0.0);
        assert_eq!(buddy.nearest(target), Some(HEAD));
        assert_eq!(buddy.nearest(Vec2::new(0.0, 0.0)), None);
    }

    #[test]
    fn awake_and_clock_modes_affect_rotation_and_head_motion() {
        let mut asleep = Buddy::new();
        asleep.awake = false;
        asleep.set_rotation(0.0);
        asleep.parts[LEFT_LEG].pos.y = asleep.bounds.y - 20.0;
        asleep.step(0.0, false);
        assert!(asleep.rotation() < 0.0);

        let mut clock = Buddy::new();
        clock.is_clock = true;
        let mut clock_without_gravity = Buddy::new();
        clock_without_gravity.is_clock = true;
        clock.step(1.0, false);
        clock_without_gravity.step(0.0, false);
        let clock_head_velocity = clock.parts[HEAD].pos - clock.parts[HEAD].prev;
        let zero_gravity_head_velocity =
            clock_without_gravity.parts[HEAD].pos - clock_without_gravity.parts[HEAD].prev;
        assert!((clock_head_velocity.y - zero_gravity_head_velocity.y).abs() < 0.001);
        assert!(clock.parts[HEAD].pos.distance(clock.parts[TORSO].pos) <= 15.1);
    }
}
