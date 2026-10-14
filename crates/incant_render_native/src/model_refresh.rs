//! Cold model compatibility after complete bounded decoding. Static geometry
//! may change; animated models retain geometry and their complete pose layout.
use crate::NativeRenderError;
use incant_assets_runtime::{CookedModel, animation::AnimationData};

pub(crate) fn validate(old: &CookedModel, next: &CookedModel) -> Result<(), NativeRenderError> {
    let refuse = |why| NativeRenderError::TextureRefresh(why);
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
            "model scene, primitive or material bindings changed",
        ));
    }
    match (&old.animation, &next.animation) {
        (None, None) => Ok(()),
        (Some(a), Some(b)) => animation(old, next, a, b),
        _ => Err(refuse("model animation presence changed")),
    }
}

fn animation(
    old: &CookedModel,
    next: &CookedModel,
    a: &AnimationData,
    b: &AnimationData,
) -> Result<(), NativeRenderError> {
    let refuse = |why| NativeRenderError::TextureRefresh(why);
    // This first slice changes clip content, not deformation bindings or the
    // geometry being deformed. Compare float bits too: PartialEq equates +/-0.
    let same_meshes = old.meshes.iter().zip(&next.meshes).all(|(a, b)| {
        a.indices == b.indices
            && a.vertices.len() == b.vertices.len()
            && a.vertices
                .iter()
                .zip(&b.vertices)
                .all(|(a, b)| a.map(f32::to_bits) == b.map(f32::to_bits))
    });
    let same_rest = a.rest.len() == b.rest.len()
        && a.rest.iter().zip(&b.rest).all(|(a, b)| match (a, b) {
            (None, None) => true,
            (Some(a), Some(b)) => {
                a.translation.map(f32::to_bits) == b.translation.map(f32::to_bits)
                    && a.rotation.map(f32::to_bits) == b.rotation.map(f32::to_bits)
                    && a.scale.map(f32::to_bits) == b.scale.map(f32::to_bits)
            }
            _ => false,
        });
    let same_skins = a.skins.len() == b.skins.len()
        && a.skins.iter().zip(&b.skins).all(|(a, b)| {
            a.name == b.name
                && a.skeleton == b.skeleton
                && a.joints == b.joints
                && a.inverse_bind.len() == b.inverse_bind.len()
                && a.inverse_bind
                    .iter()
                    .zip(&b.inverse_bind)
                    .all(|(a, b)| a.map(|v| v.map(f32::to_bits)) == b.map(|v| v.map(f32::to_bits)))
        });
    let same_influences = a.influences.len() == b.influences.len()
        && a.influences.iter().zip(&b.influences).all(|(a, b)| {
            a.len() == b.len()
                && a.iter().zip(b).all(|(a, b)| {
                    a.joints == b.joints
                        && a.weights.map(f32::to_bits) == b.weights.map(f32::to_bits)
                })
        });
    if !same_meshes || !same_rest || !same_skins || a.node_skins != b.node_skins || !same_influences
    {
        return Err(refuse(
            "animated model geometry, rest or skin bindings changed",
        ));
    }
    // Runtime clip/graph/fade clocks use the startup duration table. Preserve
    // every clip index, name and channel target, including unselected clips.
    // Interior key count/times/values may change within the decoder's bounds.
    let duration = |clip: &incant_assets_runtime::animation::Clip| {
        clip.channels
            .iter()
            .filter_map(|c| c.times.last().copied())
            .fold(0.0_f32, f32::max)
            .to_bits()
    };
    if a.clips.len() != b.clips.len()
        || a.clips.iter().zip(&b.clips).any(|(a, b)| {
            a.name != b.name
                || duration(a) != duration(b)
                || a.channels.len() != b.channels.len()
                || a.channels.iter().zip(&b.channels).any(|(a, b)| {
                    a.node != b.node
                        || a.property != b.property
                        || a.interpolation != b.interpolation
                })
        })
    {
        return Err(refuse("animation clip bindings or duration table changed"));
    }
    Ok(())
}
