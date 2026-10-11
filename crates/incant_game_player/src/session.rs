//! Optional CPU-only spatial proof on the existing source-free player. The
//! default build and default invocation retain the strict nonvisual boundary.
use incant_audio_runtime::ClipBinding;
use incant_game_runtime::OfflineGameSession;
use incant_input::{InputActions, InputEvent};
use incant_runtime::{CookedScene, NativeWorld};
use incant_script::CompiledScript;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
#[cfg(feature = "render-input")]
use incant_game_runtime::HostedOfflineGameSession;
#[cfg(feature = "render-input")]
use incant_render_game::{NativeRenderHost, NativeRenderSettings};
#[cfg(feature = "render-input")]
pub type Settings = NativeRenderSettings;
#[cfg(not(feature = "render-input"))]
pub enum Settings {}

pub fn settings(args: &[String]) -> Result<Option<Settings>> {
    if args.len() == 8 {
        return Ok(None);
    }
    if args.len() != 12 || args[8] != "--cpu-render-input" {
        return Err("expected SCENE CACHE CLIPS RECORDING ACTIONS TICKS SAMPLE_RATE OUTPUT [--cpu-render-input MODEL_CACHE TEXTURE_CACHE CAMERA_HEX]; manual input/source paths are unsupported".into());
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
        Ok(Some(NativeRenderSettings {
            model_cache: args[9].clone().into(),
            texture_cache: args[10].clone().into(),
            camera: incant_runtime::StableId(camera),
            consumer: (),
        }))
    }
}
pub enum Player {
    Plain(Box<OfflineGameSession>),
    #[cfg(feature = "render-input")]
    Spatial(Box<HostedOfflineGameSession<NativeRenderHost>>),
}
impl Player {
    pub fn new(
        scene: CookedScene,
        script: CompiledScript,
        actions: InputActions,
        clips: Vec<ClipBinding>,
        rate: u32,
        settings: Option<Settings>,
    ) -> Result<Self> {
        #[cfg(feature = "render-input")]
        if let Some(settings) = settings {
            return Ok(Self::Spatial(Box::new(HostedOfflineGameSession::new(
                scene, script, actions, clips, rate, settings,
            )?)));
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
        })
    }
    pub fn rendered_frames(&self) -> Result<u128> {
        Ok(match self {
            Self::Plain(game) => game.rendered_frames()?,
            #[cfg(feature = "render-input")]
            Self::Spatial(game) => game.rendered_frames()?,
        })
    }
    pub fn tick(&mut self, events: &[InputEvent], output: &mut [f32]) -> Result<()> {
        match self {
            Self::Plain(game) => game.tick(events, output)?,
            #[cfg(feature = "render-input")]
            Self::Spatial(game) => game.tick(events, output)?,
        };
        Ok(())
    }
    pub fn world(&self) -> Result<&NativeWorld> {
        Ok(match self {
            Self::Plain(game) => game.world()?,
            #[cfg(feature = "render-input")]
            Self::Spatial(game) => game.world()?,
        })
    }
    /// One bounded final observation, never a per-tick world projection.
    pub fn render_report(&self) -> Result<Option<serde_json::Value>> {
        #[cfg(feature = "render-input")]
        if let Self::Spatial(game) = self {
            let input = game.host()?.render_input();
            let nodes = &input.input().nodes;
            let camera = nodes
                .iter()
                .find(|n| n.id == input.camera_label())
                .ok_or("selected camera invariant")?;
            return Ok(Some(serde_json::json!({
                "mode":"cpu-only", "tick":input.tick(), "topology_generation":input.topology_generation(),
                "binding_generation":input.binding_generation(), "camera":input.camera_label(),
                "camera_position":camera.world.w_axis.to_array()[..3],
                "model_count":nodes.iter().filter(|n|n.mesh.is_some()).count(),
                "first_model_position":nodes.iter().find(|n|n.mesh.is_some()).map(|n| n.world.w_axis.to_array()[..3].to_vec()),
                "decoded_payload_bytes":input.payload_bytes(), "gpu_submitted":false
            })));
        }
        Ok(None)
    }
}
