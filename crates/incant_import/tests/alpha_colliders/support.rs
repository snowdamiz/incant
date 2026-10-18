use image::{Rgba, RgbaImage};
use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::{Entity, Project, Scene, SpriteRenderer, Transform};
use incant_import::{
    AlphaColliderError, AlphaColliderRequest, ImportRequest, ImportSnapshot, PreparedAlphaColliders,
};
use serde_json::{Value, json};
use std::fs;

pub const FRAME: &str = "00000000000000000000000001";
pub struct Fixture {
    pub root: tempfile::TempDir,
    pub bus: CommandBus,
    pub scene: String,
    pub entity: String,
    pub asset: String,
}
impl Fixture {
    pub fn new(width: u32, alpha: &[u8], atlas: bool) -> Self {
        assert_eq!(alpha.len() % width as usize, 0);
        let height = (alpha.len() / width as usize) as u32;
        let root = tempfile::tempdir().unwrap();
        let mut image = RgbaImage::new(width, height);
        for (pixel, &a) in image.pixels_mut().zip(alpha) {
            *pixel = Rgba([20, 40, 60, a]);
        }
        image.save(root.path().join("mask.png")).unwrap();
        let source = if atlas {
            RgbaImage::from_pixel(4, 4, Rgba([255; 4]))
                .save(root.path().join("other.png"))
                .unwrap();
            fs::write(root.path().join("sheet.atlas.json"), json!({
                "version":1,"id":"00000000000000000000000009","maximum_size":[64,64],"padding":2,
                "frames":{(FRAME):{"name":"Chosen", "path":"mask.png"},
                    "00000000000000000000000002":{"name":"Other", "path":"other.png"}}
            }).to_string()).unwrap();
            "sheet.atlas.json"
        } else {
            "mask.png"
        };
        let project = Project::empty("Alpha collider tests");
        let path = root.path().join("test.incant");
        fs::write(&path, project.canonical_text().unwrap()).unwrap();
        let mut bus = CommandBus::open_project_file(&path).unwrap();
        let result = ImportSnapshot::capture(&bus)
            .prepare(
                root.path(),
                None,
                &[ImportRequest {
                    source: source.into(),
                    texture_usage: None,
                }],
            )
            .unwrap()
            .commit(&mut bus, Actor::import("test"), "Import alpha texture")
            .unwrap();
        let asset = result.imports[0].asset.id.clone();
        let scene = Scene::new("Main");
        let scene_id = scene.id.clone();
        let mut entity = Entity::new("Source sprite");
        let entity_id = entity.id.clone();
        entity
            .components
            .insert("Transform".into(), json!(Transform::default()));
        let region = if atlas {
            json!({"frame":FRAME})
        } else {
            json!([0, 0, width, height])
        };
        entity.components.insert(
            "SpriteRenderer".into(),
            json!({
                "texture":asset,"region":region,"size":[width,height],"pivot":[0,0],"tint":[1,1,1,1]
            }),
        );
        bus.execute(
            vec![
                Command::CreateScene { scene },
                Command::CreateEntity {
                    scene_id: scene_id.clone(),
                    entity,
                },
            ],
            Actor::user("test"),
            "Author source sprite",
            None,
        )
        .unwrap();
        Self {
            root,
            bus,
            scene: scene_id,
            entity: entity_id,
            asset,
        }
    }
    pub fn request(&self, threshold: u8) -> AlphaColliderRequest {
        AlphaColliderRequest {
            scene_id: self.scene.clone(),
            entity_id: self.entity.clone(),
            threshold,
        }
    }
    pub fn prepare(&self, threshold: u8) -> Result<PreparedAlphaColliders, AlphaColliderError> {
        ImportSnapshot::capture(&self.bus).prepare_alpha_colliders(
            self.root.path(),
            None,
            &self.request(threshold),
        )
    }
    pub fn component(&mut self, kind: &str, value: Value) {
        self.bus
            .execute(
                vec![Command::SetComponent {
                    scene_id: self.scene.clone(),
                    entity_id: self.entity.clone(),
                    component: kind.into(),
                    value,
                }],
                Actor::user("test"),
                "Change source",
                None,
            )
            .unwrap();
    }
    pub fn sprite(&self) -> SpriteRenderer {
        serde_json::from_value(self.bus.project().scenes[&self.scene].entities[&self.entity].components["SpriteRenderer"].clone()).unwrap()
    }
    pub fn reopen(&mut self) {
        let path = self.root.path().join("test.incant");
        drop(std::mem::replace(
            &mut self.bus,
            CommandBus::new(Project::empty("placeholder")).unwrap(),
        ));
        self.bus = CommandBus::open_project_file(path).unwrap();
    }
}

pub fn assert_exact_cover(
    prepared: &PreparedAlphaColliders,
    expected: &[u8],
    width: usize,
    threshold: u8,
) {
    let mut occupancy = vec![0u8; expected.len()];
    for rect in &prepared.derivation().rectangles {
        let [x, y, w, h] = rect.pixels.map(|v| v as usize);
        for row in y..y + h {
            for col in x..x + w {
                occupancy[row * width + col] += 1;
            }
        }
    }
    for (actual, &alpha) in occupancy.iter().zip(expected) {
        assert_eq!(
            *actual,
            u8::from(alpha >= threshold),
            "each selected texel covered exactly once"
        );
    }
    assert_eq!(
        prepared.derivation().solid_pixels,
        expected.iter().filter(|&&a| a >= threshold).count()
    );
}

pub fn entities(prepared: &PreparedAlphaColliders) -> Vec<&Entity> {
    prepared
        .commands()
        .iter()
        .map(|command| match command {
            Command::CreateEntity { entity, .. } => entity,
            _ => panic!("only ordinary entity creation is allowed"),
        })
        .collect()
}
