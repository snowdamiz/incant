//! GPU resources for typed glTF materials. Bindings are immutable and retained
//! with each model version. All images and samplers preserve their cooked roles.
use crate::{ResourceError, Result};
use incant_assets::{AlphaMode, CookedModel, Material, ModelTexture, TextureFormat};
use std::{collections::HashMap, sync::Mutex};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Parameters {
    base: [f32; 4],
    emissive_roughness: [f32; 4],
    factors: [f32; 4],
    flags: [u32; 4],
}
pub(crate) struct GpuMaterial {
    pub bind_group: wgpu::BindGroup,
    pub alpha: AlphaMode,
    pub double_sided: bool,
    _uniform: wgpu::Buffer,
}
pub(crate) struct MaterialResources {
    pub materials: Vec<GpuMaterial>,
    _images: Vec<wgpu::Texture>,
    _samplers: Vec<wgpu::Sampler>,
}
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub(crate) struct PipelineKey {
    pub format: wgpu::TextureFormat,
    pub alpha: AlphaMode,
    pub double_sided: bool,
    pub mirrored: bool,
}
pub(crate) struct MaterialSystem {
    pub globals: wgpu::BindGroupLayout,
    layout: wgpu::BindGroupLayout,
    fallback: wgpu::TextureView,
    sampler: wgpu::Sampler,
    pipelines: Mutex<HashMap<PipelineKey, wgpu::RenderPipeline>>,
}
fn uniform_entry(binding: u32, visibility: wgpu::ShaderStages) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}
impl MaterialSystem {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let globals = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Material frame"),
            entries: &[uniform_entry(0, wgpu::ShaderStages::VERTEX_FRAGMENT)],
        });
        let mut entries = vec![uniform_entry(0, wgpu::ShaderStages::FRAGMENT)];
        for slot in 0..5 {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: slot * 2 + 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            });
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: slot * 2 + 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            });
        }
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("glTF material maps"),
            entries: &entries,
        });
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Material white fallback"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &[255; 4],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        Self {
            globals,
            layout,
            fallback: texture.create_view(&Default::default()),
            sampler: device.create_sampler(&Default::default()),
            pipelines: Mutex::new(HashMap::new()),
        }
    }

    pub fn upload(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        model: &CookedModel,
    ) -> Result<MaterialResources> {
        let mut images = Vec::new();
        for source in &model.images {
            source.validate()?;
            if source.width > device.limits().max_texture_dimension_2d
                || source.height > device.limits().max_texture_dimension_2d
            {
                return Err(ResourceError::TextureSize.into());
            }
            let (format, stride) = match source.format {
                TextureFormat::Rgba8Srgb => (wgpu::TextureFormat::Rgba8UnormSrgb, 4),
                TextureFormat::Rgba8Linear => (wgpu::TextureFormat::Rgba8Unorm, 4),
                TextureFormat::Rgba32Float => (wgpu::TextureFormat::Rgba16Float, 8),
            };
            let image = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Cooked glTF image"),
                size: wgpu::Extent3d {
                    width: source.width,
                    height: source.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: source.levels.len() as u32,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            for (mip, bytes) in source.levels.iter().enumerate() {
                let converted;
                let bytes = if source.format == TextureFormat::Rgba32Float {
                    let mut out = Vec::with_capacity(bytes.len() / 2);
                    for word in bytes.as_chunks::<4>().0 {
                        let value = half::f16::from_f32(f32::from_le_bytes(*word));
                        if !value.is_finite() {
                            return Err(ResourceError::TextureRange.into());
                        }
                        out.extend_from_slice(&value.to_bits().to_le_bytes());
                    }
                    converted = out;
                    converted.as_slice()
                } else {
                    bytes.as_slice()
                };
                let width = (source.width >> mip).max(1);
                let height = (source.height >> mip).max(1);
                queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &image,
                        mip_level: mip as u32,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    bytes,
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(width * stride),
                        rows_per_image: Some(height),
                    },
                    wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                );
            }
            images.push(image);
        }
        let views: Vec<_> = images
            .iter()
            .map(|i| i.create_view(&Default::default()))
            .collect();
        let samplers: Vec<_> = model
            .metadata
            .textures
            .iter()
            .map(|t| device.create_sampler(&sampler(t)))
            .collect();
        let mut materials = Vec::new();
        for material in model
            .materials
            .iter()
            .chain(std::iter::once(&Material::default()))
        {
            let params = Parameters {
                base: material.base_color,
                emissive_roughness: [
                    material.emissive[0],
                    material.emissive[1],
                    material.emissive[2],
                    material.roughness,
                ],
                factors: [
                    material.metallic,
                    material.normal_scale,
                    material.occlusion_strength,
                    material.alpha_cutoff,
                ],
                flags: [
                    match material.alpha_mode {
                        AlphaMode::Opaque => 0,
                        AlphaMode::Mask => 1,
                        AlphaMode::Blend => 2,
                    },
                    u32::from(material.maps[2].is_some()),
                    u32::from(material.double_sided),
                    0,
                ],
            };
            let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("glTF material factors"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });
            let mut bindings = vec![wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }];
            for (slot, map) in material.maps.iter().enumerate() {
                let (image, sampler) = if let Some(map) = map {
                    (&views[map.image], &samplers[map.texture])
                } else {
                    (&self.fallback, &self.sampler)
                };
                bindings.push(wgpu::BindGroupEntry {
                    binding: slot as u32 * 2 + 1,
                    resource: wgpu::BindingResource::TextureView(image),
                });
                bindings.push(wgpu::BindGroupEntry {
                    binding: slot as u32 * 2 + 2,
                    resource: wgpu::BindingResource::Sampler(sampler),
                });
            }
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("glTF material"),
                layout: &self.layout,
                entries: &bindings,
            });
            materials.push(GpuMaterial {
                bind_group,
                alpha: material.alpha_mode,
                double_sided: material.double_sided,
                _uniform: uniform,
            });
        }
        Ok(MaterialResources {
            materials,
            _images: images,
            _samplers: samplers,
        })
    }

    pub fn pipeline(
        &self,
        device: &wgpu::Device,
        key: PipelineKey,
    ) -> Result<wgpu::RenderPipeline> {
        let mut cache = self
            .pipelines
            .lock()
            .map_err(|_| ResourceError::CacheLock("material pipeline"))?;
        Ok(cache
            .entry(key)
            .or_insert_with(|| {
                crate::material_pipeline::build(device, key, &self.globals, &self.layout)
            })
            .clone())
    }
}

fn sampler(source: &ModelTexture) -> wgpu::SamplerDescriptor<'static> {
    let wrap = |v| match v {
        33071 => wgpu::AddressMode::ClampToEdge,
        33648 => wgpu::AddressMode::MirrorRepeat,
        _ => wgpu::AddressMode::Repeat,
    };
    let min = source.min_filter.unwrap_or(9987);
    wgpu::SamplerDescriptor {
        label: Some("glTF sampler"),
        address_mode_u: wrap(source.wrap_s),
        address_mode_v: wrap(source.wrap_t),
        mag_filter: if source.mag_filter == Some(9728) {
            wgpu::FilterMode::Nearest
        } else {
            wgpu::FilterMode::Linear
        },
        min_filter: if [9728, 9984, 9986].contains(&min) {
            wgpu::FilterMode::Nearest
        } else {
            wgpu::FilterMode::Linear
        },
        mipmap_filter: if [9986, 9987].contains(&min) {
            wgpu::MipmapFilterMode::Linear
        } else {
            wgpu::MipmapFilterMode::Nearest
        },
        lod_max_clamp: if [9728, 9729].contains(&min) { 0. } else { 32. },
        ..Default::default()
    }
}
