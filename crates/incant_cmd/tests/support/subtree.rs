#![allow(dead_code)]
use incant_cmd::{Actor, Command, CommandBus, CommandError};
use incant_doc::{Entity, Id, Project, Scene, Transform, new_id};
use serde_json::json;
use std::collections::BTreeMap;

pub struct Fixture {
    pub bus: CommandBus,
    pub scene: Id,
    pub parent: Id,
    pub root: Id,
    pub child: Id,
    pub leaf: Id,
    pub outside: Id,
    pub mapping: BTreeMap<Id, Id>,
}
impl Fixture {
    pub fn new() -> Self {
        let mut project = Project::empty("Subtree copies");
        let mut scene = Scene::new("World");
        let parent = Entity::new("Parent");
        let mut root = Entity::new("Mover");
        root.parent = Some(parent.id.clone());
        root.components
            .insert("Transform".into(), json!(Transform::default()));
        root.components
            .insert("Velocity".into(), json!({"linear":[1,0,0]}));
        let mut child = Entity::new("Child");
        child.parent = Some(root.id.clone());
        child
            .components
            .insert("Transform".into(), json!(Transform::default()));
        let mut leaf = Entity::new("Leaf");
        leaf.parent = Some(child.id.clone());
        leaf.components
            .insert("Transform".into(), json!(Transform::default()));
        let outside = Entity::new("Unrelated sibling");
        let ids = [
            parent.id.clone(),
            root.id.clone(),
            child.id.clone(),
            leaf.id.clone(),
            outside.id.clone(),
        ];
        let mapping = [&root, &child, &leaf]
            .into_iter()
            .map(|e| (e.id.clone(), new_id()))
            .collect();
        for entity in [parent, root, child, leaf, outside] {
            scene.entities.insert(entity.id.clone(), entity);
        }
        let sid = scene.id.clone();
        project.scenes.insert(sid.clone(), scene);
        let [parent, root, child, leaf, outside] = ids;
        Self {
            bus: CommandBus::new(project).unwrap(),
            scene: sid,
            parent,
            root,
            child,
            leaf,
            outside,
            mapping,
        }
    }
    pub fn command(&self) -> Command {
        self.with_map(self.mapping.clone())
    }
    pub fn with_map(&self, entities: BTreeMap<Id, Id>) -> Command {
        Command::DuplicateSubtree {
            scene_id: self.scene.clone(),
            root_entity_id: self.root.clone(),
            entities,
        }
    }
    pub fn run(&mut self, commands: Vec<Command>) -> Result<(), CommandError> {
        self.bus
            .execute(
                commands,
                Actor::agent("subtree-test", "conversation"),
                "Duplicate subtree",
                Some(self.bus.revision()),
            )
            .map(|_| ())
    }
    pub fn set(&self, id: &str, component: &str, value: serde_json::Value) -> Command {
        Command::SetComponent {
            scene_id: self.scene.clone(),
            entity_id: id.into(),
            component: component.into(),
            value,
        }
    }
}
