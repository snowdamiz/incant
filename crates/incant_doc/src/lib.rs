//! The text-native project format and validated CRDT projection. No filesystem or UI.
#[cfg(feature = "crdt")]
mod crdt;
#[cfg(feature = "crdt")]
pub use crdt::CollaborativeDocument;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

pub const SCHEMA_VERSION: u32 = 1;
pub type Id = String;
pub fn new_id() -> Id {
    ulid::Ulid::new().to_string()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub id: Id,
    pub schema_version: u32,
    pub name: String,
    pub scenes: BTreeMap<Id, Scene>,
    pub assets: BTreeMap<Id, Asset>,
    pub scripts: BTreeMap<Id, ScriptSource>,
    pub settings: ProjectSettings,
    pub memory: BTreeMap<String, String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProjectSettings {
    pub tick_rate: u32,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Scene {
    pub id: Id,
    pub name: String,
    pub entities: BTreeMap<Id, Entity>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Entity {
    pub id: Id,
    pub name: String,
    pub parent: Option<Id>,
    pub components: BTreeMap<String, Value>,
    pub provenance: Option<Provenance>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Asset {
    pub id: Id,
    pub name: String,
    pub path: String,
    pub kind: String,
    pub sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub import_settings: Option<AssetImportSettings>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum AssetImportSettings {
    Texture { usage: TextureUsage },
}
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum TextureUsage {
    Color,
    Linear,
    Normal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ScriptSource {
    pub id: Id,
    pub name: String,
    pub path: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    pub origin: Origin,
    pub actor: String,
    pub model: Option<String>,
    pub conversation_id: Option<String>,
    pub transaction: Id,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    User,
    Agent,
    Script,
    Import,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Transform {
    pub translation: [f64; 3],
    pub rotation: [f64; 4],
    pub scale: [f64; 3],
}
impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: [0.; 3],
            rotation: [0., 0., 0., 1.],
            scale: [1.; 3],
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Velocity {
    pub linear: [f64; 3],
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ScriptComponent {
    pub source: Id,
    pub props: BTreeMap<String, Value>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MeshRenderer {
    pub mesh: Id,
    pub materials: Vec<Id>,
    pub cast_shadows: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Camera {
    pub fov_degrees: f64,
    pub near: f64,
    pub far: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Diagnostic {
    pub path: String,
    pub message: String,
}
#[derive(Debug, Error)]
pub enum DocumentError {
    #[error("invalid project JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("project validation failed: {0:?}")]
    Validation(Vec<Diagnostic>),
    #[error("CRDT operation failed: {0}")]
    Crdt(String),
    #[error("unsupported schema version {0}")]
    Version(u32),
}

impl Project {
    pub fn empty(name: impl Into<String>) -> Self {
        Self {
            id: new_id(),
            schema_version: SCHEMA_VERSION,
            name: name.into(),
            scenes: BTreeMap::new(),
            assets: BTreeMap::new(),
            scripts: BTreeMap::new(),
            settings: ProjectSettings { tick_rate: 60 },
            memory: BTreeMap::new(),
        }
    }
    pub fn from_text(text: &str) -> Result<Self, DocumentError> {
        let value: Value = serde_json::from_str(text)?;
        let version = value
            .get("schema_version")
            .and_then(Value::as_u64)
            .ok_or_else(|| {
                DocumentError::Validation(vec![Diagnostic {
                    path: "/schema_version".into(),
                    message: "required unsigned schema version".into(),
                }])
            })?;
        if version != SCHEMA_VERSION as u64 {
            return Err(DocumentError::Version(
                version.try_into().unwrap_or(u32::MAX),
            ));
        }
        let project: Self = serde_json::from_value(value)?;
        project.validate()?;
        Ok(project)
    }
    pub fn canonical_text(&self) -> Result<String, DocumentError> {
        self.validate()?;
        // serde_json's default map representation and BTreeMap keep every object sorted.
        Ok(serde_json::to_string_pretty(&serde_json::to_value(self)?)? + "\n")
    }
    pub fn validate(&self) -> Result<(), DocumentError> {
        let errors = self.diagnostics();
        if errors.is_empty() {
            Ok(())
        } else {
            Err(DocumentError::Validation(errors))
        }
    }
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        let mut errors = vec![];
        let mut ids = BTreeSet::new();
        let mut issue = |path: String, message: &str| {
            errors.push(Diagnostic {
                path,
                message: message.into(),
            })
        };
        let mut check_id = |id: &str, path: String| {
            if ulid::Ulid::from_string(id).is_err() {
                issue(path.clone(), "invalid ULID");
            }
            if !ids.insert(id.to_owned()) {
                issue(path, "ID must be globally unique");
            }
        };
        check_id(&self.id, "/id".into());
        for (sid, scene) in &self.scenes {
            check_id(sid, format!("/scenes/{sid}/id"));
            for eid in scene.entities.keys() {
                check_id(eid, format!("/scenes/{sid}/entities/{eid}/id"));
            }
        }
        for aid in self.assets.keys() {
            check_id(aid, format!("/assets/{aid}/id"));
        }
        for sid in self.scripts.keys() {
            check_id(sid, format!("/scripts/{sid}/id"));
        }
        if self.schema_version != SCHEMA_VERSION {
            issue("/schema_version".into(), "unsupported schema version");
        }
        if self.name.trim().is_empty() {
            issue("/name".into(), "name cannot be empty");
        }
        if !(1..=240).contains(&self.settings.tick_rate) {
            issue("/settings/tick_rate".into(), "must be between 1 and 240");
        }
        for (id, asset) in &self.assets {
            if id != &asset.id {
                issue(format!("/assets/{id}/id"), "map key and ID differ");
            }
            if asset.name.trim().is_empty() || asset.kind.trim().is_empty() {
                issue(
                    format!("/assets/{id}"),
                    "asset name and kind cannot be empty",
                );
            }
            if matches!(
                asset.import_settings,
                Some(AssetImportSettings::Texture { .. })
            ) && asset.kind != "texture"
            {
                issue(
                    format!("/assets/{id}/import_settings"),
                    "texture import settings require a texture asset",
                );
            }
            if !safe_relative_path(&asset.path) {
                issue(
                    format!("/assets/{id}/path"),
                    "expected a portable relative path without traversal",
                );
            }
            if asset.sha256.len() != 64 || !asset.sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
                issue(
                    format!("/assets/{id}/sha256"),
                    "expected SHA-256 hex digest",
                );
            }
        }
        for (id, script) in &self.scripts {
            if id != &script.id {
                issue(format!("/scripts/{id}/id"), "map key and ID differ");
            }
            if !safe_relative_path(&script.path) {
                issue(
                    format!("/scripts/{id}/path"),
                    "expected a portable relative path without traversal",
                );
            }
        }
        for (sid, scene) in &self.scenes {
            let path = format!("/scenes/{sid}");
            if sid != &scene.id {
                issue(format!("{path}/id"), "map key and ID differ");
            }
            if scene.name.trim().is_empty() {
                issue(format!("{path}/name"), "name cannot be empty");
            }
            for (eid, entity) in &scene.entities {
                let path = format!("{path}/entities/{eid}");
                if eid != &entity.id {
                    issue(format!("{path}/id"), "map key and ID differ");
                }
                if entity.name.trim().is_empty() {
                    issue(format!("{path}/name"), "name cannot be empty");
                }
                let mut seen = BTreeSet::from([eid.as_str()]);
                let mut parent = entity.parent.as_deref();
                while let Some(id) = parent {
                    if !seen.insert(id) {
                        issue(format!("{path}/parent"), "hierarchy cycle");
                        break;
                    }
                    let Some(p) = scene.entities.get(id) else {
                        issue(format!("{path}/parent"), "parent is not in this scene");
                        break;
                    };
                    parent = p.parent.as_deref();
                }
                if let Some(prov) = &entity.provenance {
                    if ulid::Ulid::from_string(&prov.transaction).is_err() {
                        issue(
                            format!("{path}/provenance/transaction"),
                            "invalid transaction ULID",
                        );
                    }
                    if prov.actor.trim().is_empty() {
                        issue(format!("{path}/provenance/actor"), "actor cannot be empty");
                    }
                    if prov.origin == Origin::Agent
                        && (prov.model.as_ref().is_none_or(|s| s.is_empty())
                            || prov.conversation_id.as_ref().is_none_or(|s| s.is_empty()))
                    {
                        issue(
                            format!("{path}/provenance"),
                            "agent requires model and conversation ID",
                        );
                    }
                }
                for (kind, value) in &entity.components {
                    let path = format!("{path}/components/{kind}");
                    if let Err(message) = validate_component(kind, value, self) {
                        issue(path, &message);
                    }
                }
            }
        }
        errors
    }
}

pub fn safe_relative_path(path: &str) -> bool {
    !path.is_empty()
        && !path.contains(['\\', ':', '\0'])
        && !path.starts_with('/')
        && path
            .split('/')
            .all(|s| !s.is_empty() && s != ".." && s != ".")
}
fn validate_component(kind: &str, value: &Value, project: &Project) -> Result<(), String> {
    fn decode<T: serde::de::DeserializeOwned>(v: &Value) -> Result<T, String> {
        serde_json::from_value(v.clone()).map_err(|e| e.to_string())
    }
    match kind {
        "Transform" => {
            let t: Transform = decode(value)?;
            if !t
                .translation
                .iter()
                .chain(&t.rotation)
                .chain(&t.scale)
                .all(|v| v.is_finite())
            {
                return Err("transform must contain finite numbers".into());
            }
            if t.scale.iter().any(|v| v.abs() < 1e-9) {
                return Err("scale must be nonzero".into());
            }
            let norm: f64 = t.rotation.iter().map(|v| v * v).sum();
            if (norm - 1.).abs() > 1e-4 {
                return Err("rotation must be a unit quaternion".into());
            }
        }
        "Velocity" => {
            let v: Velocity = decode(value)?;
            if !v.linear.iter().all(|v| v.is_finite()) {
                return Err("velocity must be finite".into());
            }
        }
        "Script" => {
            let s: ScriptComponent = decode(value)?;
            if !project.scripts.contains_key(&s.source) {
                return Err("script source does not exist".into());
            }
        }
        "MeshRenderer" => {
            let m: MeshRenderer = decode(value)?;
            if !project.assets.contains_key(&m.mesh)
                || m.materials
                    .iter()
                    .any(|id| !project.assets.contains_key(id))
            {
                return Err("asset reference does not exist".into());
            }
        }
        "Camera" => {
            let c: Camera = decode(value)?;
            if !(0.1..179.).contains(&c.fov_degrees)
                || c.near <= 0.
                || c.far <= c.near
                || !c.far.is_finite()
            {
                return Err("invalid camera projection".into());
            }
        }
        _ => return Err(format!("unregistered component type: {kind}")),
    }
    Ok(())
}
pub fn schema_registry() -> BTreeMap<String, Value> {
    let mut registry = BTreeMap::from([
        ("Project".into(), json!(schemars::schema_for!(Project))),
        ("Transform".into(), json!(schemars::schema_for!(Transform))),
        ("Velocity".into(), json!(schemars::schema_for!(Velocity))),
        (
            "Script".into(),
            json!(schemars::schema_for!(ScriptComponent)),
        ),
        (
            "MeshRenderer".into(),
            json!(schemars::schema_for!(MeshRenderer)),
        ),
        ("Camera".into(), json!(schemars::schema_for!(Camera))),
    ]);
    // Shared inspector annotations. All clients receive the same field order;
    // schema properties remain the source of validation and generated types.
    for (name, order) in [
        ("Transform", vec!["translation", "rotation", "scale"]),
        ("Camera", vec!["fov_degrees", "near", "far"]),
        ("MeshRenderer", vec!["mesh", "materials", "cast_shadows"]),
        ("Script", vec!["source", "props"]),
    ] {
        if let Some(schema) = registry.get_mut(name) {
            schema["order"] = json!(order);
        }
    }
    registry
}
impl Scene {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: new_id(),
            name: name.into(),
            entities: BTreeMap::new(),
        }
    }
}
impl Entity {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: new_id(),
            name: name.into(),
            parent: None,
            components: BTreeMap::new(),
            provenance: None,
        }
    }
}
