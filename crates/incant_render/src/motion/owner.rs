use super::MotionError;
use crate::camera::CameraView;
use glam::Mat4;
#[cfg(test)]
use glam::{Vec2, Vec4};
use std::sync::Arc;

#[derive(Clone, PartialEq, Eq)]
pub(super) struct Extent {
    output: [u32; 2],
    internal: [u32; 2],
    rect: [u32; 4],
    projection: [u32; 5],
    pixel: Option<[u32; 2]>,
    output_rect: Option<[u32; 4]>,
}
impl Extent {
    pub fn new(output: [u32; 2], internal: [u32; 2], rect: [f32; 4], camera: CameraView) -> Self {
        Self {
            output,
            internal,
            output_rect: None,
            rect: rect.map(f32::to_bits),
            pixel: camera.pixel_resolution,
            projection: [
                camera.near.to_bits(),
                camera.far.to_bits(),
                camera.fov.to_bits(),
                camera.orthographic_half_height.map_or(0, f32::to_bits),
                u32::from(camera.orthographic_half_height.is_some()),
            ],
        }
    }
    pub fn upscaled(mut self, rect: [f32; 4]) -> Self {
        self.output_rect = Some(rect.map(f32::to_bits));
        self
    }
    fn compatible(&self, other: &Self) -> bool {
        if self.output_rect.is_some() && other.output_rect.is_some() {
            self.output == other.output
                && self.output_rect == other.output_rect
                && self.projection == other.projection
                && self.pixel == other.pixel
        } else {
            self == other
        }
    }
}
/// Read-only submitted-frame identity. A replacement has a distinct owner even
/// with identical entity IDs, camera and simulation tick. No texture view escapes.
#[derive(Clone, Debug)]
pub struct MotionFrame {
    owner: Arc<()>,
    sequence: u64,
    epoch: u64,
    size: [u32; 2],
    valid_previous: bool,
    bypassed: bool,
    phase: Option<u32>,
    camera_static: bool,
    /// Output-upscaled history: jitter phases index `UPSCALE_SAMPLES`.
    upscaled: bool,
}
impl MotionFrame {
    /// Exact camera, shading-eye and source/output framing equality against the
    /// last accepted temporal frame. False on reset, bypass and DRS transitions.
    /// This is not a proof that lights, geometry or renderer effects are unchanged.
    pub fn camera_static(&self) -> bool {
        self.camera_static
    }
    /// Present only for temporal accumulation; phase is internal-pixel jitter.
    /// Internal-pixel jitter of this frame's phase in its sequence.
    pub(crate) fn jitter_offset(&self) -> Option<[f32; 2]> {
        self.phase
            .map(|p| crate::temporal::jitter::offset(p, self.upscaled))
    }
    pub fn jitter_phase(&self) -> Option<u32> {
        self.phase
    }
    pub fn sequence(&self) -> u64 {
        self.sequence
    }
    pub fn epoch(&self) -> u64 {
        self.epoch
    }
    pub fn internal_size(&self) -> [u32; 2] {
        self.size
    }
    pub fn has_previous(&self) -> bool {
        self.valid_previous
    }
    pub fn bypassed(&self) -> bool {
        self.bypassed
    }
    pub fn same_owner(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner)
    }
}
pub(super) struct History {
    owner: Arc<()>,
    last: Option<Accepted>,
    cut: bool,
}
struct Accepted {
    frame: MotionFrame,
    extent: Extent,
    matrix: Mat4,
    camera: Option<CameraSignature>,
}
#[derive(Clone, PartialEq, Eq)]
struct CameraSignature {
    unjittered: [u32; 16],
    view: [u32; 16],
    eye: [u32; 3],
    forward: [u32; 3],
    jitter_guard: [u32; 2],
    shading_eye: [u32; 4],
}
pub(super) struct Pending {
    pub frame: MotionFrame,
    pub matrix: Mat4,
    pub previous: Mat4,
    pub previous_jitter_uv: [f32; 2],
    extent: Extent,
    camera: Option<CameraSignature>,
}
impl Default for History {
    fn default() -> Self {
        Self {
            owner: Arc::new(()),
            last: None,
            cut: false,
        }
    }
}
impl History {
    pub fn invalidate(&mut self) {
        self.cut = true;
    }
    pub fn frame(&self) -> Option<MotionFrame> {
        self.last.as_ref().map(|a| a.frame.clone())
    }
    pub fn prepare(&self, extent: Extent, matrix: Mat4) -> Result<Pending, MotionError> {
        if !matrix.is_finite() || !matrix.inverse().is_finite() {
            return Err(MotionError::Projection);
        }
        let reset = self.cut
            || self
                .last
                .as_ref()
                .is_none_or(|a| !a.extent.compatible(&extent) || a.frame.bypassed);
        let bypassed = extent.pixel.is_some();
        let sequence = self
            .last
            .as_ref()
            .map_or(0, |a| a.frame.sequence)
            .checked_add(1)
            .ok_or(MotionError::Sequence)?;
        let epoch = self
            .last
            .as_ref()
            .map_or(0, |a| a.frame.epoch)
            .checked_add(u64::from(reset || bypassed))
            .ok_or(MotionError::Sequence)?;
        let valid_previous = self.last.is_some() && !reset && !bypassed;
        Ok(Pending {
            frame: MotionFrame {
                owner: Arc::clone(&self.owner),
                sequence,
                epoch,
                size: extent.internal,
                valid_previous,
                bypassed,
                phase: None,
                camera_static: false,
                upscaled: extent.output_rect.is_some(),
            },
            matrix,
            previous_jitter_uv: [0.; 2],
            previous: if valid_previous {
                self.last.as_ref().unwrap().matrix
            } else {
                matrix
            },
            extent,
            camera: None,
        })
    }
    pub fn prepare_temporal(
        &self,
        extent: Extent,
        camera: CameraView,
        rect: [f32; 4],
    ) -> Result<Pending, MotionError> {
        let mut pending = self.prepare(
            extent,
            camera
                .matrix(rect[2] / rect[3])
                .map_err(|_| MotionError::Projection)?,
        )?;
        let signature = CameraSignature {
            unjittered: camera
                .unjittered()
                .matrix(rect[2] / rect[3])
                .map_err(|_| MotionError::Projection)?
                .to_cols_array()
                .map(f32::to_bits),
            view: camera.view.to_cols_array().map(f32::to_bits),
            eye: camera.eye.to_array().map(f32::to_bits),
            forward: camera.forward.to_array().map(f32::to_bits),
            jitter_guard: camera.jitter_guard.map(f32::to_bits),
            shading_eye: camera.shading_eye().map(f32::to_bits),
        };
        // Compatible history can span DRS changes. Unclipped persistence cannot:
        // source-sized receiver guards can change shadow cascade quantization.
        pending.frame.camera_static = pending.frame.has_previous()
            && self.last.as_ref().is_some_and(|old| {
                old.extent == pending.extent && old.camera.as_ref() == Some(&signature)
            });
        pending.camera = Some(signature);
        if pending.frame.has_previous()
            && let Some(old) = &self.last
            && let Some(phase) = old.frame.phase
        {
            let rect = old.extent.rect.map(f32::from_bits);
            let jitter = crate::temporal::jitter::offset(phase, old.extent.output_rect.is_some());
            pending.previous_jitter_uv = [jitter[0] / rect[2], jitter[1] / rect[3]];
        }
        if !pending.frame.bypassed {
            let phase = if pending.frame.has_previous() {
                (self
                    .last
                    .as_ref()
                    .and_then(|a| a.frame.phase)
                    .ok_or(MotionError::Topology)?
                    + 1)
                    % crate::temporal::jitter::phase_count(pending.frame.upscaled)
            } else {
                0
            };
            let mut camera = camera;
            camera.jitter = crate::temporal::jitter::ndc_offset(
                crate::temporal::jitter::offset(phase, pending.frame.upscaled),
                rect,
            );
            pending.matrix = camera
                .matrix(rect[2] / rect[3])
                .map_err(|_| MotionError::Projection)?;
            if !pending.matrix.inverse().is_finite() {
                return Err(MotionError::Projection);
            }
            if !pending.frame.has_previous() {
                pending.previous = pending.matrix;
            }
            pending.frame.phase = Some(phase);
        }
        Ok(pending)
    }
    /// Called only after the owning retained scene immediately submits its packet.
    pub fn accept(&mut self, pending: Pending) {
        debug_assert!(Arc::ptr_eq(&self.owner, &pending.frame.owner));
        self.last = Some(Accepted {
            frame: pending.frame,
            extent: pending.extent,
            matrix: pending.matrix,
            camera: pending.camera,
        });
        self.cut = false;
    }
}
/// Closed-form reference for the shader's opaque-surface displacement. This is
/// geometric history validity, not a disocclusion or transparent coverage test.
#[cfg(test)]
pub(super) fn reproject(current: Vec4, previous: Vec4) -> Option<[f32; 4]> {
    if !current.is_finite() || !previous.is_finite() || current.w <= 0. || previous.w <= 0. {
        return None;
    }
    let a = current.truncate() / current.w;
    let b = previous.truncate() / previous.w;
    if !a.is_finite()
        || !b.is_finite()
        || !(0. ..=1.).contains(&b.z)
        || b.x.abs() > 1.
        || b.y.abs() > 1.
    {
        return None;
    }
    let uv = |p: glam::Vec3| Vec2::new(0.5 * p.x + 0.5, 0.5 - 0.5 * p.y);
    if uv(b).cmplt(Vec2::ZERO).any() || uv(b).cmpge(Vec2::ONE).any() {
        return None;
    }
    let d = uv(b) - uv(a);
    Some([d.x, d.y, b.z, 1.])
}
