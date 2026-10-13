//! Real retained geometry/color/depth owner regression. No screenshots or artifacts.
use super::*;
use glam::{DMat4, DVec3};
use incant_assets_runtime::RuntimeAsset;
use incant_render_data::{MeshInput, SceneNode};
#[test]
#[ignore = "requires native GPU; Claude owns bounded numeric execution"]
fn temporal_color_motion_and_geometry_publish_once_and_failures_consume_no_phase() {
    let temp = tempfile::tempdir().unwrap();
    let asset = crate::test_support::model(temp.path(), 0.);
    let source = Arc::new(
        RuntimeAsset::load_model(
            "model",
            1,
            &temp.path().join(".incant/cache/models"),
            &asset.sha256,
        )
        .unwrap(),
    );
    temp.close().unwrap();
    let mut input = SceneInput {
        nodes: vec![
            SceneNode {
                id: "model".into(),
                world: DMat4::IDENTITY,
                camera: None,
                light: None,
                mesh: Some(MeshInput {
                    source,
                    cast_shadows: true,
                    animation: None,
                    animation_blend: None,
                    animation_pose: None,
                    animation_ik: None,
                    animation_root: None,
                }),
                sprite: None,
                tilemap: None,
                diagnostic: false,
            },
            SceneNode {
                id: "camera".into(),
                world: DMat4::from_translation(DVec3::new(0., 0., 10.)),
                camera: Some(incant_types::Camera {
                    fov_degrees: 60.,
                    near: 0.1,
                    far: 50.,
                    projection: incant_types::CameraProjection::Orthographic { vertical_size: 8. },
                }),
                light: None,
                mesh: None,
                sprite: None,
                tilemap: None,
                diagnostic: false,
            },
        ],
        ..Default::default()
    };
    let mut r = crate::test_support::checked(Renderer::headless().unwrap());
    if crate::motion::admission().is_err() {
        assert!(pollster::block_on(r.enable_temporal_accumulation()).is_err());
        return;
    }
    pollster::block_on(r.enable_temporal_accumulation()).unwrap();
    let config = crate::temporal::TemporalConfig {
        history_weight: 0.5,
        relative_depth: 0.01,
        absolute_depth: 0.001,
        reactive_threshold: 1.,
        reactive_strength: 1.,
    };
    let mut retained = r.prepare_retained(&input, "camera").unwrap();
    retained.enable_temporal_history(&r, config).unwrap();
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let texture = crate::hdr_output::texture(&r, format, 65, 33);
    let view = texture.create_view(&Default::default());
    let target = |width| RetainedTarget {
        view: &view,
        format,
        width,
        height: 33,
        viewport: None,
    };
    retained.submit(&r, &input, target(65)).unwrap();
    let first = retained.temporal_frame().unwrap();
    assert_eq!(first.jitter_phase(), Some(0));
    assert!(first.motion().same_owner(&retained.motion_frame().unwrap()));
    let color = retained
        .temporal
        .as_ref()
        .unwrap()
        .accepted_color()
        .unwrap();
    let live = retained.live.batches[0].instances.clone();
    input.nodes[0].world = DMat4::from_translation(DVec3::X);
    assert!(retained.submit(&r, &input, target(0)).is_err());
    // Resource preparation succeeds; unsupported output format refuses before
    // any writes or submission. Both history families must remain accepted-old.
    assert!(
        retained
            .submit(
                &r,
                &input,
                RetainedTarget {
                    format: wgpu::TextureFormat::R32Float,
                    ..target(65)
                }
            )
            .is_err()
    );
    assert_eq!(retained.temporal_frame().unwrap().motion().sequence(), 1);
    assert_eq!(retained.live.batches[0].instances, live);
    assert_eq!(
        retained
            .temporal
            .as_ref()
            .unwrap()
            .accepted_color()
            .unwrap(),
        color
    );
    retained.submit(&r, &input, target(65)).unwrap();
    assert_eq!(retained.temporal_frame().unwrap().jitter_phase(), Some(1));
    assert_eq!(retained.staging.batches[0].instances, live);
    assert_ne!(
        retained
            .temporal
            .as_ref()
            .unwrap()
            .accepted_color()
            .unwrap(),
        color
    );
    retained.invalidate_motion_history();
    input.nodes[0].world = DMat4::ZERO;
    assert!(retained.submit(&r, &input, target(65)).is_err());
    assert_eq!(retained.temporal_frame().unwrap().jitter_phase(), Some(1));
    input.nodes[0].world = DMat4::IDENTITY;
    retained.submit(&r, &input, target(65)).unwrap();
    let reset = retained.temporal_frame().unwrap();
    assert_eq!(reset.jitter_phase(), Some(0));
    assert!(!reset.motion().has_previous());
    assert!(reset.motion().epoch() > first.motion().epoch());
    let mut replacement = r.prepare_retained(&input, "camera").unwrap();
    replacement.enable_temporal_history(&r, config).unwrap();
    replacement.submit(&r, &input, target(65)).unwrap();
    assert!(
        !reset
            .motion()
            .same_owner(replacement.temporal_frame().unwrap().motion())
    );
    assert_eq!(
        replacement.temporal_frame().unwrap().jitter_phase(),
        Some(0)
    );
    // Same physical output: changed internal scale resets both histories together.
    let scale = crate::resolution::ResolutionScale::new(8).unwrap();
    let resolution = crate::resolution::ResolutionConfig {
        target_ms: 16.,
        deadband: 0.1,
        minimum: scale,
        maximum: scale,
        initial: scale,
        overload_samples: 2,
        recovery_samples: 2,
        cooldown_samples: 1,
        slew: 1,
    };
    retained.set_dynamic_resolution(Some(resolution)).unwrap();
    retained.submit(&r, &input, target(65)).unwrap();
    let frame = retained.temporal_frame().unwrap();
    assert_eq!(frame.jitter_phase(), Some(0));
    assert!(!frame.motion().has_previous());
    assert_eq!(frame.motion().internal_size(), [33, 17]);
    assert_ne!(
        retained
            .temporal
            .as_ref()
            .unwrap()
            .accepted_color()
            .unwrap(),
        color
    );
}
