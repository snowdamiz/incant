use std::sync::Arc;
pub(super) struct Output {
    pub size: [u32; 2],
    pub color: [wgpu::TextureView; 2],
    pub depth: [wgpu::TextureView; 2],
    pub composite: wgpu::TextureView,
    /// Unjittered opaque depth at output resolution for current-only sprites and
    /// world text drawn over `composite`. Per frame; never history.
    pub overlay_depth: wgpu::TextureView,
    pub record: Option<[wgpu::TextureView; 2]>,
    /// RG16Float (effective sample mass, integer age) per history slot, written
    /// only by the portable ordinary resolve together with colour and depth.
    pub evidence: [wgpu::TextureView; 2],
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
        u64::from(self.size[0])
            * u64::from(self.size[1])
            * if self.record.is_some() { 60 } else { 44 }
    }
    pub fn prepare(
        d: &wgpu::Device,
        size: [u32; 2],
        old: Option<&Arc<Self>>,
        owner: bool,
    ) -> Arc<Self> {
        if let Some(old) = old.filter(|x| x.size == size && x.record.is_some() == owner) {
            return Arc::clone(old);
        }
        Arc::new(Self {
            size,
            record: owner.then(|| {
                std::array::from_fn(|_| {
                    make(
                        d,
                        size,
                        wgpu::TextureFormat::Rg32Uint,
                        "Output owner provenance",
                    )
                })
            }),
            color: std::array::from_fn(|_| {
                make(d, size, crate::frame::HDR_FORMAT, "Output temporal HDR")
            }),
            evidence: std::array::from_fn(|_| {
                make(
                    d,
                    size,
                    wgpu::TextureFormat::Rg16Float,
                    "Output temporal evidence mass and age",
                )
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
            overlay_depth: make(
                d,
                size,
                wgpu::TextureFormat::Depth32Float,
                "Output-resolution current-only overlay depth",
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
