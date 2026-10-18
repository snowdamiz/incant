//! Opt-in local-pose history around the actual retained native input adapter.
//! Every accepted fixed tick enters history, independently of presentation.
//! Callers must advance the explicit continuity epoch for teleports/cuts and
//! lifecycle resets. This does not infer continuity from transform distance.
mod history;
use crate::{NativeRenderError, NativeRenderInput, SyncError, UpdateKind, VisualCaches};
use glam::DMat4;
use history::History;
use incant_render_data::SceneInput;
use incant_runtime::{SpatialView, StableId};
use thiserror::Error;

/// All hierarchy entries, including invisible ancestors, count toward the cap.
pub const MAX_POSE_NODES: usize = 65_536;
/// One history owner's allocated entry buffer. A cold topology replacement may
/// coexist with one prior owner, so its maximum combined allocation is twice
/// this bound. Existing asset and retained-input budgets are separate.
pub const MAX_POSE_BYTES: usize = 32 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum PoseError {
    #[error(transparent)]
    Native(#[from] NativeRenderError),
    #[error("pose interpolation capacity exceeded")]
    Capacity,
    #[error("pose interpolation allocation failed")]
    Allocation,
    #[error("invalid local pose, hierarchy or numeric composition")]
    Invalid,
    #[error("pose interpolation does not support this render declaration")]
    Unsupported,
    #[error("pose tick or continuity epoch moved backwards")]
    Sequence,
    #[error("same-tick pose mutation needs a new explicit continuity epoch")]
    SameTickMutation,
    #[error("presentation fraction must be finite and between zero and one")]
    Fraction,
}

#[derive(Debug, Error)]
pub enum PoseSyncError<E> {
    #[error(transparent)]
    Pose(#[from] PoseError),
    #[error("pose consumer rejected the input: {0}")]
    Consumer(E),
}

/// The image represents previous_tick + fraction fixed ticks. Ordinary values
/// in [0,1) intentionally add one fixed step of visual latency. One is reserved
/// for an explicit latest-pose flush. No simulation or audio clock is changed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PoseReceipt {
    pub previous_tick: u64,
    pub current_tick: u64,
    pub continuity_epoch: u64,
    pub fraction: f64,
}

/// Owns canonical committed input and a bounded numeric history. It exposes no
/// mutable adapter/world escape. A presentation callback borrows a temporary
/// pose overlay; the exact committed matrices are restored on success, refusal
/// and unwind. Resource identities and screen-space UI are never interpolated.
///
/// This adapter currently supports static meshes, ordinary cameras and lights.
/// Animation, sprites, tilemaps, world text and pixel-perfect cameras refuse.
/// Its caller must separately qualify any temporal/motion-vector GPU consumer.
pub struct PoseRenderInput {
    input: NativeRenderInput,
    history: History,
}

impl PoseRenderInput {
    pub fn load(
        view: &SpatialView<'_>,
        caches: VisualCaches<'_>,
        camera: StableId,
        continuity_epoch: u64,
    ) -> Result<Self, PoseError> {
        // Bounds and unsupported declarations are checked before loading assets.
        let history = History::load(view, continuity_epoch)?;
        let input = NativeRenderInput::load(view, caches, camera)?;
        Ok(Self { input, history })
    }

    pub fn committed(&self) -> &NativeRenderInput {
        &self.input
    }

    /// Ensure a composed host is presenting its exact accepted native owner.
    /// This also catches trusted same-tick numeric edits without a reset.
    pub fn validate_committed(&self, view: &SpatialView<'_>) -> Result<(), PoseError> {
        if view.identity() != &self.input.identity {
            return Err(NativeRenderError::DifferentWorld.into());
        }
        if view.tick() != self.input.tick
            || view.topology_generation() != self.input.topology_generation
            || view.binding_generation() != self.input.binding_generation
        {
            return Err(PoseError::Sequence);
        }
        self.history
            .validate(view, self.history.receipt(1.).continuity_epoch)
    }

    /// Publish history only after canonical input and its consumer both accept.
    /// Same-layout motion reuses every buffer. Gaps, explicit epoch changes and
    /// structural/binding changes snap to the new accepted state. A failed or
    /// abandoned candidate cannot reset or advance accepted history.
    pub fn try_sync<T, E>(
        &mut self,
        view: &SpatialView<'_>,
        continuity_epoch: u64,
        accept: impl FnOnce(&SceneInput, &str, UpdateKind) -> Result<T, E>,
    ) -> Result<T, PoseSyncError<E>> {
        if view.identity() != &self.input.identity {
            return Err(PoseError::Native(NativeRenderError::DifferentWorld).into());
        }
        let changed = self.input.topology_generation != view.topology_generation()
            || self.input.binding_generation != view.binding_generation();
        self.history.check_sequence(view.tick(), continuity_epoch)?;
        let replacement = if changed {
            Some(History::load(view, continuity_epoch)?)
        } else {
            self.history.validate(view, continuity_epoch)?;
            None
        };
        let result = self
            .input
            .try_sync(view, accept)
            .map_err(|error| match error {
                SyncError::Native(error) => PoseSyncError::Pose(PoseError::Native(error)),
                SyncError::Consumer(error) => PoseSyncError::Consumer(error),
            })?;
        if let Some(next) = replacement {
            self.history = next;
        } else {
            self.history.publish(view, continuity_epoch);
        }
        Ok(result)
    }

    /// Present without accepting another tick. All camera/light/model worlds
    /// derive from the same parent-first local-TRS interpolation. Endpoints use
    /// original committed globals; no global matrix decomposition is involved.
    pub fn present<T, E>(
        &mut self,
        fraction: f64,
        accept: impl FnOnce(&SceneInput, &str, PoseReceipt) -> Result<T, E>,
    ) -> Result<T, PoseSyncError<E>> {
        self.history.compose(fraction)?;
        let receipt = self.history.receipt(fraction);
        let input = &mut self.input;
        // The history validates all resulting matrices before any overlay write.
        let guard = Overlay {
            input,
            history: &self.history,
        };
        for (index, slot) in guard.input.slots.iter().enumerate() {
            if let Some(slot) = slot {
                guard.input.input.nodes[*slot].world = guard.history.global(index);
            }
        }
        accept(guard.input.input(), guard.input.camera_label(), receipt)
            .map_err(PoseSyncError::Consumer)
    }
}

struct Overlay<'a> {
    input: &'a mut NativeRenderInput,
    history: &'a History,
}
impl Drop for Overlay<'_> {
    fn drop(&mut self) {
        for (index, slot) in self.input.slots.iter().enumerate() {
            if let Some(slot) = slot {
                self.input.input.nodes[*slot].world = self.history.committed_global(index);
            }
        }
    }
}
