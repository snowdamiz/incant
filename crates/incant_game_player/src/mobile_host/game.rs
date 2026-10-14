//! Keep lifecycle/input fencing identical for offline and actual device audio.
use incant_game_runtime::{GameError, HostedOfflineGameSession};
use incant_input::InputEvent;
use incant_render_game::{NativeRenderHost, RenderHostError, RenderInputConsumer};
use incant_runtime::NativeWorld;
use incant_ui::UiEvent;

type Result<T, C> =
    std::result::Result<T, GameError<RenderHostError<<C as RenderInputConsumer>::Error>>>;
pub(super) enum Game<C: RenderInputConsumer> {
    Offline(Box<HostedOfflineGameSession<NativeRenderHost<C>>>),
    #[cfg(feature = "mobile-device")]
    Device(
        Box<
            incant_game_runtime::HostedGameSession<
                incant_audio_runtime::DeviceBackend,
                NativeRenderHost<C>,
            >,
        >,
    ),
}

macro_rules! dispatch {
    ($self:expr, $value:ident => $body:expr) => {
        match $self {
            Game::Offline($value) => $body,
            #[cfg(feature = "mobile-device")]
            Game::Device($value) => $body,
        }
    };
}
impl<C: RenderInputConsumer> Game<C> {
    pub fn is_failed(&self) -> bool {
        dispatch!(self, value => value.is_failed())
    }
    pub fn world(&self) -> Result<&NativeWorld, C> {
        dispatch!(self, value => value.world())
    }
    pub fn host(&self) -> Result<&NativeRenderHost<C>, C> {
        dispatch!(self, value => value.host())
    }
    #[cfg(test)]
    pub fn input(&self) -> Result<&incant_input::InputFrame, C> {
        dispatch!(self, value => value.input())
    }
    pub fn set_suspended(&mut self, suspended: bool) -> Result<(), C> {
        dispatch!(self, value => value.set_suspended(suspended))
    }
    pub fn resize_ui(&mut self, size: [f32; 2]) -> Result<(), C> {
        dispatch!(self, value => value.resize_ui(size))
    }
    pub fn next_output_frames(&self) -> Result<usize, C> {
        match self {
            Self::Offline(game) => game.next_output_frames(),
            #[cfg(feature = "mobile-device")]
            Self::Device(game) => game.world().map(|_| 0),
        }
    }
    pub fn tick_with_ui(
        &mut self,
        events: &[InputEvent],
        ui: &[UiEvent],
        output: &mut [f32],
    ) -> Result<(), C> {
        match self {
            Self::Offline(game) => game.tick_with_ui(events, ui, output),
            #[cfg(feature = "mobile-device")]
            Self::Device(game) => {
                if !output.is_empty() {
                    return Err(GameError::Buffer {
                        expected: 0,
                        actual: output.len(),
                    });
                }
                game.poll_device()?;
                game.tick_with_ui(events, ui)
            }
        }
    }
}
