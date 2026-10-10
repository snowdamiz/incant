//! Versioned logical game state, independent of editor history and VM handles.
//! Loading reconstructs physics and the VM; this is not a rollback snapshot.
use super::{PlaySession, ScriptError};
use incant_doc::Project;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const MAX_SAVE_BYTES: usize = 18 * 1024 * 1024;
const MAX_PROJECT_BYTES: usize = 16 * 1024 * 1024;
const MAX_STATE_BYTES: usize = 1024 * 1024;
pub(super) const MAX_SAVE_TICK: u64 = (1 << 53) - 1;
const FORMAT: &str = "incant-game-save";
const VERSION: u32 = 2;

/// The published wire schema. Decode through `PlaySession::from_save` to apply
/// semantic checks; serde alone does not establish a valid game save.
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GameSave {
    format: String,
    version: u32,
    authored_sha256: String,
    script_sha256: String,
    tick: u64,
    elapsed_seconds: f64,
    project: Project,
    script_state: Value,
    /// Version 1 has no schedule; version 2 requires this data-only schedule.
    #[serde(default)]
    schedule: Option<super::timers::Schedule>,
}

#[derive(Debug, Error)]
pub enum SaveError {
    #[error("unsupported game-save format or version")]
    Version,
    #[error("game save belongs to a different authored project revision")]
    ProjectMismatch,
    #[error("game save requires the same compiled behavior revision")]
    ScriptMismatch,
    #[error("game-save project, script state or envelope exceeds its size limit")]
    Size,
    #[error("game-save clock is invalid")]
    Clock,
    #[error(transparent)]
    Timer(#[from] super::TimerError),
    #[error("game-save loading cannot change project identity, settings or resource manifests")]
    Manifest,
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Document(#[from] incant_doc::DocumentError),
    #[error(transparent)]
    Script(#[from] ScriptError),
}

pub(super) fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub(super) fn manifest(project: &Project) -> Result<String, serde_json::Error> {
    Ok(hash(&serde_json::to_vec(&(
        &project.id,
        &project.settings,
        &project.assets,
        &project.scripts,
    ))?))
}

impl PlaySession {
    /// Save only after a complete successful tick (or before the first tick).
    /// Does not consume logs, write files or mutate the author's project.
    pub fn save_text(&self) -> Result<String, SaveError> {
        if self.failed {
            return Err(ScriptError::FailedSession.into());
        }
        let save = GameSave {
            format: FORMAT.into(),
            version: VERSION,
            authored_sha256: self.authored_sha256.clone(),
            script_sha256: self.host.source_sha256.clone(),
            tick: self.ticks,
            elapsed_seconds: self.elapsed_seconds,
            project: self.project().clone(),
            script_state: self.host.state.clone(),
            schedule: Some(self.host.schedule.clone()),
        };
        save.validate(&self.manifest_sha256)?;
        let text = serde_json::to_string(&save)?;
        if text.len() > MAX_SAVE_BYTES {
            return Err(SaveError::Size);
        }
        Ok(text)
    }

    /// Restore into a new session, leaving any existing session and authored
    /// document untouched on both success and failure. Save files contain data,
    /// never executable source or filesystem capabilities. Physics contacts,
    /// sleeping state and VM globals are reconstructed, not deserialized.
    pub fn from_save(
        authored: &Project,
        compiled_source: &str,
        text: &str,
    ) -> Result<Self, SaveError> {
        if text.len() > MAX_SAVE_BYTES {
            return Err(SaveError::Size);
        }
        let save: GameSave = serde_json::from_str(text)?;
        if save.format != FORMAT || !matches!(save.version, 1 | VERSION) {
            return Err(SaveError::Version);
        }
        let authored_sha256 = hash(authored.canonical_text()?.as_bytes());
        if save.authored_sha256 != authored_sha256 {
            return Err(SaveError::ProjectMismatch);
        }
        if save.script_sha256 != hash(compiled_source.as_bytes()) {
            return Err(SaveError::ScriptMismatch);
        }
        let manifest_sha256 = manifest(authored)?;
        save.validate(&manifest_sha256)?;
        let mut next = Self::new(&save.project, compiled_source)?;
        next.authored_sha256 = authored_sha256;
        next.manifest_sha256 = manifest_sha256;
        next.ticks = save.tick;
        next.elapsed_seconds = save.elapsed_seconds;
        next.host.state = save.script_state;
        next.host.schedule = save
            .schedule
            .unwrap_or_else(|| super::timers::Schedule::at(save.tick, save.elapsed_seconds));
        if next.host.schedule.has_timers() && !next.host.has_timer_handler {
            return Err(ScriptError::TimerHandler.into());
        }
        Ok(next)
    }
}

impl GameSave {
    fn validate(&self, expected_manifest: &str) -> Result<(), SaveError> {
        match (&self.schedule, self.version) {
            (None, 1) => {}
            (Some(schedule), VERSION) => {
                schedule.validate()?;
                if schedule.clock.tick != self.tick
                    || schedule.clock.elapsed_seconds != self.elapsed_seconds
                {
                    return Err(SaveError::Clock);
                }
            }
            _ => return Err(SaveError::Version),
        }
        self.project.validate()?;
        if manifest(&self.project)? != expected_manifest {
            return Err(SaveError::Manifest);
        }
        if serde_json::to_vec(&self.project)?.len() > MAX_PROJECT_BYTES
            || serde_json::to_vec(&self.script_state)?.len() > MAX_STATE_BYTES
        {
            return Err(SaveError::Size);
        }
        let seconds = self.tick as f64 / f64::from(self.project.settings.tick_rate);
        // Repeated f64 addition accumulates rounding. Retain its exact value for
        // continuity while rejecting impossible or grossly inconsistent clocks.
        if self.tick > MAX_SAVE_TICK
            || !self.elapsed_seconds.is_finite()
            || self.elapsed_seconds < 0.
            || (self.elapsed_seconds - seconds).abs() > seconds.max(1.) * 1e-6
        {
            return Err(SaveError::Clock);
        }
        Ok(())
    }
}
