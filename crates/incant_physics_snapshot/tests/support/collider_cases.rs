//! Native getter observations flow through the checked joined owner, never through
//! an untrusted Rapier deserializer or a live reconstruction of decoded data.
macro_rules! collider_cases {
    ($rapier:ident,$vector:path,$cuboid:path,$posebits:path) => {
        fn collider_row(w: &PhysicsWorld, h: ColliderHandle) -> ColliderState {
            collider_capture::collider(&ordered_capture::capture(&w.colliders[h]),identity().dimension)
        }
        #[test]
        fn actual_group_and_or_modes_change_sensor_intersections() {
            let mut w=PhysicsWorld::default();w.gravity=$vector(0.,0.);
            let group = |membership,filter,mode| InteractionGroups::new(membership,filter,mode);
            let (_,a)=w.insert(RigidBodyBuilder::dynamic(),$cuboid(0.5,0.5).sensor(true)
                .collision_groups(group(Group::GROUP_1,Group::GROUP_2,$rapier::geometry::InteractionTestMode::Or)));
            let b=w.insert_collider($cuboid(0.5,0.5)
                .collision_groups(group(Group::GROUP_2,Group::NONE,$rapier::geometry::InteractionTestMode::Or)),None);
            w.step();observe(&w,false).unwrap();
            assert_eq!(w.narrow_phase.intersection_pair(a,b),Some(true));
            assert_eq!(collider_row(&w,a).collision_groups.test,GroupTest::Or);
            w.colliders[a].set_collision_groups(group(Group::GROUP_1,Group::GROUP_2,$rapier::geometry::InteractionTestMode::And));
            w.step();observe(&w,false).unwrap();
            assert_eq!(w.narrow_phase.intersection_pair(a,b),Some(false));
            assert_eq!(collider_row(&w,a).collision_groups.test,GroupTest::And);
            w.colliders[b].set_collision_groups(group(Group::GROUP_2,Group::GROUP_1,$rapier::geometry::InteractionTestMode::And));
            w.step();observe(&w,false).unwrap();
            assert_eq!(w.narrow_phase.intersection_pair(a,b),Some(true));
            eprintln!("COLLIDER_NATIVE dimension={} group_intersections=true+false+true",identity().dimension as u8);
        }
        #[test]
        fn actual_collider_mass_material_groups_and_flags_roundtrip() {
            let (mut w, body, co) = fixture(true);
            w.gravity=$vector(0.,0.);
            w.bodies[body].set_translation($vector(1.,5.),true);
            let c=&mut w.colliders[co];
            c.set_friction(0.37);
            c.set_restitution(1.25);
            c.set_friction_combine_rule(CoefficientCombineRule::Min);
            c.set_restitution_combine_rule(CoefficientCombineRule::Multiply);
            c.set_contact_skin(0.0125);
            c.set_contact_force_event_threshold(4.5);
            c.set_active_events(ActiveEvents::all());
            c.set_active_hooks(ActiveHooks::all());
            c.set_active_collision_types(ActiveCollisionTypes::all());
            c.set_collision_groups(InteractionGroups::new(Group::GROUP_32,Group::GROUP_1,$rapier::geometry::InteractionTestMode::Or));
            c.set_solver_groups(InteractionGroups::new(Group::GROUP_2,Group::NONE,$rapier::geometry::InteractionTestMode::And));
            c.user_data=0xfedcba98765432100123456789abcdef;
            c.set_density(2.5);
            w.step(); observe(&w,false).unwrap();
            let r=collider_row(&w,co);
            assert_eq!(r.mass,ColliderMass::Density(2.5f32.to_bits()));
            assert_eq!(r.position,$posebits(w.colliders[co].position()));
            assert_eq!(r.material.friction,0.37f32.to_bits());
            assert_eq!(r.material.restitution,1.25f32.to_bits());
            assert_eq!(r.collision_groups.test,GroupTest::Or);
            assert_eq!(r.solver_groups.test,GroupTest::And);
            w.colliders[co].set_mass(3.25);
            w.step(); observe(&w,false).unwrap();
            assert_eq!(collider_row(&w,co).mass,ColliderMass::Mass(3.25f32.to_bits()));
            let mass=w.colliders[co].mass_properties();
            w.colliders[co].set_mass_properties(mass);
            w.step(); observe(&w,false).unwrap();
            assert_eq!(collider_row(&w,co).mass,ColliderMass::Properties(body_capture::local(&ordered_capture::capture(&mass),identity().dimension)));
            w.colliders[co].set_sensor(true);
            w.step(); observe(&w,false).unwrap();
            assert_eq!(collider_row(&w,co).kind,ColliderKind::Sensor);
            w.colliders[co].set_active_events(ActiveEvents::empty());
            w.step(); observe(&w,false).unwrap();
            assert_eq!(collider_row(&w,co).force_event_threshold,4.5f32.to_bits());
            eprintln!("COLLIDER_NATIVE dimension={} mass_modes=3 groups=and+or flags=all user_data=u128 joined=true",identity().dimension as u8);
        }
        #[test]
        fn actual_enabled_state_retains_explicit_enable_and_detached_parent_history() {
            let (mut enabled, body, co) = fixture(false);
            enabled.step();
            enabled.bodies[body].set_enabled(false); enabled.step();
            enabled.colliders[co].set_enabled(false); enabled.step();
            enabled.colliders[co].set_enabled(true); enabled.step();
            assert!(!enabled.bodies[body].is_enabled());
            assert!(enabled.colliders[co].is_enabled());
            assert_eq!(collider_row(&enabled,co).enabled,incant_physics_snapshot::ColliderEnabled::Enabled);
            let (mut detached, body, co2) = fixture(false);
            detached.step();
            detached.bodies[body].set_enabled(false); detached.step();
            detached.colliders.set_parent(co2,None,&mut detached.bodies); detached.step();
            assert!(detached.colliders[co2].parent().is_none());
            assert_eq!(collider_row(&detached,co2).enabled,incant_physics_snapshot::ColliderEnabled::DisabledByParent);
            eprintln!("COLLIDER_NATIVE dimension={} enabled_with_disabled_parent=true detached_disabled_by_parent=true",identity().dimension as u8);
            observe(&enabled,false).unwrap();
            observe(&detached,false).unwrap();
        }
        #[test]
        fn actual_collider_parent_disable_user_disable_and_standalone() {
            let (mut w, body, co)=fixture(false);
            w.step(); observe(&w,false).unwrap();
            w.bodies[body].set_enabled(false);
            w.step(); observe(&w,false).unwrap();
            assert_eq!(collider_row(&w,co).enabled,incant_physics_snapshot::ColliderEnabled::DisabledByParent);
            w.colliders[co].set_enabled(false);
            w.step(); observe(&w,false).unwrap();
            assert_eq!(collider_row(&w,co).enabled,incant_physics_snapshot::ColliderEnabled::Disabled);
            w.bodies[body].set_enabled(true);
            w.step(); observe(&w,false).unwrap();
            assert_eq!(collider_row(&w,co).enabled,incant_physics_snapshot::ColliderEnabled::Disabled);
            w.colliders[co].set_enabled(true);
            w.step(); observe(&w,false).unwrap();
            assert_eq!(collider_row(&w,co).enabled,incant_physics_snapshot::ColliderEnabled::Enabled);
            let standalone=w.insert_collider($cuboid(0.5,0.5).translation($vector(9.,0.)).enabled(false),None);
            w.step(); observe(&w,false).unwrap();
            assert_eq!(collider_row(&w,standalone).enabled,incant_physics_snapshot::ColliderEnabled::Disabled);
            w.colliders[standalone].set_enabled(true);
            w.step(); observe(&w,false).unwrap();
            assert_eq!(collider_row(&w,standalone).enabled,incant_physics_snapshot::ColliderEnabled::Enabled);
            eprintln!("COLLIDER_NATIVE dimension={} enabled=all3 standalone=true",identity().dimension as u8);
        }
    };
}
