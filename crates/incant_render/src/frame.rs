//! Retained attachments for the ordered geometry -> display -> chrome passes.
//! A single cached size bounds idle memory; encoded commands retain old textures.
use crate::{ResourceError, Result};
use std::sync::{Arc, Mutex};

pub(crate) const HDR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
// Both policies retain the 16MP ceiling. Their separate checked byte caps are
// 192MiB at 1x and 512MiB at 4x, independent of caller output format.
const MAX_PIXELS: u64 = 16 * 1024 * 1024;

pub(crate) struct FrameTargets {
    size: [u32; 2],
    pub samples: crate::SceneMsaa,
    pub color: wgpu::TextureView,
    pub resolved: wgpu::TextureView,
    pub depth: wgpu::TextureView,
}
#[derive(Default)]
pub(crate) struct FrameCache(Mutex<Option<Arc<FrameTargets>>>);
impl FrameCache {
    pub fn validate(device: &wgpu::Device, width: u32, height: u32) -> Result<()> {
        let limit = device.limits().max_texture_dimension_2d.min(8192);
        if width == 0
            || height == 0
            || width > limit
            || height > limit
            || u64::from(width) * u64::from(height) > MAX_PIXELS
        {
            return Err(ResourceError::RenderTargetSize.into());
        }
        crate::SceneMsaa::One
            .target_bytes([width, height])
            .map_err(|_| ResourceError::RenderTargetSize)?;
        Ok(())
    }
    pub fn get(
        &self,
        device: &wgpu::Device,
        width: u32,
        height: u32,
        samples: crate::SceneMsaa,
    ) -> Result<Arc<FrameTargets>> {
        Self::validate(device, width, height)?;
        samples.target_bytes([width, height])?;
        let mut cached = self
            .0
            .lock()
            .map_err(|_| ResourceError::CacheLock("frame"))?;
        if let Some(frame) = cached
            .as_ref()
            .filter(|f| f.size == [width, height] && f.samples == samples)
        {
            return Ok(Arc::clone(frame));
        }
        let create = |label, format, usage, count| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: count,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        };
        let resolved = create(
            "Resolved linear HDR scene",
            HDR_FORMAT,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            1,
        );
        let color = if samples == crate::SceneMsaa::One {
            resolved.clone()
        } else {
            create(
                "Multisampled linear HDR scene",
                HDR_FORMAT,
                wgpu::TextureUsages::RENDER_ATTACHMENT,
                samples.samples(),
            )
        };
        let frame = Arc::new(FrameTargets {
            size: [width, height],
            samples,
            color,
            resolved,
            depth: create(
                "Scene depth",
                wgpu::TextureFormat::Depth32Float,
                wgpu::TextureUsages::RENDER_ATTACHMENT,
                samples.samples(),
            ),
        });
        *cached = Some(Arc::clone(&frame));
        Ok(frame)
    }
}

/// One real hardware color resolve after every world pass. No depth sampling or
/// approximate depth resolve. The Arc keeps both multisample and 1x views owned.
pub(crate) struct SceneResolve(Arc<FrameTargets>);
impl SceneResolve {
    pub fn new(frame: Arc<FrameTargets>) -> Option<Self> {
        (frame.samples == crate::SceneMsaa::Four).then_some(Self(frame))
    }
    pub fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        queries: Option<&crate::timing::PassQueries>,
    ) {
        let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Resolve complete linear HDR world"),
            timestamp_writes: queries.map(|q| q.render()),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.0.color,
                resolve_target: Some(&self.0.resolved),
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Discard,
                },
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });
    }
}
