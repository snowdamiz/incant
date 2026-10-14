use super::{PrefabError, invalid};
use crate::{AudioBus, AudioSource, Id, NavigationMesh};
use serde_json::Value;
use std::collections::BTreeMap;

/// Only registered, typed entity references are rewritten. Script props remain literal.
/// Every known reference must be internal to the supplied complete mapping.
pub fn remap_components(
    components: &BTreeMap<String, Value>,
    mapping: &BTreeMap<Id, Id>,
) -> Result<BTreeMap<String, Value>, PrefabError> {
    let mut output = components.clone();
    let remap = |id: &mut Id| -> Result<(), PrefabError> {
        *id = mapping
            .get(id)
            .ok_or_else(|| invalid("external entity reference in prototype"))?
            .clone();
        Ok(())
    };
    for (kind, value) in &mut output {
        match kind.as_str() {
            "AudioBus" => {
                let mut v: AudioBus =
                    serde_json::from_value(value.clone()).map_err(|e| invalid(e.to_string()))?;
                if let Some(id) = &mut v.parent {
                    remap(id)?;
                }
                *value = serde_json::to_value(v).map_err(|e| invalid(e.to_string()))?;
            }
            "AudioSource" => {
                let mut v: AudioSource =
                    serde_json::from_value(value.clone()).map_err(|e| invalid(e.to_string()))?;
                if let Some(id) = &mut v.bus {
                    remap(id)?;
                }
                *value = serde_json::to_value(v).map_err(|e| invalid(e.to_string()))?;
            }
            "NavigationMesh" => {
                let mut v: NavigationMesh =
                    serde_json::from_value(value.clone()).map_err(|e| invalid(e.to_string()))?;
                for source in &mut v.sources {
                    remap(&mut source.entity)?;
                }
                *value = serde_json::to_value(v).map_err(|e| invalid(e.to_string()))?;
            }
            _ => {}
        }
    }
    Ok(output)
}
