use super::*;
use incant_physics_snapshot::MAX_ALLOCATION;

fn id(n: u8) -> StableId {
    StableId([n; 16])
}
fn budget() -> Budget {
    Budget::new(MAX_ALLOCATION).unwrap()
}
fn work() -> WorkBudget {
    crate::CheckpointWorkBudget::new(100_000_000)
}
fn allocations() -> AllocationBudget {
    AllocationBudget::new(MAX_ALLOCATION, 100_000)
}
fn spec(dynamic: bool, sensor: bool, y: f64) -> PhysicsSpec {
    PhysicsSpec {
        body: dynamic.then(DocBody::default),
        collider: DocCollider {
            shape: ColliderShape::Box {
                half_extents: if dynamic { [0.4; 3] } else { [4., 0.5, 4.] },
            },
            sensor,
            ..Default::default()
        },
        state: BodyState {
            translation: [0., y, 0.],
            rotation: [0., 0., 0., 1.],
            velocity: [0.; 3],
            angular_velocity: [0.; 3],
            sleeping: false,
        },
    }
}
fn warm() -> PhysicsRuntime<StableId> {
    let mut runtime = PhysicsRuntime::default();
    let declarations = [id(1), id(2)]
        .into_iter()
        .map(|scene| {
            (
                scene,
                BTreeMap::from([
                    (id(10), spec(false, false, -0.5)),
                    (id(11), spec(true, false, 1.)),
                    (id(12), spec(false, true, 0.6)),
                ]),
            )
        })
        .collect();
    runtime.sync(PreparedPhysics::new(declarations).unwrap());
    for _ in 0..64 {
        runtime.step(1. / 64.).unwrap();
    }
    runtime
}
fn capture(runtime: &mut PhysicsRuntime<StableId>) -> PhysicsCheckpoint {
    runtime.checkpoint(&mut budget(), &mut work()).unwrap()
}
fn ray(scene_id: StableId) -> RayQuery<StableId> {
    RayQuery {
        scene_id,
        origin: [0., 4., 0.],
        direction: [0., -1., 0.],
        max_distance: 10.,
        include_sensors: false,
        exclude_entity: None,
        memberships: u32::MAX,
        filter: u32::MAX,
    }
}
fn frames(checkpoint: &PhysicsCheckpoint) -> Vec<(StableId, Vec<u8>)> {
    checkpoint
        .scenes
        .iter()
        .map(|scene| (scene.id, scene.native.encoded().to_vec()))
        .collect()
}
fn assert_observations(source: &PhysicsRuntime<StableId>, restored: &PhysicsRuntime<StableId>) {
    assert_eq!(source.events(), restored.events());
    assert_eq!(source.scenes.len(), restored.scenes.len());
    for (&scene_id, scene) in &source.scenes {
        let other = &restored.scenes[&scene_id];
        assert_eq!(scene.entries.len(), other.entries.len());
        for &entity in scene.entries.keys() {
            assert_eq!(
                source.state(&scene_id, &entity),
                restored.state(&scene_id, &entity)
            );
        }
        assert_eq!(
            source.raycast(&ray(scene_id)),
            restored.raycast(&ray(scene_id))
        );
    }
}

#[test]
fn spatial_checkpoint_preserves_all_scenes_ids_queries_and_continuation() {
    let mut source = warm();
    let before = frames(&capture(&mut source));
    assert!(source.raycast(&ray(id(1))).unwrap().is_some());
    let echo = source.state(&id(1), &id(11)).unwrap().clone();
    source.sync_motion(&id(1), &id(11), &echo).unwrap();
    let checkpoint = capture(&mut source);
    assert_eq!(before, frames(&checkpoint));
    let mut restored = checkpoint
        .reconstruct(&mut budget(), &mut work(), &mut allocations())
        .unwrap();
    assert_observations(&source, &restored);
    for tick in 0..96 {
        for runtime in [&mut source, &mut restored] {
            if tick == 8 {
                runtime.remove(&id(1), &id(12));
            }
            if tick == 16 {
                runtime.install(
                    id(1),
                    id(13),
                    PreparedBody::new(spec(false, true, 0.6)).unwrap(),
                );
            }
            if tick == 40 {
                let mut motion = runtime.state(&id(2), &id(11)).unwrap().clone();
                motion.translation[1] += 1.;
                motion.velocity[0] = 0.2;
                runtime.sync_motion(&id(2), &id(11), &motion).unwrap();
            }
            runtime.step(1. / 64.).unwrap();
        }
        assert_observations(&source, &restored);
        assert_eq!(
            frames(&capture(&mut source)),
            frames(&capture(&mut restored)),
            "tick {tick}"
        );
    }
}

#[test]
fn spatial_late_scene_and_budget_refusals_preserve_every_live_owner() {
    let mut runtime = warm();
    let mut checkpoint = capture(&mut runtime);
    let before = frames(&checkpoint);
    assert!(
        runtime
            .checkpoint(&mut Budget::new(0).unwrap(), &mut work())
            .is_err()
    );
    assert!(
        runtime
            .checkpoint(&mut budget(), &mut WorkBudget::new(0))
            .is_err()
    );
    assert!(
        runtime
            .restore_checkpoint(
                &checkpoint,
                &mut budget(),
                &mut work(),
                &mut AllocationBudget::new(0, 0)
            )
            .is_err()
    );
    assert_eq!(before, frames(&capture(&mut runtime)));
    // Both native worlds have been reconstructed before the second scene's
    // outer declaration fails its join. Neither live scene may be replaced.
    checkpoint.scenes[1].entries[1].config.state.translation[0] += 2.;
    assert!(matches!(
        runtime.restore_checkpoint(&checkpoint, &mut budget(), &mut work(), &mut allocations()),
        Err(CheckpointError::Owner("committed body observation"))
    ));
    assert_eq!(before, frames(&capture(&mut runtime)));
    runtime.step(1. / 64.).unwrap();
}

#[test]
fn spatial_checkpoint_preserves_exits_after_entity_and_scene_removal() {
    for remove_scene in [false, true] {
        let mut runtime = warm();
        if remove_scene {
            let retained = runtime.scenes[&id(2)]
                .entries
                .iter()
                .map(|(&id, entry)| (id, entry.config.clone()))
                .collect();
            runtime.sync(PreparedPhysics::new(BTreeMap::from([(id(2), retained)])).unwrap());
        } else {
            runtime.remove(&id(1), &id(12));
        }
        runtime.step(1. / 64.).unwrap();
        assert!(
            runtime
                .events()
                .iter()
                .any(|event| event.scene_id == id(1) && !event.entered)
        );
        let checkpoint = capture(&mut runtime);
        let restored = checkpoint
            .reconstruct(&mut budget(), &mut work(), &mut allocations())
            .unwrap();
        assert_observations(&runtime, &restored);
        assert_eq!(restored.scenes.contains_key(&id(1)), !remove_scene);
    }
}

#[test]
fn spatial_real_motion_invalidates_capture_until_a_completed_step() {
    let mut runtime = warm();
    let mut motion = runtime.state(&id(1), &id(11)).unwrap().clone();
    motion.velocity[0] = 1.;
    runtime.sync_motion(&id(1), &id(11), &motion).unwrap();
    assert_eq!(runtime.checkpoint_configurations().unwrap().count(), 6);
    assert!(runtime.checkpoint(&mut budget(), &mut work()).is_err());
    runtime.step(1. / 64.).unwrap();
    capture(&mut runtime);
}

#[test]
fn spatial_outer_join_charges_false_native_intersection_candidates() {
    let mut runtime = PhysicsRuntime::default();
    for (entity, translation) in [(id(10), [0.; 3]), (id(11), [0.8, 0.8, 0.])] {
        let mut body = spec(false, true, 0.);
        body.collider.shape = ColliderShape::Sphere { radius: 0.5 };
        body.state.translation = translation;
        runtime.install(id(1), entity, PreparedBody::new(body).unwrap());
    }
    runtime.step(1. / 64.).unwrap();
    assert!(runtime.overlaps.is_empty());
    let pairs: Vec<_> = runtime.scenes[&id(1)]
        .world
        .read()
        .intersection_pairs()
        .collect();
    assert_eq!(pairs.len(), 1);
    assert!(!pairs[0].4);
    assert!(charge_native_pairs(&runtime, &mut WorkBudget::new(0)).is_err());
    let mut exact = WorkBudget::new(1);
    charge_native_pairs(&runtime, &mut exact).unwrap();
    assert_eq!(exact.remaining(), 0);
}

#[test]
fn spatial_failed_simulation_refuses_capture_before_native_access_or_charges() {
    let mut runtime = warm();
    let mut motion = runtime.state(&id(2), &id(11)).unwrap().clone();
    motion.translation[0] = 999_999.;
    motion.velocity[0] = 2.;
    runtime.sync_motion(&id(2), &id(11), &motion).unwrap();
    assert_eq!(runtime.step(1.), Err(PhysicsError::NumericRange));
    assert!(matches!(
        runtime.checkpoint_configurations(),
        Err(CheckpointError::Owner("failed runtime"))
    ));
    let mut payload = budget();
    let mut visits = work();
    let before = (payload.remaining(), visits.remaining());
    assert!(matches!(
        runtime.checkpoint(&mut payload, &mut visits),
        Err(CheckpointError::Owner("failed runtime"))
    ));
    assert_eq!(before, (payload.remaining(), visits.remaining()));
}

#[test]
fn spatial_checkpoint_configurations_borrow_ordered_cached_state_and_preserve_authority() {
    let empty = PhysicsRuntime::<StableId>::default();
    assert_eq!(empty.checkpoint_configurations().unwrap().count(), 0);
    let mut runtime = warm();
    let before = frames(&capture(&mut runtime));
    let mut keys = Vec::new();
    for (scene, entity, config) in runtime.checkpoint_configurations().unwrap() {
        keys.push((scene, entity));
        assert!(std::ptr::eq(
            config,
            &runtime.scenes[&scene].entries[&entity].config
        ));
        assert_eq!(&config.state, runtime.state(&scene, &entity).unwrap());
        assert_eq!(config.body.is_some(), entity == id(11));
        assert_eq!(config.collider.sensor, entity == id(12));
    }
    assert_eq!(
        keys,
        [id(1), id(2)]
            .into_iter()
            .flat_map(|scene| [id(10), id(11), id(12)].map(|entity| (scene, entity)))
            .collect::<Vec<_>>()
    );
    assert_eq!(before, frames(&capture(&mut runtime)));
}

mod compound;
