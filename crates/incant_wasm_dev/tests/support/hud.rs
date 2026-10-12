use incant_render_game::{NativeRenderSettings, UiSettings};
use incant_render_native::TextSettings;
use incant_runtime::*;
use incant_types::{Camera, CameraProjection, TextWrap, UiKind, UiLength, UiNode, UiStyle, UiText};
use std::{path::Path, sync::Arc};
const FONT: &[u8] = include_bytes!("../../../incant_text/tests/fonts/NotoSans-Regular.ttf");
pub fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
pub fn composed_fixture() -> (tempfile::TempDir, CookedScene, NativeRenderSettings) {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(temp.path().join("font.ttf"), FONT).unwrap();
    let font = incant_assets::cook_font(
        temp.path(),
        Path::new("font.ttf"),
        &temp.path().join("fonts"),
    )
    .unwrap();
    let fingerprint = std::array::from_fn(|i| {
        u8::from_str_radix(&font.metadata().fingerprint[2 * i..2 * i + 2], 16).unwrap()
    });
    let mut camera = CookedSpatial::empty(id(8));
    camera.camera = Some(Camera {
        fov_degrees: 60.,
        near: 0.1,
        far: 100.,
        projection: CameraProjection::Perspective {},
    });
    let entities = (1..=8)
        .map(|n| CookedEntity {
            id: id(n),
            parent: matches!(n, 2..=3).then(|| id(1)),
            transform: Default::default(),
            velocity: None,
        })
        .collect();
    let text = UiText {
        content: "Name".into(),
        fonts: vec![id(9)],
        font_size: 16.,
        line_height: 24.,
        wrap: TextWrap::WordOrGlyph,
    };
    let kinds = [
        UiKind::Root,
        UiKind::Button { text: text.clone() },
        UiKind::TextInput { text },
    ];
    let ui = kinds
        .into_iter()
        .enumerate()
        .map(|(n, kind)| CookedUi {
            entity: id(n as u128 + 1),
            node: Arc::new(UiNode {
                kind,
                style: if n == 0 {
                    UiStyle::default()
                } else {
                    UiStyle {
                        size: [UiLength::Pixels(160.), UiLength::Pixels(32.)],
                        ..Default::default()
                    }
                },
                order: 0,
                disabled: false,
            }),
        })
        .collect();
    let scene = CookedScene::new(id(99), 60, entities)
        .unwrap()
        .with_spatial(vec![], vec![camera])
        .unwrap()
        .with_text(
            vec![FontAssetRef {
                id: id(9),
                fingerprint,
            }],
            vec![CookedText {
                entity: id(4),
                text: Arc::new(NativeText {
                    content: "World".into(),
                    fonts: Arc::from([id(9)]),
                    layout: TextLayout {
                        font_size: 16.,
                        line_height: 24.,
                        width: 300.,
                        wrap: TextWrap::WordOrGlyph,
                    },
                }),
            }],
        )
        .unwrap()
        .with_ui(ui)
        .unwrap();
    let settings = NativeRenderSettings {
        model_cache: temp.path().join("models"),
        texture_cache: temp.path().join("textures"),
        camera: id(8),
        text: Some(TextSettings::new(temp.path().join("fonts"))),
        ui: Some(UiSettings::new([320., 200.])),
        consumer: (),
    };
    (temp, scene, settings)
}
