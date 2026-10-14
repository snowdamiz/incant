#[cfg(target_os = "macos")]
use super::plan::Plan;
/// Native inputs allocate the admitted maximum content extent so ordinary DRS
/// changes need neither new native pipelines nor new textures.
pub(super) struct Targets {
    pub textures: [wgpu::Texture; 6],
    pub views: [wgpu::TextureView; 6],
    pub size: [u32; 2],
    pub bytes: u64,
    pub normalized: wgpu::TextureView,
    _allocation: super::lifecycle::Allocation,
}
#[cfg(target_os = "macos")]
impl Targets {
    pub fn new(
        d: &wgpu::Device,
        p: Plan,
        allocation: super::lifecycle::Allocation,
    ) -> Result<Self, super::MetalFxError> {
        let size = p.size();
        let bytes = p.bytes()?;
        let formats = [
            crate::frame::HDR_FORMAT,
            wgpu::TextureFormat::R32Float,
            wgpu::TextureFormat::Rg16Float,
            wgpu::TextureFormat::R8Unorm,
            crate::frame::HDR_FORMAT,
            wgpu::TextureFormat::R16Float,
        ];
        let textures = std::array::from_fn(|i| {
            let size = if i == 5 { [1, 1] } else { size };
            d.create_texture(&wgpu::TextureDescriptor {
                label: Some("Bounded MetalFX private target"),
                size: wgpu::Extent3d {
                    width: size[0],
                    height: size[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: formats[i],
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | if i == 4 {
                        wgpu::TextureUsages::STORAGE_BINDING
                    } else {
                        wgpu::TextureUsages::empty()
                    },
                view_formats: &[],
            })
        });
        let views = std::array::from_fn(|i| textures[i].create_view(&Default::default()));
        let normalized = d
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("MetalFX current-coverage output"),
                size: wgpu::Extent3d {
                    width: size[0],
                    height: size[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: crate::frame::HDR_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&Default::default());
        Ok(Self {
            normalized,
            textures,
            views,
            size,
            bytes,
            _allocation: allocation,
        })
    }
}
