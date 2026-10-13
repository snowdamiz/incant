//! Completed native CCD caches, not serde's empty CCD object or live restoration.
use incant_physics_snapshot::*;
#[path = "support/body_capture.rs"]
mod body_capture;
#[path = "support/ccd_capture.rs"]
mod ccd_capture;
#[path = "support/collider_capture.rs"]
mod collider_capture;
#[path = "support/integration_fixture.rs"]
mod fixture;
#[path = "support/ordered_capture.rs"]
mod ordered_capture;
fn budget() -> Budget {
    Budget::new(MAX_ALLOCATION).unwrap()
}
macro_rules! native {
 ($name:ident,$rapier:ident,$d:expr,$vector:expr,$bits:expr)=>{mod $name {
 use super::*;use $rapier::prelude::*;use $rapier::parry::bounding_volume::BoundingVolume;
 fn vector(x:f32,y:f32)->$rapier::math::Vector {($vector)(x,y)}
 fn bits(v:$rapier::math::Vector)->[u32;3] {($bits)(v)}
 ccd_capture::ccd_observer!($d);
 fn world(n:usize)->PhysicsWorld {
  let mut w=PhysicsWorld::default();w.gravity=vector(0.,0.);
  for i in 0..n {w.insert_collider(ColliderBuilder::ball(0.2).translation(vector(i as f32*3.,0.)),None);}
  w.insert(RigidBodyBuilder::dynamic().translation(vector(-20.,10.)).linvel(vector(60.,0.)),ColliderBuilder::ball(0.1));w
 }
 #[test] fn uninitialized_populated_empty_reused_and_invalidated_are_distinct() {
  let mut w=world(0);w.integration_parameters.max_ccd_substeps=0;w.step();assert_eq!(roundtrip(&w).cache,CcdCache::Uninitialized);
  w.integration_parameters.max_ccd_substeps=1;w.step();assert!(matches!(roundtrip(&w).cache,CcdCache::Targets{ref targets,..} if targets.is_empty()));
  let mut w=world(2);w.step();let original=roundtrip(&w);assert!(matches!(original.cache,CcdCache::Targets{ref targets,..} if targets.len()==2));
  assert_eq!(serde_json::to_string(&w.ccd_solver).unwrap(),"{}");
  w.integration_parameters.max_ccd_substeps=0;w.integration_parameters.normalized_prediction_distance=0.375;w.step();assert_eq!(roundtrip(&w),original);
  let h=w.colliders.iter().next().unwrap().0;w.colliders[h].set_translation(vector(1.,0.));w.step();assert_eq!(roundtrip(&w).cache,CcdCache::Uninitialized);
  w.integration_parameters.max_ccd_substeps=1;w.step();let next=roundtrip(&w);assert!(matches!(next.cache,CcdCache::Targets{prediction,..} if prediction==0.375f32.to_bits()));
  eprintln!("CCD_NATIVE dimension={} cache_states=uninitialized+empty+populated history_retained=true scene_invalidation=true serde_omits_cache=true",$d as u8);
 }
 #[test] fn cache_key_equality_retains_negative_zero_and_ordered_membership() {
  let mut w=world(2);w.integration_parameters.normalized_prediction_distance=-0.;
  let (fixed,attached)=w.insert(RigidBodyBuilder::fixed(),ColliderBuilder::ball(0.2).translation(vector(30.,0.)).sensor(true));
  let disabled=w.insert_collider(ColliderBuilder::ball(0.2).enabled(false),None);
  w.insert(RigidBodyBuilder::kinematic_position_based(),ColliderBuilder::ball(0.2));
  w.step();let first=roundtrip(&w);let CcdCache::Targets{prediction,targets}=first.cache else {panic!("list expected")};
  assert_eq!(prediction,(-0f32).to_bits());assert_eq!(targets.len(),3);assert!(targets.iter().any(|t|t.collider==ccd_capture::handle(attached.into_raw_parts())));assert!(!targets.iter().any(|t|t.collider==ccd_capture::handle(disabled.into_raw_parts())));
  w.integration_parameters.normalized_prediction_distance=0.;w.step();assert!(matches!(roundtrip(&w).cache,CcdCache::Targets{prediction,..} if prediction==(-0f32).to_bits()));
  w.bodies[fixed].set_enabled(false);w.step();assert!(matches!(roundtrip(&w).cache,CcdCache::Targets{ref targets,..} if targets.len()==2));
 }
 #[test] fn native_list_limit_fallback_removal_and_generation_reuse() {
  let mut w=world(512);w.step();assert!(matches!(roundtrip(&w).cache,CcdCache::Targets{ref targets,..} if targets.len()==512));
  let h=w.insert_collider(ColliderBuilder::ball(0.2).translation(vector(2000.,0.)),None);w.step();assert!(matches!(roundtrip(&w).cache,CcdCache::FullBvh{..}));
  w.remove_collider(h).unwrap();w.step();assert!(matches!(roundtrip(&w).cache,CcdCache::Targets{ref targets,..} if targets.len()==512));
  let replacement=w.insert_collider(ColliderBuilder::ball(0.2).translation(vector(2001.,0.)),None);assert_ne!(h,replacement);assert_eq!(h.into_raw_parts().0,replacement.into_raw_parts().0);
  let old=w.colliders.iter().next().unwrap().0;w.remove_collider(old).unwrap();w.step();let CcdCache::Targets{targets,..}=roundtrip(&w).cache else {panic!("list expected")};assert_eq!(targets.len(),512);assert!(targets.iter().any(|t|t.collider==ccd_capture::handle(replacement.into_raw_parts())));assert!(!targets.iter().any(|t|t.collider==ccd_capture::handle(h.into_raw_parts())));
  eprintln!("CCD_NATIVE dimension={} list512=true fallback513=true removal_reuse=true",$d as u8);
 }
 #[test] fn hostile_cache_frames_join_handles_order_bounds_and_budget_before_allocation() {
  let mut w=world(2);w.step();observe(&w,|c,r|{
   let v=validate_ccd_state(r.clone(),c).unwrap();let bytes=encode_ccd_state(&v,&mut budget()).unwrap();
   for len in 0..bytes.len() {assert!(decode_ccd_state(&bytes[..len],c,&mut budget()).is_err());}
   for offset in [0,8,10,11,12] {let mut b=bytes.clone();b[offset]^=255;assert!(decode_ccd_state(&b,c,&mut budget()).is_err());}
   for (offset,word) in [(44,3),(52,513),(52,u32::MAX),(56,1)] {let mut b=bytes.clone();b[offset..offset+4].copy_from_slice(&u32::to_le_bytes(word));let mut budget=Budget::new(0).unwrap();assert!(decode_ccd_state(&b,c,&mut budget).is_err());assert_eq!(budget.remaining(),0);}
   for mutate in [
    |r:&mut CcdState|if let CcdCache::Targets{targets,..}=&mut r.cache {targets.reverse();},
    |r:&mut CcdState|if let CcdCache::Targets{targets,..}=&mut r.cache {targets[1]=targets[0];},
    |r:&mut CcdState|if let CcdCache::Targets{targets,..}=&mut r.cache {targets[0].collider.generation+=1;},
    |r:&mut CcdState|if let CcdCache::Targets{targets,..}=&mut r.cache {targets.pop();},
    |r:&mut CcdState|if let CcdCache::Targets{targets,..}=&mut r.cache {targets[0].mins[0]=f32::NAN.to_bits();},
    |r:&mut CcdState|if let CcdCache::Targets{targets,..}=&mut r.cache {targets[0].mins[0]=f32::MAX.to_bits();},
    |r:&mut CcdState|if let CcdCache::Targets{prediction,..}=&mut r.cache {*prediction=(-1f32).to_bits();},
    |r:&mut CcdState|r.cache=CcdCache::FullBvh{prediction:0},
    |r:&mut CcdState|r.identity.build[0]^=1,
   ] {let mut bad=r.clone();mutate(&mut bad);assert!(validate_ccd_state(bad,c).is_err());}
   let mut b=bytes.clone();b.push(0);assert!(matches!(decode_ccd_state(&b,c,&mut budget()),Err(Error::Length)));
   let allocation=2*size_of::<CcdTarget>();let mut budget=Budget::new(allocation*2-1).unwrap();decode_ccd_state(&bytes,c,&mut budget).unwrap();assert!(matches!(decode_ccd_state(&bytes,c,&mut budget),Err(Error::Limit)));
  });
 }
 }};
}
native!(
    planar,
    rapier2d,
    Dimension::Planar,
    rapier2d::math::Vector::new,
    |v: rapier2d::math::Vector| [v.x.to_bits(), v.y.to_bits(), 0]
);
native!(
    spatial,
    rapier3d,
    Dimension::Spatial,
    |x, y| rapier3d::math::Vector::new(x, y, 0.),
    |v: rapier3d::math::Vector| v.to_array().map(f32::to_bits)
);
