// Shared actual-native event fixture. Capture is one-way; no native deserialize.
macro_rules! narrow_fixture {
    ($vector:path, $rapier:path) => {
        #[derive(Default)]
        struct NarrowEvents(std::sync::Mutex<Vec<Vec<u32>>>);
        impl EventHandler for NarrowEvents {
            fn handle_collision_event(
                &self,
                _: &RigidBodySet,
                _: &ColliderSet,
                event: CollisionEvent,
                _: Option<&ContactPair>,
            ) {
                let (started, a, b, flags) = match event {
                    CollisionEvent::Started(a, b, f) => (1, a, b, f),
                    CollisionEvent::Stopped(a, b, f) => (0, a, b, f),
                };
                let (a, g) = a.into_raw_parts();
                let (b, h) = b.into_raw_parts();
                self.0
                    .lock()
                    .unwrap()
                    .push(vec![0, started, a, g, b, h, flags.bits()]);
            }
            fn handle_contact_force_event(
                &self,
                dt: Real,
                _: &RigidBodySet,
                _: &ColliderSet,
                pair: &ContactPair,
                magnitude: Real,
            ) {
                use $rapier as native;
                let e = native::geometry::ContactForceEvent::from_contact_pair(dt, pair, magnitude);
                let (a, g) = e.collider1.into_raw_parts();
                let (b, h) = e.collider2.into_raw_parts();
                let mut row = vec![
                    1,
                    u32::from(e.started),
                    a,
                    g,
                    b,
                    h,
                    dt.to_bits(),
                    e.total_force_magnitude.to_bits(),
                    e.max_force_magnitude.to_bits(),
                ];
                row.extend(e.total_force.to_array().iter().map(|v| v.to_bits()));
                row.extend(e.max_force_direction.to_array().iter().map(|v| v.to_bits()));
                self.0.lock().unwrap().push(row);
            }
            fn handle_soft_body_tear_event(&self, _: &SoftBodySet, _: &SoftBodyTearEvent) {
                panic!("rigid fixture");
            }
        }
        fn narrow_step(w: &mut PhysicsWorld) -> Vec<Vec<u32>> {
            let events = NarrowEvents::default();
            w.step_with_events(&(), &events);
            let mut rows = events.0.into_inner().unwrap();
            // Parallel handler scheduling is not an event-order promise. Every complete
            // identity/flag/float-bit record is compared, retaining duplicates.
            rows.sort();
            rows
        }
        fn narrow_change(
            w: &mut PhysicsWorld,
            tick: usize,
            owners: &mut [RigidBodyHandle; 3],
            sensor: ColliderHandle,
        ) {
            manager_change(w, tick, owners);
            for (_, c) in w.colliders.iter_mut() {
                c.set_active_events(if (34..38).contains(&tick) {
                    ActiveEvents::empty()
                } else {
                    ActiveEvents::COLLISION_EVENTS | ActiveEvents::CONTACT_FORCE_EVENTS
                });
                c.set_contact_force_event_threshold(if (18..23).contains(&tick) {
                    f32::MAX
                } else {
                    0.
                });
            }
            match tick {
                27 => w.colliders[sensor].set_sensor(false),
                31 => w.colliders[sensor].set_sensor(true),
                _ => {}
            }
        }
        fn narrow_run(
            mut check: impl FnMut(
                usize,
                &mut PhysicsWorld,
                [RigidBodyHandle; 3],
                ColliderHandle,
                Vec<Vec<u32>>,
            ),
        ) {
            let mut w = PhysicsWorld::default();
            w.insert_collider(
                ColliderBuilder::ball(100.).translation($vector(0., -100.5)),
                None,
            );
            let sensor = w.insert_collider(
                ColliderBuilder::ball(1.25)
                    .translation($vector(0., 0.5))
                    .sensor(true),
                None,
            );
            let mut owners = [0., 1., -1.].map(|x| {
                w.insert(
                    RigidBodyBuilder::dynamic().translation($vector(x, 0.)),
                    ColliderBuilder::ball(0.5),
                )
                .0
            });
            for (_, c) in w.colliders.iter_mut() {
                c.set_active_events(
                    ActiveEvents::COLLISION_EVENTS | ActiveEvents::CONTACT_FORCE_EVENTS,
                );
                c.set_contact_force_event_threshold(0.);
            }
            for _ in 0..64 {
                narrow_step(&mut w);
            }
            for tick in 0..64 {
                narrow_change(&mut w, tick, &mut owners, sensor);
                let events = narrow_step(&mut w);
                check(tick, &mut w, owners, sensor, events);
            }
        }
    };
}
