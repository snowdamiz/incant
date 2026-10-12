//! Checked authoring scene + source catalog + attachment package. The ordinary
//! scene-only cook still rejects Script instead of silently dropping gameplay.
use crate::{CookError, scene::cook_validated_scene_with_resources, stable_id};
use incant_doc::Project;
use incant_runtime::CookedScene;
use incant_script::{CompiledScript, ScriptBindings, ScriptError, ScriptInstance, ScriptValue};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, thiserror::Error)]
pub enum ProjectScriptError {
    #[error(transparent)]
    Cook(#[from] CookError),
    #[error(transparent)]
    Script(#[from] ScriptError),
    #[error("native authored script cook: {0}")]
    Invalid(&'static str),
}

/// Prepared from one immutable validated authoring document. Only compilation
/// consumes the package into a native scene/program pair.
pub struct PreparedScriptScene {
    scene: CookedScene,
    bindings: ScriptBindings,
    catalog: Value,
}
pub struct CookedProjectProgram {
    scene: CookedScene,
    script: CompiledScript,
}
impl PreparedScriptScene {
    /// Data for the fixed trusted TypeScript cook; not flags or executable paths.
    pub fn source_catalog(&self) -> &Value {
        &self.catalog
    }
    pub fn compile(self, commonjs: &str) -> Result<CookedProjectProgram, ProjectScriptError> {
        let script =
            CompiledScript::compile_project("incant-project-behaviors", commonjs, self.bindings)?;
        Ok(CookedProjectProgram {
            scene: self.scene,
            script,
        })
    }
}
impl CookedProjectProgram {
    pub fn into_parts(self) -> (CookedScene, CompiledScript) {
        (self.scene, self.script)
    }
}

pub fn prepare_script_scene(
    project: &Project,
    scene_id: &str,
) -> Result<PreparedScriptScene, ProjectScriptError> {
    prepare_script_scene_with_navigation(project, scene_id, &Default::default())
}

/// Bind authored behaviors and explicit checked navigation geometry to the same
/// immutable scene snapshot. Missing/stale geometry refuses before compilation;
/// the default preparation entry point retains that strict refusal.
pub fn prepare_script_scene_with_navigation(
    project: &Project,
    scene_id: &str,
    resources: &incant_nav::NavigationResources,
) -> Result<PreparedScriptScene, ProjectScriptError> {
    prepare_script_scene_with_resources(project, scene_id, resources, &Default::default())
}

/// Bind scripts and asset-checked animation metadata to the same cooked scene.
pub fn prepare_script_scene_with_resources(
    project: &Project,
    scene_id: &str,
    resources: &incant_nav::NavigationResources,
    animations: &crate::AnimationResources,
) -> Result<PreparedScriptScene, ProjectScriptError> {
    let validated = project.validated().map_err(CookError::from)?;
    let scene = project
        .scenes
        .get(scene_id)
        .ok_or_else(|| CookError::MissingScene(scene_id.into()))?;
    let mut attached = BTreeSet::new();
    let mut source_ids = BTreeSet::new();
    let mut instances = Vec::new();
    let mut budget = ConversionBudget::default();
    for entity in scene.entities.values() {
        if let Some(value) = entity.components.get("Script") {
            if instances.len() == incant_script::MAX_SCRIPT_INSTANCES {
                return Err(ScriptError::InputLimit.into());
            }
            // The document schema has already checked these exact field types.
            let source = value["source"]
                .as_str()
                .ok_or(ProjectScriptError::Invalid("source must be an ID"))?;
            let props = value["props"]
                .as_object()
                .ok_or(ProjectScriptError::Invalid("props must be an object"))?;
            source_ids.insert(source.to_owned());
            if source_ids.len() > incant_script::MAX_SCRIPT_SOURCES {
                return Err(ScriptError::InputLimit.into());
            }
            let props = props
                .iter()
                .map(|(key, value)| {
                    budget.text(key)?;
                    Ok::<_, ProjectScriptError>((key.clone(), budget.value(value, 1)?))
                })
                .collect::<Result<_, _>>()?;
            attached.insert(entity.id.clone());
            instances.push(ScriptInstance {
                entity: stable_id(&entity.id)?,
                source: stable_id(source)?,
                props,
            });
        }
    }
    if instances.is_empty() {
        return Err(ProjectScriptError::Invalid("scene has no authored scripts"));
    }
    // Documents accept case-insensitive ULID spellings. Preserve their keys for
    // lookup, but use canonical native identity/order at the compiler boundary.
    let mut sources = source_ids
        .iter()
        .map(|text| Ok((stable_id(text)?, text)))
        .collect::<Result<Vec<_>, CookError>>()?;
    sources.sort_by_key(|(id, _)| *id);
    let mut paths = BTreeSet::new();
    let catalog = sources
        .iter()
        .map(|(id, key)| {
            let source = project
                .scripts
                .get(*key)
                .ok_or(ProjectScriptError::Invalid("missing script source"))?;
            if source.path.len() > 1024
                || !source.path.ends_with(".ts")
                || source.path.ends_with(".d.ts")
                || source.path.contains(['\\', ':'])
                || source.path.chars().any(char::is_control)
                || source
                    .path
                    .split('/')
                    .any(|part| part.is_empty() || part == "." || part == "..")
            {
                return Err(ProjectScriptError::Invalid(
                    "expected a portable relative .ts source path",
                ));
            }
            if !paths.insert(source.path.clone()) {
                return Err(ProjectScriptError::Invalid(
                    "source IDs alias the same entry path",
                ));
            }
            Ok(json!({"id":ulid::Ulid::from_bytes(id.0).to_string(),"path":source.path}))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let bytes =
        cook_validated_scene_with_resources(validated, scene_id, &attached, resources, animations)?;
    let bindings = ScriptBindings::new(
        Sha256::digest(&bytes).into(),
        sources.iter().map(|(id, _)| *id).collect(),
        instances,
    )?;
    let scene = CookedScene::from_bytes(&bytes).map_err(CookError::from)?;
    Ok(PreparedScriptScene {
        scene,
        bindings,
        catalog: Value::Array(catalog),
    })
}

// Bound conversion before cloning strings/containers from untrusted documents.
// ScriptBindings also validates its public native-data boundary independently.
#[derive(Default)]
struct ConversionBudget {
    nodes: usize,
    bytes: usize,
}
impl ConversionBudget {
    fn text(&mut self, value: &str) -> Result<(), ProjectScriptError> {
        self.bytes = self
            .bytes
            .checked_add(value.len())
            .ok_or(ScriptError::InputLimit)?;
        if self.bytes > 1024 * 1024 {
            return Err(ScriptError::InputLimit.into());
        }
        Ok(())
    }
    fn value(&mut self, value: &Value, depth: usize) -> Result<ScriptValue, ProjectScriptError> {
        self.nodes += 1;
        if self.nodes > 65_536 || depth > 16 {
            return Err(ScriptError::InputLimit.into());
        }
        Ok(match value {
            Value::Null => ScriptValue::Null,
            Value::Bool(value) => ScriptValue::Bool(*value),
            Value::Number(value) => ScriptValue::Number(
                value
                    .as_f64()
                    .filter(|n| n.is_finite())
                    .ok_or(ProjectScriptError::Invalid("props number is not finite"))?,
            ),
            Value::String(value) => {
                self.text(value)?;
                ScriptValue::String(value.clone())
            }
            Value::Array(values) => ScriptValue::Array(
                values
                    .iter()
                    .map(|value| self.value(value, depth + 1))
                    .collect::<Result<_, _>>()?,
            ),
            Value::Object(values) => ScriptValue::Object(
                values
                    .iter()
                    .map(|(key, value)| {
                        self.text(key)?;
                        Ok::<_, ProjectScriptError>((key.clone(), self.value(value, depth + 1)?))
                    })
                    .collect::<Result<BTreeMap<_, _>, _>>()?,
            ),
        })
    }
}
