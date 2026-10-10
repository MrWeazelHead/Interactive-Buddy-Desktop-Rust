//! Compact autonomous Buddy behavior based on frame_25's `doAI`, `idleAction`,
//! and the `dance`, `burn`, `walkToPoint`, and `jumpUp` action handlers.
//!
//! Call once per 36 Hz simulation frame, before `Buddy::step`. `interaction`
//! represents active tickling/user contact; `burn` is zero when safe and
//! positive while any Buddy part is burning. `radio` and `nearby_grenade` carry
//! world positions; `mouse` drives tickle flee behavior. The source's
//! object/action queue is deliberately reduced to one high-priority behavior.

use bevy::prelude::Vec2;

use crate::physics::Buddy;

const HEAD: usize = 0;
const RIGHT_ARM: usize = 1;
const LEFT_ARM: usize = 2;
const RIGHT_LEG: usize = 3;
const LEFT_LEG: usize = 4;
const TORSO: usize = 5;

#[derive(Clone, Debug)]
pub struct Behavior {
    frame: u64,
    idle_frames: u16,
    idle_threshold: u16,
    pace_dir: f32,
    pace_frames: u16,
    next_pace: u16,
    jump_cooldown: u16,
    recovering: u8,
    impacted: bool,
    stable_frames: u8,
    rng: u64,
    idle_checked: bool,
    pending_flip: bool,
    /// Reward emitted during the most recent `step` call.
    pub reward: f64,
}

impl Default for Behavior {
    fn default() -> Self {
        Self {
            frame: 0,
            idle_frames: 0,
            // Source init uses a randomized 200..399 frame threshold.
            idle_threshold: 300,
            pace_dir: 1.0,
            pace_frames: 0,
            next_pace: 0,
            jump_cooldown: 0,
            recovering: 0,
            impacted: false,
            stable_frames: 0,
            rng: 0x4D59_5DF4_D0F3_3173,
            idle_checked: false,
            pending_flip: false,
            reward: 0.0,
        }
    }
}

impl Behavior {
    /// Run the Buddy's autonomous response for one 36 Hz frame.
    ///
    /// The returned label is the active high-level reaction for UI/debug use;
    /// physical movement is applied directly to the Verlet particles.
    pub fn step(
        &mut self,
        buddy: &mut Buddy,
        emotion: f32,
        burn: f32,
        interaction: bool,
        radio: Option<Vec2>,
        nearby_grenade: Option<Vec2>,
        mouse: Vec2,
    ) -> Option<String> {
        self.reward = 0.0;
        if buddy.parts.len() < 6 {
            return None;
        }
        self.frame = self.frame.wrapping_add(1);
        self.jump_cooldown = self.jump_cooldown.saturating_sub(1);
        let torso = buddy.parts[TORSO].pos;
        let bounds = buddy.bounds;
        let grounded = buddy.parts[RIGHT_LEG].pos.y > bounds.y - 32.0
            || buddy.parts[LEFT_LEG].pos.y > bounds.y - 32.0;

        // Impacts are read from Verlet displacement. Once feet settle, gradually
        // steer the source's bodyRot back toward its neutral standing angle.
        let speed_sq = buddy
            .parts
            .iter()
            .map(|part| (part.pos - part.prev).length_squared())
            .fold(0.0_f32, f32::max);
        if speed_sq > 12.0 * 12.0 {
            self.impacted = true;
            self.stable_frames = 0;
        } else if grounded {
            self.stable_frames = self.stable_frames.saturating_add(1);
        } else {
            self.stable_frames = 0;
        }
        if self.impacted && grounded && self.stable_frames >= 8 {
            self.recovering = 90;
            self.impacted = false;
        }
        let recovering = self.recovering > 0;
        if recovering {
            buddy.recover_upright();
            self.recovering -= 1;
        }

        if burn.is_finite() && burn > 0.0 {
            self.idle_frames = 0;
            self.idle_checked = false;
            if grounded {
                self.pace_dir = if torso.x > bounds.x - 120.0 {
                    -1.0
                } else if torso.x < 120.0 {
                    1.0
                } else {
                    self.pace_dir
                };
                self.run(buddy, self.pace_dir, 5.0);
                if self.random() < 0.02 {
                    Self::jump(buddy, 15.0);
                }
            }
            return Some("burn".to_owned());
        }

        if let Some(grenade) = nearby_grenade.filter(|g| g.is_finite()) {
            self.idle_frames = 0;
            self.idle_checked = false;
            if grounded && (torso.x - grenade.x).abs() < 275.0 {
                let direction = if grenade.x < 125.0 {
                    1.0
                } else if grenade.x > bounds.x - 125.0 {
                    -1.0
                } else {
                    (torso.x - grenade.x).signum() + 0.01
                };
                self.walk(buddy, direction);
                return Some("run_from_grenade".to_owned());
            }
        }

        if interaction {
            self.idle_frames = 0;
            self.idle_checked = false;
            let (body_x, body_y) = if grounded {
                (self.random() * 8.0 - 4.0, self.random() * 10.0 - 6.0)
            } else {
                (self.random() * 2.0 - 1.0, self.random() * 10.0 - 6.0)
            };
            let kick = Vec2::new(body_x, body_y);
            Self::add_velocity(buddy, TORSO, kick);
            Self::add_velocity(buddy, LEFT_ARM, Vec2::new(0.0, 3.0 - self.random() * 10.0));
            Self::add_velocity(buddy, RIGHT_ARM, Vec2::new(0.0, 3.0 - self.random() * 10.0));
            let flee = Self::run_from_mouse_direction(torso.x, mouse.x, bounds.x);
            if grounded {
                self.walk(buddy, flee);
            }
            self.reward += 0.075;
            if grounded && self.jump_cooldown == 0 && self.random() > 0.96 {
                Self::jump(buddy, 25.0);
                self.jump_cooldown = 40;
            }
            return Some("tickle".to_owned());
        }

        if self.pending_flip && grounded {
            self.pending_flip = false;
            Self::jump(buddy, 35.0);
            buddy.add_rotation_velocity(0.3175);
            self.reward += 15.0;
            self.idle_frames = 0;
            self.idle_threshold = 300 + (self.random() * 100.0) as u16;
            self.idle_checked = false;
            return Some("flip".to_owned());
        }

        if let Some(radio_position) = radio.filter(|p| p.is_finite()).filter(|_| emotion > -60.0) {
            self.idle_frames = 0;
            self.idle_checked = false;
            if grounded {
                let phase = self.frame as f32 * std::f32::consts::PI * 0.1;
                Self::add_velocity(buddy, LEFT_LEG, Vec2::new(phase.sin(), phase.cos() * 4.0));
                Self::add_velocity(
                    buddy,
                    RIGHT_LEG,
                    Vec2::new(
                        phase.sin(),
                        (phase + std::f32::consts::FRAC_PI_2).cos() * 4.0,
                    ),
                );
                Self::add_velocity(buddy, HEAD, Vec2::new(0.0, (phase * 0.25).sin() * 3.0));
                Self::add_velocity(buddy, LEFT_ARM, Vec2::new((phase * 0.5).sin() * 2.0, 0.0));
                Self::add_velocity(
                    buddy,
                    RIGHT_ARM,
                    Vec2::new((phase * 0.5 + std::f32::consts::FRAC_PI_2).sin() * 2.0, 0.0),
                );
                // Source doBodyPhysics follows the actual radio position when
                // it is more than 100px away horizontally.
                if (torso.x - radio_position.x).abs() > 100.0 {
                    Self::add_velocity(
                        buddy,
                        TORSO,
                        Vec2::new(-(torso.x - radio_position.x).signum(), 0.0),
                    );
                }
            }
            return Some("dance".to_owned());
        }

        // Priority queue equivalent: higher-priority reactions above suppress
        // ordinary idle behavior, just as burn/tickle actions preempt idle.
        self.idle_frames = self.idle_frames.saturating_add(1);
        if self.pace_frames == 0 && !self.idle_checked && self.idle_frames > self.idle_threshold {
            self.idle_checked = true;
            if emotion >= 80.0 {
                let roll = self.random();
                if roll >= 0.65 {
                    if self.random() < 0.5 {
                        // Exact source `flip`: jump(35, 0), rv=0.3175,
                        // and $15 only when jump succeeds on the ground.
                        self.pending_flip = true;
                        if grounded {
                            self.pending_flip = false;
                            Self::jump(buddy, 35.0);
                            buddy.add_rotation_velocity(0.3175);
                            self.reward += 15.0;
                            self.idle_frames = 0;
                            self.idle_threshold = 300 + (self.random() * 100.0) as u16;
                            self.idle_checked = false;
                            return Some("flip".to_owned());
                        }
                        return Some("flip".to_owned());
                    }
                    let amount = 50.0 + (self.random() * 6.0).floor() * 10.0;
                    self.reward += amount as f64;
                    self.idle_frames = 0;
                    self.idle_threshold = 300 + (self.random() * 100.0) as u16;
                    self.idle_checked = false;
                    return Some("happy_reward".to_owned());
                }
                return Some("happy".to_owned());
            } else if emotion <= -80.0 {
                return Some("sad".to_owned());
            } else if (-80.0..-20.0).contains(&emotion) {
                let sad_chance = (-(emotion + 20.0)) / 60.0;
                return Some(
                    if self.random() < sad_chance {
                        "sad"
                    } else {
                        "idle"
                    }
                    .to_owned(),
                );
            } else if (20.0..80.0).contains(&emotion) {
                let happy_chance = (emotion - 20.0) / 60.0;
                return Some(
                    if self.random() < happy_chance {
                        "happy"
                    } else {
                        "idle"
                    }
                    .to_owned(),
                );
            } else {
                return Some("idle".to_owned());
            }
        }
        if self.pace_frames > 0 {
            if grounded {
                self.walk(buddy, self.pace_dir);
            }
            self.pace_frames -= 1;
            if self.pace_frames == 0 {
                self.next_pace = 120;
            }
            return Some("idle_walk".to_owned());
        }
        self.next_pace = self.next_pace.saturating_sub(1);
        if self.idle_frames >= self.idle_threshold && self.next_pace == 0 && grounded {
            self.idle_frames = 0;
            self.idle_threshold = 300 + (self.frame % 100) as u16;
            self.idle_checked = false;
            self.pace_dir = if torso.x > bounds.x * 0.5 { -1.0 } else { 1.0 };
            self.pace_frames = 36;
            self.walk(buddy, self.pace_dir);
            return Some("idle_walk".to_owned());
        }

        if self.impacted || recovering {
            Some("recovering".to_owned())
        } else {
            None
        }
    }

    fn random(&mut self) -> f32 {
        // Small xorshift PRNG local to this behavior instance, avoiding any
        // shared/global random stream in the game loop.
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng = x;
        ((x >> 40) as u32) as f32 / ((1u32 << 24) as f32)
    }

    fn run_from_mouse_direction(body_x: f32, mouse_x: f32, stage_width: f32) -> f32 {
        let direction = (body_x - mouse_x).signum();
        if body_x < 80.0 {
            direction.max(0.0)
        } else if body_x > stage_width - 80.0 {
            direction.min(0.0)
        } else {
            direction
        }
    }

    fn add_velocity(buddy: &mut Buddy, index: usize, velocity: Vec2) {
        if let Some(part) = buddy.parts.get_mut(index) {
            part.prev -= velocity;
        }
    }

    fn walk(&mut self, buddy: &mut Buddy, direction: f32) {
        if direction == 0.0 {
            return;
        }
        let phase = self.frame as f32 * 0.15 * -direction;
        Self::add_velocity(
            buddy,
            RIGHT_LEG,
            Vec2::new(phase.sin() * 2.0, phase.cos() * 3.5),
        );
        Self::add_velocity(
            buddy,
            LEFT_LEG,
            Vec2::new((phase + 180.0).sin() * 2.0, (phase + 180.0).cos() * 3.5),
        );
        for i in [HEAD, RIGHT_ARM, LEFT_ARM, TORSO] {
            Self::add_velocity(buddy, i, Vec2::new(direction * 3.0, 0.0));
        }
    }

    fn run(&mut self, buddy: &mut Buddy, direction: f32, speed: f32) {
        let phase = self.frame as f32 * 0.15 * -direction;
        Self::add_velocity(
            buddy,
            RIGHT_LEG,
            Vec2::new(phase.sin() * 2.0, phase.cos() * 4.5),
        );
        Self::add_velocity(
            buddy,
            LEFT_LEG,
            Vec2::new((phase + 180.0).sin() * 2.0, (phase + 180.0).cos() * 4.5),
        );
        for i in [HEAD, RIGHT_ARM, LEFT_ARM, TORSO] {
            Self::add_velocity(buddy, i, Vec2::new(direction * speed, 0.0));
        }
    }

    fn jump(buddy: &mut Buddy, height: f32) {
        Self::add_velocity(buddy, TORSO, Vec2::new(0.0, -height));
        for i in [RIGHT_LEG, LEFT_LEG] {
            Self::add_velocity(buddy, i, Vec2::new(0.0, -height * 1.25));
        }
        for i in [HEAD, RIGHT_ARM, LEFT_ARM] {
            Self::add_velocity(buddy, i, Vec2::new(0.0, -height));
        }
    }
}
