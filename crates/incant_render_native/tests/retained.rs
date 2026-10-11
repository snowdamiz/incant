mod common;
use common::*;
use incant_render_native::{
    NativeRenderError, NativeRenderInput, SyncError, UpdateKind, VisualCaches,
};
use incant_runtime::{CookedScene, NativeWorld, StructuralCommand as Cmd};
use incant_types::Velocity;
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    path::Path,
};

#[test]
fn committed_parent_poses_and_labels_reuse_storage_through_motion_and_archetype_relocation() {
    let mut w = world();
    let mut a = load(&w);
    assert_eq!(a.input().nodes.len(), 2); // transform-only parent is not diagnostic geometry
    assert!(
        a.input()
            .nodes
            .iter()
            .all(|n| !n.diagnostic && n.sprite.is_none())
    );
    assert_eq!(a.camera_id(), id(2));
    assert_eq!(a.camera_label(), "00000000000000000000000000000002");
    assert_eq!(
        a.input()
            .nodes
            .iter()
            .find(|n| n.id == a.camera_label())
            .unwrap()
            .world
            .w_axis
            .x,
        3.25
    );
    let ptr = a.input().nodes.as_ptr();
    let label = a.camera_label().as_ptr();
    let capacity = a.input().nodes.capacity();
    for tick in 1..=32 {
        w.step().unwrap();
        a.try_sync(&w.spatial_view().unwrap(), |input, camera, kind| {
            assert_eq!(kind, UpdateKind::Motion);
            assert_eq!(camera.as_ptr(), label);
            assert_eq!(
                input
                    .nodes
                    .iter()
                    .find(|n| n.id == camera)
                    .unwrap()
                    .world
                    .w_axis
                    .x,
                3.25 + 2. * tick as f64
            );
            Ok::<_, ()>(())
        })
        .unwrap();
        assert_eq!(a.tick(), tick);
        assert_eq!(a.input().nodes.as_ptr(), ptr);
        assert_eq!(a.input().nodes.capacity(), capacity);
    }
    // Removing/reinserting Velocity relocates ECS rows without changing topology.
    for velocity in [
        None,
        Some(Velocity {
            linear: [120., 0., 0.],
        }),
    ] {
        w.apply(&mut queue([Cmd::SetVelocity {
            entity: id(2),
            velocity,
        }]))
        .unwrap();
        a.try_sync(&w.spatial_view().unwrap(), |_, _, kind| {
            assert_eq!(kind, UpdateKind::Motion);
            Ok::<_, ()>(())
        })
        .unwrap();
        assert_eq!(a.input().nodes.as_ptr(), ptr);
    }
    assert_eq!(a.topology_generation(), 0);
    assert_eq!(a.binding_generation(), 0);
}

#[test]
fn rejected_and_unwinding_motion_restore_all_input_and_publish_only_after_acceptance() {
    let mut w = world();
    let mut a = load(&w);
    let old = matrices(&a);
    let generations = generations(&a);
    w.step().unwrap();
    let error = a.try_sync(&w.spatial_view().unwrap(), |input, _, _| {
        assert_ne!(input.nodes[0].world.to_cols_array(), old[0]);
        Err::<(), _>("late rejection")
    });
    assert!(matches!(error, Err(SyncError::Consumer("late rejection"))));
    assert_eq!(matrices(&a), old);
    assert_eq!(common::generations(&a), generations);
    let panic = catch_unwind(AssertUnwindSafe(|| {
        a.try_sync(
            &w.spatial_view().unwrap(),
            |input, _, _| -> Result<(), ()> {
                assert_ne!(input.nodes[1].world.to_cols_array(), old[1]);
                panic!("consumer unwind")
            },
        )
        .unwrap();
    }));
    assert!(panic.is_err());
    assert_eq!(matrices(&a), old);
    assert_eq!(common::generations(&a), generations);
    a.try_sync(&w.spatial_view().unwrap(), |_, _, _| Ok::<_, ()>(()))
        .unwrap();
    assert_ne!(matrices(&a), old);
    assert_eq!(a.tick(), 1);
}

#[test]
fn structural_rebuild_is_atomic_on_error_and_unwind_and_can_retry_same_world() {
    let mut w = world();
    let mut a = load(&w);
    let old = matrices(&a);
    let oldgen = generations(&a);
    let oldptr = a.input().nodes.as_ptr();
    let mut light = declaration(3);
    light.point.as_mut().unwrap().intensity = 42.;
    w.apply(&mut queue([
        Cmd::SetParent {
            entity: id(2),
            parent: Some(id(3)),
        },
        Cmd::SetSpatial {
            entity: id(3),
            spatial: Some(light),
        },
        Cmd::Despawn(id(1)),
        Cmd::Spawn(entity(4, None)),
    ]))
    .unwrap();
    let err = a.try_sync(&w.spatial_view().unwrap(), |input, _, kind| {
        assert_eq!(kind, UpdateKind::Rebuilt);
        assert_eq!(input.nodes.len(), 2);
        Err::<(), _>(7)
    });
    assert!(matches!(err, Err(SyncError::Consumer(7))));
    assert_eq!(a.input().nodes.as_ptr(), oldptr);
    assert_eq!(matrices(&a), old);
    assert_eq!(generations(&a), oldgen);
    assert!(
        catch_unwind(AssertUnwindSafe(|| a
            .try_sync(&w.spatial_view().unwrap(), |_, _, _| -> Result<(), ()> {
                panic!("reject candidate")
            })))
        .is_err()
    );
    assert_eq!(matrices(&a), old);
    assert_eq!(generations(&a), oldgen);
    a.try_sync(&w.spatial_view().unwrap(), |input, cam, kind| {
        assert_eq!(kind, UpdateKind::Rebuilt);
        assert_eq!(
            input
                .nodes
                .iter()
                .find(|n| n.id == cam)
                .unwrap()
                .world
                .w_axis
                .x,
            5.25
        );
        Ok::<_, ()>(())
    })
    .unwrap();
    assert_eq!(generations(&a), (0, 1, 1));
    assert!(a.input().nodes.iter().any(|node| matches!(&node.light, Some(incant_render_data::SceneLight::Point(light)) if light.intensity==42.)));
    w.step().unwrap();
    a.try_sync(&w.spatial_view().unwrap(), |_, _, kind| {
        assert_eq!(kind, UpdateKind::Motion);
        Ok::<_, ()>(())
    })
    .unwrap();
}

#[test]
fn camera_removal_and_different_world_fail_before_consumer_and_keep_previous_input() {
    let mut w = world();
    let mut a = load(&w);
    let old = matrices(&a);
    let other = world();
    assert_eq!(other.scene_id(), w.scene_id());
    assert!(matches!(
        a.try_sync(
            &other.spatial_view().unwrap(),
            |_, _, _| -> Result<(), ()> { panic!("cross-world callback") }
        ),
        Err(SyncError::Native(NativeRenderError::DifferentWorld))
    ));
    assert_eq!(matrices(&a), old);
    w.apply(&mut queue([Cmd::SetSpatial {
        entity: id(2),
        spatial: None,
    }]))
    .unwrap();
    assert!(
        matches!(a.try_sync(&w.spatial_view().unwrap(),|_,_,_|->Result<(),()>{panic!("missing camera callback")}),Err(SyncError::Native(NativeRenderError::MissingCamera(x))) if x==id(2))
    );
    assert_eq!(matrices(&a), old);
    assert_eq!(generations(&a), (0, 0, 0));
    w.apply(&mut queue([Cmd::SetSpatial {
        entity: id(2),
        spatial: Some(declaration(2)),
    }]))
    .unwrap();
    let moved = Box::new(w);
    a.try_sync(&moved.spatial_view().unwrap(), |_, _, kind| {
        assert_eq!(kind, UpdateKind::Rebuilt);
        Ok::<_, ()>(())
    })
    .unwrap();
    assert_eq!(a.binding_generation(), 2);
    let empty = NativeWorld::new(CookedScene::new(id(99), 60, vec![entity(2, None)]).unwrap());
    assert!(matches!(
        NativeRenderInput::load(
            &empty.spatial_view().unwrap(),
            VisualCaches {
                models: Path::new("absent"),
                textures: Path::new("absent")
            },
            id(2)
        ),
        Err(NativeRenderError::MissingCamera(_))
    ));
}

#[test]
fn failed_world_tick_and_failed_structural_batch_do_not_leak_candidate_values() {
    let mut root = entity(1, None);
    root.transform.scale = [1e100; 3];
    let mut child = entity(2, Some(1));
    child.velocity = Some(Velocity {
        linear: [1e300, 0., 0.],
    });
    let scene = CookedScene::new(id(99), 60, vec![root, child])
        .unwrap()
        .with_spatial(vec![], vec![declaration(2)])
        .unwrap();
    let mut w = NativeWorld::new(scene);
    let mut a = load(&w);
    let old = matrices(&a);
    assert!(w.step().is_err());
    assert!(
        w.apply(&mut queue([Cmd::SetParent {
            entity: id(1),
            parent: Some(id(2))
        }]))
        .is_err()
    );
    a.try_sync(&w.spatial_view().unwrap(), |_, _, kind| {
        assert_eq!(kind, UpdateKind::Motion);
        Ok::<_, ()>(())
    })
    .unwrap();
    assert_eq!(matrices(&a), old);
    assert_eq!(generations(&a), (0, 0, 0));
}
