use std::sync::Arc;
pub(super) struct Output {
    pub size: [u32; 2],
    pub color: [wgpu::TextureView; 2],
    pub depth: [wgpu::TextureView; 2],
    pub composite: wgpu::TextureView,
}
pub(super) struct Overlay {
    pub size: [u32; 2],
    pub view: wgpu::TextureView,
}
fn make(
    device: &wgpu::Device,
    size: [u32; 2],
    format: wgpu::TextureFormat,
    label: &str,
) -> wgpu::TextureView {
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
}
impl Output {
    pub fn bytes(&self) -> u64 {
        u64::from(self.size[0]) * u64::from(self.size[1]) * 32
    }
    pub fn prepare(d: &wgpu::Device, size: [u32; 2], old: Option<&Arc<Self>>) -> Arc<Self> {
        if let Some(old) = old.filter(|x| x.size == size) {
            return Arc::clone(old);
        }
        Arc::new(Self {
            size,
            color: std::array::from_fn(|_| {
                make(d, size, crate::frame::HDR_FORMAT, "Output temporal HDR")
            }),
            depth: std::array::from_fn(|_| {
                make(
                    d,
                    size,
                    wgpu::TextureFormat::R32Float,
                    "Output temporal view depth",
                )
            }),
            composite: make(
                d,
                size,
                crate::frame::HDR_FORMAT,
                "Output HDR with current overlays",
            ),
        })
    }
}
impl Overlay {
    pub fn bytes(&self) -> u64 {
        u64::from(self.size[0]) * u64::from(self.size[1]) * 8
    }
    pub fn prepare(d: &wgpu::Device, size: [u32; 2], old: Option<&Arc<Self>>) -> Arc<Self> {
        if let Some(old) = old.filter(|x| x.size == size) {
            return Arc::clone(old);
        }
        Arc::new(Self {
            size,
            view: make(
                d,
                size,
                crate::frame::HDR_FORMAT,
                "Current-only scene overlay",
            ),
        })
    }
}
