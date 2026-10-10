//! Immutable sprite resources, deterministic painter order and retained frame commands.
//! Geometry, premultiplied linear color and clip/depth math; no style presets.
mod drawing;
mod texture;
use crate::{ResourceError, Result};
pub(crate) use drawing::SpriteDraw;
use incant_2d::{SpriteRenderer, SpriteSampling};
use incant_assets::{RuntimeAsset, RuntimeAssetData};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, Weak},
};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    uv: [f32; 2],
    tint: [f32; 4],
    uv_bounds: [f32; 4],
}
pub(crate) struct SpritePlan {
    pub source: Arc<RuntimeAsset>,
    pub sprite: SpriteRenderer,
    pub world: glam::DMat4,
    pub id: String,
}
struct GpuTexture {
    _source: Arc<RuntimeAsset>,
    _image: wgpu::Texture,
    groups: [wgpu::BindGroup; 2],
}
struct Batch {
    texture: Arc<GpuTexture>,
    sampling: usize,
    start: u32,
    count: u32,
}
pub(crate) struct SpriteScene {
    vertices: wgpu::Buffer,
    batches: Vec<Batch>,
}
pub(crate) struct SpriteSystem {
    globals: wgpu::BindGroupLayout,
    textures: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    cache: Mutex<HashMap<String, Weak<GpuTexture>>>,
}
impl SpriteSystem {
    pub fn new(device: &wgpu::Device) -> Self {
        let globals = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Sprite camera"),
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
        });
        let textures = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Sprite texture"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Sprite pipeline"),
            bind_group_layouts: &[Some(&globals), Some(&textures)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Sprite coordinate and premultiplied alpha math"),
            source: wgpu::ShaderSource::Wgsl(include_str!("sprite.wgsl").into()),
        });
        let pipeline=device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label:Some("Sorted sprites in shared HDR graph"),layout:Some(&layout),
            vertex:wgpu::VertexState{module:&shader,entry_point:Some("vertex"),compilation_options:Default::default(),buffers:&[wgpu::VertexBufferLayout {
                array_stride:size_of::<Vertex>() as u64,step_mode:wgpu::VertexStepMode::Vertex,
                attributes:&wgpu::vertex_attr_array![0=>Float32x3,1=>Float32x2,2=>Float32x4,3=>Float32x4],
            }]},
            fragment:Some(wgpu::FragmentState{module:&shader,entry_point:Some("fragment"),compilation_options:Default::default(),targets:&[Some(wgpu::ColorTargetState {
                format:crate::frame::HDR_FORMAT,blend:Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),write_mask:wgpu::ColorWrites::ALL,
            })]}),
            primitive:wgpu::PrimitiveState{cull_mode:None,..Default::default()},
            depth_stencil:Some(wgpu::DepthStencilState{format:wgpu::TextureFormat::Depth32Float,depth_write_enabled:Some(false),depth_compare:Some(wgpu::CompareFunction::LessEqual),stencil:Default::default(),bias:Default::default()}),
            multisample:Default::default(),multiview_mask:None,cache:None,
        });
        Self {
            globals,
            textures,
            pipeline,
            cache: Mutex::new(HashMap::new()),
        }
    }
    pub fn upload(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        mut plans: Vec<SpritePlan>,
    ) -> Result<Option<Arc<SpriteScene>>> {
        if plans.is_empty() {
            return Ok(None);
        }
        if plans.len() > 32768 {
            return Err(ResourceError::BufferSize("32768 sprite instances").into());
        }
        plans.sort_by(|a, b| {
            (a.sprite.layer, a.sprite.order, &a.id).cmp(&(b.sprite.layer, b.sprite.order, &b.id))
        });
        let mut cache = self
            .cache
            .lock()
            .map_err(|_| ResourceError::CacheLock("sprite texture"))?;
        cache.retain(|_, v| v.strong_count() != 0);
        let mut unique = std::collections::BTreeSet::new();
        let mut texture_bytes = 0_u64;
        for plan in &plans {
            if unique.insert(&plan.source.info().fingerprint) {
                let RuntimeAssetData::Texture(source) = plan.source.data() else {
                    return Err(ResourceError::TextureSize.into());
                };
                texture_bytes +=
                    u64::from(source.texture.width) * u64::from(source.texture.height) * 8;
                if texture_bytes > 256 * 1024 * 1024 {
                    return Err(ResourceError::BufferSize("256 MiB sprite texture set").into());
                }
            }
        }
        let mut vertices = Vec::new();
        let mut batches: Vec<Batch> = Vec::new();
        for plan in plans {
            let RuntimeAssetData::Texture(source) = plan.source.data() else {
                return Err(ResourceError::TextureSize.into());
            };
            let key = plan.source.info().fingerprint.clone();
            let texture = match cache.get(&key).and_then(Weak::upgrade) {
                Some(texture) => texture,
                None => {
                    let texture = Arc::new(texture::upload(
                        device,
                        queue,
                        &self.textures,
                        Arc::clone(&plan.source),
                    )?);
                    cache.insert(key, Arc::downgrade(&texture));
                    texture
                }
            };
            let sprite = &plan.sprite;
            let start = vertices.len() as u32;
            let [x, y, w, h] = sprite
                .region
                .pixels()
                .ok_or(incant_2d::SpriteError::UnresolvedAtlas)?;
            let tw = source.texture.width as f64;
            let th = source.texture.height as f64;
            let bounds = [
                (f64::from(x) + 0.5) / tw,
                (f64::from(y) + 0.5) / th,
                (f64::from(x + w) - 0.5) / tw,
                (f64::from(y + h) - 0.5) / th,
            ]
            .map(|v| v as f32);
            for local in
                incant_2d::tessellate(sprite, [source.texture.width, source.texture.height])?
            {
                let position = plan
                    .world
                    .transform_point3(glam::DVec3::new(local.position[0], local.position[1], 0.))
                    .as_vec3();
                if !position.is_finite() {
                    return Err(ResourceError::GeometryRange.into());
                }
                vertices.push(Vertex {
                    position: position.to_array(),
                    uv: local.uv.map(|v| v as f32),
                    tint: sprite.tint.map(|v| v as f32),
                    uv_bounds: bounds,
                });
            }
            let count = vertices.len() as u32 - start;
            let sampling = usize::from(sprite.sampling == SpriteSampling::Linear);
            if let Some(last) = batches
                .last_mut()
                .filter(|last| Arc::ptr_eq(&last.texture, &texture) && last.sampling == sampling)
            {
                last.count += count;
            } else {
                batches.push(Batch {
                    texture,
                    sampling,
                    start,
                    count,
                });
            }
        }
        let bytes = bytemuck::cast_slice(&vertices);
        if bytes.len() as u64 > device.limits().max_buffer_size || bytes.len() > 128 * 1024 * 1024 {
            return Err(ResourceError::BufferSize("sprite geometry").into());
        }
        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sorted sprite vertices"),
            contents: bytes,
            usage: wgpu::BufferUsages::VERTEX,
        });
        Ok(Some(Arc::new(SpriteScene { vertices, batches })))
    }
}
