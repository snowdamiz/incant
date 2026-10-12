//! Command-bus final-state UI validation. No separate GUI-only mutation route.
use crate::{Scene, Transform};
use incant_types::{StableId, UiKind, UiLink, UiNode, validate_ui_tree};
use std::collections::BTreeSet;
pub(crate) fn validate_scene(scene: &Scene) -> Result<(), String> {
    let count = scene
        .entities
        .values()
        .filter(|e| e.components.contains_key("UiNode"))
        .count();
    if count == 0 {
        return Ok(());
    }
    if count > incant_types::MAX_UI_NODES {
        return Err("UI node count".into());
    }
    let id = |s: &str| {
        s.parse::<ulid::Ulid>()
            .map(|n| StableId(n.to_bytes()))
            .map_err(|_| "invalid UI entity ID".to_owned())
    };
    let mut data = Vec::with_capacity(count);
    let mut ui_ids = BTreeSet::new();
    for e in scene.entities.values() {
        let Some(v) = e.components.get("UiNode") else {
            continue;
        };
        let n: UiNode = serde_json::from_value(v.clone()).map_err(|e| e.to_string())?;
        n.validate().map_err(str::to_owned)?;
        if e.components
            .keys()
            .any(|k| !matches!(k.as_str(), "UiNode" | "Transform" | "Script"))
        {
            return Err("UI widgets require separate entities from world components".into());
        }
        if let Some(t) = e.components.get("Transform") {
            let t: Transform = serde_json::from_value(t.clone()).map_err(|e| e.to_string())?;
            if t != Default::default() {
                return Err("UI widgets require identity Transform".into());
            }
        }
        ui_ids.insert(e.id.as_str());
        data.push((id(&e.id)?, e.parent.as_deref().map(id).transpose()?, n));
    }
    let links: Vec<_> = data
        .iter()
        .map(|(id, parent, node)| UiLink {
            id: *id,
            parent: *parent,
            node,
        })
        .collect();
    validate_ui_tree(&links).map_err(str::to_owned)?;
    for e in scene.entities.values() {
        if e.parent.as_deref().is_some_and(|p| ui_ids.contains(p))
            && !ui_ids.contains(e.id.as_str())
        {
            return Err("UI containers require UI children".into());
        }
    }
    // Root is a layout viewport, not an implicit camera or world transform.
    debug_assert!(data.iter().any(|(_, _, n)| matches!(n.kind, UiKind::Root)));
    Ok(())
}
