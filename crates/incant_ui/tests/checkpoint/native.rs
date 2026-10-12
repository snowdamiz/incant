use super::*;
use incant_runtime::{
    CookedEntity, CookedScene, CookedUi, FontAssetRef, FrameCommands, NativeWorld,
    StructuralCommand,
};
fn scene() -> CookedScene {
    let nodes = widgets();
    CookedScene::new(
        id(80),
        60,
        nodes
            .iter()
            .map(|w| CookedEntity {
                id: w.id,
                parent: w.parent,
                transform: Default::default(),
                velocity: None,
            })
            .collect(),
    )
    .unwrap()
    .with_text(
        vec![FontAssetRef {
            id: id(99),
            fingerprint: [9; 32],
        }],
        vec![],
    )
    .unwrap()
    .with_ui(
        nodes
            .into_iter()
            .map(|w| CookedUi {
                entity: w.id,
                node: w.node,
            })
            .collect(),
    )
    .unwrap()
}
fn bound(world: &NativeWorld) -> UiRuntime {
    let mut ui = UiRuntime::for_world(world).unwrap();
    let next = ui.prepare(&[], [400., 400.], &mut Measure).unwrap();
    ui.commit_for_world(next, world).unwrap();
    ui
}
#[test]
fn bound_capture_rejects_another_world_stale_tick_and_stale_declarations() {
    let mut world = NativeWorld::new(scene());
    let mut ui = bound(&world);
    let other = NativeWorld::new(scene());
    assert!(ui.checkpoint_for_world(&other).is_err());
    assert!(runtime(widgets()).checkpoint_for_world(&world).is_err());
    let old = ui.checkpoint_for_world(&world).unwrap();
    world.step().unwrap();
    assert!(old.validate_world(&world).is_err());
    assert!(ui.checkpoint_for_world(&world).is_err());
    let next = ui.prepare_refresh([400., 400.], &mut Measure).unwrap();
    ui.commit_for_world(next, &world).unwrap();
    ui.checkpoint_for_world(&world).unwrap();
    let mut changed = world
        .ui_view()
        .unwrap()
        .nodes()
        .find(|n| n.id == id(4))
        .unwrap()
        .node
        .as_ref()
        .clone();
    changed.disabled = true;
    let mut commands = FrameCommands::default();
    commands
        .push(StructuralCommand::SetUi {
            entity: id(4),
            node: Some(Arc::new(changed)),
        })
        .unwrap();
    world.apply(&mut commands).unwrap();
    assert!(ui.checkpoint_for_world(&world).is_err());
    let next = ui.prepare_refresh([400., 400.], &mut Measure).unwrap();
    assert!(ui.commit_for_world(next, &world).is_err());
    let nodes = world
        .ui_view()
        .unwrap()
        .nodes()
        .map(|n| Widget {
            id: n.id,
            parent: n.parent,
            incarnation: n.incarnation,
            node: Arc::clone(n.node),
        })
        .collect();
    let next = ui.prepare_refresh([400., 400.], &mut Measure).unwrap();
    let next = ui.reconcile(next, nodes, &mut Measure).unwrap();
    ui.commit_for_world(next, &world).unwrap();
    ui.checkpoint_for_world(&world).unwrap();
}
#[test]
fn pending_restore_requires_exact_fresh_world_and_exact_committed_widget_data() {
    let world = NativeWorld::new(scene());
    let mut ui = bound(&world);
    let next = ui
        .prepare(&[UiEvent::Focus(id(3))], [400., 400.], &mut Measure)
        .unwrap();
    ui.commit_for_world(next, &world).unwrap();
    let old_token = ui.focus().unwrap();
    let next = ui
        .prepare(
            &[UiEvent::Commit {
                target: old_token.clone(),
                text: "abc".into(),
            }],
            [400., 400.],
            &mut Measure,
        )
        .unwrap();
    ui.commit_for_world(next, &world).unwrap();
    let bytes = ui.checkpoint_for_world(&world).unwrap();
    let restored_world = NativeWorld::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
    let pending = PendingUiCheckpoint::new(&restored_world, bytes.bytes()).unwrap();
    assert!(pending.restore(&world, [400., 400.], &mut Measure).is_err());
    let mut restored = pending
        .restore(&restored_world, [600., 300.], &mut Measure)
        .unwrap();
    pending
        .validate_restored(&restored_world, &restored)
        .unwrap();
    assert!(
        pending
            .validate_restored(&restored_world, &bound(&restored_world))
            .is_err()
    );
    assert_ne!(restored.focus().unwrap(), old_token);
    let next = restored
        .prepare(
            &[UiEvent::Commit {
                target: old_token,
                text: "late".into(),
            }],
            [600., 300.],
            &mut Measure,
        )
        .unwrap();
    assert_eq!(next.discarded_events(), 1);
    restored.commit_for_world(next, &restored_world).unwrap();
    assert_eq!(restored.checkpoint_bytes().unwrap(), bytes.bytes());
}
