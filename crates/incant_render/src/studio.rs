//! Preview lighting only. Authored lights/render graph and production look-dev
//! remain separate work. Initial direction/weights inherit the diagnostic pass;
//! Claude owns appearance tuning and approval of this preview.
//!
//! Tuning (handoff 0014): the shader divides direct diffuse by pi, so a key
//! radiance of 2.0 puts a face-on white dielectric near 0.8 linear without
//! clipping, while the 0.3 fill keeps unlit sides near a 3:1 key/fill ratio.
//! There is no specular environment or tonemapping: metals stay dark outside
//! their direct highlight, and glossy highlights can clip to white.
pub const EYE: [f32; 3] = [6., 5., 9.];
pub const LIGHT_DIRECTION: [f32; 3] = [1., 2., 3.];
pub const LIGHT_RADIANCE: [f32; 3] = [2.0; 3];
pub const DIFFUSE_ENVIRONMENT: [f32; 3] = [0.3; 3];
