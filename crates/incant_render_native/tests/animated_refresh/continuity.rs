use super::*;

#[test]
fn live_graph_phase_parameters_and_weighted_skin_pose_survive_clip_content_refresh() {
    let root = tempfile::tempdir().unwrap();
    let original = model(root.path(), Change::Original);
    let cooked = scene(root.path(), vec![original], false);
    let mut declarations = cooked.spatial().to_vec();
    let definition = Arc::make_mut(declarations[0].animation.as_mut().unwrap());
    definition.initial.clip = None;
    definition.graph = Some(Arc::new(
        NativeAnimationGraph::new(
            0,
            vec![NativeAnimationParameter {
                id: id(10),
                initial: 0.25,
                min: 0.,
                max: 1.,
            }],
            vec![NativeAnimationGraphState {
                id: id(20),
                looping: true,
                motion: NativeAnimationMotion::Blend1d {
                    parameter: 0,
                    cycle_seconds: 2.,
                    points: Arc::from([
                        NativeAnimationBlendPoint {
                            threshold: 0.,
                            clip: 0,
                        },
                        NativeAnimationBlendPoint {
                            threshold: 1.,
                            clip: 1,
                        },
                    ]),
                },
            }],
            vec![],
            &definition.clip_durations,
        )
        .unwrap(),
    ));
    let mut world = NativeWorld::new(cooked.with_spatial(vec![original], declarations).unwrap());
    let mut input = load(root.path(), &world).unwrap();
    for _ in 0..30 {
        world.step().unwrap();
    }
    sync(&mut input, &world);
    let old = mesh(&input).source.clone();
    let before = world.animation_graph_state(id(1)).unwrap();
    let next = model(root.path(), Change::Keys);
    let mut staged = input
        .stage_texture_refresh(
            &world.spatial_view().unwrap(),
            prepared(&input, root.path(), &[next]),
        )
        .unwrap();
    input
        .try_publish_textures(&world.spatial_view().unwrap(), &mut staged, || {
            Ok::<_, ()>(())
        })
        .unwrap();
    assert_eq!(world.animation_graph_state(id(1)), Some(before));
    let primary = mesh(&input).animation.unwrap();
    let blend = mesh(&input).animation_blend.unwrap();
    assert_eq!(blend.weight, 0.25);
    let sample = |source: &RuntimeAsset| {
        let RuntimeAssetData::Model(model) = source.data() else {
            panic!()
        };
        let mut player = AnimationPlayer::new(model).unwrap();
        player
            .sample_blend(
                incant_assets_runtime::animation::ClipSample {
                    clip: blend.sample.clip.map(usize::from),
                    seconds: blend.sample.seconds,
                },
                incant_assets_runtime::animation::ClipSample {
                    clip: primary.clip.map(usize::from),
                    seconds: primary.seconds,
                },
                blend.weight,
            )
            .unwrap();
        let v = model.meshes[0].vertices[0];
        player.skin_position(3, 0, 0, [v[0], v[1], v[2]]).unwrap()
    };
    let a = sample(&old);
    let b = sample(&mesh(&input).source);
    assert!((a[0] - 11.375).abs() < 1e-6);
    assert!((b[0] - 11.75).abs() < 1e-6);
    root.close().unwrap();
    world.step().unwrap();
    sync(&mut input, &world);
    assert!((mesh(&input).animation.unwrap().seconds - 31. / 60.).abs() < 1e-12);
    assert_eq!(mesh(&input).animation_blend.unwrap().weight, 0.25);
}

#[test]
fn compatible_refresh_keeps_crossfade_state_and_original_authority_through_structural_rebind() {
    let root = tempfile::tempdir().unwrap();
    let original = model(root.path(), Change::Original);
    let mut world = NativeWorld::new(scene(root.path(), vec![original], false));
    let mut input = load(root.path(), &world).unwrap();
    for _ in 0..12 {
        world.step().unwrap();
    }
    apply(
        &mut world,
        StructuralCommand::ControlAnimation {
            entity: id(1),
            control: AnimationControl::Crossfade {
                target: AnimationSample {
                    clip: Some(1),
                    seconds: 0.25,
                },
                duration: 1.,
            },
        },
    );
    world.step().unwrap();
    sync(&mut input, &world);
    let next = model(root.path(), Change::Keys);
    let mut stage = input
        .stage_texture_refresh(
            &world.spatial_view().unwrap(),
            prepared(&input, root.path(), &[next]),
        )
        .unwrap();
    world.step().unwrap();
    sync(&mut input, &world);
    let state = world.animation_state(id(1)).unwrap();
    let sample = mesh(&input).animation;
    let blend = mesh(&input).animation_blend;
    let pose = mesh(&input).animation_pose;
    assert!(blend.is_some());
    input
        .try_publish_textures(&world.spatial_view().unwrap(), &mut stage, || {
            Ok::<_, ()>(())
        })
        .unwrap();
    assert_eq!(world.animation_state(id(1)), Some(state));
    assert_eq!(
        (
            mesh(&input).animation,
            mesh(&input).animation_blend,
            mesh(&input).animation_pose
        ),
        (sample, blend, pose)
    );
    // The complete catalog remains content-checked at fresh startup. A new
    // model body under the old key cannot acquire the old world authority.
    let old_cache = caches(root.path())
        .0
        .join(format!("{}.incmodel", key(original)));
    let saved = fs::read(&old_cache).unwrap();
    fs::write(
        &old_cache,
        fs::read(
            caches(root.path())
                .0
                .join(format!("{}.incmodel", key(next))),
        )
        .unwrap(),
    )
    .unwrap();
    assert!(matches!(
        load(root.path(), &world),
        Err(NativeRenderError::Asset { .. })
    ));
    fs::write(old_cache, saved).unwrap();
    let mut declaration = world
        .spatial_view()
        .unwrap()
        .nodes()
        .find(|node| node.id == id(1))
        .unwrap()
        .declaration
        .unwrap()
        .clone();
    declaration.mesh.as_mut().unwrap().cast_shadows = false;
    root.close().unwrap();
    apply(
        &mut world,
        StructuralCommand::SetSpatial {
            entity: id(1),
            spatial: Some(declaration),
        },
    );
    assert_eq!(world.animation_state(id(1)), Some(state));
    input
        .try_sync(&world.spatial_view().unwrap(), |_, _, kind| {
            assert_eq!(kind, UpdateKind::Rebuilt);
            Ok::<_, ()>(())
        })
        .unwrap();
    assert_eq!(mesh(&input).source.info().fingerprint, key(next));
    assert_eq!(mesh(&input).animation, Some(state.sample));
    assert_eq!(mesh(&input).animation_blend, blend);
}

#[test]
fn material_rebinding_keeps_current_crossfade_root_and_deformation_without_final_allocation() {
    let root = tempfile::tempdir().unwrap();
    let original = model(root.path(), Change::Original);
    let mut world = NativeWorld::new(scene(root.path(), vec![original], true));
    let mut input = load(root.path(), &world).unwrap();
    for _ in 0..12 {
        world.step().unwrap();
    }
    apply(
        &mut world,
        StructuralCommand::ControlAnimation {
            entity: id(1),
            control: AnimationControl::Crossfade {
                target: AnimationSample {
                    clip: Some(1),
                    seconds: 0.25,
                },
                duration: 1.,
            },
        },
    );
    world.step().unwrap();
    sync(&mut input, &world);
    let old = mesh(&input).source.clone();
    let next = model(root.path(), Change::MaterialBindings);
    let mut staged = input
        .stage_texture_refresh(
            &world.spatial_view().unwrap(),
            prepared(&input, root.path(), &[next]),
        )
        .unwrap();
    world.step().unwrap();
    sync(&mut input, &world);
    let before = world.animation_state(id(1)).unwrap();
    let pose = (
        mesh(&input).animation,
        mesh(&input).animation_blend,
        mesh(&input).animation_pose,
        mesh(&input).animation_root,
    );
    assert!(pose.1.is_some());
    let mut before_root = Vec::new();
    world.snapshot_root_motion(&mut before_root).unwrap();
    let (result, count, disposals) = allocations::record_lifecycle(|| {
        input.try_publish_textures(&world.spatial_view().unwrap(), &mut staged, || {
            Ok::<_, ()>(())
        })
    });
    result.unwrap();
    assert_eq!((count, disposals), (0, 0));
    assert_eq!(world.animation_state(id(1)), Some(before));
    assert_eq!(
        (
            mesh(&input).animation,
            mesh(&input).animation_blend,
            mesh(&input).animation_pose,
            mesh(&input).animation_root
        ),
        pose
    );
    let mut after_root = Vec::new();
    world.snapshot_root_motion(&mut after_root).unwrap();
    assert_eq!(before_root, after_root);
    let RuntimeAssetData::Model(previous) = old.data() else {
        panic!("model")
    };
    let RuntimeAssetData::Model(current) = mesh(&input).source.data() else {
        panic!("model")
    };
    assert_eq!(previous.meshes, current.meshes);
    assert_eq!(previous.metadata.mesh_materials, [Some(0)]);
    assert_eq!(current.metadata.mesh_materials, [Some(1)]);
    assert_eq!(current.materials.len(), 2);
    assert!(current.materials[1].maps[2].is_some());
    assert_eq!(
        position(&old, before.sample),
        position(&mesh(&input).source, before.sample)
    );
    root.close().unwrap();
    world.step().unwrap();
    sync(&mut input, &world);
    assert_eq!(world.tick(), 15);
    assert_eq!(old.info().fingerprint, key(original));
}
