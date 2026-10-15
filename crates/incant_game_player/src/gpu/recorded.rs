//! Manifest admission happens against the actual native input before requesting
//! a GPU device. The ordinary retained consumer still owns every real frame.
use super::*;
use incant_render_data::{
    SceneInput, UiInput,
    prewarm::{MaterialIdentity, MaterialManifest, MaterialProfile, PrewarmError},
};
use incant_render_game::RenderInputConsumer;
use incant_render_gpu_host::GpuHostError;
use incant_render_native::UpdateKind;
use sha2::{Digest, Sha256};

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error(transparent)]
    Manifest(#[from] PrewarmError),
    #[error("recorded renderer setup: {0}")]
    Render(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error(transparent)]
    Consumer(#[from] GpuHostError<TargetError>),
}
pub(crate) struct Settings {
    manifest: MaterialManifest,
    scene: [u8; 16],
    cooked_scene_sha256: [u8; 32],
    ticks: u64,
}
pub(crate) struct Consumer {
    inner: GpuRenderConsumer<Targets>,
    renderer: Arc<Renderer>,
    observation: Arc<Mutex<Observation>>,
}
impl RenderInputConsumer for Consumer {
    type Settings = Settings;
    type Error = Error;
    fn prepare(
        input: &SceneInput,
        camera: &str,
        ui: Option<&Arc<UiInput>>,
        settings: Settings,
    ) -> std::result::Result<Self, Error> {
        let expected = MaterialIdentity::loaded(
            settings.scene,
            settings.cooked_scene_sha256,
            MaterialProfile {
                // The manifest selects only the two supported attachment modes;
                // the renderer uses this admitted mode and validates device
                // formats before allocating either mode's finite catalog.
                samples: settings.manifest.identity.profile.samples,
                program: Renderer::material_program(),
            },
            input,
        )?;
        let admitted = settings.manifest.admit(&expected)?;
        // No GPU work occurs until every persisted identity and key is admitted.
        let instance = wgpu::Instance::default();
        let renderer = Arc::new(
            pollster::block_on(Renderer::new_with_recorded_materials(
                &instance, None, admitted,
            ))
            .map_err(Error::Render)?,
        );
        let observation = Arc::new(Mutex::new(Observation {
            captures: Vec::with_capacity(2),
            adapter: renderer.adapter.get_info(),
        }));
        let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Recorded material source-free offscreen player"),
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
            label: Some("Recorded material first/final frame readback"),
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
            last: settings.ticks,
        };
        let inner = GpuRenderConsumer::prepare(
            input,
            camera,
            ui,
            GpuRenderSettings {
                renderer: Arc::clone(&renderer),
                targets,
            },
        )?;
        Ok(Self {
            inner,
            renderer,
            observation,
        })
    }
    fn accept(
        &mut self,
        input: &SceneInput,
        camera: &str,
        kind: UpdateKind,
        ui: Option<&Arc<UiInput>>,
    ) -> std::result::Result<(), Error> {
        Ok(self.inner.accept(input, camera, kind, ui)?)
    }
    fn admit_ui(&mut self, ui: &Arc<UiInput>) -> std::result::Result<(), Error> {
        Ok(self.inner.admit_ui(ui)?)
    }
}
type Game = HostedOfflineGameSession<NativeRenderHost<Consumer>>;
pub(crate) struct Session {
    inner: Option<Game>,
    renderer: Option<Arc<Renderer>>,
}
impl Session {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        scene: CookedScene,
        script: SessionProgram,
        actions: InputActions,
        clips: Vec<ClipBinding>,
        rate: u32,
        settings: NativeRenderSettings,
        ticks: u64,
        manifest: MaterialManifest,
    ) -> Result<Self> {
        let consumer = Settings {
            manifest,
            scene: scene.id().0,
            cooked_scene_sha256: Sha256::digest(scene.to_bytes()).into(),
            ticks,
        };
        let inner: Game = HostedOfflineGameSession::new(
            scene,
            script,
            actions,
            clips,
            rate,
            NativeRenderSettings {
                ui: settings.ui,
                text: settings.text,
                model_cache: settings.model_cache,
                texture_cache: settings.texture_cache,
                camera: settings.camera,
                consumer,
            },
        )?;
        let renderer = Arc::clone(&inner.host()?.consumer().renderer);
        Ok(Self {
            inner: Some(inner),
            renderer: Some(renderer),
        })
    }
    pub(crate) fn inner(&self) -> Result<&Game> {
        self.inner
            .as_ref()
            .ok_or_else(|| "recorded GPU session is closed".into())
    }
    pub(crate) fn inner_mut(&mut self) -> Result<&mut Game> {
        self.inner
            .as_mut()
            .ok_or_else(|| "recorded GPU session is closed".into())
    }
    pub(crate) fn report(&self) -> Result<serde_json::Value> {
        let host = self.inner()?.host()?;
        let consumer = host.consumer();
        let observed = consumer
            .observation
            .lock()
            .map_err(|_| TargetError::Observation)?;
        let mut report = input_report(host.render_input())?;
        report["mode"] = "gpu-offscreen-recorded-materials".into();
        report["gpu_submitted"] = true.into();
        report["gpu"] = serde_json::json!({ "width":WIDTH,"height":HEIGHT,"on_screen":false,
            "accepted_frames":consumer.inner.accepted_frames(),
            "adapter":{"name":observed.adapter.name,"backend":format!("{:?}",observed.adapter.backend),"device_type":format!("{:?}",observed.adapter.device_type)},
            "captures":observed.captures.iter().map(|c|serde_json::json!({"frame":c.frame,"sha256":c.sha256,"non_background_pixels":c.non_background_pixels})).collect::<Vec<_>>() });
        report["material_prewarm"] =
            serde_json::to_value(consumer.renderer.material_prewarm_report())?;
        Ok(report)
    }
    pub(crate) fn close(&mut self) -> Result<serde_json::Value> {
        // Destroy frame/consumer owners first, then reclaim the sole renderer.
        // The worker owns only its device/layout/shader handles, never this Arc.
        drop(self.inner.take());
        let renderer = self
            .renderer
            .take()
            .ok_or("recorded GPU session already closed")?;
        let mut renderer = Arc::try_unwrap(renderer)
            .map_err(|_| "unexpected recorded renderer owner at cold close")?;
        renderer
            .close_recorded_materials(Duration::from_secs(2))
            .map_err(|e| -> Box<dyn std::error::Error> { e })?;
        Ok(
            serde_json::json!({"closed":true,"wait_bound_ms":2000,"report":renderer.material_prewarm_report()}),
        )
    }
}
