//! Fixed output ABI: this mirrors the host cook's deliberately narrow contract.
use std::num::NonZeroU64;
const ENTRIES: [wgpu::BindGroupLayoutEntry; 2] = [
    wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: false },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    },
    wgpu::BindGroupLayoutEntry {
        binding: 1,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: NonZeroU64::new(80),
        },
        count: None,
    },
];
pub(super) fn create(device: &wgpu::Device) -> wgpu::PipelineLayout {
    let bind_group = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Cooked display ABI"),
        entries: &ENTRIES,
    });
    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Cooked display ABI"),
        bind_group_layouts: &[Some(&bind_group)],
        immediate_size: 0,
    })
}
#[cfg(feature = "cooked-bloom")]
pub(super) fn create_bloom(device: &wgpu::Device) -> wgpu::PipelineLayout {
    let entries = [
        ENTRIES[0],
        ENTRIES[1],
        wgpu::BindGroupLayoutEntry {
            binding: 2,
            ..ENTRIES[0]
        },
        wgpu::BindGroupLayoutEntry {
            binding: 3,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: NonZeroU64::new(16),
            },
            ..ENTRIES[1]
        },
    ];
    let bindings = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Cooked bloom display ABI"),
        entries: &entries,
    });
    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Cooked bloom display ABI"),
        bind_group_layouts: &[Some(&bindings)],
        immediate_size: 0,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn output_formats_remain_the_same_fixed_catalog() {
        for format in [
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            wgpu::TextureFormat::Bgra8Unorm,
            wgpu::TextureFormat::Bgra8UnormSrgb,
        ] {
            super::super::OutputPass::validate(format).unwrap();
        }
        assert!(super::super::OutputPass::validate(wgpu::TextureFormat::Rgba16Float).is_err());
    }
    #[test]
    fn uniform_offsets_and_binding_size_match_the_cook_contract() {
        assert_eq!(size_of::<super::super::OutputUniform>(), 80);
        assert_eq!(
            std::mem::offset_of!(super::super::OutputUniform, backdrop),
            0
        );
        assert_eq!(std::mem::offset_of!(super::super::OutputUniform, rect), 16);
        assert_eq!(std::mem::offset_of!(super::super::OutputUniform, radii), 32);
        assert_eq!(
            std::mem::offset_of!(super::super::OutputUniform, canvas),
            48
        );
        assert_eq!(
            std::mem::offset_of!(super::super::OutputUniform, pixel_grid),
            64
        );
        assert!(
            ENTRIES
                .iter()
                .all(|e| e.visibility == wgpu::ShaderStages::FRAGMENT && e.count.is_none())
        );
    }
}
