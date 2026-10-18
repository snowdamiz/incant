//! Test-only GPU reconstruction prototype. No CPU pixel admission/readback.
//! 144 B/output-pixel visibility + 32 B old histories + 8 B owner histories.
use super::{
    Pending,
    system::{Draw, Frame},
};
use crate::{Renderer, Result, motion::visibility::Probe, temporal::TemporalError};
use std::sync::Arc;
use wgpu::util::DeviceExt;

pub(crate) struct System {
    pub size: [u32; 2],
    pub visibility: Arc<Probe>,
    owner: [wgpu::TextureView; 2],
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    classify: Option<(wgpu::RenderPipeline, wgpu::BindGroupLayout)>,
    pub(crate) fallback: Option<Arc<crate::motion::visibility_shading::System>>,
}
// Correctness episode is intentionally tiny. Allocation is checked before any
// GPU resource creation; this is not a proposed shipping memory budget.
pub(super) fn bytes(size: [u32; 2]) -> Result<u64> {
    if size.contains(&0) || size.iter().any(|&n| n > 128) {
        return Err(TemporalError::Capacity.into());
    }
    Ok(u64::from(size[0]) * u64::from(size[1]) * 184)
}
pub(super) fn source() -> String {
    let common = include_str!("resolve.wgsl")
        .split_once("struct Resolved")
        .expect("shared pure helpers")
        .0;
    format!("{common}\n{}", include_str!("visibility_resolve.wgsl"))
}
impl System {
    pub(crate) fn owner_view(&self, slot: usize) -> wgpu::TextureView {
        self.owner[slot].clone()
    }
    pub fn new(r: &Renderer, size: [u32; 2]) -> Result<Self> {
        Self::with_fallback(r, size, None)
    }
    pub(crate) fn with_fallback(
        r: &Renderer,
        size: [u32; 2],
        mode: Option<super::visibility_fallback::Mode>,
    ) -> Result<Self> {
        fallback_bytes(size, mode.is_some())?;
        let d = &r.device;
        let visibility = Arc::new(Probe::reconstruction(r, size));
        let owner = std::array::from_fn(|_| {
            d.create_texture(&wgpu::TextureDescriptor {
                label: Some("Single-owner radiance provenance"),
                size: wgpu::Extent3d {
                    width: size[0],
                    height: size[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R32Uint,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&Default::default())
        });
        let mut entries: Vec<_> = (0..13)
            .map(|binding| wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::FRAGMENT,
                count: None,
                ty: if binding == 0 {
                    wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(64),
                    }
                } else {
                    wgpu::BindingType::Texture {
                        sample_type: if [3, 11].contains(&binding) {
                            wgpu::TextureSampleType::Depth
                        } else if binding >= 6 {
                            wgpu::TextureSampleType::Uint
                        } else {
                            wgpu::TextureSampleType::Float { filterable: false }
                        },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: (7..=11).contains(&binding),
                    }
                },
            })
            .collect();
        let classify = mode.map(|mode| {
            let layout = d.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &entries,
            });
            let pipeline_layout = d.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[Some(&layout)],
                immediate_size: 0,
            });
            let module = d.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("GPU missing-sample classification"),
                source: wgpu::ShaderSource::Wgsl(
                    super::visibility_fallback::classify_source(mode).into(),
                ),
            });
            let pipeline = d.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vertex"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some("classify"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::R32Uint,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            });
            (pipeline, layout)
        });
        let fallback = mode.map(|mode| {
            Arc::new(crate::motion::visibility_shading::System::new(
                r,
                size,
                mode == super::visibility_fallback::Mode::AllCurrent,
            ))
        });
        if fallback.is_some() {
            entries.extend((13..16).map(|binding| wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::FRAGMENT,
                count: None,
                ty: wgpu::BindingType::Texture {
                    sample_type: match binding {
                        13 => wgpu::TextureSampleType::Uint,
                        15 => wgpu::TextureSampleType::Depth,
                        _ => wgpu::TextureSampleType::Float { filterable: false },
                    },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: binding != 13,
                },
            }));
        }
        let layout = d.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("GPU visibility reconstruction"),
            entries: &entries,
        });
        let pl = d.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let shader = d.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Visibility admitted radiance"),
            source: wgpu::ShaderSource::Wgsl(
                mode.map_or_else(source, super::visibility_fallback::resolve_source)
                    .into(),
            ),
        });
        let targets = [
            crate::frame::HDR_FORMAT,
            wgpu::TextureFormat::Rg32Float,
            wgpu::TextureFormat::R32Uint,
        ]
        .map(|format| {
            Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })
        });
        let pipeline = d.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("GPU visibility reconstruction"),
            layout: Some(&pl),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("resolve"),
                compilation_options: Default::default(),
                targets: &targets,
            }),
            multiview_mask: None,
            cache: None,
        });
        Ok(Self {
            size,
            visibility,
            owner,
            pipeline,
            layout,
            classify,
            fallback,
        })
    }
    pub fn prepare(
        &self,
        d: &wgpu::Device,
        p: &Pending,
        current: &wgpu::TextureView,
        camera: crate::camera::CameraView,
    ) -> Draw {
        let s = p.input.surfaces.as_ref().expect("admitted motion");
        let jitter =
            crate::temporal::jitter::SAMPLES[p.input.frame.jitter_phase().unwrap() as usize];
        let uniform = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            usage: wgpu::BufferUsages::UNIFORM,
            contents: bytemuck::bytes_of(&Frame {
                source: p.input.rect,
                output: [self.size[0] as f32, self.size[1] as f32, 0., 0.],
                depth: [
                    camera.near,
                    camera.far,
                    f32::from(u8::from(camera.orthographic_half_height.is_some())),
                    f32::from(u8::from(p.input.frame.has_previous())),
                ],
                jitter: [
                    jitter[0],
                    jitter[1],
                    p.previous_jitter[0],
                    p.previous_jitter[1],
                ],
            }),
        });
        let views = [
            current,
            &s.motion,
            &s.current_depth,
            &p.targets.color[1 - s.slot],
            &p.targets.support[1 - s.slot],
            s.owner.as_ref().expect("admitted owner"),
            &self.visibility.colors[0],
            &self.visibility.colors[1],
            &self.visibility.colors[2],
            &self.visibility.colors[3],
            &self.visibility.depth,
            &self.owner[1 - s.slot],
        ];
        let mut entries = vec![wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform.as_entire_binding(),
        }];
        entries.extend(
            views
                .into_iter()
                .enumerate()
                .map(|(i, v)| wgpu::BindGroupEntry {
                    binding: i as u32 + 1,
                    resource: wgpu::BindingResource::TextureView(v),
                }),
        );
        let classification = self.classify.as_ref().map(|(pipeline, layout)| {
            super::visibility_fallback::Classification {
                pipeline: pipeline.clone(),
                group: d.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout,
                    entries: &entries,
                }),
                mask: self.fallback.as_ref().unwrap().mask.clone(),
            }
        });
        if let Some(f) = &self.fallback {
            entries.extend(
                [&f.mask, &f.color, &f.depth]
                    .into_iter()
                    .enumerate()
                    .map(|(i, v)| wgpu::BindGroupEntry {
                        binding: 13 + i as u32,
                        resource: wgpu::BindingResource::TextureView(v),
                    }),
            );
        }
        Draw {
            pipeline: self.pipeline.clone(),
            group: d.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &self.layout,
                entries: &entries,
            }),
            color: p.targets.color[s.slot].clone(),
            support: p.targets.support[s.slot].clone(),
            identity: Some(self.owner[s.slot].clone()),
            visibility_before: true,
            classification,
        }
    }
}

pub(super) fn fallback_bytes(size: [u32; 2], enabled: bool) -> Result<u64> {
    let old = bytes(size)?;
    Ok(old
        + if enabled {
            u64::from(size[0]) * u64::from(size[1]) * 52
        } else {
            0
        })
}
