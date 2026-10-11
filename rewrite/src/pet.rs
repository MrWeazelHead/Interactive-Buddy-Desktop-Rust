//! Optional care simulation. Needs use simulation time; no offline neglect.
use crate::*;

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Pet {
    pub enabled: bool,
    pub needs: [f32; 3], // Food, water, play: 100 is satisfied.
    pub affection: f32,
    pub decay: f32,
    pub autonomous: bool,
    pub shy: bool,
    pub playful: bool,
    #[serde(rename = "stat_panel")]
    pub show_stats: bool,
    pub need_dialogue: bool,
    pub need_lights: bool,
    pub stations: Vec<Station>,
    pub cage: Option<f32>,
    pub hygiene: f32,
    pub health: f32,
    pub energy: f32,
    pub toilet: f32, // Urgency: 100 means an accident is due.
    pub wastes: Vec<Waste>,
    pub accidents: bool,
    pub sleep: bool,
    pub accident_cooldown: f32, // Simulation seconds only.
}
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Waste {
    pub x: f32, // Fraction of work-area width.
}
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Station {
    pub kind: usize,
    pub x: f32, // Fraction of work-area width, so switching surfaces is safe.
    pub supply: f32,
}
const NEED_LIMITS: [f32; 7] = [40., 40., 35., 40., 60., 30., 35.];
const NEED_MESSAGES: [&str; 7] = [
    "I'm hungry. Could you put out some food?",
    "I'm thirsty. Could you refill my water?",
    "I'm bored. Let's play with some toys!",
    "I feel grubby. Could you wash me?",
    "I don't feel well. Could I have some medicine?",
    "I'm sleepy. Could you put down a bed?",
    "I need the toilet! Where's my litter tray?",
];
impl Pet {
    fn cage_bounds(&self, stage: Vec2) -> Option<(Vec2, Vec2)> {
        if !self.enabled || !stage.is_finite() || stage.min_element() < 100. {
            return None;
        }
        let x = self.cage?;
        let half = 80_f32.min(stage.x * 0.45);
        let center = (x * stage.x).clamp(half, stage.x - half);
        Some((
            Vec2::new(center - half, (stage.y - 180.).max(24.)),
            Vec2::new(center + half, stage.y - 5.),
        ))
    }
    fn need_values(&self) -> [f32; 7] {
        [
            self.needs[0],
            self.needs[1],
            self.needs[2],
            self.hygiene,
            self.health,
            self.energy,
            100. - self.toilet,
        ]
    }
    fn need_message(&self) -> Option<&'static str> {
        self.need_values()
            .into_iter()
            .enumerate()
            .filter(|(i, v)| *v < NEED_LIMITS[*i])
            .min_by(|(i, a), (j, b)| (a / NEED_LIMITS[*i]).total_cmp(&(b / NEED_LIMITS[*j])))
            .map(|(i, _)| NEED_MESSAGES[i])
    }
}

pub(crate) fn draw_need_lights(d: &mut Painter<'_, '_, '_>, g: &Game, mood: Vec2) {
    if !g.save.pet.enabled || !g.save.pet.need_lights {
        return;
    }
    let colors = [
        Color::srgb(1., 0.6, 0.2),
        Color::srgb(0.3, 0.7, 1.),
        Color::srgb(0.8, 0.4, 1.),
        Color::srgb(0.3, 1., 0.8),
        Color::srgb(1., 0.3, 0.4),
        Color::srgb(1., 0.9, 0.3),
        Color::srgb(0.7, 0.45, 0.25),
    ];
    let x = (mood.x - 14.).clamp(2., (g.stage.x - 30.).max(2.));
    let y = (mood.y + 13.).clamp(2., (g.stage.y - 6.).max(2.));
    for (i, value) in g.save.pet.need_values().into_iter().enumerate() {
        if value < NEED_LIMITS[i] {
            d.rect(
                x + i as f32 * 4.,
                y,
                4.,
                4.,
                Color::srgba(0.02, 0.03, 0.05, 0.8),
            );
            d.rect(x + 1. + i as f32 * 4., y + 1., 2., 2., colors[i]);
        }
    }
}

impl Default for Pet {
    fn default() -> Self {
        Self {
            enabled: false,
            needs: [75.; 3],
            affection: 60.,
            decay: 1.,
            autonomous: true,
            shy: true,
            playful: true,
            show_stats: false,
            need_dialogue: true,
            need_lights: true,
            stations: vec![],
            cage: None,
            hygiene: 90.,
            health: 100.,
            energy: 85.,
            toilet: 0.,
            wastes: vec![],
            accidents: true,
            sleep: true,
            accident_cooldown: 0.,
        }
    }
}
impl Pet {
    pub fn validate(&mut self) {
        for n in &mut self.needs {
            *n = finite_clamp(*n, 75., 0., 100.);
        }
        self.affection = finite_clamp(self.affection, 60., 0., 100.);
        self.decay = finite_clamp(self.decay, 1., 0.5, 4.);
        self.hygiene = finite_clamp(self.hygiene, 90., 0., 100.);
        self.health = finite_clamp(self.health, 100., 25., 100.);
        self.energy = finite_clamp(self.energy, 85., 0., 100.);
        self.toilet = finite_clamp(self.toilet, 0., 0., 100.);
        self.accident_cooldown = finite_clamp(self.accident_cooldown, 0., 0., 60.);
        self.cage = self
            .cage
            .filter(|x| x.is_finite())
            .map(|x| x.clamp(0.05, 0.95));
        self.wastes.retain(|w| w.x.is_finite());
        self.wastes.truncate(16);
        for w in &mut self.wastes {
            w.x = w.x.clamp(0.05, 0.95);
        }
        self.stations
            .retain(|s| s.kind < SUPPLIES.len() && s.x.is_finite() && s.supply.is_finite());
        self.stations.truncate(16);
        for s in &mut self.stations {
            s.x = s.x.clamp(0.05, 0.95);
            s.supply = s.supply.clamp(0., 100.);
        }
    }
}
fn finite_clamp(value: f32, fallback: f32, low: f32, high: f32) -> f32 {
    if value.is_finite() {
        value.clamp(low, high)
    } else {
        fallback
    }
}
pub(crate) const SUPPLIES: [&str; 5] = [
    "Food Bowl",
    "Water Bowl",
    "Toy Basket",
    "Pet Bed",
    "Litter Tray",
];
const KEYS: [&str; 5] = ["foodbowl", "waterbowl", "toybasket", "petbed", "littertray"];

impl Pet {
    fn station_need(&self, kind: usize) -> Option<f32> {
        match kind {
            0..=2 => Some(self.needs[kind]),
            3 if self.sleep => Some(self.energy),
            4 => Some(100. - self.toilet),
            _ => None,
        }
    }
    fn care_tick(&mut self, x: f32, at_toilet: bool) {
        let dt = self.decay / 36.;
        self.accident_cooldown = (self.accident_cooldown - 1. / 36.).max(0.);
        self.toilet = (self.toilet + 0.12 * dt).min(100.);
        self.hygiene =
            (self.hygiene - (0.015 + self.wastes.len().min(5) as f32 * 0.01) * dt).max(0.);
        // Illness is reversible discomfort, never death or physical poison damage.
        let health_rate = if self.hygiene < 25. {
            -0.04
        } else if self.hygiene > 60. {
            0.06
        } else {
            0.
        };
        self.health = (self.health + health_rate * dt).clamp(25., 100.);
        let deficiency = (100. - self.health) / 100.;
        self.energy = (self.energy - (0.06 + 0.04 * deficiency) * dt).max(0.);
        if self.accidents && !at_toilet && self.toilet >= 100. && self.accident_cooldown <= 0. {
            if self.wastes.len() < 16 {
                self.wastes.push(Waste {
                    x: finite_clamp(x, 0.5, 0.05, 0.95),
                });
            }
            self.toilet = 0.;
            self.hygiene = (self.hygiene - 8.).max(0.);
            self.accident_cooldown = 60.;
        }
    }
}

impl Game {
    pub(crate) fn pet_action(&mut self, key: &str) {
        match key {
            "toggle" => {
                self.save.pet.enabled = !self.save.pet.enabled;
                self.cancel_tool_input();
                self.save.item = "Open Hand".into();
                self.desktop_interact = false;
                self.behavior = behavior::Behavior::default();
                self.notify(if self.save.pet.enabled {
                    "Passive Pet on. Place care stations from Items > Pet Items."
                } else {
                    "Passive Pet off. Normal Buddy behavior restored."
                });
            }
            "decay" => {
                self.save.pet.decay = if self.save.pet.decay < 1. {
                    1.
                } else if self.save.pet.decay < 4. {
                    4.
                } else {
                    0.5
                }
            }
            "auto" => self.save.pet.autonomous = !self.save.pet.autonomous,
            "shy" => self.save.pet.shy = !self.save.pet.shy,
            "playful" => self.save.pet.playful = !self.save.pet.playful,
            "stats" => self.save.pet.show_stats = !self.save.pet.show_stats,
            "need_dialogue" => self.save.pet.need_dialogue = !self.save.pet.need_dialogue,
            "need_lights" => self.save.pet.need_lights = !self.save.pet.need_lights,
            "accidents" => self.save.pet.accidents = !self.save.pet.accidents,
            "sleep" => {
                self.save.pet.sleep = !self.save.pet.sleep;
                self.pet_target = None;
            }
            "reset" => {
                self.save.pet.needs = [75.; 3];
                self.save.pet.affection = 60.;
                self.save.pet.hygiene = 90.;
                self.save.pet.health = 100.;
                self.save.pet.energy = 85.;
                self.save.pet.toilet = 0.;
                self.save.pet.accident_cooldown = 0.;
                self.save.pet.wastes.clear();
                self.pet_target = None;
            }
            "release_cage" => {
                self.save.pet.cage = None;
                self.pet_target = None;
                self.notify("You're free to roam again, Buddy!");
            }
            "clear" => {
                self.save.pet.stations.clear();
                self.pet_target = None;
            }
            "food" | "water" | "toy" | "bed" | "littertray" => {
                let index = ["food", "water", "toy", "bed", "littertray"]
                    .iter()
                    .position(|k| *k == key)
                    .unwrap();
                self.action(Action::Select("Items".into(), SUPPLIES[index].into()));
                self.panel.clear();
                self.desktop_section.clear();
            }
            _ => return,
        }
        self.persist();
    }
    pub(crate) fn pet_damage(&mut self, strength: f32) {
        if self.save.pet.enabled && strength.is_finite() && strength > 0. {
            self.save.pet.affection = (self.save.pet.affection - strength.clamp(0.1, 15.)).max(0.);
            self.emotion = self.save.pet.affection * 2. - 100.;
        }
    }
    pub(crate) fn pet_tool(&mut self, item: &str, p: Vec2) {
        if !p.is_finite() || self.stage.x <= 0. || self.mode("freeze") {
            return;
        }
        if item == "Pet Cage" {
            if self.pressed && self.save.pet.enabled {
                self.save.pet.cage = Some((p.x / self.stage.x).clamp(0.05, 0.95));
                let Some((low, high)) = self.save.pet.cage_bounds(self.stage) else {
                    self.save.pet.cage = None;
                    return;
                };
                let target = Vec2::new((low.x + high.x) * 0.5, self.stage.y - 70.);
                let delta = target - self.buddy.parts[5].pos;
                for part in &mut self.buddy.parts {
                    part.pos += delta;
                    part.prev += delta;
                }
                let count = self.save.pet.stations.len();
                for (i, s) in self.save.pet.stations.iter_mut().enumerate() {
                    s.x = (low.x
                        + 20.
                        + (high.x - low.x - 40.) * (i + 1) as f32 / (count + 1) as f32)
                        / self.stage.x;
                }
                self.pet_target = None;
                self.pet_cage_step();
                self.notify("Pet cage placed. Release Buddy in Passive Pet settings.");
                self.persist();
            }
            return;
        }
        if item == "Poop Scoop" {
            if self.pressed {
                let before = self.save.pet.wastes.len();
                let stage = self.stage;
                self.save
                    .pet
                    .wastes
                    .retain(|w| Vec2::new(w.x * stage.x, stage.y - 14.).distance(p) > 48.);
                if before != self.save.pet.wastes.len() {
                    self.notify("Waste cleaned up.");
                    self.persist();
                }
            }
            return;
        }
        if let Some(kind) = SUPPLIES.iter().position(|s| *s == item) {
            if !self.pressed {
                return;
            }
            let world_x = if let Some((low, high)) = self.save.pet.cage_bounds(self.stage) {
                p.x.clamp(low.x + 20., high.x - 20.)
            } else {
                p.x
            };
            let x = (world_x / self.stage.x).clamp(0.05, 0.95);
            if let Some(s) = self
                .save
                .pet
                .stations
                .iter_mut()
                .find(|s| s.kind == kind && (s.x - x).abs() * self.stage.x < 32.)
            {
                s.supply = 100.;
                self.notify("Care station refilled.");
            } else if self.save.pet.stations.len() < 16 {
                self.save.pet.stations.push(Station {
                    kind,
                    x,
                    supply: 100.,
                });
                self.notify(format!("{item} placed on the floor."));
            } else {
                self.notify("16 care stations maximum. Clear them in Pet settings.");
            }
            self.persist();
            // Give the response after the placement notice has expired.
            if self.save.pet.enabled && kind < 3 && self.save.pet.needs[kind] >= 70. {
                self.pet_refusal = Some(kind);
            }
            return;
        }
        if self.buddy.parts[5].pos.distance(p) > 110. {
            return;
        }
        if matches!(item, "Pet Soap" | "Pet Medicine") && !self.save.pet.enabled {
            return;
        }
        if self.save.pet.enabled {
            if item == "Pet Soap" {
                self.save.pet.hygiene = (self.save.pet.hygiene + 10. / 36.).min(100.);
            } else if item == "Pet Medicine" {
                self.save.pet.health = (self.save.pet.health + 5. / 36.).min(100.);
            } else if item == "Pet Brush" {
                self.save.pet.hygiene = (self.save.pet.hygiene + 0.8 / 36.).min(100.);
            }
            let needy = self.save.pet.needs[2] < 70.;
            self.save.pet.affection = (self.save.pet.affection + 0.3 / 36.).min(100.);
            if item == "Feather Wand" && needy {
                self.save.pet.needs[2] = (self.save.pet.needs[2] + 2. / 36.).min(85.);
            }
            if self.frame % 180 == 0 {
                self.say(if item == "Pet Brush" {
                    "That feels nice. I trust you."
                } else if needy {
                    "Catch the feather! Again!"
                } else {
                    "I'm all played out. Let's rest."
                });
            }
        }
        if self.frame % 8 == 0 {
            let col = if item == "Feather Wand" {
                Color::srgb(1., 0.7, 0.85)
            } else {
                Color::srgb(0.6, 1., 0.65)
            };
            self.fx.push(Fx {
                pos: p,
                vel: Vec2::new(0., -0.6),
                life: 0.6,
                max: 0.6,
                r: 4.,
                color: col,
            });
            if !self.save.pet.enabled {
                self.emotion = (self.emotion + 0.2).min(100.);
            }
        }
    }
    pub(crate) fn pet_cage_step(&mut self) {
        let Some((low, high)) = self.save.pet.cage_bounds(self.stage) else {
            return;
        };
        if self
            .buddy
            .parts
            .iter()
            .any(|p| p.radius * 2. > (high - low).min_element())
        {
            return;
        }
        let left = self
            .buddy
            .parts
            .iter()
            .map(|p| p.pos.x - p.radius)
            .fold(f32::INFINITY, f32::min);
        let right = self
            .buddy
            .parts
            .iter()
            .map(|p| p.pos.x + p.radius)
            .fold(f32::NEG_INFINITY, f32::max);
        let top = self
            .buddy
            .parts
            .iter()
            .map(|p| p.pos.y - p.radius)
            .fold(f32::INFINITY, f32::min);
        let bottom = self
            .buddy
            .parts
            .iter()
            .map(|p| p.pos.y + p.radius)
            .fold(f32::NEG_INFINITY, f32::max);
        let correction = Vec2::new(
            if left < low.x {
                low.x - left
            } else if right > high.x {
                high.x - right
            } else {
                0.
            },
            if top < low.y {
                low.y - top
            } else if bottom > high.y {
                high.y - bottom
            } else {
                0.
            },
        );
        for part in &mut self.buddy.parts {
            let mut velocity = part.pos - part.prev;
            let pos = part.pos + correction;
            let next = pos.clamp(
                low + Vec2::splat(part.radius),
                high - Vec2::splat(part.radius),
            );
            if correction.x != 0. || next.x != pos.x {
                velocity.x = 0.;
            }
            if correction.y != 0. || next.y != pos.y {
                velocity.y = 0.;
            }
            part.pos = next;
            part.prev = next - velocity;
        }
        for s in &mut self.save.pet.stations {
            s.x = (s.x * self.stage.x).clamp(low.x + 20., high.x - 20.) / self.stage.x;
        }
    }
    pub(crate) fn pet_step(&mut self) {
        if !self.save.pet.enabled {
            self.pet_target = None;
            self.pet_refusal = None;
            self.pet_status = "Resting".into();
            return;
        }
        if self.mode("freeze") {
            self.pet_status = "Frozen".into();
            return;
        }
        let x = self.buddy.parts[5].pos.x / self.stage.x.max(1.);
        let torso = self.buddy.parts[5].pos;
        let at_toilet = self.drag.is_none()
            && self.burn <= 0.
            && self.buddy.awake
            && self.save.pet.stations.iter().any(|s| {
                s.kind == 4
                    && s.supply > 0.
                    && (torso.x - s.x * self.stage.x).abs() < 28.
                    && (torso.y - (self.stage.y - 30.)).abs() < 65.
            });
        self.save.pet.care_tick(x, at_toilet);
        for (i, rate) in [0.15, 0.2, 0.25].into_iter().enumerate() {
            self.save.pet.needs[i] =
                (self.save.pet.needs[i] - rate * self.save.pet.decay / 36.).max(0.);
        }
        if self.save.pet.needs.iter().any(|n| *n < 20.) {
            self.save.pet.affection = (self.save.pet.affection - 0.08 / 36.).max(0.);
        }
        self.emotion = self.save.pet.affection * 2. - 100.;
        if self.save.pet.need_dialogue && self.frame % 180 == 0 {
            if let Some(message) = self.save.pet.need_message() {
                self.say(message);
            }
        }
        if let Some(kind) = self.pet_refusal {
            if kind >= 3
                || self.save.pet.needs[kind] < 70.
                || !self.save.dialogue
                || self.say(
                    [
                        "Thanks, but I'm full right now.",
                        "I'm not thirsty yet, thanks!",
                        "I've had enough play for now.",
                    ][kind],
                )
            {
                self.pet_refusal = None;
            }
        }
        self.pet_status = "Resting".into();
        if self.drag.is_some() || self.burn > 0. || !self.buddy.awake {
            return;
        }
        let torso = self.buddy.parts[5].pos;
        let center = self.stage * 0.5;
        let mouse = (self.mouse - center) / self.camera_scale + center - self.camera_offset;
        let hiding = self.save.pet.autonomous
            && self.save.pet.shy
            && self.save.pet.affection < 30.
            && torso.distance(mouse) < 180.;
        // Critical food/water or toilet care overrides shyness and play.
        let urgent =
            self.save.pet.needs[..2].iter().any(|n| *n < 20.) || self.save.pet.toilet > 80.;
        if hiding && !urgent {
            self.pet_status = "Hiding from you".into();
            self.pet_walk(if mouse.x < self.stage.x * 0.5 {
                self.stage.x - 55.
            } else {
                55.
            });
            if self.frame % 180 == 0 {
                self.say("I need some space. Please be gentle.");
            }
            return;
        }
        if let Some(i) = self.pet_target {
            if self.save.pet.stations.get(i).is_none_or(|s| {
                s.supply <= 0.
                    || self.save.pet.station_need(s.kind).is_none_or(|n| n >= 85.)
                    || (s.kind >= 2 && self.save.pet.needs[..2].iter().any(|n| *n < 20.))
                    || (s.kind == 2 && (self.save.pet.toilet > 80. || self.save.pet.energy < 20.))
                    || (s.kind == 3 && self.save.pet.toilet > 80.)
                    || (!self.save.pet.autonomous
                        && ((torso.x - s.x * self.stage.x).abs() >= 28.
                            || (torso.y - (self.stage.y - 30.)).abs() >= 65.))
            }) {
                self.pet_target = None;
            }
        }
        if self.pet_target.is_none() {
            self.pet_target = self
                .save
                .pet
                .stations
                .iter()
                .enumerate()
                .filter(|(_, s)| {
                    s.supply > 0.
                        && self.save.pet.station_need(s.kind).is_some_and(|n| n < 45.)
                        && (self.save.pet.autonomous
                            || ((torso.x - s.x * self.stage.x).abs() < 28.
                                && (torso.y - (self.stage.y - 30.)).abs() < 65.))
                })
                .min_by(|(_, a), (_, b)| {
                    // Critical hunger/thirst always wins; otherwise compare urgency.
                    let priority = |s: &Station| {
                        let n = self.save.pet.station_need(s.kind).unwrap_or(100.);
                        if s.kind < 2 && n < 20. { n - 100. } else { n }
                    };
                    priority(a).total_cmp(&priority(b)).then_with(|| {
                        (a.x * self.stage.x - torso.x)
                            .abs()
                            .total_cmp(&(b.x * self.stage.x - torso.x).abs())
                    })
                })
                .map(|(i, _)| i);
        }
        if let Some(i) = self.pet_target {
            let s = &self.save.pet.stations[i];
            let kind = s.kind;
            let x = s.x * self.stage.x;
            self.pet_status = [
                "Seeking food",
                "Seeking water",
                "Seeking toys",
                "Going to bed",
                "Seeking toilet",
            ][kind]
                .into();
            if (torso.x - x).abs() < 28. && (torso.y - (self.stage.y - 30.)).abs() < 65. {
                self.pet_status = [
                    "Eating",
                    "Drinking",
                    "Playing",
                    "Sleeping",
                    "Using litter tray",
                ][kind]
                    .into();
                match kind {
                    0..=2 => {
                        self.save.pet.needs[kind] = (self.save.pet.needs[kind] + 8. / 36.).min(85.)
                    }
                    3 => self.save.pet.energy = (self.save.pet.energy + 3. / 36.).min(85.),
                    4 => {
                        self.save.pet.toilet = (self.save.pet.toilet - 12. / 36.).max(0.);
                        self.save.pet.stations[i].supply =
                            (self.save.pet.stations[i].supply - 2. / 36.).max(0.);
                    }
                    _ => {}
                }
                self.save.pet.affection = (self.save.pet.affection + 0.6 / 36.).min(100.);
                if kind < 2 {
                    self.save.pet.stations[i].supply =
                        (self.save.pet.stations[i].supply - 4. / 36.).max(0.);
                }
                if self.frame % 180 == 0 {
                    self.say(
                        [
                            "You left this for me? Thank you!",
                            "Ahh, fresh water. You're the best.",
                            "This toy is fun! Let's play again.",
                            "A comfy bed. Time to recharge.",
                            "Much better. Thanks for the litter tray!",
                        ][kind],
                    );
                }
                if kind == 2 && self.frame % 90 == 0 {
                    self.pet_hop();
                }
                // Complete the visit now, before next frame's decay drops it below 85.
                if self.save.pet.station_need(kind).is_none_or(|n| n >= 85.) {
                    self.pet_target = None;
                }
            } else {
                self.pet_walk(x);
            }
            return;
        }
        if !self.save.pet.autonomous {
            return;
        }
        if self.save.pet.playful && self.save.pet.affection >= 70. {
            self.pet_status = "Playful".into();
            if mouse.x > 35. && mouse.x < self.stage.x - 35. {
                self.pet_walk(mouse.x);
            }
            if self.frame % 180 == 0 {
                self.pet_hop();
                self.say("Hey, friend! Want to play?");
            }
        } else if self.save.pet.sleep {
            if let Some(x) = self
                .save
                .pet
                .stations
                .iter()
                .find(|s| s.kind == 3 && s.supply > 0.)
                .map(|s| s.x * self.stage.x)
            {
                self.pet_status = "Going to bed".into();
                if (torso.x - x).abs() < 28. && (torso.y - (self.stage.y - 30.)).abs() < 65. {
                    self.pet_status = "Resting in bed".into();
                    self.save.pet.affection = (self.save.pet.affection + 0.08 / 36.).min(100.);
                    self.save.pet.energy = (self.save.pet.energy + 1. / 36.).min(100.);
                } else {
                    self.pet_walk(x);
                }
            }
        }
        self.emotion = self.save.pet.affection * 2. - 100.;
    }
    fn pet_walk(&mut self, x: f32) {
        // Floating modes still need a vertical approach to floor care stations.
        if self.mode("zerograv") || self.save.gravity <= 0. {
            let dy = self.stage.y - 50. - self.buddy.parts[5].pos.y;
            let gravity = if self.mode("zerograv") {
                0.
            } else {
                self.save.gravity.min(0.)
            };
            for p in &mut self.buddy.parts {
                p.prev.y = p.pos.y - (dy.clamp(-1.4, 1.4) - gravity);
            }
        }
        let delta = x - self.buddy.parts[5].pos.x;
        let dir = if delta.abs() < 12. {
            0.
        } else {
            delta.signum()
        };
        // Small fatigue/illness penalty; even an exhausted pet can reach supplies.
        let vigor = 0.8 + 0.1 * self.save.pet.energy / 100. + 0.1 * self.save.pet.health / 100.;
        for p in &mut self.buddy.parts {
            let v = ((p.pos.x - p.prev.x) * 0.85 + dir * 0.18 * vigor).clamp(-1.4, 1.4);
            p.prev.x = p.pos.x - v;
        }
        self.buddy.recover_upright();
    }
    fn pet_hop(&mut self) {
        if self.buddy.parts[3].pos.y > self.stage.y - 38.
            || self.buddy.parts[4].pos.y > self.stage.y - 38.
        {
            for p in &mut self.buddy.parts {
                p.prev.y += 3.;
            }
        }
    }
}
pub(crate) fn draw_world(d: &mut Painter<'_, '_, '_>, g: &Game) {
    if !g.save.pet.enabled {
        return;
    }
    if let Some((low, high)) = g.save.pet.cage_bounds(g.stage) {
        d.rect(
            low.x,
            low.y,
            high.x - low.x,
            high.y - low.y,
            Color::srgba(0.12, 0.2, 0.24, 0.15),
        );
        for i in 0..=8 {
            let x = low.x + (high.x - low.x) * i as f32 / 8.;
            d.line(
                Vec2::new(x, low.y),
                Vec2::new(x, high.y),
                1.5,
                Color::srgba(0.6, 0.75, 0.8, 0.65),
            );
        }
        d.line(
            low,
            Vec2::new(high.x, low.y),
            3.,
            Color::srgb(0.5, 0.7, 0.75),
        );
        d.line(
            Vec2::new(low.x, high.y),
            high,
            3.,
            Color::srgb(0.5, 0.7, 0.75),
        );
    }
    for s in &g.save.pet.stations {
        let p = Vec2::new(s.x * g.stage.x, g.stage.y - 18.);
        if let Some(h) = KEYS.get(s.kind).and_then(|key| d.art.tools.get(*key)) {
            d.image(p, Vec2::new(36., 25.), h.clone(), Color::WHITE);
        }
        if s.kind < 2 || s.kind == 4 {
            d.rect(p.x - 14., p.y + 14., 28., 3., ui::SURFACE);
            d.rect(p.x - 14., p.y + 14., 28. * s.supply / 100., 3., ui::SUCCESS);
        }
    }
    for w in &g.save.pet.wastes {
        let p = Vec2::new(w.x * g.stage.x, g.stage.y - 14.);
        if let Some(h) = d.art.tools.get("poop") {
            d.image(p, Vec2::new(24., 20.), h.clone(), Color::WHITE);
        } else {
            d.circle(p, 8., Color::srgb(0.4, 0.24, 0.12));
        }
    }
}
pub(crate) fn draw_status(d: &mut Painter<'_, '_, '_>, g: &Game) {
    if !g.save.pet.enabled
        || !g.save.pet.show_stats
        || !g.panel.is_empty()
        || !g.menu.is_empty()
        || (g.desktop && g.desktop_open && !g.desktop_section.is_empty())
    {
        return;
    }
    let x = 8.;
    let y = if g.desktop { 8. } else { 82. };
    d.rect(x, y, 152., 108., ui::SURFACE.with_alpha(0.92));
    for (i, (label, value)) in [
        ("Food", g.save.pet.needs[0]),
        ("Water", g.save.pet.needs[1]),
        ("Play", g.save.pet.needs[2]),
        ("Hygiene", g.save.pet.hygiene),
        ("Health", g.save.pet.health),
        ("Energy", g.save.pet.energy),
        ("Toilet", g.save.pet.toilet),
    ]
    .into_iter()
    .enumerate()
    {
        d.text(x + 5., y + 3. + i as f32 * 12., label, 9., ui::TEXT);
        d.rect(x + 42., y + 6. + i as f32 * 12., 66., 4., ui::RAISED);
        d.rect(
            x + 42.,
            y + 6. + i as f32 * 12.,
            66. * value / 100.,
            4.,
            if (i == 6 && value > 70.) || (i != 6 && value < 30.) {
                Color::srgb(1., 0.6, 0.3)
            } else {
                ui::SUCCESS
            },
        );
        d.text(
            x + 114.,
            y + 2. + i as f32 * 12.,
            format!("{value:.0}"),
            9.,
            ui::TEXT,
        );
    }
    d.text(
        x + 5.,
        y + 88.,
        format!("{} · Trust {:.0}", g.pet_status, g.save.pet.affection),
        9.,
        ui::MUTED,
    );
}
pub(crate) fn draw_panel(d: &mut Painter<'_, '_, '_>, g: &Game) {
    let pet = &g.save.pet;
    d.button(
        98.,
        96.,
        350.,
        23.,
        if pet.enabled {
            "Passive Pet: On"
        } else {
            "Passive Pet: Off (optional)"
        },
        Action::Pet("toggle".into()),
    );
    for (i, (label, n)) in [
        ("Food", pet.needs[0]),
        ("Water", pet.needs[1]),
        ("Play", pet.needs[2]),
        ("Trust", pet.affection),
        ("Hygiene", pet.hygiene),
        ("Health", pet.health),
        ("Energy", pet.energy),
        ("Toilet urgency", pet.toilet),
    ]
    .into_iter()
    .enumerate()
    {
        let x = 100. + (i / 4) as f32 * 178.;
        let y = 125. + (i % 4) as f32 * 12.;
        d.text(x, y, format!("{label}: {n:.0}%"), 10., ui::TEXT);
        d.rect(x + 114., y + 4., 48., 4., ui::RAISED);
        d.rect(x + 114., y + 4., 48. * n / 100., 4., ui::SUCCESS);
    }
    for (i, (label, key)) in [
        (format!("Needs decay: {}x", pet.decay), "decay"),
        (
            format!(
                "Autonomous care: {}",
                if pet.autonomous { "On" } else { "Off" }
            ),
            "auto",
        ),
        (
            format!("Shy when upset: {}", if pet.shy { "On" } else { "Off" }),
            "shy",
        ),
        (
            format!(
                "Playful when happy: {}",
                if pet.playful { "On" } else { "Off" }
            ),
            "playful",
        ),
        (
            format!("Stat panel: {}", if pet.show_stats { "On" } else { "Off" }),
            "stats",
        ),
        (
            format!(
                "Poop accidents: {}",
                if pet.accidents { "On" } else { "Off" }
            ),
            "accidents",
        ),
        (
            format!("Sleep in bed: {}", if pet.sleep { "On" } else { "Off" }),
            "sleep",
        ),
        ("Reset care / trust".into(), "reset"),
        (
            format!(
                "Need dialogue: {}",
                if pet.need_dialogue { "On" } else { "Off" }
            ),
            "need_dialogue",
        ),
        (
            format!(
                "Need lights: {}",
                if pet.need_lights { "On" } else { "Off" }
            ),
            "need_lights",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        d.button(
            98. + (i % 2) as f32 * 178.,
            180. + (i / 2) as f32 * 23.,
            172.,
            21.,
            &label,
            Action::Pet(key.into()),
        );
    }
    d.button(
        98.,
        298.,
        172.,
        22.,
        "Clear care stations",
        Action::Pet("clear".into()),
    );
    d.button(
        276.,
        298.,
        172.,
        22.,
        if pet.cage.is_some() {
            "Release from cage"
        } else {
            "No cage placed"
        },
        Action::Pet("release_cage".into()),
    );
    d.text(98., 324., "Care tools: Items > Pet Items. Lights: F/W/P/Hyg/HP/E/T.\nNeeds use simulation time; no offline neglect.", 10., ui::MUTED);
}
#[cfg(test)]
mod tests {
    use super::*;
    fn game() -> Game {
        let mut g = Game::new();
        g.demo = true;
        g.save.pet = Pet::default();
        g.stage = Vec2::new(W, H);
        g.save.pet.enabled = true;
        g.reset_buddy();
        g
    }
    fn floor_station(g: &mut Game, kind: usize) {
        let shift = Vec2::new(0., g.stage.y - 50. - g.buddy.parts[5].pos.y);
        for p in &mut g.buddy.parts {
            p.pos += shift;
            p.prev += shift;
        }
        g.save.pet.stations.push(Station {
            kind,
            x: g.buddy.parts[5].pos.x / g.stage.x,
            supply: 100.,
        });
    }
    #[test]
    fn old_saves_default_new_care_and_validate_boundaries() {
        let mut p: Pet = serde_json::from_str(r#"{"enabled":true,"needs":[12,34,56]}"#).unwrap();
        assert_eq!(p.needs, [12., 34., 56.]);
        assert_eq!(
            (p.hygiene, p.health, p.energy, p.toilet),
            (90., 100., 85., 0.)
        );
        assert!(p.accidents && p.sleep);
        p.hygiene = f32::NAN;
        p.health = -100.;
        p.energy = f32::INFINITY;
        p.toilet = 999.;
        p.needs[0] = f32::NAN;
        p.wastes = (0..20).map(|_| Waste { x: 3. }).collect();
        p.wastes.push(Waste { x: f32::NAN });
        p.stations = vec![
            Station {
                kind: 4,
                x: 0.5,
                supply: 100.,
            },
            Station {
                kind: 5,
                x: 0.5,
                supply: 100.,
            },
        ];
        p.validate();
        assert_eq!(
            (p.hygiene, p.health, p.energy, p.toilet),
            (90., 25., 85., 100.)
        );
        assert_eq!(p.needs[0], 75.);
        assert_eq!(p.wastes.len(), 16);
        assert!(p.wastes.iter().all(|w| w.x == 0.95));
        assert_eq!(p.stations.len(), 1);
        let round: Pet = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        assert_eq!(round.stations[0].kind, 4);
        assert_eq!(round.wastes.len(), 16);
        assert_eq!(round.toilet, 100.);
    }
    #[test]
    fn disabled_and_frozen_simulation_has_no_care_penalty() {
        let mut g = game();
        g.save.pet.toilet = 100.;
        g.save.pet.health = 30.;
        g.save.pet.wastes.push(Waste { x: 0.5 });
        g.save.pet.enabled = false;
        let before = serde_json::to_string(&g.save.pet).unwrap();
        for _ in 0..100 {
            g.pet_step();
        }
        g.pet_damage(20.);
        assert_eq!(before, serde_json::to_string(&g.save.pet).unwrap());
        g.save.pet.enabled = true;
        g.save.modes.insert("freeze".into());
        let before = serde_json::to_string(&g.save.pet).unwrap();
        for _ in 0..100 {
            g.pet_step();
        }
        assert_eq!(before, serde_json::to_string(&g.save.pet).unwrap());
    }
    #[test]
    fn accidents_are_bounded_and_scoop_cleans_only_nearby() {
        let mut g = game();
        g.save.pet.toilet = 100.;
        g.pet_step();
        assert_eq!(g.save.pet.wastes.len(), 1);
        assert!(g.save.pet.toilet < 1.);
        g.save.pet.toilet = 100.;
        for _ in 0..36 {
            g.pet_step();
        }
        assert_eq!(
            g.save.pet.wastes.len(),
            1,
            "cooldown must block repeated accidents"
        );
        g.save.pet.wastes = vec![Waste { x: 0.1 }, Waste { x: 0.9 }];
        let p = Vec2::new(g.stage.x * 0.1, g.stage.y - 14.);
        g.pressed = false;
        g.pet_tool("Poop Scoop", p);
        assert_eq!(g.save.pet.wastes.len(), 2);
        g.pressed = true;
        g.pet_tool("Poop Scoop", p);
        assert_eq!(g.save.pet.wastes.len(), 1);
        assert_eq!(g.save.pet.wastes[0].x, 0.9);
        g.save.pet.wastes = (0..16).map(|_| Waste { x: 0.5 }).collect();
        g.save.pet.accident_cooldown = 0.;
        g.save.pet.toilet = 100.;
        g.pet_step();
        assert_eq!(g.save.pet.wastes.len(), 16);
        g.save.pet.accidents = false;
        g.save.pet.wastes.clear();
        g.save.pet.toilet = 100.;
        g.save.pet.accident_cooldown = 0.;
        g.pet_step();
        assert!(g.save.pet.wastes.is_empty());
        assert_eq!(g.save.pet.toilet, 100.);
    }
    #[test]
    fn cage_saves_confines_physics_and_releases_buddy() {
        let mut g = game();
        g.save.pet.enabled = true;
        g.pressed = true;
        floor_station(&mut g, 0);
        g.pet_tool("Pet Cage", Vec2::new(100., 200.));
        assert!(g.save.pet.cage.is_some());
        let (low, high) = g.save.pet.cage_bounds(g.stage).unwrap();
        assert!(
            g.save
                .pet
                .stations
                .iter()
                .all(|s| s.x * g.stage.x >= low.x + 20. && s.x * g.stage.x <= high.x - 20.)
        );
        g.pet_tool("Water Bowl", Vec2::new(g.stage.x, 100.));
        assert!(
            g.save
                .pet
                .stations
                .iter()
                .all(|s| s.x * g.stage.x <= high.x - 20.)
        );
        let saved: Pet =
            serde_json::from_str(&serde_json::to_string(&g.save.pet).unwrap()).unwrap();
        assert_eq!(saved.cage, g.save.pet.cage);
        g.pressed = false;
        g.save.pet.needs = [5., 5., 90.];
        g.buddy.parts[5].prev -= Vec2::new(200., -200.);
        let mut app = App::new();
        app.insert_resource(g).add_systems(Update, crate::simulate);
        for _ in 0..720 {
            app.update();
            let g = app.world().resource::<Game>();
            for p in &g.buddy.parts {
                assert!(
                    p.pos.x - p.radius >= low.x - 0.001
                        && p.pos.x + p.radius <= high.x + 0.001
                        && p.pos.y - p.radius >= low.y - 0.001
                        && p.pos.y + p.radius <= high.y + 0.001
                );
            }
        }
        let mut g = app.world_mut().resource_mut::<Game>();
        assert!(g.save.pet.needs[0] > 5. && g.save.pet.needs[1] > 5.);
        g.save.modes.insert("gravityCarousel".into());
        let entrance = g.buddy.parts[5].pos;
        g.pressed = true;
        g.advanced_tool("Portal Gun", entrance);
        g.advanced_tool("Portal Gun", Vec2::new(450., 100.));
        g.pressed = false;
        g.down = true;
        g.drag = Some(0);
        g.mouse = Vec2::new(500., 30.);
        drop(g);
        for _ in 0..90 {
            app.update();
            let g = app.world().resource::<Game>();
            for p in &g.buddy.parts {
                assert!(
                    p.pos.x - p.radius >= low.x - 0.001
                        && p.pos.x + p.radius <= high.x + 0.001
                        && p.pos.y - p.radius >= low.y - 0.001
                        && p.pos.y + p.radius <= high.y + 0.001
                );
            }
        }
        let mut g = app.world_mut().resource_mut::<Game>();
        g.code = "setBuddyPos(450,335)".into();
        g.every_frame = true;
        g.drag = None;
        g.down = false;
        g.save.modes.clear();
        g.run_script();
        for p in &g.buddy.parts {
            assert!(p.pos.x - p.radius >= low.x - 0.001 && p.pos.x + p.radius <= high.x + 0.001);
        }
        drop(g);
        for _ in 0..36 {
            app.update();
            let g = app.world().resource::<Game>();
            for p in &g.buddy.parts {
                assert!(
                    p.pos.x - p.radius >= low.x - 0.001
                        && p.pos.x + p.radius <= high.x + 0.001
                        && p.pos.y - p.radius >= low.y - 0.001
                        && p.pos.y + p.radius <= high.y + 0.001
                );
            }
        }
        let mut g = app.world_mut().resource_mut::<Game>();
        g.pet_action("release_cage");
        assert!(g.save.pet.cage.is_none());
        let old = g.buddy.parts[5].pos;
        g.buddy.parts[5].pos.x = 500.;
        g.pet_cage_step();
        assert_eq!(g.buddy.parts[5].pos.x, 500.);
        assert_ne!(old.x, 500.);
    }
    #[test]
    fn need_cues_replace_stats_and_respect_settings() {
        let mut pet = Pet::default();
        assert!(!pet.show_stats);
        assert!(pet.need_lights && pet.need_dialogue);
        assert!(pet.need_message().is_none());
        pet.needs[1] = 5.;
        assert_eq!(pet.need_message(), Some(NEED_MESSAGES[1]));
        pet.needs = [90.; 3];
        pet.toilet = 95.;
        assert_eq!(pet.need_message(), Some(NEED_MESSAGES[6]));
        let old: Pet = serde_json::from_str(r#"{"show_stats":true}"#).unwrap();
        assert!(!old.show_stats);
        pet.show_stats = true;
        let round: Pet = serde_json::from_str(&serde_json::to_string(&pet).unwrap()).unwrap();
        assert!(round.show_stats);
        let mut g = game();
        g.save.pet.enabled = true;
        g.save.pet.needs = [5., 90., 90.];
        g.frame = 180;
        g.pet_step();
        assert_eq!(g.message, NEED_MESSAGES[0]);
        g.message_time = 0.;
        g.dialogue_after = 0;
        g.save.pet.need_dialogue = false;
        g.pet_step();
        assert_eq!(g.message_time, 0.);
        g.save.pet.need_dialogue = true;
        g.save.dialogue = false;
        g.pet_step();
        assert_eq!(g.message_time, 0.);
    }
    #[test]
    fn litter_and_bed_work_but_never_starve_critical_food() {
        let mut g = game();
        floor_station(&mut g, 4);
        g.save.pet.toilet = 100.;
        g.pet_step();
        assert_eq!(g.pet_status, "Using litter tray");
        assert!(g.save.pet.toilet < 100.);
        assert!(
            g.save.pet.wastes.is_empty(),
            "a usable nearby tray prevents accidents"
        );
        assert!(g.save.pet.stations[0].supply < 100.);
        floor_station(&mut g, 0);
        g.save.pet.needs[0] = 5.;
        g.pet_step();
        assert_eq!(g.pet_status, "Eating");
        assert!(g.save.pet.needs[0] > 5.);
        g.save.pet.needs = [90.; 3];
        g.save.pet.toilet = 0.;
        g.save.pet.energy = 10.;
        floor_station(&mut g, 3);
        g.pet_step();
        assert_eq!(g.pet_status, "Sleeping");
        assert!(g.save.pet.energy > 10.);
        g.save.pet.sleep = false;
        g.pet_step();
        assert_ne!(g.pet_status, "Sleeping");
        g.save.pet.sleep = true;
        g.save.pet.autonomous = false;
        g.save.pet.energy = 10.;
        g.pet_step();
        assert_eq!(g.pet_status, "Sleeping");
    }
    #[test]
    fn damage_updates_emotion_immediately_and_care_reverses_illness() {
        let mut g = game();
        let trust = g.save.pet.affection;
        g.pet_damage(0.);
        g.pet_damage(f32::NAN);
        assert_eq!(g.save.pet.affection, trust);
        g.pet_damage(0.01);
        assert!(g.save.pet.affection < trust);
        assert_eq!(g.emotion, g.save.pet.affection * 2. - 100.);
        g.save.pet.hygiene = 0.;
        g.save.pet.health = 25.;
        g.save.pet.energy = 0.;
        for _ in 0..3600 {
            g.save.pet.care_tick(0.5, false);
        }
        assert_eq!(g.save.pet.health, 25.);
        assert_eq!(g.save.pet.energy, 0.);
        let p = g.buddy.parts[5].pos;
        for _ in 0..360 {
            g.pet_tool("Pet Soap", p);
            g.pet_tool("Pet Medicine", p);
        }
        assert!(g.save.pet.hygiene > 90.);
        assert!(g.save.pet.health > 70.);
        g.save.pet.enabled = false;
        let before = (g.save.pet.hygiene, g.save.pet.health);
        g.pet_tool("Pet Soap", p);
        g.pet_tool("Pet Medicine", p);
        assert_eq!(before, (g.save.pet.hygiene, g.save.pet.health));
    }
    #[test]
    fn optional_needs_and_affection_reflect_care() {
        let mut g = game();
        g.save.pet.enabled = false;
        let needs = g.save.pet.needs;
        g.pet_step();
        assert_eq!(needs, g.save.pet.needs);
        g.save.pet.enabled = true;
        g.save.pet.needs[0] = 20.;
        let shift = Vec2::new(0., g.stage.y - 50. - g.buddy.parts[5].pos.y);
        for p in &mut g.buddy.parts {
            p.pos += shift;
            p.prev += shift;
        }
        let x = g.buddy.parts[5].pos.x / g.stage.x;
        g.save.pet.stations.push(Station {
            kind: 0,
            x,
            supply: 100.,
        });
        let trust = g.save.pet.affection;
        g.pet_step();
        assert!(g.save.pet.needs[0] > 20.);
        assert!(g.save.pet.affection > trust);
        assert!(g.save.pet.stations[0].supply < 100.);
        assert_eq!(g.pet_status, "Eating");
    }
    #[test]
    fn satisfied_pet_refuses_and_needy_pet_seeks() {
        let mut g = game();
        g.save.pet.needs = [95.; 3];
        g.save.pet.stations.push(Station {
            kind: 1,
            x: 0.1,
            supply: 100.,
        });
        g.pet_step();
        assert!(g.pet_target.is_none());
        g.save.pet.needs[1] = 20.;
        g.pet_step();
        assert_eq!(g.pet_target, Some(0));
        assert_eq!(g.pet_status, "Seeking water");
        assert!(g.buddy.parts[5].prev.x > g.buddy.parts[5].pos.x);
        g.save.pet.needs[1] = 90.;
        g.pet_step();
        assert!(g.pet_target.is_none());
    }
    #[test]
    fn low_trust_hides_high_trust_plays() {
        let mut g = game();
        g.mouse = g.buddy.parts[5].pos;
        g.save.pet.affection = 10.;
        g.pet_step();
        assert_eq!(g.pet_status, "Hiding from you");
        g.save.pet.affection = 90.;
        g.pet_step();
        assert_eq!(g.pet_status, "Playful");
    }
    #[test]
    fn real_physics_reaches_and_uses_all_care_stations() {
        let mut g = game();
        g.save.pet.needs = [20., 25., 30.];
        for (kind, x) in [0.15, 0.85, 0.5].into_iter().enumerate() {
            g.save.pet.stations.push(Station {
                kind,
                x,
                supply: 100.,
            });
        }
        let mut app = App::new();
        app.insert_resource(g).add_systems(Update, simulate);
        for _ in 0..2400 {
            app.update();
        }
        let g = app.world().resource::<Game>();
        assert!(
            g.save.pet.needs.iter().all(|n| *n > 60.),
            "needs {:?}, status {}, torso {:?}",
            g.save.pet.needs,
            g.pet_status,
            g.buddy.parts[5].pos
        );
        assert!(g.buddy.parts.iter().all(|p| p.pos.is_finite()));
    }
    #[test]
    fn real_physics_reaches_litter_then_bed() {
        let mut g = game();
        g.save.pet.needs = [90.; 3];
        g.save.pet.toilet = 90.;
        g.save.pet.energy = 20.;
        g.save.pet.stations = vec![
            Station {
                kind: 4,
                x: 0.15,
                supply: 100.,
            },
            Station {
                kind: 3,
                x: 0.85,
                supply: 100.,
            },
        ];
        let mut app = App::new();
        app.insert_resource(g).add_systems(Update, simulate);
        for _ in 0..2400 {
            app.update();
        }
        let g = app.world().resource::<Game>();
        assert!(
            g.save.pet.toilet < 45.,
            "toilet {}, status {}",
            g.save.pet.toilet,
            g.pet_status
        );
        assert!(
            g.save.pet.energy > 60.,
            "energy {}, status {}",
            g.save.pet.energy,
            g.pet_status
        );
        assert!(g.save.pet.stations[0].supply < 100.);
        assert!(g.save.pet.wastes.is_empty());
        assert!(g.buddy.parts.iter().all(|p| p.pos.is_finite()));
    }
    #[test]
    fn nearby_care_works_with_autonomy_off_and_refusals_retry() {
        let mut g = game();
        g.save.pet.autonomous = false;
        g.save.pet.needs[0] = 20.;
        let shift = Vec2::new(0., g.stage.y - 50. - g.buddy.parts[5].pos.y);
        for p in &mut g.buddy.parts {
            p.pos += shift;
            p.prev += shift;
        }
        g.save.pet.stations.push(Station {
            kind: 0,
            x: g.buddy.parts[5].pos.x / g.stage.x,
            supply: 100.,
        });
        g.pet_step();
        assert!(g.save.pet.needs[0] > 20.);
        g.save.pet.needs[0] = 90.;
        g.pet_refusal = Some(0);
        g.message_time = 0.;
        g.dialogue_after = 0;
        g.warn_time = 2.;
        g.pet_step();
        assert_eq!(g.pet_refusal, Some(0));
        g.warn_time = 0.;
        g.pet_step();
        assert!(g.pet_refusal.is_none());
        assert!(g.message.contains("full"));
        g.pet_refusal = Some(0);
        g.save.pet.needs[0] = 20.;
        g.pet_step();
        assert!(g.pet_refusal.is_none());
    }
    #[test]
    fn floating_modes_can_reach_floor_care() {
        for gravity in [0., -1.] {
            let mut g = game();
            g.save.gravity = gravity;
            g.save.pet.needs = [20., 90., 90.];
            g.save.pet.stations.push(Station {
                kind: 0,
                x: 0.25,
                supply: 100.,
            });
            if gravity == 0. {
                g.action(Action::Select("Modes".into(), "Zero Gravity".into()));
                assert!(g.mode("zerograv"));
            }
            let mut app = App::new();
            app.insert_resource(g).add_systems(Update, simulate);
            for _ in 0..1800 {
                app.update();
            }
            let g = app.world().resource::<Game>();
            assert!(
                g.save.pet.needs[0] > 70.,
                "gravity {gravity}, needs {:?}, torso {:?}",
                g.save.pet.needs,
                g.buddy.parts[5].pos
            );
        }
    }
    #[test]
    fn stations_refill_and_limits_hold() {
        let mut g = game();
        g.pressed = true;
        let p = Vec2::new(100., 100.);
        g.pet_tool("Food Bowl", p);
        g.save.pet.stations[0].supply = 0.;
        g.pet_tool("Food Bowl", p);
        assert_eq!(g.save.pet.stations.len(), 1);
        assert_eq!(g.save.pet.stations[0].supply, 100.);
        for i in 0..50 {
            g.pet_tool(
                SUPPLIES[i % SUPPLIES.len()],
                Vec2::new((i as f32 * 33.) % W, 100.),
            );
        }
        assert!(g.save.pet.stations.len() <= 16);
        let round: Pet =
            serde_json::from_str(&serde_json::to_string(&g.save.pet).unwrap()).unwrap();
        assert_eq!(round.stations.len(), g.save.pet.stations.len());
    }
}
