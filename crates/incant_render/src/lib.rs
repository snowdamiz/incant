//! Native rendering and GPU readback with retained imported material previews.
//! Bevy-scheduled HDR geometry, lighting, display transform and composition.
//! Authored clustered lights; the full production render graph remains open.
mod camera;
mod diagnostic_draw;
mod environment;
mod frame;
mod graph;
#[cfg(test)]
#[path = "../tests/hdr_output/mod.rs"]
mod hdr_output;
mod lighting;
mod material_pipeline;
mod materials;
mod models;
mod output;
mod pixel_grid;
mod preview_environment;
mod resource_error;
mod scene;
mod shadows;
mod sprites;
mod studio;
#[cfg(test)]
#[path = "../tests/support/mod.rs"]
mod test_support;
#[cfg(test)]
mod version_tests;
#[cfg(any(target_arch = "wasm32", test))]
mod web_error;
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
pub use incant_render_data::{SceneInput, ScenePreparer};
pub use lighting::LocalLightSelection;
pub use models::{RenderScene, RetainedScene, RetainedTarget};
pub use resource_error::{PreparationError, ResourceError};
pub use scene::{SceneError, SceneStats};
use std::{collections::HashMap, error::Error, sync::Mutex, time::Duration};
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
    diagnostic_pipeline: wgpu::RenderPipeline,
    models: Mutex<models::ModelCache>,
    materials: materials::MaterialSystem,
    sprites: sprites::SpriteSystem,
    environments: environment::EnvironmentSystem,
    lighting: lighting::LightingSystem,
    shadows: shadows::ShadowSystem,
    frames: frame::FrameCache,
    output: output::OutputPass,
    graph: graph::FrameGraph,
}
impl Renderer {
    pub async fn new(
        instance: &wgpu::Instance,
        surface: Option<&wgpu::Surface<'_>>,
    ) -> Result<Self> {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: surface,
                force_fallback_adapter: false,
            })
            .await?;
        let adapter_name = adapter.get_info().name;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Incant renderer"),
                required_features: output::required_features(&adapter)?,
                ..Default::default()
            })
            .await?;
        let device_lost = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let lost = std::sync::Arc::clone(&device_lost);
        device.set_device_lost_callback(move |_, _| {
            lost.store(true, std::sync::atomic::Ordering::Release);
        });
        // The renderer owns a finite pipeline catalog. Prepare it before any
        // caller can submit a frame; frame preparation only looks up handles.
        let validation = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let environments = environment::EnvironmentSystem::new(&device, &queue);
        let lighting = lighting::LightingSystem::new(&device);
        let materials =
            materials::MaterialSystem::new(&device, &queue, &environments.layout, &lighting.layout);
        let shadows = shadows::ShadowSystem::new(&device, &materials.layout);
        let diagnostic_pipeline = Self::pipeline(&device, frame::HDR_FORMAT);
        let output = output::OutputPass::new(&device, adapter.get_info().backend)?;
        let sprites = sprites::SpriteSystem::new(&device);
        if let Some(error) = validation.pop().await {
            #[cfg(target_arch = "wasm32")]
            let error = error.into();
            return Err(ResourceError::Preparation(error).into());
        }
        Ok(Self {
            identity: std::sync::Arc::new(()),
            device_lost,
            sprites,
            device,
            queue,
            adapter,
            adapter_name,
            diagnostic_pipeline,
            models: Mutex::new(HashMap::new()),
            materials,
            environments,
            lighting,
            shadows,
            frames: Default::default(),
            output,
            graph: Default::default(),
        })
    }
    pub fn headless() -> Result<Self> {
        let instance = wgpu::Instance::default();
        pollster::block_on(Self::new(&instance, None))
    }
    /// Drive native callbacks without waiting for GPU completion and reject a
    /// reported device loss. Successfully queued work cannot be rolled back.
    /// The owning renderer installs the callback once, shared by all consumers.
    pub fn check_device(&self) -> Result<()> {
        self.device.poll(wgpu::PollType::Poll)?;
        if self.device_lost.load(std::sync::atomic::Ordering::Acquire) {
            return Err(ResourceError::DeviceLost.into());
        }
        Ok(())
    }
    fn pipeline(device: &wgpu::Device, format: wgpu::TextureFormat) -> wgpu::RenderPipeline {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Phase 0 diagnostic shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("fixture.wgsl").into()),
        });
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Phase 0 geometry"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0=>Float32x4,1=>Float32x3],
                }],
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
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        })
    }
    pub fn draw_scene(
        &self,
        scene: &RenderScene,
        view: &wgpu::TextureView,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        viewport: Option<Viewport>,
    ) -> Result<wgpu::CommandBuffer> {
        self.check_device()?;
        output::OutputPass::validate(format)?;
        let rect = viewport
            .map(|v| v.rect)
            .unwrap_or([0., 0., width as f32, height as f32]);
        if rect.iter().any(|v| !v.is_finite())
            || rect[0] < 0.
            || rect[1] < 0.
            || rect[2] <= 0.
            || rect[3] <= 0.
            || rect[0] + rect[2] > width as f32
            || rect[1] + rect[3] > height as f32
        {
            return Err("viewport outside render target".into());
        }
        if viewport.is_some_and(|v| v.corner_radii.iter().any(|r| !r.is_finite() || *r < 0.)) {
            return Err("invalid viewport corner radius".into());
        }
        frame::FrameCache::validate(&self.device, width, height)?;
        let grid = scene
            .camera
            .pixel_resolution
            .map(|resolution| pixel_grid::PixelGrid::fit(resolution, rect))
            .transpose()?;
        let (frame_size, scene_rect) = if let Some(grid) = &grid {
            (
                grid.resolution,
                [0., 0., grid.resolution[0] as f32, grid.resolution[1] as f32],
            )
        } else {
            ([width, height], rect)
        };
        let attachments = self
            .frames
            .get(&self.device, frame_size[0], frame_size[1])?;
        let diagnostics =
            diagnostic_draw::DiagnosticDraw::prepare(self, scene, attachments.clone(), scene_rect)?;
        let models = self.prepare_models(
            scene,
            models::ModelTarget {
                color: attachments.color.clone(),
                depth: attachments.depth.clone(),
                format: frame::HDR_FORMAT,
                rect: scene_rect,
            },
        )?;
        let sprites = self.sprites.prepare(
            &self.device,
            scene,
            models::ModelTarget {
                color: attachments.color.clone(),
                depth: attachments.depth.clone(),
                format: frame::HDR_FORMAT,
                rect: scene_rect,
            },
        )?;
        let output = self.output.prepare(
            &self.device,
            &attachments.color,
            view,
            format,
            viewport,
            grid.map(|g| g.output),
        )?;
        self.graph.encode(
            &self.device,
            graph::PreparedFrame {
                diagnostics,
                models,
                sprites,
                output,
            },
        )
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
