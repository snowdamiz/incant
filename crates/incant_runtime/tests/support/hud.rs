use incant_runtime::*;
use incant_types::{UiKind, UiNode, UiStyle, UiText};
use std::sync::Arc;
pub fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
pub fn scene() -> CookedScene {
    let label = UiText {
        content: "Health 10".into(),
        fonts: vec![id(9)],
        font_size: 16.,
        line_height: 20.,
        wrap: TextWrap::WordOrGlyph,
    };
    let node = |kind| {
        Arc::new(UiNode {
            kind,
            style: UiStyle::default(),
            order: 0,
            disabled: false,
        })
    };
    CookedScene::new(
        id(0),
        60,
        (1..=5)
            .map(|n| CookedEntity {
                id: id(n),
                parent: (n == 2 || n == 3).then_some(id(1)),
                transform: Default::default(),
                velocity: None,
            })
            .collect(),
    )
    .unwrap()
    .with_text(
        vec![FontAssetRef {
            id: id(9),
            fingerprint: [7; 32],
        }],
        vec![CookedText {
            entity: id(4),
            text: Arc::new(NativeText {
                content: "World".into(),
                fonts: Arc::from([id(9)]),
                layout: TextLayout {
                    font_size: 16.,
                    line_height: 20.,
                    width: 300.,
                    wrap: TextWrap::WordOrGlyph,
                },
            }),
        }],
    )
    .unwrap()
    .with_ui(vec![
        CookedUi {
            entity: id(1),
            node: node(UiKind::Root),
        },
        CookedUi {
            entity: id(2),
            node: node(UiKind::Label {
                text: label.clone(),
            }),
        },
        CookedUi {
            entity: id(3),
            node: node(UiKind::TextInput { text: label }),
        },
    ])
    .unwrap()
}
pub type Declarations = (Vec<(StableId, NativeUi)>, Vec<(StableId, NativeText)>);
pub fn declarations(w: &NativeWorld) -> Declarations {
    (
        w.ui_view()
            .unwrap()
            .nodes()
            .map(|n| (n.id, (**n.node).clone()))
            .collect(),
        w.text_view()
            .unwrap()
            .nodes()
            .map(|(id, t)| (id, t.clone()))
            .collect(),
    )
}
