//! Actual retained buffers/submission tests, deliberately not run by CPU reviews.
use super::*;
use crate::models::{animation, skeletal_fixture};
use glam::{DMat4, DVec3};
use incant_assets_runtime::RuntimeAsset;
use incant_render_data::{MeshInput, SceneNode};
fn camera() -> SceneNode {
    SceneNode {
        id: "camera".into(),
        world: DMat4::from_translation(DVec3::new(0., 0., 10.)),
        camera: Some(incant_types::Camera {
            fov_degrees: 60.,
            near: 0.1,
            far: 50.,
            projection: incant_types::CameraProjection::Orthographic { vertical_size: 8. },
        }),
        mesh: None,
        light: None,
        sprite: None,
        tilemap: None,
        diagnostic: false,
    }
}
fn read(renderer: &Renderer, view: &wgpu::TextureView) -> Vec<[f32; 4]> {
    let texture = view.texture();
    let w = texture.width();
    let h = texture.height();
    let stride = (w * 16).div_ceil(256) * 256;
    let buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Motion numeric readback"),
        size: u64::from(stride) * u64::from(h),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut commands = renderer.device.create_command_encoder(&Default::default());
    commands.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(stride),
                rows_per_image: Some(h),
            },
        },
        texture.size(),
    );
    renderer.queue.submit([commands.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
    renderer
        .device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(10)),
        })
        .unwrap();
    rx.recv_timeout(std::time::Duration::from_secs(10))
        .unwrap()
        .unwrap();
    let mapped = buffer.slice(..).get_mapped_range();
    let result = mapped
        .chunks(stride as usize)
        .flat_map(|row| {
            row[..w as usize * 16].as_chunks::<16>().0.iter().map(|p| {
                std::array::from_fn(|i| f32::from_le_bytes(p[i * 4..i * 4 + 4].try_into().unwrap()))
            })
        })
        .collect();
    drop(mapped);
    buffer.unmap();
    result
}
#[test]
#[ignore = "requires native GPU; Claude owns bounded numerical execution"]
fn accepted_pose_buffers_and_history_swap_only_after_submission() {
    let dir = tempfile::tempdir().unwrap();
    let asset = crate::test_support::model(dir.path(), 0.);
    let source = Arc::new(
        RuntimeAsset::load_model(
            "model",
            1,
            &dir.path().join(".incant/cache/models"),
            &asset.sha256,
        )
        .unwrap(),
    );
    dir.close().unwrap();
    let mut input = SceneInput {
        nodes: vec![
            SceneNode {
                id: "mesh".into(),
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
            camera(),
        ],
        ..Default::default()
    };
    let mut renderer = crate::test_support::checked(Renderer::headless().unwrap());
    if crate::motion::admission().is_err() {
        assert!(pollster::block_on(renderer.enable_motion_vectors()).is_err());
        return;
    }
    pollster::block_on(renderer.enable_motion_vectors()).unwrap();
    let mut retained = renderer.prepare_retained(&input, "camera").unwrap();
    retained.enable_motion_history(&renderer).unwrap();
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let output = crate::hdr_output::texture(&renderer, format, 65, 33);
    let view = output.create_view(&Default::default());
    let target = |width| RetainedTarget {
        view: &view,
        format,
        width,
        height: 33,
        viewport: None,
    };
    assert!(retained.motion_changes().is_none());
    retained.submit(&renderer, &input, target(65)).unwrap();
    assert!(!retained.motion_frame().unwrap().has_previous());
    let changes = retained.motion_changes().unwrap();
    assert_eq!(
        changes.highest_owner(),
        1,
        "handedness twins share a placement"
    );
    assert_eq!(changes.changed_count(), 1);
    assert_eq!(changes.changed(1), Some(true));
    assert!(
        read(
            &renderer,
            &retained.motion.as_ref().unwrap().readback_view().unwrap()
        )
        .iter()
        .all(|p| p[3] == 0.)
    );
    let accepted = retained.motion_frame().unwrap();
    let live = retained.live.batches[0].instances.clone();
    input.nodes[0].world = DMat4::from_translation(DVec3::X);
    assert!(retained.submit(&renderer, &input, target(0)).is_err());
    assert_eq!(
        retained.motion_frame().unwrap().sequence(),
        accepted.sequence()
    );
    assert_eq!(retained.live.batches[0].instances, live);
    assert_eq!(
        retained.motion_changes().unwrap().frame().sequence(),
        accepted.sequence()
    );
    assert_eq!(retained.motion_changes().unwrap().changed(1), Some(true));
    retained.submit(&renderer, &input, target(65)).unwrap();
    assert_eq!(retained.staging.batches[0].instances, live);
    assert_eq!(retained.motion_changes().unwrap().changed_count(), 1);
    let image = read(
        &renderer,
        &retained.motion.as_ref().unwrap().readback_view().unwrap(),
    );
    let valid: Vec<_> = image.iter().filter(|p| p[3] == 1.).collect();
    assert!(valid.len() > 8);
    for p in valid {
        assert!((p[0] + 33. / (65. * 8.)).abs() < 1e-5);
        assert!(p[1].abs() < 1e-6);
    }
    input.nodes[0].world = DMat4::ZERO;
    assert!(retained.submit(&renderer, &input, target(65)).is_err());
    assert_eq!(retained.motion_frame().unwrap().sequence(), 2);
    assert_eq!(retained.motion_changes().unwrap().changed(1), Some(true));
    input.nodes[0].world = DMat4::from_scale(DVec3::new(-1., 1., 1.));
    retained.submit(&renderer, &input, target(65)).unwrap();
    assert!(
        read(
            &renderer,
            &retained.motion.as_ref().unwrap().readback_view().unwrap()
        )
        .iter()
        .any(|p| p[3] == 1. && p[0].abs() > 0.01)
    );
    assert_eq!(retained.motion_changes().unwrap().highest_owner(), 1);
    assert_eq!(retained.motion_changes().unwrap().changed(1), Some(true));
    retained.invalidate_motion_history();
    retained.submit(&renderer, &input, target(65)).unwrap();
    assert!(!retained.motion_frame().unwrap().has_previous());
    assert_eq!(retained.motion_changes().unwrap().changed(1), Some(true));
    retained.submit(&renderer, &input, target(65)).unwrap();
    assert!(retained.motion_frame().unwrap().has_previous());
    assert_eq!(retained.motion_changes().unwrap().changed(1), Some(false));
    assert_eq!(retained.motion_changes().unwrap().changed_count(), 0);
    let mut replacement = renderer.prepare_retained(&input, "camera").unwrap();
    replacement.enable_motion_history(&renderer).unwrap();
    replacement.submit(&renderer, &input, target(65)).unwrap();
    assert!(!replacement.motion_frame().unwrap().has_previous());
    assert_eq!(replacement.motion_changes().unwrap().changed(1), Some(true));
    assert!(
        !replacement
            .motion_frame()
            .unwrap()
            .same_owner(&retained.motion_frame().unwrap())
    );
}

#[test]
#[ignore = "requires native GPU; Claude owns bounded numerical execution"]
fn skinned_and_ik_history_uses_exact_live_deformed_buffer_and_rejects_rebound_asset() {
    let dir = tempfile::tempdir().unwrap();
    skeletal_fixture::fixture(dir.path());
    let cache = dir.path().join("cache");
    let cooked =
        incant_assets::cook_gltf(dir.path(), std::path::Path::new("model.gltf"), &cache).unwrap();
    let source =
        Arc::new(RuntimeAsset::load_model("rig", 1, &cache, &cooked.metadata.fingerprint).unwrap());
    let foreign =
        Arc::new(RuntimeAsset::load_model("rig", 2, &cache, &cooked.metadata.fingerprint).unwrap());
    dir.close().unwrap();
    let mut camera = camera();
    camera.world = DMat4::from_translation(DVec3::new(12., 1., 8.));
    let mut input = SceneInput {
        nodes: vec![
            SceneNode {
                id: "rig".into(),
                world: DMat4::IDENTITY,
                camera: None,
                light: None,
                mesh: Some(MeshInput {
                    source: source.clone(),
                    cast_shadows: true,
                    animation: Some(incant_types::AnimationSample {
                        clip: Some(0),
                        seconds: 0.,
                    }),
                    animation_blend: None,
                    animation_pose: None,
                    animation_ik: None,
                    animation_root: None,
                }),
                sprite: None,
                tilemap: None,
                diagnostic: false,
            },
            camera,
        ],
        ..Default::default()
    };
    let mut renderer = crate::test_support::checked(Renderer::headless().unwrap());
    if crate::motion::admission().is_err() {
        assert!(pollster::block_on(renderer.enable_motion_vectors()).is_err());
        return;
    }
    pollster::block_on(renderer.enable_motion_vectors()).unwrap();
    let mut retained = renderer.prepare_retained(&input, "camera").unwrap();
    retained.enable_motion_history(&renderer).unwrap();
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let output = crate::hdr_output::texture(&renderer, format, 96, 64);
    let view = output.create_view(&Default::default());
    let target = || RetainedTarget {
        view: &view,
        format,
        width: 96,
        height: 64,
        viewport: None,
    };
    retained.submit(&renderer, &input, target()).unwrap();
    let owners = retained.motion_changes().unwrap().highest_owner();
    assert!(owners > 0);
    assert_eq!(retained.motion_changes().unwrap().changed_count(), owners);
    let before: Vec<_> = retained
        .live
        .batches
        .iter()
        .enumerate()
        .filter_map(|(i, b)| {
            b.vertices.as_ref().map(|buffer| {
                (
                    i,
                    buffer.clone(),
                    animation::motion_test_vertex(b.animated.unwrap(), &retained.live.animations)
                        .unwrap(),
                )
            })
        })
        .collect();
    assert!(!before.is_empty());
    input.nodes[0]
        .mesh
        .as_mut()
        .unwrap()
        .animation
        .as_mut()
        .unwrap()
        .seconds = 1.;
    retained.submit(&renderer, &input, target()).unwrap();
    let mut changed = false;
    for (i, buffer, vertex) in &before {
        assert_eq!(retained.staging.batches[*i].vertices.as_ref(), Some(buffer));
        assert_ne!(retained.live.batches[*i].vertices.as_ref(), Some(buffer));
        assert_eq!(
            animation::motion_test_vertex(
                retained.staging.batches[*i].animated.unwrap(),
                &retained.staging.animations
            ),
            Some(*vertex)
        );
        changed |= animation::motion_test_vertex(
            retained.live.batches[*i].animated.unwrap(),
            &retained.live.animations,
        ) != Some(*vertex);
    }
    assert!(changed);
    assert_eq!(retained.motion_changes().unwrap().highest_owner(), owners);
    assert!(retained.motion_changes().unwrap().changed_count() > 0);
    let changed_count = retained.motion_changes().unwrap().changed_count();
    let changed_classes = retained.motion_changes().unwrap().changed_classes();
    assert!(
        read(
            &renderer,
            &retained.motion.as_ref().unwrap().readback_view().unwrap()
        )
        .iter()
        .any(|p| p[3] == 1. && p[0].abs() + p[1].abs() > 0.005)
    );
    input.nodes[0].mesh.as_mut().unwrap().source = foreign;
    assert!(retained.submit(&renderer, &input, target()).is_err());
    assert_eq!(retained.motion_frame().unwrap().sequence(), 2);
    assert_eq!(
        retained.motion_changes().unwrap().changed_count(),
        changed_count
    );
    assert_eq!(
        retained.motion_changes().unwrap().changed_classes(),
        changed_classes
    );
    let mesh = input.nodes[0].mesh.as_mut().unwrap();
    mesh.source = source;
    mesh.animation = Some(incant_types::AnimationSample::REST);
    mesh.animation_ik = Some(Arc::from([incant_types::AnimationIkSample {
        id: incant_types::StableId([3; 16]),
        chain: [0, 1, 2],
        target: [11., 1., 0.],
        pole: [10., 1., 0.],
        weight: 1.,
    }]));
    retained.submit(&renderer, &input, target()).unwrap();
    assert_eq!(retained.motion_frame().unwrap().sequence(), 3);
    assert_eq!(retained.motion_changes().unwrap().highest_owner(), owners);
    assert!(retained.motion_changes().unwrap().changed_count() > 0);
    let image = read(
        &renderer,
        &retained.motion.as_ref().unwrap().readback_view().unwrap(),
    );
    assert!(image.iter().flatten().all(|v| v.is_finite()));
    assert!(
        image
            .iter()
            .any(|p| p[3] == 1. && p[0].abs() + p[1].abs() > 0.005)
    );
}
