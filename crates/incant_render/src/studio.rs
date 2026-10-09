//! Preview lighting only. Authored lights/render graph and production look-dev
//! remain separate work. Initial direction/weights inherit the diagnostic pass;
//! Claude owns appearance tuning and approval of this preview.
//!
//! Tuning (handoff 0014): the shader divides direct diffuse by pi, so a key
//! radiance of 2.0 puts a face-on white dielectric near 0.8 linear without
//! clipping, while the 0.3 fill keeps unlit sides near a 3:1 key/fill ratio.
//! There is no specular environment yet, so metals stay dark outside their
//! direct highlight; an output transform cannot conceal that.
//!
//! HDR output (handoff 0015, pending integration and capture review): the
//! scene renders to linear HDR and `tone_map.wgsl` applies the Khronos PBR
//! Neutral curve at fixed exposure 1.0. These values are kept unchanged for
//! the first pass. Ordinary values in [0.08, 0.8] lose only a constant 0.04,
//! a camera-facing white dielectric (about 0.79 linear) maps to about 0.75,
//! a fully lit white face (about 0.91) maps to about 0.84 on the shoulder, and
//! glossy highlights roll off on a hue-preserving shoulder instead of
//! clipping. The constant offset costs dark fill-lit surfaces the most, so
//! dark albedos may crush; radiance and fill may be retuned only after
//! reviewing captures.
pub const EYE: [f32; 3] = [6., 5., 9.];
pub const LIGHT_DIRECTION: [f32; 3] = [1., 2., 3.];
pub const LIGHT_RADIANCE: [f32; 3] = [2.0; 3];
pub const DIFFUSE_ENVIRONMENT: [f32; 3] = [0.3; 3];
