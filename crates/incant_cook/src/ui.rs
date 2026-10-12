use crate::{CookError, stable_id};
use incant_doc::{Entity, Project};
use incant_runtime::{CookedUi, FontAssetRef, StableId};
use incant_types::{UiKind, UiNode, UiText};
use std::{collections::BTreeMap, sync::Arc};
pub(crate) fn cook(
    project: &Project,
    entity: &Entity,
    assets: &mut BTreeMap<StableId, FontAssetRef>,
) -> Result<Option<CookedUi>, CookError> {
    let Some(value) = entity.components.get("UiNode") else {
        return Ok(None);
    };
    let n: UiNode = serde_json::from_value(value.clone())?;
    let kind = match n.kind {
        UiKind::Root => UiKind::Root,
        UiKind::Container => UiKind::Container,
        UiKind::Label { text } => UiKind::Label {
            text: convert(project, text, assets)?,
        },
        UiKind::Button { text } => UiKind::Button {
            text: convert(project, text, assets)?,
        },
        UiKind::TextInput { text } => UiKind::TextInput {
            text: convert(project, text, assets)?,
        },
    };
    Ok(Some(CookedUi {
        entity: stable_id(&entity.id)?,
        node: Arc::new(UiNode {
            kind,
            style: n.style,
            order: n.order,
            disabled: n.disabled,
        }),
    }))
}
fn convert(
    project: &Project,
    text: UiText,
    assets: &mut BTreeMap<StableId, FontAssetRef>,
) -> Result<UiText<StableId>, CookError> {
    let fonts = text
        .fonts
        .iter()
        .map(|id| crate::text::font(project, id, assets))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(UiText {
        content: text.content,
        fonts,
        font_size: text.font_size,
        line_height: text.line_height,
        wrap: text.wrap,
    })
}
