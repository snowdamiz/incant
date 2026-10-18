use super::{DMat4, MAX_POSE_BYTES, MAX_POSE_NODES, PoseError, PoseReceipt};
use glam::{DQuat, DVec3};
use incant_runtime::{SpatialNode, SpatialView, StableId};
use incant_types::{CameraProjection, Transform};

#[derive(Clone, Copy, PartialEq)]
struct Pose {
    translation: DVec3,
    rotation: DQuat,
    scale: DVec3,
}
impl Pose {
    fn read(transform: &Transform) -> Result<Self, PoseError> {
        let result = Self {
            translation: DVec3::from_array(transform.translation),
            rotation: DQuat::from_array(transform.rotation),
            scale: DVec3::from_array(transform.scale),
        };
        if !result.translation.is_finite()
            || !result.rotation.is_finite()
            || !result.scale.is_finite()
            || (result.rotation.length_squared() - 1.).abs() > 1e-4
            || result.scale.abs().cmplt(DVec3::splat(1e-9)).any()
        {
            return Err(PoseError::Invalid);
        }
        Ok(result)
    }
    fn compatible(self, next: Self) -> bool {
        // Never cross a singular scale or a mirrored-orientation transition.
        (self.scale * next.scale).cmpgt(DVec3::ZERO).all()
    }
    fn interpolate(self, next: Self, alpha: f64) -> DMat4 {
        DMat4::from_scale_rotation_translation(
            self.scale.lerp(next.scale, alpha),
            self.rotation
                .normalize()
                .slerp(next.rotation.normalize(), alpha)
                .normalize(),
            self.translation.lerp(next.translation, alpha),
        )
    }
}

struct Entry {
    id: StableId,
    parent: Option<usize>,
    previous: Pose,
    current: Pose,
    previous_global: DMat4,
    current_global: DMat4,
    blended: DMat4,
    snapped: bool,
}

pub(super) struct History {
    entries: Vec<Entry>,
    previous_tick: u64,
    tick: u64,
    epoch: u64,
}

fn supported(node: &SpatialNode<'_>) -> Result<(), PoseError> {
    if let Some(d) = node.declaration
        && (d.animation.is_some()
            || d.ik.is_some()
            || d.root_motion.is_some()
            || d.sprite.is_some()
            || d.flipbook.is_some()
            || d.tilemap.is_some()
            || d.camera
                .as_ref()
                .is_some_and(|c| matches!(c.projection, CameraProjection::PixelPerfect { .. })))
    {
        return Err(PoseError::Unsupported);
    }
    if node.parent_index.is_some_and(|p| p >= node.index) || !node.global.is_finite() {
        return Err(PoseError::Invalid);
    }
    Ok(())
}

impl History {
    pub(super) fn load(view: &SpatialView<'_>, epoch: u64) -> Result<Self, PoseError> {
        let count = view.nodes().len();
        let bytes = count
            .checked_mul(size_of::<Entry>())
            .ok_or(PoseError::Capacity)?;
        if count > MAX_POSE_NODES || bytes > MAX_POSE_BYTES {
            return Err(PoseError::Capacity);
        }
        if view.has_text() {
            return Err(PoseError::Unsupported);
        }
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(count)
            .map_err(|_| PoseError::Allocation)?;
        if entries
            .capacity()
            .checked_mul(size_of::<Entry>())
            .is_none_or(|n| n > MAX_POSE_BYTES)
        {
            return Err(PoseError::Capacity);
        }
        for node in view.nodes() {
            supported(&node)?;
            let pose = Pose::read(node.local)?;
            entries.push(Entry {
                id: node.id,
                parent: node.parent_index,
                previous: pose,
                current: pose,
                previous_global: *node.global,
                current_global: *node.global,
                blended: *node.global,
                snapped: true,
            });
        }
        Ok(Self {
            entries,
            previous_tick: view.tick(),
            tick: view.tick(),
            epoch,
        })
    }
    pub(super) fn check_sequence(&self, tick: u64, epoch: u64) -> Result<(), PoseError> {
        if tick < self.tick || epoch < self.epoch {
            return Err(PoseError::Sequence);
        }
        Ok(())
    }
    pub(super) fn validate(&self, view: &SpatialView<'_>, epoch: u64) -> Result<(), PoseError> {
        if self.entries.len() != view.nodes().len() || view.has_text() {
            return Err(PoseError::Invalid);
        }
        for (node, entry) in view.nodes().zip(&self.entries) {
            supported(&node)?;
            let pose = Pose::read(node.local)?;
            if node.id != entry.id || node.parent_index != entry.parent {
                return Err(PoseError::Invalid);
            }
            if view.tick() == self.tick
                && epoch == self.epoch
                && (pose != entry.current || *node.global != entry.current_global)
            {
                return Err(PoseError::SameTickMutation);
            }
        }
        Ok(())
    }
    pub(super) fn publish(&mut self, view: &SpatialView<'_>, epoch: u64) {
        if view.tick() == self.tick && epoch == self.epoch {
            return;
        }
        let continuous = epoch == self.epoch && self.tick.checked_add(1) == Some(view.tick());
        self.previous_tick = if continuous { self.tick } else { view.tick() };
        for (node, entry) in view.nodes().zip(&mut self.entries) {
            // Exact immutable view already checked before the canonical consumer.
            let pose = Pose::read(node.local).expect("preflighted immutable local pose");
            entry.previous = if continuous { entry.current } else { pose };
            entry.previous_global = if continuous {
                entry.current_global
            } else {
                *node.global
            };
            entry.current = pose;
            entry.current_global = *node.global;
        }
        self.tick = view.tick();
        self.epoch = epoch;
    }
    pub(super) fn compose(&mut self, fraction: f64) -> Result<(), PoseError> {
        if !fraction.is_finite() || !(0. ..=1.).contains(&fraction) {
            return Err(PoseError::Fraction);
        }
        for index in 0..self.entries.len() {
            let parent = self.entries[index].parent;
            let parent_snapped = parent.is_some_and(|p| self.entries[p].snapped);
            let parent_global = parent.map(|p| self.entries[p].blended);
            let entry = &mut self.entries[index];
            entry.snapped = parent_snapped || !entry.previous.compatible(entry.current);
            entry.blended = if fraction == 1. || entry.snapped {
                entry.current_global
            } else if fraction == 0. {
                entry.previous_global
            } else {
                let local = entry.previous.interpolate(entry.current, fraction);
                parent_global.map_or(local, |p| p * local)
            };
            if !entry.blended.is_finite() {
                return Err(PoseError::Invalid);
            }
        }
        Ok(())
    }
    pub(super) fn receipt(&self, fraction: f64) -> PoseReceipt {
        PoseReceipt {
            previous_tick: self.previous_tick,
            current_tick: self.tick,
            continuity_epoch: self.epoch,
            fraction,
        }
    }
    pub(super) fn global(&self, index: usize) -> DMat4 {
        self.entries[index].blended
    }
    pub(super) fn committed_global(&self, index: usize) -> DMat4 {
        self.entries[index].current_global
    }
}
