//! Companion content and optional, local-only contextual chatter.
use crate::*;

pub(crate) fn notice_timers(mut g: ResMut<Game>, time: Res<Time<Real>>) {
    let dt = time.delta_secs();
    g.message_time = (g.message_time - dt).max(0.);
    g.warn_time = (g.warn_time - dt).max(0.);
}

/// Optional rendering probe; use with --classic --playtest. Never runs in normal play.
pub(crate) fn smoke(
    mut commands: Commands,
    mut g: ResMut<Game>,
    mut frame: Local<u32>,
    mut exit: MessageWriter<AppExit>,
) {
    if !std::env::args().any(|arg| arg == "--content-smoke-test") {
        return;
    }
    *frame += 1;
    let step = (*frame / 60).min(5);
    if *frame % 60 == 30 {
        g.buddy = Buddy::new();
        g.burn = 0.;
        g.unconscious = 0;
        g.save.skin = ["ice", "neon", "ember", "neon", "ice", "ember"][step as usize].into();
        g.save.modes = ["glitter".into()].into_iter().collect();
        g.objects.clear();
        g.paused = false;
        g.panel.clear();
        g.menu.clear();
        g.message_time = 0.;
        g.notify(
            [
                "Bubbles! Finally, something gentle.",
                "Oh, you're going to Discord!",
                "Wheee! Careful where you put me!",
            ][step.min(2) as usize],
        );
        if step <= 2 {
            for (kind, x) in [
                ("Tomatoes", 120.),
                ("Basketballs", 205.),
                ("Rubber Ducks", 290.),
                ("Baseballs", 375.),
            ] {
                g.objects.push(Object {
                    pos: Vec2::new(x, 345.),
                    prev: Vec2::new(x, 345.),
                    r: objects::properties(kind).radius,
                    kind: kind.into(),
                    life: f32::INFINITY,
                    held: false,
                    hit_timer: 0,
                    window_hit_timer: 0,
                });
            }
        }
        g.save.item = [
            "Ice Wand",
            "Sparkler",
            "Machine Gun",
            "Rubber Ducks",
            "Ice Wand",
            "Sparkler",
        ][step as usize]
            .into();
        g.mouse = Vec2::new(380., 160.);
        if step == 3 {
            g.save.favorites.insert("Ice Wand".into());
            g.save.quick_slots = [
                Some("Ice Wand".into()),
                Some("Rubber Ducks".into()),
                Some("Sparkler".into()),
            ];
            g.preset_name = "Neon playtime".into();
            g.action(Action::SavePreset);
            g.panel = "Loadout".into();
        } else if step == 4 {
            g.panel = "Physics".into();
            g.action(Action::Step);
        } else if step == 5 {
            g.panel = "Dialogue".into();
        }
        if step < 2 {
            let item = g.save.item.clone();
            let pos = g.buddy.parts[5].pos;
            g.content_tool(&item, pos);
        }
    }
    if *frame % 60 == 50 {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("screenshots");
        std::fs::create_dir_all(&root).unwrap();
        commands
            .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
            .observe(bevy::render::view::screenshot::save_to_disk(
                root.join(format!("content-{step}.png")),
            ));
    }
    if *frame == 360 {
        exit.write(AppExit::Success);
    }
}

pub(crate) fn tool_art(item: &str) -> Option<&'static str> {
    match item {
        "Tractor Beam" => Some("tractorbeam"),
        "Launch Pad" => Some("launchpad"),
        "Pet Cage" => Some("petcage"),
        "Portal Gun" => Some("portalgun"),
        "Grappling Hook" => Some("grapple"),
        "Stasis Field" => Some("stasis"),
        "Litter Tray" => Some("littertray"),
        "Poop Scoop" => Some("poopscoop"),
        "Pet Soap" => Some("petsoap"),
        "Pet Medicine" => Some("petmedicine"),
        "Food Bowl" => Some("foodbowl"),
        "Water Bowl" => Some("waterbowl"),
        "Toy Basket" => Some("toybasket"),
        "Pet Bed" => Some("petbed"),
        "Feather Wand" => Some("featherwand"),
        "Pet Brush" => Some("petbrush"),
        "Beach Balls" => Some("beachball"),
        "Anvils" => Some("anvil"),
        "Bubble Wand" => Some("bubblewand"),
        "Repulsor" => Some("repulsor"),
        "Confetti Cannon" => Some("confetti"),
        "Ice Wand" => Some("icewand"),
        "Sparkler" => Some("sparkler"),
        "Rubber Ducks" => Some("duck"),
        _ => None,
    }
}

impl Game {
    pub(crate) fn say(&mut self, text: impl Into<String>) -> bool {
        // Chatter must never overwrite purchases, warnings, or script errors.
        if self.save.dialogue
            && self.frame >= self.dialogue_after
            && self.message_time <= 0.
            && self.warn_time <= 0.
            && self.save_error.is_none()
        {
            self.notify(text);
            self.dialogue_after =
                self.frame + 36 * u64::from(self.save.dialogue_interval.clamp(8, 60));
            return true;
        }
        false
    }

    fn window_comment(&mut self, app: String) {
        if app == self.window_candidate {
            if app != self.window_app && self.say(format!("Oh, you're going to {app}!")) {
                self.window_app = app;
            }
        } else {
            self.window_candidate = app;
        }
    }

    pub(crate) fn content_tool(&mut self, item: &str, p: Vec2) -> bool {
        match item {
            "Repulsor" => {
                let body = self.buddy.parts[5].pos;
                let distance = body.distance(p);
                if distance < 240. {
                    let push = (body - p).normalize_or_zero() * (1. - distance / 240.) * 1.8;
                    for part in &mut self.buddy.parts {
                        part.prev -= push;
                    }
                }
                for object in &mut self.objects {
                    if !object.held {
                        let d = object.pos - p;
                        object.prev -=
                            d.normalize_or_zero() * (1. - d.length() / 240.).max(0.) * 2.;
                    }
                }
                if self.frame % 4 == 0 {
                    self.spray(p, Color::srgb(1., 0.65, 0.3), 8, 2., 0.5);
                }
                self.say("Whoa! Personal space, please!");
            }
            "Bubble Wand" => {
                if self.frame % 6 == 0 {
                    let direction = self.direction;
                    self.fx.push(Fx {
                        pos: p,
                        vel: direction * 2. + Vec2::new(0., -1.),
                        life: 1.8,
                        max: 1.8,
                        r: 5.,
                        color: Color::srgba(0.55, 0.9, 1., 0.65),
                    });
                    if self.buddy.parts[5].pos.distance(p) < 140. {
                        self.emotion = (self.emotion + 0.7).min(100.);
                        self.burn = (self.burn - 0.4).max(0.);
                    }
                }
                self.say("Bubbles! Finally, something gentle.");
            }
            "Ice Wand" => {
                if self.buddy.parts[5].pos.distance(p) < 140. {
                    self.burn = (self.burn - 0.3).max(0.);
                    for part in &mut self.buddy.parts {
                        if part.pos.distance(p) < 140. {
                            part.prev = part.pos - (part.pos - part.prev) * 0.97;
                        }
                    }
                    if self.frame % 6 == 0 {
                        self.emotion = (self.emotion + 0.25).min(100.);
                    }
                }
                if self.frame % 6 == 0 {
                    self.spray(p, Color::srgb(0.55, 0.85, 1.), 4, 1.2, 0.6);
                }
                self.say("Ahh, that's refreshingly cool!");
            }
            "Sparkler" => {
                if self.frame % 6 == 0 {
                    self.spray(p, Color::srgb(1., 0.75, 0.3), 5, 1.6, 0.45);
                    if self.buddy.parts[5].pos.distance(p) < 140. {
                        self.emotion = (self.emotion + 0.1).min(100.);
                    }
                }
                self.say("Pretty lights! All sparkle, no scorch.");
            }
            "Confetti Cannon" => {
                if self.pressed || self.cooldown <= 0 {
                    self.confetti(p);
                    self.cooldown = 18;
                    self.emotion = (self.emotion + 1.).min(100.);
                    self.say("Is it my birthday already?");
                }
            }
            _ => return false,
        }
        true
    }

    fn spray(&mut self, p: Vec2, color: Color, count: usize, speed: f32, life: f32) {
        for i in 0..count {
            let a = i as f32 * std::f32::consts::TAU / count as f32;
            self.fx.push(Fx {
                pos: p,
                vel: Vec2::new(a.cos(), a.sin()) * speed,
                life,
                max: life,
                r: 2.5,
                color,
            });
        }
    }

    fn confetti(&mut self, p: Vec2) {
        for i in 0..18 {
            let color = match i % 3 {
                0 => Color::srgb(1., 0.45, 0.65),
                1 => Color::srgb(0.4, 0.9, 0.85),
                _ => Color::srgb(1., 0.85, 0.3),
            };
            let vel = Vec2::new((self.random() - 0.5) * 9., -self.random() * 5. - 1.);
            self.fx.push(Fx {
                pos: p,
                vel,
                life: 1.2,
                max: 1.2,
                r: 2.5,
                color,
            });
        }
    }

    pub(crate) fn party_burst(&mut self) {
        let head = self.buddy.parts[0].pos;
        for i in 0..30 {
            let angle = i as f32 * std::f32::consts::TAU / 30.;
            let speed = 1.5 + (i % 4) as f32 * 0.6;
            self.fx.push(Fx {
                pos: head + Vec2::new((i as f32 - 15.) * 3., -20.),
                vel: Vec2::new(angle.cos() * speed, angle.sin() * speed - 1.),
                life: 1.8,
                max: 1.8,
                r: 5. + (i % 3) as f32,
                color: [
                    Color::srgb(1., 0.3, 0.5),
                    Color::srgb(1., 0.85, 0.2),
                    Color::srgb(0.3, 0.9, 1.),
                    Color::srgb(0.6, 1., 0.3),
                ][i % 4],
            });
        }
    }
    pub(crate) fn content_step(&mut self) {
        if self.mode("bubbleShower") && self.frame % 6 == 0 {
            let p = self.buddy.parts[0].pos + Vec2::new((self.random() - 0.5) * 140., 35.);
            self.fx.push(Fx {
                pos: p,
                vel: Vec2::new((self.frame as f32 * 0.1).sin() * 0.5, -1.5),
                life: 2.,
                max: 2.,
                r: 5.,
                color: Color::srgb(0.5, 0.85, 1.),
            });
        }
        if self.mode("toyOrbit") {
            let center = self.buddy.parts[5].pos;
            let frame = self.frame;
            for (i, o) in self.objects.iter_mut().enumerate() {
                if !o.held
                    && matches!(
                        o.kind.as_str(),
                        "Baseballs" | "Rubber Balls" | "Beach Balls" | "Rubber Ducks" | "Snowballs"
                    )
                {
                    let a = frame as f32 * 0.025 + i as f32 * 2.;
                    let target = center + Vec2::new(a.cos(), a.sin()) * 85.;
                    o.prev = o.pos - (target - o.pos).clamp_length_max(2.4);
                }
            }
        }
        if self.mode("wind") {
            let force = 0.12 + (self.frame as f32 * 0.015).sin() * 0.08;
            for part in &mut self.buddy.parts {
                part.prev.x -= force;
            }
            for object in &mut self.objects {
                if !object.held {
                    object.prev.x -= force / objects::properties(&object.kind).mass.max(0.1);
                }
            }
            if self.frame % 12 == 0 {
                let y = self.random() * self.stage.y;
                self.fx.push(Fx {
                    pos: Vec2::new(8., y),
                    vel: Vec2::new(7., 0.),
                    life: 1.8,
                    max: 1.8,
                    r: 1.5,
                    color: Color::srgb(0.7, 0.85, 1.),
                });
            }
        }
        if self.mode("magnet") {
            // Match the pointer's inverse camera transform used by tools/rendering.
            let center = self.stage * 0.5;
            let target = (self.mouse - center) / self.camera_scale + center - self.camera_offset;
            // Only loose objects follow the cursor; Buddy remains freely draggable.
            for object in &mut self.objects {
                if !object.held {
                    let d = target - object.pos;
                    object.prev -= d.normalize_or_zero() * (d.length() / 150.).min(0.8);
                }
            }
        }
        if self.mode("glitter") && self.frame % 3 == 0 {
            let head = &self.buddy.parts[0];
            if head.pos.distance_squared(head.prev) > 0.25 {
                let p = head.pos;
                let color = [
                    Color::srgb(0.1, 0.9, 1.),
                    Color::srgb(1., 0.2, 0.7),
                    Color::srgb(1., 0.85, 0.2),
                ][(self.frame / 3 % 3) as usize];
                self.spray(p, color, 2, 0.7, 0.8);
            }
        }
        if self.mode("party") {
            if self.frame % 36 == 0 {
                self.party_burst();
                if !self.save.pet.enabled {
                    self.emotion = (self.emotion + 1.).min(100.);
                }
            }
            if self.drag.is_none() && self.burn <= 0. && self.buddy.awake && !self.save.pet.enabled
            {
                let sway = (self.frame as f32 * 0.22).sin() * 0.8;
                self.buddy.parts[1].prev.x -= sway;
                self.buddy.parts[2].prev.x += sway;
            }
        }
        if self.down && self.drag.is_some() {
            self.say("Wheee! Careful where you put me!");
        }
        if self.burn > 0. {
            self.say("Hot! A fire extinguisher would be nice!");
        }
        if self.unconscious > 0 {
            self.say("I might need a little break...");
        }
        if self.desktop && self.save.window_dialogue && self.frame % 36 == 0 {
            let app = foreground_app(self.desktop_handle);
            if let Some(app) = app {
                self.window_comment(app);
            }
        }
        // Sample every five seconds, only when higher-priority dialogue can finish.
        if self.frame % (36 * 5) == 0
            && self.save.dialogue
            && self.save.idle_chatter
            && self.frame >= self.dialogue_after
            && self.message_time <= 0.
            && self.warn_time <= 0.
            && self.save_error.is_none()
            && self.burn <= 0.
            && self.unconscious == 0
            && self.drag.is_none()
        {
            let lines = if self.mode("zerograv") {
                &[
                    "Look, no floor!",
                    "I could float here all day.",
                    "Who misplaced gravity?",
                ][..]
            } else if self.mode("superbounce") {
                &[
                    "Boing! And again!",
                    "Those toys just keep bouncing!",
                    "Good luck catching that ball!",
                ][..]
            } else if self.mode("party") {
                &[
                    "Now this is a party!",
                    "More confetti? Don't mind if I do!",
                    "Who's picking the music?",
                ][..]
            } else if self.mode("glitter") {
                &[
                    "Did I leave sparkles again?",
                    "Now that's a shiny entrance!",
                    "I bring my own glitter.",
                ][..]
            } else if self.mode("wind") {
                &[
                    "A little breezy in here!",
                    "Good day for a kite.",
                    "Hold on to your hats!",
                ][..]
            } else if self.mode("magnet") {
                &[
                    "Those things really like your cursor.",
                    "Quite an attractive pointer!",
                    "Keep that magnet away from my snacks.",
                ][..]
            } else if self.emotion > 35. {
                &[
                    "This is a pretty good day!",
                    "You're fun to have around.",
                    "I think we're getting along!",
                ][..]
            } else if self.emotion < -35. {
                &[
                    "Maybe something gentler next?",
                    "Could use a little cheering up.",
                    "A few bubbles might help.",
                ][..]
            } else if self.save.idle_chatter
                && !self.down
                && !self.pressed
                && self.mouse.distance_squared(self.old_mouse) < 1.
            {
                &[
                    "Just hanging out.",
                    "Got any plans for us?",
                    "I wonder what rubber ducks dream about.",
                    "Quiet moments are nice, too.",
                    "I'm ready when you are.",
                ][..]
            } else {
                &[][..]
            };
            if !lines.is_empty() && self.random() < 0.35 {
                let index = (self.random() * lines.len() as f32) as usize;
                self.say(lines[index]);
            }
        }
    }
}

// Recognize app names, never echo document names, chat names, or browser URLs.
fn app_from_title(title: &str) -> String {
    let lower = title.to_lowercase();
    for (needle, name) in [
        ("discord", "Discord"),
        ("firefox", "Firefox"),
        ("chrome", "Chrome"),
        ("edge", "Edge"),
        ("visual studio code", "VS Code"),
        ("notepad", "Notepad"),
        ("spotify", "Spotify"),
        ("steam", "Steam"),
        ("explorer", "File Explorer"),
        ("terminal", "Terminal"),
    ] {
        if lower.contains(needle) {
            return name.into();
        }
    }
    "another window".into()
}

#[cfg(windows)]
fn foreground_app(own: Option<usize>) -> Option<String> {
    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetForegroundWindow() -> desktop::Hwnd;
        fn GetWindowTextW(hwnd: desktop::Hwnd, text: *mut u16, max: i32) -> i32;
    }
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_null() || Some(hwnd as usize) == own {
        return None;
    }
    let mut text = [0u16; 512];
    let n = unsafe { GetWindowTextW(hwnd, text.as_mut_ptr(), text.len() as i32) };
    if n <= 0 {
        return None;
    }
    let title = String::from_utf16_lossy(&text[..n as usize]);
    if title.contains("Interactive Buddy") {
        return None;
    }
    Some(app_from_title(&title))
}
#[cfg(not(windows))]
fn foreground_app(_: Option<usize>) -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn party_time_is_immediate_and_keeps_dancing() {
        let mut g = Game::new();
        g.demo = true;
        g.save = Save::default();
        g.action(Action::Select("Modes".into(), "Party Time".into()));
        assert!(g.mode("party"));
        assert_eq!(g.fx.len(), 30);
        assert!(g.fx.iter().all(|f| f.r >= 5.));
        let arm = g.buddy.parts[1].prev;
        g.frame = 35;
        g.content_step();
        assert_ne!(arm, g.buddy.parts[1].prev);
        g.frame = 36;
        g.content_step();
        assert_eq!(g.fx.len(), 60);
        g.action(Action::Select("Modes".into(), "Party Time".into()));
        g.frame = 72;
        g.content_step();
        assert_eq!(g.fx.len(), 60);
    }
    #[test]
    fn new_modes_emit_bubbles_and_orbit_safe_toys() {
        let mut g = Game::new();
        g.demo = true;
        g.save = Save::default();
        g.save
            .modes
            .extend(["bubbleShower".into(), "toyOrbit".into()]);
        g.frame = 6;
        let p = g.buddy.parts[5].pos;
        for kind in ["Rubber Balls", "Grenades"] {
            g.objects.push(Object {
                pos: p,
                prev: p,
                r: 8.,
                kind: kind.into(),
                life: 5.,
                held: false,
                hit_timer: 0,
                window_hit_timer: 0,
            });
        }
        g.content_step();
        assert!(!g.fx.is_empty());
        assert_ne!(g.objects[0].prev, p);
        assert_eq!(g.objects[1].prev, p);
    }
    #[test]
    fn window_comments_hide_private_titles() {
        assert_eq!(app_from_title("private-channel - Discord"), "Discord");
        assert_eq!(app_from_title("bank account - Google Chrome"), "Chrome");
        assert_eq!(app_from_title("secret document"), "another window");
    }
    #[test]
    fn content_tools_are_distinct_and_chatter_respects_notices() {
        let mut g = Game::new();
        let p = g.buddy.parts[5].pos - Vec2::new(40., 0.);
        let before = g.buddy.parts[5].prev;
        assert!(g.content_tool("Repulsor", p));
        assert!(g.buddy.parts[5].prev.x < before.x);
        g.notify("Purchased Anvils");
        g.say("do not replace");
        assert_eq!(g.message, "Purchased Anvils");
        let before = g.fx.len();
        g.pressed = true;
        assert!(g.content_tool("Confetti Cannon", p));
        assert!(g.fx.len() >= before + 18);
        let before = g.emotion;
        assert!(g.content_tool("Bubble Wand", p));
        assert!(g.emotion > before);
        assert!(!g.content_tool("Beach Balls", p));
    }
    #[test]
    fn new_tools_have_distinct_art_and_gentle_effects() {
        assert_eq!(tool_art("Ice Wand"), Some("icewand"));
        assert_eq!(tool_art("Sparkler"), Some("sparkler"));
        assert_eq!(tool_art("Rubber Ducks"), Some("duck"));
        let mut g = Game::new();
        g.frame = 6;
        g.emotion = 0.;
        g.burn = 2.;
        let p = g.buddy.parts[5].pos;
        for part in &mut g.buddy.parts {
            part.prev = part.pos - Vec2::new(4., 0.);
        }
        let far_pos = p + Vec2::new(300., 0.);
        g.buddy.parts[0].pos = far_pos;
        g.buddy.parts[0].prev = far_pos - Vec2::new(4., 0.);
        assert!(g.content_tool("Ice Wand", p));
        assert!(g.burn < 2.);
        assert!((g.buddy.parts[5].pos.x - g.buddy.parts[5].prev.x - 3.88).abs() < 0.001);
        assert_eq!(g.buddy.parts[0].prev, far_pos - Vec2::new(4., 0.));
        assert_eq!(g.emotion, 0.25);
        assert_eq!(g.fx.len(), 4);
        let burn = g.burn;
        let health = g.health;
        let before = g.buddy.parts[5].prev;
        assert!(g.content_tool("Sparkler", p));
        assert_eq!(g.burn, burn);
        assert_eq!(g.health, health);
        assert_eq!(g.buddy.parts[5].prev, before);
        assert!((g.emotion - 0.35).abs() < 0.001);
        assert_eq!(g.fx.len(), 9);
        assert!(g.desktop_impacts.is_empty());
        g.frame = 7;
        assert!(g.content_tool("Sparkler", p));
        assert_eq!(g.fx.len(), 9, "particles must not spawn every frame");
        assert!(!g.content_tool("Rubber Ducks", p), "ducks are throwables");
    }

    #[test]
    fn dialogue_interval_is_clamped_and_notices_keep_priority() {
        let mut g = Game::new();
        g.save = Save::default();
        for (setting, seconds) in [(0, 8), (8, 8), (16, 16), (30, 30), (100, 60)] {
            g.save.dialogue_interval = setting;
            g.frame = g.dialogue_after;
            g.message_time = 0.;
            assert!(g.say("Hello!"));
            assert_eq!(g.dialogue_after, g.frame + 36 * seconds);
        }
        g.frame = g.dialogue_after;
        g.message_time = 0.;
        g.warn_time = 1.;
        assert!(!g.say("Must not overwrite warnings"));
        g.warn_time = 0.;
        g.save_error = Some("save failed".into());
        assert!(!g.say("Must not overwrite save errors"));
        g.save_error = None;
        g.save.dialogue = false;
        assert!(!g.say("Dialogue disabled"));
    }

    #[test]
    fn idle_chatter_is_optional_random_and_rate_limited() {
        let mut g = Game::new();
        g.save = Save::default();
        g.save.idle_chatter = false;
        g.emotion = 0.;
        g.frame = 180;
        g.rng = 0; // Deterministic successful chatter roll.
        g.old_mouse = g.mouse;
        g.content_step();
        assert_eq!(g.message_time, 0.);
        g.save.idle_chatter = true;
        g.frame = 181;
        g.content_step();
        assert_eq!(g.message_time, 0., "no every-frame chatter checks");
        g.frame = 360;
        g.content_step();
        assert_eq!(g.message, "Just hanging out.");
        let after = g.dialogue_after;
        g.message_time = 0.;
        g.frame = 540;
        g.content_step();
        assert_eq!(g.dialogue_after, after, "cooldown must still apply");
        g.frame = 3600;
        g.save.idle_chatter = false;
        g.save.modes.insert("zerograv".into());
        g.content_step();
        assert_eq!(
            g.message_time, 0.,
            "random chatter toggle also silences mode remarks"
        );
        g.save.idle_chatter = true;
        g.frame = 5400;
        g.content_step();
        assert_eq!(g.message, "Look, no floor!");
        g.message_time = 0.;
        g.frame = 7200;
        g.save.dialogue = false;
        g.content_step();
        assert_eq!(g.message_time, 0., "master dialogue disables all chatter");
    }

    #[test]
    fn glitter_trails_motion_only_when_enabled() {
        let mut g = Game::new();
        g.save = Save::default();
        g.frame = 3;
        g.buddy.parts[0].prev -= Vec2::new(2., 0.);
        g.content_step();
        assert!(g.fx.is_empty());
        g.save.modes.insert("glitter".into());
        g.content_step();
        assert_eq!(g.fx.len(), 2);
    }
    #[test]
    fn modes_apply_forces_and_particles() {
        let mut g = Game::new();
        g.frame = 72;
        g.save
            .modes
            .extend(["wind".into(), "party".into(), "magnet".into()]);
        g.objects.push(Object {
            pos: Vec2::new(100., 100.),
            prev: Vec2::new(100., 100.),
            r: 11.,
            kind: "Beach Balls".into(),
            life: f32::INFINITY,
            held: false,
            hit_timer: 0,
            window_hit_timer: 0,
        });
        g.mouse = Vec2::new(200., 100.);
        let before = g.buddy.parts[5].prev;
        g.content_step();
        assert!(g.buddy.parts[5].prev.x < before.x);
        assert!(g.objects[0].prev.x < 100.);
        assert!(g.fx.len() >= 19);
    }
    #[test]
    fn window_comment_retries_after_notices_and_discards_stale_apps() {
        let mut g = Game::new();
        g.save = Save::default();
        g.notify("Purchased Anvils");
        g.window_comment("Discord".into());
        g.window_comment("Discord".into());
        assert_ne!(g.window_app, "Discord");
        assert_eq!(g.message, "Purchased Anvils");
        // A newer stable app replaces the old one, rather than queuing stale comments.
        g.window_comment("Chrome".into());
        g.message_time = 0.;
        g.window_comment("Chrome".into());
        assert_eq!(g.window_app, "Chrome");
        assert!(g.message.contains("Chrome"));
        g.message_time = 0.;
        g.window_comment("Discord".into());
        g.window_comment("Discord".into());
        assert_eq!(g.window_app, "Chrome", "cooldown must defer the comment");
        g.frame = g.dialogue_after;
        g.window_comment("Discord".into());
        assert_eq!(g.window_app, "Discord");
    }

    #[test]
    fn magnetic_cursor_uses_camera_space_and_leaves_held_objects_alone() {
        let mut g = Game::new();
        g.save = Save::default();
        g.save.modes.insert("magnet".into());
        g.camera_scale = 2.;
        g.camera_offset = Vec2::new(100., 0.);
        g.mouse = g.stage * 0.5;
        let pos = g.stage * 0.5;
        for held in [false, true] {
            g.objects.push(Object {
                pos,
                prev: pos,
                r: 11.,
                kind: "Beach Balls".into(),
                life: f32::INFINITY,
                held,
                hit_timer: 0,
                window_hit_timer: 0,
            });
        }
        g.content_step();
        assert!(
            g.objects[0].prev.x > pos.x,
            "attraction must pull left in scene space"
        );
        assert_eq!(g.objects[1].prev, pos, "held objects must not be pulled");
    }

    #[test]
    fn new_items_survive_real_simulation_and_release() {
        for item in [
            "Beach Balls",
            "Anvils",
            "Bubble Wand",
            "Repulsor",
            "Confetti Cannon",
            "Ice Wand",
            "Sparkler",
            "Rubber Ducks",
        ] {
            let mut g = Game::new();
            g.demo = true;
            g.save = Save::default();
            g.save.item = item.into();
            g.mouse = Vec2::new(200., 150.);
            g.pressed = true;
            g.down = true;
            let mut app = App::new();
            app.insert_resource(g).add_systems(Update, simulate);
            for frame in 0..180 {
                if frame == 20 {
                    let mut g = app.world_mut().resource_mut::<Game>();
                    g.down = false;
                    g.released = true;
                }
                app.update();
                let g = app.world().resource::<Game>();
                assert!(g.buddy.parts.iter().all(|p| p.pos.is_finite()), "{item}");
                assert!(g.objects.iter().all(|o| o.pos.is_finite()), "{item}");
                assert!(g.fx.iter().all(|f| f.pos.is_finite()), "{item}");
            }
            let g = app.world().resource::<Game>();
            if matches!(item, "Beach Balls" | "Anvils" | "Rubber Ducks") {
                assert_eq!(g.objects.len(), 1, "{item} did not spawn");
                assert!(!g.objects[0].held);
            }
        }
    }

    #[test]
    fn options_load_old_saves_and_playtest_never_writes_progress() {
        let mut g = Game::new();
        g.save = serde_json::from_str("{\"cash\":42}").unwrap();
        assert!(g.save.dialogue && g.save.window_dialogue);
        g.demo = true;
        g.save_path = std::env::temp_dir().join(format!("ib-playtest-{}.json", std::process::id()));
        std::fs::write(&g.save_path, "normal progress").unwrap();
        g.action(Action::ToggleDialogue);
        g.action(Action::ToggleWindowDialogue);
        assert!(!g.save.dialogue && !g.save.window_dialogue);
        let encoded = serde_json::to_string(&g.save).unwrap();
        let decoded: Save = serde_json::from_str(&encoded).unwrap();
        assert!(!decoded.dialogue && !decoded.window_dialogue);
        g.persist();
        assert_eq!(
            std::fs::read_to_string(&g.save_path).unwrap(),
            "normal progress"
        );
        std::fs::remove_file(&g.save_path).unwrap();
    }

    #[test]
    fn new_assets_match_base_dimensions() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
        let manifest: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(root.join("content_manifest.json")).unwrap(),
        )
        .unwrap();
        for (asset, entry) in manifest.as_object().unwrap() {
            let png = std::fs::read(root.join(asset)).unwrap();
            let base = std::fs::read(root.join(entry["base"].as_str().unwrap())).unwrap();
            assert_eq!(&png[16..24], &base[16..24], "{asset} dimensions differ");
        }
    }
}
