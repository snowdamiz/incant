//! Preview lighting only. Authored lights/render graph and production look-dev
//! remain separate work. Initial direction/weights inherit the diagnostic pass;
//! Claude owns appearance tuning and approval of this preview.
pub const EYE: [f32; 3] = [6., 5., 9.];
pub const LIGHT_DIRECTION: [f32; 3] = [1., 2., 3.];
pub const LIGHT_RADIANCE: [f32; 3] = [0.65; 3];
pub const DIFFUSE_ENVIRONMENT: [f32; 3] = [0.25; 3];
