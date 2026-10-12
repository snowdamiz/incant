use super::*;
#[test]
fn cold_motion_and_rejected_consumer_keep_exact_ik_owner_payload() {
    let (dir, scene, definition) = prepare();
    let mut nodes = scene.spatial().to_vec();
    nodes.iter_mut().find(|n| n.entity == id(1)).unwrap().ik = Some(Arc::new(NativeIk {
        mesh_fingerprint: definition.mesh_fingerprint,
        initial: Arc::from([AnimationIkSample {
            id: id(9),
            chain: [0, 1, 2],
            target: [11., 1., 0.],
            pole: [10., 1., 0.],
            weight: 1.,
        }]),
    }));
    let scene = scene
        .clone()
        .with_spatial(scene.visual_assets().to_vec(), nodes)
        .unwrap();
    let mut world = NativeWorld::new(scene);
    let mut input = load(&dir, &world).unwrap();
    let original = binding(&input, 1).animation_ik.as_ref().unwrap().clone();
    dir.close().unwrap();
    world
        .apply(&mut queue([StructuralCommand::ControlIk {
            entity: id(1),
            control: IkControl {
                constraint: id(9),
                target: [12., 0.5, 0.],
                pole: [10., 1., 0.],
                weight: 0.5,
            },
        }]))
        .unwrap();
    let result = input.try_sync(&world.spatial_view().unwrap(), |scene, _, kind| {
        assert_eq!(kind, UpdateKind::Motion);
        let ik = scene
            .nodes
            .iter()
            .find_map(|n| n.mesh.as_ref()?.animation_ik.as_ref())
            .unwrap();
        assert_eq!(ik[0].target, [12., 0.5, 0.]);
        Err::<(), _>("refusal")
    });
    assert!(matches!(result, Err(SyncError::Consumer("refusal"))));
    assert!(Arc::ptr_eq(
        binding(&input, 1).animation_ik.as_ref().unwrap(),
        &original
    ));
    input
        .try_sync(&world.spatial_view().unwrap(), |_, _, _| Ok::<_, ()>(()))
        .unwrap();
    let accepted = binding(&input, 1).animation_ik.as_ref().unwrap().clone();
    assert_eq!(accepted[0].weight, 0.5);
    let ((), count) = allocation_counter::record(|| {
        for _ in 0..128 {
            input
                .try_sync(&world.spatial_view().unwrap(), |_, _, _| Ok::<_, ()>(()))
                .unwrap();
        }
    });
    assert_eq!(count, 0);
    assert!(Arc::ptr_eq(
        binding(&input, 1).animation_ik.as_ref().unwrap(),
        &accepted
    ));
    // A rebind removes the descriptor without leaving a retained IK ghost.
    world
        .apply(&mut queue([StructuralCommand::SetSpatial {
            entity: id(1),
            spatial: Some(mesh(1, Some(definition))),
        }]))
        .unwrap();
    input
        .try_sync(&world.spatial_view().unwrap(), |_, _, kind| {
            assert_eq!(kind, UpdateKind::Rebuilt);
            Ok::<_, ()>(())
        })
        .unwrap();
    assert!(binding(&input, 1).animation_ik.is_none());
}

#[test]
fn actual_sampled_pose_ik_failure_rolls_back_canonical_consumer_payload() {
    use incant_assets_runtime::animation::{TwoBoneIk, TwoBoneIkError};
    let (dir, scene, definition) = prepare();
    let mut nodes = scene.spatial().to_vec();
    nodes.iter_mut().find(|n| n.entity == id(1)).unwrap().ik = Some(Arc::new(NativeIk {
        mesh_fingerprint: definition.mesh_fingerprint,
        initial: Arc::from([AnimationIkSample {
            id: id(9),
            chain: [0, 1, 2],
            target: [11., 1., 0.],
            pole: [10., 1., 0.],
            weight: 1.,
        }]),
    }));
    let initial = scene
        .clone()
        .with_spatial(scene.visual_assets().to_vec(), nodes)
        .unwrap();
    let mut world = NativeWorld::new(initial);
    let mut input = load(&dir, &world).unwrap();
    let source = binding(&input, 1).source.clone();
    let RuntimeAssetData::Model(model) = source.data() else {
        panic!()
    };
    let mut staging = AnimationPlayer::new(model).unwrap();
    let old_sample = binding(&input, 1).animation;
    let old_ik = binding(&input, 1).animation_ik.as_ref().unwrap().clone();
    dir.close().unwrap();
    control(
        &mut world,
        1,
        AnimationControl::Select(incant_types::AnimationSample {
            clip: Some(2),
            seconds: 2.,
        }),
    );
    let rejected = input.try_sync(&world.spatial_view().unwrap(), |candidate, _, _| {
        let mesh = candidate
            .nodes
            .iter()
            .find(|n| n.id == format!("{:032x}", 1))
            .unwrap()
            .mesh
            .as_ref()
            .unwrap();
        let sample = mesh.animation.unwrap();
        staging
            .sample(sample.clip.map(usize::from), sample.seconds)
            .unwrap();
        let ik = mesh.animation_ik.as_ref().unwrap()[0];
        staging.apply_two_bone_ik(&[TwoBoneIk {
            root: 0,
            middle: 1,
            tip: 2,
            target: ik.target,
            pole: ik.pole,
            weight: ik.weight,
        }])
    });
    assert!(matches!(
        rejected,
        Err(SyncError::Consumer(TwoBoneIkError::UnsupportedTransform))
    ));
    assert_eq!(binding(&input, 1).animation, old_sample);
    assert!(Arc::ptr_eq(
        binding(&input, 1).animation_ik.as_ref().unwrap(),
        &old_ik
    ));
    control(
        &mut world,
        1,
        AnimationControl::Select(incant_types::AnimationSample::REST),
    );
    input
        .try_sync(&world.spatial_view().unwrap(), |candidate, _, _| {
            let mesh = candidate
                .nodes
                .iter()
                .find(|n| n.id == format!("{:032x}", 1))
                .unwrap()
                .mesh
                .as_ref()
                .unwrap();
            let s = mesh.animation.unwrap();
            staging.sample(s.clip.map(usize::from), s.seconds).unwrap();
            let ik = mesh.animation_ik.as_ref().unwrap()[0];
            staging.apply_two_bone_ik(&[TwoBoneIk {
                root: 0,
                middle: 1,
                tip: 2,
                target: ik.target,
                pole: ik.pole,
                weight: ik.weight,
            }])
        })
        .unwrap();
    let tip = staging.globals()[2].w_axis;
    assert!((tip.x - 11.).abs() < 1e-9 && (tip.y - 1.).abs() < 1e-9);
}
