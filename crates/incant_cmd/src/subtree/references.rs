//! Entity edges are distinct from asset, script and component-local identities.
//! See the schema-derived inventory control and docs/spikes/subtree-duplication.md.
use super::invalid;
use crate::CommandError;
use incant_doc::{AudioBus, AudioSource, Id, NavigationMesh};
use serde_json::Value;
use std::collections::BTreeMap;

// Fail closed for a newly registered component until its reference semantics
// have been reviewed. Existing schema edits are covered by the inventory test.
const LITERAL_COMPONENTS: &[&str] = &[
    "AngularVelocity",
    "AngularVelocity2D",
    "Animation",
    "AnimationGraph",
    "AnimationIk",
    "AnimationRootMotion",
    "AudioListener",
    "Camera",
    "Collider",
    "Collider2D",
    "DirectionalLight",
    "EnvironmentLight",
    "MeshRenderer",
    "NavigationGrid",
    "PointLight",
    "RigidBody",
    "RigidBody2D",
    "Script",
    "SpotLight",
    "SpriteFlipbook",
    "SpriteRenderer",
    "Text",
    "Tilemap",
    "Transform",
    "UiNode",
    "Velocity",
];

pub(super) fn remap(
    components: &BTreeMap<String, Value>,
    mapping: &BTreeMap<Id, Id>,
) -> Result<BTreeMap<String, Value>, CommandError> {
    let mut result = components.clone();
    for (kind, value) in &mut result {
        match kind.as_str() {
            "AudioBus" => {
                let bus: AudioBus = decode(value)?;
                if let Some(id) = &bus.parent {
                    replace(&mut value["parent"], id, mapping);
                }
            }
            "AudioSource" => {
                let source: AudioSource = decode(value)?;
                if let Some(id) = &source.bus {
                    replace(&mut value["bus"], id, mapping);
                }
            }
            "NavigationMesh" => {
                let mesh: NavigationMesh = decode(value)?;
                for (index, source) in mesh.sources.iter().enumerate() {
                    replace(
                        &mut value["sources"][index]["entity"],
                        &source.entity,
                        mapping,
                    );
                }
            }
            kind if LITERAL_COMPONENTS.contains(&kind) => {}
            _ => return Err(invalid("component reference semantics are not registered")),
        }
    }
    Ok(result)
}

fn decode<T: serde::de::DeserializeOwned>(value: &Value) -> Result<T, CommandError> {
    serde_json::from_value(value.clone()).map_err(|e| invalid(&format!("typed reference: {e}")))
}
fn replace(value: &mut Value, source: &str, mapping: &BTreeMap<Id, Id>) {
    if let Some(target) = mapping.get(source) {
        *value = Value::String(target.clone());
    }
}

#[cfg(test)]
mod tests;
