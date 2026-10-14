//! Explicit fixed-host-step physical input. Presentation retries and OS pauses
//! never consume a recording tick; a game restart never rewinds the recording.
use incant_input::{InputEvent, InputReplay};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("native window recording requires an explicit --stop-after bound")]
    Unbounded,
    #[error("recording does not cover the requested native host steps")]
    Range,
    #[error("native window replay does not yet support recorded UI or viewport changes")]
    UiUnsupported,
    #[error(transparent)]
    Recording(#[from] incant_input::RecordingError),
}

pub struct Replay {
    input: InputReplay,
    pub sha256: String,
}
impl Replay {
    pub fn parse(text: &str, rate: u32, stop_after: Option<u64>) -> Result<Self, Error> {
        let stop = stop_after.ok_or(Error::Unbounded)?;
        let input = InputReplay::from_text(text, 0, rate)?;
        if stop == 0 || stop > input.end_tick() {
            return Err(Error::Range);
        }
        // A native resize is asynchronous and must not silently substitute the
        // current platform layout for the recorded one. Refuse before devices.
        if input.has_ui() {
            return Err(Error::UiUnsupported);
        }
        Ok(Self {
            input,
            sha256: incant_assets_runtime::sha256(text.as_bytes()),
        })
    }
    pub fn events(&self, accepted_host_steps: u64) -> Result<&[InputEvent], Error> {
        let next = accepted_host_steps.checked_add(1).ok_or(Error::Range)?;
        Ok(self.input.events_at(next)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use incant_input::InputRuntime;
    fn clip() -> serde_json::Value {
        serde_json::json!({"format":"incant-input","version":1,"start_tick":0,
        "tick_rate":60,"ticks":5,"frames":[
            {"tick":1,"events":[{"type":"gamepad_connected","id":0},
                {"type":"gamepad_axis","id":0,"axis":"left_x","value":0.75}]},
            {"tick":3,"events":[{"type":"gamepad_axis","id":0,"axis":"left_x","value":0.75}]},
            {"tick":4,"events":[{"type":"gamepad_axis","id":0,"axis":"left_x","value":0.0}]},
            {"tick":5,"events":[{"type":"gamepad_axis","id":0,"axis":"left_x","value":-0.5}]}
        ]})
    }
    #[test]
    fn repeated_presentation_reads_do_not_advance_and_restart_does_not_rewind() {
        let text = clip().to_string();
        let replay = Replay::parse(&text, 60, Some(5)).unwrap();
        assert_eq!(
            replay.sha256,
            incant_assets_runtime::sha256(text.as_bytes())
        );
        assert_eq!(replay.events(0).unwrap(), replay.events(0).unwrap());
        let mut fence = crate::restart::InputFence::default();
        let mut state = InputRuntime::default();
        let mut values = Vec::new();
        for step in 0..5 {
            state
                .advance(
                    fence.filter(replay.events(step).unwrap()).unwrap(),
                    1. / 60.,
                )
                .unwrap();
            values.push(state.frame().gamepads[&0].left_stick[0]);
            if step == 1 {
                fence.replace_session();
                state = InputRuntime::default();
            }
        }
        assert_eq!(values, [0.75, 0.75, 0., 0., -0.5]);
        assert!(replay.events(5).is_err());
        assert!(replay.events(u64::MAX).is_err());
    }
    #[test]
    fn invalid_or_unsupported_recordings_are_rejected_before_device_setup() {
        let original = clip();
        assert!(matches!(
            Replay::parse(&original.to_string(), 60, None),
            Err(Error::Unbounded)
        ));
        for bound in [0, 6] {
            assert!(matches!(
                Replay::parse(&original.to_string(), 60, Some(bound)),
                Err(Error::Range)
            ));
        }
        assert!(Replay::parse(&original.to_string(), 30, Some(5)).is_err());
        let mut shifted = original.clone();
        shifted["start_tick"] = 1.into();
        assert!(Replay::parse(&shifted.to_string(), 60, Some(5)).is_err());
        let mut ui = original.clone();
        ui["version"] = 2.into();
        ui["frames"][0]["viewport"] = serde_json::json!([640, 480]);
        assert!(matches!(
            Replay::parse(&ui.to_string(), 60, Some(5)),
            Err(Error::UiUnsupported)
        ));
        ui["frames"][0].as_object_mut().unwrap().remove("viewport");
        ui["frames"][0]["ui"] = serde_json::json!([{"type":"activate"}]);
        assert!(matches!(
            Replay::parse(&ui.to_string(), 60, Some(5)),
            Err(Error::UiUnsupported)
        ));
        let mut invalid = original;
        invalid["frames"][0]["events"][1]["value"] = 2.into();
        assert!(Replay::parse(&invalid.to_string(), 60, Some(5)).is_err());
    }
}
