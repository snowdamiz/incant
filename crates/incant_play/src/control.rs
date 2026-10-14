//! Development transport over the real native game lifecycle. It retains no
//! authoring state; the editor thread owns the clock, surface and input queue.
use crate::PreparedPlay;
use incant_audio_runtime::AudioManagerSettings;
use incant_game_runtime::{GameSession, HostedGameSession};
use incant_input::{InputEvent, InputFrame};
use incant_render_game::{
    CpuInputObserver, NativeRenderHost, NativeRenderSettings, RenderInputConsumer,
};
use incant_runtime::NativeWorld;
use incant_script::NativeLogs;
use kira::backend::Backend;
use serde::Serialize;
use std::{error::Error, fmt::Debug};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayMode {
    Running,
    Paused,
    Stopped,
    Failed,
}

#[derive(Debug, thiserror::Error)]
pub enum PlayError {
    #[error("Play has stopped; start a new session")]
    Stopped,
    #[error("Play failed; start a new session")]
    Failed,
    #[error("Pause Play before stepping one tick")]
    NotPaused,
    #[error("Resume Play before advancing")]
    Paused,
    #[error("Native Play failed: {0}")]
    Native(#[source] Box<dyn Error>),
}
/// Refused candidates remain owned until the caller leaves its publication lock.
pub enum TexturePublishAttempt<C: incant_render_game::TextureRefreshConsumer> {
    Published(incant_render_game::TexturePublication<C>),
    Refused {
        error: PlayError,
        candidate: incant_render_game::PreparedHostTextureRefresh<C>,
    },
}
pub struct TexturePublicationRefusal<C: incant_render_game::TextureRefreshConsumer> {
    pub error: PlayError,
    pub candidate: incant_render_game::PreparedHostTextureRefresh<C>,
}
pub struct ReadyTexturePublication<'a, B: Backend, C: incant_render_game::TextureRefreshConsumer> {
    host: incant_game_runtime::PreparedHostUpdate<'a, B, NativeRenderHost<C>>,
    candidate: Option<incant_render_game::PreparedHostTextureRefresh<C>>,
}
impl<B: Backend, C: incant_render_game::TextureRefreshConsumer> ReadyTexturePublication<'_, B, C> {
    pub fn publish(&mut self) -> TexturePublishAttempt<C> {
        let request = incant_render_game::TextureRefreshOperation::Publish(
            self.candidate.take().expect("single publication"),
        );
        match self.host.update(request) {
            Ok(incant_render_game::TextureRefreshReply::Published(publication)) => {
                TexturePublishAttempt::Published(publication)
            }
            Ok(incant_render_game::TextureRefreshReply::Refused { error, candidate }) => {
                TexturePublishAttempt::Refused {
                    error: native(error),
                    candidate,
                }
            }
            Err((error, incant_render_game::TextureRefreshOperation::Publish(candidate))) => {
                TexturePublishAttempt::Refused {
                    error: native(error),
                    candidate,
                }
            }
            _ => unreachable!("typed native publication reply"),
        }
    }
}
type Result<T> = std::result::Result<T, PlayError>;
fn native(error: impl Error + 'static) -> PlayError {
    PlayError::Native(Box::new(error))
}

enum Game<B: Backend, C: RenderInputConsumer> {
    Plain(Box<GameSession<B>>),
    Spatial(Box<HostedGameSession<B, NativeRenderHost<C>>>),
}

/// Device/backend-driven native Play. New sessions start paused, so a host can
/// acquire a drawable and install input routing before allowing simulation.
/// Device audio suspension takes effect at the next callback boundary; startup
/// can emit samples before that command, and step is not sample-exact audio.
/// Stopping destroys the world, VM, renderer and audio together. Read-only world
/// access is intended for bounded explicit inspection, never a per-frame clone.
pub struct PlaySession<B: Backend, C: RenderInputConsumer = CpuInputObserver> {
    game: Option<Game<B, C>>,
    mode: PlayMode,
    pub scene: String,
    pub script_source_sha256: String,
    pub authored_scripts: bool,
}
impl<B: Backend, C: RenderInputConsumer> PlaySession<B, C>
where
    B::Error: Debug,
{
    pub fn new(
        prepared: PreparedPlay,
        audio: AudioManagerSettings<B>,
        consumer: C::Settings,
    ) -> Result<Self> {
        let game = if let Some(settings) = prepared.render {
            Game::Spatial(Box::new(
                HostedGameSession::new(
                    prepared.cooked,
                    prepared.code,
                    prepared.actions,
                    prepared.clips,
                    audio,
                    NativeRenderSettings {
                        ui: settings.ui,
                        text: settings.text,
                        model_cache: settings.model_cache,
                        texture_cache: settings.texture_cache,
                        camera: settings.camera,
                        consumer,
                    },
                )
                .map_err(native)?,
            ))
        } else {
            Game::Plain(Box::new(
                GameSession::new(
                    prepared.cooked,
                    prepared.code,
                    prepared.actions,
                    prepared.clips,
                    audio,
                )
                .map_err(native)?,
            ))
        };
        let mut result = Self {
            game: Some(game),
            mode: PlayMode::Paused,
            scene: prepared.scene,
            script_source_sha256: prepared.script_source_sha256,
            authored_scripts: prepared.authored_scripts,
        };
        result.suspend(true)?;
        Ok(result)
    }
    pub fn mode(&self) -> PlayMode {
        if self.game.as_ref().is_some_and(|game| match game {
            Game::Plain(game) => game.is_failed(),
            Game::Spatial(game) => game.is_failed(),
        }) {
            PlayMode::Failed
        } else {
            self.mode
        }
    }
    pub fn is_spatial(&self) -> bool {
        matches!(self.game, Some(Game::Spatial(_)))
    }
    pub fn stop(&mut self) {
        self.game = None;
        self.mode = PlayMode::Stopped;
    }
    fn ready(&self) -> Result<&Game<B, C>> {
        match self.mode() {
            PlayMode::Stopped => Err(PlayError::Stopped),
            PlayMode::Failed => Err(PlayError::Failed),
            _ => self.game.as_ref().ok_or(PlayError::Stopped),
        }
    }
    fn ready_mut(&mut self) -> Result<&mut Game<B, C>> {
        self.ready()?;
        self.game.as_mut().ok_or(PlayError::Stopped)
    }
    fn suspend(&mut self, value: bool) -> Result<()> {
        match self.ready_mut()? {
            Game::Plain(g) => g.set_suspended(value).map_err(native),
            Game::Spatial(g) => g.set_suspended(value).map_err(native),
        }
    }
    pub fn pause(&mut self) -> Result<()> {
        self.suspend(true)?;
        self.mode = PlayMode::Paused;
        Ok(())
    }
    pub fn resume(&mut self) -> Result<()> {
        self.suspend(false)?;
        self.mode = PlayMode::Running;
        Ok(())
    }
    pub fn advance(&mut self, input: &[InputEvent]) -> Result<()> {
        self.ready()?;
        if self.mode == PlayMode::Paused {
            return Err(PlayError::Paused);
        }
        self.tick(input)
    }
    fn tick(&mut self, input: &[InputEvent]) -> Result<()> {
        match self.ready_mut()? {
            Game::Plain(g) => g.tick(input).map_err(native),
            Game::Spatial(g) => g.tick(input).map_err(native),
        }
    }
    /// A retryable input refusal leaves the session paused. Any accepted-tick
    /// failure (or unwind) destroys the native composition; mode() reports Failed.
    pub fn step(&mut self, input: &[InputEvent]) -> Result<()> {
        self.ready()?;
        if self.mode != PlayMode::Paused {
            return Err(PlayError::NotPaused);
        }
        self.suspend(false)?;
        let result = self.tick(input);
        if self.mode() != PlayMode::Failed {
            self.suspend(true)?;
        }
        result
    }
    /// Present the committed world without advancing simulation or audio.
    /// Returns false for a scene without a spatial host.
    pub fn refresh(&mut self) -> Result<bool> {
        match self.ready_mut()? {
            Game::Plain(_) => Ok(false),
            Game::Spatial(g) => {
                g.refresh_host().map_err(native)?;
                Ok(true)
            }
        }
    }
    pub fn poll_audio(&mut self) -> Result<()> {
        match self.ready_mut()? {
            Game::Plain(g) => g.poll_audio().map_err(native),
            Game::Spatial(g) => g.poll_audio().map_err(native),
        }
    }
    pub fn world(&self) -> Result<&NativeWorld> {
        match self.ready()? {
            Game::Plain(g) => g.world().map_err(native),
            Game::Spatial(g) => g.world().map_err(native),
        }
    }
    pub fn consumer(&self) -> Result<Option<&C>> {
        match self.ready()? {
            Game::Plain(_) => Ok(None),
            Game::Spatial(g) => Ok(Some(g.host().map_err(native)?.consumer())),
        }
    }
    pub fn texture_refresh_ticket(&self) -> Result<incant_render_game::TextureRefreshTicket> {
        match self.ready()? {
            Game::Spatial(g) => Ok(g.host().map_err(native)?.texture_refresh_ticket()),
            Game::Plain(_) => Err(native(
                incant_game_runtime::GameError::<std::convert::Infallible>::SpatialHost,
            )),
        }
    }
    pub fn prepare_texture_refresh(
        &mut self,
        request: incant_render_game::PreparedTextureRefresh,
    ) -> Result<incant_render_game::PreparedHostTextureRefresh<C>>
    where
        C: incant_render_game::TextureRefreshConsumer,
    {
        match self.ready_mut()? {
            Game::Spatial(g) => match g
                .update_host(incant_render_game::TextureRefreshOperation::Prepare(
                    request,
                ))
                .map_err(native)?
            {
                incant_render_game::TextureRefreshReply::Prepared(prepared) => Ok(prepared),
                incant_render_game::TextureRefreshReply::PreparationRefused(error) => {
                    Err(native(error))
                }
                _ => unreachable!("native prepare reply"),
            },
            Game::Plain(_) => Err(native(
                incant_game_runtime::GameError::<std::convert::Infallible>::SpatialHost,
            )),
        }
    }
    pub fn publish_texture_refresh(
        &mut self,
        request: incant_render_game::PreparedHostTextureRefresh<C>,
    ) -> Result<TexturePublishAttempt<C>>
    where
        C: incant_render_game::TextureRefreshConsumer,
    {
        match self.prepare_texture_publication(request) {
            Ok(mut ready) => Ok(ready.publish()),
            Err(TexturePublicationRefusal { error, candidate }) => {
                Ok(TexturePublishAttempt::Refused { error, candidate })
            }
        }
    }
    /// Establish game/world readiness and retain all owners before an external
    /// final lock. Every early refusal returns the complete candidate intact.
    pub fn prepare_texture_publication(
        &mut self,
        candidate: incant_render_game::PreparedHostTextureRefresh<C>,
    ) -> std::result::Result<ReadyTexturePublication<'_, B, C>, TexturePublicationRefusal<C>>
    where
        C: incant_render_game::TextureRefreshConsumer,
    {
        if let Err(error) = self.ready() {
            return Err(TexturePublicationRefusal { error, candidate });
        }
        match self.game.as_mut().expect("checked game") {
            Game::Spatial(g) => match g.prepare_host_update() {
                Ok(host) => Ok(ReadyTexturePublication {
                    host,
                    candidate: Some(candidate),
                }),
                Err(error) => Err(TexturePublicationRefusal {
                    error: native(error),
                    candidate,
                }),
            },
            Game::Plain(_) => Err(TexturePublicationRefusal {
                error: native(
                    incant_game_runtime::GameError::<std::convert::Infallible>::SpatialHost,
                ),
                candidate,
            }),
        }
    }
    pub fn input(&self) -> Result<&InputFrame> {
        match self.ready()? {
            Game::Plain(g) => g.input().map_err(native),
            Game::Spatial(g) => g.input().map_err(native),
        }
    }
    pub fn logs(&self) -> Result<&NativeLogs> {
        match self.ready()? {
            Game::Plain(g) => g.logs().map_err(native),
            Game::Spatial(g) => g.logs().map_err(native),
        }
    }
}
#[cfg(all(feature = "device", not(target_arch = "wasm32")))]
impl<C: RenderInputConsumer> PlaySession<incant_audio_runtime::DeviceBackend, C> {
    pub fn poll_device(&mut self) -> Result<()> {
        match self.ready_mut()? {
            Game::Plain(g) => g.poll_device().map_err(native),
            Game::Spatial(g) => g.poll_device().map_err(native),
        }
    }
}
