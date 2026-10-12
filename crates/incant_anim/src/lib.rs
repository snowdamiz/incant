//! Checked numerical animation data. No model importer, filesystem, renderer,
//! document, script host or mutable gameplay world belongs to this crate.
mod binary;
mod catalog;
mod root_motion;
pub mod track;
pub use binary::RootMotionBinaryError;

pub use catalog::{
    MAX_ROOT_MOTION_CATALOG_BYTES, MAX_ROOT_MOTION_CLIPS, MAX_ROOT_MOTION_INSTANCES,
    MAX_ROOT_MOTION_KEYS, MAX_SCENE_ROOT_MOTION_BYTES, RootMotionCatalog, RootMotionClip,
    RootMotionTrack,
};
pub use root_motion::{
    MAX_ROOT_MOTION_CYCLES, MAX_ROOT_MOTION_DURATION, RootMotionDelta, RootMotionError,
    RootMotionSampler, RootMotionTime,
};
