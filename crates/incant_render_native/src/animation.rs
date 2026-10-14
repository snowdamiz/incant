//! Cold binding checks. Motion carries only the committed numeric sample.
use crate::{NativeRenderError, RuntimeAsset, StableId};
use incant_assets_runtime::RuntimeAssetData;
use incant_runtime::NativeAnimation;

pub(crate) fn validate(
    entity: StableId,
    source: &RuntimeAsset,
    original_fingerprint: &[u8; 32],
    definition: Option<&NativeAnimation>,
) -> Result<(), NativeRenderError> {
    let Some(definition) = definition else {
        return Ok(()); // Animated models without playback use their authored rest pose.
    };
    let invalid = |reason| NativeRenderError::Animation { entity, reason };
    // The sealed catalog initially loads these exact world-reference bytes.
    // Later compatible clip publication changes the private visual version,
    // while the world's playback/graph definitions and clocks stay unchanged.
    // Only the checked model-refresh ticket can establish that new version.
    if definition.mesh_fingerprint != *original_fingerprint {
        return Err(invalid("model fingerprint differs from native clip table"));
    }
    let RuntimeAssetData::Model(model) = source.data() else {
        return Err(invalid("expected a model"));
    };
    let data = model
        .animation
        .as_ref()
        .ok_or_else(|| invalid("model is static"))?;
    if data.clips.len() != definition.clip_durations.len()
        || data
            .clips
            .iter()
            .zip(definition.clip_durations.iter())
            .any(|(clip, expected)| {
                let actual = clip.channels.iter().fold(0.0f32, |duration, channel| {
                    duration.max(*channel.times.last().expect("validated nonempty keys"))
                });
                actual.to_bits() != expected.to_bits()
            })
    {
        return Err(invalid("clip duration table differs from cooked model"));
    }
    Ok(())
}

pub(crate) fn validate_ik(
    entity: StableId,
    source: &RuntimeAsset,
    definition: Option<&incant_runtime::NativeIk>,
) -> Result<(), NativeRenderError> {
    let Some(definition) = definition else {
        return Ok(());
    };
    let invalid = || NativeRenderError::Animation {
        entity,
        reason: "IK chain differs from cooked model",
    };
    definition.validate().map_err(|_| invalid())?;
    let RuntimeAssetData::Model(model) = source.data() else {
        return Err(invalid());
    };
    let data = model.animation.as_ref().ok_or_else(invalid)?;
    for constraint in definition.initial.iter() {
        let [root, middle, tip] = constraint.chain.map(usize::from);
        let nodes = &model.metadata.nodes;
        if root >= nodes.len()
            || middle >= nodes.len()
            || tip >= nodes.len()
            || !nodes[root].children.contains(&middle)
            || !nodes[middle].children.contains(&tip)
            || data.rest[root].is_none()
            || data.rest[middle].is_none()
        {
            return Err(invalid());
        }
    }
    // Pose-dependent lengths/affine frames are checked after actual sampling by
    // the canonical consumer, before private staging is published or uploaded.
    Ok(())
}

pub(crate) fn validate_root(
    entity: StableId,
    source: &RuntimeAsset,
    definition: Option<&incant_runtime::NativeRootMotion>,
) -> Result<(), NativeRenderError> {
    let Some(definition) = definition else {
        return Ok(());
    };
    let invalid = || NativeRenderError::Animation {
        entity,
        reason: "root catalog differs from cooked model",
    };
    let RuntimeAssetData::Model(model) = source.data() else {
        return Err(invalid());
    };
    // Binding/structural boundary only. Compare the complete retained root-only
    // payload against the same canonical compiler used by ordinary cooking.
    let actual = incant_assets_runtime::animation::compile_root_motion_catalog(
        model,
        definition.catalog.fingerprint(),
        definition.catalog.root(),
    )
    .map_err(|_| invalid())?;
    if actual != *definition.catalog {
        return Err(invalid());
    }
    Ok(())
}
