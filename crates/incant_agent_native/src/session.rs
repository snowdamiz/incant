//! One startup boundary shared by the native CLI and retained RPC play.
//! The session contains no authoring document, command bus or source text.
use crate::{FallbackCompiler, StartOptions};
use incant_audio_runtime::ClipBinding;
use incant_game_runtime::{HostedOfflineGameSession, OfflineGameSession};
use incant_input::{InputActions, InputEvent, InputFrame, RecordedUiEvent};
use incant_render_data::prewarm::{MaterialManifest, MaterialProfile, RecordingContext};
use incant_render_game::{MaterialRecordingConsumer, NativeRenderHost, NativeRenderSettings};
use incant_runtime::{CookedScene, NativeWorld};
use incant_script::{NativeLogs, SessionProgram};
use sha2::{Digest, Sha256};
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("native material recording: {0}")]
    Recording(&'static str),
    #[error("native play startup/runtime failure: {0}")]
    Native(#[source] Box<dyn std::error::Error>),
}
type Result<T> = std::result::Result<T, SessionError>;
fn native<E: std::error::Error + 'static>(error: E) -> SessionError {
    SessionError::Native(Box::new(error))
}

pub struct Session {
    pub scene: String,
    pub script_source_sha256: String,
    pub authored_scripts: bool,
    game: Game,
}
impl Session {
    pub fn new(
        document: &incant_doc::Project,
        root: &Path,
        options: &StartOptions,
        sample_rate: u32,
        fallback: FallbackCompiler,
    ) -> Result<Self> {
        Self::new_configured(document, root, options, sample_rate, fallback, None)
    }
    pub fn new_recording_materials(
        document: &incant_doc::Project,
        root: &Path,
        options: &StartOptions,
        sample_rate: u32,
        fallback: FallbackCompiler,
        profile: MaterialProfile,
        input_sha256: [u8; 32],
    ) -> Result<Self> {
        Self::new_configured(
            document,
            root,
            options,
            sample_rate,
            fallback,
            Some((profile, input_sha256)),
        )
    }
    fn new_configured(
        document: &incant_doc::Project,
        root: &Path,
        options: &StartOptions,
        sample_rate: u32,
        fallback: FallbackCompiler,
        recording: Option<(MaterialProfile, [u8; 32])>,
    ) -> Result<Self> {
        let prepared =
            incant_play::PreparedPlay::load(document, root, options, sample_rate, fallback)
                .map_err(native)?;
        let recording = recording
            .map(|(profile, input_sha256)| {
                Ok::<_, SessionError>(RecordingContext {
                    scene: prepared.cooked.id().0,
                    cooked_scene_sha256: Sha256::digest(prepared.cooked.to_bytes()).into(),
                    profile,
                    input_sha256,
                    tick_rate: prepared.cooked.tick_rate(),
                    script_sha256: prepared
                        .code
                        .artifact_sha256()
                        .ok_or(SessionError::Recording("program has no artifact receipt"))?,
                })
            })
            .transpose()?;
        let game = Game::new(
            prepared.cooked,
            prepared.code,
            prepared.actions,
            prepared.clips,
            prepared.sample_rate,
            prepared.render,
            recording,
        )?;
        Ok(Self {
            scene: prepared.scene,
            script_source_sha256: prepared.script_source_sha256,
            authored_scripts: prepared.authored_scripts,
            game,
        })
    }
    pub fn material_manifest(&self) -> Result<Option<MaterialManifest>> {
        match &self.game {
            Game::Recording(game) => Ok(Some(
                game.host()
                    .map_err(native)?
                    .consumer()
                    .manifest()
                    .map_err(native)?,
            )),
            _ => Ok(None),
        }
    }
    pub fn world(&self) -> Result<&NativeWorld> {
        self.game.world()
    }
    pub fn input(&self) -> Result<&InputFrame> {
        self.game.input()
    }
    pub fn logs(&self) -> Result<&NativeLogs> {
        self.game.logs()
    }
    pub fn next_frames(&self) -> Result<usize> {
        self.game.next_frames()
    }
    pub fn frames(&self) -> Result<u128> {
        self.game.frames()
    }
    pub fn tick(&mut self, input: &[InputEvent], output: &mut [f32]) -> Result<()> {
        self.game.tick(input, output)
    }
    pub fn validate_ui_recording(&self, uses_ui: bool) -> Result<()> {
        match &self.game {
            Game::Recording(game) => {
                incant_render_game::validate_ui_recording(game, uses_ui).map_err(native)
            }
            Game::Spatial(game) => {
                incant_render_game::validate_ui_recording(game, uses_ui).map_err(native)
            }
            Game::Plain(_) if uses_ui => Err(native(incant_render_game::RecordedUiError::<
                std::convert::Infallible,
            >::Invalid(
                "UI recording requires an explicitly configured CPU UI host",
            ))),
            Game::Plain(_) => Ok(()),
        }
    }
    pub fn tick_recorded(
        &mut self,
        physical: &[InputEvent],
        records: &[RecordedUiEvent],
        viewport: Option<[f32; 2]>,
        output: &mut [f32],
    ) -> Result<()> {
        if records.is_empty() && viewport.is_none() {
            return self.tick(physical, output);
        }
        self.validate_ui_recording(true)?;
        match &mut self.game {
            Game::Spatial(game) => {
                incant_render_game::tick_recorded_ui(game, physical, records, viewport, output)
                    .map_err(native)
            }
            Game::Recording(game) => {
                incant_render_game::tick_recorded_ui(game, physical, records, viewport, output)
                    .map_err(native)
            }
            Game::Plain(_) => unreachable!("UI admission checked"),
        }
    }
    pub fn is_spatial(&self) -> bool {
        matches!(self.game, Game::Spatial(_) | Game::Recording(_))
    }
}

enum Game {
    Plain(Box<OfflineGameSession>),
    Spatial(Box<HostedOfflineGameSession<NativeRenderHost>>),
    Recording(Box<HostedOfflineGameSession<NativeRenderHost<MaterialRecordingConsumer>>>),
}
impl Game {
    fn new(
        scene: CookedScene,
        code: SessionProgram,
        actions: InputActions,
        clips: Vec<ClipBinding>,
        rate: u32,
        settings: Option<NativeRenderSettings>,
        recording: Option<RecordingContext>,
    ) -> Result<Self> {
        if let Some(context) = recording {
            let settings = settings.ok_or(SessionError::Recording(
                "recording requires a spatial camera host",
            ))?;
            return Ok(Self::Recording(Box::new(
                HostedOfflineGameSession::new(
                    scene,
                    code,
                    actions,
                    clips,
                    rate,
                    NativeRenderSettings {
                        consumer: context,
                        camera: settings.camera,
                        model_cache: settings.model_cache,
                        texture_cache: settings.texture_cache,
                        text: settings.text,
                        ui: settings.ui,
                    },
                )
                .map_err(native)?,
            )));
        }
        if let Some(settings) = settings {
            Ok(Self::Spatial(Box::new(
                HostedOfflineGameSession::new(scene, code, actions, clips, rate, settings)
                    .map_err(native)?,
            )))
        } else {
            Ok(Self::Plain(Box::new(
                OfflineGameSession::new(scene, code, actions, clips, rate).map_err(native)?,
            )))
        }
    }
    fn world(&self) -> Result<&NativeWorld> {
        match self {
            Self::Plain(g) => g.world().map_err(native),
            Self::Spatial(g) => g.world().map_err(native),
            Self::Recording(g) => g.world().map_err(native),
        }
    }
    fn input(&self) -> Result<&InputFrame> {
        match self {
            Self::Plain(g) => g.input().map_err(native),
            Self::Spatial(g) => g.input().map_err(native),
            Self::Recording(g) => g.input().map_err(native),
        }
    }
    fn logs(&self) -> Result<&NativeLogs> {
        match self {
            Self::Plain(g) => g.logs().map_err(native),
            Self::Spatial(g) => g.logs().map_err(native),
            Self::Recording(g) => g.logs().map_err(native),
        }
    }
    fn next_frames(&self) -> Result<usize> {
        match self {
            Self::Plain(g) => g.next_output_frames().map_err(native),
            Self::Spatial(g) => g.next_output_frames().map_err(native),
            Self::Recording(g) => g.next_output_frames().map_err(native),
        }
    }
    fn frames(&self) -> Result<u128> {
        match self {
            Self::Plain(g) => g.rendered_frames().map_err(native),
            Self::Spatial(g) => g.rendered_frames().map_err(native),
            Self::Recording(g) => g.rendered_frames().map_err(native),
        }
    }
    fn tick(&mut self, input: &[InputEvent], output: &mut [f32]) -> Result<()> {
        match self {
            Self::Plain(g) => g.tick(input, output).map_err(native),
            Self::Spatial(g) => g.tick(input, output).map_err(native),
            Self::Recording(g) => g.tick(input, output).map_err(native),
        }
    }
}
