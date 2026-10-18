use super::*;
use incant_render_native::interpolation::{PoseError, PoseRenderInput, PoseSyncError};
use std::panic::{AssertUnwindSafe, catch_unwind};

fn pose(dir: &tempfile::TempDir, world: &NativeWorld) -> PoseRenderInput {
    PoseRenderInput::load(
        &world.spatial_view().unwrap(),
        VisualCaches {
            models: &dir.path().join("models"),
            textures: &dir.path().join("textures"),
        },
        id(2),
        0,
    )
    .unwrap()
}
#[test]
fn rigid_child_mesh_animation_uses_the_same_sampled_local_pose_blend() {
    let (dir, scene, _) = prepare_edited(|_, doc| {
        // Same local animation channels, now a rigid child mesh with no skin.
        // This is the binding used by Core's bobbing/spinning actor models.
        doc.as_object_mut().unwrap().remove("skins");
        for index in [3, 4] {
            doc["nodes"][index].as_object_mut().unwrap().remove("skin");
        }
        let attributes = doc["meshes"][0]["primitives"][0]["attributes"]
            .as_object_mut()
            .unwrap();
        attributes.remove("JOINTS_0");
        attributes.remove("WEIGHTS_0");
        doc["nodes"][1]["children"] = serde_json::json!([2, 3]);
        doc["nodes"][3]["translation"] = serde_json::json!([0, 0, 0]);
        doc["scenes"][0]["nodes"] = serde_json::json!([0, 4]);
    });
    let mut world = NativeWorld::new(scene);
    let mut input = pose(&dir, &world);
    let source = binding(input.committed(), 1).source.clone();
    let RuntimeAssetData::Model(model) = source.data() else {
        panic!()
    };
    assert!(model.animation.as_ref().unwrap().skins.is_empty());
    let mut player = AnimationPlayer::new(model).unwrap();
    world.step().unwrap();
    input
        .try_sync(&world.spatial_view().unwrap(), 0, |_, _, _| Ok::<_, ()>(()))
        .unwrap();
    dir.close().unwrap();
    for fraction in [0., 0.5, 1.] {
        input
            .present(fraction, |scene, _, _| {
                let mesh = scene
                    .nodes
                    .iter()
                    .find(|n| n.id == format!("{:032x}", 1))
                    .unwrap()
                    .mesh
                    .as_ref()
                    .unwrap();
                incant_render_data::animation_sample::sample_committed(&mut player, mesh).unwrap();
                assert!((player.globals()[3].w_axis.x - (11. + fraction / 60.)).abs() < 1e-7);
                Ok::<_, ()>(())
            })
            .unwrap();
    }
    assert_eq!(world.tick(), 1);
    assert!(binding(input.committed(), 1).animation_blend.is_none());
}
#[test]
fn rendered_animation_uses_accepted_endpoints_without_advancing_world_or_losing_canonical_input() {
    let (dir, scene, _) = prepare();
    let mut world = NativeWorld::new(scene);
    let mut input = pose(&dir, &world);
    let source = binding(input.committed(), 1).source.clone();
    let RuntimeAssetData::Model(model) = source.data() else {
        panic!()
    };
    let mut player = AnimationPlayer::new(model).unwrap();
    dir.close().unwrap();
    for _ in 0..8 {
        world.step().unwrap();
        input
            .try_sync(&world.spatial_view().unwrap(), 0, |_, _, _| Ok::<_, ()>(()))
            .unwrap();
    }
    let state = world.animation_state(id(1)).unwrap();
    let canonical = binding(input.committed(), 1).animation;
    let worlds = matrices(input.committed());
    for fraction in [0., 0.25, 0.5, 1.] {
        input
            .present(fraction, |scene, _, receipt| {
                let mesh = scene
                    .nodes
                    .iter()
                    .find(|n| n.id == format!("{:032x}", 1))
                    .unwrap()
                    .mesh
                    .as_ref()
                    .unwrap();
                incant_render_data::animation_sample::sample_committed(&mut player, mesh).unwrap();
                let vertex = model.meshes[0].vertices[0];
                let position = player
                    .skin_position(3, 0, 0, [vertex[0], vertex[1], vertex[2]])
                    .unwrap();
                assert!((f64::from(position[0]) - (11. + (7. + fraction) / 60.)).abs() < 1e-5);
                assert_eq!((receipt.previous_tick, receipt.current_tick), (7, 8));
                Ok::<_, ()>(())
            })
            .unwrap();
        assert_eq!(binding(input.committed(), 1).animation, canonical);
        assert!(binding(input.committed(), 1).animation_blend.is_none());
        assert!(Arc::ptr_eq(&binding(input.committed(), 1).source, &source));
        assert_eq!(matrices(input.committed()), worlds);
        assert_eq!(world.animation_state(id(1)), Some(state));
        assert_eq!(world.tick(), 8);
    }
    assert!(
        input
            .present(0.4, |_, _, _| Err::<(), _>("display refused"))
            .is_err()
    );
    assert!(
        catch_unwind(AssertUnwindSafe(|| input
            .present(0.3, |_, _, _| -> Result<(), ()> {
                panic!("display unwind")
            })))
        .is_err()
    );
    assert_eq!(binding(input.committed(), 1).animation, canonical);
    assert!(binding(input.committed(), 1).animation_blend.is_none());
    control(&mut world, 1, AnimationControl::Seek(1.));
    assert!(matches!(
        input.try_sync(&world.spatial_view().unwrap(), 0, |_, _, _| Ok::<_, ()>(())),
        Err(PoseSyncError::Pose(PoseError::SameTickMutation))
    ));
    input
        .try_sync(&world.spatial_view().unwrap(), 1, |_, _, _| Ok::<_, ()>(()))
        .unwrap();
    input
        .present(0., |scene, _, _| {
            let mesh = scene
                .nodes
                .iter()
                .find(|n| n.id == format!("{:032x}", 1))
                .unwrap()
                .mesh
                .as_ref()
                .unwrap();
            assert_eq!(mesh.animation.unwrap().seconds, 1.);
            assert!(mesh.animation_blend.is_none());
            Ok::<_, ()>(())
        })
        .unwrap();
}

#[test]
fn loop_boundary_blends_local_poses_instead_of_interpolating_wrapped_seconds() {
    let (dir, scene, _) = prepare_edited(|path, doc| {
        // A closed triangular translation curve. The two poses across wrap are
        // both near rest; sampling their mean clock would incorrectly hit its peak.
        let mut bytes = std::fs::read(path.join("model.bin")).unwrap();
        let mut add = |values: &[f32], kind: &str| {
            let offset = bytes.len();
            bytes.extend(values.iter().flat_map(|x| x.to_le_bytes()));
            let view = doc["bufferViews"].as_array().unwrap().len();
            doc["bufferViews"].as_array_mut().unwrap().push(
                serde_json::json!({"buffer":0,"byteOffset":offset,"byteLength":bytes.len()-offset}),
            );
            let accessor = doc["accessors"].as_array().unwrap().len();
            doc["accessors"].as_array_mut().unwrap().push(
                serde_json::json!({"bufferView":view,"componentType":5126,"count":3,"type":kind}),
            );
            accessor
        };
        let times = add(&[0., 1., 2.], "SCALAR");
        let positions = add(&[1., 0., 0., 6., 0., 0., 1., 0., 0.], "VEC3");
        doc["accessors"][times]["min"] = serde_json::json!([0]);
        doc["accessors"][times]["max"] = serde_json::json!([2]);
        doc["animations"][0]["samplers"][0]["input"] = times.into();
        doc["animations"][0]["samplers"][0]["output"] = positions.into();
        doc["buffers"][0]["byteLength"] = bytes.len().into();
        std::fs::write(path.join("model.bin"), bytes).unwrap();
    });
    let mut world = NativeWorld::new(scene);
    control(&mut world, 1, AnimationControl::Seek(1.99));
    let mut input = pose(&dir, &world);
    let source = binding(input.committed(), 1).source.clone();
    let RuntimeAssetData::Model(model) = source.data() else {
        panic!()
    };
    let mut player = AnimationPlayer::new(model).unwrap();
    let before = world.animation_state(id(1)).unwrap().sample.seconds;
    world.step().unwrap();
    input
        .try_sync(&world.spatial_view().unwrap(), 0, |_, _, _| Ok::<_, ()>(()))
        .unwrap();
    let after = world.animation_state(id(1)).unwrap().sample.seconds;
    assert!(before > 1.9 && after < 0.1);
    input
        .present(0.5, |scene, _, _| {
            let mesh = scene
                .nodes
                .iter()
                .find(|n| n.id == format!("{:032x}", 1))
                .unwrap()
                .mesh
                .as_ref()
                .unwrap();
            incant_render_data::animation_sample::sample_committed(&mut player, mesh).unwrap();
            let vertex = model.meshes[0].vertices[0];
            let actual = player
                .skin_position(3, 0, 0, [vertex[0], vertex[1], vertex[2]])
                .unwrap()[0];
            let expected = 11. + (5. * (2. - before) + 5. * after) * 0.5;
            assert!((f64::from(actual) - expected).abs() < 1e-5);
            assert!(
                actual < 11.1,
                "wrapped clock midpoint would select the curve peak"
            );
            Ok::<_, ()>(())
        })
        .unwrap();
    let accepted = binding(input.committed(), 1).animation;
    control(
        &mut world,
        1,
        AnimationControl::Crossfade {
            target: AnimationSample {
                clip: Some(1),
                seconds: 0.,
            },
            duration: 1.,
        },
    );
    assert!(matches!(
        input.try_sync(&world.spatial_view().unwrap(), 1, |_, _, _| Ok::<_, ()>(())),
        Err(PoseSyncError::Pose(PoseError::Unsupported))
    ));
    assert_eq!(binding(input.committed(), 1).animation, accepted);
}
