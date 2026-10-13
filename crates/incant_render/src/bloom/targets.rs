//! One retained extent. Old encoded frames keep their own wgpu handles alive.
use super::config::Plan;
pub(super) struct Level {
    pub down: wgpu::TextureView,
    pub up: Option<wgpu::TextureView>,
}
pub(super) struct Targets {
    pub plan: Plan,
    pub levels: Vec<Level>,
}
impl Targets {
    pub fn new(device: &wgpu::Device, plan: Plan) -> Self {
        let levels = plan
            .sizes
            .iter()
            .enumerate()
            .map(|(i, size)| Level {
                down: texture(device, *size),
                up: (i + 1 < plan.sizes.len()).then(|| texture(device, *size)),
            })
            .collect();
        Self { plan, levels }
    }
}
fn texture(device: &wgpu::Device, [width, height]: [u32; 2]) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("Bloom retained RGBA16 pyramid"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: crate::frame::HDR_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
        .create_view(&Default::default())
}
