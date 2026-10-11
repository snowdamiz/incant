//! Scalar controls for existing final audio declarations. Mixer cursors remain
//! in the retained audio host; these commands never contact a backend directly.
use crate::{
    CookedAudio, FrameCommands, NativeWorld, RuntimeError, SceneError, StableId, StructuralCommand,
};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AudioControl {
    /// Desired pause/resume state. An unchanged true value does not replay an
    /// ended voice; false then true at separate sync points creates a new edge.
    SetPlaying(bool),
    SourceGain(f32),
    BusGain(f32),
}

impl NativeWorld {
    /// Controls use the final declaration, and compose in submitted order.
    /// Work scales with the bounded command/audio set, never all ECS entities.
    pub(crate) fn prepare_audio_controls(
        &self,
        commands: &FrameCommands,
        mut changes: BTreeMap<StableId, Option<CookedAudio>>,
        exists: impl Fn(StableId) -> bool,
    ) -> Result<BTreeMap<StableId, Option<CookedAudio>>, RuntimeError> {
        for command in commands.iter() {
            let StructuralCommand::ControlAudio { entity, control } = command else {
                continue;
            };
            if !exists(*entity) {
                return Err(RuntimeError::MissingEntity(*entity));
            }
            if let AudioControl::SourceGain(db) | AudioControl::BusGain(db) = control
                && !(-80. ..=24.).contains(db)
            {
                return Err(SceneError::Invalid(
                    "audio control gain must be finite in [-80,24] dB",
                )
                .into());
            }
            let node = changes
                .entry(*entity)
                .or_insert_with(|| {
                    self.entities
                        .get(entity)
                        .and_then(|h| self.world.get::<CookedAudio>(*h))
                        .cloned()
                })
                .as_mut()
                .ok_or(SceneError::Invalid(
                    "audio control requires final audio declaration",
                ))?;
            match *control {
                AudioControl::SetPlaying(playing) => {
                    node.source
                        .as_mut()
                        .ok_or(SceneError::Invalid("audio control requires final source"))?
                        .playing = playing;
                }
                AudioControl::SourceGain(db) => {
                    node.source
                        .as_mut()
                        .ok_or(SceneError::Invalid("audio control requires final source"))?
                        .gain_db = db;
                }
                AudioControl::BusGain(db) => {
                    node.bus
                        .as_mut()
                        .ok_or(SceneError::Invalid("audio control requires final bus"))?
                        .gain_db = db;
                }
            }
        }
        Ok(changes)
    }
}
