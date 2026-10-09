//! Distant image lighting. Scene versions retain immutable resources across
//! reimports; the cache cannot bypass AssetStore's current grant/fingerprint.
mod cube;
mod filter;
mod source;
#[cfg(test)]
mod tests;
use crate::{ResourceError, Result};
use filter::{Filtered, LUT_SIZE};
pub(crate) const MAX_SPECULAR_LOD: f32 = filter::SPECULAR_SIZE.ilog2() as f32;
use incant_assets::{RuntimeAsset, RuntimeAssetData};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock, Weak},
};

pub(crate) struct EnvironmentPlan {
    pub source: Arc<RuntimeAsset>,
    pub intensity: f32,
    pub rotation: f32,
}
pub(crate) struct Binding {
    pub resource: Arc<GpuEnvironment>,
    pub intensity: f32,
    pub rotation: f32,
}
pub(crate) struct GpuEnvironment {
    pub group: wgpu::BindGroup,
    _source: Option<Arc<RuntimeAsset>>,
}
pub(crate) struct EnvironmentSystem {
    pub layout: wgpu::BindGroupLayout,
    lut: wgpu::TextureView,
    sampler: wgpu::Sampler,
    default: OnceLock<Arc<GpuEnvironment>>,
    cache: Mutex<HashMap<String, Weak<GpuEnvironment>>>,
}
impl EnvironmentSystem {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let mut entries = Vec::new();
        for binding in 0..3 {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: if binding == 2 {
                        wgpu::TextureViewDimension::D2
                    } else {
                        wgpu::TextureViewDimension::Cube
                    },
                    multisampled: false,
                },
                count: None,
            });
        }
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: 3,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Environment lighting"),
            entries: &entries,
        });
        // The integration is deterministic and independent of device/project.
        static LUT: OnceLock<Vec<glam::Vec2>> = OnceLock::new();
        let lut = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Correlated Smith GGX DFG"),
            size: wgpu::Extent3d {
                width: LUT_SIZE,
                height: LUT_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rg16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let bytes: Vec<_> = LUT
            .get_or_init(filter::brdf_lut)
            .iter()
            .flat_map(|p| p.to_array())
            .flat_map(|v| half::f16::from_f32(v).to_bits().to_le_bytes())
            .collect();
        queue.write_texture(
            lut.as_image_copy(),
            &bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(LUT_SIZE * 4),
                rows_per_image: Some(LUT_SIZE),
            },
            lut.size(),
        );
        Self {
            layout,
            lut: lut.create_view(&Default::default()),
            sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("Environment trilinear"),
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::MipmapFilterMode::Linear,
                ..Default::default()
            }),
            default: OnceLock::new(),
            cache: Mutex::new(HashMap::new()),
        }
    }
    pub fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        plan: Option<EnvironmentPlan>,
    ) -> Result<Binding> {
        let Some(plan) = plan else {
            return Ok(Binding {
                resource: Arc::clone(self.default.get_or_init(|| {
                    static STUDIO: OnceLock<Filtered> = OnceLock::new();
                    let studio = STUDIO.get_or_init(|| {
                        filter::filter(cube::Cube::from_fn(filter::SPECULAR_SIZE, |d| {
                            glam::Vec3::from_array(crate::preview_environment::radiance(
                                d.to_array(),
                            ))
                        }))
                    });
                    Arc::new(self.upload(device, queue, studio, None))
                })),
                intensity: 1.,
                rotation: 0.,
            });
        };
        let mut cache = self
            .cache
            .lock()
            .map_err(|_| ResourceError::CacheLock("environment"))?;
        cache.retain(|_, value| value.strong_count() != 0);
        let key = plan.source.info().fingerprint.clone();
        let resource = if let Some(resource) = cache.get(&key).and_then(Weak::upgrade) {
            resource
        } else {
            let RuntimeAssetData::Texture(texture) = plan.source.data() else {
                unreachable!("validated environment binding")
            };
            let filtered = filter::filter(source::decode(texture)?);
            let resource = Arc::new(self.upload(device, queue, &filtered, Some(plan.source)));
            cache.insert(key, Arc::downgrade(&resource));
            resource
        };
        Ok(Binding {
            resource,
            intensity: plan.intensity,
            rotation: plan.rotation,
        })
    }
    fn upload(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        filtered: &Filtered,
        source: Option<Arc<RuntimeAsset>>,
    ) -> GpuEnvironment {
        let specular = upload_cube(device, queue, &filtered.specular);
        let diffuse = upload_cube(device, queue, std::slice::from_ref(&filtered.diffuse));
        let views = [&specular, &diffuse, &self.lut];
        let mut entries: Vec<_> = views
            .iter()
            .enumerate()
            .map(|(i, view)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: wgpu::BindingResource::TextureView(view),
            })
            .collect();
        entries.push(wgpu::BindGroupEntry {
            binding: 3,
            resource: wgpu::BindingResource::Sampler(&self.sampler),
        });
        GpuEnvironment {
            group: device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Retained environment"),
                layout: &self.layout,
                entries: &entries,
            }),
            _source: source,
        }
    }
}
fn upload_cube(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    mips: &[cube::Cube],
) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Prefiltered environment"),
        size: wgpu::Extent3d {
            width: mips[0].size,
            height: mips[0].size,
            depth_or_array_layers: 6,
        },
        mip_level_count: mips.len() as u32,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba16Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for (level, mip) in mips.iter().enumerate() {
        let bytes: Vec<_> = mip
            .pixels
            .iter()
            .flat_map(|p| p.extend(1.).to_array())
            .flat_map(|v| half::f16::from_f32(v).to_bits().to_le_bytes())
            .collect();
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: level as u32,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(mip.size * 8),
                rows_per_image: Some(mip.size),
            },
            wgpu::Extent3d {
                width: mip.size,
                height: mip.size,
                depth_or_array_layers: 6,
            },
        );
    }
    texture.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::Cube),
        ..Default::default()
    })
}
