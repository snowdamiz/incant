//! The canonical native adapter feeding retained GPU text: source-free motion,
//! topology rebuild reuse, changed text and consumer refusal. Numeric only.
use super::*;
use crate::{
    hdr_output::{read, texture},
    text::tests::MIXED,
};
use incant_render_native::{NativeRenderInput, TextSettings, UpdateKind, VisualCaches};
use incant_runtime::{
    CookedEntity, CookedScene, CookedSpatial, CookedText, FontAssetRef, NativeText, NativeWorld,
    StableId, StructuralCommand, TextLayout, TextWrap,
};
use incant_types::{Camera, CameraProjection, PointLight, Transform, Velocity};
use std::path::Path;

fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
fn text(content: &str, fonts: &[FontAssetRef]) -> Arc<NativeText> {
    Arc::new(NativeText {
        content: content.into(),
        fonts: fonts.iter().map(|f| f.id).collect(),
        layout: TextLayout {
            font_size: 24.,
            line_height: 32.,
            width: 400.,
            wrap: TextWrap::WordOrGlyph,
        },
    })
}
fn fixture() -> (NativeWorld, NativeRenderInput, Vec<FontAssetRef>) {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let mut fonts = Vec::new();
    for (n, name) in [
        "NotoSans-Regular.ttf",
        "NotoSansArabic.ttf",
        "IncantCJKFixture.ttf",
    ]
    .into_iter()
    .enumerate()
    {
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../incant_text/tests/fonts")
            .join(name);
        std::fs::copy(source, root.join(name)).unwrap();
        let font = incant_assets::cook_font(root, Path::new(name), &root.join("fonts")).unwrap();
        fonts.push(FontAssetRef {
            id: id(70 + n as u128),
            fingerprint: std::array::from_fn(|i| {
                u8::from_str_radix(&font.metadata().fingerprint[2 * i..2 * i + 2], 16).unwrap()
            }),
        });
    }
    let entity = |n: u128, at: [f64; 3], velocity: Option<[f64; 3]>| CookedEntity {
        id: id(n),
        parent: None,
        transform: Transform {
            translation: at,
            ..Default::default()
        },
        velocity: velocity.map(|linear| Velocity { linear }),
    };
    let mut camera = CookedSpatial::empty(id(2));
    camera.camera = Some(Camera {
        fov_degrees: 50.,
        near: 0.1,
        far: 50.,
        projection: CameraProjection::Perspective {},
    });
    let mut light = CookedSpatial::empty(id(3));
    light.point = Some(PointLight {
        color: [1.; 3],
        intensity: 1.,
        range: 4.,
    });
    let scene = CookedScene::new(
        id(99),
        60,
        vec![
            entity(1, [-1.2, 0.5, 0.], Some([0.6, -0.3, 0.])),
            entity(2, [0., 0., 5.], None),
            entity(3, [0.5, -0.5, 0.], None),
        ],
    )
    .unwrap()
    .with_spatial(vec![], vec![camera, light])
    .unwrap()
    .with_text(
        fonts.clone(),
        vec![CookedText {
            entity: id(1),
            text: text(MIXED, &fonts),
        }],
    )
    .unwrap();
    let world = NativeWorld::from_bytes(&scene.to_bytes()).unwrap();
    let input = NativeRenderInput::load_with_text(
        &world.spatial_view().unwrap(),
        &world.text_view().unwrap(),
        VisualCaches {
            models: Path::new("absent"),
            textures: Path::new("absent"),
        },
        id(2),
        TextSettings::new(root.join("fonts")),
    )
    .unwrap();
    directory.close().unwrap();
    (world, input, fonts)
}

#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn source_free_native_text_moves_rebuilds_and_changes_through_the_retained_consumer() {
    let (mut world, mut input, fonts) = fixture();
    let renderer = Renderer::headless().unwrap();
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let output = texture(&renderer, format, 160, 96);
    let view = output.create_view(&Default::default());
    let target = |width| RetainedTarget {
        view: &view,
        format,
        width,
        height: 96,
        viewport: None,
    };
    let mut gpu = renderer
        .prepare_retained(input.input(), input.camera_label())
        .unwrap();
    gpu.submit(&renderer, input.input(), target(160)).unwrap();
    let mut uploads = renderer.text_uploads();
    let initial = read(&renderer, &output);
    let mut rebuilt = 0;
    for frame in 0..9 {
        if frame == 3 {
            // Spatial topology only: the adapter reuses its layout/atlas Arcs.
            let mut commands = incant_runtime::FrameCommands::default();
            commands
                .push(StructuralCommand::SetParent {
                    entity: id(1),
                    parent: Some(id(3)),
                })
                .unwrap();
            world.apply(&mut commands).unwrap();
        }
        if frame == 6 {
            let mut commands = incant_runtime::FrameCommands::default();
            commands
                .push(StructuralCommand::SetText {
                    entity: id(1),
                    text: Some(text("Changed سلام 文", &fonts)),
                })
                .unwrap();
            world.apply(&mut commands).unwrap();
        }
        world.step().unwrap();
        if frame == 1 {
            // A refused consumer frame publishes nothing and is retryable.
            let tick = input.tick();
            let accepted = renderer.screenshot_scene_png(&gpu.live, 160, 96).unwrap();
            assert!(
                input
                    .try_sync_with_text(
                        &world.spatial_view().unwrap(),
                        &world.text_view().unwrap(),
                        |scene, _, _| gpu.submit(&renderer, scene, target(0)),
                    )
                    .is_err()
            );
            assert_eq!(input.tick(), tick);
            assert_eq!(
                renderer.screenshot_scene_png(&gpu.live, 160, 96).unwrap(),
                accepted
            );
        }
        let replacement = input
            .try_sync_with_text(
                &world.spatial_view().unwrap(),
                &world.text_view().unwrap(),
                |scene, camera, kind| -> Result<Option<RetainedScene>> {
                    if kind == UpdateKind::Rebuilt {
                        rebuilt += 1;
                        let mut candidate = renderer.prepare_retained(scene, camera)?;
                        candidate.submit(&renderer, scene, target(160))?;
                        Ok(Some(candidate))
                    } else {
                        gpu.submit(&renderer, scene, target(160))?;
                        Ok(None)
                    }
                },
            )
            .unwrap();
        let after = renderer.text_uploads();
        if let Some(candidate) = replacement {
            let shared = candidate
                .live
                .text
                .as_ref()
                .unwrap()
                .shares_geometry(gpu.live.text.as_ref().unwrap());
            if frame == 3 {
                assert!(shared, "topology rebuild must keep text geometry");
                assert_eq!(after, uploads);
            } else {
                assert_eq!(frame, 6);
                assert!(!shared);
                assert_eq!(after.atlas_textures, uploads.atlas_textures + 1);
                assert_eq!(after.glyph_buffers, uploads.glyph_buffers + 1);
                uploads = after;
            }
            gpu = candidate;
        } else {
            assert_eq!(after, uploads, "motion uploaded text at {frame}");
        }
        let actual = read(&renderer, &output);
        assert_ne!(actual, initial, "frame {frame}");
        let reference = renderer
            .prepare(input.input())
            .unwrap()
            .with_camera(input.camera_label())
            .unwrap();
        renderer.queue.submit([renderer
            .draw_scene(&reference, &view, format, 160, 96, None)
            .unwrap()]);
        assert_eq!(
            actual,
            read(&renderer, &output),
            "native text frame {frame}"
        );
    }
    assert_eq!(rebuilt, 2);
    let stats = gpu.stats();
    assert_eq!(stats.text_entities, 1);
    assert!(stats.text_glyph_quads > 6);
    assert_eq!(stats.text_missing_glyphs, 0);
}
