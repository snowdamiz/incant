//! Retained attachments for the ordered geometry -> display -> chrome passes.
//! A single cached size bounds idle memory; encoded commands retain old textures.
use crate::{ResourceError, Result};
use std::sync::{Arc, Mutex};

pub(crate) const HDR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
// At most 192 MiB of color/depth attachments, independent of surface format.
const MAX_PIXELS: u64 = 16 * 1024 * 1024;

pub(crate) struct FrameTargets {
    size: [u32; 2],
    pub color: wgpu::TextureView,
    pub depth: wgpu::TextureView,
}
#[derive(Default)]
pub(crate) struct FrameCache(Mutex<Option<Arc<FrameTargets>>>);
impl FrameCache {
    pub fn get(&self, device: &wgpu::Device, width: u32, height: u32) -> Result<Arc<FrameTargets>> {
        let limit = device.limits().max_texture_dimension_2d.min(8192);
        if width == 0
            || height == 0
            || width > limit
            || height > limit
            || u64::from(width) * u64::from(height) > MAX_PIXELS
        {
            return Err(ResourceError::RenderTargetSize.into());
        }
        let mut cached = self
            .0
            .lock()
            .map_err(|_| ResourceError::CacheLock("frame"))?;
        if let Some(frame) = cached.as_ref().filter(|f| f.size == [width, height]) {
            return Ok(Arc::clone(frame));
        }
        let create = |label, format, usage| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        };
        let frame = Arc::new(FrameTargets {
            size: [width, height],
            color: create(
                "Linear HDR scene",
                HDR_FORMAT,
                wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            ),
            depth: create(
                "Scene depth",
                wgpu::TextureFormat::Depth32Float,
                wgpu::TextureUsages::RENDER_ATTACHMENT,
            ),
        });
        *cached = Some(Arc::clone(&frame));
        Ok(frame)
    }
}
