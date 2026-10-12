use super::*;
use incant_assets_runtime::animation::{
    ClipSample, PoseSource, TwoBoneIk, compile_root_motion_catalog,
};
use incant_types::RootMotionApplication;

fn root_scene() -> (tempfile::TempDir, CookedScene, Arc<NativeAnimation>) {
    let directory = tempfile::tempdir().unwrap();
    let mut doc = fixture::fixture(directory.path());
    // Keep the selected locomotion joint rigid; the descendant scale track is
    // retained without weakening the catalog's root-scale refusal.
    doc["animations"][2]["channels"][0]["target"]["node"] = serde_json::json!(2);
    fixture::write(directory.path(), &doc);
    let cooked = incant_assets::cook_gltf(
        directory.path(),
        Path::new("model.gltf"),
        &directory.path().join("models"),
    )
    .unwrap();
    let fingerprint = std::array::from_fn(|i| {
        u8::from_str_radix(&cooked.metadata.fingerprint[2 * i..2 * i + 2], 16).unwrap()
    });
    let definition = definition(fingerprint);
    let model = incant_assets_runtime::load_model(
        &directory.path().join("models"),
        &cooked.metadata.fingerprint,
    )
    .unwrap();
    let root = Arc::new(NativeRootMotion {
        catalog: Arc::new(compile_root_motion_catalog(&model, fingerprint, 1).unwrap()),
        application: RootMotionApplication::Transform,
        applying: true,
    });
    let mut node = mesh(1, Some(definition.clone()));
    node.root_motion = Some(root);
    let mut owner = entity(1, None);
    owner.velocity = None;
    owner.transform = Default::default();
    let scene = CookedScene::new(id(99), 60, vec![owner, entity(2, None)])
        .unwrap()
        .with_spatial(
            vec![VisualAssetRef {
                id: id(90),
                kind: VisualAssetKind::Model,
                fingerprint,
            }],
            vec![node, declaration(2)],
        )
        .unwrap();
    (
        directory,
        CookedScene::from_bytes(&scene.to_bytes()).unwrap(),
        definition,
    )
}

#[test]
fn source_deleted_root_transport_strips_pose_once_before_ik_and_rolls_back_rejected_consumer() {
    let (dir, scene, definition) = root_scene();
    let mut world = NativeWorld::new(scene);
    let mut input = load(&dir, &world).unwrap();
    let source = binding(&input, 1).source.clone();
    let RuntimeAssetData::Model(model) = source.data() else {
        panic!()
    };
    let mut player = AnimationPlayer::new(model).unwrap();
    let ik = [TwoBoneIk {
        root: 0,
        middle: 1,
        tip: 2,
        target: [11., 1., 0.],
        pole: [10., 1., 0.],
        weight: 1.,
    }];
    let rest = model.meshes[0].vertices[0];
    let owner = format!("{:032x}", 1);
    dir.close().unwrap();
    for n in 1..=80 {
        world.step().unwrap();
        let (_, count) = allocation_counter::record(|| {
            input
                .try_sync(&world.spatial_view().unwrap(), |candidate, _, _| {
                    let mesh = candidate
                        .nodes
                        .iter()
                        .find(|n| n.id == owner)
                        .unwrap()
                        .mesh
                        .as_ref()
                        .unwrap();
                    assert_eq!(mesh.animation_root, Some(1));
                    let sample = mesh.animation.unwrap();
                    let pose = PoseSource::Clip(ClipSample {
                        clip: sample.clip.map(usize::from),
                        seconds: sample.seconds,
                    });
                    player.sample_pose_pipeline(pose, pose, 0., mesh.animation_root, &ik)
                })
                .unwrap()
        });
        assert_eq!(count, 0);
        assert!(
            (world.inspect(id(1)).unwrap().transform.translation[0] - n as f64 / 60.).abs() < 2e-6
        );
        let vertex = player
            .skin_position(3, 0, 0, [rest[0], rest[1], rest[2]])
            .unwrap();
        assert!((vertex[0] - 11.).abs() < 2e-6);
    }
    let old = player.globals().to_vec();
    let old_sample = binding(&input, 1).animation;
    let refused = input.try_sync(&world.spatial_view().unwrap(), |_, _, _| {
        Err::<(), _>("consumer")
    });
    assert!(matches!(refused, Err(SyncError::Consumer("consumer"))));
    assert_eq!(binding(&input, 1).animation, old_sample);
    assert_eq!(binding(&input, 1).animation_root, Some(1));
    let pose = PoseSource::Clip(ClipSample {
        clip: Some(0),
        seconds: 1.,
    });
    assert!(
        player
            .sample_pose_pipeline(pose, pose, 0., Some(u16::MAX), &ik)
            .is_err()
    );
    assert_eq!(player.globals(), old);
    let (_, allocations) = allocation_counter::record(|| {
        for _ in 0..1024 {
            player
                .sample_pose_pipeline(pose, pose, 0., Some(1), &ik)
                .unwrap();
        }
    });
    assert_eq!(allocations, 0);
    // Removing root extraction is an explicit structural rebind, not tied to
    // whether its authored body-application switch is currently enabled.
    world
        .apply(&mut queue([StructuralCommand::SetSpatial {
            entity: id(1),
            spatial: Some(mesh(1, Some(definition))),
        }]))
        .unwrap();
    input
        .try_sync(&world.spatial_view().unwrap(), |_, _, _| Ok::<_, ()>(()))
        .unwrap();
    assert!(binding(&input, 1).animation_root.is_none());
}
