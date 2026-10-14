//! Probe-only copies of native playback and a real committed-refusal/recovery.
use super::*;
use incant_runtime::AnimationState;
use incant_types::Animation;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(in crate::play) struct AnimationObservation {
    playback: Animation,
    completed: bool,
    transition: Option<Transition>,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
struct Transition {
    playback: Animation,
    elapsed: f64,
    duration: f64,
}
impl From<AnimationState> for AnimationObservation {
    fn from(state: AnimationState) -> Self {
        Self {
            playback: Animation {
                clip: state.sample.clip,
                seconds: state.sample.seconds,
                speed: state.speed,
                playing: state.playing,
                looping: state.looping,
            },
            completed: state.completed,
            transition: state.transition.map(|t| Transition {
                playback: Animation {
                    clip: t.source.clip,
                    seconds: t.source.seconds,
                    speed: t.speed,
                    playing: t.playing,
                    looping: t.looping,
                },
                elapsed: t.elapsed,
                duration: t.duration,
            }),
        }
    }
}
pub(super) fn validate_fixture_state(state: Option<&AnimationObservation>) -> Result<(), String> {
    let state = state.ok_or("Animated probe requires a native playback state")?;
    let p = state.playback;
    require(
        p.clip == Some(0)
            && p.seconds.is_finite()
            && (0. ..2.).contains(&p.seconds)
            && p.speed == 1.
            && p.playing
            && p.looping
            && !state.completed
            && state.transition.is_none(),
        "Animated probe playback does not match its fixed fixture",
    )
}
pub(super) fn validate_at_tick(
    state: Option<&AnimationObservation>,
    tick: u64,
) -> Result<(), String> {
    validate_fixture_state(state)?;
    require(
        (state.unwrap().playback.seconds - (0.5 + tick as f64 / 60.).rem_euclid(2.)).abs() < 1e-9,
        "Animation clock does not match the authored starting time and native tick",
    )
}
pub(super) fn continued(
    before: Option<&AnimationObservation>,
    after: Option<&AnimationObservation>,
) -> Result<(), String> {
    validate_fixture_state(before)?;
    validate_fixture_state(after)?;
    let (before, after) = (before.unwrap(), after.unwrap());
    let expected = (before.playback.seconds + 1. / 60.).rem_euclid(2.);
    require(
        (after.playback.seconds - expected).abs() < 1e-9,
        "Animation did not continue from the paused clock on the next step",
    )
}

pub(super) struct Recovery {
    pub refusal: Value,
    pub authoring: (Value, u64, usize),
}
fn wait_commit(
    editor: &Editor,
    asset: &str,
    revision: u64,
    expected: Option<&str>,
    prior: &str,
) -> Result<(Value, u64, usize), String> {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let snapshot = project(editor)?;
        require(
            snapshot.1 <= revision,
            "Watcher appended more than one controlled transaction",
        )?;
        let fingerprint = snapshot.0["assets"][asset]["sha256"]
            .as_str()
            .ok_or("Watched model missing")?;
        if snapshot.1 == revision
            && expected.map_or(fingerprint != prior, |value| fingerprint == value)
        {
            require(
                history(editor)?["transactions"]
                    .as_array()
                    .and_then(|h| h.last())
                    .is_some_and(|t| t["actor"]["origin"] == "import"),
                "Animation source watcher lost import provenance",
            )?;
            return Ok(snapshot);
        }
        require(
            Instant::now() < deadline,
            "Animation watcher commit deadline",
        )?;
        thread::sleep(Duration::from_millis(20));
    }
}
pub(super) fn refuse_and_recover(
    editor: &Editor,
    generation: u32,
    asset: ObservedAsset,
    fingerprint: &str,
    baseline: &Observation,
    replacement: &[u8],
    stages: &mut Vec<Value>,
) -> Result<Recovery, String> {
    let root = editor
        .asset_root
        .as_ref()
        .ok_or("Saved animation fixture required")?;
    let before = project(editor)?;
    let console_before = editor.console.lock().map_err(|_| "console lock")?.len();
    let invalid = fs::read(root.join("refresh-incompatible.gltf")).map_err(|e| e.to_string())?;
    fs::write(root.join(Kind::AnimatedModel.source()), invalid).map_err(|e| e.to_string())?;
    let asset_id = ulid::Ulid::from_bytes(asset.id.0).to_string();
    let committed = wait_commit(editor, &asset_id, before.1 + 1, None, fingerprint)?;
    let authored = committed.0["assets"][&asset_id]["sha256"]
        .as_str()
        .ok_or("Committed recipe absent")?;
    let deadline = Instant::now() + Duration::from_secs(20);
    let event = loop {
        let event = editor
            .console
            .lock()
            .map_err(|_| "console lock")?
            .iter()
            .skip(console_before)
            .find(|e| {
                e.level == "error"
                    && e.message.starts_with("Native Play asset refresh:")
                    && e.message
                        .contains("animation clip bindings or duration table changed")
            })
            .cloned();
        if let Some(event) = event {
            break serde_json::to_value(event).map_err(|e| e.to_string())?;
        }
        require(
            Instant::now() < deadline,
            "Committed animation refusal console deadline",
        )?;
        thread::sleep(Duration::from_millis(20));
    };
    let observed = catalog_frame(editor, generation, asset, fingerprint, Some(baseline))?;
    let mut retained = point(
        Kind::AnimatedModel.stage(4),
        &observed,
        fingerprint,
        history(editor)?,
    );
    retained["authored_fingerprint"] = json!(authored);
    retained["refresh_error"] = event.clone();
    stages.push(retained);
    fs::write(root.join(Kind::AnimatedModel.source()), replacement).map_err(|e| e.to_string())?;
    let recovered = wait_commit(editor, &asset_id, before.1 + 2, Some(fingerprint), authored)?;
    require(
        recovered.0 == before.0,
        "Animation recovery did not restore authored model recipe",
    )?;
    let observed = catalog_frame(editor, generation, asset, fingerprint, Some(baseline))?;
    stages.push(point(
        Kind::AnimatedModel.stage(5),
        &observed,
        fingerprint,
        history(editor)?,
    ));
    Ok(Recovery {
        refusal: json!({"committed_revision":committed.1,"authored_fingerprint":authored,
        "accepted_fingerprint":fingerprint,"event":event,"native_code_available":false}),
        authoring: recovered,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample(seconds: f64) -> AnimationObservation {
        AnimationObservation::from(AnimationState {
            sample: incant_runtime::AnimationSample {
                clip: Some(0),
                seconds,
            },
            speed: 1.,
            playing: true,
            looping: true,
            completed: false,
            transition: None,
        })
    }
    #[test]
    fn numeric_observer_preserves_complete_transition_without_advancing_state() {
        let mut native = AnimationState {
            sample: incant_runtime::AnimationSample {
                clip: Some(0),
                seconds: 0.75,
            },
            speed: 1.,
            playing: true,
            looping: true,
            completed: false,
            transition: None,
        };
        native.transition = Some(incant_runtime::AnimationTransition {
            source: native.sample,
            speed: 0.5,
            playing: true,
            looping: false,
            elapsed: 0.1,
            duration: 0.4,
        });
        let before = native;
        let observed = AnimationObservation::from(native);
        assert_eq!(native, before);
        assert_eq!(observed.transition.as_ref().unwrap().playback.seconds, 0.75);
        assert_eq!(observed.transition.as_ref().unwrap().duration, 0.4);
        assert!(validate_fixture_state(Some(&observed)).is_err());
    }
    #[test]
    fn next_step_continuation_accepts_clock_wrap_and_refuses_reset_missing_or_changed_playback() {
        let before = sample(1.99);
        let after = sample((1.99f64 + 1. / 60.).rem_euclid(2.));
        continued(Some(&before), Some(&after)).unwrap();
        assert!(continued(Some(&before), Some(&sample(0.5))).is_err());
        assert!(continued(Some(&before), None).is_err());
        let mut wrong = after.clone();
        wrong.playback.playing = false;
        assert!(continued(Some(&before), Some(&wrong)).is_err());
        validate_at_tick(Some(&sample(0.5 + 3. / 60.)), 3).unwrap();
        assert!(validate_at_tick(Some(&sample(0.5)), 3).is_err());
    }
}
