//! Cold compatibility for static content only. Full bounded decoding precedes
//! this check; GPU resources/bounds are rebuilt in the existing staged consumer.
use crate::NativeRenderError;
use incant_assets_runtime::CookedModel;

pub(crate) fn validate(old: &CookedModel, next: &CookedModel) -> Result<(), NativeRenderError> {
    let refuse = |why| NativeRenderError::TextureRefresh(why);
    if old.animation.is_some() || next.animation.is_some() {
        return Err(refuse("animated or skinned model refresh is unsupported"));
    }
    // Geometry buffers may grow/shrink, but node/primitive/material references
    // and interpreted texture roles keep exactly the same meaning. Fingerprints
    // and source dependencies intentionally change with the cooked content.
    if old.metadata.nodes != next.metadata.nodes
        || old.metadata.scenes != next.metadata.scenes
        || old.metadata.default_scene != next.metadata.default_scene
        || old.meshes.len() != next.meshes.len()
        || old.metadata.mesh_materials != next.metadata.mesh_materials
        || old.materials.len() != next.materials.len()
        || old.metadata.textures != next.metadata.textures
        || old.images.len() != next.images.len()
        || old
            .materials
            .iter()
            .zip(&next.materials)
            .any(|(a, b)| a.maps != b.maps)
    {
        return Err(refuse(
            "static model scene, topology or material bindings changed",
        ));
    }
    Ok(())
}
