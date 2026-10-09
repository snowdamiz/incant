//! Preview lighting only. Authored lights/render graph and production look-dev
//! remain separate work. Initial direction/weights inherit the diagnostic pass;
//! Claude owns appearance tuning and approval of this preview.
//!
//! The directional key remains at its approved handoff 0014 value. Handoff
//! 0016 replaces the legacy constant diffuse fill with a convolved studio
//! environment, including specular reflections. See preview_environment.rs.
//!
//! HDR output (handoff 0015): the scene renders to linear HDR and
//! `tone_map.wgsl` applies an Incant preview curve derived from the Khronos
//! PBR Neutral shoulder, at fixed exposure 1.0. Colors whose brightest channel
//! is below 0.8 pass through unchanged, so the camera-facing white face (about
//! 0.79 linear), fill-lit dark albedos and emissive colors keep their values.
//! A fully lit white face (about 0.91) rolls onto the shoulder near 0.87, and
//! glossy highlights compress smoothly toward white without changing hue.
//! These constants stay unchanged pending review of fresh captures.
pub const EYE: [f32; 3] = [6., 5., 9.];
pub const LIGHT_DIRECTION: [f32; 3] = [1., 2., 3.];
pub const LIGHT_RADIANCE: [f32; 3] = [2.0; 3];
