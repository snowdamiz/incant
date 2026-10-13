//! Initial canonical text route: shaped glyph coverage drawn into the shared
//! linear HDR scene after sprites and before display output. This draws literal
//! shaped strings only; it is not widget layout, editable text or focus.
//!
//! Numerical contract (see `geometry.rs` constants):
//! * Placement. `TextNode.world` is used unchanged as the entity transform.
//!   Layout logical pixels map to that entity's local XY plane at
//!   `TEXT_LOGICAL_PIXELS_PER_UNIT` (100) pixels per local unit: layout (x, y),
//!   Y down, is local (x / 100, -y / 100, 0). The layout box's top-left corner is
//!   the entity origin, so line `i` has its baseline at local y = -baseline_i/100.
//!   Translation, rotation, scale and shear then apply exactly as for sprites.
//! * Glyphs. A glyph's inner coverage rectangle starts at `position + origin`
//!   (Y-down bearing from its baseline origin) with the atlas entry's texel size;
//!   the quad adds the entry's one-texel zero padding so filtered edges are
//!   complete. Empty outlines (spaces) add nothing; `.notdef` is drawn as its own
//!   outline. Shaped order, bidi placement and fallback come from the layout.
//! * Depth and clipping. Two-sided quads are depth-tested (LessEqual) against the
//!   opaque scene without writing depth, drawn in stable-ID then glyph order over
//!   sprites. There is no widget clip rectangle: every shaped line is drawn.
//! * Coverage and color. 8-bit atlas texels are linear area coverage multiplying
//!   unlit linear radiance `TEXT_COLOR` (premultiplied over). Pixels whose
//!   footprint exceeds 1.25 texels average an N×N grid (N ≤ 4) of bilinear taps
//!   clamped to the glyph's own padded rectangle; there are no mipmaps.
//!
//! Retention. Atlas textures and glyph quad buffers are immutable and shared by
//! identity: the exact atlas Arc plus every node's ID, layout Arc and handle Arc.
//! Motion writes one 256-byte transform slot per node and nothing else.
mod drawing;
mod geometry;
mod shader;
use crate::{ResourceError, Result, SceneError};
pub(crate) use drawing::TextDraw;
pub use geometry::{
    MAX_TEXT_GLYPHS, MAX_TEXT_NODES, TEXT_COLOR, TEXT_LOGICAL_PIXELS_PER_UNIT, text_layout_to_local,
};
pub(crate) use geometry::{NODE_STRIDE, QUAD_BYTES, TextPlan, plan};
use incant_render_data::TextInput;
use incant_text::{GlyphAtlas, GlyphHandle, ShapedText};
use incant_types::StableId;
use std::sync::{
    Arc, Mutex, Weak,
    atomic::{AtomicU64, Ordering},
};
use wgpu::util::DeviceExt;

/// Renderer-lifetime upload counts; unchanged retained text adds to neither.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TextUploads {
    pub atlas_textures: u64,
    pub glyph_buffers: u64,
}
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct NodeUniform {
    world: [[f32; 4]; 4],
    _padding: [[f32; 4]; 12],
}
const _: () = assert!(size_of::<NodeUniform>() as u64 == NODE_STRIDE);
impl NodeUniform {
    fn new(world: [[f32; 4]; 4]) -> Self {
        Self {
            world,
            _padding: [[0.; 4]; 12],
        }
    }
}
struct GpuAtlas {
    source: Arc<GlyphAtlas>,
    extent: [u32; 2],
    _texture: wgpu::Texture,
    group: wgpu::BindGroup,
}
struct NodeGeometry {
    id: StableId,
    layout: Arc<ShapedText>,
    glyphs: Arc<[GlyphHandle]>,
    start: u32,
    count: u32,
    bounds: [f64; 4],
}
/// Immutable for its lifetime. Holding the source atlas keeps every validated
/// handle valid and makes Arc pointer identity unique while this lives.
struct TextGeometry {
    source: Arc<GlyphAtlas>,
    atlas: Option<Arc<GpuAtlas>>,
    quads: Option<wgpu::Buffer>,
    nodes: Vec<NodeGeometry>,
}
impl TextGeometry {
    /// Exact retained identity: the atlas Arc and every node's ID, layout Arc
    /// and handle Arc. Equal content in another allocation is a different key.
    fn matches<'a>(
        &self,
        atlas: &Arc<GlyphAtlas>,
        mut nodes: impl ExactSizeIterator<
            Item = (StableId, &'a Arc<ShapedText>, &'a Arc<[GlyphHandle]>),
        >,
    ) -> bool {
        Arc::ptr_eq(&self.source, atlas)
            && self.nodes.len() == nodes.len()
            && self.nodes.iter().all(|a| {
                nodes.next().is_some_and(|(id, layout, glyphs)| {
                    a.id == id && Arc::ptr_eq(&a.layout, layout) && Arc::ptr_eq(&a.glyphs, glyphs)
                })
            })
    }
    fn matches_plan(&self, plan: &TextPlan) -> bool {
        self.matches(
            &plan.atlas,
            plan.nodes.iter().map(|n| (n.id, &n.layout, &n.glyphs)),
        )
    }
    fn matches_input(&self, input: &TextInput) -> bool {
        self.matches(
            &input.atlas,
            input.nodes.iter().map(|n| (n.id, &n.layout, &n.glyphs)),
        )
    }
    fn gpu_bytes(&self) -> u64 {
        self.atlas
            .as_ref()
            .map_or(0, |a| u64::from(a.extent[0]) * u64::from(a.extent[1]))
            + self.quads.as_ref().map_or(0, wgpu::Buffer::size)
    }
}
/// One scene's text: shared geometry plus private node transforms. The accepted
/// and staged retained frames each own a transform buffer and CPU rows.
pub(crate) struct TextScene {
    geometry: Arc<TextGeometry>,
    transforms: wgpu::Buffer,
    group: wgpu::BindGroup,
    updates: Vec<NodeUniform>,
}
impl TextScene {
    pub(crate) fn gpu_bytes(&self) -> u64 {
        self.geometry.gpu_bytes() + self.transforms.size()
    }
    pub(crate) fn transform_bytes(&self) -> u64 {
        self.transforms.size()
    }
    /// A second private transform buffer over the same immutable geometry.
    pub(crate) fn staging(&self, system: &TextSystem, device: &wgpu::Device) -> Self {
        system.scene(
            device,
            Arc::clone(&self.geometry),
            self.updates.clone(),
            true,
        )
    }
    /// Identity check and numeric transform copy only; never allocates.
    pub(crate) fn stage(&mut self, input: &TextInput) -> Result<()> {
        if !self.geometry.matches_input(input) {
            return Err(
                SceneError::Input("retained text bindings changed; prepare a replacement").into(),
            );
        }
        for ((node, geometry), update) in input
            .nodes
            .iter()
            .zip(&self.geometry.nodes)
            .zip(&mut self.updates)
        {
            update.world = geometry::node_matrix(node.world, geometry.bounds)?;
        }
        Ok(())
    }
    pub(crate) fn write(&self, queue: &wgpu::Queue) {
        queue.write_buffer(&self.transforms, 0, bytemuck::cast_slice(&self.updates));
    }
    #[cfg(test)]
    pub(crate) fn shares_geometry(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.geometry, &other.geometry)
    }
    #[cfg(test)]
    pub(crate) fn update_capacity(&self) -> (*const u8, usize) {
        (self.updates.as_ptr().cast(), self.updates.capacity())
    }
}

#[derive(Default)]
struct Cache {
    atlases: Vec<Weak<GpuAtlas>>,
    geometry: Vec<Weak<TextGeometry>>,
}
pub(crate) struct TextSystem {
    frame: wgpu::BindGroupLayout,
    nodes: wgpu::BindGroupLayout,
    atlas: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    pipeline: wgpu::RenderPipeline,
    cache: Mutex<Cache>,
    atlas_uploads: AtomicU64,
    glyph_uploads: AtomicU64,
}
fn uniform_entry(
    visibility: wgpu::ShaderStages,
    dynamic: bool,
    size: u64,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: dynamic,
            min_binding_size: std::num::NonZeroU64::new(size),
        },
        count: None,
    }
}

fn frame_entry() -> wgpu::BindGroupLayoutEntry {
    uniform_entry(
        wgpu::ShaderStages::VERTEX_FRAGMENT,
        false,
        size_of::<drawing::FrameUniform>() as u64,
    )
}

fn node_entry() -> wgpu::BindGroupLayoutEntry {
    uniform_entry(wgpu::ShaderStages::VERTEX, true, 64)
}

fn atlas_entries() -> [wgpu::BindGroupLayoutEntry; 2] {
    [
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
    ]
}
impl TextSystem {
    pub fn new(
        device: &wgpu::Device,
        backend: wgpu::Backend,
        cache: Option<&wgpu::PipelineCache>,
    ) -> Result<Self> {
        let frame = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Text frame camera and color"),
            entries: &[frame_entry()],
        });
        let nodes = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Text node transform slot"),
            entries: &[node_entry()],
        });
        let atlas = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Text coverage atlas"),
            entries: &atlas_entries(),
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Text coverage bilinear"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let pipeline = drawing::pipeline(device, [&frame, &nodes, &atlas], backend, cache)?;
        Ok(Self {
            frame,
            nodes,
            atlas,
            sampler,
            pipeline,
            cache: Mutex::default(),
            atlas_uploads: AtomicU64::new(0),
            glyph_uploads: AtomicU64::new(0),
        })
    }
    pub fn uploads(&self) -> TextUploads {
        TextUploads {
            atlas_textures: self.atlas_uploads.load(Ordering::Relaxed),
            glyph_buffers: self.glyph_uploads.load(Ordering::Relaxed),
        }
    }
    /// Device limits, checked before the renderer allocates any scene resource.
    pub fn preflight(&self, device: &wgpu::Device, plan: &TextPlan) -> Result<()> {
        let limits = device.limits();
        if plan.quad_bytes() > limits.max_buffer_size
            || plan.transform_bytes() > limits.max_buffer_size
        {
            return Err(ResourceError::BufferSize("text glyph quads or node transforms").into());
        }
        if plan
            .extent
            .iter()
            .any(|&d| d > limits.max_texture_dimension_2d)
        {
            return Err(ResourceError::TextureSize.into());
        }
        Ok(())
    }
    pub fn upload(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        plan: TextPlan,
        retained: bool,
    ) -> Result<Option<Arc<TextScene>>> {
        if plan.nodes.is_empty() {
            return Ok(None);
        }
        self.preflight(device, &plan)?;
        let updates = plan
            .nodes
            .iter()
            .map(|n| geometry::node_matrix(n.world, n.bounds).map(NodeUniform::new))
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let geometry = self.geometry(device, queue, &plan)?;
        Ok(Some(Arc::new(
            self.scene(device, geometry, updates, retained),
        )))
    }
    fn scene(
        &self,
        device: &wgpu::Device,
        geometry: Arc<TextGeometry>,
        updates: Vec<NodeUniform>,
        retained: bool,
    ) -> TextScene {
        let transforms = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Text node transforms"),
            contents: bytemuck::cast_slice(&updates),
            usage: wgpu::BufferUsages::UNIFORM
                | if retained {
                    wgpu::BufferUsages::COPY_DST
                } else {
                    wgpu::BufferUsages::empty()
                },
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Text node transform slot"),
            layout: &self.nodes,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &transforms,
                    offset: 0,
                    size: std::num::NonZeroU64::new(64),
                }),
            }],
        });
        TextScene {
            geometry,
            transforms,
            group,
            updates: if retained { updates } else { Vec::new() },
        }
    }
    /// Shared immutable geometry for an unchanged atlas/layout identity; a new
    /// atlas or layout uploads exactly once after the plan's counted preflight.
    fn geometry(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        plan: &TextPlan,
    ) -> Result<Arc<TextGeometry>> {
        let mut cache = self
            .cache
            .lock()
            .map_err(|_| ResourceError::CacheLock("text"))?;
        cache.geometry.retain(|g| g.strong_count() != 0);
        cache.atlases.retain(|a| a.strong_count() != 0);
        if let Some(found) = cache
            .geometry
            .iter()
            .filter_map(Weak::upgrade)
            .find(|g| g.matches_plan(plan))
        {
            return Ok(found);
        }
        let atlas = if plan.quads == 0 {
            None
        } else if let Some(found) = cache.atlases.iter().filter_map(Weak::upgrade).find(|a| {
            Arc::ptr_eq(&a.source, &plan.atlas)
                && a.extent[0] >= plan.extent[0]
                && a.extent[1] >= plan.extent[1]
        }) {
            Some(found)
        } else {
            let atlas = Arc::new(self.upload_atlas(device, queue, plan));
            cache.atlases.push(Arc::downgrade(&atlas));
            self.atlas_uploads.fetch_add(1, Ordering::Relaxed);
            Some(atlas)
        };
        let mut quads = Vec::with_capacity(plan.quads);
        let mut nodes = Vec::with_capacity(plan.nodes.len());
        for node in &plan.nodes {
            let start = quads.len() as u32;
            geometry::expand(&plan.atlas, node, &mut quads)?;
            if quads.len() as u32 - start != node.quads {
                return Err(SceneError::Input("text glyph quads changed after planning").into());
            }
            nodes.push(NodeGeometry {
                id: node.id,
                layout: Arc::clone(&node.layout),
                glyphs: Arc::clone(&node.glyphs),
                start,
                count: quads.len() as u32 - start,
                bounds: node.bounds,
            });
        }
        let quads = (!quads.is_empty()).then(|| {
            self.glyph_uploads.fetch_add(1, Ordering::Relaxed);
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Immutable text glyph quads"),
                contents: bytemuck::cast_slice(&quads),
                usage: wgpu::BufferUsages::VERTEX,
            })
        });
        let geometry = Arc::new(TextGeometry {
            source: Arc::clone(&plan.atlas),
            atlas,
            quads,
            nodes,
        });
        cache.geometry.push(Arc::downgrade(&geometry));
        Ok(geometry)
    }
    /// Uploads only the referenced atlas prefix; texel coordinates stay absolute.
    fn upload_atlas(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        plan: &TextPlan,
    ) -> GpuAtlas {
        let size = wgpu::Extent3d {
            width: plan.extent[0],
            height: plan.extent[1],
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Text coverage atlas"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let [width, height] = plan.atlas.size();
        queue.write_texture(
            texture.as_image_copy(),
            plan.atlas.pixels(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width),
                rows_per_image: Some(height),
            },
            size,
        );
        let view = texture.create_view(&Default::default());
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Text coverage atlas"),
            layout: &self.atlas,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        GpuAtlas {
            source: Arc::clone(&plan.atlas),
            extent: plan.extent,
            _texture: texture,
            group,
        }
    }
}

#[cfg(test)]
#[path = "../text_tests.rs"]
pub(crate) mod tests;

#[cfg(test)]
#[path = "../text_draw_tests.rs"]
pub(crate) mod draw_tests;
