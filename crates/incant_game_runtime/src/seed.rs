//! Immutable native startup data. No live world, VM, input, audio or host state
//! is retained here, and no checkpoint decoder participates in reconstruction.
use crate::{GameError, Result};
use incant_audio_runtime::{ClipBinding, MAX_SCENE_CLIPS};
use incant_input::{InputActions, InputError, ValidatedActions};
use incant_runtime::CookedScene;
use incant_script::{CompiledScript, SessionError};

/// Trusted compiled native program paired with the original typed startup data.
/// Cold reconstruction uses the ordinary constructors and repeats their exact
/// scene/program/clip/consumer checks before exposing a fresh session. Runtime
/// physics is rebuilt from its initial declarations, never from a lossy save.
/// Static/development Wasm restart request parity is deliberately not implied.
pub struct GameSeed {
    pub(crate) scene: CookedScene,
    pub(crate) program: CompiledScript,
    pub(crate) actions: InputActions,
    pub(crate) clips: Vec<ClipBinding>,
}
impl GameSeed {
    pub fn new(
        scene: CookedScene,
        program: CompiledScript,
        actions: InputActions,
        clips: Vec<ClipBinding>,
    ) -> Result<Self> {
        if clips.len() > MAX_SCENE_CLIPS {
            return Err(GameError::Audio(incant_audio_runtime::AudioError::Capacity));
        }
        ValidatedActions::new(actions.clone())
            .map_err(InputError::from)
            .map_err(SessionError::from)?;
        Ok(Self {
            scene,
            program,
            actions,
            clips,
        })
    }
    pub fn tick_rate(&self) -> u32 {
        self.scene.tick_rate()
    }
}
