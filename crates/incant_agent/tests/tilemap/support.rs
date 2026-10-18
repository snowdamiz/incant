use incant_agent::{
    Agent, AgentError, ApprovalMode, Budget, NoViewport, ProjectHost, Usage,
    provider::{Completion, Provider},
};
use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::{Entity, Project, Scene, Tilemap, Transform};
use incant_import::{ImportRequest, ImportSnapshot};
use serde_json::{Value, json};
use std::fs;

pub const TOOL: &str = "tilemap_paint";
pub fn frame(index: usize) -> String {
    format!("{:026X}", index + 1)
}
pub fn rules() -> Value {
    json!((0..16).map(frame).collect::<Vec<_>>())
}
pub struct Fixture {
    pub root: tempfile::TempDir,
    pub bus: CommandBus,
    pub scene: String,
    pub entity: String,
    pub layer: String,
}
impl Fixture {
    pub fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        // Checked 2x2 RGBA PNG: alpha [255, 0; 255, 255], no image dependency.
        fs::write(
            root.path().join("mask.png"),
            [
                137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 2, 0, 0, 0,
                2, 8, 6, 0, 0, 0, 114, 182, 13, 36, 0, 0, 0, 19, 73, 68, 65, 84, 120, 156, 99, 16,
                209, 176, 249, 15, 196, 12, 12, 80, 198, 127, 0, 37, 46, 4, 222, 82, 161, 75, 119,
                0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
            ],
        )
        .unwrap();
        let frames: std::collections::BTreeMap<_, _> = (0..17)
            .map(|n| (frame(n), json!({"name":format!("F{n}"),"path":"mask.png"})))
            .collect();
        fs::write(root.path().join("tiles.atlas.json"), json!({"version":1,"id":incant_doc::new_id(),"maximum_size":[64,64],"padding":1,"frames":frames}).to_string()).unwrap();
        let path = root.path().join("game.incant");
        fs::write(
            &path,
            Project::empty("Agent tilemap").canonical_text().unwrap(),
        )
        .unwrap();
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
            .commit(&mut bus, Actor::import("fixture"), "Import atlas")
            .unwrap();
        let scene = Scene::new("Main");
        let scene_id = scene.id.clone();
        let mut entity = Entity::new("Map");
        let entity_id = entity.id.clone();
        entity
            .components
            .insert("Transform".into(), json!(Transform::default()));
        let layer = incant_doc::new_id();
        entity.components.insert(
            "Tilemap".into(),
            json!({
            "tileset":imported.imports[0].asset.id,"size":[3,2],"cell_size":[2.,0.5],"layers":[
            {"id":layer,"cells":vec![None::<String>;6],"solid":true},
            {"id":incant_doc::new_id(),"cells":vec![Some(frame(16));6],"solid":false}]}),
        );
        bus.execute(
            vec![
                Command::CreateScene { scene },
                Command::CreateEntity {
                    scene_id: scene_id.clone(),
                    entity,
                },
            ],
            Actor::user("fixture"),
            "Author map",
            None,
        )
        .unwrap();
        Self {
            root,
            bus,
            scene: scene_id,
            entity: entity_id,
            layer,
        }
    }
    pub fn args(&self) -> Value {
        json!({"scene_id":self.scene,"entity_id":self.entity,"layer_id":self.layer,
            "expected_revision":self.bus.revision(),"cells":[[0,0],[1,0]],
            "brush":{"kind":"autotile","action":"paint","frames":rules()}})
    }
    pub fn map(&self) -> Tilemap {
        serde_json::from_value(
            self.bus.project().scenes[&self.scene].entities[&self.entity].components["Tilemap"]
                .clone(),
        )
        .unwrap()
    }
    pub fn reopen(&mut self) {
        let old = std::mem::replace(
            &mut self.bus,
            CommandBus::new(Project::empty("Release")).unwrap(),
        );
        drop(old);
        let path = self.root.path().join("game.incant");
        self.bus =
            CommandBus::open_project_file_with_journal(&path, path.with_extension("journal.jsonl"))
                .unwrap();
    }
    pub fn host(&self) -> ProjectHost<NoViewport> {
        ProjectHost::new(self.bus.project(), self.root.path(), NoViewport).unwrap()
    }
}

pub fn actor() -> Actor {
    Actor::agent("mock-tilemap-model", "tilemap-turn")
}
pub fn agent(approval: ApprovalMode) -> Agent {
    Agent {
        budget: Budget {
            max_tokens: 100_000,
            used_tokens: 0,
        },
        approval,
        max_steps: 3,
        max_output_tokens: 1000,
    }
}
pub struct Mock {
    pub args: Value,
    pub advertised: Vec<bool>,
    pub result: Option<Value>,
    step: usize,
}
impl Mock {
    pub fn new(args: Value) -> Self {
        Self {
            args,
            advertised: vec![],
            result: None,
            step: 0,
        }
    }
}
impl Provider for Mock {
    fn model(&self) -> &str {
        "mock-tilemap-model"
    }
    fn complete(
        &mut self,
        input: &[Value],
        tools: &[Value],
        _: u64,
        _: &mut dyn FnMut(&str),
    ) -> Result<Completion, AgentError> {
        self.advertised
            .push(tools.iter().any(|t| t["name"] == TOOL));
        let output = match self.step {
            0 => vec![
                json!({"type":"function_call", "name":TOOL, "call_id":"tilemap",
                "arguments": self.args.to_string()}),
            ],
            1 => {
                let row = input
                    .iter()
                    .rev()
                    .find(|v| v["type"] == "function_call_output" && v["call_id"] == "tilemap")
                    .unwrap();
                self.result = Some(serde_json::from_str(row["output"].as_str().unwrap()).unwrap());
                vec![]
            }
            _ => panic!("unexpected provider step"),
        };
        self.step += 1;
        Ok(Completion {
            output,
            usage: Usage::default(),
        })
    }
}
