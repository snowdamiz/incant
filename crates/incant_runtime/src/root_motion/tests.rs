use super::*;
use crate::{animation::Playback, animation_step::*, *};
use glam::{DQuat, DVec3};
use incant_anim::{
    RootMotionClip, RootMotionDelta, RootMotionTime, RootMotionTrack,
    track::{Interpolation, Kind},
};
use incant_types::{
    Animation, AnimationPoseSamples, RootMotionApplication, WeightedAnimationSample,
};
mod continuation;
mod tree;

fn id(n: u8) -> StableId {
    StableId([n; 16])
}
#[path = "../../tests/support/alloc.rs"]
mod alloc;
#[global_allocator]
static ALLOC: alloc::CountAlloc = alloc::CountAlloc;

fn fixture() -> (NativeRootMotion, Playback) {
    fixture_rotations(false)
}
fn fixture_rotations(yaw: bool) -> (NativeRootMotion, Playback) {
    let mut clips = Vec::new();
    for (duration, end, rotation) in [
        (2., [2., 0., 0., 0.], DQuat::from_rotation_y(1.5)),
        (
            4.,
            [0., 0., 8., 0.],
            if yaw {
                DQuat::from_rotation_y(2.4)
            } else {
                DQuat::from_rotation_x(2.4)
            },
        ),
    ] {
        clips.push(
            RootMotionClip::new(
                duration,
                Some(
                    RootMotionTrack::new(
                        vec![0., duration],
                        vec![[0.; 4], end],
                        Interpolation::Linear,
                        Kind::Vector,
                    )
                    .unwrap(),
                ),
                Some(
                    RootMotionTrack::new(
                        vec![0., duration],
                        vec![[0., 0., 0., 1.], rotation.to_array().map(|n| n as f32)],
                        Interpolation::Linear,
                        Kind::Rotation,
                    )
                    .unwrap(),
                ),
            )
            .unwrap(),
        );
    }
    let root = NativeRootMotion {
        catalog: Arc::new(
            RootMotionCatalog::new(
                [1; 32],
                0,
                [0.; 3],
                [0., 0., 0., 1.],
                RootMotionDelta::IDENTITY,
                clips,
            )
            .unwrap(),
        ),
        application: RootMotionApplication::ExtractOnly,
        applying: true,
    };
    let definition = Arc::new(NativeAnimation {
        mesh_fingerprint: [1; 32],
        clip_durations: Arc::from([2., 4.]),
        initial: Animation {
            clip: Some(0),
            seconds: 0.,
            speed: 1.,
            playing: true,
            looping: true,
        },
        graph: None,
    });
    let p = Playback {
        state: definition.initial_state(),
        definition,
        graph: None,
    };
    (root, p)
}
fn publish(p: &mut Playback, state: &mut RootMotionState, step: PreparedRootMotion) {
    p.state = step.animation.state;
    p.graph = step.animation.graph;
    *state = step.root;
}
fn near(a: RootMotionDelta, b: RootMotionDelta) {
    assert!(
        DVec3::from_array(a.translation).distance(DVec3::from_array(b.translation)) < 2e-12,
        "{a:?} != {b:?}"
    );
    assert!(
        (DQuat::from_array(a.rotation)
            .normalize()
            .dot(DQuat::from_array(b.rotation).normalize())
            .abs()
            - 1.)
            .abs()
            < 2e-12,
        "{a:?} != {b:?}"
    );
}
fn graph(p: &mut Playback, compound: bool) {
    let def = Arc::make_mut(&mut p.definition);
    def.graph = Some(Arc::new(
        NativeAnimationGraph::new(
            0,
            vec![NativeAnimationParameter {
                id: id(8),
                initial: 0.25,
                min: 0.,
                max: 1.,
            }],
            vec![
                NativeAnimationGraphState {
                    id: id(1),
                    motion: NativeAnimationMotion::Clip(Some(0)),
                    looping: true,
                },
                NativeAnimationGraphState {
                    id: id(2),
                    motion: if compound {
                        NativeAnimationMotion::Blend1d {
                            parameter: 0,
                            cycle_seconds: 4.,
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
                        }
                    } else {
                        NativeAnimationMotion::Clip(Some(1))
                    },
                    looping: true,
                },
                // The same clip in a distinct state makes guessing outgoing state by
                // sample identity ambiguous. Retained provenance must stay at state 0.
                NativeAnimationGraphState {
                    id: id(3),
                    motion: NativeAnimationMotion::Clip(Some(0)),
                    looping: true,
                },
            ],
            vec![NativeAnimationGraphTransition {
                id: id(7),
                from: 0,
                to: 1,
                priority: 0,
                fade_seconds: 0.3,
                after_seconds: Some(0.1),
                conditions: Arc::from([]),
            }],
            &[2., 4.],
        )
        .unwrap(),
    ));
    p.graph = Some(def.graph.as_ref().unwrap().initial_state(true));
}

#[test]
fn retained_cycles_cover_forward_reverse_endpoint_rebase_and_overflow_without_publication() {
    let (root, mut p) = fixture();
    let mut state = RootMotionState::new(&root, &p, 0);
    let first = PreparedRootMotion::new(&root, &p, state, 12.5, 1).unwrap();
    assert_eq!(first.root.current.cycle, 6);
    near(
        first.root.requested,
        root.catalog
            .sampler(0)
            .unwrap()
            .loop_delta(
                RootMotionTime {
                    cycle: 0,
                    seconds: 0.,
                },
                RootMotionTime {
                    cycle: 6,
                    seconds: 0.5,
                },
            )
            .unwrap(),
    );
    publish(&mut p, &mut state, first);
    p.state.speed = -1.;
    let back = PreparedRootMotion::new(&root, &p, state, 12.5, 2).unwrap();
    assert_eq!(back.root.current.cycle, 0);
    near(back.root.accumulated_requested, RootMotionDelta::IDENTITY);
    publish(&mut p, &mut state, back);
    let old = p.state;
    p.definition
        .control(&mut p.state, AnimationControl::Seek(2.))
        .unwrap();
    let before = Playback {
        definition: p.definition.clone(),
        state: old,
        graph: None,
    };
    state.animation_control(&before, &p, AnimationControl::Seek(2.));
    assert_eq!(state.current.cycle, 0);
    let endpoint = PreparedRootMotion::new(&root, &p, state, 0.5, 3).unwrap();
    near(
        endpoint.root.requested,
        root.catalog
            .sampler(0)
            .unwrap()
            .clamped_delta(2., 1.5)
            .unwrap(),
    );
    state.current.cycle = i64::MIN;
    p.state.sample.seconds = 0.;
    state.current.seconds = 0.;
    assert!(PreparedRootMotion::new(&root, &p, state, 0.5, 3).is_err());
    assert_eq!(state.current.cycle, i64::MIN);
}

#[test]
fn graph_fades_retain_exact_pre_edge_owner_and_independent_speed_with_saturated_mean() {
    for compound in [false, true] {
        let (root, mut p) = fixture();
        graph(&mut p, compound);
        let mut state = RootMotionState::new(&root, &p, 0);
        let first = PreparedRootMotion::new(&root, &p, state, 0.125, 1).unwrap();
        assert_eq!(first.root.outgoing.unwrap().owner, ClockOwner::State(0));
        assert_eq!(first.root.current.owner, ClockOwner::State(1));
        near(
            first.root.requested,
            root.catalog
                .sampler(0)
                .unwrap()
                .clamped_delta(0., 0.125)
                .unwrap(),
        );
        publish(&mut p, &mut state, first);
        p.state.speed = -2.;
        for tick in 2..=4 {
            let step = PreparedRootMotion::new(&root, &p, state, 0.125, tick).unwrap();
            let (incoming, _) =
                interval::sample(&root.catalog, &step.animation.current, state.current.cycle)
                    .unwrap();
            let span = step.animation.outgoing.unwrap();
            let (outgoing, _) =
                interval::sample(&root.catalog, &span, state.outgoing.unwrap().cycle).unwrap();
            let from = (tick - 2) as f64 * 0.125;
            let stop = (from + 0.125).min(0.3);
            let average =
                ((stop * stop - from * from) / (2. * 0.3) + (from + 0.125 - stop)) / 0.125;
            near(
                step.root.requested,
                interval::spherical(outgoing, incoming, average).unwrap(),
            );
            assert_eq!(step.root.current.cycle, -1);
            publish(&mut p, &mut state, step);
        }
        assert!(state.outgoing.is_none());
        p.graph.as_mut().unwrap().paused = true;
        p.state.playing = false;
        let paused = PreparedRootMotion::new(&root, &p, state, 0.125, 5).unwrap();
        assert_eq!(paused.root.current, state.current);
        assert_eq!(paused.root.requested, RootMotionDelta::IDENTITY);
        assert_eq!(
            paused.root.accumulated_requested,
            state.accumulated_requested
        );
    }
}

#[test]
fn interval_spherical_and_weighted_rotation_rules_stay_distinct_and_rest_contributes_identity() {
    let (root, p) = fixture();
    let base = PreparedAnimationStep::new(&p, 1.).unwrap().current;
    let sample = |clip, seconds| AnimationSample { clip, seconds };
    let pair = |t| {
        AnimationPoseSamples::spherical_pair(sample(Some(0), t), sample(Some(1), t * 2.), 0.25)
            .unwrap()
    };
    let weighted = |t| {
        AnimationPoseSamples::new(&[
            WeightedAnimationSample {
                sample: sample(Some(0), t),
                weight: 0.75,
            },
            WeightedAnimationSample {
                sample: sample(Some(1), t * 2.),
                weight: 0.25,
            },
        ])
        .unwrap()
    };
    let a = ClockSpan {
        before_pose: pair(0.),
        after_pose: pair(1.),
        ..base
    };
    let b = ClockSpan {
        before_pose: weighted(0.),
        after_pose: weighted(1.),
        ..base
    };
    let spherical = interval::sample(&root.catalog, &a, 0).unwrap().0;
    let weighted = interval::sample(&root.catalog, &b, 0).unwrap().0;
    assert!(
        (DQuat::from_array(spherical.rotation)
            .dot(DQuat::from_array(weighted.rotation))
            .abs()
            - 1.)
            .abs()
            > 1e-7
    );
    assert_eq!(spherical.translation, weighted.translation);
    let rest = |t| {
        AnimationPoseSamples::new(&[
            WeightedAnimationSample {
                sample: sample(Some(0), t),
                weight: 0.5,
            },
            WeightedAnimationSample {
                sample: AnimationSample::REST,
                weight: 0.5,
            },
        ])
        .unwrap()
    };
    let span = ClockSpan {
        before_pose: rest(0.),
        after_pose: rest(1.),
        ..base
    };
    let result = interval::sample(&root.catalog, &span, 0).unwrap().0;
    assert_eq!(result.translation, [0.5, 0., 0.]);
}

#[test]
fn controls_do_not_teleport_and_pause_is_exact_with_nonidentity_model_reference() {
    let (mut root, mut p) = fixture();
    let reference = RootMotionDelta {
        translation: [3., 4., 5.],
        rotation: DQuat::from_rotation_y(0.8).to_array(),
    };
    let original = RootMotionCatalog::from_bytes(&root.catalog.to_bytes().unwrap()).unwrap();
    // Rebuild the independently owned catalog through its canonical binary
    // payload to exercise a nonidentity static model/root reference.
    let mut bytes = original.to_bytes().unwrap();
    for (i, v) in reference
        .translation
        .into_iter()
        .chain(reference.rotation)
        .enumerate()
    {
        bytes[76 + i * 8..84 + i * 8].copy_from_slice(&v.to_le_bytes());
    }
    root.catalog = Arc::new(RootMotionCatalog::from_bytes(&bytes).unwrap());
    graph(&mut p, true);
    let mut state = RootMotionState::new(&root, &p, 0);
    let first = PreparedRootMotion::new(&root, &p, state, 0.125, 1).unwrap();
    publish(&mut p, &mut state, first);
    let accumulated = state.accumulated_requested;
    let control = AnimationGraphControl::SelectState(id(2));
    p.definition
        .graph
        .as_ref()
        .unwrap()
        .control(
            p.graph.as_mut().unwrap(),
            &mut p.state,
            &p.definition,
            control,
        )
        .unwrap();
    state.graph_control(&p, control);
    assert_eq!(state.accumulated_requested, accumulated);
    assert!(state.outgoing.is_none());
    p.state.playing = false;
    p.graph.as_mut().unwrap().paused = true;
    let paused = PreparedRootMotion::new(&root, &p, state, 0.125, 2).unwrap();
    assert_eq!(paused.root.requested, RootMotionDelta::IDENTITY);
    assert_eq!(paused.root.accumulated_requested, accumulated);
}

fn native_scene(policy: RootMotionApplication, compound: bool) -> CookedScene {
    let (mut root, mut p) = fixture_rotations(policy == RootMotionApplication::Character3d);
    root.application = policy;
    if compound {
        graph(&mut p, true);
    }
    let mut node = CookedSpatial::empty(id(10));
    node.mesh = Some(NativeMeshRenderer {
        mesh: id(11),
        cast_shadows: false,
    });
    node.animation = Some(p.definition);
    node.root_motion = Some(Arc::new(root));
    let mut scene = CookedScene::new(
        id(0),
        8,
        vec![CookedEntity {
            id: id(10),
            parent: None,
            transform: Default::default(),
            velocity: None,
        }],
    )
    .unwrap();
    if matches!(
        policy,
        RootMotionApplication::Kinematic3d | RootMotionApplication::Character3d
    ) {
        scene = scene
            .with_physics(vec![CookedPhysics {
                entity: id(10),
                body: Some(incant_types::RigidBody {
                    motion: incant_types::BodyMotion::Kinematic,
                    ..Default::default()
                }),
                collider: incant_types::Collider {
                    shape: incant_types::ColliderShape::Sphere { radius: 0.1 },
                    ..Default::default()
                },
                angular_velocity: None,
            }])
            .unwrap();
    }
    scene
        .with_spatial(
            vec![VisualAssetRef {
                id: id(11),
                kind: VisualAssetKind::Model,
                fingerprint: [1; 32],
            }],
            vec![node],
        )
        .unwrap()
}
fn command(world: &mut NativeWorld, command: StructuralCommand) {
    let mut queue = FrameCommands::default();
    queue.push(command).unwrap();
    world.apply(&mut queue).unwrap();
}
fn observation(world: &NativeWorld) -> RootMotionSnapshot {
    let mut rows = Vec::new();
    world.snapshot_root_motion(&mut rows).unwrap();
    rows[0]
}

#[test]
fn native_step_cooks_loads_moves_once_and_atomic_controls_preserve_clock_provenance() {
    let scene = native_scene(RootMotionApplication::Transform, true);
    let bytes = scene.to_bytes();
    assert_eq!(
        u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
        18 | (u32::from(PHYSICS_VARIANT.tag()) << 16)
    );
    let mut world = NativeWorld::from_bytes(&bytes).unwrap();
    let node = &scene.spatial()[0];
    let mut p = Playback {
        definition: node.animation.clone().unwrap(),
        state: node.animation.as_ref().unwrap().initial_state(),
        graph: node
            .animation
            .as_ref()
            .unwrap()
            .graph
            .as_ref()
            .map(|g| g.initial_state(true)),
    };
    let mut state = RootMotionState::new(node.root_motion.as_ref().unwrap(), &p, 0);
    let mut expected = RootMotionDelta::IDENTITY;
    for tick in 1..=12 {
        let next =
            PreparedRootMotion::new(node.root_motion.as_ref().unwrap(), &p, state, 0.125, tick)
                .unwrap();
        expected = expected.compose(next.root.requested).unwrap();
        publish(&mut p, &mut state, next);
        world.step().unwrap();
        let actual = world.inspect(id(10)).unwrap().transform;
        near(
            RootMotionDelta {
                translation: actual.translation,
                rotation: actual.rotation,
            },
            expected,
        );
        assert_eq!(world.animation_state(id(10)), Some(p.state));
        assert_eq!(observation(&world).current.cycle, state.current.cycle);
    }
    let before = world.inspect(id(10)).unwrap();
    command(
        &mut world,
        StructuralCommand::ControlAnimationGraph {
            entity: id(10),
            control: AnimationGraphControl::SelectState(id(2)),
        },
    );
    assert_eq!(world.inspect(id(10)).unwrap(), before);
    assert_eq!(observation(&world).current.cycle, 0);
    command(
        &mut world,
        StructuralCommand::ControlRootMotion {
            entity: id(10),
            control: RootMotionControl::SetApplying(false),
        },
    );
    world.step().unwrap();
    assert_eq!(world.inspect(id(10)).unwrap(), before);
    assert_ne!(observation(&world).requested, RootMotionDelta::IDENTITY);
    assert_eq!(observation(&world).applied, RootMotionDelta::IDENTITY);
    let accepted = observation(&world);
    let mut failed = FrameCommands::default();
    failed
        .push(StructuralCommand::ControlRootMotion {
            entity: id(10),
            control: RootMotionControl::ResetAccumulated,
        })
        .unwrap();
    failed
        .push(StructuralCommand::ControlAnimation {
            entity: id(10),
            control: AnimationControl::SetSpeed(f64::NAN),
        })
        .unwrap();
    assert!(world.apply(&mut failed).is_err());
    assert_eq!(observation(&world), accepted);
    assert_eq!(failed.len(), 2);
}

#[test]
fn native_root_failure_does_not_publish_other_owner_clocks_transform_or_tick() {
    let mut world = NativeWorld::new(native_scene(RootMotionApplication::Transform, false));
    world.step().unwrap();
    let handle = world.entities[&id(10)];
    world
        .world
        .get_mut::<RootMotionState>(handle)
        .unwrap()
        .current
        .cycle = i64::MAX;
    command(
        &mut world,
        StructuralCommand::ControlAnimation {
            entity: id(10),
            control: AnimationControl::SetSpeed(16.),
        },
    );
    let before = world.snapshot();
    let animation = world.animation_state(id(10));
    let root = observation(&world);
    let tick = world.tick();
    assert!(world.step().is_err());
    assert_eq!(world.snapshot(), before);
    assert_eq!(world.animation_state(id(10)), animation);
    assert_eq!(observation(&world), root);
    assert_eq!(world.tick(), tick);
    world
        .world
        .get_mut::<RootMotionState>(handle)
        .unwrap()
        .current
        .cycle = 0;
    world.step().unwrap();
    assert_eq!(world.tick(), tick + 1);
}

#[test]
fn kinematic_motion_uses_velocity_once_and_reports_actual_solver_displacement() {
    let mut world = NativeWorld::new(native_scene(RootMotionApplication::Kinematic3d, false));
    for _ in 0..8 {
        world.step().unwrap();
    }
    let state = world.physics_state(id(10)).unwrap();
    let observed = observation(&world);
    near(
        RootMotionDelta {
            translation: state.translation,
            rotation: state.rotation,
        },
        observed.accumulated_applied,
    );
    // Translation would be doubled by teleporting to the candidate and also
    // submitting its velocity. The canonical source advances one meter here.
    assert!((state.translation[0] - 1.).abs() < 2e-6, "{state:?}");
    command(
        &mut world,
        StructuralCommand::ControlRootMotion {
            entity: id(10),
            control: RootMotionControl::SetApplying(false),
        },
    );
    let previous = state.translation;
    world.step().unwrap();
    assert_eq!(world.physics_state(id(10)).unwrap().translation, previous);
}

#[test]
fn character_root_uses_shared_sweep_and_reports_collision_limited_application() {
    let scene = native_scene(RootMotionApplication::Character3d, false);
    let mut entities = scene.entities().to_vec();
    entities.push(CookedEntity {
        id: id(12),
        parent: None,
        transform: incant_types::Transform {
            translation: [0.75, 0., 0.],
            ..Default::default()
        },
        velocity: None,
    });
    let mut physics = scene.physics().to_vec();
    physics.push(CookedPhysics {
        entity: id(12),
        body: None,
        collider: incant_types::Collider {
            shape: incant_types::ColliderShape::Box {
                half_extents: [0.1, 5., 5.],
            },
            ..Default::default()
        },
        angular_velocity: None,
    });
    let scene = CookedScene::new(id(0), 8, entities)
        .unwrap()
        .with_physics(physics)
        .unwrap()
        .with_spatial(scene.visual_assets().to_vec(), scene.spatial().to_vec())
        .unwrap();
    let mut world = NativeWorld::new(scene.clone());
    let mut oracle = NativeWorld::new(
        CookedScene::new(id(0), 8, scene.entities().to_vec())
            .unwrap()
            .with_physics(scene.physics().to_vec())
            .unwrap(),
    );
    let root = scene.spatial()[0].root_motion.as_ref().unwrap();
    for tick in 0..8 {
        let prior = world.physics_state(id(10)).unwrap();
        let mut input = prior.clone();
        input.angular_velocity = [0.; 3];
        oracle.physics.sync_motion(&id(0), &id(10), &input).unwrap();
        let delta = root
            .catalog
            .sampler(0)
            .unwrap()
            .clamped_delta(f64::from(tick) * 0.125, f64::from(tick + 1) * 0.125)
            .unwrap();
        let requested = (DQuat::from_array(prior.rotation).normalize()
            * DVec3::from_array(delta.translation))
        .to_array();
        let expected = oracle
            .physics
            .compute_character_motion(
                &incant_physics_runtime::CharacterQuery {
                    scene_id: id(0),
                    entity_id: id(10),
                    translation: requested,
                    options: Default::default(),
                },
                0.125,
            )
            .unwrap();
        world.step().unwrap();
        let now = world.physics_state(id(10)).unwrap();
        for i in 0..3 {
            assert!(
                (now.translation[i] - prior.translation[i] - expected.translation[i]).abs() < 2e-6
            );
        }
    }
    let actual = world.physics_state(id(10)).unwrap();
    assert!(
        actual.translation[0] > 0.50 && actual.translation[0] < 0.55,
        "{actual:?}"
    );
    let observation = observation(&world);
    assert!((observation.accumulated_requested.translation[0] - 1.).abs() < 1e-6);
    assert!(observation.accumulated_applied.translation[0] < 0.55);
    assert!(matches!(
        world.save_bytes(),
        Err(SaveError::PhysicsUnsupported)
    ));
    command(
        &mut world,
        StructuralCommand::ControlRootMotion {
            entity: id(10),
            control: RootMotionControl::SetApplying(false),
        },
    );
    world.step().unwrap();
    assert_eq!(
        world.physics_state(id(10)).unwrap().translation,
        actual.translation
    );
}

#[test]
fn root_delta_preparation_and_retained_character_queries_allocate_nothing_after_admission() {
    let (root, mut p) = fixture();
    graph(&mut p, true);
    let mut state = RootMotionState::new(&root, &p, 0);
    let (_, allocations) = alloc::measured(|| {
        for tick in 1..=4096 {
            let next = PreparedRootMotion::new(&root, &p, state, 0.125, tick).unwrap();
            publish(&mut p, &mut state, next);
        }
    });
    assert_eq!(allocations, 0);
    let mut world = NativeWorld::new(native_scene(RootMotionApplication::Character3d, false));
    let request = incant_physics_runtime::CharacterQuery {
        scene_id: id(0),
        entity_id: id(10),
        translation: [0.01, 0., 0.],
        options: Default::default(),
    };
    let input = world.physics_state(id(10)).unwrap();
    world
        .physics
        .prepare_character_queries(&id(0), |_| Some(input.clone()))
        .unwrap();
    world
        .physics
        .compute_prepared_character_motion(&request, 0.125)
        .unwrap();
    let (_, allocations) = alloc::measured(|| {
        for _ in 0..1024 {
            world
                .physics
                .prepare_character_queries(&id(0), |_| Some(input.clone()))
                .unwrap();
            assert!(
                world
                    .physics
                    .compute_prepared_character_motion(&request, 0.125)
                    .unwrap()
                    .translation[0]
                    > 0.
            );
        }
    });
    assert_eq!(allocations, 0);
    assert!(
        world
            .physics
            .prepare_character_queries(&id(0), |_| None)
            .is_err()
    );
    assert!(
        world
            .physics
            .compute_prepared_character_motion(&request, 0.125)
            .is_err()
    );
    world
        .physics
        .prepare_character_queries(&id(0), |_| Some(input.clone()))
        .unwrap();
    world.physics.sync_motion(&id(0), &id(10), &input).unwrap();
    assert!(
        world
            .physics
            .compute_prepared_character_motion(&request, 0.125)
            .is_err()
    );
}

#[test]
fn root_preparation_reuses_storage_and_records_matched_world_allocation_limits() {
    // Measure the complete native schedule after cold ECS/query initialization,
    // in addition to the isolated interval and collision query measurements.
    let mut world = NativeWorld::new(native_scene(RootMotionApplication::Transform, true));
    for _ in 0..8 {
        world.step().unwrap();
    }
    let (_, preparation) = alloc::measured(|| {
        for _ in 0..1024 {
            world.prepare_root_motion(0.125, 9).unwrap();
        }
    });
    let (_, globals) = alloc::measured(|| {
        for _ in 0..1024 {
            world.prepare_globals(None).unwrap();
        }
    });
    let (_, allocations) = alloc::measured(|| {
        for _ in 0..1024 {
            world.step().unwrap();
        }
    });
    let mut initial = native_scene(RootMotionApplication::Transform, true);
    initial.spatial[0].root_motion = None;
    let mut plain = NativeWorld::new(initial);
    for _ in 0..8 {
        plain.step().unwrap();
    }
    let (_, baseline) = alloc::measured(|| {
        for _ in 0..1024 {
            plain.step().unwrap();
        }
    });
    eprintln!(
        "root preparation={preparation}; global preflight={globals}; full root={allocations}; matched baseline={baseline}"
    );
    assert_eq!(preparation, 0);
}

#[test]
fn root_policy_refusals_are_atomic_and_combined_body_binding_uses_final_transaction_state() {
    use incant_types::{BodyMotion, Collider, ColliderShape, RigidBody, Velocity};
    let initial = native_scene(RootMotionApplication::Transform, false);
    let mut world = NativeWorld::new(initial.clone());
    let saved = world.save_bytes().unwrap();
    let mut rejected = FrameCommands::default();
    rejected
        .push(StructuralCommand::ControlRootMotion {
            entity: id(10),
            control: RootMotionControl::SetApplying(false),
        })
        .unwrap();
    rejected
        .push(StructuralCommand::SetVelocity {
            entity: id(10),
            velocity: Some(Velocity {
                linear: [1., 0., 0.],
            }),
        })
        .unwrap();
    assert!(world.apply(&mut rejected).is_err());
    assert_eq!(world.save_bytes().unwrap(), saved);
    let physics = CookedPhysics {
        entity: id(10),
        body: Some(RigidBody {
            motion: BodyMotion::Kinematic,
            ..Default::default()
        }),
        collider: Collider {
            shape: ColliderShape::Sphere { radius: 0.1 },
            ..Default::default()
        },
        angular_velocity: None,
    };
    let mut node = initial.spatial[0].clone();
    Arc::make_mut(node.root_motion.as_mut().unwrap()).application =
        RootMotionApplication::Kinematic3d;
    let mut changes = FrameCommands::default();
    changes
        .push(StructuralCommand::SetSpatial {
            entity: id(10),
            spatial: Some(node),
        })
        .unwrap();
    changes
        .push(StructuralCommand::SetPhysics {
            entity: id(10),
            physics: Some(physics),
        })
        .unwrap();
    world.apply(&mut changes).unwrap();
    world.step().unwrap();
    assert!(world.inspect(id(10)).unwrap().transform.translation[0] > 0.);
    let mut bad = initial.spatial[0].clone();
    Arc::make_mut(bad.root_motion.as_mut().unwrap()).application =
        RootMotionApplication::Character3d;
    // The catalog includes pitch, which cannot be silently projected to yaw.
    assert!(
        initial
            .clone()
            .with_spatial(initial.visual_assets().to_vec(), vec![bad])
            .is_err()
    );
    let mut scaled = initial.clone();
    scaled.entities[0].transform.scale = [2., 1., 1.];
    assert!(CookedScene::from_bytes(&scaled.to_bytes()).is_err());
}

#[test]
fn late_unrelated_invalid_numeric_input_cannot_publish_prepared_root_or_character_frame() {
    for application in [
        RootMotionApplication::Transform,
        RootMotionApplication::Character3d,
    ] {
        let scene = native_scene(application, false);
        let mut entities = scene.entities().to_vec();
        entities.push(CookedEntity {
            id: id(12),
            parent: None,
            transform: Default::default(),
            velocity: None,
        });
        let scene = CookedScene::new(id(0), 8, entities)
            .unwrap()
            .with_physics(scene.physics().to_vec())
            .unwrap()
            .with_spatial(scene.visual_assets().to_vec(), scene.spatial().to_vec())
            .unwrap();
        let mut world = NativeWorld::new(scene);
        world.step().unwrap();
        let root = observation(&world);
        let owner = world.inspect(id(10)).unwrap();
        let body = world.physics_state(id(10));
        world
            .world
            .get_mut::<crate::world::Local>(world.entities[&id(12)])
            .unwrap()
            .0
            .scale[1] = f64::NAN;
        assert!(world.step().is_err());
        assert_eq!(world.tick(), 1);
        assert_eq!(observation(&world), root);
        assert_eq!(world.inspect(id(10)).unwrap(), owner);
        assert_eq!(world.physics_state(id(10)), body);
        if application == RootMotionApplication::Character3d {
            assert!(
                world
                    .physics
                    .compute_prepared_character_motion(
                        &incant_physics_runtime::CharacterQuery {
                            scene_id: id(0),
                            entity_id: id(10),
                            translation: [0.1, 0., 0.],
                            options: Default::default()
                        },
                        0.125
                    )
                    .is_err()
            );
        }
        world
            .world
            .get_mut::<crate::world::Local>(world.entities[&id(12)])
            .unwrap()
            .0
            .scale[1] = 1.;
        world.step().unwrap();
        assert_eq!(world.tick(), 2);
        assert_ne!(observation(&world), root);
    }
}
