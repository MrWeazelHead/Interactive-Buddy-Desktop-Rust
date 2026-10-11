//! Small in-game painter: a 16x16 color grid for each original-sized masked part.
use crate::*;
pub(crate) const PARTS: [&str; 6] = ["head", "rArm", "lArm", "rLeg", "lLeg", "body"];
const LABELS: [&str; 6] = [
    "Head",
    "Right arm",
    "Left arm",
    "Right leg",
    "Left leg",
    "Body",
];
pub(crate) const COLORS: [[u8; 3]; 16] = [
    [28, 35, 45],
    [250, 250, 245],
    [150, 170, 185],
    [80, 100, 115],
    [245, 90, 90],
    [250, 160, 70],
    [250, 220, 90],
    [110, 210, 110],
    [70, 190, 170],
    [90, 210, 250],
    [80, 130, 230],
    [145, 100, 230],
    [235, 130, 210],
    [155, 105, 70],
    [230, 190, 150],
    [75, 55, 95],
];
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct CustomSkin {
    pub id: u64,
    pub name: String,
    pub pixels: Vec<Vec<u8>>,
}
impl CustomSkin {
    pub fn entry(&self) -> Entry {
        Entry {
            name: format!("Custom: {}", self.name),
            category: String::new(),
            id: format!("custom_{}", self.id),
            price: 0.,
            initially_owned: true,
        }
    }
    fn validate(&mut self) {
        self.name = self
            .name
            .chars()
            .filter(|c| !c.is_control())
            .take(20)
            .collect();
        if self.name.trim().is_empty() {
            self.name = "My skin".into();
        }
        self.pixels.resize(6, vec![2; 256]);
        self.pixels.truncate(6);
        for p in &mut self.pixels {
            p.resize(256, 2);
            p.truncate(256);
            for c in p {
                *c = (*c).min(15);
            }
        }
    }
}
pub(crate) fn validate(skins: &mut Vec<CustomSkin>) {
    skins.truncate(8);
    let mut ids = HashSet::new();
    skins.retain(|s| s.id > 0 && s.id < u64::MAX && ids.insert(s.id));
    for s in skins {
        s.validate();
    }
}
#[derive(Clone)]
pub(crate) struct Editor {
    pub skin: CustomSkin,
    pub part: usize,
    pub brush: u8,
    pub revision: u64,
    pub browse: usize,
}
impl Default for Editor {
    fn default() -> Self {
        Self {
            skin: CustomSkin {
                id: 0,
                name: "My skin".into(),
                pixels: vec![vec![2; 256]; 6],
            },
            part: 0,
            brush: 4,
            revision: 1,
            browse: 0,
        }
    }
}
impl Game {
    pub(crate) fn skin_action(&mut self, key: &str, value: usize) {
        match key {
            "name" => {
                self.editing_preset = true;
                return;
            }
            "part" => self.skin_editor.part = (self.skin_editor.part + 1) % 6,
            "brush" => self.skin_editor.brush = value.min(15) as u8,
            "fill" => {
                self.skin_editor.skin.pixels[self.skin_editor.part].fill(self.skin_editor.brush)
            }
            "new" => {
                let rev = self.skin_editor.revision;
                self.skin_editor = Editor::default();
                self.skin_editor.revision = rev;
            }
            "browse" => {
                if !self.save.custom_skins.is_empty() {
                    self.skin_editor.browse =
                        (self.skin_editor.browse + 1) % self.save.custom_skins.len();
                }
            }
            "load" => {
                if let Some(s) = self.save.custom_skins.get(self.skin_editor.browse) {
                    self.skin_editor.skin = s.clone();
                }
            }
            "delete" => {
                if self.skin_editor.skin.id == 0 {
                    self.notify("Load a saved design before deleting it.");
                    return;
                }
                let id = self.skin_editor.skin.entry().id;
                self.save.custom_skins.retain(|s| s.entry().id != id);
                let old = self
                    .catalog
                    .skins
                    .iter()
                    .find(|s| s.id == id)
                    .map(|s| s.name.clone());
                self.catalog.skins.retain(|s| s.id != id);
                if let Some(name) = old {
                    self.save.owned.remove(&name);
                }
                if self.save.skin == id {
                    self.save.skin = "default".into();
                }
                for p in &mut self.save.presets {
                    if p.skin == id {
                        p.skin = "default".into();
                    }
                }
                let rev = self.skin_editor.revision;
                self.skin_editor = Editor::default();
                self.skin_editor.revision = rev;
                self.skin_revision += 1;
                self.persist();
                self.notify("Custom skin deleted. References restored to default.");
            }
            "save" => {
                let mut s = self.skin_editor.skin.clone();
                s.validate();
                if self
                    .save
                    .custom_skins
                    .iter()
                    .any(|old| old.id != s.id && old.name == s.name)
                {
                    self.notify("Choose a different skin name.");
                    return;
                }
                if s.id == 0 {
                    if self.save.custom_skins.len() >= 8 {
                        self.notify("Eight custom skins maximum. Load and delete one first.");
                        return;
                    }
                    s.id = (1..=9)
                        .find(|id| !self.save.custom_skins.iter().any(|old| old.id == *id))
                        .expect("at most eight saved skins");
                }
                if let Some(old) = self.catalog.skins.iter().find(|e| e.id == s.entry().id) {
                    self.save.owned.remove(&old.name);
                }
                self.catalog.skins.retain(|e| e.id != s.entry().id);
                self.save.owned.insert(s.entry().name.clone());
                self.catalog.skins.push(s.entry());
                self.save.skin = s.entry().id;
                self.save.custom_skins.retain(|old| old.id != s.id);
                self.save.custom_skins.push(s.clone());
                self.skin_editor.skin = s;
                self.skin_editor.browse = self.save.custom_skins.len() - 1;
                self.skin_revision += 1;
                self.persist();
                self.notify("Custom skin saved and equipped!");
            }
            _ => return,
        }
        self.skin_editor.revision += 1;
    }
}
fn recolor(base: &Image, pixels: &[u8]) -> Option<Image> {
    use bevy::render::render_resource::TextureFormat;
    if !matches!(
        base.texture_descriptor.format,
        TextureFormat::Rgba8UnormSrgb | TextureFormat::Rgba8Unorm
    ) {
        return None;
    }
    let mut image = base.clone();
    let w = image.texture_descriptor.size.width as usize;
    let h = image.texture_descriptor.size.height as usize;
    let data = image.data.as_mut()?;
    if pixels.len() != 256 || data.len() != w * h * 4 {
        return None;
    }
    for (i, rgba) in data.chunks_exact_mut(4).enumerate() {
        let x = i % w;
        let y = i / w;
        let col = COLORS[usize::from(pixels[(y * 16 / h) * 16 + x * 16 / w].min(15))];
        let light = (rgba[0] as f32 + rgba[1] as f32 + rgba[2] as f32) / 765.;
        for j in 0..3 {
            rgba[j] = (col[j] as f32 * (0.3 + 0.7 * light)) as u8;
        }
    }
    Some(image)
}
pub(crate) fn sync_art(
    g: Res<Game>,
    mut art: ResMut<Art>,
    mut images: ResMut<Assets<Image>>,
    mut previous: Local<Option<(u64, u64)>>,
) {
    let revision = (g.skin_revision, g.skin_editor.revision);
    if *previous == Some(revision) {
        return;
    }
    let mut bases = Vec::new();
    for part in PARTS {
        let Some(h) = art.skins.get(&format!("default_{part}")) else {
            return;
        };
        let Some(im) = images.get(h) else {
            return;
        };
        bases.push(im.clone());
    }
    // Drop obsolete handles; IDs remain stable across editing and deletion.
    art.skins
        .retain(|key, _| !key.starts_with("custom_") && !key.starts_with("draft_"));
    for skin in g
        .save
        .custom_skins
        .iter()
        .chain(std::iter::once(&g.skin_editor.skin))
    {
        let prefix = if std::ptr::eq(skin, &g.skin_editor.skin) {
            "draft".into()
        } else {
            format!("custom_{}", skin.id)
        };
        for (i, part) in PARTS.iter().enumerate() {
            if let Some(image) = recolor(&bases[i], &skin.pixels[i]) {
                art.skins
                    .insert(format!("{prefix}_{part}"), images.add(image));
            }
        }
    }
    *previous = Some(revision);
}
pub(crate) fn paint(mut g: ResMut<Game>) {
    if g.panel != "Skin Maker" || g.editing_preset || !g.down {
        return;
    }
    let p = g.mouse - g.panel_origin() - Vec2::new(100., 147.);
    if p.x >= 0. && p.y >= 0. && p.x < 144. && p.y < 144. {
        let index = (p.y / 9.) as usize * 16 + (p.x / 9.) as usize;
        let part = g.skin_editor.part;
        let brush = g.skin_editor.brush;
        if g.skin_editor.skin.pixels[part][index] != brush {
            g.skin_editor.skin.pixels[part][index] = brush;
            g.skin_editor.revision += 1;
        }
    }
}
pub(crate) fn draw_panel(d: &mut Painter<'_, '_, '_>, g: &Game) {
    let ed = &g.skin_editor;
    d.button(
        98.,
        96.,
        180.,
        23.,
        &format!("Name: {}", ed.skin.name),
        Action::Skin("name".into(), 0),
    );
    d.button(286., 96., 69., 23., "New", Action::Skin("new".into(), 0));
    d.primary(
        362.,
        96.,
        89.,
        23.,
        "Save + Equip",
        Action::Skin("save".into(), 0),
    );
    for (i, col) in COLORS.iter().enumerate() {
        let x = 100. + i as f32 * 21.;
        let color = Color::srgb_u8(col[0], col[1], col[2]);
        d.rect(
            x,
            125.,
            18.,
            16.,
            if i == ed.brush as usize {
                Color::WHITE
            } else {
                ui::HAIRLINE
            },
        );
        d.rect(x + 2., 127., 14., 12., color);
        d.hits.push(Hit {
            pos: d.view(Vec2::new(x + 9., 133.)),
            size: Vec2::new(18., 16.),
            action: Action::Skin("brush".into(), i),
        });
    }
    for (i, pixel) in ed.skin.pixels[ed.part].iter().enumerate() {
        let col = COLORS[*pixel as usize];
        d.rect(
            100. + (i % 16) as f32 * 9.,
            147. + (i / 16) as f32 * 9.,
            8.,
            8.,
            Color::srgb_u8(col[0], col[1], col[2]),
        );
    }
    d.button(
        264.,
        147.,
        185.,
        22.,
        &format!("{} >", LABELS[ed.part]),
        Action::Skin("part".into(), 0),
    );
    d.button(
        264.,
        175.,
        185.,
        22.,
        "Fill this part",
        Action::Skin("fill".into(), 0),
    );
    // Fit the six-part preview between the controls and footer.
    for (part, p, size) in [
        ("body", Vec2::new(350., 260.), Vec2::new(55., 73.)),
        ("rLeg", Vec2::new(339., 285.), Vec2::new(22., 22.)),
        ("lLeg", Vec2::new(361., 285.), Vec2::new(24., 22.)),
        ("rArm", Vec2::new(328., 253.), Vec2::new(22., 22.)),
        ("lArm", Vec2::new(372., 253.), Vec2::new(22., 22.)),
        ("head", Vec2::new(350., 227.), Vec2::new(54., 79.)),
    ] {
        if let Some(h) = d.art.skins.get(&format!("draft_{part}")) {
            d.image(p, size * 0.65, h.clone(), Color::WHITE);
        }
    }
    let saved = g.save.custom_skins.get(ed.browse);
    d.button(
        98.,
        301.,
        176.,
        22.,
        &saved.map_or("Saved: none".into(), |s| format!("Saved: {} >", s.name)),
        Action::Skin("browse".into(), 0),
    );
    d.button(280., 301., 80., 22., "Load", Action::Skin("load".into(), 0));
    d.button(
        366.,
        301.,
        83.,
        22.,
        "Delete",
        Action::Skin("delete".into(), 0),
    );
    d.text(
        98.,
        330.,
        if g.editing_preset {
            "Type name; Enter finishes. Gameplay shortcuts are disabled."
        } else {
            "Drag to paint. Fill + part switch. Original sizes and silhouette kept."
        },
        9.,
        ui::MUTED,
    );
}
#[cfg(test)]
mod tests {
    use super::*;
    fn game() -> Game {
        let mut g = Game::new();
        g.demo = true;
        g.save.custom_skins.clear();
        g.skin_editor = Editor::default();
        g
    }
    #[test]
    fn save_edit_delete_and_presets() {
        let mut g = game();
        g.skin_action("save", 0);
        let id = g.save.skin.clone();
        assert!(id.starts_with("custom_"));
        assert_eq!(g.save.custom_skins.len(), 1);
        g.skin_editor.skin.name = "New name".into();
        g.skin_action("save", 0);
        assert_eq!(g.save.skin, id);
        assert_eq!(g.save.custom_skins.len(), 1);
        g.preset_name = "Custom preset".into();
        g.gameplay_action(Action::SavePreset);
        g.save.skin = "default".into();
        g.gameplay_action(Action::LoadPreset(0));
        assert_eq!(g.save.skin, id);
        g.skin_action("delete", 0);
        assert_eq!(g.save.skin, "default");
        assert_eq!(g.save.presets[0].skin, "default");
        assert!(g.save.custom_skins.is_empty());
    }
    #[test]
    fn boundary_id_allocates_a_valid_unused_id() {
        let mut g = game();
        let mut old = g.skin_editor.skin.clone();
        old.id = u64::MAX - 1;
        old.name = "Boundary".into();
        g.save.custom_skins.push(old);
        g.skin_action("save", 0);
        assert_eq!(g.skin_editor.skin.id, 1);
        let mut roundtrip: Vec<CustomSkin> =
            serde_json::from_str(&serde_json::to_string(&g.save.custom_skins).unwrap()).unwrap();
        validate(&mut roundtrip);
        assert_eq!(roundtrip.len(), 2);
    }
    #[test]
    fn draft_is_not_saved_until_save() {
        let mut g = game();
        g.skin_action("fill", 0);
        assert!(g.save.custom_skins.is_empty());
        g.skin_action("save", 0);
        let before = g.save.custom_skins[0].pixels.clone();
        g.skin_action("brush", 10);
        g.skin_action("fill", 0);
        assert_eq!(g.save.custom_skins[0].pixels, before);
    }
    #[test]
    fn recoloring_preserves_dimensions_and_alpha() {
        use bevy::{
            asset::RenderAssetUsages,
            render::render_resource::{Extent3d, TextureDimension, TextureFormat},
        };
        let image = Image::new_fill(
            Extent3d {
                width: 54,
                height: 79,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[255, 255, 255, 127],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );
        let out = recolor(&image, &vec![4; 256]).unwrap();
        assert_eq!(out.texture_descriptor.size, image.texture_descriptor.size);
        for p in out.data.unwrap().chunks_exact(4) {
            assert_eq!(p[3], 127);
            assert_eq!(&p[..3], &COLORS[4]);
        }
    }
    #[test]
    fn corrupt_shapes_are_bounded() {
        let mut s = CustomSkin {
            id: 1,
            name: "x".repeat(100),
            pixels: vec![vec![255; 10000]; 10],
        };
        s.validate();
        assert_eq!(s.name.len(), 20);
        assert_eq!(s.pixels.len(), 6);
        assert!(
            s.pixels
                .iter()
                .all(|p| p.len() == 256 && p.iter().all(|c| *c < 16))
        );
    }
}
