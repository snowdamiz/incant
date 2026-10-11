//! Immutable sprite resources, deterministic painter order and retained frame commands.
//! Geometry, premultiplied linear color and clip/depth math; no style presets.
mod drawing;
mod geometry;
mod plans;
pub(crate) use plans::{SpritePlan, TilemapPlan};
mod texture;
use crate::{ResourceError, Result};
pub(crate) use drawing::SpriteDraw;
use incant_2d::SpriteSampling;
use incant_assets_runtime::{RuntimeAsset, RuntimeAssetData};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, Weak},
};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Attributes {
    uv: [f32; 2],
    tint: [f32; 4],
    uv_bounds: [f32; 4],
    texel: [f32; 2],
    sampling: u32,
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
    selection: Option<(usize, u16)>,
}
pub(crate) struct SpriteScene {
    positions: wgpu::Buffer,
    geometry: Arc<SpriteGeometry>,
    updates: Vec<[f32; 3]>,
    active_frames: Vec<u16>,
}
struct SpriteGeometry {
    attributes: wgpu::Buffer,
    batches: Vec<Batch>,
    placements: Vec<Placement>,
}
struct Placement {
    node: usize,
    start: usize,
    local: Vec<[f64; 2]>,
    selection: Option<(usize, u16)>,
}
impl SpriteScene {
    /// Share frozen geometry and keep only mutable positions private to each
    /// accepted/staged frame. No second sort, tessellation or texture upload.
    pub(crate) fn staging(&self, device: &wgpu::Device) -> Self {
        debug_assert!(!self.updates.is_empty());
        Self {
            positions: position_buffer(device, &self.updates, true),
            geometry: Arc::clone(&self.geometry),
            updates: self.updates.clone(),
            active_frames: self.active_frames.clone(),
        }
    }
    pub(crate) fn stage(&mut self, input: &incant_render_data::SceneInput) -> Result<()> {
        for (index, frame) in self.active_frames.iter_mut().enumerate() {
            *frame = input.nodes[index].sprite.as_ref().map_or(0, |s| s.frame);
        }
        for placement in &self.geometry.placements {
            if placement
                .selection
                .is_some_and(|(node, frame)| self.active_frames[node] != frame)
            {
                continue;
            }

            let world = input.nodes[placement.node].world;
            for (local, vertex) in placement
                .local
                .iter()
                .zip(&mut self.updates[placement.start..])
            {
                let position = world
                    .transform_point3(glam::DVec3::new(local[0], local[1], 0.))
                    .as_vec3();
                if !position.is_finite() {
                    return Err(ResourceError::GeometryRange.into());
                }
                *vertex = position.to_array();
            }
        }
        Ok(())
    }
    pub(crate) fn write(&self, queue: &wgpu::Queue) {
        if self.active_frames.is_empty() {
            if !self.updates.is_empty() {
                queue.write_buffer(&self.positions, 0, bytemuck::cast_slice(&self.updates));
            }
        } else {
            for span in active_spans(&self.geometry.placements, &self.active_frames) {
                queue.write_buffer(
                    &self.positions,
                    (span.start * size_of::<[f32; 3]>()) as u64,
                    bytemuck::cast_slice(&self.updates[span]),
                );
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn retained_storage(&self, other: &Self) -> (usize, usize, usize) {
        // Actual buffer sizes and allocated vector capacities, excluding batch/
        // placement metadata, textures and driver-side staging allocations.
        let geometry_bytes = |geometry: &SpriteGeometry| {
            geometry.attributes.size() as usize
                + geometry
                    .placements
                    .iter()
                    .map(|p| p.local.capacity() * 16)
                    .sum::<usize>()
        };
        let shared = geometry_bytes(&self.geometry)
            + if Arc::ptr_eq(&self.geometry, &other.geometry) {
                0
            } else {
                geometry_bytes(&other.geometry)
            };
        let mutable = self.positions.size() as usize
            + other.positions.size() as usize
            + (self.updates.capacity() + other.updates.capacity()) * size_of::<[f32; 3]>();
        (
            self.updates.len(),
            shared + mutable,
            self.updates.len() * size_of::<[f32; 3]>(),
        )
    }
}
fn position_buffer(device: &wgpu::Device, positions: &[[f32; 3]], retained: bool) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Sorted sprite positions"),
        contents: bytemuck::cast_slice(positions),
        usage: wgpu::BufferUsages::VERTEX
            | if retained {
                wgpu::BufferUsages::COPY_DST
            } else {
                wgpu::BufferUsages::empty()
            },
    })
}
pub(crate) struct SpriteSystem {
    globals: wgpu::BindGroupLayout,
    textures: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    cache: Mutex<HashMap<incant_assets_runtime::AssetVersionId, Weak<GpuTexture>>>,
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
                array_stride:size_of::<[f32;3]>() as u64,step_mode:wgpu::VertexStepMode::Vertex,
                attributes:&wgpu::vertex_attr_array![0=>Float32x3],
            }, wgpu::VertexBufferLayout {
                array_stride:size_of::<Attributes>() as u64,step_mode:wgpu::VertexStepMode::Vertex,
                attributes:&wgpu::vertex_attr_array![1=>Float32x2,2=>Float32x4,3=>Float32x4,4=>Float32x2,5=>Uint32],
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
        plans: Vec<SpritePlan>,
        tilemaps: Vec<TilemapPlan>,
        retained: bool,
    ) -> Result<Option<Arc<SpriteScene>>> {
        let items = plans::ordered(&plans, &tilemaps)?;
        if items.is_empty() {
            return Ok(None);
        }
        let prepared = preflight(&items, device.limits().max_buffer_size, retained)?;
        let mut active_frames = vec![
            0;
            items
                .iter()
                .filter_map(|i| i.selection().map(|(node, _)| node + 1))
                .max()
                .unwrap_or(0)
        ];
        for item in &items {
            if let Some((node, _)) = item.selection() {
                active_frames[node] = item.initial_frame();
            }
        }
        let vertex_count = prepared.vertices;
        let mut span_sizes = prepared.spans.into_iter();
        let mut cache = self
            .cache
            .lock()
            .map_err(|_| ResourceError::CacheLock("sprite texture"))?;
        cache.retain(|_, v| v.strong_count() != 0);
        let mut unique = std::collections::BTreeSet::new();
        let mut texture_bytes = 0_u64;
        for item in &items {
            if unique.insert(item.source().version_id()) {
                let RuntimeAssetData::Texture(source) = item.source().data() else {
                    return Err(ResourceError::TextureSize.into());
                };
                texture_bytes +=
                    u64::from(source.texture.width) * u64::from(source.texture.height) * 8;
                if texture_bytes > 256 * 1024 * 1024 {
                    return Err(ResourceError::BufferSize("256 MiB sprite texture set").into());
                }
            }
        }
        let mut positions = Vec::with_capacity(vertex_count);
        let mut attributes = Vec::with_capacity(vertex_count);
        let mut placements: Vec<Placement> = Vec::new();
        let mut batches: Vec<Batch> = Vec::new();
        for item in items {
            let RuntimeAssetData::Texture(source) = item.source().data() else {
                return Err(ResourceError::TextureSize.into());
            };
            let sprite = item.sprite();
            let (geometry, region, texture_size) = base_geometry(&item)?;
            let geometry = if sprite.sampling == SpriteSampling::Nearest {
                geometry::nearest(&geometry, region, texture_size)
            } else {
                geometry
                    .into_iter()
                    .map(|v| geometry::NearestVertex {
                        position: v.position,
                        texel: [0.; 2],
                        base: v.uv,
                    })
                    .collect()
            };
            debug_assert!((positions.len() + geometry.len()) <= vertex_count);
            let key = item.source().version_id();
            let texture = match cache.get(&key).and_then(Weak::upgrade) {
                Some(texture) => texture,
                None => {
                    let texture = Arc::new(texture::upload(
                        device,
                        queue,
                        &self.textures,
                        Arc::clone(item.source()),
                    )?);
                    cache.insert(key, Arc::downgrade(&texture));
                    texture
                }
            };
            let start = positions.len() as u32;
            let [x, y, w, h] = region;
            let tw = source.texture.width as f64;
            let th = source.texture.height as f64;
            let bounds = if sprite.sampling == SpriteSampling::Linear {
                [
                    (f64::from(x) + 0.5) / tw,
                    (f64::from(y) + 0.5) / th,
                    (f64::from(x + w) - 0.5) / tw,
                    (f64::from(y + h) - 0.5) / th,
                ]
            } else {
                [f64::from(x), f64::from(y), f64::from(w), f64::from(h)]
            }
            .map(|v| v as f32);
            let offset = item.offset();
            if retained {
                if let Some(last) = placements
                    .last_mut()
                    .filter(|p| p.node == item.node() && p.selection == item.selection())
                {
                    last.local.extend(
                        geometry
                            .iter()
                            .map(|v| [v.position[0] + offset[0], v.position[1] + offset[1]]),
                    );
                } else {
                    placements.push(Placement {
                        node: item.node(),
                        selection: item.selection(),
                        start: positions.len(),
                        local: {
                            let count = span_sizes.next().expect("preflight placement count");
                            let mut local = Vec::with_capacity(count);
                            local.extend(
                                geometry.iter().map(|v| {
                                    [v.position[0] + offset[0], v.position[1] + offset[1]]
                                }),
                            );
                            local
                        },
                    });
                }
            }
            for local in geometry {
                let position = item
                    .world()
                    .transform_point3(glam::DVec3::new(
                        local.position[0] + offset[0],
                        local.position[1] + offset[1],
                        0.,
                    ))
                    .as_vec3();
                if !position.is_finite() {
                    return Err(ResourceError::GeometryRange.into());
                }
                positions.push(position.to_array());
                attributes.push(Attributes {
                    // Nearest uses this slot as a flat integer source base;
                    // linear retains its original normalized texture UVs.
                    uv: local.base.map(|v| v as f32),
                    tint: sprite.tint.map(|v| v as f32),
                    uv_bounds: bounds,
                    texel: local.texel.map(|v| v as f32),
                    sampling: u32::from(sprite.sampling == SpriteSampling::Linear),
                });
            }
            let count = positions.len() as u32 - start;
            let sampling = usize::from(sprite.sampling == SpriteSampling::Linear);
            if let Some(last) = batches.last_mut().filter(|last| {
                Arc::ptr_eq(&last.texture, &texture)
                    && last.sampling == sampling
                    && last.selection == item.selection()
            }) {
                last.count += count;
            } else {
                batches.push(Batch {
                    texture,
                    sampling,
                    start,
                    count,
                    selection: item.selection(),
                });
            }
        }
        debug_assert_eq!(positions.len(), vertex_count);
        debug_assert!(span_sizes.next().is_none());
        let attributes = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Immutable sprite attributes"),
            contents: bytemuck::cast_slice(&attributes),
            usage: wgpu::BufferUsages::VERTEX,
        });
        Ok(Some(Arc::new(SpriteScene {
            positions: position_buffer(device, &positions, retained),
            geometry: Arc::new(SpriteGeometry {
                attributes,
                batches,
                placements,
            }),
            updates: if retained { positions } else { Vec::new() },
            active_frames,
        })))
    }
}

fn base_geometry(
    item: &plans::DrawItem<'_>,
) -> Result<(Vec<incant_2d::SpriteVertex>, [u32; 4], [u32; 2])> {
    let RuntimeAssetData::Texture(texture) = item.source().data() else {
        return Err(ResourceError::TextureSize.into());
    };
    let sprite = item.sprite();
    let region = sprite
        .region
        .pixels()
        .ok_or(incant_2d::SpriteError::UnresolvedAtlas)?;
    let size = [texture.texture.width, texture.texture.height];
    let geometry =
        incant_2d::tessellate(&sprite, size).map_err(|source| ResourceError::SpriteGeometry {
            entity: item.id().to_owned(),
            texture: item.source().info().id.clone(),
            region,
            texture_size: size,
            source,
        })?;
    Ok((geometry, region, size))
}

struct GeometrySize {
    vertices: usize,
    spans: Vec<usize>,
}
fn preflight(
    items: &[plans::DrawItem<'_>],
    buffer_limit: u64,
    retained: bool,
) -> Result<GeometrySize> {
    // Exact combined count precedes any expanded vertex buffer or texture.
    // Per-item tessellation scratch never exceeds 54 vertices.
    let mut result = GeometrySize {
        vertices: 0,
        spans: Vec::new(),
    };
    let mut last_node = None;
    for item in items {
        let (geometry, region, size) = base_geometry(item)?;
        let count = if item.sprite().sampling == SpriteSampling::Nearest {
            geometry::nearest_len(&geometry, region, size)
        } else {
            geometry.len()
        };
        result.vertices = result
            .vertices
            .checked_add(count)
            .ok_or(ResourceError::BufferSize("sprite/tile geometry"))?;
        let bytes = result
            .vertices
            .checked_mul(plans::LOGICAL_VERTEX_BYTES)
            .ok_or(ResourceError::BufferSize("sprite/tile geometry"))?;
        if bytes > plans::MAX_GEOMETRY_BYTES || bytes as u64 > buffer_limit {
            return Err(ResourceError::BufferSize("sprite/tile geometry").into());
        }
        if retained {
            if last_node == Some((item.node(), item.selection())) {
                *result.spans.last_mut().expect("prior span") += count;
            } else {
                result.spans.push(count);
                last_node = Some((item.node(), item.selection()));
            }
        }
    }
    Ok(result)
}
#[cfg(test)]
pub(crate) fn test_vertex_count(input: &incant_render_data::SceneInput) -> Result<usize> {
    let scene = crate::scene::resolve(input)?;
    let items = plans::ordered(&scene.sprites, &scene.tilemaps)?;
    Ok(preflight(&items, u64::MAX, false)?.vertices)
}

// Positions follow painter order. Skip inactive immutable frames, then coalesce
// adjacent selected ranges; a single flipbook must not turn N static entities
// into N queue writes. The iterator and its lookahead live on the stack.
fn active_spans<'a>(
    placements: &'a [Placement],
    frames: &'a [u16],
) -> impl Iterator<Item = std::ops::Range<usize>> + 'a {
    let mut active = placements
        .iter()
        .filter(|p| {
            !p.local.is_empty()
                && p.selection
                    .is_none_or(|(node, frame)| frames[node] == frame)
        })
        .peekable();
    std::iter::from_fn(move || {
        let first = active.next()?;
        let start = first.start;
        let mut end = start + first.local.len();
        while active.peek().is_some_and(|p| p.start == end) {
            let next = active.next().expect("peeked placement");
            end += next.local.len();
        }
        Some(start..end)
    })
}
#[cfg(test)]
mod upload_span_tests {
    use super::*;
    #[test]
    fn one_flipbook_preserves_coalesced_uploads_for_many_static_entities() {
        let mut placements = Vec::new();
        for i in 0..10256 {
            let selection = (5000..5256).contains(&i).then(|| (0, (i - 5000) as u16));
            placements.push(Placement {
                node: i,
                start: i * 6,
                local: vec![[0.; 2]; 6],
                selection,
            });
        }
        for frame in [0, 1, 127, 255] {
            let spans: Vec<_> = active_spans(&placements, &[frame]).collect();
            assert!(
                spans.len() <= 3,
                "{} uploads for one animation",
                spans.len()
            );
            assert_eq!(spans.iter().map(|r| r.len()).sum::<usize>(), 10001 * 6);
            let expected: Vec<_> = (0..5000 * 6)
                .chain((5000 + usize::from(frame)) * 6..(5001 + usize::from(frame)) * 6)
                .chain(5256 * 6..10256 * 6)
                .collect();
            assert_eq!(spans.into_iter().flatten().collect::<Vec<_>>(), expected);
        }
        for p in &mut placements {
            p.selection = None;
        }
        assert_eq!(
            active_spans(&placements, &[]).collect::<Vec<_>>(),
            vec![0..10256 * 6]
        );
    }
}
