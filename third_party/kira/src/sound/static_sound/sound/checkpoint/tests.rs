use super::*;
use crate::{info::MockInfoBuilder, sound::static_sound::StaticSoundSettings};
fn data(looping: bool, start: f64) -> StaticSoundData {
    let d = StaticSoundData {
        sample_rate: 22050,
        frames: (0..97)
            .map(|i| Frame::new((i as f32 * 0.07).sin() * 0.5, (i as f32 * 0.13).cos() * 0.3))
            .collect::<Vec<_>>()
            .into(),
        settings: StaticSoundSettings::default(),
        slice: None,
    }
    .playback_rate(1.13)
    .volume(-3.0)
    .panning(-0.17)
    .start_position(start);
    if looping {
        d.loop_region(..)
    } else {
        d
    }
}
fn run(sound: &mut dyn Sound, n: usize, rate: u32) -> Vec<Frame> {
    let mut out = vec![Frame::ZERO; n];
    sound.on_start_processing();
    sound.process(
        &mut out,
        1.0 / f64::from(rate),
        &MockInfoBuilder::new().build(),
    );
    out
}
#[test]
fn wrapper_preserves_original_math_and_fresh_resume_at_fractional_positions() {
    for rate in [32000, 44100, 48000] {
        for looping in [false, true] {
            for split in [0, 1, 2, 19, 145, 301] {
                let d = data(looping, 0.00037);
                let (mut reference, _) = d.clone().into_sound().unwrap();
                let (mut live, handle) = CheckpointData::new(d.clone())
                    .unwrap()
                    .into_sound()
                    .unwrap();
                assert_eq!(
                    run(&mut *reference, split, rate),
                    run(&mut *live, split, rate)
                );
                let state =
                    Checkpoint::from_bytes(handle.checkpoint().unwrap().as_bytes(), &d).unwrap();
                let (mut resumed, resumed_handle) = CheckpointData::resume(d, state)
                    .unwrap()
                    .into_sound()
                    .unwrap();
                for block in [1, 3, 7, 64, 128, 13] {
                    let expected = run(&mut *reference, block, rate);
                    assert_eq!(run(&mut *live, block, rate), expected);
                    assert_eq!(run(&mut *resumed, block, rate), expected);
                    assert_eq!(handle.checkpoint(), resumed_handle.checkpoint());
                }
            }
        }
    }
}
#[test]
fn pending_controls_refuse_then_paused_gain_rate_stop_states_resume_exactly() {
    let d = data(true, 0.0);
    let (mut live, mut handle) = CheckpointData::new(d.clone())
        .unwrap()
        .into_sound()
        .unwrap();
    run(&mut *live, 31, 48000);
    handle.configure(-9.0, 0.4, 0.37).unwrap();
    assert_eq!(handle.checkpoint(), Err(CheckpointError::Pending));
    run(&mut *live, 17, 48000);
    handle.pause(true);
    assert_eq!(handle.checkpoint(), Err(CheckpointError::Pending));
    assert!(run(&mut *live, 20, 48000).iter().all(|f| *f == Frame::ZERO));
    let (mut resumed, mut resumed_handle) =
        CheckpointData::resume(d.clone(), handle.checkpoint().unwrap())
            .unwrap()
            .into_sound()
            .unwrap();
    assert_eq!(handle.state(), PlaybackState::Paused);
    assert_eq!(resumed_handle.state(), PlaybackState::Paused);
    handle.pause(false);
    resumed_handle.pause(false);
    for block in [1, 91, 17] {
        assert_eq!(
            run(&mut *live, block, 48000),
            run(&mut *resumed, block, 48000)
        );
    }
    assert_eq!(handle.checkpoint(), resumed_handle.checkpoint());
    handle.stop();
    run(&mut *live, 33, 48000);
    let (mut stopped, stopped_handle) = CheckpointData::resume(d, handle.checkpoint().unwrap())
        .unwrap()
        .into_sound()
        .unwrap();
    assert_eq!(stopped_handle.state(), PlaybackState::Stopped);
    assert!(run(&mut *stopped, 99, 48000)
        .iter()
        .all(|f| *f == Frame::ZERO));
}
#[test]
fn malformed_and_mismatched_states_refuse_before_a_sound_exists() {
    let d = data(true, 0.0);
    let (mut sound, handle) = CheckpointData::new(d.clone())
        .unwrap()
        .into_sound()
        .unwrap();
    run(&mut *sound, 71, 48000);
    let original = *handle.checkpoint().unwrap().as_bytes();
    for end in [0, 8, 143, 145] {
        let mut b = original.to_vec();
        b.resize(end, 0);
        assert!(Checkpoint::from_bytes(&b, &d).is_err());
    }
    for (p, replacement) in [
        (0, 255),
        (12, 7),
        (13, 2),
        (14, 2),
        (15, 1),
        (60, 5),
        (80, 255),
    ] {
        let mut b = original;
        b[p] = replacement;
        assert!(Checkpoint::from_bytes(&b, &d).is_err(), "byte {p}");
    }
    for p in [32, 48] {
        let mut b = original;
        b[p..p + 8].copy_from_slice(&f64::NAN.to_le_bytes());
        assert!(Checkpoint::from_bytes(&b, &d).is_err());
    }
    let mut wrong = d.clone();
    wrong.sample_rate = 44100;
    assert!(Checkpoint::from_bytes(&original, &wrong).is_err());
    wrong = d.clone();
    wrong.frames = vec![Frame::from_mono(0.99); 97].into();
    assert!(Checkpoint::from_bytes(&original, &wrong).is_err());
    assert_eq!(*handle.checkpoint().unwrap().as_bytes(), original);
}
#[test]
fn current_instant_parameters_only_and_invalid_data_are_explicit() {
    for d in [
        data(false, 0.0).reverse(true),
        data(false, 0.0).fade_in_tween(Tween::default()),
        data(false, 0.0).loop_region(0.001..0.002),
        data(false, f64::NAN),
        data(false, 0.0).playback_rate(f64::INFINITY),
    ] {
        assert!(CheckpointData::new(d).is_err());
    }
    let mut empty = data(false, 0.0);
    empty.frames = Arc::new([]);
    assert!(CheckpointData::new(empty).is_err());
}

#[test]
fn retired_sound_keeps_last_state_and_invalid_loop_endpoints_refuse() {
    for start in [f64::NAN, -1.0, f64::INFINITY] {
        assert!(CheckpointData::new(data(false, 0.0).loop_region(start..)).is_err());
    }
    let (mut sound, mut handle) = CheckpointData::new(data(false, 0.0))
        .unwrap()
        .into_sound()
        .unwrap();
    run(&mut *sound, 400, 48000);
    let before = handle.checkpoint().unwrap();
    assert!(handle.retained());
    assert!(sound.finished());
    assert!(!handle.retained());
    handle.stop();
    handle.pause(false);
    handle.configure(-7.0, 0.2, 0.9).unwrap();
    assert_eq!(handle.checkpoint().unwrap(), before);
}
