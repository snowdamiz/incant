//! Bounded offscreen proof target, never a surface or screenshot interface. The
//! real retained GPU consumer submits every accepted tick; only first/final
//! frames are read back for numerical evidence. No second renderer is created.
use crate::session::{Result, input_report};
use incant_assets_runtime::sha256;
use incant_audio_runtime::ClipBinding;
use incant_game_runtime::HostedOfflineGameSession;
use incant_input::InputActions;
use incant_render::{Renderer, RetainedTarget, wgpu};
use incant_render_game::{NativeRenderHost, NativeRenderSettings};
use incant_render_gpu_host::{GpuRenderConsumer, GpuRenderSettings, RenderFrame, TargetProvider};
use incant_runtime::CookedScene;
use incant_script::CompiledScript;
use std::sync::{Arc, Mutex};
use std::time::Duration;
const WIDTH: u32 = 160;
const HEIGHT: u32 = 96;
const STRIDE: u32 = 768;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
#[derive(Debug, thiserror::Error)]
pub(crate) enum TargetError {
    #[error("offscreen GPU readback failed: {0}")]
    Map(#[from] wgpu::BufferAsyncError),
    #[error("offscreen GPU callback failed: {0}")]
    Receive(#[from] std::sync::mpsc::RecvTimeoutError),
    #[error("offscreen GPU device failed: {0}")]
    Device(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("offscreen frame counter exhausted")]
    FrameLimit,
    #[error("offscreen observation lock failed")]
    Observation,
}
struct Capture {
    frame: u64,
    sha256: String,
    non_background_pixels: usize,
}
struct Observation {
    captures: Vec<Capture>,
    adapter: wgpu::AdapterInfo,
}
pub(crate) struct Targets {
    renderer: Arc<Renderer>,
    texture: wgpu::Texture,
    readback: wgpu::Buffer,
    observation: Arc<Mutex<Observation>>,
    acquired: u64,
    last: u64,
}
pub(crate) struct Frame {
    renderer: Arc<Renderer>,
    view: wgpu::TextureView,
    capture: Option<(wgpu::Texture, wgpu::Buffer, Arc<Mutex<Observation>>)>,
    number: u64,
}
impl TargetProvider for Targets {
    type Error = TargetError;
    type Frame = Frame;
    fn acquire(&mut self, _: &Renderer) -> std::result::Result<Frame, TargetError> {
        let number = self
            .acquired
            .checked_add(1)
            .ok_or(TargetError::FrameLimit)?;
        self.acquired = number;
        Ok(Frame {
            renderer: Arc::clone(&self.renderer),
            view: self.texture.create_view(&Default::default()),
            capture: if number == 1 || number == self.last {
                Some((
                    self.texture.clone(),
                    self.readback.clone(),
                    Arc::clone(&self.observation),
                ))
            } else {
                None
            },
            number,
        })
    }
}
impl RenderFrame for Frame {
    type Error = TargetError;
    fn target(&self) -> RetainedTarget<'_> {
        RetainedTarget {
            view: &self.view,
            format: FORMAT,
            width: WIDTH,
            height: HEIGHT,
            viewport: None,
        }
    }
    fn present(self) -> std::result::Result<(), TargetError> {
        // Offscreen acceptance releases the owned target. It does not claim an
        // OS presentation; readback follows the consumer's actual submission.
        let Some((texture, buffer, observation)) = self.capture else {
            return Ok(());
        };
        let mut encoder = self
            .renderer
            .device
            .create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(STRIDE),
                    rows_per_image: Some(HEIGHT),
                },
            },
            texture.size(),
        );
        self.renderer.queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = tx.send(result);
            });
        self.renderer
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_secs(10)),
            })
            .map_err(|e| TargetError::Device(e.into()))?;
        rx.recv_timeout(Duration::from_secs(10))??;
        let mapped = buffer.slice(..).get_mapped_range();
        let bytes: Vec<_> = mapped
            .chunks(STRIDE as usize)
            .flat_map(|row| row[..WIDTH as usize * 4].iter().copied())
            .collect();
        drop(mapped);
        buffer.unmap();
        self.renderer.check_device().map_err(TargetError::Device)?;
        let non_background_pixels = bytes
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|pixel| pixel.as_slice() != &bytes[..4])
            .count();
        observation
            .lock()
            .map_err(|_| TargetError::Observation)?
            .captures
            .push(Capture {
                frame: self.number,
                sha256: sha256(&bytes),
                non_background_pixels,
            });
        Ok(())
    }
}
pub(crate) struct Session {
    pub(crate) inner: HostedOfflineGameSession<NativeRenderHost<GpuRenderConsumer<Targets>>>,
    observation: Arc<Mutex<Observation>>,
}
impl Session {
    pub(crate) fn new(
        scene: CookedScene,
        script: CompiledScript,
        actions: InputActions,
        clips: Vec<ClipBinding>,
        rate: u32,
        settings: NativeRenderSettings,
        ticks: u64,
    ) -> Result<Self> {
        let renderer =
            Arc::new(Renderer::headless().map_err(|e| -> Box<dyn std::error::Error> { e })?);
        let observation = Arc::new(Mutex::new(Observation {
            captures: Vec::with_capacity(2),
            adapter: renderer.adapter.get_info(),
        }));
        let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Native source-free offscreen player"),
            size: wgpu::Extent3d {
                width: WIDTH,
                height: HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let readback = renderer.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Bounded first/final frame readback"),
            size: u64::from(STRIDE) * u64::from(HEIGHT),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let targets = Targets {
            renderer: Arc::clone(&renderer),
            texture,
            readback,
            observation: Arc::clone(&observation),
            acquired: 0,
            last: ticks,
        };
        let inner = HostedOfflineGameSession::new(
            scene,
            script,
            actions,
            clips,
            rate,
            NativeRenderSettings {
                model_cache: settings.model_cache,
                texture_cache: settings.texture_cache,
                camera: settings.camera,
                consumer: GpuRenderSettings { renderer, targets },
            },
        )?;
        Ok(Self { inner, observation })
    }
    pub(crate) fn report(&self) -> Result<serde_json::Value> {
        let host = self.inner.host()?;
        let mut report = input_report(host.render_input())?;
        let observed = self
            .observation
            .lock()
            .map_err(|_| TargetError::Observation)?;
        report["mode"] = "gpu-offscreen".into();
        report["gpu_submitted"] = true.into();
        report["gpu"] = serde_json::json!({ "width":WIDTH,"height":HEIGHT,"on_screen":false,
            "accepted_frames":host.consumer().accepted_frames(),
            "adapter":{"name":observed.adapter.name,"backend":format!("{:?}",observed.adapter.backend),"device_type":format!("{:?}",observed.adapter.device_type)},
            "captures":observed.captures.iter().map(|c|serde_json::json!({"frame":c.frame,"sha256":c.sha256,"non_background_pixels":c.non_background_pixels})).collect::<Vec<_>>() });
        Ok(report)
    }
}
