//! Optional CPU/GPU spatial proofs on the existing source-free player. Default
//! builds and invocations retain the strict nonvisual boundary.
use incant_audio_runtime::ClipBinding;
use incant_game_runtime::OfflineGameSession;
use incant_input::{InputActions, InputEvent};
use incant_runtime::{CookedScene, NativeWorld};
use incant_script::CompiledScript;
pub(crate) type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
#[cfg(feature = "render-input")]
use incant_game_runtime::HostedOfflineGameSession;
#[cfg(feature = "render-input")]
use incant_render_game::{NativeRenderHost, NativeRenderSettings};
#[cfg(feature = "render-input")]
pub enum Settings {
    Cpu(NativeRenderSettings),
    #[cfg(feature = "render-gpu")]
    Gpu(NativeRenderSettings),
}
#[cfg(not(feature = "render-input"))]
pub enum Settings {}

pub fn settings(args: &[String]) -> Result<Option<Settings>> {
    if args.len() == 8 {
        return Ok(None);
    }
    if args.len() != 12 || !matches!(args[8].as_str(), "--cpu-render-input" | "--gpu-offscreen") {
        return Err("expected SCENE CACHE CLIPS RECORDING ACTIONS TICKS SAMPLE_RATE OUTPUT [--cpu-render-input|--gpu-offscreen MODEL_CACHE TEXTURE_CACHE CAMERA_HEX]; manual input/source paths are unsupported".into());
    }
    #[cfg(not(feature = "render-gpu"))]
    if args[8] == "--gpu-offscreen" {
        return Err("offscreen GPU mode requires the render-gpu build feature".into());
    }
    #[cfg(not(feature = "render-input"))]
    {
        Err("CPU render-input mode requires the render-input build feature".into())
    }
    #[cfg(feature = "render-input")]
    {
        let text = &args[11];
        if text.len() != 32 || !text.bytes().all(|c| c.is_ascii_hexdigit()) {
            return Err("CAMERA_HEX must contain exactly 32 hexadecimal digits".into());
        }
        let mut camera = [0; 16];
        for (i, value) in camera.iter_mut().enumerate() {
            *value = u8::from_str_radix(&text[i * 2..i * 2 + 2], 16)?;
        }
        let settings = NativeRenderSettings {
            model_cache: args[9].clone().into(),
            texture_cache: args[10].clone().into(),
            camera: incant_runtime::StableId(camera),
            consumer: (),
        };
        #[cfg(feature = "render-gpu")]
        if args[8] == "--gpu-offscreen" {
            return Ok(Some(Settings::Gpu(settings)));
        }
        Ok(Some(Settings::Cpu(settings)))
    }
}
pub enum Player {
    Plain(Box<OfflineGameSession>),
    #[cfg(feature = "render-input")]
    Spatial(Box<HostedOfflineGameSession<NativeRenderHost>>),
    #[cfg(feature = "render-gpu")]
    Gpu(Box<crate::gpu::Session>),
}
impl Player {
    pub fn new(
        scene: CookedScene,
        script: CompiledScript,
        actions: InputActions,
        clips: Vec<ClipBinding>,
        rate: u32,
        settings: Option<Settings>,
        requested_ticks: u64,
    ) -> Result<Self> {
        #[cfg(not(feature = "render-gpu"))]
        let _ = requested_ticks;
        #[cfg(feature = "render-input")]
        if let Some(settings) = settings {
            return Ok(match settings {
                Settings::Cpu(settings) => Self::Spatial(Box::new(HostedOfflineGameSession::new(
                    scene, script, actions, clips, rate, settings,
                )?)),
                #[cfg(feature = "render-gpu")]
                Settings::Gpu(settings) => Self::Gpu(Box::new(crate::gpu::Session::new(
                    scene,
                    script,
                    actions,
                    clips,
                    rate,
                    settings,
                    requested_ticks,
                )?)),
            });
        }
        #[cfg(not(feature = "render-input"))]
        if let Some(settings) = settings {
            match settings {}
        }
        Ok(Self::Plain(Box::new(OfflineGameSession::new(
            scene, script, actions, clips, rate,
        )?)))
    }
    pub fn next_output_frames(&self) -> Result<usize> {
        Ok(match self {
            Self::Plain(game) => game.next_output_frames()?,
            #[cfg(feature = "render-input")]
            Self::Spatial(game) => game.next_output_frames()?,
            #[cfg(feature = "render-gpu")]
            Self::Gpu(game) => game.inner.next_output_frames()?,
        })
    }
    pub fn rendered_frames(&self) -> Result<u128> {
        Ok(match self {
            Self::Plain(game) => game.rendered_frames()?,
            #[cfg(feature = "render-input")]
            Self::Spatial(game) => game.rendered_frames()?,
            #[cfg(feature = "render-gpu")]
            Self::Gpu(game) => game.inner.rendered_frames()?,
        })
    }
    pub fn tick(&mut self, events: &[InputEvent], output: &mut [f32]) -> Result<()> {
        match self {
            Self::Plain(game) => game.tick(events, output)?,
            #[cfg(feature = "render-input")]
            Self::Spatial(game) => game.tick(events, output)?,
            #[cfg(feature = "render-gpu")]
            Self::Gpu(game) => game.inner.tick(events, output)?,
        };
        Ok(())
    }
    pub fn world(&self) -> Result<&NativeWorld> {
        Ok(match self {
            Self::Plain(game) => game.world()?,
            #[cfg(feature = "render-input")]
            Self::Spatial(game) => game.world()?,
            #[cfg(feature = "render-gpu")]
            Self::Gpu(game) => game.inner.world()?,
        })
    }
    /// One bounded final observation, never a per-tick world projection.
    pub fn render_report(&self) -> Result<Option<serde_json::Value>> {
        #[cfg(feature = "render-gpu")]
        if let Self::Gpu(game) = self {
            return Ok(Some(game.report()?));
        }
        #[cfg(feature = "render-input")]
        if let Self::Spatial(game) = self {
            return Ok(Some(input_report(game.host()?.render_input())?));
        }
        Ok(None)
    }
}
#[cfg(feature = "render-input")]
pub(crate) fn input_report(
    input: &incant_render_native::NativeRenderInput,
) -> Result<serde_json::Value> {
    let nodes = &input.input().nodes;
    let camera = nodes
        .iter()
        .find(|n| n.id == input.camera_label())
        .ok_or("selected camera invariant")?;
    Ok(serde_json::json!({
        "mode":"cpu-only", "tick":input.tick(), "topology_generation":input.topology_generation(),
        "binding_generation":input.binding_generation(), "camera":input.camera_label(),
        "camera_position":camera.world.w_axis.to_array()[..3],
        "model_count":nodes.iter().filter(|n|n.mesh.is_some()).count(),
        "first_model_position":nodes.iter().find(|n|n.mesh.is_some()).map(|n| n.world.w_axis.to_array()[..3].to_vec()),
        "sprite_count":nodes.iter().filter(|n|n.sprite.is_some()).count(),
        "first_sprite_position":nodes.iter().find(|n|n.sprite.is_some()).map(|n| n.world.w_axis.to_array()[..3].to_vec()),
        "decoded_payload_bytes":input.payload_bytes(), "gpu_submitted":false
    }))
}
