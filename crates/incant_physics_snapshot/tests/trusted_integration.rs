//! Actual pinned native public settings and completed-step observations.
use incant_physics_snapshot::*;
#[path = "support/integration_fixture.rs"]
mod fixture;
fn budget() -> Budget {
    Budget::new(MAX_ALLOCATION).unwrap()
}
macro_rules! native {
 ($name:ident,$rapier:ident,$d:expr,$vector:expr,$friction:expr)=>{mod $name {
 use super::*; use $rapier::prelude::*;
 fn vector(x:f32,y:f32)->$rapier::math::Vector {($vector)(x,y)}
 fn capture(p:&IntegrationParameters)->IntegrationConfig {
  // Fail on any newly added/removed serialized field, rather than forgetting it.
  let value=serde_json::to_value(p).unwrap();let mut fields=value.as_object().unwrap().keys().map(String::as_str).collect::<Vec<_>>();fields.sort();
  let mut expected=vec!["dt","min_ccd_dt","contact_softness","static_contact_softness","warmstart_coefficient","length_unit","soft_bodies","normalized_allowed_linear_error","normalized_max_corrective_velocity","normalized_prediction_distance","normalized_max_linear_velocity","num_solver_iterations","num_internal_pgs_iterations","num_internal_stabilization_iterations","max_ccd_substeps","contact_clustering","contact_recycling","normalized_contact_recycle_distance","friction_in_bias_pass","warmstart_joints"];
  if $d==Dimension::Spatial {expected.push("friction_model");}expected.sort();assert_eq!(fields,expected);
  let soft_bodies=if serde_json::to_vec(&p.soft_bodies).unwrap()==serde_json::to_vec(&$rapier::dynamics::SoftBodiesSettings::default()).unwrap() {IntegrationSoftBodies::PinnedDefaults} else {IntegrationSoftBodies::Unsupported};
  IntegrationConfig {identity:fixture::identity($d),dt:p.dt.to_bits(),min_ccd_dt:p.min_ccd_dt.to_bits(),
   contact_softness:ContactSoftness{natural_frequency:p.contact_softness.natural_frequency.to_bits(),damping_ratio:p.contact_softness.damping_ratio.to_bits()},
   static_contact_softness:ContactSoftness{natural_frequency:p.static_contact_softness.natural_frequency.to_bits(),damping_ratio:p.static_contact_softness.damping_ratio.to_bits()},
   warmstart_coefficient:p.warmstart_coefficient.to_bits(),length_unit:p.length_unit.to_bits(),
   normalized_allowed_linear_error:p.normalized_allowed_linear_error.to_bits(),normalized_max_corrective_velocity:p.normalized_max_corrective_velocity.to_bits(),
   normalized_prediction_distance:p.normalized_prediction_distance.to_bits(),normalized_max_linear_velocity:p.normalized_max_linear_velocity.to_bits(),normalized_contact_recycle_distance:p.normalized_contact_recycle_distance.to_bits(),
   num_solver_iterations:p.num_solver_iterations as u64,num_internal_pgs_iterations:p.num_internal_pgs_iterations as u64,num_internal_stabilization_iterations:p.num_internal_stabilization_iterations as u64,max_ccd_substeps:p.max_ccd_substeps as u64,
   contact_clustering:p.contact_clustering,contact_recycling:p.contact_recycling,friction_in_bias_pass:p.friction_in_bias_pass,warmstart_joints:p.warmstart_joints,
   friction_model:($friction)(p),soft_bodies}
 }
 fn roundtrip(p:&IntegrationParameters) {
  let r=capture(p);let v=validate_integration_config(r).unwrap();
  assert_eq!(v.prediction_distance(),p.prediction_distance().to_bits());
  let b=encode_integration_config(&v,&mut budget()).unwrap();assert_eq!(decode_integration_config(&b,r.identity).unwrap().record(),&r);
 }
 #[test] fn exact_settings_sentinels_and_nondefault_soft_refusal() {
  let mut p=IntegrationParameters::default();assert_eq!(capture(&p),fixture::config($d));
  p.length_unit=2.;p.normalized_max_linear_velocity=f32::MAX;p.normalized_max_corrective_velocity=f32::MAX;
  p.normalized_prediction_distance=-0.;p.min_ccd_dt=-0.;p.warmstart_coefficient=-0.;p.num_internal_pgs_iterations=0;p.num_internal_stabilization_iterations=0;p.max_ccd_substeps=0;
  p.contact_clustering=false;p.contact_recycling=false;p.friction_in_bias_pass=true;p.warmstart_joints=true;
  p.contact_softness.damping_ratio=0.;p.static_contact_softness.natural_frequency=0.;
  roundtrip(&p);assert_eq!(p.max_linear_velocity(),f32::MAX);assert_eq!(p.max_corrective_velocity(),f32::MAX);
  assert!(p.contact_softness.cfm_coeff(p.dt).is_finite());assert_eq!(p.static_contact_softness.cfm_coeff(p.dt),0.);
  p.soft_bodies.recovery.overlap_kept_depth=-0.;
  assert_eq!(p.soft_bodies,$rapier::dynamics::SoftBodiesSettings::default());
  assert!(matches!(validate_integration_config(capture(&p)),Err(Error::UnsupportedIntegration)));
  p.soft_bodies.max_extra_substeps=0;
  assert!(matches!(validate_integration_config(capture(&p)),Err(Error::UnsupportedIntegration)));
  eprintln!("INTEGRATION_NATIVE dimension={} exact_fields=true max_sentinels=true signed_zero=true soft_bits_refused=true",$d as u8);
 }
 #[test] fn nonpositive_dt_is_actual_collision_only_with_zero_solver_count() {
  for dt in [0.,-0.,-0.01] {
   let mut w=PhysicsWorld::default();w.gravity=vector(0.,0.);w.integration_parameters.dt=dt;w.integration_parameters.num_solver_iterations=0;
   let (b,a)=w.insert(RigidBodyBuilder::dynamic().linvel(vector(2.,0.)),ColliderBuilder::ball(0.5));
   let other=w.insert_collider(ColliderBuilder::ball(0.5).translation(vector(0.75,0.)),None);
   let position=*w.bodies[b].position();let velocity=w.bodies[b].linvel();let before=capture(&w.integration_parameters);
   w.step();assert_eq!(*w.bodies[b].position(),position);assert_eq!(w.bodies[b].linvel(),velocity);
   assert!(w.narrow_phase.contact_pair(a,other).is_some());assert_eq!(capture(&w.integration_parameters),before);roundtrip(&w.integration_parameters);
  }
 }
 #[test] fn native_zero_base_with_body_extras_is_explicitly_outside_scalar_domain() {
  let mut w=PhysicsWorld::default();w.integration_parameters.num_solver_iterations=0;
  let (body,co)=w.insert(RigidBodyBuilder::dynamic().additional_solver_iterations(2).linvel(vector(6.,0.)),ColliderBuilder::ball(0.5));
  let ground=w.insert_collider(ColliderBuilder::ball(0.5).translation(vector(0.,-0.9)),None);
  w.step();assert!(w.bodies[body].translation().is_finite());assert!(w.bodies[body].translation().x>0.09);
  assert!(w.narrow_phase.contact_pair(co,ground).is_some());assert_eq!(w.integration_parameters.num_solver_iterations,0);
  assert!(matches!(validate_integration_config(capture(&w.integration_parameters)),Err(Error::Integration)));
  eprintln!("INTEGRATION_NATIVE dimension={} native_zero_base_extra2=true stricter_scalar_refusal=true",$d as u8);
 }
 #[test] fn zero_optional_passes_and_ccd_disabled_are_native_controls() {
  let mut w=PhysicsWorld::default();w.gravity=vector(0.,0.);w.integration_parameters.num_internal_pgs_iterations=0;w.integration_parameters.num_internal_stabilization_iterations=0;w.integration_parameters.max_ccd_substeps=0;
  let (b,_)=w.insert(RigidBodyBuilder::dynamic().linvel(vector(6.,0.)),ColliderBuilder::ball(0.5));
  let before=capture(&w.integration_parameters);w.step();assert!(w.bodies[b].translation().x>0.09);
  assert_eq!(capture(&w.integration_parameters),before);roundtrip(&w.integration_parameters);assert!(w.ccd_solver.checkpoint_fixed_targets().is_none());
 }
 }};
}
native!(
    planar,
    rapier2d,
    Dimension::Planar,
    rapier2d::math::Vector::new,
    |_: &rapier2d::dynamics::IntegrationParameters| IntegrationFriction::Planar
);
native!(
    spatial,
    rapier3d,
    Dimension::Spatial,
    |x, y| rapier3d::math::Vector::new(x, y, 0.),
    |p: &rapier3d::dynamics::IntegrationParameters| match p.friction_model {
        rapier3d::dynamics::FrictionModel::Simplified => IntegrationFriction::Simplified,
        rapier3d::dynamics::FrictionModel::Coulomb => IntegrationFriction::Coulomb,
    }
);
