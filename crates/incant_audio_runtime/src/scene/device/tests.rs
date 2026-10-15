use super::*;
use std::collections::VecDeque;
#[path = "../../../../incant_assets_runtime/tests/support/mod.rs"]
mod support;

struct Queue {
    errors: VecDeque<Error>,
    discarded: Option<u64>,
    lose_after_pop: bool,
    refill: bool,
    pops: usize,
}
impl Queue {
    fn new(kinds: &[ErrorKind]) -> Self {
        Self {
            errors: kinds.iter().map(|&kind| kind.into()).collect(),
            discarded: Some(0),
            lose_after_pop: false,
            refill: false,
            pops: 0,
        }
    }
}
impl ErrorQueue for Queue {
    fn discarded(&self) -> Option<u64> {
        self.discarded
    }
    fn pop(&mut self) -> Option<Error> {
        self.pops += 1;
        if self.lose_after_pop {
            self.discarded = Some(1);
        }
        if self.refill {
            Some(ErrorKind::Xrun.into())
        } else {
            self.errors.pop_front()
        }
    }
}

#[test]
fn xruns_are_counted_and_empty_polls_keep_the_observation() {
    let mut diagnostics = DeviceAudioDiagnostics::default();
    let mut queue = Queue::new(&[ErrorKind::Xrun, ErrorKind::Xrun]);
    diagnostics.poll_queue(&mut queue).unwrap();
    assert_eq!(diagnostics.xrun_notifications, 2);
    diagnostics.poll_queue(&mut queue).unwrap();
    assert_eq!(diagnostics.xrun_notifications, 2);
    assert_eq!(diagnostics.poll_limit_hits, 0);
    assert!(!diagnostics.counters_saturated);
}

#[test]
fn every_other_pinned_kind_remains_fatal_even_after_xruns() {
    for kind in [
        ErrorKind::DeviceBusy,
        ErrorKind::DeviceChanged,
        ErrorKind::DeviceNotAvailable,
        ErrorKind::HostUnavailable,
        ErrorKind::InvalidInput,
        ErrorKind::PermissionDenied,
        ErrorKind::RealtimeDenied,
        ErrorKind::ResourceExhausted,
        ErrorKind::StreamInvalidated,
        ErrorKind::UnsupportedConfig,
        ErrorKind::UnsupportedOperation,
        ErrorKind::BackendError,
        ErrorKind::Other,
    ] {
        let mut diagnostics = DeviceAudioDiagnostics::default();
        let mut queue = Queue::new(&[ErrorKind::Xrun, kind, ErrorKind::Xrun]);
        assert!(diagnostics.poll_queue(&mut queue).is_err(), "{kind:?}");
        assert_eq!(diagnostics.xrun_notifications, 1);
        assert_eq!(queue.errors.len(), 1);
    }
}

#[test]
fn formatted_xrun_text_cannot_disguise_a_backend_failure() {
    let mut diagnostics = DeviceAudioDiagnostics::default();
    let mut queue = Queue::new(&[]);
    queue.errors.push_back(Error::with_message(
        ErrorKind::BackendError,
        ErrorKind::Xrun.to_string(),
    ));
    assert!(diagnostics.poll_queue(&mut queue).is_err());
    assert_eq!(diagnostics.xrun_notifications, 0);
}

#[test]
fn unavailable_or_lossy_queues_refuse_before_consuming() {
    for discarded in [None, Some(1), Some(u64::MAX)] {
        let mut queue = Queue::new(&[ErrorKind::Xrun]);
        queue.discarded = discarded;
        assert!(
            DeviceAudioDiagnostics::default()
                .poll_queue(&mut queue)
                .is_err()
        );
        assert_eq!(queue.pops, 0);
    }
}

#[test]
fn loss_during_an_xrun_batch_is_still_fatal() {
    let mut diagnostics = DeviceAudioDiagnostics::default();
    let mut queue = Queue::new(&[ErrorKind::Xrun]);
    queue.lose_after_pop = true;
    assert!(diagnostics.poll_queue(&mut queue).is_err());
    assert_eq!(diagnostics.xrun_notifications, 1);
}

#[test]
fn continuous_notifications_cannot_make_a_poll_unbounded() {
    let mut diagnostics = DeviceAudioDiagnostics::default();
    let mut queue = Queue::new(&[]);
    queue.refill = true;
    diagnostics.poll_queue(&mut queue).unwrap();
    assert_eq!(queue.pops, MAX_EVENTS_PER_POLL);
    assert_eq!(diagnostics.xrun_notifications, MAX_EVENTS_PER_POLL as u64);
    assert_eq!(diagnostics.poll_limit_hits, 1);
    queue.refill = false;
    queue.errors.push_back(ErrorKind::StreamInvalidated.into());
    assert!(diagnostics.poll_queue(&mut queue).is_err());
}

#[test]
fn lifetime_counters_saturate_visibly_instead_of_wrapping() {
    let mut diagnostics = DeviceAudioDiagnostics {
        xrun_notifications: u64::MAX,
        poll_limit_hits: u64::MAX,
        counters_saturated: false,
    };
    let mut queue = Queue::new(&[]);
    queue.refill = true;
    diagnostics.poll_queue(&mut queue).unwrap();
    assert_eq!(diagnostics.xrun_notifications, u64::MAX);
    assert_eq!(diagnostics.poll_limit_hits, u64::MAX);
    assert!(diagnostics.counters_saturated);
}

#[test]
fn host_continues_after_xrun_but_fatal_device_result_drops_the_mixer() {
    use crate::{OfflineBackend, SceneAudio};
    use incant_runtime::{CookedScene, NativeWorld, StableId};

    let scene = CookedScene::new(StableId([1; 16]), 60, vec![]).unwrap();
    let world = NativeWorld::from_bytes(&scene.to_bytes()).unwrap();
    let mut host = SceneAudio::<OfflineBackend>::offline(&world, vec![], 48_000).unwrap();
    let result = host
        .device_diagnostics
        .poll_queue(&mut Queue::new(&[ErrorKind::Xrun]));
    host.terminal(result).unwrap();
    host.render(&mut [0.; 16]).unwrap();
    assert_eq!(host.device_diagnostics.xrun_notifications, 1);

    let result = host
        .device_diagnostics
        .poll_queue(&mut Queue::new(&[ErrorKind::DeviceNotAvailable]));
    assert!(host.terminal(result).is_err());
    assert!(matches!(
        host.render(&mut [0.; 16]),
        Err(AudioError::FailedSession)
    ));
}

#[test]
fn xrun_notification_preserves_nonzero_voice_cursor_continuation() {
    use crate::{ClipBinding, OfflineBackend, SceneAudio};
    use incant_runtime::{
        CookedAudio, CookedEntity, CookedScene, NativeAudioSource, NativeWorld, StableId,
    };
    use incant_types::Transform;

    let id = StableId([2; 16]);
    let clip = StableId([3; 16]);
    let scene = CookedScene::new(
        StableId([1; 16]),
        60,
        vec![CookedEntity {
            id,
            parent: None,
            transform: Transform::default(),
            velocity: None,
        }],
    )
    .unwrap()
    .with_audio(vec![CookedAudio {
        entity: id,
        bus: None,
        listener: false,
        source: Some(NativeAudioSource {
            transport_epoch: 0,
            clip,
            bus: None,
            gain_db: 0.,
            pan: 0.,
            rate: 1.,
            start_seconds: 0.,
            playing: true,
            looping: true,
            streaming: false,
            spatial: None,
        }),
    }])
    .unwrap();
    let world = NativeWorld::from_bytes(&scene.to_bytes()).unwrap();
    let temp = tempfile::tempdir().unwrap();
    let frames: Vec<_> = (0..2000)
        .map(|i| [((i as f32) * 0.035).sin() * 0.25; 2])
        .collect();
    let key = support::cooked(temp.path(), &frames);
    let make = || {
        SceneAudio::<OfflineBackend>::offline(
            &world,
            vec![ClipBinding::load(clip, temp.path(), &key).unwrap()],
            48_000,
        )
        .unwrap()
    };
    let mut baseline = make();
    let mut observed = make();
    let mut expected = [0.; 4096];
    baseline.render(&mut expected).unwrap();
    let mut actual = [0.; 4096];
    observed.render(&mut actual[..1024]).unwrap();
    let result = observed
        .device_diagnostics
        .poll_queue(&mut Queue::new(&[ErrorKind::Xrun]));
    observed.terminal(result).unwrap();
    observed.render(&mut actual[1024..]).unwrap();
    assert!(expected.iter().any(|v| v.abs() > 0.1));
    assert_eq!(actual, expected);
    assert_eq!(observed.device_diagnostics.xrun_notifications, 1);
}
