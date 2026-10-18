use image::{Rgba, RgbaImage};
use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::{Entity, Project, Scene, Tilemap, Transform};
use incant_import::{
    ImportRequest, ImportSnapshot, PreparedTilemapPaint, TilemapBrush, TilemapPaintError,
    TilemapPaintRequest,
};
use serde_json::json;
use std::{collections::BTreeMap, fs};

pub fn frame(index: usize) -> String {
    format!("{:026X}", index + 1)
}
pub fn rules() -> Box<[String; 16]> {
    Box::new(std::array::from_fn(frame))
}

pub struct Fixture {
    pub root: tempfile::TempDir,
    pub bus: CommandBus,
    pub scene: String,
    pub entity: String,
    pub layer: String,
    pub texture: String,
}
impl Fixture {
    pub fn new(size: [u32; 2]) -> Self {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("game.incant");
        fs::write(&path, Project::empty("Paint").canonical_text().unwrap()).unwrap();
        RgbaImage::from_pixel(1, 1, Rgba([255u8, 0, 0, 255]))
            .save(root.path().join("tile.png"))
            .unwrap();
        let frames: BTreeMap<_, _> = (0..17)
            .map(|n| (frame(n), json!({"name":format!("F{n}"),"path":"tile.png"})))
            .collect();
        fs::write(root.path().join("tiles.atlas.json"), json!({"version":1,"id":incant_doc::new_id(),"maximum_size":[64,64],"padding":1,"frames":frames}).to_string()).unwrap();
        let mut bus =
            CommandBus::open_project_file_with_journal(&path, path.with_extension("journal.jsonl"))
                .unwrap();
        let imported = ImportSnapshot::capture(&bus)
            .prepare(
                root.path(),
                None,
                &[ImportRequest {
                    source: "tiles.atlas.json".into(),
                    texture_usage: None,
                }],
            )
            .unwrap()
            .commit(&mut bus, Actor::import("fixture"), "Atlas")
            .unwrap();
        let texture = imported.imports[0].asset.id.clone();
        let scene = Scene::new("Main");
        let sid = scene.id.clone();
        let mut entity = Entity::new("Map");
        let eid = entity.id.clone();
        let layer = incant_doc::new_id();
        entity.components.insert(
            "Transform".into(),
            json!(Transform {
                translation: [7., -3., 0.],
                rotation: [0., 0., 1., 0.],
                ..Default::default()
            }),
        );
        let area = (size[0] * size[1]) as usize;
        entity.components.insert("Tilemap".into(), json!({"tileset":texture,"size":size,"cell_size":[2.,0.5],"layers":[
            {"id":layer,"cells":vec![None::<String>;area],"solid":true,"layer":2,"order":-3},
            {"id":incant_doc::new_id(),"cells":vec![Some(frame(16));area],"solid":false,"layer":4,"order":7}]}));
        bus.execute(
            vec![
                Command::CreateScene { scene },
                Command::CreateEntity {
                    scene_id: sid.clone(),
                    entity,
                },
            ],
            Actor::user("fixture"),
            "Map",
            None,
        )
        .unwrap();
        Self {
            root,
            bus,
            scene: sid,
            entity: eid,
            layer,
            texture,
        }
    }
    pub fn request(&self, cells: Vec<[u32; 2]>, brush: TilemapBrush) -> TilemapPaintRequest {
        TilemapPaintRequest {
            scene_id: self.scene.clone(),
            entity_id: self.entity.clone(),
            layer_id: self.layer.clone(),
            expected_revision: self.bus.revision(),
            cells,
            brush,
        }
    }
    pub fn prepare(
        &self,
        request: &TilemapPaintRequest,
    ) -> Result<PreparedTilemapPaint, TilemapPaintError> {
        ImportSnapshot::capture(&self.bus).prepare_tilemap_paint(self.root.path(), None, request)
    }
    pub fn map(&self) -> Tilemap {
        serde_json::from_value(
            self.bus.project().scenes[&self.scene].entities[&self.entity].components["Tilemap"]
                .clone(),
        )
        .unwrap()
    }
    pub fn set(&mut self, cells: Vec<Option<String>>) {
        let mut map = self.map();
        map.layers[0].cells = cells;
        self.bus
            .execute(
                vec![Command::SetComponent {
                    scene_id: self.scene.clone(),
                    entity_id: self.entity.clone(),
                    component: "Tilemap".into(),
                    value: json!(map),
                }],
                Actor::user("fixture"),
                "Cells",
                None,
            )
            .unwrap();
    }
    pub fn reopen(&mut self) {
        let path = self.root.path().join("game.incant");
        let old = std::mem::replace(
            &mut self.bus,
            CommandBus::new(Project::empty("release")).unwrap(),
        );
        drop(old);
        self.bus =
            CommandBus::open_project_file_with_journal(&path, path.with_extension("journal.jsonl"))
                .unwrap();
    }
}
