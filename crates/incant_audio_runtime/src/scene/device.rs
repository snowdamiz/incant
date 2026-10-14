//! Device notifications are distinct from decoder failures and device loss.
use crate::{AudioError, DeviceBackend, Result};
use kira::backend::cpal::cpal::{Error, ErrorKind};

// Kira's handled-error queue has 64 entries. Bound each host poll even if its
// producer continues filling the queue while we consume it.
const MAX_EVENTS_PER_POLL: usize = 64;

/// Observed notifications for this audio owner, reset only by owner replacement.
/// Xruns may have caused audible glitches. Zero notifications do not prove device
/// liveness or glitch-free output, and these counters are not game-save state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct DeviceAudioDiagnostics {
    pub xrun_notifications: u64,
    /// Polls that consumed their entire allowance; more events may remain.
    pub poll_limit_hits: u64,
    pub counters_saturated: bool,
}

trait ErrorQueue {
    fn discarded(&self) -> Option<u64>;
    fn pop(&mut self) -> Option<Error>;
}

impl ErrorQueue for DeviceBackend {
    fn discarded(&self) -> Option<u64> {
        self.num_stream_errors_discarded()
    }
    fn pop(&mut self) -> Option<Error> {
        self.pop_error()
    }
}

impl DeviceAudioDiagnostics {
    pub(super) fn poll(&mut self, backend: &mut DeviceBackend) -> Result<()> {
        self.poll_queue(backend)
    }

    fn poll_queue(&mut self, queue: &mut impl ErrorQueue) -> Result<()> {
        check_queue(queue)?;
        let mut drained = 0;
        while drained < MAX_EVENTS_PER_POLL {
            let Some(error) = queue.pop() else { break };
            drained += 1;
            // CPAL's typed Xrun reports a missed buffer deadline. Its CoreAudio
            // overload listener does not stop the stream, and Kira preserves it.
            // Never infer recoverability from an error's formatted message.
            if error.kind() != ErrorKind::Xrun {
                return Err(AudioError::Backend(error.to_string()));
            }
            increment(&mut self.xrun_notifications, &mut self.counters_saturated);
        }
        if drained == MAX_EVENTS_PER_POLL {
            increment(&mut self.poll_limit_hits, &mut self.counters_saturated);
        }
        // Loss during this poll is terminal too: a discarded event may have
        // described device loss. An xrun cannot hide a later non-xrun error.
        check_queue(queue)
    }
}

fn check_queue(queue: &impl ErrorQueue) -> Result<()> {
    if queue.discarded() != Some(0) {
        Err(AudioError::Backend(
            "device error queue unavailable or errors discarded".into(),
        ))
    } else {
        Ok(())
    }
}

fn increment(value: &mut u64, saturated: &mut bool) {
    if let Some(next) = value.checked_add(1) {
        *value = next;
    } else {
        *saturated = true;
    }
}

#[cfg(test)]
mod tests;
