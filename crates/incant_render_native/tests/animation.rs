#[path = "common/allocations.rs"]
mod allocation_counter;
mod common;
#[path = "../../incant_assets/tests/skeletal/fixture.rs"]
mod fixture;
use common::*;
use incant_assets_runtime::{RuntimeAssetData, animation::AnimationPlayer};
use incant_render_data::MeshInput;
use incant_render_native::{
    NativeRenderError, NativeRenderInput, SyncError, UpdateKind, VisualCaches,
};
use incant_runtime::*;
use incant_types::Animation;
use std::{path::Path, sync::Arc};

fn definition(fingerprint: [u8; 32]) -> Arc<NativeAnimation> {
    Arc::new(NativeAnimation {
        mesh_fingerprint: fingerprint,
        clip_durations: Arc::from([2.; 3]),
        initial: Animation {
            clip: Some(0),
            seconds: 0.,
            speed: 1.,
            playing: true,
            looping: true,
        },
    })
}
fn mesh(n: u128, animation: Option<Arc<NativeAnimation>>) -> CookedSpatial {
    let mut node = CookedSpatial::empty(id(n));
    node.mesh = Some(NativeMeshRenderer {
        mesh: id(90),
        cast_shadows: true,
    });
    node.animation = animation;
    node
}
fn prepare() -> (tempfile::TempDir, CookedScene, Arc<NativeAnimation>) {
    let directory = tempfile::tempdir().unwrap();
    fixture::fixture(directory.path());
    let model = incant_assets::cook_gltf(
        directory.path(),
        Path::new("model.gltf"),
        &directory.path().join("models"),
    )
    .unwrap();
    let fingerprint = std::array::from_fn(|i| {
        u8::from_str_radix(&model.metadata.fingerprint[2 * i..2 * i + 2], 16).unwrap()
    });
    let definition = definition(fingerprint);
    let mut paused = (*definition).clone();
    paused.initial.playing = false;
    let scene = CookedScene::new(
        id(99),
        60,
        vec![entity(1, None), entity(2, None), entity(3, None)],
    )
    .unwrap()
    .with_spatial(
        vec![VisualAssetRef {
            id: id(90),
            kind: VisualAssetKind::Model,
            fingerprint,
        }],
        vec![
            mesh(1, Some(definition.clone())),
            declaration(2),
            mesh(3, Some(Arc::new(paused))),
        ],
    )
    .unwrap();
    (
        directory,
        CookedScene::from_bytes(&scene.to_bytes()).unwrap(),
        definition,
    )
}
fn load(
    dir: &tempfile::TempDir,
    world: &NativeWorld,
) -> Result<NativeRenderInput, NativeRenderError> {
    NativeRenderInput::load(
        &world.spatial_view().unwrap(),
        VisualCaches {
            models: &dir.path().join("models"),
            textures: &dir.path().join("textures"),
        },
        id(2),
    )
}
fn binding(input: &NativeRenderInput, n: u128) -> &MeshInput {
    input
        .input()
        .nodes
        .iter()
        .find(|node| node.id == format!("{n:032x}"))
        .unwrap()
        .mesh
        .as_ref()
        .unwrap()
}
fn control(world: &mut NativeWorld, n: u128, control: AnimationControl) {
    world
        .apply(&mut queue([StructuralCommand::ControlAnimation {
            entity: id(n),
            control,
        }]))
        .unwrap();
}
#[test]
fn source_deleted_native_samples_use_independent_clocks_without_rebinding_or_allocation() {
    let (dir, scene, _) = prepare();
    let mut world = NativeWorld::new(scene);
    let mut input = load(&dir, &world).unwrap();
    let source = binding(&input, 1).source.clone();
    let RuntimeAssetData::Model(model) = source.data() else {
        panic!()
    };
    let mut player = AnimationPlayer::new(model).unwrap();
    let nodes = input.input().nodes.as_ptr();
    let references = Arc::strong_count(&source);
    dir.close().unwrap();
    for tick in 1..=90 {
        world.step().unwrap();
        let (result, allocations) = allocation_counter::record(|| {
            input.try_sync(&world.spatial_view().unwrap(), |_, _, kind| {
                assert_eq!(kind, UpdateKind::Motion);
                Ok::<_, ()>(())
            })
        });
        result.unwrap();
        assert_eq!(allocations, 0);
        let sample = binding(&input, 1).animation.unwrap();
        assert!((sample.seconds - tick as f64 / 60.).abs() < 1e-12);
        assert_eq!(binding(&input, 3).animation.unwrap().seconds, 0.);
        player
            .sample(sample.clip.map(usize::from), sample.seconds)
            .unwrap();
        // One actual expanded vertex, bound to joint 1, moves by one unit/s.
        let vertex = model.meshes[0].vertices[0];
        let expected = player
            .skin_position(3, 0, 0, [vertex[0], vertex[1], vertex[2]])
            .unwrap();
        assert!((expected[0] - (11. + sample.seconds)).abs() < 1e-6);
        assert_eq!(input.input().nodes.as_ptr(), nodes);
        assert!(Arc::ptr_eq(&source, &binding(&input, 1).source));
        assert_eq!(Arc::strong_count(&source), references);
    }
    control(
        &mut world,
        1,
        AnimationControl::Select(AnimationSample::REST),
    );
    control(
        &mut world,
        3,
        AnimationControl::Select(AnimationSample {
            clip: Some(1),
            seconds: 1.,
        }),
    );
    input
        .try_sync(&world.spatial_view().unwrap(), |_, _, kind| {
            assert_eq!(kind, UpdateKind::Motion);
            Ok::<_, ()>(())
        })
        .unwrap();
    assert_eq!(binding(&input, 1).animation, Some(AnimationSample::REST));
    assert_eq!(binding(&input, 3).animation.unwrap().clip, Some(1));
    assert_eq!(input.binding_generation(), 0);
}
#[test]
fn sample_rejection_and_unwind_restore_complete_prior_input_then_allow_retry() {
    let (dir, scene, _) = prepare();
    let mut world = NativeWorld::new(scene);
    let mut input = load(&dir, &world).unwrap();
    dir.close().unwrap();
    let old = binding(&input, 1).animation;
    let old_world = matrices(&input);
    world.step().unwrap();
    let result = input.try_sync(&world.spatial_view().unwrap(), |scene, _, _| {
        assert_ne!(
            scene
                .nodes
                .iter()
                .find_map(|n| n.mesh.as_ref())
                .unwrap()
                .animation,
            old
        );
        Err::<(), _>("rejected")
    });
    assert!(matches!(result, Err(SyncError::Consumer("rejected"))));
    assert_eq!(binding(&input, 1).animation, old);
    assert_eq!(matrices(&input), old_world);
    assert_eq!(input.tick(), 0);
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = input.try_sync(
                &world.spatial_view().unwrap(),
                |_, _, _| -> Result<(), ()> { panic!("consumer") },
            );
        }))
        .is_err()
    );
    assert_eq!(binding(&input, 1).animation, old);
    input
        .try_sync(&world.spatial_view().unwrap(), |_, _, _| Ok::<_, ()>(()))
        .unwrap();
    assert_eq!(
        binding(&input, 1).animation,
        world.animation_state(id(1)).map(|s| s.sample)
    );
    assert_eq!(input.tick(), 1);
}
#[test]
fn complete_duration_table_checked_on_startup_and_atomic_rebinding() {
    let (dir, scene, definition) = prepare();
    let mut world = NativeWorld::new(scene.clone());
    let mut input = load(&dir, &world).unwrap();
    let mut wrong = (*definition).clone();
    // Alter an unselected clip: checking only the current sample is insufficient.
    wrong.clip_durations = Arc::from([2., 2., 3.]);
    world
        .apply(&mut queue([StructuralCommand::SetSpatial {
            entity: id(1),
            spatial: Some(mesh(1, Some(Arc::new(wrong)))),
        }]))
        .unwrap();
    assert!(matches!(
        load(&dir, &world),
        Err(NativeRenderError::Animation {
            reason: "clip duration table differs from cooked model",
            ..
        })
    ));
    let old = binding(&input, 1).animation;
    let ptr = input.input().nodes.as_ptr();
    dir.close().unwrap();
    assert!(matches!(
        input.try_sync(
            &world.spatial_view().unwrap(),
            |_, _, _| -> Result<(), ()> { panic!("bad binding published") }
        ),
        Err(SyncError::Native(NativeRenderError::Animation { .. }))
    ));
    assert_eq!(input.input().nodes.as_ptr(), ptr);
    assert_eq!(binding(&input, 1).animation, old);
    assert_eq!(input.binding_generation(), 0);
    world
        .apply(&mut queue([StructuralCommand::SetSpatial {
            entity: id(1),
            spatial: Some(mesh(1, None)),
        }]))
        .unwrap();
    input
        .try_sync(&world.spatial_view().unwrap(), |_, _, kind| {
            assert_eq!(kind, UpdateKind::Rebuilt);
            Ok::<_, ()>(())
        })
        .unwrap();
    assert_eq!(binding(&input, 1).animation, None); // Explicit rest-pose consumer.
    assert!(binding(&input, 3).animation.is_some());
}

#[path = "animation/crossfade.rs"]
mod crossfade;
