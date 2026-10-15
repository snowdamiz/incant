//! Bounded logical pipeline records. These contain data keys, never shader or
//! native cache bytes. Admission compares actual host-loaded scene identities.
mod manifest;
mod recording;
mod selection;
pub use manifest::{
    AdmittedMaterials, MAX_MANIFEST_BYTES, MAX_MODEL_RECEIPTS, MaterialIdentity, MaterialManifest,
    MaterialProfile, MaterialProgram, Observation, PrewarmError, RecordingProvenance,
};
pub use recording::{MaterialRecorder, RecordingContext};
include!(concat!(env!("OUT_DIR"), "/material_catalog.rs"));
