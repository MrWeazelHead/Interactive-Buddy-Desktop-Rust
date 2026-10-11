//! Opt-in native UI/asset probe. Always uses an unsaved playtest.
use crate::*;
pub(crate) fn run(
    mut commands: Commands,
    mut g: ResMut<Game>,
    art: Res<Art>,
    images: Res<Assets<Image>>,
    target: Res<desktop_present::DesktopFrame>,
    mut frame: Local<u32>,
    mut exit: MessageWriter<AppExit>,
) {
    if !std::env::args().any(|a| a == "--pet-smoke-test") {
        return;
    }
    *frame += 1;
    if *frame == 30 {
        g.demo = true;
        g.save.pet.enabled = true;
        g.save.pet.needs = [30., 40., 35.];
        g.panel = "Passive Pet".into();
    }
    if *frame == 90 {
        g.panel = "Skin Maker".into();
        g.skin_editor.skin.name = "Painted Buddy".into();
        g.skin_action("brush", 9);
        g.skin_action("fill", 0);
        g.skin_editor.part = 5;
        g.skin_action("brush", 12);
        g.skin_action("fill", 0);
        for i in 0..256 {
            if (i / 16 + i % 16) % 4 == 0 {
                g.skin_editor.skin.pixels[0][i] = 4;
            }
        }
        g.skin_editor.revision += 1;
        g.skin_action("save", 0);
    }
    if *frame == 150 {
        g.panel.clear();
        g.reset_buddy();
        g.save.pet.affection = 60.;
        g.save.pet.needs = [25.; 3];
        g.save.pet.stations = [0.25, 0.5, 0.75, 0.9]
            .into_iter()
            .enumerate()
            .map(|(kind, x)| pet::Station {
                kind,
                x,
                supply: 100.,
            })
            .collect();
    }
    if *frame == 210 {
        g.save.pet.enabled = false;
        g.action(Action::Select("Modes".into(), "Party Time".into()));
    }
    if *frame == 270 {
        g.action(Action::Select("Modes".into(), "Bubble Shower".into()));
        g.action(Action::Select("Modes".into(), "Toy Orbit".into()));
        g.panel = "Settings".into();
        g.panel.clear();
        g.desktop_open = true;
        g.desktop_section = "Settings".into();
        if !g.desktop {
            g.menu = "Settings".into();
        }
    }
    if *frame == 330 {
        g.message_time = 0.;
        g.warn_time = 0.;
        g.save.modes.clear();
        g.save.pet.enabled = true;
        g.panel.clear();
        g.menu.clear();
        g.desktop_open = true;
        g.desktop_section = "Items".into();
        g.category = "Pet Items".into();
        g.page = 0;
        if !g.desktop {
            g.menu = "Items".into();
        }
        g.save.pet.hygiene = 35.;
        g.save.pet.health = 65.;
        g.save.pet.energy = 25.;
        g.save.pet.toilet = 75.;
        g.save.pet.wastes = vec![pet::Waste { x: 0.4 }, pet::Waste { x: 0.65 }];
        g.save.pet.stations.push(pet::Station {
            kind: 4,
            x: 0.55,
            supply: 100.,
        });
    }
    if *frame == 390 {
        g.menu.clear();
        g.panel = "Passive Pet".into();
    }
    if *frame == 450 {
        g.panel.clear();
        g.menu.clear();
        g.desktop_section.clear();
        g.save.pet.enabled = false;
        g.reset_buddy();
        g.save.modes.insert("pinball".into());
        g.pressed = true;
        let stage = g.stage;
        g.advanced_tool("Portal Gun", stage * Vec2::new(0.25, 0.5));
        g.advanced_tool("Portal Gun", stage * Vec2::new(0.75, 0.5));
        g.advanced_tool("Stasis Field", stage * 0.5);
        g.pressed = false;
    }
    if *frame == 510 {
        g.panel.clear();
        g.menu.clear();
        g.desktop_section.clear();
        g.save.modes.clear();
        g.fx.clear();
        g.save.window_dialogue = false;
        g.save.idle_chatter = false;
        g.message_time = 0.;
        g.warn_time = 0.;
        g.dialogue_after = 0;
        g.save.pet.enabled = true;
        g.save.pet.show_stats = false;
        g.save.pet.need_lights = true;
        g.save.pet.need_dialogue = true;
        g.save.pet.needs = [90., 5., 90.];
        g.save.pet.toilet = 0.;
        g.save.pet.energy = 90.;
        g.save.pet.hygiene = 90.;
        g.save.pet.health = 100.;
        g.save.pet.stations.clear();
        g.frame = 180;
        g.pet_step();
        assert!(g.message.contains("thirsty"));
    }
    if *frame == 570 {
        g.save.pet.show_stats = true;
    }
    if *frame == 630 {
        g.save.pet.enabled = false;
        g.message_time = 0.;
        g.save.modes.clear();
        g.reset_buddy();
        g.objects.clear();
        g.save.modes.insert("gravityCarousel".into());
        g.save.modes.insert("toySwarm".into());
        let stage = g.stage;
        g.pressed = true;
        g.advanced_tool("Launch Pad", stage * Vec2::new(0.5, 0.7));
        g.pressed = false;
        for i in 0..5 {
            let pos = stage * 0.5 + Vec2::new(i as f32 * 24. - 48., -30.);
            g.objects.push(Object {
                pos,
                prev: pos,
                r: 6.,
                kind: "Baseballs".into(),
                life: 100.,
                held: false,
                hit_timer: 0,
                window_hit_timer: 0,
            });
        }
    }
    if *frame == 690 {
        g.advanced = crate::advanced::Advanced::default();
        g.objects.clear();
        g.fx.clear();
        g.save.modes.clear();
        g.save.pet.enabled = true;
        g.save.pet.show_stats = false;
        g.save.pet.need_dialogue = false;
        g.save.pet.stations.clear();
        g.save.pet.wastes.clear();
        g.save.pet.needs = [35., 20., 20.];
        let stage = g.stage;
        g.pressed = true;
        g.pet_tool("Pet Cage", stage * 0.5);
        for (i, item) in [
            "Food Bowl",
            "Water Bowl",
            "Toy Basket",
            "Pet Bed",
            "Litter Tray",
        ]
        .into_iter()
        .enumerate()
        {
            g.pet_tool(
                item,
                Vec2::new(stage.x * 0.5 + (i as f32 - 2.) * 22., stage.y - 30.),
            );
        }
        g.pressed = false;
        g.message_time = 0.;
    }
    if *frame == 750 {
        g.pet_action("release_cage");
    }
    if [
        60, 120, 180, 240, 300, 360, 420, 480, 540, 600, 660, 720, 780,
    ]
    .contains(&*frame)
    {
        for part in skinmaker::PARTS {
            let h = art
                .skins
                .get(&format!("draft_{part}"))
                .expect("generated draft");
            let im = images.get(h).expect("generated image");
            let base = images
                .get(art.skins.get(&format!("default_{part}")).unwrap())
                .unwrap();
            assert_eq!(im.texture_descriptor.size, base.texture_descriptor.size);
        }
        for key in [
            "foodbowl",
            "waterbowl",
            "toybasket",
            "petbed",
            "featherwand",
            "petbrush",
            "littertray",
            "poopscoop",
            "petsoap",
            "petmedicine",
            "poop",
            "portalgun",
            "grapple",
            "stasis",
            "bazooka_launcher",
            "bazooka_rocket",
            "tractorbeam",
            "launchpad",
            "petcage",
        ] {
            assert!(
                images
                    .get(art.tools.get(key).expect("tool loaded"))
                    .is_some(),
                "{key}"
            );
        }
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("screenshots");
        std::fs::create_dir_all(&root).unwrap();
        let capture = target
            .target
            .clone()
            .map(bevy::render::view::screenshot::Screenshot::image)
            .unwrap_or_else(bevy::render::view::screenshot::Screenshot::primary_window);
        commands
            .spawn(capture)
            .observe(bevy::render::view::screenshot::save_to_disk(root.join(
                format!(
                    "pet-{}-{}.png",
                    if g.desktop { "desktop" } else { "classic" },
                    *frame / 60
                ),
            )));
    }
    if *frame == 810 {
        println!(
            "PET/SKIN NATIVE SMOKE PASSED: UI, skins, Tamagotchi care, Pet Items, portals, stasis, pinball, assets and Party Time"
        );
        exit.write(AppExit::Success);
    }
}
