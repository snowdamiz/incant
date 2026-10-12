use super::*;
use incant_assets_runtime::animation::ClipSample;

fn samples(
    input: &NativeRenderInput,
    id: u128,
) -> (Option<AnimationSample>, Option<AnimationBlendSample>) {
    let mesh = binding(input, id);
    (mesh.animation, mesh.animation_blend)
}
fn start(world: &mut NativeWorld) {
    control(
        world,
        1,
        AnimationControl::Crossfade {
            target: AnimationSample {
                clip: Some(0),
                seconds: 0.5,
            },
            duration: 1.,
        },
    );
}

#[test]
fn native_fade_samples_survive_source_deletion_and_reuse_motion_storage() {
    let (dir, scene, _) = prepare();
    let mut world = NativeWorld::new(scene);
    start(&mut world);
    // Cold binding must include a transition already active before host startup.
    let mut input = load(&dir, &world).unwrap();
    assert_eq!(samples(&input, 1).1.unwrap().weight, 0.);
    let source = binding(&input, 1).source.clone();
    let RuntimeAssetData::Model(model) = source.data() else {
        panic!()
    };
    let mut player = AnimationPlayer::new(model).unwrap();
    let ptr = input.input().nodes.as_ptr();
    let refs = Arc::strong_count(&source);
    dir.close().unwrap();
    for tick in 1..=60 {
        world.step().unwrap();
        let (result, allocations) = allocation_counter::record(|| {
            input.try_sync(&world.spatial_view().unwrap(), |_, _, kind| {
                assert_eq!(kind, UpdateKind::Motion);
                Ok::<_, ()>(())
            })
        });
        result.unwrap();
        assert_eq!(allocations, 0);
        let (primary, blend) = samples(&input, 1);
        let primary = primary.unwrap();
        let state = world.animation_state(id(1)).unwrap();
        assert_eq!(primary, state.sample);
        assert_eq!(blend, state.blend_sample());
        if let Some(blend) = blend {
            player
                .sample_blend(
                    ClipSample {
                        clip: blend.sample.clip.map(usize::from),
                        seconds: blend.sample.seconds,
                    },
                    ClipSample {
                        clip: primary.clip.map(usize::from),
                        seconds: primary.seconds,
                    },
                    blend.weight,
                )
                .unwrap();
            assert!((blend.weight - tick as f64 / 60.).abs() < 1e-12);
        } else {
            assert_eq!(tick, 60);
            player
                .sample(primary.clip.map(usize::from), primary.seconds)
                .unwrap();
        }
        let vertex = model.meshes[0].vertices[0];
        let position = player
            .skin_position(3, 0, 0, [vertex[0], vertex[1], vertex[2]])
            .unwrap();
        // Both move clips have unit velocity. The destination starts 0.5s
        // ahead, so linear crossfade adds 0.5 * tick/60 to the source clock.
        assert!((position[0] - (11. + 1.5 * tick as f64 / 60.)).abs() < 2e-6);
        assert_eq!(samples(&input, 3).1, None);
        assert_eq!(binding(&input, 3).animation.unwrap().seconds, 0.);
        assert_eq!(input.input().nodes.as_ptr(), ptr);
        assert_eq!(Arc::strong_count(&source), refs);
    }
    assert_eq!(input.binding_generation(), 0);
}

#[test]
fn rejected_motion_unwind_and_rebuild_preserve_both_samples_then_retry() {
    let (dir, scene, _) = prepare();
    let mut world = NativeWorld::new(scene);
    let mut input = load(&dir, &world).unwrap();
    dir.close().unwrap();
    start(&mut world);
    world.step().unwrap();
    let old = samples(&input, 1);
    let old_matrices = matrices(&input);
    let old_generations = generations(&input);
    let (rejected, allocations) = allocation_counter::record(|| {
        input.try_sync(&world.spatial_view().unwrap(), |scene, _, kind| {
            assert_eq!(kind, UpdateKind::Motion);
            assert!(
                scene
                    .nodes
                    .iter()
                    .any(|n| n.mesh.as_ref().is_some_and(|m| m.animation_blend.is_some()))
            );
            Err::<(), _>("target")
        })
    });
    assert!(matches!(rejected, Err(SyncError::Consumer("target"))));
    assert_eq!(allocations, 0);
    assert_eq!(samples(&input, 1), old);
    assert_eq!(matrices(&input), old_matrices);
    assert_eq!(generations(&input), old_generations);
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = input.try_sync(
                &world.spatial_view().unwrap(),
                |_, _, _| -> Result<(), ()> { panic!("target") },
            );
        }))
        .is_err()
    );
    assert_eq!(samples(&input, 1), old);
    assert_eq!(matrices(&input), old_matrices);
    world
        .apply(&mut queue([StructuralCommand::Spawn(entity(4, None))]))
        .unwrap();
    let before = input.input().nodes.as_ptr();
    assert!(
        input
            .try_sync(&world.spatial_view().unwrap(), |_, _, kind| {
                assert_eq!(kind, UpdateKind::Rebuilt);
                Err::<(), _>(())
            })
            .is_err()
    );
    assert_eq!(input.input().nodes.as_ptr(), before);
    assert_eq!(samples(&input, 1), old);
    input
        .try_sync(&world.spatial_view().unwrap(), |_, _, kind| {
            assert_eq!(kind, UpdateKind::Rebuilt);
            Ok::<_, ()>(())
        })
        .unwrap();
    let state = world.animation_state(id(1)).unwrap();
    assert_eq!(
        samples(&input, 1),
        (Some(state.sample), state.blend_sample())
    );
    let accepted = samples(&input, 1);
    assert!(accepted.1.is_some());
    world.step().unwrap();
    let (rejected, allocations) = allocation_counter::record(|| {
        input.try_sync(&world.spatial_view().unwrap(), |_, _, _| Err::<(), _>(()))
    });
    assert!(rejected.is_err());
    assert_eq!(allocations, 0);
    assert_eq!(samples(&input, 1), accepted);
    control(
        &mut world,
        1,
        AnimationControl::Select(AnimationSample::REST),
    );
    // Rejection of cancellation must restore an existing outgoing sample,
    // rather than merely clearing it to the no-transition default.
    assert!(
        input
            .try_sync(&world.spatial_view().unwrap(), |_, _, _| Err::<(), _>(()))
            .is_err()
    );
    assert_eq!(samples(&input, 1), accepted);
    input
        .try_sync(&world.spatial_view().unwrap(), |_, _, _| Ok::<_, ()>(()))
        .unwrap();
    assert_eq!(samples(&input, 1), (Some(AnimationSample::REST), None));
}
