//! Authoring conversion to immutable, source-free font recipe bindings.
use crate::{CookError, stable_id};
use incant_doc::{Entity, Project, Text};
use incant_runtime::{CookedText, FontAssetRef, NativeText, StableId};
use std::{collections::BTreeMap, sync::Arc};

pub(crate) fn cook(
    project: &Project,
    entity: &Entity,
    assets: &mut BTreeMap<StableId, FontAssetRef>,
) -> Result<Option<CookedText>, CookError> {
    let Some(value) = entity.components.get("Text") else {
        return Ok(None);
    };
    let text: Text = serde_json::from_value(value.clone())?;
    let mut fonts = Vec::with_capacity(text.fonts.len());
    for id in &text.fonts {
        let invalid = || CookError::FontAsset(id.clone());
        let asset = project.assets.get(id).ok_or_else(invalid)?;
        if asset.kind != "font" || asset.sha256.len() != 64 {
            return Err(invalid());
        }
        let mut fingerprint = [0; 32];
        for (out, pair) in fingerprint
            .iter_mut()
            .zip(asset.sha256.as_bytes().as_chunks::<2>().0)
        {
            let digit = |b| {
                char::from(b)
                    .to_digit(16)
                    .map(|n| n as u8)
                    .ok_or_else(invalid)
            };
            *out = digit(pair[0])? * 16 + digit(pair[1])?;
        }
        let id = stable_id(id)?;
        assets.insert(id, FontAssetRef { id, fingerprint });
        fonts.push(id);
    }
    let layout = text.layout();
    Ok(Some(CookedText {
        entity: stable_id(&entity.id)?,
        text: Arc::new(NativeText {
            content: text.content.into(),
            fonts: fonts.into(),
            layout,
        }),
    }))
}
