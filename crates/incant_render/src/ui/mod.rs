//! Screen-space game UI layer, drawn over the finished display output (after
//! the tone map) in the same target. World text keeps its own world-space pass.
//!
//! Numerical contract (theme values in `theme.rs`):
//! * Space. `UiInput` is logical pixels, top-left origin, Y down. The target
//!   region (the whole target, or `RetainedTarget::viewport`) shows the logical
//!   viewport with per-axis scale `region size / viewport` physical pixels per
//!   logical pixel. The host keeps the viewport at `physical size / scale factor`.
//! * Snapping. Rectangle and clip edges round to the nearest physical pixel. A
//!   glyph's pen (content origin + shaped glyph position) rounds to the nearest
//!   physical pixel and its padded coverage rectangle follows at `scale` pixels
//!   per atlas texel, so at scale 1 texels land on pixels unresampled. Drawn
//!   glyphs therefore sit within 0.5 physical pixel of the layout's positions.
//! * Clipping. Each quad is intersected with its own clip in the vertex stage.
//!   Glyphs use `UiWidget::clip` (ancestor clips ∩ own content box) applied to
//!   their actual padded bounds, so an overflowing Word token or an RTL glyph at
//!   negative x is cut at the box edge. Surfaces, selection, caret and focus
//!   ring use `visible` (ancestor clips ∩ border box).
//! * Order. One instanced draw in canonical layout preorder. Within a widget:
//!   surface, selection, glyphs (a label's shadow pass first), composition
//!   underline, caret, focus ring.
//! * Colour. Theme colours are display-referred sRGB with straight alpha and
//!   are never tone mapped. sRGB targets decode them and blend premultiplied in
//!   linear light; UNORM targets blend the encoded values. Glyph texels are
//!   linear area coverage; minification averages an N×N (N ≤ 4) tap grid
//!   clamped to the glyph's padded texels, as the world text pass does.
//!   Filtering precision is the backend's (8-bit fixed point on some). A
//!   fragment with zero coverage is discarded, so pixels a quad does not cover
//!   keep their stored value on every backend.
//! * States. Surfaces are opaque. Disabled uses a flatter surface and muted
//!   text; focus adds an inset accent ring; a captured pointer press darkens a
//!   button. Labels sit on the scene and get a hard one-pixel shadow. The caret
//!   and selection draw only in a focused, enabled field, from `UiEdit` byte
//!   ranges mapped through the shaped clusters; nothing is inferred from text.
//!
//! Retention. A stage reuses the accepted resources when the `Arc<UiInput>` is
//! identical, and the uploaded atlas prefix when only the atlas Arc is shared.
//! Changed input uploads one instance buffer. The frame uniform is rewritten only
//! when the target region changes. A refused stage never touches accepted
//! resources, which stay drawable.
mod geometry;
pub mod theme;
use crate::{Renderer, ResourceError, Result, RetainedTarget, SceneError};
pub use geometry::{MAX_UI_GLYPHS, MAX_UI_QUADS, MAX_UI_WIDGETS, UiCaret, ui_caret};
use geometry::{QUAD_BYTES, UiPlan, plan};
#[cfg(test)]
use geometry::{caret, spans_for};
use incant_render_data::UiInput;
use incant_text::GlyphAtlas;
use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
use wgpu::util::DeviceExt;

/// Renderer-lifetime upload counts; an unchanged UI frame adds to none.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UiUploads {
    pub atlas_textures: u64,
    pub atlas_bytes: u64,
    pub quad_buffers: u64,
    pub quad_bytes: u64,
    pub frame_writes: u64,
}
/// The accepted UI frame's resources, for retention and resource checks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UiStats {
    pub widgets: usize,
    pub glyph_quads: usize,
    pub quads: usize,
    pub missing_glyphs: usize,
    /// Atlas prefix texels plus instance bytes; the frame uniform is excluded.
    pub gpu_bytes: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
struct FrameUniform {
    region: [f32; 4],
    scale: [f32; 2],
    surface: [f32; 2],
    encode: [f32; 4],
}

struct GpuAtlas {
    source: Option<Arc<GlyphAtlas>>,
    extent: [u32; 2],
    _texture: wgpu::Texture,
    group: wgpu::BindGroup,
}
/// Immutable for its lifetime; holding the input keeps every handle valid and
/// the Arc identities unique.
struct UiGeometry {
    input: Arc<UiInput>,
    atlas: Arc<GpuAtlas>,
    quads: Option<wgpu::Buffer>,
    stats: UiStats,
}

pub(crate) struct UiSystem {
    frame: wgpu::BindGroupLayout,
    atlas: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    pipelines: HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>,
    empty: Arc<GpuAtlas>,
    atlas_textures: AtomicU64,
    atlas_bytes: AtomicU64,
    quad_buffers: AtomicU64,
    quad_bytes: AtomicU64,
    frame_writes: AtomicU64,
}
impl UiSystem {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let frame = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("UI frame region"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: std::num::NonZeroU64::new(size_of::<FrameUniform>() as u64),
                },
                count: None,
            }],
        });
        let atlas = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("UI coverage atlas"),
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
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("UI coverage bilinear"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Screen-space UI"),
            bind_group_layouts: &[Some(&frame), Some(&atlas)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Screen-space UI placement, coverage and colour"),
            source: wgpu::ShaderSource::Wgsl(include_str!("ui.wgsl").into()),
        });
        // The same fixed output catalogue as the display transform.
        let pipelines = [
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            wgpu::TextureFormat::Bgra8Unorm,
            wgpu::TextureFormat::Bgra8UnormSrgb,
        ]
        .into_iter()
        .map(|format| (format, pipeline(device, &layout, &shader, format)))
        .collect();
        let empty = upload_texture(device, queue, &atlas, &sampler, None, [1, 1]);
        Self {
            frame,
            atlas,
            sampler,
            pipelines,
            empty: Arc::new(empty),
            atlas_textures: AtomicU64::new(0),
            atlas_bytes: AtomicU64::new(0),
            quad_buffers: AtomicU64::new(0),
            quad_bytes: AtomicU64::new(0),
            frame_writes: AtomicU64::new(0),
        }
    }
    pub fn uploads(&self) -> UiUploads {
        UiUploads {
            atlas_textures: self.atlas_textures.load(Ordering::Relaxed),
            atlas_bytes: self.atlas_bytes.load(Ordering::Relaxed),
            quad_buffers: self.quad_buffers.load(Ordering::Relaxed),
            quad_bytes: self.quad_bytes.load(Ordering::Relaxed),
            frame_writes: self.frame_writes.load(Ordering::Relaxed),
        }
    }
}

fn pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Screen-space UI over display output"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vertex"),
            compilation_options: Default::default(),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: QUAD_BYTES,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &wgpu::vertex_attr_array![
                    0 => Float32x4, 1 => Float32x4, 2 => Float32x4,
                    3 => Float32x2, 4 => Unorm8x4, 5 => Uint32
                ],
            }],
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fragment"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    })
}

/// Uploads only the referenced atlas prefix; texel coordinates stay absolute.
fn upload_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    source: Option<&Arc<GlyphAtlas>>,
    extent: [u32; 2],
) -> GpuAtlas {
    let size = wgpu::Extent3d {
        width: extent[0],
        height: extent[1],
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("UI coverage atlas"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    match source {
        Some(atlas) => {
            let [width, height] = atlas.size();
            queue.write_texture(
                texture.as_image_copy(),
                atlas.pixels(),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(width),
                    rows_per_image: Some(height),
                },
                size,
            );
        }
        None => queue.write_texture(
            texture.as_image_copy(),
            &[0],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(1),
                rows_per_image: Some(1),
            },
            size,
        ),
    }
    let view = texture.create_view(&Default::default());
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("UI coverage atlas"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    });
    GpuAtlas {
        source: source.cloned(),
        extent,
        _texture: texture,
        group,
    }
}

/// Retained UI GPU state owned by one native consumer.
pub struct UiLayer {
    owner: Arc<()>,
    accepted: Option<Arc<UiGeometry>>,
    frame: wgpu::Buffer,
    frame_group: wgpu::BindGroup,
    written: Option<FrameUniform>,
}
/// A validated, uploaded candidate. Dropping it leaves the accepted UI intact.
pub struct UiStage {
    owner: Arc<()>,
    geometry: Arc<UiGeometry>,
}
impl UiStage {
    pub fn stats(&self) -> UiStats {
        self.geometry.stats
    }
}

impl Renderer {
    /// Allocates the layer's retained frame uniform; nothing is drawn yet.
    pub fn ui_layer(&self) -> UiLayer {
        let frame = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("UI frame region"),
            size: size_of::<FrameUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let frame_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("UI frame region"),
            layout: &self.ui.frame,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: frame.as_entire_binding(),
            }],
        });
        UiLayer {
            owner: Arc::clone(&self.identity),
            accepted: None,
            frame,
            frame_group,
            written: None,
        }
    }
    /// Renderer-lifetime UI upload counts, for retention checks.
    pub fn ui_uploads(&self) -> UiUploads {
        self.ui.uploads()
    }
}

impl UiLayer {
    pub fn stats(&self) -> Option<UiStats> {
        self.accepted.as_ref().map(|g| g.stats)
    }
    #[cfg(test)]
    fn accepted_atlas(&self) -> Option<std::sync::Weak<GpuAtlas>> {
        self.accepted.as_ref().map(|g| Arc::downgrade(&g.atlas))
    }
    /// Admission: every validation, count and device limit precedes any GPU
    /// allocation, then changed resources upload into new objects. The accepted
    /// resources are never modified here; an error leaves them drawable.
    pub fn stage(&self, renderer: &Renderer, input: &Arc<UiInput>) -> Result<UiStage> {
        if !Arc::ptr_eq(&self.owner, &renderer.identity) {
            return Err(SceneError::Input("UI layer belongs to another renderer").into());
        }
        if let Some(accepted) = &self.accepted
            && Arc::ptr_eq(&accepted.input, input)
        {
            return Ok(UiStage {
                owner: Arc::clone(&self.owner),
                geometry: Arc::clone(accepted),
            });
        }
        let UiPlan {
            quads,
            extent,
            glyphs,
            missing,
        } = plan(input)?;
        let limits = renderer.device.limits();
        let quad_bytes = quads.len() as u64 * QUAD_BYTES;
        if quad_bytes > limits.max_buffer_size {
            return Err(ResourceError::BufferSize("UI quads").into());
        }
        if extent.iter().any(|&d| d > limits.max_texture_dimension_2d) {
            return Err(ResourceError::TextureSize.into());
        }
        let system = &renderer.ui;
        let reusable = self.accepted.as_ref().map(|g| &g.atlas).filter(|a| {
            a.source
                .as_ref()
                .is_some_and(|s| Arc::ptr_eq(s, &input.atlas))
                && a.extent[0] >= extent[0]
                && a.extent[1] >= extent[1]
        });
        let atlas = match reusable {
            Some(atlas) => Arc::clone(atlas),
            None if glyphs == 0 => Arc::clone(&system.empty),
            None => {
                system.atlas_textures.fetch_add(1, Ordering::Relaxed);
                system.atlas_bytes.fetch_add(
                    u64::from(extent[0]) * u64::from(extent[1]),
                    Ordering::Relaxed,
                );
                Arc::new(upload_texture(
                    &renderer.device,
                    &renderer.queue,
                    &system.atlas,
                    &system.sampler,
                    Some(&input.atlas),
                    extent,
                ))
            }
        };
        let buffer = (!quads.is_empty()).then(|| {
            system.quad_buffers.fetch_add(1, Ordering::Relaxed);
            system.quad_bytes.fetch_add(quad_bytes, Ordering::Relaxed);
            renderer
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Immutable UI quads"),
                    contents: bytemuck::cast_slice(&quads),
                    usage: wgpu::BufferUsages::VERTEX,
                })
        });
        let atlas_bytes = if atlas.source.is_some() {
            u64::from(atlas.extent[0]) * u64::from(atlas.extent[1])
        } else {
            0
        };
        Ok(UiStage {
            owner: Arc::clone(&self.owner),
            geometry: Arc::new(UiGeometry {
                stats: UiStats {
                    widgets: input.widgets.len(),
                    glyph_quads: glyphs,
                    quads: quads.len(),
                    missing_glyphs: missing,
                    gpu_bytes: atlas_bytes + quad_bytes,
                },
                input: Arc::clone(input),
                atlas,
                quads: buffer,
            }),
        })
    }
    /// Accept a staged frame without drawing it (a viewport-only relayout that
    /// the next presented frame will show).
    pub fn accept(&mut self, stage: UiStage) -> Result<()> {
        if !Arc::ptr_eq(&self.owner, &stage.owner) {
            return Err(SceneError::Input("UI stage belongs to another layer").into());
        }
        self.accepted = Some(stage.geometry);
        Ok(())
    }
    /// Draw a staged frame over the target's finished display output, submit it
    /// on the renderer's queue, then accept it. Call after the world frame.
    pub fn submit(
        &mut self,
        renderer: &Renderer,
        stage: UiStage,
        target: &RetainedTarget<'_>,
    ) -> Result<wgpu::SubmissionIndex> {
        if !Arc::ptr_eq(&self.owner, &renderer.identity) || !Arc::ptr_eq(&self.owner, &stage.owner)
        {
            return Err(SceneError::Input("UI stage belongs to another renderer").into());
        }
        let pipeline = renderer
            .ui
            .pipelines
            .get(&target.format)
            .ok_or(ResourceError::OutputFormat(target.format))?;
        let geometry = &stage.geometry;
        let region = target
            .viewport
            .map_or([0., 0., target.width as f32, target.height as f32], |v| {
                v.rect
            });
        if target.width == 0
            || target.height == 0
            || region.iter().any(|v| !v.is_finite())
            || region[2] <= 0.
            || region[3] <= 0.
        {
            return Err(SceneError::Input("UI target region").into());
        }
        let viewport = geometry.input.viewport;
        let uniform = FrameUniform {
            region,
            scale: [region[2] / viewport[0], region[3] / viewport[1]],
            surface: [target.width as f32, target.height as f32],
            encode: [if target.format.is_srgb() { 0. } else { 1. }, 0., 0., 0.],
        };
        if self.written != Some(uniform) {
            renderer
                .queue
                .write_buffer(&self.frame, 0, bytemuck::bytes_of(&uniform));
            renderer.ui.frame_writes.fetch_add(1, Ordering::Relaxed);
            self.written = Some(uniform);
        }
        let mut encoder = renderer
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Screen-space UI"),
            });
        if let Some(quads) = &geometry.quads {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Screen-space UI over display output"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target.view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &self.frame_group, &[]);
            pass.set_bind_group(1, &geometry.atlas.group, &[]);
            pass.set_vertex_buffer(0, quads.slice(..));
            pass.draw(0..6, 0..geometry.stats.quads as u32);
        }
        let submission = renderer.queue.submit([encoder.finish()]);
        self.accepted = Some(stage.geometry);
        Ok(submission)
    }
}

#[cfg(test)]
#[path = "../ui_tests.rs"]
pub(crate) mod tests;

#[cfg(test)]
#[path = "../ui_draw_tests.rs"]
pub(crate) mod draw_tests;
