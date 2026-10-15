//! Native rendering and GPU readback with retained imported material previews.
//! Bevy-scheduled HDR geometry, lighting, display transform and composition.
//! Authored clustered lights; the full production render graph remains open.
mod ambient_occlusion;
pub use ambient_occlusion::{AmbientOcclusion, AmbientOcclusionError};
#[cfg(feature = "ssao-stage-readback")]
pub use ambient_occlusion::{AmbientOcclusionCaptureError, AmbientOcclusionStages};
mod bloom;
pub use bloom::{Bloom, BloomError};
mod camera;
#[cfg(all(feature = "native-compilation", target_os = "macos"))]
pub mod compilation;
mod diagnostic_draw;
#[cfg(test)]
#[path = "../tests/diagnostic_output/mod.rs"]
mod diagnostic_output;
mod environment;
mod frame;
mod geometry_shader;
mod graph;
#[cfg(test)]
#[path = "../tests/hdr_output/mod.rs"]
mod hdr_output;
mod lighting;
mod material_pipeline;
mod materials;
mod models;
mod output;
mod pipeline_cache;
mod pixel_grid;
pub use pipeline_cache::{CacheRead, CacheWrite, PipelineCacheReport};
pub mod metalfx;
#[cfg(all(test, feature = "native-metalfx", target_os = "macos"))]
extern crate self as incant_render;
#[cfg(all(test, feature = "native-metalfx", target_os = "macos"))]
mod metalfx_image;
mod preview_environment;
pub mod resolution;
mod resource_error;
#[cfg(test)]
mod retained_alloc;
mod scene;
mod scene_draw;
mod scene_msaa;
pub use scene_msaa::{SceneMsaa, SceneMsaaError};
mod shader;
mod shadows;
mod sprites;
mod studio;
pub mod temporal;
mod temporal_shader;
#[cfg(test)]
#[path = "../tests/support/mod.rs"]
mod test_support;
mod text;
pub mod timing;
mod ui;
#[cfg(test)]
mod version_tests;
#[cfg(any(target_arch = "wasm32", test))]
mod web_error;
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
pub use incant_render_data::{SceneInput, ScenePreparer};
pub use lighting::LocalLightSelection;
pub use models::motion;
pub use models::{RenderScene, RetainedScene, RetainedTarget};
pub use resource_error::{PreparationError, ResourceError};
pub use scene::{SceneError, SceneStats};
use std::{collections::HashMap, error::Error, sync::Mutex, time::Duration};
pub use text::{
    MAX_TEXT_GLYPHS, MAX_TEXT_NODES, TEXT_COLOR, TEXT_LOGICAL_PIXELS_PER_UNIT, TextUploads,
    text_layout_to_local,
};
pub use ui::{
    MAX_UI_GLYPHS, MAX_UI_QUADS, MAX_UI_WIDGETS, UiCaret, UiLayer, UiStage, UiStats, UiUploads,
    theme as ui_theme, ui_caret,
};
#[cfg(target_arch = "wasm32")]
pub use web_error::PreparationErrorKind;
pub use wgpu;
type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;
pub const MIN_SCREENSHOT_DIMENSION: u32 = 16;
pub const MAX_SCREENSHOT_WIDTH: u32 = 1920;
pub const MAX_SCREENSHOT_HEIGHT: u32 = 1080;
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    position: [f32; 4],
    normal: [f32; 3],
}
/// Physical-pixel viewport placement supplied by the editor, including its chrome mask.
#[derive(Clone, Copy)]
pub struct Viewport {
    pub rect: [f32; 4],
    pub corner_radii: [f32; 4],
    pub canvas_srgb: [u8; 3],
}
pub struct Renderer {
    identity: std::sync::Arc<()>,
    device_lost: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub adapter: wgpu::Adapter,
    pub adapter_name: String,
    /// Initial cache read and latest successful cold-preparation write attempt.
    /// Supplying cache bytes is not a measured driver hit.
    pub pipeline_cache_report: PipelineCacheReport,
    pipeline_cache: pipeline_cache::Session,
    diagnostic_pipelines: Vec<wgpu::RenderPipeline>,
    scene_msaa: SceneMsaa,
    models: Mutex<models::ModelCache>,
    materials: materials::MaterialSystem,
    ambient_occlusion: Option<ambient_occlusion::System>,
    sprites: sprites::SpriteSystem,
    text: text::TextSystem,
    ui: ui::UiSystem,
    environments: environment::EnvironmentSystem,
    lighting: lighting::LightingSystem,
    shadows: shadows::ShadowSystem,
    frames: frame::FrameCache,
    output: output::OutputPass,
    bloom: Option<bloom::BloomSystem>,
    graph: graph::FrameGraph,
    motion: Option<motion::MotionSystem>,
    temporal: Option<temporal::TemporalSystem>,
    #[cfg(feature = "native-metalfx")]
    metalfx: Option<metalfx::System>,
    #[cfg(feature = "native-metalfx")]
    metalfx_admission: Option<std::result::Result<[f32; 2], metalfx::Fallback>>,
}
impl Renderer {
    pub async fn new(
        instance: &wgpu::Instance,
        surface: Option<&wgpu::Surface<'_>>,
    ) -> Result<Self> {
        Self::new_configured(
            instance,
            surface,
            false,
            SceneMsaa::One,
            #[cfg(all(feature = "native-compilation", target_os = "macos"))]
            None,
        )
        .await
    }
    /// Explicit opt-in to bounded, asynchronous native GPU timestamp collection.
    /// Unsupported adapters fail rather than silently substituting CPU timings.
    /// Instrumentation can perturb performance and is off in `new`/`headless`.
    pub async fn new_with_gpu_timing(
        instance: &wgpu::Instance,
        surface: Option<&wgpu::Surface<'_>>,
    ) -> Result<Self> {
        Self::new_configured(
            instance,
            surface,
            true,
            SceneMsaa::One,
            #[cfg(all(feature = "native-compilation", target_os = "macos"))]
            None,
        )
        .await
    }
    async fn new_configured(
        instance: &wgpu::Instance,
        surface: Option<&wgpu::Surface<'_>>,
        gpu_timing: bool,
        scene_msaa: SceneMsaa,
        #[cfg(all(feature = "native-compilation", target_os = "macos"))] compilation: Option<
            &mut compilation::Capture,
        >,
    ) -> Result<Self> {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: surface,
                force_fallback_adapter: false,
            })
            .await?;
        // Refuse unsupported attachment/resolve capabilities before requesting
        // a device or allocating any renderer textures/pipelines.
        scene_msaa.validate_formats(
            adapter.get_texture_format_features(frame::HDR_FORMAT),
            adapter.get_texture_format_features(wgpu::TextureFormat::Depth32Float),
        )?;
        let adapter_name = adapter.get_info().name;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Incant renderer"),
                required_features: output::required_features(&adapter)?
                    | pipeline_cache::required_features(&adapter)
                    | timing::required_features(gpu_timing, adapter.features())?,
                ..Default::default()
            })
            .await?;
        // Attach before any renderer catalog work. The caller owns this handle,
        // so partial startup failures remain observable after this future returns.
        #[cfg(all(feature = "native-compilation", target_os = "macos"))]
        let _compilation_scope = compilation
            .map(|capture| capture.attach(&device))
            .transpose()?;
        let device_lost = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let lost = std::sync::Arc::clone(&device_lost);
        device.set_device_lost_callback(move |_, _| {
            lost.store(true, std::sync::atomic::Ordering::Release);
        });
        // The renderer owns a finite pipeline catalog. Prepare it before any
        // caller can submit a frame; frame preparation only looks up handles.
        let validation = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let mut cache_session = pipeline_cache::Session::new(&device, &adapter);
        let cache = cache_session.cache.as_ref();
        let environments = environment::EnvironmentSystem::new(&device, &queue);
        let lighting = lighting::LightingSystem::new(&device, adapter.get_info().backend, cache)?;
        let materials = materials::MaterialSystem::new(
            &device,
            &queue,
            &environments.layout,
            &lighting.layout,
            adapter.get_info().backend,
            cache,
            scene_msaa,
        )?;
        let shadows = shadows::ShadowSystem::new(
            &device,
            &materials.layout,
            adapter.get_info().backend,
            cache,
        )?;
        let diagnostic_pipelines = scene_msaa
            .catalog()
            .iter()
            .map(|&samples| {
                Self::pipeline(
                    &device,
                    frame::HDR_FORMAT,
                    adapter.get_info().backend,
                    cache,
                    samples,
                )
            })
            .collect::<Result<Vec<_>>>()?;
        let output = output::OutputPass::new(&device, adapter.get_info().backend, cache)?;
        let sprites =
            sprites::SpriteSystem::new(&device, adapter.get_info().backend, cache, scene_msaa)?;
        let text = text::TextSystem::new(&device, adapter.get_info().backend, cache, scene_msaa)?;
        let ui = ui::UiSystem::new(&device, &queue, adapter.get_info().backend, cache)?;
        let graph = if gpu_timing {
            graph::FrameGraph::with_timings(&device, &queue)?
        } else {
            Default::default()
        };
        if let Some(error) = validation.pop().await {
            #[cfg(target_arch = "wasm32")]
            let error = error.into();
            return Err(ResourceError::Preparation(error).into());
        }
        cache_session.complete(Ok(()))?;
        Ok(Self {
            identity: std::sync::Arc::new(()),
            device_lost,
            sprites,
            text,
            ui,
            device,
            queue,
            adapter,
            adapter_name,
            pipeline_cache_report: cache_session.report.clone(),
            pipeline_cache: cache_session,
            diagnostic_pipelines,
            scene_msaa,
            models: Mutex::new(HashMap::new()),
            materials,
            ambient_occlusion: None,
            environments,
            lighting,
            shadows,
            frames: Default::default(),
            output,
            bloom: None,
            graph,
            motion: None,
            temporal: None,
            #[cfg(feature = "native-metalfx")]
            metalfx: None,
            #[cfg(feature = "native-metalfx")]
            metalfx_admission: None,
        })
    }
    /// Called only from explicit cold preparation after the validation scope.
    fn complete_pipeline_preparation<T>(&mut self, candidate: Result<T>) -> Result<T> {
        let prepared = self.pipeline_cache.complete(candidate)?;
        self.pipeline_cache_report = self.pipeline_cache.report.clone();
        Ok(prepared)
    }
    pub fn headless() -> Result<Self> {
        let instance = wgpu::Instance::default();
        pollster::block_on(Self::new(&instance, None))
    }
    /// Poll callbacks without waiting, then consume at most one completed sample.
    /// Covers the scene graph through display conversion, excluding later UI,
    /// presentation, uploads, CPU work, GPU readback and timestamp-query resolve.
    /// Includes scene color resolve when multisampling is selected.
    pub fn take_gpu_timing(&self) -> Result<Option<timing::GpuFrameTiming>> {
        self.check_device()?;
        Ok(self.graph.take_timing()?)
    }
    pub fn gpu_timing_stats(
        &self,
    ) -> std::result::Result<timing::GpuTimingStats, timing::GpuTimingError> {
        self.graph.timing_stats()
    }
    /// Drive native callbacks without waiting for GPU completion and reject a
    /// reported device loss. Successfully queued work cannot be rolled back.
    /// The owning renderer installs the callback once, shared by all consumers.
    pub fn check_device(&self) -> Result<()> {
        self.device.poll(wgpu::PollType::Poll)?;
        self.check_device_status()
    }
    /// Observe an already-delivered terminal flag without polling the device or
    /// driving callbacks. Cold final publication uses this after preflight.
    pub fn check_device_status(&self) -> Result<()> {
        if self.device_lost.load(std::sync::atomic::Ordering::Acquire) {
            return Err(ResourceError::DeviceLost.into());
        }
        Ok(())
    }
    fn pipeline(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        backend: wgpu::Backend,
        cache: Option<&wgpu::PipelineCache>,
        samples: SceneMsaa,
    ) -> Result<wgpu::RenderPipeline> {
        let shader = geometry_shader::stages(device, backend, geometry_shader::Kind::Diagnostic)?;
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Diagnostic empty layout"),
            bind_group_layouts: &[],
            immediate_size: 0,
        });
        Ok(
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Phase 0 geometry"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader.vertex,
                    entry_point: Some(shader.vertex_entry),
                    compilation_options: Default::default(),
                    buffers: &geometry_shader::diagnostic_buffers(),
                },
                primitive: wgpu::PrimitiveState {
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::Less),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: samples.state(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader.fragment,
                    entry_point: Some(shader.fragment_entry),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache,
            }),
        )
    }
    /// Renderer-lifetime text texture/quad upload counts, for retention checks.
    pub fn text_uploads(&self) -> TextUploads {
        self.text.uploads()
    }
    pub fn screenshot_scene_png(
        &self,
        scene: &RenderScene,
        width: u32,
        height: u32,
    ) -> Result<Vec<u8>> {
        if !(MIN_SCREENSHOT_DIMENSION..=MAX_SCREENSHOT_WIDTH).contains(&width)
            || !(MIN_SCREENSHOT_DIMENSION..=MAX_SCREENSHOT_HEIGHT).contains(&height)
        {
            return Err("screenshot size out of bounds".into());
        }
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Screenshot"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        self.queue.submit([self.draw_scene(
            scene,
            &texture.create_view(&Default::default()),
            wgpu::TextureFormat::Rgba8UnormSrgb,
            width,
            height,
            None,
        )?]);
        let stride = (width * 4).div_ceil(256) * 256;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Screenshot readback"),
            size: u64::from(stride) * u64::from(height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(stride),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = tx.send(result);
            });
        self.device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(10)),
        })?;
        rx.recv_timeout(Duration::from_secs(10))??;
        let data = buffer.slice(..).get_mapped_range();
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for row in data.chunks(stride as usize) {
            pixels.extend_from_slice(&row[..(width * 4) as usize]);
        }
        drop(data);
        buffer.unmap();
        let mut bytes = vec![];
        {
            let mut encoder = png::Encoder::new(&mut bytes, width, height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.write_header()?.write_image_data(&pixels)?;
        }
        Ok(bytes)
    }
}
const CAMERA_NEAR: f32 = 0.1;
const CAMERA_FAR: f32 = 1000.;
const CAMERA_FOV: f32 = 50f32.to_radians();
fn camera_view() -> Mat4 {
    camera::CameraView::preview().view
}
fn vertices(transforms: &[Mat4], camera: Mat4) -> Vec<Vertex> {
    let corners = [
        [-0.5, -0.5, -0.5],
        [0.5, -0.5, -0.5],
        [0.5, 0.5, -0.5],
        [-0.5, 0.5, -0.5],
        [-0.5, -0.5, 0.5],
        [0.5, -0.5, 0.5],
        [0.5, 0.5, 0.5],
        [-0.5, 0.5, 0.5],
    ];
    let faces = [
        ([0, 1, 2, 0, 2, 3], [0., 0., -1.]),
        ([4, 6, 5, 4, 7, 6], [0., 0., 1.]),
        ([0, 4, 5, 0, 5, 1], [0., -1., 0.]),
        ([3, 2, 6, 3, 6, 7], [0., 1., 0.]),
        ([0, 3, 7, 0, 7, 4], [-1., 0., 0.]),
        ([1, 5, 6, 1, 6, 2], [1., 0., 0.]),
    ];
    let mut output = vec![];
    for world in transforms {
        let mvp = camera * world;
        for (indices, normal) in faces {
            let normal = world
                .inverse()
                .transpose()
                .transform_vector3(Vec3::from_array(normal))
                .normalize()
                .to_array();
            for index in indices {
                let position = mvp * Vec3::from_array(corners[index]).extend(1.);
                output.push(Vertex {
                    position: position.to_array(),
                    normal,
                });
            }
        }
    }
    output
}

impl ScenePreparer for Renderer {
    type Prepared = RenderScene;
    fn prepare_input(&self, input: &SceneInput) -> incant_render_data::Result<RenderScene> {
        self.prepare(input)
    }
    fn capture_prepared(
        &self,
        scene: &RenderScene,
        width: u32,
        height: u32,
    ) -> incant_render_data::Result<Vec<u8>> {
        self.screenshot_scene_png(scene, width, height)
    }
}
