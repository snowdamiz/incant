//! Handoff 0070 helper: bounded numerical and appearance frames for the opt-in
//! SSAO and bloom union. Public engine APIs only; nothing here is engine code,
//! enables an effect by default, waits on a timer or changes shader math.
pub mod appearance;
pub mod common;
pub mod composed;
pub mod gltf;
pub mod numeric;
