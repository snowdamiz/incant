//! Trusted owner round trips only. Native serde is never an untrusted save path.
macro_rules! continuation_cases {
    ($rapier:ident,$vector:path,$cuboid:path) => {
        fn native_step_events(w:&mut PhysicsWorld)->(Vec<Value>,Vec<Value>){
            let (cs,cr)=std::sync::mpsc::channel();let (fs,fr)=std::sync::mpsc::channel();let (ss,sr)=std::sync::mpsc::channel();
            let events=ChannelEventCollector::new(cs,fs,ss);w.step_with_events(&(),&events);
            assert!(sr.try_iter().next().is_none());
            let collisions=cr.try_iter().map(|e|serde_json::to_value(e).unwrap()).collect();
            let forces=fr.try_iter().map(|e|serde_json::json!({
                "colliders":[e.collider1.into_raw_parts(),e.collider2.into_raw_parts()],
                "total_force_bits":e.total_force.to_array().map(f32::to_bits),
                "magnitude_bits":e.total_force_magnitude.to_bits(),
                "max_direction_bits":e.max_force_direction.to_array().map(f32::to_bits),
                "max_magnitude_bits":e.max_force_magnitude.to_bits(),"started":e.started
            })).collect();(collisions,forces)
        }
        fn trusted_restore(w: &PhysicsWorld) -> PhysicsWorld {
            let bytes = serde_json::to_vec(w).unwrap();
            // This scratch is taken from the bounded joined record after its
            // actual encode/decode, not copied straight around the validator.
            let (_,owner)=observe_with_owner(w,false).unwrap();
            let scan=$rapier::dynamics::SleepScanCheckpoint{stamp:owner.sleep_scan_stamp,slots:owner.sleep_scan.iter().map(|s|(s.stamp,s.eligible)).collect()};
            let mut restored: PhysicsWorld = serde_json::from_slice(&bytes).unwrap();
            restored.islands.restore_sleep_scan(scan).unwrap();
            assert_eq!(serde_json::to_vec(&restored).unwrap(), bytes);
            restored
        }
        #[test]
        fn trusted_complete_owner_continues_contacts_sleep_wake_and_slot_reuse() {
            let mut totals=[0usize;2];
            for checkpoint in [1, 20, 100] {
                let (mut uninterrupted, body, collider) = fixture(false);
                uninterrupted.colliders[collider].set_active_events(ActiveEvents::COLLISION_EVENTS|ActiveEvents::CONTACT_FORCE_EVENTS);
                for _ in 0..checkpoint {
                    uninterrupted.step();
                }
                observe(&uninterrupted, false).unwrap();
                let mut restored = trusted_restore(&uninterrupted);
                for tick in 0..100 {
                    let mut actual_events=vec![];
                    for w in [&mut uninterrupted, &mut restored] {
                        match tick {
                            12 => w.wake_up(body, true),
                            16 => w.bodies[body].apply_impulse($vector(0.2, 0.1), true),
                            25 => w.bodies[body].set_enabled(false),
                            27 => w.bodies[body].set_enabled(true),
                            40 => {
                                w.remove_body(body).unwrap();
                            }
                            42 => {
                                w.insert(
                                    RigidBodyBuilder::dynamic().translation($vector(0., 2.)),
                                    $cuboid(0.5, 0.5).active_events(ActiveEvents::COLLISION_EVENTS|ActiveEvents::CONTACT_FORCE_EVENTS),
                                );
                            }
                            _ => {}
                        }
                        actual_events.push(native_step_events(w));
                        observe(w, false).unwrap();
                    }
                    assert_eq!(actual_events[0],actual_events[1]);
                    totals[0]+=actual_events[0].0.len();totals[1]+=actual_events[0].1.len();
                    assert_eq!(
                        serde_json::to_vec(&uninterrupted).unwrap(),
                        serde_json::to_vec(&restored).unwrap(),
                        "checkpoint={checkpoint} tick={tick}"
                    );
                    assert_eq!(
                        uninterrupted.islands.checkpoint_sleep_scan().unwrap(),
                        restored.islands.checkpoint_sleep_scan().unwrap()
                    );
                }
            }
            assert!(totals[0]>0 && totals[1]>0,"actual event controls must execute");
            println!("trusted_continuation dimension={} checkpoints=3 steps=300 collision_events={} force_events={} state_and_scratch_exact=true",identity().dimension as u8,totals[0],totals[1]);
        }
        #[test]
        fn actual_sleep_stamp_wrap_requires_retained_scratch_not_zero_reconstruction() {
            let mut original = PhysicsWorld::default();
            original.gravity = $vector(0., 0.);
            let (body, _) = original.insert(RigidBodyBuilder::dynamic(), $cuboid(0.5, 0.5));
            original.step();
            // Synthetic counter placement, followed by real native stepping. Do
            // not describe this as executing four billion historical steps.
            for _ in 0..1024 {
                let activation=original.bodies[body].activation();
                if activation.time_since_can_sleep + original.integration_parameters.dt >= activation.time_until_sleep {break;}
                original.step();
            }
            let activation=original.bodies[body].activation();
            assert!(!activation.sleeping);
            assert!(activation.time_since_can_sleep + original.integration_parameters.dt >= activation.time_until_sleep);
            let mut json = serde_json::to_value(&original).unwrap();
            json["islands"]["persistent"]["sleep_scan_stamp"] = Value::from(u32::MAX);
            let bytes = serde_json::to_vec(&json).unwrap();
            let mut uninterrupted: PhysicsWorld = serde_json::from_slice(&bytes).unwrap();
            let mut scratch = original.islands.checkpoint_sleep_scan().unwrap();
            scratch.stamp = u32::MAX;
            assert!(scratch.slots.iter().any(|s| s.0 != 0));
            uninterrupted.islands.restore_sleep_scan(scratch).unwrap();
            let mut restored = trusted_restore(&uninterrupted);
            let mut discarded: PhysicsWorld = serde_json::from_slice(&bytes).unwrap();
            uninterrupted.step();
            restored.step();
            discarded.step();
            assert!(uninterrupted.bodies[body].is_sleeping());
            assert!(restored.bodies[body].is_sleeping());
            assert!(
                !discarded.bodies[body].is_sleeping(),
                "negative control must expose actual wrap alias"
            );
            assert_eq!(
                serde_json::to_vec(&uninterrupted).unwrap(),
                serde_json::to_vec(&restored).unwrap()
            );
            for tick in 0..32 {
                if tick == 3 {
                    uninterrupted.wake_up(body, true);
                    restored.wake_up(body, true);
                }
                uninterrupted.step();
                restored.step();
                assert_eq!(
                    serde_json::to_vec(&uninterrupted).unwrap(),
                    serde_json::to_vec(&restored).unwrap()
                );
                assert_eq!(
                    uninterrupted.islands.checkpoint_sleep_scan().unwrap(),
                    restored.islands.checkpoint_sleep_scan().unwrap()
                );
            }
        }
        #[test]
        fn scratch_install_refuses_mismatch_atomically() {
            let (mut w, _, _) = fixture(false);
            w.step();
            let before = w.islands.checkpoint_sleep_scan().unwrap();
            let mut bad = before.clone();
            bad.stamp = bad.stamp.wrapping_add(1);
            assert_eq!(
                w.islands.restore_sleep_scan(bad),
                Err($rapier::dynamics::SleepScanCheckpointError::Stamp)
            );
            let mut bad = before.clone();
            bad.slots.clear();
            assert_eq!(
                w.islands.restore_sleep_scan(bad),
                Err($rapier::dynamics::SleepScanCheckpointError::Length)
            );
            let mut bad = before.clone();
            bad.slots.resize(MAX_ITEMS + 1, (0, false));
            assert_eq!(
                w.islands.restore_sleep_scan(bad),
                Err($rapier::dynamics::SleepScanCheckpointError::Limit)
            );
            assert_eq!(w.islands.checkpoint_sleep_scan().unwrap(), before);
        }
    };
}
