//! Bounded retained directional depth atlases and immutable per-frame cascades.
pub(crate) mod cascade;
mod pipeline;
use crate::{ResourceError, Result, camera::CameraView};
use cascade::{Bounds, CASCADES, MAP_SIZE};
pub(crate) use pipeline::PipelineKey;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use wgpu::util::DeviceExt;

pub(crate) const MAX_SHADOW_LIGHTS: usize = 4;
#[derive(Clone, Copy)]
pub(crate) struct ShadowLight {
    pub direction: glam::Vec3,
    pub distance: f32,
}
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct CascadeUniform {
    matrix: [[f32; 4]; 4],
    parameters: [f32; 4],
}
struct Atlas {
    lights: usize,
    pub view: wgpu::TextureView,
    layers: Vec<wgpu::TextureView>,
}
pub(crate) struct ShadowSystem {
    cameras: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    atlas: Mutex<Option<Arc<Atlas>>>,
    pipelines: Mutex<HashMap<PipelineKey, wgpu::RenderPipeline>>,
}
pub(crate) struct ShadowLayer {
    pub view: wgpu::TextureView,
    pub camera: wgpu::BindGroup,
}
pub(crate) struct PreparedShadows {
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub uniform: wgpu::Buffer,
    pub layers: Vec<ShadowLayer>,
}
impl ShadowSystem {
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            cameras: device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Directional cascade camera"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            }),
            sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("Directional shadow comparison"),
                compare: Some(wgpu::CompareFunction::LessEqual),
                min_filter: wgpu::FilterMode::Linear,
                mag_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            }),
            atlas: Mutex::new(None),
            pipelines: Mutex::new(HashMap::new()),
        }
    }
    fn atlas(&self, device: &wgpu::Device, lights: usize) -> Result<Arc<Atlas>> {
        let layers = (lights * CASCADES).max(1) as u32;
        let size = if lights == 0 { 1 } else { MAP_SIZE };
        if lights > MAX_SHADOW_LIGHTS
            || layers > device.limits().max_texture_array_layers
            || size > device.limits().max_texture_dimension_2d
        {
            return Err(ResourceError::ShadowMapSize.into());
        }
        let mut cache = self
            .atlas
            .lock()
            .map_err(|_| ResourceError::CacheLock("shadow atlas"))?;
        if let Some(atlas) = cache.as_ref().filter(|a| a.lights == lights) {
            return Ok(Arc::clone(atlas));
        }
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Directional shadow atlas"),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: layers,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let atlas = Arc::new(Atlas {
            lights,
            view: texture.create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2Array),
                ..Default::default()
            }),
            layers: (0..layers)
                .map(|layer| {
                    texture.create_view(&wgpu::TextureViewDescriptor {
                        dimension: Some(wgpu::TextureViewDimension::D2),
                        base_array_layer: layer,
                        array_layer_count: Some(1),
                        ..Default::default()
                    })
                })
                .collect(),
        });
        *cache = Some(Arc::clone(&atlas));
        Ok(atlas)
    }
    pub fn prepare(
        &self,
        device: &wgpu::Device,
        camera: CameraView,
        aspect: f32,
        lights: &[ShadowLight],
        casters: &[Bounds],
    ) -> Result<PreparedShadows> {
        let atlas = self.atlas(device, lights.len())?;
        let mut data = [CascadeUniform {
            matrix: [[0.; 4]; 4],
            parameters: [0.; 4],
        }; MAX_SHADOW_LIGHTS * CASCADES];
        let mut layers = Vec::new();
        for (light_index, light) in lights.iter().enumerate() {
            if let Some(cascades) =
                cascade::fit(camera, aspect, light.direction, light.distance, casters)?
            {
                for (i, cascade) in cascades.into_iter().enumerate() {
                    let index = light_index * CASCADES + i;
                    let matrix = cascade.matrix.to_cols_array_2d();
                    data[index] = CascadeUniform {
                        matrix,
                        parameters: [
                            cascade.far,
                            cascade.blend_start,
                            cascade.texel_world,
                            cascade.inverse_depth,
                        ],
                    };
                    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Immutable cascade camera"),
                        contents: bytemuck::bytes_of(&matrix),
                        usage: wgpu::BufferUsages::UNIFORM,
                    });
                    layers.push(ShadowLayer {
                        view: atlas.layers[index].clone(),
                        camera: device.create_bind_group(&wgpu::BindGroupDescriptor {
                            label: Some("Immutable cascade camera"),
                            layout: &self.cameras,
                            entries: &[wgpu::BindGroupEntry {
                                binding: 0,
                                resource: buffer.as_entire_binding(),
                            }],
                        }),
                    });
                }
            }
        }
        Ok(PreparedShadows {
            view: atlas.view.clone(),
            sampler: self.sampler.clone(),
            layers,
            uniform: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Immutable shadow cascades"),
                contents: bytemuck::cast_slice(&data),
                usage: wgpu::BufferUsages::UNIFORM,
            }),
        })
    }
    pub fn pipeline(
        &self,
        device: &wgpu::Device,
        materials: &wgpu::BindGroupLayout,
        key: PipelineKey,
    ) -> Result<wgpu::RenderPipeline> {
        let mut cache = self
            .pipelines
            .lock()
            .map_err(|_| ResourceError::CacheLock("shadow pipeline"))?;
        Ok(cache
            .entry(key)
            .or_insert_with(|| pipeline::build(device, &self.cameras, materials, key))
            .clone())
    }
}
