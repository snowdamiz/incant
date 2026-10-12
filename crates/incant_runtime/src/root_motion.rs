//! Root-only retained declarations; no full model or importer dependency.
pub(crate) mod binary;
mod interval;
pub(crate) mod policy;
mod snapshot;
pub(crate) mod state;
pub(crate) mod step;
#[cfg(test)]
mod tests;
use crate::{CookedSpatial, NativeAnimation, SceneError};
use incant_anim::{MAX_ROOT_MOTION_INSTANCES, MAX_SCENE_ROOT_MOTION_BYTES, RootMotionCatalog};
use incant_types::RootMotionApplication;
pub use snapshot::{RootMotionClockSnapshot, RootMotionSnapshot};
pub(crate) use state::{PreparedRootMotion, RootMotionState};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootMotionControl {
    /// Enabling/disabling application never seeks or applies stored deltas.
    SetApplying(bool),
    /// Reset observations at this transaction boundary without changing clocks.
    ResetAccumulated,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NativeRootMotion {
    pub catalog: Arc<RootMotionCatalog>,
    pub application: RootMotionApplication,
    pub applying: bool,
}
impl NativeRootMotion {
    pub fn validate(&self, animation: &NativeAnimation) -> Result<(), SceneError> {
        if self.catalog.fingerprint() != animation.mesh_fingerprint
            || self.catalog.clips().len() != animation.clip_durations.len()
            || self
                .catalog
                .clips()
                .zip(animation.clip_durations.iter())
                .any(|(clip, duration)| clip.duration().to_bits() != duration.to_bits())
        {
            return Err(SceneError::Invalid("root-motion animation binding"));
        }
        Ok(())
    }
}

/// Cold deduplication/capacity admission. Equal identities may share one Arc;
/// caller-invented conflicting payloads with the same identity refuse.
#[derive(Default)]
pub(crate) struct CatalogBudget<'a> {
    catalogs: BTreeMap<([u8; 32], u16), &'a Arc<RootMotionCatalog>>,
    bytes: usize,
    bindings: usize,
}
impl<'a> CatalogBudget<'a> {
    pub fn admit(&mut self, node: &'a CookedSpatial) -> Result<(), SceneError> {
        let Some(root) = &node.root_motion else {
            return Ok(());
        };
        root.validate(
            node.animation
                .as_deref()
                .ok_or(SceneError::Invalid("root motion requires animation"))?,
        )?;
        self.bindings += 1;
        if self.bindings > MAX_ROOT_MOTION_INSTANCES {
            return Err(SceneError::Invalid("root-motion instance count"));
        }
        let key = (root.catalog.fingerprint(), root.catalog.root());
        if let Some(old) = self.catalogs.insert(key, &root.catalog) {
            if !Arc::ptr_eq(old, &root.catalog) && old != &root.catalog {
                return Err(SceneError::Invalid(
                    "conflicting root-motion catalog identity",
                ));
            }
        } else {
            self.bytes = self
                .bytes
                .checked_add(root.catalog.retained_bytes())
                .filter(|n| *n <= MAX_SCENE_ROOT_MOTION_BYTES)
                .ok_or(SceneError::Invalid("root-motion retained scene capacity"))?;
        }
        Ok(())
    }
}
