use crate::{CookError, stable_id};
use incant_assets_runtime::{
    CookedModel,
    animation::{AnimationPlayer, TwoBoneIk},
};
use incant_runtime::{AnimationIkSample, NativeIk, StableId};
use std::{collections::BTreeMap, sync::Arc};

pub(super) fn compile(
    authored: &incant_types::AnimationIk,
    fingerprint: [u8; 32],
) -> Result<NativeIk, CookError> {
    authored
        .validate()
        .map_err(incant_runtime::SceneError::Invalid)?;
    let initial = authored
        .constraints
        .iter()
        .map(|c| {
            Ok(AnimationIkSample {
                id: stable_id(&c.id)?,
                chain: c.chain,
                target: c.target,
                pole: c.pole,
                weight: c.weight,
            })
        })
        .collect::<Result<Vec<_>, CookError>>()?;
    let result = NativeIk {
        mesh_fingerprint: fingerprint,
        initial: initial.into(),
    };
    result.validate()?;
    Ok(result)
}
pub(super) fn admit(
    scene: &incant_doc::Scene,
    asset: &str,
    model: &CookedModel,
    fingerprint: [u8; 32],
    budget: &mut usize,
) -> Result<BTreeMap<StableId, Arc<NativeIk>>, CookError> {
    let mut result = BTreeMap::new();
    // Parse/count before allocating a model sampler. There is one such temporary
    // sampler per loaded model, and only the bounded checked bindings survive.
    for entity in scene.entities.values() {
        let Some(value) = entity.components.get("AnimationIk") else {
            continue;
        };
        if super::mesh(entity)?.mesh != asset {
            continue;
        }
        let authored: incant_types::AnimationIk = serde_json::from_value(value.clone())?;
        authored
            .validate()
            .map_err(incant_runtime::SceneError::Invalid)?;
        let count = authored.constraints.len();
        if count > incant_runtime::MAX_SCENE_ANIMATION_IK.saturating_sub(*budget) {
            return Err(incant_runtime::SceneError::Invalid("IK aggregate capacity").into());
        }
        *budget += count;
        result.insert(
            stable_id(&entity.id)?,
            Arc::new(compile(&authored, fingerprint)?),
        );
    }
    if result.is_empty() {
        return Ok(result);
    }
    let error = || CookError::AnimationResource(asset.into());
    let mut player = AnimationPlayer::new(model).map_err(|_| error())?;
    for declaration in result.values() {
        player.sample(None, 0.0).map_err(|_| error())?;
        let mut constraints = [TwoBoneIk {
            root: 0,
            middle: 1,
            tip: 2,
            target: [0.; 3],
            pole: [0.; 3],
            weight: 0.,
        }; 32];
        for (out, c) in constraints.iter_mut().zip(declaration.initial.iter()) {
            *out = TwoBoneIk {
                root: usize::from(c.chain[0]),
                middle: usize::from(c.chain[1]),
                tip: usize::from(c.chain[2]),
                target: c.target,
                pole: c.pole,
                weight: c.weight,
            };
        }
        // Actual rest hierarchy, editable TRS, lengths and affine admission are
        // checked using the same solver. Animated poses still recheck at consume.
        player
            .apply_two_bone_ik(&constraints[..declaration.initial.len()])
            .map_err(|_| error())?;
    }
    Ok(result)
}
