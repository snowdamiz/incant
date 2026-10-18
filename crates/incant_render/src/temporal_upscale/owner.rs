use crate::{
    Result,
    motion::{MotionFrame, TemporalInput},
    temporal::TemporalError,
};
use std::sync::Arc;
pub(super) struct Targets {
    pub size: [u32; 2],
    pub color: [wgpu::TextureView; 2],
    pub support: [wgpu::TextureView; 2],
}
pub(crate) struct State {
    targets: Option<Arc<Targets>>,
    last: Option<MotionFrame>,
    slot: usize,
    continuity: super::continuity::Continuity,
}
pub(crate) struct Pending {
    pub(super) targets: Arc<Targets>,
    pub(super) input: TemporalInput,
    pub(super) previous_jitter: [f32; 2],
    pub(super) stationary: bool,
    camera: super::continuity::Camera,
}
pub(super) fn bytes(size: [u32; 2], limit: u32) -> Result<u64> {
    let n = u64::from(size[0]) * u64::from(size[1]);
    let n = n.checked_mul(32).ok_or(TemporalError::Capacity)?;
    if size.contains(&0)
        || size.iter().any(|&v| v > limit.min(8192))
        || n > super::MAX_HISTORY_BYTES
    {
        return Err(TemporalError::Capacity.into());
    }
    Ok(n)
}
impl State {
    #[cfg(test)]
    pub(crate) fn color_test_view(&self) -> wgpu::TextureView {
        self.targets.as_ref().unwrap().color[self.slot].clone()
    }
    pub fn new(light_capacity: usize) -> Self {
        Self {
            targets: None,
            last: None,
            slot: 1,
            continuity: super::continuity::Continuity::new(light_capacity),
        }
    }
    // Explicit cold allocation. Accepted and candidate history are bounded to
    // 2*MAX_HISTORY_BYTES; queued driver resources are separately bounded by host.
    pub fn prepare_extent(&mut self, device: &wgpu::Device, size: [u32; 2]) -> Result<()> {
        bytes(size, device.limits().max_texture_dimension_2d)?;
        if self.targets.as_ref().is_some_and(|t| t.size == size) {
            return Ok(());
        }
        let make = |format| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("Experimental TAAU output history"),
                    size: wgpu::Extent3d {
                        width: size[0],
                        height: size[1],
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING
                        | wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        };
        self.targets = Some(Arc::new(Targets {
            size,
            color: std::array::from_fn(|_| make(crate::frame::HDR_FORMAT)),
            support: std::array::from_fn(|_| make(wgpu::TextureFormat::Rg32Float)),
        }));
        self.last = None;
        self.slot = 1;
        Ok(())
    }
    pub fn prepare(
        &self,
        input: TemporalInput,
        output: [u32; 2],
        camera: crate::camera::CameraView,
        lights: &[crate::lighting::scene::Light],
        geometry_stationary: bool,
    ) -> Result<Pending> {
        if !self.continuity.fits(lights) {
            return Err(TemporalError::Capacity.into());
        }
        let targets =
            self.targets
                .as_ref()
                .filter(|t| t.size == output)
                .ok_or(crate::SceneError::Input(
                    "TAAU output extent was not prepared at startup",
                ))?;
        let s = input.surfaces.as_ref().ok_or(TemporalError::History)?;
        if input.frame.bypassed()
            || s.slot > 1
            || input.frame.jitter_phase().is_none()
            || (input.frame.has_previous()
                && (s.slot != 1 - self.slot
                    || self.last.as_ref().is_none_or(|old| {
                        !old.same_owner(&input.frame)
                            || old.epoch() != input.frame.epoch()
                            || old.sequence().checked_add(1) != Some(input.frame.sequence())
                    })))
        {
            return Err(TemporalError::History.into());
        }
        let previous_jitter = self
            .last
            .as_ref()
            .and_then(|f| f.jitter_phase())
            .map_or([0.; 2], |p| crate::temporal::jitter::SAMPLES[p as usize]);
        let camera = super::continuity::Camera::new(camera, input.rect);
        let stationary = input.frame.has_previous()
            && self.continuity.admits(camera, lights, geometry_stationary);
        Ok(Pending {
            targets: Arc::clone(targets),
            input,
            previous_jitter,
            stationary,
            camera,
        })
    }
    pub fn accept(&mut self, pending: Pending, lights: &[crate::lighting::scene::Light]) {
        self.continuity.accept(pending.camera, lights);
        self.slot = pending
            .input
            .surfaces
            .as_ref()
            .expect("admitted surfaces")
            .slot;
        self.last = Some(pending.input.frame);
    }
}
