//! Actual pinned owner/event observations; hostile data stays in inert codecs.
macro_rules! narrow_cases {
    ($rapier:ident,$vector:path,$cuboid:path) => {
        fn narrow_record(w: &PhysicsWorld) -> (CoarenaRecord, NarrowOwnerRecord) {
            let native = ordered_capture::capture(&w.narrow_phase);
            let data = native["graph_indices"]["data"].as_array().unwrap();
            let co = CoarenaRecord { identity: identity(), rows: data.iter().enumerate().map(|(i,v)| CoarenaRow {generation:word(&v[0]),row:i as u32}).collect() };
            let raw = w.narrow_phase.intersection_graph().raw_graph();
            let record = NarrowOwnerRecord {
                identity: identity(),
                indices: data.iter().map(|v| ColliderGraphIndices {contact:word(&v[1]["contact_graph_index"]),intersection:word(&v[1]["intersection_graph_index"])}).collect(),
                nodes: raw.raw_nodes().iter().map(|n|handle(n.weight.into_raw_parts())).collect(),
                intersections: raw.raw_edges().iter().map(|e| IntersectionState {intersecting:e.weight.intersecting,start_event_emitted:ordered_capture::capture(&e.weight)["start_event_emitted"].as_bool().unwrap()}).collect(),
            };
            (co,record)
        }
        fn check_narrow(w:&PhysicsWorld,rigid:&ValidatedRigidOwner<'_>,arena:&ValidatedArena) {
            broad_observation(w);
            let (co,r)=narrow_record(w);
            let co=validate_coarena(co,arena,&mut budget()).unwrap();
            let bytes=encode_coarena(&co,&mut budget()).unwrap();
            let indices=decode_coarena(&bytes,arena,&mut budget()).unwrap();
            let graph=graph(&ordered_capture::capture(w.narrow_phase.intersection_graph().raw_graph()),identity());
            let graph_bytes=encode_graph(&graph,&mut budget()).unwrap();
            let graph=decode_graph(&graph_bytes,identity(),&mut budget()).unwrap();
            let c=NarrowOwnerContext{rigid,indices:&indices,intersections:&graph};
            let v=validate_narrow_owner(r.clone(),c,&mut budget()).unwrap_or_else(|e|panic!("{e:?} narrow={r:?}"));
            check_broad(w,&v,arena);
            let bytes=encode_narrow_owner(&v,&mut budget()).unwrap();
            let d=decode_narrow_owner(&bytes,c,&mut budget()).unwrap();
            if !r.intersections.is_empty() { narrow_checks::check(&r,c); }
            assert_eq!(d.record(),&r);assert_eq!(encode_narrow_owner(&d,&mut budget()).unwrap(),bytes);
            for (i,node) in graph.record().nodes.iter().enumerate() {
                let h=r.nodes[node.row as usize];
                assert_eq!(d.collider_indices(h).unwrap().intersection,i as u32);
                let observed:Vec<_>=w.narrow_phase.intersection_pairs_with(ColliderHandle::from_raw_parts(h.slot,h.generation)).collect();
                let decoded:Vec<_>=graph.incident(i as u32).unwrap().map(|edge| {
                    let e=graph.record().edges[edge as usize];
                    let hs=e.node.map(|n|r.nodes[graph.record().nodes[n as usize].row as usize]);
                    (ColliderHandle::from_raw_parts(hs[0].slot,hs[0].generation),ColliderHandle::from_raw_parts(hs[1].slot,hs[1].generation),d.intersection(edge).unwrap().intersecting)
                }).collect();
                assert_eq!(decoded,observed,"native adjacency order and actual intersection values");
                assert!(d.collider_indices(Handle{generation:h.generation.wrapping_add(1),..h}).is_none());
            }
            let mut permuted=indices.record().clone();let n=permuted.rows.len() as u32;
            for row in &mut permuted.rows {row.row=n-1-row.row;}
            let permuted=validate_coarena(permuted,arena,&mut budget()).unwrap();
            let mut reversed=r.clone();reversed.indices.reverse();
            let reversed=validate_narrow_owner(reversed,NarrowOwnerContext{indices:&permuted,..c},&mut budget()).unwrap();
            for (h,_) in w.colliders.iter() {let h=handle(h.into_raw_parts());assert_eq!(reversed.collider_indices(h),d.collider_indices(h));}
            // Equal arena bytes do not authorize a different checked owner.
            let other=validate_arena(arena.record().clone(),&mut budget()).unwrap();
            let other_co=validate_coarena(indices.record().clone(),&other,&mut budget()).unwrap();
            assert!(matches!(validate_narrow_owner(r.clone(),NarrowOwnerContext{indices:&other_co,..c},&mut budget()),Err(Error::Identity)));
            let mut bad=r.clone();bad.identity.build[0]^=1;
            assert!(matches!(validate_narrow_owner(bad,c,&mut budget()),Err(Error::Identity)));
            if !r.nodes.is_empty() {
                let mut bad=r.clone();bad.nodes[0].generation=u32::MAX;
                assert!(matches!(validate_narrow_owner(bad,c,&mut budget()),Err(Error::Generation)));
                let mut bad=r.clone();let row=indices.lookup(r.nodes[0]).unwrap() as usize;bad.indices[row].intersection=u32::MAX;
                assert!(matches!(validate_narrow_owner(bad,c,&mut budget()),Err(Error::Ownership)));
            }
            if !r.indices.is_empty() {
                assert!(decode_narrow_owner(&bytes,c,&mut Budget::new(0).unwrap()).is_err());
                let mut bad=r.clone();bad.indices[0].contact=u32::MAX-1;
                assert!(validate_narrow_owner(bad,c,&mut budget()).is_err());
            }
            if !r.intersections.is_empty() {
                let offset=60+8*(r.indices.len()+r.nodes.len());
                let mut bad=bytes.clone();bad[offset..offset+4].copy_from_slice(&2u32.to_le_bytes());
                assert!(matches!(decode_narrow_owner(&bad,c,&mut budget()),Err(Error::Header)));
                let mut bad=bytes.clone();bad[offset+4..offset+8].copy_from_slice(&u32::MAX.to_le_bytes());
                assert!(matches!(decode_narrow_owner(&bad,c,&mut budget()),Err(Error::Header)));
                // Historical bits are independent; all four combinations have
                // a lossless inert representation, no current-mask inference.
                for intersecting in [false,true] {for emitted in [false,true] {
                    let mut changed=r.clone();changed.intersections[0]=IntersectionState{intersecting,start_event_emitted:emitted};
                    let changed=validate_narrow_owner(changed,c,&mut budget()).unwrap();
                    let b=encode_narrow_owner(&changed,&mut budget()).unwrap();
                    assert_eq!(decode_narrow_owner(&b,c,&mut budget()).unwrap().record(),changed.record());
                }}
            }
            for offset in [0,8,12,44,48,52,56] {
                let mut bad=bytes.clone();bad[offset]^=0x80;
                assert!(decode_narrow_owner(&bad,c,&mut budget()).is_err());
            }
            assert!(decode_narrow_owner(&bytes[..bytes.len()-1],c,&mut budget()).is_err());
            let mut long=bytes.clone();long.push(0);assert!(decode_narrow_owner(&long,c,&mut budget()).is_err());
        }
        fn sensor_fixture(events:bool)->(PhysicsWorld,RigidBodyHandle,ColliderHandle,ColliderHandle) {
            let mut w=PhysicsWorld::default();w.gravity=$vector(0.,0.);
            let (body,a)=w.insert(RigidBodyBuilder::dynamic(),$cuboid(0.5,0.5).sensor(true).active_events(if events {ActiveEvents::COLLISION_EVENTS}else{ActiveEvents::empty()}));
            let b=w.insert_collider($cuboid(0.5,0.5),None);
            (w,body,a,b)
        }
        fn sensor_state(w:&PhysicsWorld)->IntersectionState {
            observe(w,false).unwrap();
            let (_,r)=narrow_record(w);assert_eq!(r.intersections.len(),1);r.intersections[0]
        }
        fn event(w:&mut PhysicsWorld)->Vec<$rapier::geometry::CollisionEvent> {
            let (c,f)=native_step_events(w);assert!(f.is_empty());observe(w,false).unwrap();
            c.into_iter().map(|e|serde_json::from_value(e).unwrap()).collect()
        }
        #[test]
        fn narrow_sensor_enter_stay_exit_and_toggle_preserve_emitted_history() {
            let (mut w,_,a,b)=sensor_fixture(true);
            let first=event(&mut w);assert_eq!(first.len(),1);assert!(first[0].started()&&first[0].sensor());
            assert_eq!(w.narrow_phase.intersection_pair(a,b),Some(true));
            assert_eq!(sensor_state(&w),IntersectionState{intersecting:true,start_event_emitted:true});
            assert!(event(&mut w).is_empty());
            w.colliders[a].set_active_events(ActiveEvents::empty());
            // Reject via interaction groups while AABBs still overlap. The
            // retained false intersection keeps the historical emitted bit.
            w.colliders[a].set_collision_groups(InteractionGroups::none());
            assert!(event(&mut w).is_empty());
            assert_eq!(sensor_state(&w),IntersectionState{intersecting:false,start_event_emitted:true});
            let mut restored=trusted_restore(&w);
            for world in [&mut w,&mut restored] {world.colliders[a].set_active_events(ActiveEvents::COLLISION_EVENTS);}
            assert_eq!(serde_json::to_value(event(&mut w)).unwrap(),serde_json::to_value(event(&mut restored)).unwrap());
            for world in [&mut w,&mut restored] {world.colliders[a].set_collision_groups(InteractionGroups::all());}
            let enter=event(&mut w);assert_eq!(serde_json::to_value(&enter).unwrap(),serde_json::to_value(event(&mut restored)).unwrap());assert_eq!(enter.len(),1);assert!(enter[0].started());
            for world in [&mut w,&mut restored] {world.colliders[a].set_collision_groups(InteractionGroups::none());}
            let exit=event(&mut w);assert_eq!(serde_json::to_value(&exit).unwrap(),serde_json::to_value(event(&mut restored)).unwrap());assert_eq!(exit.len(),1);assert!(exit[0].stopped()&&exit[0].sensor());
            assert_eq!(sensor_state(&w),IntersectionState{intersecting:false,start_event_emitted:false});
            assert_eq!(serde_json::to_vec(&w).unwrap(),serde_json::to_vec(&restored).unwrap());
            eprintln!("NARROW_NATIVE dimension={} enter_stay_exit_toggle=true historical_false_true=true",identity().dimension as u8);
        }
        #[test]
        fn narrow_sensor_enable_events_does_not_invent_enter_and_removal_reuses_generation() {
            let (mut w,_,a,b)=sensor_fixture(false);
            assert!(event(&mut w).is_empty());assert_eq!(sensor_state(&w),IntersectionState{intersecting:true,start_event_emitted:false});
            w.colliders[a].set_active_events(ActiveEvents::COLLISION_EVENTS);
            assert!(event(&mut w).is_empty());assert!(!sensor_state(&w).start_event_emitted);
            w.colliders[a].set_collision_groups(InteractionGroups::none());
            let stop=event(&mut w);assert_eq!(stop.len(),1);assert!(stop[0].stopped());
            w.colliders[a].set_collision_groups(InteractionGroups::all());assert_eq!(event(&mut w).len(),1);
            w.colliders[a].set_active_events(ActiveEvents::empty());assert!(event(&mut w).is_empty());
            let before=narrow_record(&w);assert!(before.1.intersections[0].start_event_emitted);
            w.remove_collider(b).unwrap();
            assert!(observe(&w,true).is_none());
            let removed=event(&mut w);assert_eq!(removed.len(),1);assert!(removed[0].stopped()&&removed[0].sensor()&&removed[0].removed());
            let after=narrow_record(&w);assert!(after.1.intersections.is_empty());
            assert_eq!(after.0.rows[b.into_raw_parts().0 as usize].generation,u32::MAX);
            assert_eq!(after.1.indices[b.into_raw_parts().0 as usize],ColliderGraphIndices{contact:u32::MAX,intersection:u32::MAX});
            let replacement=w.insert_collider($cuboid(0.5,0.5),None);
            assert_eq!(replacement.into_raw_parts().0,b.into_raw_parts().0);assert_ne!(replacement,b);
            assert!(event(&mut w).is_empty());
            assert_eq!(w.narrow_phase.intersection_pair(a,b),None);
            assert_eq!(w.narrow_phase.intersection_pair(a,replacement),Some(true));
            assert!(!sensor_state(&w).start_event_emitted);
            eprintln!("NARROW_NATIVE dimension={} late_mask_no_enter=true removal_without_mask=true stale_generation_refused=true",identity().dimension as u8);
        }
        #[test]
        fn narrow_contact_sensor_transfer_keeps_isolated_nodes_and_order() {
            let (mut w,_,a,_)=sensor_fixture(true);
            let extra=w.insert_collider($cuboid(0.4,0.4).translation($vector(0.3,0.)),None);
            event(&mut w);let first=narrow_record(&w);assert_eq!(first.1.intersections.len(),2);
            w.colliders[a].set_sensor(false);event(&mut w);
            let solid=narrow_record(&w);assert!(solid.1.intersections.is_empty());assert_eq!(solid.1.nodes,first.1.nodes);
            assert_eq!(w.narrow_phase.contact_graph().raw_graph().raw_edges().len(),2);
            w.colliders[a].set_sensor(true);event(&mut w);
            assert_eq!(narrow_record(&w).1.intersections.len(),2);
            w.remove_collider(extra).unwrap();event(&mut w);
            assert_eq!(narrow_record(&w).1.intersections.len(),1);
            eprintln!("NARROW_NATIVE dimension={} kind_transfer=true graph_order=true isolated_nodes=true swap_remove=true",identity().dimension as u8);
        }

        #[test]
        fn narrow_spatial_exit_and_retired_pair_churn_continue_exactly() {
            let (mut w,body,collider)=fixture(false);
            w.gravity=$vector(0.,0.);
            w.bodies[body].set_linvel($vector(0.,0.),true);
            w.colliders[collider].set_active_events(ActiveEvents::COLLISION_EVENTS);
            let initial=event(&mut w);assert!(initial.iter().any(|e|e.started()));
            assert_eq!(w.narrow_phase.contact_graph().raw_graph().raw_edges().len(),1);
            w.bodies[body].set_translation($vector(30.,30.),true);
            let exit=event(&mut w);assert!(exit.iter().any(|e|e.stopped()));
            assert!(w.narrow_phase.contact_graph().raw_graph().raw_edges().is_empty());
            // remove_pair retires this contact into the bounded pool. Native
            // serde omits the pool: compare actual reused versus fresh pair
            // paths below, backed by reset_for_reuse source audit, not skip alone.
            let mut restored=trusted_restore(&w);let mut counts=[0usize;3];
            for tick in 0..64 {
                for world in [&mut w,&mut restored] {
                    match tick%8 {
                        0=> {world.bodies[body].set_translation($vector(1.,1.49),true);world.bodies[body].set_linvel($vector(0.,0.),true);}
                        2=>world.colliders[collider].set_sensor(true),
                        4=>world.bodies[body].set_translation($vector(30.,30.),true),
                        6=>world.colliders[collider].set_sensor(false),
                        _=>{}
                    }
                }
                let a=event(&mut w);let b=event(&mut restored);
                assert_eq!(serde_json::to_value(&a).unwrap(),serde_json::to_value(&b).unwrap(),"tick={tick}");
                for e in a {counts[0]+=usize::from(e.started());counts[1]+=usize::from(e.stopped());counts[2]+=usize::from(e.sensor());}
                assert_eq!(serde_json::to_vec(&w).unwrap(),serde_json::to_vec(&restored).unwrap(),"tick={tick}");
                assert_eq!(w.islands.checkpoint_sleep_scan().unwrap(),restored.islands.checkpoint_sleep_scan().unwrap());
            }
            assert!(counts.iter().all(|n|*n>0));
            eprintln!("NARROW_NATIVE dimension={} continued_steps=64 retired_pair_reuse=true started={} stopped={} sensor_events={}",identity().dimension as u8,counts[0],counts[1],counts[2]);
        }
    };
}
