//! Native structural observations before enforcing the inert broad-owner join.
macro_rules! broad_cases {
    ($rapier:ident,$vector:path,$cuboid:path) => {
        fn check_broad(
            w: &PhysicsWorld,
            narrow: &ValidatedNarrowOwner<'_>,
            arena: &ValidatedArena,
        ) {
            let (tree, co, r) =
                broad_capture::capture(&ordered_capture::capture(&w.broad_phase), identity());
            let colliders = narrow.context().rigid.collider_state();
            let tc = PendingBvhContext {
                colliders,
                pending_aabbs: &r.pending_aabbs,
            };
            if !r.pending_aabbs.is_empty() && r.frame <= 2 {
                broad_checks::pending(&tree, tc);
            }
            let tree = validate_pending_bvh(tree, tc, &mut budget()).unwrap();
            let tb = encode_pending_bvh(&tree, &mut budget()).unwrap();
            let tree = decode_pending_bvh(&tb, tc, &mut budget()).unwrap();
            assert_eq!(encode_pending_bvh(&tree, &mut budget()).unwrap(), tb);
            let co = validate_coarena(co, arena, &mut budget()).unwrap();
            let c = BroadOwnerContext {
                tree: &tree,
                narrow,
                adjacency: &co,
            };
            let v = validate_broad_owner(r.clone(), c, &mut budget())
                .unwrap_or_else(|e| panic!("{e:?} broad={r:?} tree={:?}", tree.record()));
            let b = encode_broad_owner(&v, &mut budget()).unwrap();
            let d = decode_broad_owner(&b, c, &mut budget()).unwrap();
            assert_eq!(d.record(), &r);
            assert_eq!(encode_broad_owner(&d, &mut budget()).unwrap(), b);
            for (h, _) in w.colliders.iter() {
                let h = handle(h.into_raw_parts());
                let expected = co.lookup(h).map(|row| {
                    let s = r.adjacency[row as usize];
                    &r.neighbors[s.start as usize..(s.start + s.count) as usize]
                });
                assert_eq!(d.neighbors(h), expected);
                assert!(
                    d.neighbors(Handle {
                        generation: h.generation.wrapping_add(1),
                        ..h
                    })
                    .is_none()
                );
            }
            if r.frame == 1 && r.pairs.len() == 3 {
                broad_checks::check(&r, c, arena);
            }
        }
        fn broad_record(w: &PhysicsWorld) -> BroadOwnerRecord {
            broad_capture::capture(&ordered_capture::capture(&w.broad_phase),identity()).2
        }
        fn broad_three() -> (PhysicsWorld, Vec<(RigidBodyHandle,ColliderHandle)>) {
            let mut w=PhysicsWorld::default();w.gravity=$vector(0.,0.);
            let owners=(0..3).map(|i|w.insert(RigidBodyBuilder::dynamic().translation($vector(i as f32*0.2,0.)), $cuboid(0.5,0.5).sensor(true).active_events(ActiveEvents::COLLISION_EVENTS))).collect();
            (w,owners)
        }
        #[test]
        fn broad_pairs_adjacency_mutations_and_current_geometry_are_distinct() {
            let (mut w, owners)=broad_three();
            assert_eq!(event(&mut w).len(),3);
            let initial=broad_record(&w);assert_eq!(initial.pairs.len(),3);
            assert_eq!(initial.neighbors.len(),6);assert!(initial.pending_aabbs.is_empty());
            // Pair scan precedes integration: current geometry can already be
            // far away while pair and pending-tree history must remain exact.
            w.bodies[owners[0].0].set_linvel($vector(120.,0.),true);
            assert!(event(&mut w).is_empty());
            assert!((w.bodies[owners[0].0].translation().x-120.*w.integration_parameters.dt).abs()<1e-6);
            assert!(w.colliders[owners[0].1].translation().x-w.colliders[owners[2].1].translation().x>1.,"actual cuboid extents no longer overlap");
            let moved=broad_record(&w);assert_eq!(moved.pairs.len(),3);assert!(!moved.pending_aabbs.is_empty());
            let mut restored=trusted_restore(&w);
            for tick in 0..24 {
                let mut events=vec![];
                for world in [&mut w,&mut restored] {
                    match tick {
                        2=>world.colliders[owners[1].1].set_enabled(false),
                        3=>world.colliders[owners[1].1].set_enabled(true),
                        4=>world.colliders[owners[1].1].set_collision_groups(InteractionGroups::none()),
                        5=>world.colliders[owners[1].1].set_collision_groups(InteractionGroups::all()),
                        6=>world.colliders[owners[2].1].set_sensor(false),
                        7=>world.colliders[owners[2].1].set_sensor(true),
                        8=>{world.remove_body(owners[2].0).unwrap();},
                        9=>{let (_,replacement)=world.insert(RigidBodyBuilder::dynamic(),$cuboid(0.5,0.5).sensor(true));assert_eq!(replacement.into_raw_parts().0,owners[2].1.into_raw_parts().0);assert_ne!(replacement,owners[2].1);},
                        _=>{}
                    }
                    events.push(event(world));
                }
                assert_eq!(serde_json::to_vec(&events[0]).unwrap(),serde_json::to_vec(&events[1]).unwrap());
                if tick==0 {assert_eq!(events[0].len(),2);assert!(events[0].iter().all(|e|e.stopped()));}
                assert_eq!(serde_json::to_vec(&w).unwrap(),serde_json::to_vec(&restored).unwrap());
                assert_eq!(broad_record(&w),broad_record(&restored));
            }
            eprintln!("BROAD_NATIVE dimension={} hostile_codec=complete three_pairs=true pending_geometry_distinct=true continuation_steps=24 toggle_remove_reuse=true",identity().dimension as u8);
        }
        #[test]
        fn broad_regime_strategy_adaptive_and_wrapping_history_continue() {
            for strategy in [$rapier::geometry::BvhOptimizationStrategy::SubtreeOptimizer,$rapier::geometry::BvhOptimizationStrategy::None] {
                let mut w=PhysicsWorld::default();w.gravity=$vector(0.,0.);
                w.broad_phase=$rapier::geometry::BroadPhaseBvh::with_optimization_strategy(strategy);
                w.broad_phase.adaptive_change_detection_margin=true;
                let owners:Vec<_>=(0..40).map(|i|w.insert(RigidBodyBuilder::dynamic().translation($vector(i as f32*3.,0.)),$cuboid(0.5,0.5).sensor(true))).collect();
                event(&mut w);event(&mut w);assert!(broad_record(&w).reinsert_leaf_updates);
                // Trusted synthetic counter placement exercises actual wrap,
                // not a claim of billions of executed frames.
                let mut raw=serde_json::to_value(&w).unwrap();raw["broad_phase"]["frame_index"]=Value::from(u32::MAX-1);
                let scratch=w.islands.checkpoint_sleep_scan().unwrap();
                w=serde_json::from_value(raw).unwrap();w.islands.restore_sleep_scan(scratch).unwrap();
                let mut restored=trusted_restore(&w);let mut saw_bulk=false;let mut saw_sparse=false;
                for tick in 0..12 {
                    for world in [&mut w,&mut restored] {
                        if tick==0 {world.bodies[owners[0].0].set_translation($vector(0.,1.),true);}
                        if tick==2 {for (i,(body,_)) in owners.iter().enumerate(){world.bodies[*body].set_translation($vector(i as f32*3.,2.),true);}}
                        event(world);
                    }
                    let r=broad_record(&w);saw_sparse|=r.reinsert_leaf_updates;saw_bulk|=!r.reinsert_leaf_updates;
                    if tick==1 {assert_eq!(r.frame,0);}
                    assert!(r.adaptive_margin);assert_eq!(r.optimization,if strategy==$rapier::geometry::BvhOptimizationStrategy::None{BroadOptimization::None}else{BroadOptimization::Subtree});
                    assert_eq!(serde_json::to_vec(&w).unwrap(),serde_json::to_vec(&restored).unwrap());
                    assert_eq!(r,broad_record(&restored));
                }
                assert!(saw_bulk&&saw_sparse);
            }
            eprintln!("BROAD_NATIVE dimension={} continuation_steps=24 two_strategies=true adaptive=true sparse_bulk=true actual_frame_wrap=true",identity().dimension as u8);
        }
        #[test]
        fn broad_pending_tree_preserves_deferred_parent_without_weakening_strict_bvh() {
            let (mut w,_,a,_)=sensor_fixture(true);
            w.insert_collider($cuboid(0.4,0.4).translation($vector(0.3,0.)),None);
            event(&mut w);w.colliders[a].set_sensor(false);event(&mut w);
            let (tree,_,r)=broad_capture::capture(&ordered_capture::capture(&w.broad_phase),identity());
            assert!(!r.pending_aabbs.is_empty());
            assert!(matches!(validate(tree,&mut budget()),Err(Error::Metadata|Error::Bounds)));
            // event() above already checked this exact native tree through the
            // separate pending witness, including hostile closure mutations.
            eprintln!("BROAD_NATIVE dimension={} strict_pending_refusal=true separate_witness_roundtrip=true pending_count={}",identity().dimension as u8,r.pending_aabbs.len());
        }
        #[test]
        fn broad_zero_dt_deferred_tree_is_not_a_complete_owner() {
            let (mut w,owners)=broad_three();event(&mut w);
            for (i,(body,_)) in owners.iter().enumerate(){w.bodies[*body].set_translation($vector(i as f32*3.,3.),true);}
            w.integration_parameters.dt=0.;
            native_step_events(&mut w);
            let (tree,_,r)=broad_capture::capture(&ordered_capture::capture(&w.broad_phase),identity());
            assert!(tree.nodes.is_empty()&&tree.leaves.occupied==0);
            assert_eq!(w.colliders.iter().filter(|(_,co)|co.is_enabled()).count(),3);
            assert_eq!(r.frame,2);
            // Empty bytes alone are a valid strict tree; they are not a complete
            // joined owner. The hostile codec control below checks that refusal
            // against actual nonempty collider witnesses. No native tree refill.
            validate(tree,&mut budget()).unwrap();
            eprintln!("BROAD_NATIVE dimension={} zero_dt_missing_tree=true enabled_colliders=3 live_admission=false",identity().dimension as u8);
        }
        fn broad_observation(w: &PhysicsWorld) {
            let b = ordered_capture::capture(&w.broad_phase);
            let h = |v: &Value| (word(&v["index"]), word(&v["generation"]));
            let pairs: Vec<_> = b["pairs"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| (h(&p[0][0]), h(&p[0][1])))
                .collect();
            let mut narrow: Vec<_> = w
                .narrow_phase
                .contact_graph()
                .raw_graph()
                .raw_edges()
                .iter()
                .map(|e| {
                    (
                        e.weight.collider1.into_raw_parts(),
                        e.weight.collider2.into_raw_parts(),
                    )
                })
                .collect();
            let g = w.narrow_phase.intersection_graph().raw_graph();
            let raw = ordered_capture::capture(g);
            narrow.extend(raw["edges"].as_array().unwrap().iter().map(|e| {
                (
                    g.raw_nodes()[word(&e["node"][0]) as usize]
                        .weight
                        .into_raw_parts(),
                    g.raw_nodes()[word(&e["node"][1]) as usize]
                        .weight
                        .into_raw_parts(),
                )
            }));
            for pair in &mut narrow {
                if pair.0 > pair.1 {
                    core::mem::swap(&mut pair.0, &mut pair.1);
                }
            }
            narrow.sort();
            assert_eq!(
                pairs, narrow,
                "actual completed broad pair set equals both narrow graphs"
            );
        }
    };
}
