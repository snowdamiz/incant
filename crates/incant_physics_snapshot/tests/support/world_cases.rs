//! Typed live clones are deliberately separate from all hostile inert decoding.
macro_rules! world_cases {
 ($rapier:ident,$vector:path,$cuboid:path) => {
 fn fresh_world(w:&mut PhysicsWorld)->PhysicsWorld {
  // The caller owns an actual completed step. No deserialization, decoded data,
  // pointer reattachment or generic external-state setter participates here.
  assert!(w.integration_parameters.dt>0.);
  PhysicsWorld { gravity:w.gravity,integration_parameters:w.integration_parameters,
   physics_pipeline:w.physics_pipeline.checkpoint_fresh_workspace().unwrap(),
   collision_pipeline:CollisionPipeline::new(),islands:w.islands.clone(),broad_phase:w.broad_phase.clone(),narrow_phase:w.narrow_phase.clone(),
   bodies:w.bodies.clone(),colliders:w.colliders.clone(),impulse_joints:w.impulse_joints.clone(),multibody_joints:w.multibody_joints.clone(),soft_bodies:w.soft_bodies.clone(),ccd_solver:w.ccd_solver.clone() }
 }
 fn same_world(a:&mut PhysicsWorld,b:&mut PhysicsWorld){
  assert_eq!(serde_json::to_vec(a).unwrap(),serde_json::to_vec(b).unwrap());
  assert_eq!(world_cache(a),world_cache(b));
  assert_eq!(a.physics_pipeline.checkpoint_observation().unwrap(),b.physics_pipeline.checkpoint_observation().unwrap());
  assert_eq!(world_bytes(a),world_bytes(b));
  assert_eq!(a.physics_pipeline.counters.ccd.num_substeps,b.physics_pipeline.counters.ccd.num_substeps);
 }
 #[test] fn world_boundary_warm_contacts_sleep_and_wake_continue_exactly(){
  let(mut w,body,_)=fixture(true);
  for _ in 0..48{w.step();}
  let bytes=world_bytes(&mut w);assert!(!bytes.is_empty());
  let mut fresh=fresh_world(&mut w);
  // Fresh counters intentionally reset until the first actual continuation step.
  let mut warm=false;let mut sleep=false;
  for tick in 0..96 { for world in [&mut w,&mut fresh] {
   match tick {8=>world.bodies[body].sleep(),24=>world.bodies[body].wake_up(true),40=>world.bodies[body].set_linvel($vector(0.12,0.),true),_=>{}}
  }
   let ea=native_step_events(&mut w);let eb=native_step_events(&mut fresh);assert_eq!(ea,eb);same_world(&mut w,&mut fresh);
   sleep|=w.bodies[body].is_sleeping();
   warm |= w.narrow_phase.contact_graph().raw_graph().raw_edges().iter().any(|e| {let r=e.weight.rigid().unwrap();r.manifolds.iter().chain(&r.solver_clusters).any(|m| m.points.iter().any(|p| p.data.warmstart_impulse != 0.))});
  }
  assert!(warm&&sleep,"warm={warm} sleep={sleep}");eprintln!("WORLD_BOUNDARY dimension={} steps=96 warm=true sleep_wake=true typed_clone=true inert_bytes_not_installed=true",identity().dimension as u8);
 }
 #[test] fn world_boundary_sensor_order_remove_reuse_and_events_continue(){
  let(mut w,owners)=broad_three();native_step_events(&mut w);world_bytes(&mut w);
  let mut fresh=fresh_world(&mut w);let mut transitions=0;
  for tick in 0..32{for world in [&mut w,&mut fresh]{match tick{
   1=>world.bodies[owners[0].0].set_translation($vector(4.,0.),true),
   3=>world.bodies[owners[0].0].set_translation($vector(0.,0.),true),
   5=>world.colliders[owners[1].1].set_active_events(ActiveEvents::empty()),
   7=>world.colliders[owners[1].1].set_collision_groups(InteractionGroups::none()),
   9=>world.colliders[owners[1].1].set_active_events(ActiveEvents::COLLISION_EVENTS),
   11=>world.colliders[owners[1].1].set_collision_groups(InteractionGroups::all()),
   13=>{world.remove_body(owners[2].0).unwrap();},
   15=>{let(b,c)=world.insert(RigidBodyBuilder::dynamic(),$cuboid(0.5,0.5).sensor(true).active_events(ActiveEvents::COLLISION_EVENTS));assert_eq!(b.into_raw_parts().0,owners[2].0.into_raw_parts().0);assert_ne!(b,owners[2].0);assert_ne!(c,owners[2].1);},_=>{}}}
   let a=native_step_events(&mut w);let b=native_step_events(&mut fresh);transitions+=a.0.len();assert_eq!(a,b);same_world(&mut w,&mut fresh);
  }assert!(transitions>=6);eprintln!("WORLD_BOUNDARY dimension={} steps=32 events={} removal_reuse=true",identity().dimension as u8,transitions);
 }
 #[test] fn world_boundary_retained_ccd_and_omission_negative_control(){
  let mut w=PhysicsWorld::default();w.gravity=$vector(-0.,0.);w.insert_collider($cuboid(0.25,0.25),None);
  let(body,_)=w.insert(RigidBodyBuilder::dynamic().translation($vector(-20.,10.)).linvel($vector(60.,0.)),$cuboid(0.1,0.1));w.step();
  assert_eq!(w.physics_pipeline.checkpoint_observation().unwrap().post_solve_ccd_active,Some(true));world_bytes(&mut w);
  let mut fresh=fresh_world(&mut w);let mut omitted=fresh_world(&mut w);omitted.physics_pipeline=PhysicsPipeline::new();
  for world in [&mut w,&mut fresh,&mut omitted]{world.remove_body(body).unwrap();}
  for _ in 0..4 {w.step();fresh.step();omitted.step();same_world(&mut w,&mut fresh);
   assert_eq!(serde_json::to_vec(&w).unwrap(),serde_json::to_vec(&omitted).unwrap());
   assert_ne!(world_cache(&w),world_cache(&omitted));assert_eq!(w.physics_pipeline.counters.ccd.num_substeps,1);assert_eq!(omitted.physics_pipeline.counters.ccd.num_substeps,0);
  }
  for world in [&mut w,&mut fresh] {world.insert(RigidBodyBuilder::dynamic().translation($vector(-20.,10.)).linvel($vector(60.,0.)),$cuboid(0.1,0.1));}
  for tick in 0..8{for world in [&mut w,&mut fresh]{world.integration_parameters.max_ccd_substeps=if tick%2==0{0}else{1};world.step();}same_world(&mut w,&mut fresh);}
  eprintln!("WORLD_BOUNDARY dimension={} steps=12 retained_ccd=true omitted_cache_counter_divergence=true no_trajectory_claim=true",identity().dimension as u8);
 }
 #[test] fn world_boundary_refuses_actual_deferred_and_quarantined_work_without_consuming(){
  use $rapier::pipeline::CheckpointWorkspaceError;
  let(mut w,owners)=broad_three();w.step();w.step();for(i,(b,_))in owners.iter().enumerate(){w.bodies[*b].set_translation($vector(i as f32*3.,3.),true);}
  w.integration_parameters.dt=0.;w.step();let before=w.physics_pipeline.checkpoint_observation().unwrap();assert!(before.deferred_bvh_receiver^before.deferred_bvh_inline);
  assert!(matches!(w.physics_pipeline.checkpoint_fresh_workspace(),Err(CheckpointWorkspaceError::DeferredBvh)));assert_eq!(before,w.physics_pipeline.checkpoint_observation().unwrap());
  w.integration_parameters.dt=1./60.;w.step();w.physics_pipeline.checkpoint_fresh_workspace().unwrap();
  w.bodies[owners[0].0].set_linvel($vector(f32::NAN,0.),true);w.step();let before=w.physics_pipeline.checkpoint_observation().unwrap();assert_eq!(before.quarantine_bodies,1);
  assert!(matches!(w.physics_pipeline.checkpoint_fresh_workspace(),Err(CheckpointWorkspaceError::Quarantine)));assert_eq!(before,w.physics_pipeline.checkpoint_observation().unwrap());
 }
 #[test] fn world_boundary_hostile_sections_budget_and_exact_witness_join(){
  let(mut w,_)=broad_three();w.step();let bytes=world_bytes(&mut w);world_checks::hostile(&bytes,identity());
  let mut fresh=fresh_world(&mut w);with_world(&mut w,|a|with_world(&mut fresh,|b|{
   assert_eq!(a.broad().record(),b.broad().record());assert_eq!(join_world_boundary(a.broad(),b.pipeline()).err(),Some(Error::Identity));
  }));
 }
 };
}
