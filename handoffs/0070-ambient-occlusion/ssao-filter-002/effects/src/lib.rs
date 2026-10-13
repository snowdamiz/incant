//! Handoff 0070 filter execution namespace 0070-ssao-filter-002: bounded
//! numerical frames, R8 stage planes and appearance frames for the opt-in
//! R1/R3 SSAO candidate. Public engine APIs plus the non-default diagnostic
//! `ssao-stage-readback` capture only; nothing here is engine code, enables an
//! effect by default, waits on a timer or changes shader math.
pub mod appearance;
pub mod common;
pub mod composed;
pub mod gltf;
pub mod numeric;
