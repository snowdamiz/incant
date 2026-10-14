use super::*;

#[test]
fn texture_publication_retires_intervening_ik_owner_after_the_lock() {
    let (dir, scene, definition) = prepare();
    let cook_texture = |value: u8| {
        let path = dir.path().join("refresh.png");
        let mut encoder = png::Encoder::new(std::fs::File::create(&path).unwrap(), 1, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&[value, 0, 0, 255])
            .unwrap();
        let key = incant_assets::cook_texture(
            dir.path(),
            Path::new("refresh.png"),
            &dir.path().join("textures"),
            incant_assets_runtime::TextureUsage::Color,
        )
        .unwrap()
        .metadata
        .fingerprint;
        VisualAssetRef {
            id: id(91),
            kind: VisualAssetKind::Texture,
            fingerprint: std::array::from_fn(|i| {
                u8::from_str_radix(&key[2 * i..2 * i + 2], 16).unwrap()
            }),
        }
    };
    let mut references = scene.visual_assets().to_vec();
    references.push(cook_texture(255));
    let mut nodes = scene.spatial().to_vec();
    let mesh = nodes.iter_mut().find(|n| n.entity == id(1)).unwrap();
    mesh.ik = Some(Arc::new(NativeIk {
        mesh_fingerprint: definition.mesh_fingerprint,
        initial: Arc::from([AnimationIkSample {
            id: id(9),
            chain: [0, 1, 2],
            target: [11., 1., 0.],
            pole: [10., 1., 0.],
            weight: 1.,
        }]),
    }));
    mesh.environment = Some(NativeEnvironmentLight {
        texture: id(91),
        intensity: 1.,
        rotation_degrees: 0.,
    });
    let scene = scene.with_spatial(references.clone(), nodes).unwrap();
    let mut world = NativeWorld::new(scene);
    let mut input = load(&dir, &world).unwrap();
    let move_ik = |world: &mut NativeWorld, weight| {
        world
            .apply(&mut queue([StructuralCommand::ControlIk {
                entity: id(1),
                control: IkControl {
                    constraint: id(9),
                    target: [12., 0.5, 0.],
                    pole: [10., 1., 0.],
                    weight,
                },
            }]))
            .unwrap();
    };
    move_ik(&mut world, 0.75);
    input
        .try_sync(&world.spatial_view().unwrap(), |_, _, _| Ok::<_, ()>(()))
        .unwrap();
    let staged_ik = Arc::downgrade(binding(&input, 1).animation_ik.as_ref().unwrap());
    let old_texture = input.input().environment.as_ref().unwrap().source.clone();
    let old_version = old_texture.version_id();
    references[1] = cook_texture(100);
    let prepared = input
        .texture_refresh_ticket()
        .prepare(&references, &dir.path().join("textures"))
        .unwrap();
    let mut staged = input
        .stage_texture_refresh(&world.spatial_view().unwrap(), prepared)
        .unwrap();
    move_ik(&mut world, 0.25);
    // The second sync clears the numeric rollback buffer's intermediate owner.
    for _ in 0..2 {
        input
            .try_sync(&world.spatial_view().unwrap(), |_, _, _| Ok::<_, ()>(()))
            .unwrap();
    }
    assert_eq!(staged_ik.strong_count(), 1);
    let current = binding(&input, 1).animation_ik.as_ref().unwrap().clone();
    let view = world.spatial_view().unwrap();
    let (result, allocations, disposals) = allocation_counter::record_lifecycle(|| {
        input.try_publish_textures(&view, &mut staged, || {
            // Weak does not keep the payload alive. The displaced staged owner
            // must still exist throughout this final publication callback.
            assert_eq!(staged_ik.upgrade().unwrap()[0].weight, 0.75);
            Ok::<_, ()>(())
        })
    });
    let receipt = result.unwrap();
    assert_eq!(receipt.changed, 1);
    assert_eq!((allocations, disposals), (0, 0));
    assert!(Arc::ptr_eq(
        binding(&input, 1).animation_ik.as_ref().unwrap(),
        &current
    ));
    assert_eq!(staged_ik.upgrade().unwrap()[0].weight, 0.75);
    assert_ne!(
        input
            .input()
            .environment
            .as_ref()
            .unwrap()
            .source
            .version_id(),
        old_texture.version_id()
    );
    drop(staged);
    assert!(staged_ik.upgrade().is_none());
    assert_eq!(current[0].weight, 0.25);
    assert_eq!(old_texture.version_id(), old_version);
    assert!(matches!(old_texture.data(), RuntimeAssetData::Texture(_)));
}

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
