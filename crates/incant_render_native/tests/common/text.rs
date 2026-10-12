use super::common::*;
use incant_render_native::{NativeRenderInput, TextSettings, VisualCaches};
use incant_runtime::*;
use std::{path::Path, sync::Arc};
const FONT: &[u8] = include_bytes!("../../../incant_text/tests/fonts/NotoSans-Regular.ttf");
pub fn text(content: &str) -> Arc<NativeText> {
    Arc::new(NativeText {
        content: content.into(),
        fonts: Arc::from([id(70)]),
        layout: TextLayout {
            font_size: 20.,
            line_height: 28.,
            width: 400.,
            wrap: TextWrap::WordOrGlyph,
        },
    })
}
pub fn fixture() -> (tempfile::TempDir, NativeWorld, NativeRenderInput) {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(temp.path().join("font.ttf"), FONT).unwrap();
    let cache = temp.path().join("fonts");
    let font = incant_assets::cook_font(temp.path(), Path::new("font.ttf"), &cache).unwrap();
    let fingerprint = std::array::from_fn(|i| {
        u8::from_str_radix(&font.metadata().fingerprint[2 * i..2 * i + 2], 16).unwrap()
    });
    let world = NativeWorld::new(
        scene()
            .with_text(
                vec![FontAssetRef {
                    id: id(70),
                    fingerprint,
                }],
                vec![CookedText {
                    entity: id(1),
                    text: text("ffi Hello"),
                }],
            )
            .unwrap(),
    );
    let input = NativeRenderInput::load_with_text(
        &world.spatial_view().unwrap(),
        &world.text_view().unwrap(),
        VisualCaches {
            models: Path::new("absent"),
            textures: Path::new("absent"),
        },
        id(2),
        TextSettings::new(cache),
    )
    .unwrap();
    (temp, world, input)
}
