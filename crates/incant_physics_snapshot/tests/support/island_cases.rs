//! Native island cases composed with the existing checked manifold observer.
macro_rules! island_cases {
    ($rapier:ident,$vector:path,$cuboid:path) => {
        fn check_islands(
            w: &PhysicsWorld,
            a: &ValidatedAttachments<'_>,
            m: &ValidatedManifolds<'_>,
        ) -> RigidOwnerRecord {
            let b = ordered_capture::capture(&w.bodies);
            let (_, handles) = layout(&b["bodies"], identity());
            let bodies = handles
                .iter()
                .map(|h| {
                    body_capture::body(
                        &ordered_capture::capture(
                            &w.bodies[RigidBodyHandle::from_raw_parts(h.slot, h.generation)],
                        ),
                        identity().dimension,
                    )
                })
                .collect();
            let bodies = validate_body_state(
                BodyStateTable {
                    identity: identity(),
                    bodies,
                },
                BodyContext {
                    attachments: a,
                    journals: PendingJournals::default(),
                },
            )
            .unwrap();
            let r = island_capture::capture(&ordered_capture::capture(&w.islands), identity());
            let c = IslandContext {
                bodies: &bodies,
                manifolds: m,
                domain: IslandDomain::default(),
            };
            let v = validate_islands(r.clone(), c, &mut budget()).unwrap_or_else(|e| {
                panic!(
                    "{e:?} island={r:?} bodies={:?}",
                    bodies
                        .record()
                        .bodies
                        .iter()
                        .map(|b| (b.ids, b.kind, b.enabled, b.activation.sleeping))
                        .collect::<Vec<_>>()
                )
            });
            let pairs = validate_pair_state(pair_states(w), m, &mut budget()).unwrap();
            let scan = w.islands.checkpoint_sleep_scan().unwrap();
            let owner = rigid_owner_capture::capture(
                &ordered_capture::capture(&w.narrow_phase),
                identity(),
                scan.stamp,
                &scan.slots,
            );
            let cv = ordered_capture::capture(&w.colliders);
            let (_, colliders) = layout(&cv["colliders"], identity());
            let thresholds: Vec<_> = colliders
                .iter()
                .map(|h| {
                    let co = &w.colliders[ColliderHandle::from_raw_parts(h.slot, h.generation)];
                    if co
                        .active_events()
                        .contains(ActiveEvents::CONTACT_FORCE_EVENTS)
                    {
                        co.contact_force_event_threshold()
                    } else {
                        f32::MAX
                    }
                    .to_bits()
                })
                .collect();
            let owner_context = RigidOwnerContext {
                islands: &v,
                pairs: &pairs,
                force_thresholds: &thresholds,
                multibody_epoch: word(
                    &ordered_capture::capture(&w.multibody_joints)["topology_epoch"],
                ),
            };
            let joined = validate_rigid_owner(owner.clone(), owner_context, &mut budget())
                .unwrap_or_else(|e| panic!("{e:?} owner={owner:?}"));
            let bytes = encode_rigid_owner(&joined, &mut budget()).unwrap();
            let decoded_owner = decode_rigid_owner(&bytes, owner_context, &mut budget()).unwrap();
            assert_eq!(decoded_owner.record(), &owner);
            check_owner_corruption(owner, owner_context, &bytes);
            let owner = decoded_owner.record().clone();
            let bytes = encode_islands(&v, &mut budget()).unwrap();
            let loaded = decode_islands(&bytes, c, &mut budget()).unwrap();
            assert_eq!(loaded.record(), &r);
            assert_eq!(encode_islands(&loaded, &mut budget()).unwrap(), bytes);
            for h in handles {
                let native = &w.bodies[RigidBodyHandle::from_raw_parts(h.slot, h.generation)];
                assert_eq!(
                    loaded.body_membership(h).unwrap()[1].is_some(),
                    native.is_enabled() && !native.is_fixed()
                );
            }
            owner
        }

        fn observed_islands(w: &PhysicsWorld) -> IslandTable {
            observe(w, false).unwrap();
            island_capture::capture(&ordered_capture::capture(&w.islands), identity())
        }
        #[test]
        fn actual_islands_keep_empty_awake_container_and_sleep_wake_order() {
            let (mut w, first, _) = fixture(false);
            let (second, _) = w.insert(
                RigidBodyBuilder::dynamic().translation($vector(3.5, 1.49)),
                $cuboid(0.4, 0.5),
            );
            for _ in 0..80 {
                w.step();
            }
            let sleeping = observed_islands(&w);
            assert!(w.bodies[first].is_sleeping() && w.bodies[second].is_sleeping());
            assert_eq!(
                sleeping.active[sleeping.awake.unwrap() as usize]
                    .unwrap()
                    .count,
                0
            );
            assert_eq!(sleeping.persistent_live, 2);
            assert!(sleeping.persistent.iter().flatten().all(|p| p.sleeping));
            w.wake_up(second, true);
            w.step();
            let awake = observed_islands(&w);
            assert_ne!(awake.active_epoch, sleeping.active_epoch);
            let a = awake.active[awake.awake.unwrap() as usize].unwrap();
            assert_eq!(
                &awake.active_bodies[a.start as usize..(a.start + a.count) as usize],
                &[handle(second.into_raw_parts())]
            );
            assert!(w.bodies[first].is_sleeping());
            w.wake_up(first, true);
            w.step();
            let both = observed_islands(&w);
            let a = both.active[both.awake.unwrap() as usize].unwrap();
            assert_eq!(
                &both.active_bodies[a.start as usize..(a.start + a.count) as usize],
                &[
                    handle(second.into_raw_parts()),
                    handle(first.into_raw_parts())
                ]
            );
            assert_eq!(both.active_live, 1);
            assert_eq!(both.active_free.len(), 2);
        }
        #[test]
        fn actual_islands_reorder_disable_type_remove_and_generation_reuse() {
            let mut w = PhysicsWorld::default();
            w.gravity = $vector(0., 0.);
            let mut handles = vec![];
            for x in [0., 10., 20.] {
                handles.push(
                    w.insert(
                        RigidBodyBuilder::dynamic()
                            .translation($vector(x, 0.))
                            .can_sleep(false),
                        $cuboid(0.5, 0.5),
                    )
                    .0,
                );
            }
            w.step();
            let initial = observed_islands(&w);
            assert_eq!(initial.persistent_live, 3);
            w.bodies[handles[2]].set_additional_solver_iterations(3);
            w.step();
            let reordered = observed_islands(&w);
            assert_ne!(initial.active_bodies, reordered.active_bodies);
            assert_eq!(
                reordered.active_bodies[0],
                handle(handles[2].into_raw_parts())
            );
            w.remove_body(handles[1]).unwrap();
            w.step();
            let removed = observed_islands(&w);
            assert_eq!(removed.persistent_live, 2);
            assert!(!removed.persistent_free.is_empty());
            let new = w
                .insert(
                    RigidBodyBuilder::kinematic_position_based().translation($vector(10., 0.)),
                    $cuboid(0.5, 0.5),
                )
                .0;
            assert_eq!(new.into_raw_parts().0, handles[1].into_raw_parts().0);
            assert_ne!(new.into_raw_parts().1, handles[1].into_raw_parts().1);
            w.step();
            let reused = observed_islands(&w);
            assert_eq!(reused.persistent_live, 3);
            w.bodies[new].set_enabled(false);
            w.step();
            let disabled = observed_islands(&w);
            assert_eq!(disabled.persistent_live, 2);
            w.bodies[new].set_enabled(true);
            w.step();
            assert_eq!(observed_islands(&w).persistent_live, 3);
            w.bodies[new].set_body_type(RigidBodyType::Fixed, true);
            w.step();
            assert_eq!(observed_islands(&w).persistent_live, 2);
            w.bodies[new].set_body_type(RigidBodyType::Dynamic, true);
            w.step();
            assert_eq!(observed_islands(&w).persistent_live, 3);
        }
        #[test]
        fn actual_islands_contact_merge_disconnect_and_empty_bootstrapped_world() {
            let mut w = PhysicsWorld::default();
            w.gravity = $vector(0., 0.);
            w.step();
            let empty = observed_islands(&w);
            assert_eq!(empty.active_live, 0);
            assert!(empty.bootstrapped);
            let (a, ca) = w.insert(
                RigidBodyBuilder::dynamic()
                    .translation($vector(0., 0.))
                    .can_sleep(false),
                $cuboid(0.5, 0.5),
            );
            let (b, _) = w.insert(
                RigidBodyBuilder::dynamic()
                    .translation($vector(0.9, 0.))
                    .can_sleep(false),
                $cuboid(0.5, 0.5),
            );
            w.step();
            let merged = observed_islands(&w);
            assert_eq!(merged.persistent_live, 1);
            assert_eq!(merged.contacts.len(), 1);
            w.bodies[b].set_translation($vector(10., 0.), true);
            w.step();
            let split = observed_islands(&w);
            assert_eq!(split.persistent_live, 1);
            assert!(split.contacts.is_empty());
            assert!(
                split
                    .persistent
                    .iter()
                    .flatten()
                    .any(|p| p.constraint_remove_count > 0)
            );
            for h in [a, b] {
                w.bodies[h].set_linvel($vector(0., 0.), true);
                let act = w.bodies[h].activation_mut();
                act.normalized_linear_threshold = 1000.;
                act.angular_threshold = 1000.;
                act.time_until_sleep = 0.;
            }
            w.step();
            let pending = observed_islands(&w);
            assert!(pending.pending_split.is_some());
            w.step();
            let separated = observed_islands(&w);
            assert_eq!(separated.persistent_live, 2);
            w.wake_up(a, true);
            w.wake_up(b, true);
            w.bodies[b].set_translation($vector(0.9, 0.), true);
            w.step();
            let again = observed_islands(&w);
            assert_eq!(again.persistent_live, 1);
            w.remove_collider(ca).unwrap();
            w.step();
            let detached = observed_islands(&w);
            assert!(detached.contacts.is_empty());
            assert_eq!(detached.persistent_live, 2);
            w.remove_body(a).unwrap();
            w.remove_body(b).unwrap();
            w.step();
            let last = observed_islands(&w);
            assert_eq!(last.persistent_live, 0);
            assert_eq!(last.active_live, 1);
            assert!(last.active_bodies.is_empty());
            assert_eq!(last.active[last.awake.unwrap() as usize].unwrap().count, 0);
            assert!(!last.persistent_free.is_empty());
            assert!(!last.active_free.is_empty());
        }
    };
}
