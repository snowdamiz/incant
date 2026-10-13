// Shared actual-native fixture for capture and private construction tests.
// No serialized native state is ever used to initialize a live owner.
macro_rules! manager_fixture {
    ($vector:path) => {
        fn manager_change(w: &mut PhysicsWorld, tick: usize, owners: &mut [RigidBodyHandle; 3]) {
            match tick {
                3 => w.bodies[owners[0]].sleep(),
                7 => w.bodies[owners[0]].wake_up(true),
                9 => w.bodies[owners[1]].set_translation($vector(4., 1.), true),
                13 => {
                    w.remove_body(owners[1]).unwrap();
                }
                15 => {
                    owners[1] = w
                        .insert(
                            RigidBodyBuilder::dynamic().translation($vector(1., 0.5)),
                            ColliderBuilder::ball(0.5),
                        )
                        .0;
                }
                21 => w.bodies[owners[1]].set_enabled(false),
                24 => w.bodies[owners[1]].set_enabled(true),
                29 => w.bodies[owners[2]].set_body_type(RigidBodyType::Fixed, true),
                32 => w.bodies[owners[2]].set_body_type(RigidBodyType::Dynamic, true),
                37 => w.bodies[owners[0]].set_additional_solver_iterations(2),
                41 => {
                    w.remove_body(owners[2]).unwrap();
                }
                45 => {
                    owners[2] = w
                        .insert(
                            RigidBodyBuilder::dynamic().translation($vector(-1., 0.5)),
                            ColliderBuilder::ball(0.5),
                        )
                        .0;
                }
                51 => w.bodies[owners[0]].sleep(),
                55 => w.bodies[owners[0]].wake_up(true),
                _ => {}
            }
        }
        fn manager_run(mut check: impl FnMut(usize, &mut PhysicsWorld, [RigidBodyHandle; 3])) {
            let mut w = PhysicsWorld::default();
            w.insert_collider(
                ColliderBuilder::ball(100.).translation($vector(0., -100.5)),
                None,
            );
            let mut owners = [0., 1., -1.].map(|x| {
                w.insert(
                    RigidBodyBuilder::dynamic().translation($vector(x, 0.0)),
                    ColliderBuilder::ball(0.5),
                )
                .0
            });
            for _ in 0..64 {
                w.step();
            }
            for tick in 0..64 {
                manager_change(&mut w, tick, &mut owners);
                w.step();
                check(tick, &mut w, owners);
            }
        }
    };
}
