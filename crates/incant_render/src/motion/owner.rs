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
}
impl Extent {
    pub fn new(output: [u32; 2], internal: [u32; 2], rect: [f32; 4], camera: CameraView) -> Self {
        Self {
            output,
            internal,
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
}
impl MotionFrame {
    /// Present only for temporal accumulation; phase is internal-pixel jitter.
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
}
pub(super) struct Pending {
    pub frame: MotionFrame,
    pub matrix: Mat4,
    pub previous: Mat4,
    extent: Extent,
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
        self.prepare_mode(extent, matrix, false)
    }
    fn prepare_mode(
        &self,
        extent: Extent,
        matrix: Mat4,
        temporal: bool,
    ) -> Result<Pending, MotionError> {
        if !matrix.is_finite() || !matrix.inverse().is_finite() {
            return Err(MotionError::Projection);
        }
        let reset = self.cut
            || self.last.as_ref().is_none_or(|a| {
                a.extent != extent || a.frame.bypassed || a.frame.phase.is_some() != temporal
            });
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
            },
            matrix,
            previous: if valid_previous {
                self.last.as_ref().unwrap().matrix
            } else {
                matrix
            },
            extent,
        })
    }
    pub fn prepare_temporal(
        &self,
        extent: Extent,
        camera: CameraView,
        rect: [f32; 4],
    ) -> Result<Pending, MotionError> {
        let mut pending = self.prepare_mode(
            extent,
            camera
                .matrix(rect[2] / rect[3])
                .map_err(|_| MotionError::Projection)?,
            true,
        )?;
        if !pending.frame.bypassed {
            let phase = if pending.frame.has_previous() {
                (self
                    .last
                    .as_ref()
                    .and_then(|a| a.frame.phase)
                    .ok_or(MotionError::Topology)?
                    + 1)
                    % 8
            } else {
                0
            };
            let mut camera = camera;
            camera.jitter = crate::temporal::jitter::ndc(phase, rect);
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

#[cfg(test)]
#[test]
fn switching_jitter_modes_resets_only_the_candidate_until_accept() {
    let camera = CameraView::preview();
    let rect = [0., 0., 64., 64.];
    let extent = Extent::new([64, 64], [64, 64], rect, camera);
    let matrix = camera.matrix(1.).unwrap();
    let mut h = History::default();
    h.accept(h.prepare_temporal(extent.clone(), camera, rect).unwrap());
    let accepted = h.frame().unwrap();
    let candidate = h.prepare(extent.clone(), matrix).unwrap();
    assert!(!candidate.frame.has_previous());
    assert_eq!(candidate.frame.epoch(), accepted.epoch() + 1);
    assert_eq!(h.frame().unwrap().sequence(), accepted.sequence());
    drop(candidate);
    assert!(
        h.prepare_temporal(extent.clone(), camera, rect)
            .unwrap()
            .frame
            .has_previous()
    );
    h.accept(h.prepare(extent.clone(), matrix).unwrap());
    let native = h.prepare_temporal(extent, camera, rect).unwrap();
    assert!(!native.frame.has_previous());
    assert_eq!(native.frame.jitter_phase(), Some(0));
}
