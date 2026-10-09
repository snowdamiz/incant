//! Preview lighting only. Authored lights/render graph and production look-dev
//! remain separate work. Initial direction/weights inherit the diagnostic pass;
//! Claude owns appearance tuning and approval of this preview.
//!
//! Direct key (handoff 0014): the shader divides direct diffuse by pi, so a key
//! radiance of 2.0 puts a face-on white dielectric near 0.8 linear without
//! clipping. It stays a separate analytic light and is not part of the
//! environment. The key softbox in preview_environment.rs is aligned with
//! LIGHT_DIRECTION on purpose, so reflected and direct highlights coincide.
//! The light casts no shadows; nothing in the preview is occluded.
//!
//! Environment (handoff 0016): the legacy constant diffuse fill is gone.
//! Indirect light now comes from the procedural studio in
//! preview_environment.rs, sampled to a cubemap, diffuse-convolved and
//! GGX-prefiltered. Its cosine-weighted mean radiance averages about 0.30, so
//! unlit sides keep roughly the old readability, with tops brighter (about
//! 0.42) and undersides darker (about 0.15). Metals now reflect a neutral cove
//! with key, fill and rim panels instead of staying dark outside the direct
//! highlight. The environment's floor is distant lighting only, not geometry.
//!
//! HDR output (handoff 0015): the scene renders to linear HDR and
//! `tone_map.wgsl` applies an Incant preview curve derived from the Khronos
//! PBR Neutral shoulder, at fixed exposure 1.0. Colors whose brightest channel
//! is below 0.8 pass through unchanged, so mid and dark albedos and emissive
//! colors keep their values. The key panel's reflection in bright metals lands
//! on the shoulder near white, and glossy highlights compress smoothly toward
//! white without changing hue. EYE, LIGHT_DIRECTION and LIGHT_RADIANCE are
//! unchanged by handoff 0016.
pub const EYE: [f32; 3] = [6., 5., 9.];
pub const LIGHT_DIRECTION: [f32; 3] = [1., 2., 3.];
pub const LIGHT_RADIANCE: [f32; 3] = [2.0; 3];
