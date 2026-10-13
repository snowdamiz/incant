use super::math::target_bytes;
use crate::{ResourceError, Result};
use std::sync::{Arc, Mutex};

pub(crate) struct Targets {
    size: [u32; 2],
    pub depth: wgpu::TextureView,
    pub normals: wgpu::TextureView,
    pub accessibility: wgpu::TextureView,
    pub scratch: wgpu::TextureView,
}
#[derive(Default)]
pub(crate) struct Cache(Mutex<Option<Arc<Targets>>>);
impl Cache {
    pub fn get(&self, device: &wgpu::Device, size: [u32; 2]) -> Result<Arc<Targets>> {
        target_bytes(size)?;
        crate::frame::FrameCache::validate(device, size[0], size[1])?;
        let mut cached = self
            .0
            .lock()
            .map_err(|_| ResourceError::CacheLock("SSAO targets"))?;
        if let Some(targets) = cached.as_ref().filter(|t| t.size == size) {
            return Ok(Arc::clone(targets));
        }
        let create = |label, format| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d {
                        width: size[0],
                        height: size[1],
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING
                        | if cfg!(all(
                            feature = "ssao-stage-readback",
                            not(target_arch = "wasm32")
                        )) && format == wgpu::TextureFormat::R8Unorm
                        {
                            wgpu::TextureUsages::COPY_SRC
                        } else {
                            wgpu::TextureUsages::empty()
                        },
                    view_formats: &[],
                })
                .create_view(&Default::default())
        };
        let targets = Arc::new(Targets {
            size,
            depth: create("SSAO depth", wgpu::TextureFormat::Depth32Float),
            normals: create(
                "SSAO authored shading normals",
                wgpu::TextureFormat::Rgba16Float,
            ),
            accessibility: create("SSAO diffuse accessibility", wgpu::TextureFormat::R8Unorm),
            scratch: create(
                "SSAO horizontal accessibility",
                wgpu::TextureFormat::R8Unorm,
            ),
        });
        *cached = Some(Arc::clone(&targets));
        Ok(targets)
    }
}
