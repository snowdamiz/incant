//! Actual authoring cook -> native binary world -> retained CPU/GPU consumer.
use super::*;
use crate::hdr_output::{read, texture};
use incant_doc::{Entity, Transform, new_id};
use incant_render_native::{NativeRenderInput, UpdateKind, VisualCaches};
use incant_runtime::{
    CookedSpatial, FrameCommands, NativeMeshRenderer, NativeWorld, StructuralCommand,
};
use serde_json::json;
#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn binary_native_motion_and_structural_rebuilds_submit_without_source_or_document() {
    let directory = tempfile::tempdir().unwrap();
    let (mut project, store, asset_id) = crate::test_support::fixture(directory.path());
    let scene = project.scenes.values_mut().next().unwrap();
    let moving_id = scene.entities.keys().next().unwrap().clone();
    scene
        .entities
        .get_mut(&moving_id)
        .unwrap()
        .components
        .insert("Velocity".into(), json!({"linear":[2.,0.,0.]}));
    let mut camera = Entity::new("Native camera");
    camera.id = new_id();
    let camera_id = incant_cook::stable_id(&camera.id).unwrap();
    camera.components.insert(
        "Transform".into(),
        json!(Transform {
            translation: [0., 2., 9.],
            ..Default::default()
        }),
    );
    camera.components.insert(
        "Camera".into(),
        json!({"fov_degrees":55.,"near":0.1,"far":80.}),
    );
    scene.entities.insert(camera.id.clone(), camera);
    let scene_id = scene.id.clone();
    let bytes = incant_cook::cook_scene(&project, &scene_id).unwrap();
    let mut world = NativeWorld::from_bytes(&bytes).unwrap();
    drop(bytes);
    drop(store);
    drop(project);
    let mut input = NativeRenderInput::load(
        &world.spatial_view().unwrap(),
        VisualCaches {
            models: &directory.path().join(".incant/cache/models"),
            textures: &directory.path().join(".incant/cache/textures"),
        },
        camera_id,
    )
    .unwrap();
    directory.close().unwrap();
    let renderer = Renderer::headless().unwrap();
    let mut gpu = renderer
        .prepare_retained(input.input(), input.camera_label())
        .unwrap();
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
    let initial = renderer.screenshot_scene_png(&gpu.live, 160, 96).unwrap();
    let mut rebuilt = 0;
    for frame in 0..12 {
        if frame == 6 {
            let mut spatial = CookedSpatial::empty(incant_cook::stable_id(&moving_id).unwrap());
            spatial.mesh = Some(NativeMeshRenderer {
                mesh: incant_cook::stable_id(&asset_id).unwrap(),
                cast_shadows: false,
            });
            let mut commands = FrameCommands::default();
            commands
                .push(StructuralCommand::SetSpatial {
                    entity: spatial.entity,
                    spatial: Some(spatial),
                })
                .unwrap();
            world.apply(&mut commands).unwrap();
        }
        world.step().unwrap();
        let spatial = world.spatial_view().unwrap();
        if frame == 3 {
            let old_tick = input.tick();
            let old_pose = input
                .input()
                .nodes
                .iter()
                .map(|n| n.world)
                .collect::<Vec<_>>();
            let before = renderer.screenshot_scene_png(&gpu.live, 160, 96).unwrap();
            assert!(
                input
                    .try_sync(&spatial, |scene, _, _| gpu.submit(
                        &renderer,
                        scene,
                        target(0)
                    ))
                    .is_err()
            );
            assert_eq!(input.tick(), old_tick);
            assert_eq!(
                input
                    .input()
                    .nodes
                    .iter()
                    .map(|n| n.world)
                    .collect::<Vec<_>>(),
                old_pose
            );
            assert_eq!(
                renderer.screenshot_scene_png(&gpu.live, 160, 96).unwrap(),
                before
            );
        }
        let replacement = input
            .try_sync(
                &spatial,
                |scene, camera, kind| -> Result<Option<RetainedScene>> {
                    match kind {
                        UpdateKind::Motion => {
                            gpu.submit(&renderer, scene, target(160))?;
                            Ok(None)
                        }
                        UpdateKind::Rebuilt => {
                            rebuilt += 1;
                            let mut candidate = renderer.prepare_retained(scene, camera)?;
                            candidate.submit(&renderer, scene, target(160))?;
                            Ok(Some(candidate))
                        }
                    }
                },
            )
            .unwrap();
        if let Some(candidate) = replacement {
            gpu = candidate;
        }
        assert_eq!(input.tick(), world.tick());
        let actual = read(&renderer, &output);
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
            "binary native frame {frame}"
        );
    }
    assert_eq!(rebuilt, 1);
    assert_ne!(
        initial,
        renderer.screenshot_scene_png(&gpu.live, 160, 96).unwrap()
    );
}
