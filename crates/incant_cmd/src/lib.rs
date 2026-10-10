//! The only mutable owner of an editor document. GUI, scripts and AI share commands.
mod journal;
use incant_doc::{
    Asset, CollaborativeDocument, DocumentError, Entity, Id, Origin, Project, Provenance, Scene,
    new_id,
};
pub use journal::Journal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    CreateScene {
        scene: Scene,
    },
    DeleteScene {
        scene_id: Id,
    },
    CreateEntity {
        scene_id: Id,
        entity: Entity,
    },
    DeleteEntity {
        scene_id: Id,
        entity_id: Id,
    },
    RenameEntity {
        scene_id: Id,
        entity_id: Id,
        name: String,
    },
    ReparentEntity {
        scene_id: Id,
        entity_id: Id,
        parent: Option<Id>,
    },
    SetComponent {
        scene_id: Id,
        entity_id: Id,
        component: String,
        value: Value,
    },
    RemoveComponent {
        scene_id: Id,
        entity_id: Id,
        component: String,
    },
    /// Register or update an imported asset without changing its stable identity.
    UpsertAsset {
        asset: Asset,
    },
    RemoveAsset {
        asset_id: Id,
    },
    SetMemory {
        section: String,
        text: String,
    },
    /// Replace named gameplay bindings through the same undoable transaction.
    SetInputActions {
        actions: incant_doc::InputActions,
    },
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Actor {
    pub origin: Origin,
    pub actor: String,
    pub model: Option<String>,
    pub conversation_id: Option<String>,
}
impl Actor {
    pub fn user(name: impl Into<String>) -> Self {
        Self {
            origin: Origin::User,
            actor: name.into(),
            model: None,
            conversation_id: None,
        }
    }
    pub fn agent(model: impl Into<String>, conversation_id: impl Into<String>) -> Self {
        let model = model.into();
        Self {
            origin: Origin::Agent,
            actor: "incant-agent".into(),
            model: Some(model),
            conversation_id: Some(conversation_id.into()),
        }
    }
    pub fn import(name: impl Into<String>) -> Self {
        Self {
            origin: Origin::Import,
            ..Self::user(name)
        }
    }
    fn provenance(&self, transaction: &str) -> Result<Provenance, CommandError> {
        if self.actor.trim().is_empty()
            || (self.origin == Origin::Agent
                && (self.model.as_ref().is_none_or(|s| s.trim().is_empty())
                    || self
                        .conversation_id
                        .as_ref()
                        .is_none_or(|s| s.trim().is_empty())))
        {
            return Err(CommandError::Invalid("actor metadata is incomplete".into()));
        }
        Ok(Provenance {
            origin: self.origin.clone(),
            actor: self.actor.clone(),
            model: self.model.clone(),
            conversation_id: self.conversation_id.clone(),
            transaction: transaction.into(),
        })
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Transaction {
    pub id: Id,
    pub description: String,
    pub actor: Actor,
    pub commands: Vec<Command>,
    pub before: Project,
    pub after: Project,
}
#[derive(Debug, Error)]
pub enum CommandError {
    #[error(transparent)]
    Document(#[from] DocumentError),
    #[error("invalid command: {0}")]
    Invalid(String),
    #[error("document revision conflict: expected {expected}, current {actual}")]
    Conflict { expected: u64, actual: u64 },
    #[error("nothing to {0}")]
    Empty(&'static str),
    #[error("journal: {0}")]
    Io(#[from] std::io::Error),
    #[error("journal corrupt: {0}")]
    Corrupt(String),
}

pub struct CommandBus {
    document: Option<CollaborativeDocument>,
    project: Project,
    revision: u64,
    history: Vec<Transaction>,
    redo: Vec<Transaction>,
    journal: Option<Journal>,
}
impl CommandBus {
    pub fn new(project: Project) -> Result<Self, CommandError> {
        Ok(Self {
            document: Some(CollaborativeDocument::new(&project)?),
            project,
            revision: 0,
            history: vec![],
            redo: vec![],
            journal: None,
        })
    }
    pub fn from_crdt(bytes: &[u8]) -> Result<Self, CommandError> {
        let document = CollaborativeDocument::from_export(bytes)?;
        Ok(Self {
            project: document.project()?,
            document: Some(document),
            revision: 0,
            history: vec![],
            redo: vec![],
            journal: None,
        })
    }
    /// Play-state bus uses the same validation and atomic commands without
    /// appending runtime ticks to authored CRDT history. The caller keeps the
    /// immutable authoring bus and discards this projection when play stops.
    pub fn simulation(project: Project) -> Result<Self, CommandError> {
        project.validate()?;
        Ok(Self {
            document: None,
            project,
            revision: 0,
            history: vec![],
            redo: vec![],
            journal: None,
        })
    }
    fn stage(&self, project: &Project) -> Result<Option<CollaborativeDocument>, CommandError> {
        if let Some(document) = &self.document {
            let mut next = document.fork();
            next.replace(project)?;
            Ok(Some(next))
        } else {
            Ok(None)
        }
    }
    pub fn persistent(
        path: impl AsRef<std::path::Path>,
        initial: Project,
    ) -> Result<Self, CommandError> {
        Journal::open(path.as_ref(), initial)
    }
    pub fn project(&self) -> &Project {
        &self.project
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn history(&self) -> &[Transaction] {
        &self.history
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
    pub fn redo_history(&self) -> impl Iterator<Item = &Transaction> {
        self.redo.iter().rev()
    }
    pub fn export_crdt(&self) -> Result<Vec<u8>, CommandError> {
        Ok(self
            .document
            .as_ref()
            .ok_or_else(|| {
                CommandError::Invalid("play-state cannot be shared as authored CRDT edits".into())
            })?
            .export()?)
    }
    pub fn execute(
        &mut self,
        commands: Vec<Command>,
        actor: Actor,
        description: impl Into<String>,
        expected_revision: Option<u64>,
    ) -> Result<&Transaction, CommandError> {
        self.check_revision(expected_revision)?;
        if commands.is_empty() {
            return Err(CommandError::Invalid("empty transaction".into()));
        }
        if commands.len() > 10_000 {
            return Err(CommandError::Invalid(
                "transaction exceeds 10000 commands".into(),
            ));
        }
        let id = new_id();
        let provenance = actor.provenance(&id)?;
        let mut after = self.project.clone();
        for command in &commands {
            apply(&mut after, command, &provenance)?;
        }
        after.validate()?;
        let transaction = Transaction {
            id,
            description: description.into(),
            actor,
            commands,
            before: self.project.clone(),
            after,
        };
        self.commit_transaction(transaction)?;
        Ok(self.history.last().expect("transaction was appended"))
    }
    fn commit_transaction(&mut self, transaction: Transaction) -> Result<(), CommandError> {
        let next = self.stage(&transaction.after)?;
        if let Some(journal) = &mut self.journal {
            journal.append(
                journal::Action::Apply {
                    transaction: Box::new(transaction.clone()),
                },
                &next
                    .as_ref()
                    .expect("persistent authoring document")
                    .export()?,
            )?;
        }
        self.project = transaction.after.clone();
        self.document = next;
        self.revision += 1;
        if self.document.is_none() {
            self.history.clear();
        }
        self.history.push(transaction);
        self.redo.clear();
        Ok(())
    }
    pub fn undo(&mut self) -> Result<Id, CommandError> {
        let tx = self.history.last().ok_or(CommandError::Empty("undo"))?;
        let id = tx.id.clone();
        let project = tx.before.clone();
        let next = self.stage(&project)?;
        if let Some(journal) = &mut self.journal {
            journal.append(
                journal::Action::Undo {
                    transaction_id: id.clone(),
                },
                &next
                    .as_ref()
                    .expect("persistent authoring document")
                    .export()?,
            )?;
        }
        self.project = project;
        self.document = next;
        self.revision += 1;
        self.redo
            .push(self.history.pop().expect("history was checked"));
        Ok(id)
    }
    pub fn redo(&mut self) -> Result<Id, CommandError> {
        let tx = self.redo.last().ok_or(CommandError::Empty("redo"))?;
        let id = tx.id.clone();
        let project = tx.after.clone();
        let next = self.stage(&project)?;
        if let Some(journal) = &mut self.journal {
            journal.append(
                journal::Action::Redo {
                    transaction_id: id.clone(),
                },
                &next
                    .as_ref()
                    .expect("persistent authoring document")
                    .export()?,
            )?;
        }
        self.project = project;
        self.document = next;
        self.revision += 1;
        self.history
            .push(self.redo.pop().expect("redo was checked"));
        Ok(id)
    }
    /// Phase-0 revert deliberately refuses to overwrite edits made after a turn.
    pub fn revert(&mut self, id: &str) -> Result<Id, CommandError> {
        if self.history.last().is_none_or(|tx| tx.id != id) {
            return Err(CommandError::Invalid(
                "only the latest transaction can be reverted; undo later edits first".into(),
            ));
        }
        self.undo()
    }
    /// Remote CRDT integration is itself an atomic, reversible, journaled transaction.
    pub fn merge(&mut self, bytes: &[u8], actor: Actor) -> Result<&Transaction, CommandError> {
        let mut next = self
            .document
            .as_ref()
            .ok_or_else(|| {
                CommandError::Invalid("cannot merge into a play-state projection".into())
            })?
            .fork();
        next.merge(bytes)?;
        let after = next.project()?;
        let id = new_id();
        actor.provenance(&id)?;
        let transaction = Transaction {
            id,
            description: "Merge remote document".into(),
            actor,
            commands: vec![],
            before: self.project.clone(),
            after,
        };
        if let Some(journal) = &mut self.journal {
            journal.append(
                journal::Action::Apply {
                    transaction: Box::new(transaction.clone()),
                },
                &next.export()?,
            )?;
        }
        self.project = transaction.after.clone();
        self.document = Some(next);
        self.revision += 1;
        self.history.push(transaction);
        self.redo.clear();
        Ok(self.history.last().expect("transaction was appended"))
    }
    fn check_revision(&self, expected: Option<u64>) -> Result<(), CommandError> {
        if let Some(expected) = expected
            && expected != self.revision
        {
            return Err(CommandError::Conflict {
                expected,
                actual: self.revision,
            });
        }
        Ok(())
    }
}
fn scene_mut<'a>(project: &'a mut Project, id: &str) -> Result<&'a mut Scene, CommandError> {
    project
        .scenes
        .get_mut(id)
        .ok_or_else(|| CommandError::Invalid(format!("scene {id} does not exist")))
}
fn entity_mut<'a>(
    project: &'a mut Project,
    scene: &str,
    id: &str,
) -> Result<&'a mut Entity, CommandError> {
    scene_mut(project, scene)?
        .entities
        .get_mut(id)
        .ok_or_else(|| CommandError::Invalid(format!("entity {id} does not exist")))
}
fn apply(
    project: &mut Project,
    command: &Command,
    provenance: &Provenance,
) -> Result<(), CommandError> {
    match command {
        Command::UpsertAsset { asset } => {
            project.assets.insert(asset.id.clone(), asset.clone());
        }
        Command::RemoveAsset { asset_id } => {
            if project.assets.remove(asset_id).is_none() {
                return Err(CommandError::Invalid("asset does not exist".into()));
            }
        }
        Command::CreateScene { scene } => {
            if project.scenes.contains_key(&scene.id) {
                return Err(CommandError::Invalid("scene already exists".into()));
            }
            let mut scene = scene.clone();
            for entity in scene.entities.values_mut() {
                entity.provenance = Some(provenance.clone());
            }
            project.scenes.insert(scene.id.clone(), scene);
        }
        Command::DeleteScene { scene_id } => {
            if project.scenes.remove(scene_id).is_none() {
                return Err(CommandError::Invalid("scene does not exist".into()));
            }
        }
        Command::CreateEntity { scene_id, entity } => {
            let scene = scene_mut(project, scene_id)?;
            if scene.entities.contains_key(&entity.id) {
                return Err(CommandError::Invalid("entity already exists".into()));
            }
            let mut entity = entity.clone();
            entity.provenance = Some(provenance.clone());
            scene.entities.insert(entity.id.clone(), entity);
        }
        Command::DeleteEntity {
            scene_id,
            entity_id,
        } => {
            let scene = scene_mut(project, scene_id)?;
            if scene
                .entities
                .values()
                .any(|e| e.parent.as_ref() == Some(entity_id))
            {
                return Err(CommandError::Invalid(
                    "reparent or delete children before deleting their parent".into(),
                ));
            }
            if scene.entities.remove(entity_id).is_none() {
                return Err(CommandError::Invalid("entity does not exist".into()));
            }
        }
        Command::RenameEntity {
            scene_id,
            entity_id,
            name,
        } => {
            let entity = entity_mut(project, scene_id, entity_id)?;
            entity.name = name.clone();
            entity.provenance = Some(provenance.clone());
        }
        Command::ReparentEntity {
            scene_id,
            entity_id,
            parent,
        } => {
            let entity = entity_mut(project, scene_id, entity_id)?;
            entity.parent = parent.clone();
            entity.provenance = Some(provenance.clone());
        }
        Command::SetComponent {
            scene_id,
            entity_id,
            component,
            value,
        } => {
            let entity = entity_mut(project, scene_id, entity_id)?;
            entity.components.insert(component.clone(), value.clone());
            entity.provenance = Some(provenance.clone());
        }
        Command::RemoveComponent {
            scene_id,
            entity_id,
            component,
        } => {
            let entity = entity_mut(project, scene_id, entity_id)?;
            if entity.components.remove(component).is_none() {
                return Err(CommandError::Invalid("component does not exist".into()));
            }
            entity.provenance = Some(provenance.clone());
        }
        Command::SetMemory { section, text } => {
            if section.trim().is_empty() {
                return Err(CommandError::Invalid(
                    "memory section cannot be empty".into(),
                ));
            }
            project.memory.insert(section.clone(), text.clone());
        }
        Command::SetInputActions { actions } => {
            project.settings.input_actions = actions.clone();
        }
    }
    Ok(())
}
