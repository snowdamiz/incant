//! Numerical behavior and resource-retention checks, not appearance approval.
use super::*;
use crate::hdr_output::{read, texture};
use crate::retained_alloc as allocation;
use glam::{DMat4, DVec3};
use incant_render_data::SceneNode;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
fn node(id: &str) -> SceneNode {
    SceneNode {
        id: id.into(),
        world: DMat4::IDENTITY,
        camera: None,
        light: None,
        mesh: None,
        sprite: None,
        tilemap: None,
        diagnostic: false,
    }
}
fn fixture() -> SceneInput {
    let directory = tempfile::tempdir().unwrap();
    let (project, assets, _) = crate::test_support::fixture(directory.path());
    let mut input = incant_render_authoring::project_input(&project, Some(&assets)).unwrap();
    let mut camera = node("camera");
    camera.world =
        DMat4::look_at_rh(DVec3::new(0., 3., 9.), DVec3::new(0., 2., 0.), DVec3::Y).inverse();
    camera.camera = Some(incant_types::Camera {
        fov_degrees: 55.,
        near: 0.1,
        far: 80.,
        projection: Default::default(),
    });
    input.nodes.push(camera);
    let mut light = node("key");
    light.world = DMat4::from_rotation_y(0.3);
    light.light = Some(SceneLight::Directional(incant_types::DirectionalLight {
        color: [1.; 3],
        intensity: 1.5,
        shadows: Some(incant_types::DirectionalShadows { distance: 30. }),
    }));
    input.nodes.push(light);
    let mut point = node("point");
    point.world = DMat4::from_translation(DVec3::new(1., 3., 2.));
    point.light = Some(SceneLight::Point(incant_types::PointLight {
        color: [1., 0.4, 0.2],
        intensity: 4.,
        range: 8.,
    }));
    input.nodes.push(point);
    drop(assets);
    drop(project);
    directory.close().unwrap();
    input
}
fn target(view: &wgpu::TextureView) -> RetainedTarget<'_> {
    RetainedTarget {
        view,
        format: FORMAT,
        width: 160,
        height: 96,
        viewport: None,
    }
}
fn fresh_pixels(renderer: &Renderer, input: &SceneInput) -> Vec<u8> {
    let scene = renderer
        .prepare(input)
        .unwrap()
        .with_camera("camera")
        .unwrap();
    let output = texture(renderer, FORMAT, 160, 96);
    renderer.queue.submit([renderer
        .draw_scene(
            &scene,
            &output.create_view(&Default::default()),
            FORMAT,
            160,
            96,
            None,
        )
        .unwrap()]);
    read(renderer, &output)
}
#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn retained_motion_matches_fresh_geometry_lights_camera_and_reflections_after_source_deletion() {
    let renderer = Renderer::headless().unwrap();
    let mut input = fixture();
    let mut retained = renderer.prepare_retained(&input, "camera").unwrap();
    let output = texture(&renderer, FORMAT, 160, 96);
    let view = output.create_view(&Default::default());
    let initial = fresh_pixels(&renderer, &input);
    let mut changed = false;
    for frame in 0..8 {
        let t = frame as f64 * 0.1;
        input.nodes[0].world = DMat4::from_scale_rotation_translation(
            DVec3::new(if frame % 2 == 0 { 1. } else { -1. }, 1. + t, 1.),
            glam::DQuat::from_rotation_y(t),
            DVec3::new(-2. + t, 0., 0.),
        );
        input.nodes[2].world =
            DMat4::look_at_rh(DVec3::new(t, 3., 9.), DVec3::new(0., 2., 0.), DVec3::Y).inverse();
        input.nodes[3].world = DMat4::from_rotation_y(0.3 + t);
        input.nodes[4].world = DMat4::from_translation(DVec3::new(1. + t * 4., 3., 2.));
        retained.submit(&renderer, &input, target(&view)).unwrap();
        let pixels = read(&renderer, &output);
        assert_eq!(
            pixels,
            fresh_pixels(&renderer, &input),
            "moving frame {frame}"
        );
        changed |= pixels != initial;
    }
    assert!(changed, "motion test must change visible output");
    assert_eq!(renderer.models.lock().unwrap().len(), 1);
}
#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn rejected_retained_frame_preserves_accepted_gpu_state_and_can_retry() {
    let renderer = Renderer::headless().unwrap();
    let mut input = fixture();
    let mut retained = renderer.prepare_retained(&input, "camera").unwrap();
    let output = texture(&renderer, FORMAT, 160, 96);
    let view = output.create_view(&Default::default());
    retained.submit(&renderer, &input, target(&view)).unwrap();
    let accepted = renderer
        .screenshot_scene_png(&retained.live, 160, 96)
        .unwrap();
    let original = input.nodes[0].world;
    input.nodes[0].world = DMat4::from_translation(DVec3::splat(0.5));
    let mut invalid = target(&view);
    invalid.width = 0;
    assert!(retained.submit(&renderer, &input, invalid).is_err());
    assert_eq!(
        accepted,
        renderer
            .screenshot_scene_png(&retained.live, 160, 96)
            .unwrap()
    );
    input.nodes[0].world = DMat4::from_scale(DVec3::ZERO);
    assert!(retained.submit(&renderer, &input, target(&view)).is_err());
    input.nodes[0].world = original;
    input.nodes[0].id.push('x');
    assert!(retained.submit(&renderer, &input, target(&view)).is_err());
    input.nodes[0].id.pop();
    input.nodes[0].mesh.as_mut().unwrap().cast_shadows = false;
    assert!(retained.submit(&renderer, &input, target(&view)).is_err());
    input.nodes[0].mesh.as_mut().unwrap().cast_shadows = true;
    assert_eq!(
        accepted,
        renderer
            .screenshot_scene_png(&retained.live, 160, 96)
            .unwrap()
    );
    retained.submit(&renderer, &input, target(&view)).unwrap();
    assert_eq!(read(&renderer, &output), fresh_pixels(&renderer, &input));
    let other = Renderer::headless().unwrap();
    assert!(retained.submit(&other, &input, target(&view)).is_err());
}
#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn motion_staging_reuses_reserved_slots_and_rejects_unsupported_bindings() {
    let renderer = Renderer::headless().unwrap();
    let mut input = fixture();
    let mut retained = renderer.prepare_retained(&input, "camera").unwrap();
    let storage = |s: &RetainedScene| {
        s.staging
            .batches
            .iter()
            .map(|b| {
                (
                    Arc::as_ptr(b),
                    b.updates.as_ptr(),
                    b.updates.capacity(),
                    b.centers.as_ptr(),
                    b.centers.capacity(),
                    b.instances.size(),
                )
            })
            .collect::<Vec<_>>()
    };
    let before = storage(&retained);
    let light_pointer = retained.packed_lights.as_ptr();
    for frame in 0..64 {
        input.nodes[0].world =
            DMat4::from_scale(DVec3::new(if frame % 2 == 0 { 1. } else { -1. }, 1., 1.));
        let allocations = allocation::measured(|| retained.stage(&input).unwrap());
        assert_eq!(
            allocations, 0,
            "CPU pose staging allocated at frame {frame}"
        );
        assert_eq!(storage(&retained), before);
        assert_eq!(retained.packed_lights.as_ptr(), light_pointer);
        assert_eq!(
            retained
                .staging
                .batches
                .iter()
                .map(|b| b.count)
                .sum::<u32>(),
            2
        );
    }
    input.nodes[0].diagnostic = true;
    assert!(renderer.prepare_retained(&input, "camera").is_err());
    assert!(retained.stage(&input).is_err());
}

#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn retained_sprite_cpu_staging_allocates_nothing_and_rejects_changed_bindings() {
    let (_, input, _) = super::sprite_tests::fixture();
    let renderer = Renderer::headless().unwrap();
    let mut retained = renderer
        .prepare_retained(input.input(), input.camera_label())
        .unwrap();
    retained.stage(input.input()).unwrap();
    let allocations = allocation::measured(|| {
        for _ in 0..64 {
            retained.stage(input.input()).unwrap();
        }
    });
    assert_eq!(allocations, 0);
    // A mismatch with frozen binding metadata requires a prepared replacement.
    retained
        .bindings
        .iter_mut()
        .find_map(|n| n.sprite.as_mut())
        .unwrap()
        .1
        .layer += 1;
    assert!(retained.stage(input.input()).is_err());
}

#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn retained_tile_cpu_staging_allocates_nothing_without_visiting_binding_cells() {
    let (_, input, _) = super::sprite_tests::tile_fixture();
    let renderer = Renderer::headless().unwrap();
    let mut retained = renderer
        .prepare_retained(input.input(), input.camera_label())
        .unwrap();
    retained.stage(input.input()).unwrap();
    assert_eq!(
        allocation::measured(|| for _ in 0..64 {
            retained.stage(input.input()).unwrap();
        }),
        0
    );
}

#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn retained_flipbook_selection_stages_without_allocation() {
    let (mut world, mut input, id) = super::sprite_tests::fixture();
    let mut declared = world
        .spatial_view()
        .unwrap()
        .nodes()
        .find(|n| n.id == id)
        .unwrap()
        .declaration
        .unwrap()
        .clone();
    declared.flipbook = Some(std::sync::Arc::new(incant_runtime::NativeFlipbook {
        frames: vec![
            incant_runtime::NativeSpriteRegion::Atlas {
                frame: incant_cook::stable_id("00000000000000000000000001").unwrap(),
            },
            incant_runtime::NativeSpriteRegion::Atlas {
                frame: incant_cook::stable_id("00000000000000000000000002").unwrap(),
            },
        ]
        .into(),
        rate: incant_runtime::FrameRate {
            numerator: 60,
            denominator: 1,
        },
        looping: true,
        playing: true,
    }));
    let mut commands = incant_runtime::FrameCommands::default();
    commands
        .push(incant_runtime::StructuralCommand::SetSpatial {
            entity: id,
            spatial: Some(declared),
        })
        .unwrap();
    world.apply(&mut commands).unwrap();
    input
        .try_sync(&world.spatial_view().unwrap(), |_, _, _| Ok::<_, ()>(()))
        .unwrap();
    let renderer = Renderer::headless().unwrap();
    let mut retained = renderer
        .prepare_retained(input.input(), input.camera_label())
        .unwrap();
    for _ in 0..64 {
        world.step().unwrap();
        input
            .try_sync(&world.spatial_view().unwrap(), |_, _, _| Ok::<_, ()>(()))
            .unwrap();
        assert_eq!(
            allocation::measured(|| retained.stage(input.input()).unwrap()),
            0
        );
    }
}
