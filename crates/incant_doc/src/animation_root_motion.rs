use crate::{AnimationRootMotion, Entity, MeshRenderer, Project};

pub(crate) fn validate(entity: &Entity, project: &Project) -> Result<(), String> {
    let Some(value) = entity.components.get("AnimationRootMotion") else {
        return Ok(());
    };
    let binding: AnimationRootMotion =
        serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
    binding.validate().map_err(str::to_owned)?;
    if !entity.components.contains_key("Animation") {
        return Err("AnimationRootMotion requires Animation".into());
    }
    let mesh: MeshRenderer = serde_json::from_value(
        entity
            .components
            .get("MeshRenderer")
            .ok_or("AnimationRootMotion requires MeshRenderer")?
            .clone(),
    )
    .map_err(|e| e.to_string())?;
    let asset = project
        .assets
        .get(&binding.model)
        .ok_or("root-motion model does not exist")?;
    if binding.model != mesh.mesh
        || asset.kind != "model"
        || binding.fingerprint != asset.sha256.to_ascii_lowercase()
    {
        return Err("root-motion model revision differs; explicitly rebind the root".into());
    }
    Ok(())
}
