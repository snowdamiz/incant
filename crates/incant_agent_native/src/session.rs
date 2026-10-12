//! One startup boundary shared by the native CLI and retained RPC play.
//! The session contains no authoring document, command bus or source text.
use crate::{FallbackCompiler, StartOptions};
use incant_audio_runtime::ClipBinding;
use incant_game_runtime::{HostedOfflineGameSession, OfflineGameSession};
use incant_input::{InputActions, InputEvent, InputFrame};
use incant_render_game::{NativeRenderHost, NativeRenderSettings};
use incant_runtime::{CookedScene, NativeWorld};
use incant_script::{NativeLogs, SessionProgram};
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum SessionError {
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
        let prepared =
            incant_play::PreparedPlay::load(document, root, options, sample_rate, fallback)
                .map_err(native)?;
        let game = Game::new(
            prepared.cooked,
            prepared.code,
            prepared.actions,
            prepared.clips,
            prepared.sample_rate,
            prepared.render,
        )?;
        Ok(Self {
            scene: prepared.scene,
            script_source_sha256: prepared.script_source_sha256,
            authored_scripts: prepared.authored_scripts,
            game,
        })
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
    pub fn is_spatial(&self) -> bool {
        matches!(self.game, Game::Spatial(_))
    }
}

enum Game {
    Plain(Box<OfflineGameSession>),
    Spatial(Box<HostedOfflineGameSession<NativeRenderHost>>),
}
impl Game {
    fn new(
        scene: CookedScene,
        code: SessionProgram,
        actions: InputActions,
        clips: Vec<ClipBinding>,
        rate: u32,
        settings: Option<NativeRenderSettings>,
    ) -> Result<Self> {
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
        }
    }
    fn input(&self) -> Result<&InputFrame> {
        match self {
            Self::Plain(g) => g.input().map_err(native),
            Self::Spatial(g) => g.input().map_err(native),
        }
    }
    fn logs(&self) -> Result<&NativeLogs> {
        match self {
            Self::Plain(g) => g.logs().map_err(native),
            Self::Spatial(g) => g.logs().map_err(native),
        }
    }
    fn next_frames(&self) -> Result<usize> {
        match self {
            Self::Plain(g) => g.next_output_frames().map_err(native),
            Self::Spatial(g) => g.next_output_frames().map_err(native),
        }
    }
    fn frames(&self) -> Result<u128> {
        match self {
            Self::Plain(g) => g.rendered_frames().map_err(native),
            Self::Spatial(g) => g.rendered_frames().map_err(native),
        }
    }
    fn tick(&mut self, input: &[InputEvent], output: &mut [f32]) -> Result<()> {
        match self {
            Self::Plain(g) => g.tick(input, output).map_err(native),
            Self::Spatial(g) => g.tick(input, output).map_err(native),
        }
    }
}
