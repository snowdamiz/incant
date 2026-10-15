//! Publication follows the successful candidate world, never global Capture.
use super::{Controller, Result};
use serde_json::Value;
#[cfg(feature = "agent-profiler")]
use {super::Error, serde_json::json};

pub(super) struct Pending {
    #[cfg(feature = "agent-profiler")]
    capture: Option<incant_profile::session::Capture>,
}
impl Pending {
    pub fn new(enabled: bool) -> Result<Self> {
        #[cfg(feature = "agent-profiler")]
        return Ok(Self {
            capture: enabled
                .then(incant_profile::session::Capture::new)
                .transpose()?,
        });
        #[cfg(not(feature = "agent-profiler"))]
        {
            if enabled {
                return Err(incant_profile::Error::Disabled.into());
            }
            Ok(Self {})
        }
    }
    pub fn frame<T, E>(
        &mut self,
        _tick: u64,
        run: impl FnOnce() -> std::result::Result<T, E>,
    ) -> Result<std::result::Result<T, E>> {
        #[cfg(feature = "agent-profiler")]
        if let Some(capture) = &mut self.capture {
            return Ok(capture.frame(_tick, run)?);
        }
        Ok(run())
    }
    pub fn finish(self, _project_id: String) -> Result<Option<Completed>> {
        #[cfg(feature = "agent-profiler")]
        return self
            .capture
            .map(|c| {
                Ok(Completed {
                    recording: c.finish()?,
                    project_id: _project_id,
                    id: ulid::Ulid::new().to_string(),
                })
            })
            .transpose();
        #[cfg(not(feature = "agent-profiler"))]
        Ok(None)
    }
}
pub(super) struct Completed {
    #[cfg(feature = "agent-profiler")]
    recording: incant_profile::session::Recording,
    #[cfg(feature = "agent-profiler")]
    project_id: String,
    #[cfg(feature = "agent-profiler")]
    id: String,
}
impl Completed {
    pub fn identity(&self) -> Value {
        #[cfg(feature = "agent-profiler")]
        return json!({"id":self.id,"retained_capacity":incant_profile::session::MAX_FRAMES,"cpu_only":true});
        #[cfg(not(feature = "agent-profiler"))]
        unreachable!("disabled profile cannot be created")
    }
}
impl Controller {
    pub fn profiler_read(
        &self,
        _project_id: &str,
        _revision: u64,
        _frames: usize,
    ) -> Result<Value> {
        #[cfg(feature = "agent-profiler")]
        {
            let live = self
                .live
                .as_ref()
                .ok_or(Error::Invalid("no accepted play session"))?;
            let profile = live
                .profile
                .as_ref()
                .ok_or(Error::Invalid("accepted play_run did not enable profiling"))?;
            if profile.project_id != _project_id {
                return Err(Error::Invalid("profile belongs to a different project"));
            }
            let current_tick = live.session.world()?.tick();
            let report = profile.recording.read(_frames)?;
            let value = json!({"profile_id":profile.id,"project_id":profile.project_id,
                "started_revision":live.started_revision,"current_revision":_revision,
                "scene":live.session.scene,"script_source_sha256":live.session.script_source_sha256,
                "current_session_tick":current_tick,"measurements":report,
                "limitations":["CPU observations include instrumentation overhead; no shipping or performance gate verdict",
                    "Nested phase/source timings are inclusive and must not be summed",
                    "No GPU, display, worker-thread, allocation or heap measurements"]});
            if serde_json::to_vec(&value)?.len() > 256 * 1024 {
                return Err(Error::Invalid("profile response exceeds 256 KiB"));
            }
            Ok(value)
        }
        #[cfg(not(feature = "agent-profiler"))]
        Err(incant_profile::Error::Disabled.into())
    }
}
