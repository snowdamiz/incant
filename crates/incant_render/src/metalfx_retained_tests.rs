//! Actual native selection is mandatory; fallback never counts as GPU success.
use super::*;
use crate::metalfx::{Config, Fallback, Preference, Selection};
use glam::{DMat4, DVec3};
use incant_assets_runtime::RuntimeAsset;
use incant_render_data::{MeshInput, SceneNode};
#[test]
#[ignore = "requires coordinated Mac MetalFX execution; exactly eight submitted scene frames"]
fn actual_reduced_native_selection_reset_fallback_and_replacement() {
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
    pollster::block_on(r.enable_metalfx()).unwrap();
    assert!(
        r.metalfx_admission.is_some_and(|r| r.is_ok()),
        "actual native support required: {:?}",
        r.metalfx_admission
    );
    println!(
        "actual adapter {:?}; native range {:?}",
        r.adapter.get_info(),
        r.metalfx_admission
    );
    let config = Config {
        relative_depth: 0.01,
        absolute_depth: 0.001,
    };
    let mut scene = r.prepare_retained(&input, "camera").unwrap();
    scene
        .enable_metalfx(&r, Preference::PreferMetalFx, config)
        .unwrap();
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let texture = crate::hdr_output::texture(&r, format, 96, 96);
    let view = texture.create_view(&Default::default());
    let target = || RetainedTarget {
        view: &view,
        format,
        width: 96,
        height: 96,
        viewport: None,
    };
    let wait = || {
        r.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(10)),
            })
            .unwrap()
    };
    // Explicit unprepared required extent fails before accepting a frame or
    // publishing any scene update. No pipeline work is deferred to submit.
    scene
        .enable_metalfx(&r, Preference::RequireMetalFx, config)
        .unwrap();
    let error = scene.submit(&r, &input, target()).unwrap_err();
    assert_eq!(
        error.downcast_ref::<crate::metalfx::MetalFxError>(),
        Some(&crate::metalfx::MetalFxError::Unavailable(
            Fallback::UnpreparedExtent
        ))
    );
    assert!(scene.motion_frame().is_none());
    scene
        .enable_metalfx(&r, Preference::PreferMetalFx, config)
        .unwrap();
    // Explicit preparation can be superseded before any ordinary frame.
    scene.prepare_metalfx_extent(&r, [64, 64]).unwrap();
    scene.prepare_metalfx_extent(&r, [96, 96]).unwrap();
    let resolution = |scale| crate::resolution::ResolutionConfig {
        initial: scale,
        minimum: scale,
        maximum: scale,
        target_ms: 16.,
        deadband: 0.1,
        overload_samples: 1,
        recovery_samples: 1,
        cooldown_samples: 0,
        slew: 1,
    };
    let half = crate::resolution::ResolutionScale::new(8).unwrap();
    scene
        .set_dynamic_resolution(Some(resolution(half)))
        .unwrap();
    scene.submit(&r, &input, target()).unwrap();
    wait();
    assert_eq!(scene.metalfx_selection(), Some(Selection::MetalFx));
    assert_eq!(scene.motion_frame().unwrap().internal_size(), [48, 48]);
    let first = scene.motion_frame().unwrap();
    scene.submit(&r, &input, target()).unwrap();
    wait();
    assert!(scene.motion_frame().unwrap().has_previous());
    input.nodes[0].world = DMat4::from_translation(DVec3::X);
    scene.submit(&r, &input, target()).unwrap();
    wait();
    assert_eq!(scene.metalfx_selection(), Some(Selection::MetalFx));
    let old = scene.live.batches[0].instances.clone();
    assert!(
        scene
            .submit(
                &r,
                &input,
                RetainedTarget {
                    format: wgpu::TextureFormat::R32Float,
                    ..target()
                }
            )
            .is_err()
    );
    assert_eq!(scene.motion_frame().unwrap().sequence(), 3);
    assert_eq!(scene.live.batches[0].instances, old);
    scene
        .set_dynamic_resolution(Some(resolution(
            crate::resolution::ResolutionScale::new(12).unwrap(),
        )))
        .unwrap();
    scene.submit(&r, &input, target()).unwrap();
    wait();
    assert_eq!(scene.metalfx_selection(), Some(Selection::MetalFx));
    assert_eq!(scene.motion_frame().unwrap().internal_size(), [72, 72]);
    assert!(!scene.motion_frame().unwrap().has_previous());
    scene
        .submit(
            &r,
            &input,
            RetainedTarget {
                viewport: Some(crate::Viewport {
                    rect: [0.5, 0.5, 95., 95.],
                    corner_radii: [0.; 4],
                    canvas_srgb: [0; 3],
                }),
                ..target()
            },
        )
        .unwrap();
    wait();
    assert_eq!(
        scene.metalfx_selection(),
        Some(Selection::Spatial(Some(Fallback::FractionalViewport)))
    );
    assert_eq!(scene.motion_frame().unwrap().jitter_phase(), None);
    scene.submit(&r, &input, target()).unwrap();
    wait();
    assert_eq!(scene.metalfx_selection(), Some(Selection::MetalFx));
    assert_eq!(scene.motion_frame().unwrap().jitter_phase(), Some(0));
    assert!(!scene.motion_frame().unwrap().has_previous());
    scene.invalidate_motion_history();
    scene.submit(&r, &input, target()).unwrap();
    wait();
    assert!(scene.motion_frame().unwrap().epoch() > first.epoch());
    assert!(!scene.motion_frame().unwrap().has_previous());
    let texture = crate::hdr_output::texture(&r, format, 64, 64);
    let view = texture.create_view(&Default::default());
    let mut fresh = r.prepare_retained(&input, "camera").unwrap();
    fresh
        .enable_metalfx(&r, Preference::RequireMetalFx, config)
        .unwrap();
    fresh.prepare_metalfx_extent(&r, [64, 64]).unwrap();
    fresh
        .set_dynamic_resolution(Some(resolution(half)))
        .unwrap();
    fresh
        .submit(
            &r,
            &input,
            RetainedTarget {
                view: &view,
                format,
                width: 64,
                height: 64,
                viewport: None,
            },
        )
        .unwrap();
    wait();
    assert_eq!(fresh.metalfx_selection(), Some(Selection::MetalFx));
    assert!(
        !fresh
            .motion_frame()
            .unwrap()
            .same_owner(&scene.motion_frame().unwrap())
    );
    println!(
        "native frames7; spatial frames1; rejected format1; unprepared1; submissions8; maps0; quality unreviewed"
    );
}
