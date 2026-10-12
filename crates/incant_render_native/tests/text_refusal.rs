mod common;
use common::*;
use incant_render_native::{NativeRenderError, NativeRenderInput, VisualCaches};
use incant_runtime::*;
use std::{path::Path, sync::Arc};
#[test]
fn mixed_text_is_refused_before_cache_reads_or_camera_publication() {
    let scene = scene()
        .with_text(
            vec![FontAssetRef {
                id: id(70),
                fingerprint: [1; 32],
            }],
            vec![CookedText {
                entity: id(1),
                text: Arc::new(NativeText {
                    content: Arc::from("Hello"),
                    fonts: Arc::from([id(70)]),
                    layout: TextLayout {
                        font_size: 16.,
                        line_height: 20.,
                        width: 100.,
                        wrap: TextWrap::None,
                    },
                }),
            }],
        )
        .unwrap();
    let world = NativeWorld::new(scene);
    let result = NativeRenderInput::load(
        &world.spatial_view().unwrap(),
        VisualCaches {
            models: Path::new("must-not-read"),
            textures: Path::new("must-not-read"),
        },
        id(2),
    );
    assert!(matches!(result, Err(NativeRenderError::TextUnsupported)));
}
