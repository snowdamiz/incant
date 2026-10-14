#![allow(dead_code)]
use incant_cmd::{Actor, Command, CommandBus, CommandError};
use incant_doc::{Entity, Id, Project, Scene, Transform, new_id};
use serde_json::json;
use std::collections::BTreeMap;
pub struct Fixture {
    pub bus: CommandBus,
    pub scene: Id,
    pub source: Id,
    pub child: Id,
    pub prefab: Id,
    pub nodes: BTreeMap<Id, Id>,
}
impl Fixture {
    pub fn new() -> Self {
        let mut p = Project::empty("Linked rooms");
        let mut s = Scene::new("World");
        let mut root = Entity::new("Mover");
        root.components
            .insert("Transform".into(), json!(Transform::default()));
        root.components
            .insert("Velocity".into(), json!({"linear":[1,0,0]}));
        let mut child = Entity::new("Child");
        child.parent = Some(root.id.clone());
        child
            .components
            .insert("Transform".into(), json!(Transform::default()));
        let source = root.id.clone();
        let cid = child.id.clone();
        s.entities.insert(source.clone(), root);
        s.entities.insert(cid.clone(), child);
        let sid = s.id.clone();
        p.scenes.insert(sid.clone(), s);
        Self {
            bus: CommandBus::new(p).unwrap(),
            scene: sid,
            source: source.clone(),
            child: cid.clone(),
            prefab: new_id(),
            nodes: BTreeMap::from([(source, new_id()), (cid, new_id())]),
        }
    }
    pub fn run(&mut self, commands: Vec<Command>) -> Result<(), CommandError> {
        self.bus
            .execute(
                commands,
                Actor::agent("prefab-test", "conversation"),
                "Prefab edit",
                Some(self.bus.revision()),
            )
            .map(|_| ())
    }
    pub fn create_command(&self) -> Command {
        Command::CreatePrefab {
            scene_id: self.scene.clone(),
            root_entity_id: self.source.clone(),
            prefab_id: self.prefab.clone(),
            name: "Mover subtree".into(),
            nodes: self.nodes.clone(),
        }
    }
    pub fn create(&mut self) {
        self.run(vec![self.create_command()]).unwrap();
    }
    pub fn instance(&mut self) -> BTreeMap<Id, Id> {
        let map = self
            .nodes
            .values()
            .map(|id| (id.clone(), new_id()))
            .collect::<BTreeMap<_, _>>();
        self.run(vec![Command::InstantiatePrefab {
            source: self.prefab.clone(),
            scene_id: self.scene.clone(),
            parent: None,
            entities: map.clone(),
        }])
        .unwrap();
        map
    }
}
