//! Compiled for delegated numeric execution only. No screenshots or appearance.
use super::*;
use crate::metalfx::{Fallback, Preference, Selection};
use glam::{DMat4, DVec3};
use incant_assets_runtime::RuntimeAsset;
use incant_render_data::{MeshInput, SceneNode};
#[test]
#[ignore = "requires delegated MetalFX GPU execution; at most six submitted frames"]
fn actual_native_batch_and_fallback_preserve_retained_submission_ownership() {
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
        assert!(pollster::block_on(r.enable_metalfx()).is_err());
        return;
    }
    pollster::block_on(r.enable_metalfx()).unwrap();
    println!(
        "MetalFX adapter: {:?}; admission: {:?}",
        r.adapter.get_info(),
        r.metalfx_admission
    );
    // Require the actual native path on the delegated supported device: an
    // unsupported adapter must fail this case rather than count fallback as it.
    assert_eq!(r.metalfx_admission, Some(Ok(())));
    let c = crate::temporal::TemporalConfig {
        history_weight: 0.5,
        relative_depth: 0.01,
        absolute_depth: 0.001,
        reactive_threshold: 1.,
        reactive_strength: 1.,
    };
    let mut scene = r.prepare_retained(&input, "camera").unwrap();
    scene
        .enable_metalfx(&r, Preference::PreferMetalFx, c)
        .unwrap();
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let texture = crate::hdr_output::texture(&r, format, 64, 64);
    let view = texture.create_view(&Default::default());
    let target = |width| RetainedTarget {
        view: &view,
        format,
        width,
        height: 64,
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
    scene.submit(&r, &input, target(64)).unwrap();
    wait();
    assert_eq!(scene.metalfx_selection(), Some(Selection::MetalFx));
    let first = scene.upscale_frame().unwrap();
    let cached = scene.metalfx.as_ref().unwrap().cached_texture().unwrap();
    let old = scene.live.batches[0].instances.clone();
    let old_color = scene.upscaling.as_ref().unwrap().accepted_color().unwrap();
    input.nodes[0].world = DMat4::from_translation(DVec3::X);
    assert!(
        scene
            .submit(
                &r,
                &input,
                RetainedTarget {
                    format: wgpu::TextureFormat::R32Float,
                    ..target(64)
                }
            )
            .is_err()
    );
    assert_eq!(scene.metalfx_selection(), Some(Selection::MetalFx));
    assert_eq!(scene.upscale_frame().unwrap().motion().sequence(), 1);
    assert_eq!(scene.live.batches[0].instances, old);
    assert_eq!(
        scene.upscaling.as_ref().unwrap().accepted_color().unwrap(),
        old_color
    );
    scene.submit(&r, &input, target(64)).unwrap();
    wait();
    assert_eq!(scene.upscale_frame().unwrap().motion().sequence(), 2);
    assert_eq!(scene.staging.batches[0].instances, old);
    // An explicit fractional viewport uses the portable path; native coordinates
    // must never be rounded into a different scene footprint.
    scene
        .submit(
            &r,
            &input,
            RetainedTarget {
                viewport: Some(crate::Viewport {
                    rect: [0.5, 0.5, 63., 63.],
                    corner_radii: [0.; 4],
                    canvas_srgb: [0; 3],
                }),
                ..target(64)
            },
        )
        .unwrap();
    wait();
    assert_eq!(
        scene.metalfx_selection(),
        Some(Selection::Portable(Some(Fallback::FractionalViewport)))
    );
    assert_eq!(
        scene.metalfx.as_ref().unwrap().cached_texture().unwrap(),
        cached
    );
    scene.submit(&r, &input, target(64)).unwrap();
    wait();
    assert_eq!(scene.metalfx_selection(), Some(Selection::MetalFx));
    assert_eq!(
        scene.metalfx.as_ref().unwrap().cached_texture().unwrap(),
        cached
    );
    scene.invalidate_motion_history();
    scene.submit(&r, &input, target(64)).unwrap();
    wait();
    let reset = scene.upscale_frame().unwrap();
    assert!(!reset.motion().has_previous());
    assert!(reset.motion().epoch() > first.motion().epoch());
    let mut fresh = r.prepare_retained(&input, "camera").unwrap();
    fresh
        .enable_metalfx(&r, Preference::RequireMetalFx, c)
        .unwrap();
    fresh.submit(&r, &input, target(64)).unwrap();
    wait();
    assert_eq!(fresh.metalfx_selection(), Some(Selection::MetalFx));
    assert!(
        !fresh
            .motion_frame()
            .unwrap()
            .same_owner(&scene.motion_frame().unwrap())
    );
    println!(
        "native submissions=5; portable submissions=1; rejected submissions=1; old sequence={}, fresh sequence={}",
        scene.motion_frame().unwrap().sequence(),
        fresh.motion_frame().unwrap().sequence()
    );
}
