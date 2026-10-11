//! Favorites, quick slots, named loadouts, and simulation controls.
use crate::*;

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Preset {
    name: String,
    item: String,
    pub(crate) skin: String,
    modes: HashSet<String>,
    gravity: f32,
    speed: f32,
}

pub(crate) fn timing(g: Res<Game>, mut fixed: ResMut<Time<Fixed>>) {
    let speed = if [0.5, 1., 2.].contains(&g.save.simulation_speed) {
        g.save.simulation_speed
    } else {
        1.
    };
    fixed.set_timestep_hz(36. * speed as f64);
}

impl Game {
    pub(crate) fn in_category(&self, entry: &Entry, category: &str) -> bool {
        if !self.item_visible(entry) {
            return false;
        }
        if category == "Favorites" {
            self.save.favorites.contains(&entry.name)
        } else {
            entry.category == category
        }
    }

    pub(crate) fn cancel_tool_input(&mut self) {
        self.advanced_cancel_pointer();
        self.down = false;
        self.pressed = false;
        self.released = false;
        self.drag = None;
        self.desktop_capture = false;
        for object in &mut self.objects {
            if object.held {
                object.held = false;
                object.prev = object.pos;
            }
        }
    }

    fn usable_tool(&self, name: &str) -> bool {
        self.save.owned.contains(name)
            && self
                .catalog
                .items
                .iter()
                .any(|e| e.name == name && self.item_visible(e))
    }

    pub(crate) fn gameplay_action(&mut self, action: Action) {
        match action {
            Action::ToggleFavorite => {
                let item = self.save.item.clone();
                if self.usable_tool(&item) && !self.save.favorites.remove(&item) {
                    self.save.favorites.insert(item);
                }
            }
            Action::AssignSlot(index) => {
                if index < 3 && self.usable_tool(&self.save.item) {
                    self.save.quick_slots[index] = Some(self.save.item.clone());
                }
            }
            Action::UseSlot(index) => {
                if let Some(name) = self
                    .save
                    .quick_slots
                    .get(index)
                    .and_then(Clone::clone)
                    .filter(|name| self.usable_tool(name))
                {
                    self.action(Action::Select("Items".into(), name));
                }
                return;
            }
            Action::EditPresetName => {
                self.editing_preset = true;
                self.preset_name.clear();
            }
            Action::SavePreset => {
                if self.mode("script") {
                    self.notify("Turn off Script mode before saving a preset.");
                    return;
                }
                let name: String = self.preset_name.trim().chars().take(24).collect();
                if name.is_empty() {
                    self.notify("Give the preset a name first.");
                    return;
                }
                let preset = Preset {
                    name: name.clone(),
                    item: self.save.item.clone(),
                    skin: self.save.skin.clone(),
                    modes: self.save.modes.clone(),
                    gravity: self.save.gravity,
                    speed: self.save.simulation_speed,
                };
                if let Some(old) = self.save.presets.iter_mut().find(|p| p.name == name) {
                    *old = preset;
                } else if self.save.presets.len() < 4 {
                    self.save.presets.push(preset);
                } else {
                    self.notify("Four presets saved. Delete one or reuse its name.");
                    return;
                }
                self.notify(format!("Saved preset: {name}"));
            }
            Action::LoadPreset(index) => {
                if let Some(preset) = self.save.presets.get(index).cloned() {
                    let skin_owned = self
                        .catalog
                        .skins
                        .iter()
                        .any(|e| e.id == preset.skin && self.save.owned.contains(&e.name));
                    let modes_owned = preset.modes.iter().all(|id| {
                        self.catalog.modes.iter().any(|e| {
                            &e.id == id && id != "script" && self.save.owned.contains(&e.name)
                        })
                    });
                    if !self.usable_tool(&preset.item) || !skin_owned || !modes_owned {
                        self.notify("Preset needs content you no longer own.");
                        return;
                    }
                    self.action(Action::Select("Items".into(), preset.item));
                    self.save.skin = preset.skin;
                    self.save.modes = preset.modes;
                    self.save.gravity = if preset.gravity.is_finite() {
                        preset.gravity.clamp(-5., 5.)
                    } else {
                        1.
                    };
                    self.save.simulation_speed = if [0.5, 1., 2.].contains(&preset.speed) {
                        preset.speed
                    } else {
                        1.
                    };
                    self.notify(format!("Loaded preset: {}", preset.name));
                }
            }
            Action::DeletePreset(index) => {
                if index < self.save.presets.len() {
                    self.save.presets.remove(index);
                }
            }
            Action::ToggleIdleChatter => self.save.idle_chatter = !self.save.idle_chatter,
            Action::CycleDialogueInterval => {
                self.save.dialogue_interval = match self.save.dialogue_interval {
                    8 => 16,
                    16 => 30,
                    _ => 8,
                }
            }
            Action::CycleSpeed => {
                self.save.simulation_speed = match self.save.simulation_speed {
                    0.5 => 1.,
                    1. => 2.,
                    _ => 0.5,
                }
            }
            Action::Step => {
                self.paused = true;
                self.step_once = true;
                return;
            }
            _ => unreachable!("not a gameplay action"),
        }
        self.persist();
    }
}

pub(crate) fn draw_loadout(d: &mut Painter<'_, '_, '_>, g: &Game) {
    let favorite = g.save.favorites.contains(&g.save.item);
    d.button(
        98.,
        96.,
        356.,
        21.,
        &ellipsize(
            &format!(
                "{} favorite: {}",
                if favorite { "Remove" } else { "Add" },
                g.save.item
            ),
            47,
        ),
        Action::ToggleFavorite,
    );
    for (index, slot) in g.save.quick_slots.iter().enumerate() {
        let y = 122. + index as f32 * 25.;
        d.button(
            98.,
            y,
            238.,
            21.,
            &ellipsize(
                &format!("{}: {}", index + 1, slot.as_deref().unwrap_or("Empty")),
                32,
            ),
            Action::UseSlot(index),
        );
        d.button(
            342.,
            y,
            112.,
            21.,
            "Assign current",
            Action::AssignSlot(index),
        );
    }
    d.rule(98., 201., 356.);
    d.button(
        98.,
        208.,
        250.,
        23.,
        &format!(
            "{}{}",
            g.preset_name,
            if g.editing_preset {
                " | Enter to finish"
            } else {
                " (rename)"
            }
        ),
        Action::EditPresetName,
    );
    d.primary(354., 208., 100., 23., "Save preset", Action::SavePreset);
    for (index, preset) in g.save.presets.iter().take(4).enumerate() {
        let y = 240. + index as f32 * 24.;
        d.button(
            98.,
            y,
            300.,
            21.,
            &format!("Load: {}", preset.name),
            Action::LoadPreset(index),
        );
        d.button(404., y, 50., 21., "Delete", Action::DeletePreset(index));
    }
    d.text(
        98.,
        339.,
        if g.desktop {
            "Quick slots: Ctrl+Alt+1 / 2 / 3"
        } else {
            "Quick slots: 1 / 2 / 3"
        },
        10.,
        ui::MUTED,
    );
}

pub(crate) fn draw_physics(d: &mut Painter<'_, '_, '_>, g: &Game) {
    d.text(
        102.,
        98.,
        format!(
            "Gravity: {:.2}   ·   Speed: {}x",
            g.save.gravity, g.save.simulation_speed
        ),
        12.,
        ui::TEXT,
    );
    for (x, name, gravity) in [
        (102., "Low", 0.15),
        (219., "Normal", 1.),
        (336., "Reverse", -1.),
    ] {
        d.button(x, 128., 112., 24., name, Action::Gravity(gravity));
    }
    d.button(
        102.,
        169.,
        346.,
        24.,
        &format!("Simulation speed: {}x (cycle)", g.save.simulation_speed),
        Action::CycleSpeed,
    );
    d.button(
        102.,
        202.,
        167.,
        24.,
        if g.paused { "Resume" } else { "Pause" },
        Action::Pause,
    );
    d.button(
        281.,
        202.,
        167.,
        24.,
        if g.desktop {
            "Step  Ctrl+Alt+."
        } else {
            "Single step  ."
        },
        Action::Step,
    );
    d.text(102., 242., "Single step pauses and advances one physics frame.\nResume, then close the panel for continuous play.\nSpeed changes timing, not the physics step size.", 11., ui::MUTED);
}

pub(crate) fn draw_dialogue(d: &mut Painter<'_, '_, '_>, g: &Game) {
    for (index, (label, on, action)) in [
        ("Buddy dialogue", g.save.dialogue, Action::ToggleDialogue),
        (
            "Window comments",
            g.save.window_dialogue,
            Action::ToggleWindowDialogue,
        ),
        (
            "Random chatter",
            g.save.idle_chatter,
            Action::ToggleIdleChatter,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        d.button(
            102.,
            100. + index as f32 * 32.,
            346.,
            25.,
            &format!("{label}: {}", if on { "On" } else { "Off" }),
            action,
        );
    }
    d.button(
        102.,
        204.,
        346.,
        25.,
        &format!("Minimum chatter gap: {}s (cycle)", g.save.dialogue_interval),
        Action::CycleDialogueInterval,
    );
    d.text(102., 249., "Chatter respects purchases, warnings and save errors.\nWindow comments use local app recognition on Windows.\nDialogue timing follows simulation speed.", 11., ui::MUTED);
}

#[cfg(test)]
mod tests {
    use super::*;
    fn game() -> Game {
        let mut g = Game::new();
        g.demo = true;
        g.save = Save::default();
        g.save.owned = g
            .catalog
            .items
            .iter()
            .chain(&g.catalog.skins)
            .chain(&g.catalog.modes)
            .map(|e| e.name.clone())
            .collect();
        g
    }
    #[test]
    fn favorites_slots_and_named_presets_round_trip() {
        let mut g = game();
        g.action(Action::Select("Items".into(), "Bubble Wand".into()));
        g.action(Action::ToggleFavorite);
        g.action(Action::AssignSlot(0));
        g.save.skin = "robot".into();
        g.save.modes.insert("wind".into());
        g.preset_name = "Bubbles in wind".into();
        g.action(Action::SavePreset);
        g.save.item = "Open Hand".into();
        g.save.skin = "default".into();
        g.save.modes.clear();
        g.action(Action::LoadPreset(0));
        assert_eq!(g.save.item, "Bubble Wand");
        assert_eq!(g.save.skin, "robot");
        assert!(g.mode("wind"));
        let saved: Save = serde_json::from_str(&serde_json::to_string(&g.save).unwrap()).unwrap();
        assert!(saved.favorites.contains("Bubble Wand"));
        assert_eq!(saved.quick_slots[0].as_deref(), Some("Bubble Wand"));
        assert_eq!(saved.presets[0].name, "Bubbles in wind");
        g.action(Action::ToggleFavorite);
        assert!(!g.save.favorites.contains("Bubble Wand"));
        g.action(Action::DeletePreset(0));
        assert!(g.save.presets.is_empty());
        g.action(Action::Hand);
        g.action(Action::UseSlot(0));
        assert_eq!(g.save.item, "Bubble Wand");
        g.save.owned.remove("Bubble Wand");
        g.action(Action::Hand);
        g.action(Action::UseSlot(0));
        assert_eq!(g.save.item, "Open Hand");
    }
    #[test]
    fn presets_reject_locked_content_and_cancel_drops_without_a_throw() {
        let mut g = game();
        g.preset_name = "test".into();
        g.save.skin = "robot".into();
        g.action(Action::SavePreset);
        g.save.owned.remove("Robot");
        g.save.skin = "default".into();
        g.action(Action::LoadPreset(0));
        assert_eq!(g.save.skin, "default");
        assert!(g.message.contains("no longer own"));
        g.save.modes.insert("script".into());
        g.preset_name = "unsupported".into();
        g.action(Action::SavePreset);
        assert_eq!(g.save.presets.len(), 1);
        g.down = true;
        g.pressed = true;
        g.released = true;
        g.objects.push(Object {
            pos: Vec2::new(20., 40.),
            prev: Vec2::ZERO,
            kind: "Tomatoes".into(),
            r: 8.,
            held: true,
            life: f32::INFINITY,
            hit_timer: 0,
            window_hit_timer: 0,
        });
        g.action(Action::Hand);
        assert!(!g.down && !g.pressed && !g.released);
        assert!(!g.objects[0].held);
        assert_eq!(g.objects[0].pos, g.objects[0].prev);
    }
    #[test]
    fn native_escape_ends_editing_and_passthrough_overrides_slot_capture() {
        let mut g = game();
        g.desktop = true;
        g.save.quick_slots[0] = Some("Machine Gun".into());
        g.set_passthrough(true);
        g.action(Action::UseSlot(0));
        assert_eq!(g.save.item, "Machine Gun");
        assert!(!g.captures_desktop_tools());
        g.set_passthrough(false);
        assert!(g.captures_desktop_tools());
        g.panel = "Loadout".into();
        g.editing_preset = true;
        g.escape_desktop();
        assert!(!g.editing_preset);
        assert!(g.panel.is_empty());
        assert!(!g.captures_desktop_tools());
    }
    #[test]
    fn quit_is_emitted_even_while_editing_a_preset() {
        let mut g = game();
        g.editing_preset = true;
        g.quit = true;
        let mut app = App::new();
        app.insert_resource(g)
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<ButtonInput<KeyCode>>()
            .add_message::<bevy::input::keyboard::KeyboardInput>()
            .add_message::<AppExit>()
            .add_systems(Update, input);
        app.update();
        assert!(!app.world().resource::<Messages<AppExit>>().is_empty());
    }
    #[test]
    fn stepping_under_a_modal_does_not_use_hover_tools() {
        for tool in ["Fist", "Medieval Flail"] {
            let mut g = game();
            g.panel = "Physics".into();
            g.save.item = tool.into();
            g.mouse = g.buddy.parts[5].pos;
            g.old_mouse = g.mouse - Vec2::new(20., 0.);
            let flail = g.flail_pos;
            let hits = g.save.hits;
            g.action(Action::Step);
            let mut app = App::new();
            app.insert_resource(g).add_systems(Update, simulate);
            app.update();
            let g = app.world().resource::<Game>();
            assert_eq!(g.frame, 1);
            assert_eq!(g.flail_pos, flail, "{tool} fired under modal");
            assert_eq!(g.save.hits, hits, "{tool} hit Buddy under modal");
        }
    }
    #[test]
    fn single_step_runs_once_even_with_panel_open() {
        let mut g = game();
        g.panel = "Physics".into();
        g.action(Action::Step);
        let mut app = App::new();
        app.insert_resource(g).add_systems(Update, simulate);
        app.update();
        assert_eq!(app.world().resource::<Game>().frame, 1);
        app.update();
        assert_eq!(app.world().resource::<Game>().frame, 1);
        assert!(app.world().resource::<Game>().paused);
    }
    #[test]
    fn speed_updates_fixed_clock_and_old_saves_default() {
        let mut g = game();
        g.action(Action::CycleSpeed);
        assert_eq!(g.save.simulation_speed, 2.);
        let mut app = App::new();
        app.insert_resource(g)
            .insert_resource(Time::<Fixed>::from_hz(36.))
            .add_systems(Update, timing);
        app.update();
        assert!(
            (app.world()
                .resource::<Time<Fixed>>()
                .timestep()
                .as_secs_f64()
                - 1. / 72.)
                .abs()
                < 1e-8
        );
        let old: Save = serde_json::from_str("{}").unwrap();
        assert_eq!(old.simulation_speed, 1.);
        assert!(old.presets.is_empty());
        assert!(old.quick_slots.iter().all(Option::is_none));
    }
}
