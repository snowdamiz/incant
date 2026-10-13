//! Compare the actual declarations used by pipeline construction to the pinned
//! native backend map. Material's unused bindings still reserve native slots.
use super::Kind;
fn uniform(size: u64, visibility: wgpu::ShaderStages) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: wgpu::BufferSize::new(size),
        },
        count: None,
    }
}
fn texture(binding: u32, depth: bool) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: if depth {
                wgpu::TextureSampleType::Depth
            } else {
                wgpu::TextureSampleType::Float { filterable: false }
            },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}
pub(super) fn validate(kind: Kind) -> Result<(), &'static str> {
    use wgpu::{VertexFormat as F, VertexStepMode as S};
    if matches!(kind, Kind::Motion | Kind::Depth | Kind::MotionOwner) {
        if crate::motion::system::frame_entry() != uniform(144, wgpu::ShaderStages::VERTEX_FRAGMENT)
        {
            return Err("motion frame layout");
        }
        let mut expected = vec![
            (
                48,
                S::Vertex,
                vec![(0, 0, F::Float32x3), (1, 24, F::Float32x2)],
            ),
            (48, S::Vertex, vec![(2, 0, F::Float32x3)]),
            (
                112,
                S::Instance,
                (3..=6)
                    .map(|i| (i, u64::from(i - 3) * 16, F::Float32x4))
                    .collect(),
            ),
            (
                80,
                S::Instance,
                (7..=11)
                    .map(|i| (i, u64::from(i - 7) * 16, F::Float32x4))
                    .collect(),
            ),
        ];
        let actual = if kind == Kind::MotionOwner {
            expected[3].2 = (7..=10)
                .map(|i| (i, u64::from(i - 7) * 16, F::Float32x4))
                .chain([(11, 64, F::Float32), (12, 68, F::Uint32)])
                .collect();
            crate::motion::system::owner_buffers()
        } else {
            crate::motion::system::buffers()
        };
        for (actual, (stride, step, attributes)) in actual.iter().zip(expected) {
            let got: Vec<_> = actual
                .attributes
                .iter()
                .map(|a| (a.shader_location, a.offset, a.format))
                .collect();
            if actual.array_stride != stride || actual.step_mode != step || got != attributes {
                return Err("motion vertex ABI");
            }
        }
        // This existing validator covers the complete 64-byte material host
        // uniform + five texture/sampler pairs, rather than only live base-map.
        crate::geometry_shader::cooked::validate_host(crate::geometry_shader::Kind::Shadow)?;
    } else {
        let (actual, size, count) = match kind {
            Kind::Resolve => (crate::temporal::system::layout_entries(false), 64, 4),
            Kind::Copy => (crate::temporal::system::layout_entries(true), 64, 1),
            Kind::Motion | Kind::Depth | Kind::MotionOwner => unreachable!(),
        };
        let expected = if kind == Kind::Copy {
            vec![
                texture(0, false),
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    ..uniform(64, wgpu::ShaderStages::FRAGMENT)
                },
            ]
        } else {
            let mut e = vec![uniform(size, wgpu::ShaderStages::FRAGMENT)];
            e.extend((1..=count).map(|i| texture(i, i == 3)));
            e
        };
        if actual != expected {
            return Err("temporal texture/uniform host ABI");
        }
    }
    if crate::frame::HDR_FORMAT != wgpu::TextureFormat::Rgba16Float {
        return Err("temporal HDR target");
    }
    Ok(())
}
