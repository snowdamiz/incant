use super::{MotionError, plan};
use std::sync::Arc;
pub(super) struct Targets {
    pub size: [u32; 2],
    pub bytes: u64,
    pub owner_bytes: u64,
    pub owner: Option<wgpu::TextureView>,
    pub motion: wgpu::TextureView,
    pub depth: [wgpu::TextureView; 2],
}
impl Targets {
    pub fn prepare(
        device: &wgpu::Device,
        size: [u32; 2],
        old: Option<&Arc<Self>>,
        owner: bool,
    ) -> Result<Arc<Self>, MotionError> {
        let bytes = plan::target_bytes(size, device.limits().max_texture_dimension_2d)?;
        if let Some(old) = old.filter(|old| old.size == size && old.owner.is_some() == owner) {
            return Ok(Arc::clone(old));
        }
        plan::overlap(
            old.map_or(0, |o| o.bytes),
            bytes,
            plan::MAX_MOTION_TARGET_BYTES,
        )?;
        let owner_bytes = if owner { bytes / 6 } else { 0 };
        plan::overlap(
            old.map_or(0, |o| o.owner_bytes),
            owner_bytes,
            super::attachment::MAX_OWNER_TARGET_BYTES,
        )?;
        let make = |label, format| {
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
                        | wgpu::TextureUsages::COPY_SRC,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        };
        Ok(Arc::new(Self {
            size,
            bytes,
            owner_bytes,
            owner: owner.then(|| make("Opaque surface owner", wgpu::TextureFormat::R32Uint)),
            motion: make("Opaque surface motion", wgpu::TextureFormat::Rgba32Float),
            depth: [
                make("Motion depth A", wgpu::TextureFormat::Depth32Float),
                make("Motion depth B", wgpu::TextureFormat::Depth32Float),
            ],
        }))
    }
}
