//! Stop game delivery before native event-loop shutdown callbacks can reenter a
//! failed session. Teardown still runs, but cannot replace the original failure.
use crate::session;

#[derive(Default)]
pub(super) struct Termination {
    requested: bool,
    failure: Option<Box<dyn std::error::Error>>,
}

impl Termination {
    pub(super) fn requested(&self) -> bool {
        self.requested
    }

    pub(super) fn event(&mut self, deliver: impl FnOnce(&mut bool) -> session::Result<()>) {
        if self.requested {
            return;
        }
        if let Err(error) = deliver(&mut self.requested) {
            self.failure = Some(error);
            self.requested = true;
        }
    }

    pub(super) fn finish(
        self,
        code: i32,
        cleanup: impl FnOnce() -> session::Result<()>,
    ) -> session::Result<()> {
        let cleanup_result = cleanup();
        if let Some(error) = self.failure {
            return Err(error);
        }
        if code != 0 {
            return Err(format!("native event loop failed with code{code}").into());
        }
        cleanup_result
    }
}

#[cfg(test)]
mod tests {
    use super::Termination;
    use std::cell::Cell;

    #[derive(Debug, thiserror::Error)]
    #[error("original frame failure")]
    struct FrameFailure;

    #[test]
    fn failed_frame_stops_shutdown_delivery_and_preserves_typed_cause_through_cleanup() {
        let mut termination = Termination::default();
        let frames = Cell::new(0);
        let cleanup_calls = Cell::new(0);
        termination.event(|_| {
            frames.set(frames.get() + 1);
            Err(FrameFailure.into())
        });
        assert!(termination.requested());
        // A native loop may still deliver focus, destruction and LoopDestroyed
        // callbacks after Exit. None may query or tick the now-failed session.
        for _ in 0..3 {
            termination.event(|_| {
                frames.set(frames.get() + 1);
                Err("generic session already failed".into())
            });
        }
        let error = termination
            .finish(7, || {
                cleanup_calls.set(cleanup_calls.get() + 1);
                Err("text session retirement failed".into())
            })
            .unwrap_err();
        assert!(error.downcast_ref::<FrameFailure>().is_some());
        assert_eq!(frames.get(), 1);
        assert_eq!(cleanup_calls.get(), 1);
    }

    #[test]
    fn explicit_stop_skips_later_events_but_retires_native_input() {
        let mut termination = Termination::default();
        termination.event(|stop| {
            *stop = true;
            Ok(())
        });
        termination.event(|_| panic!("delivery after explicit close or stop-after"));
        let retired = Cell::new(false);
        termination
            .finish(0, || {
                retired.set(true);
                Ok(())
            })
            .unwrap();
        assert!(retired.get());
    }

    #[test]
    fn clean_callback_keeps_delivery_open_and_teardown_failure_visible() {
        let mut termination = Termination::default();
        let delivered = Cell::new(0);
        for _ in 0..2 {
            termination.event(|_| {
                delivered.set(delivered.get() + 1);
                Ok(())
            });
        }
        assert!(!termination.requested());
        assert_eq!(delivered.get(), 2);
        let error = termination
            .finish(0, || Err("text session retirement failed".into()))
            .unwrap_err();
        assert_eq!(error.to_string(), "text session retirement failed");
    }

    #[test]
    fn native_loop_failure_precedes_cleanup_failure() {
        let error = Termination::default()
            .finish(9, || Err("text session retirement failed".into()))
            .unwrap_err();
        assert_eq!(error.to_string(), "native event loop failed with code9");
    }
}
