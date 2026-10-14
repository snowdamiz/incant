use super::*;
use incant_assets_runtime::sha256;
use incant_input::KeyCode;
use incant_render_data::{SceneInput, UiInput};
use incant_render_native::UpdateKind;
use incant_runtime::{CookedEntity, CookedScene, CookedSpatial, StableId};
use incant_types::{Camera, CameraProjection, Transform, Velocity};
use serde_json::json;
use std::{
    collections::BTreeMap,
    fs,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
fn key(code: KeyCode, down: bool) -> InputEvent {
    InputEvent::Key { code, down }
}

fn package() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    let mut camera = CookedSpatial::empty(id(2));
    camera.camera = Some(Camera {
        fov_degrees: 60.,
        near: 0.1,
        far: 100.,
        projection: CameraProjection::Perspective {},
    });
    let scene = CookedScene::new(
        id(99),
        144,
        vec![
            CookedEntity {
                id: id(1),
                parent: None,
                transform: Transform::default(),
                velocity: Some(Velocity { linear: [0.; 3] }),
            },
            CookedEntity {
                id: id(2),
                parent: None,
                transform: Transform::default(),
                velocity: None,
            },
        ],
    )
    .unwrap()
    .with_spatial(vec![], vec![camera])
    .unwrap();
    let files = [
        ("world.scene", scene.to_bytes()),
        ("clips.bin", [b"NCLIPS01".as_slice(), &[0; 4]].concat()),
        (
            "input-actions.json",
            serde_json::to_vec(&json!({
                "right": {"kind":"button","bindings":[{"type":"key","code":"KeyD"}]},
                "fail": {"kind":"button","bindings":[{"type":"key","code":"KeyF"}]}
            }))
            .unwrap(),
        ),
        // Startup authenticity is checked, but this offline recording is never run.
        ("empty-input.json", b"not a recording".to_vec()),
    ];
    let mut hashes = BTreeMap::new();
    for (name, data) in files {
        hashes.insert(name, sha256(&data));
        fs::write(root.path().join(name), data).unwrap();
    }
    fs::write(root.path().join("package.json"), serde_json::to_vec(&json!({
        "format":"incant-native-package","version":1,"camera":format!("{:032x}",2),"files":hashes
    })).unwrap()).unwrap();
    root
}

#[derive(Default)]
struct Probe {
    prepares: AtomicUsize,
    frames: AtomicUsize,
    drops: AtomicUsize,
    fail: AtomicBool,
}
struct Consumer(Arc<Probe>);
#[derive(Debug, thiserror::Error)]
#[error("consumer refused frame")]
struct ConsumerError;
impl Drop for Consumer {
    fn drop(&mut self) {
        self.0.drops.fetch_add(1, Ordering::Relaxed);
    }
}
impl RenderInputConsumer for Consumer {
    const SUPPORTS_UI: bool = true;
    type Settings = Arc<Probe>;
    type Error = ConsumerError;
    fn prepare(
        _: &SceneInput,
        _: &str,
        _: Option<&Arc<UiInput>>,
        settings: Arc<Probe>,
    ) -> std::result::Result<Self, ConsumerError> {
        settings.prepares.fetch_add(1, Ordering::Relaxed);
        Ok(Self(settings))
    }
    fn accept(
        &mut self,
        _: &SceneInput,
        _: &str,
        _: UpdateKind,
        _: Option<&Arc<UiInput>>,
    ) -> std::result::Result<(), ConsumerError> {
        if self.0.fail.load(Ordering::Relaxed) {
            return Err(ConsumerError);
        }
        self.0.frames.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
    fn admit_ui(&mut self, _: &Arc<UiInput>) -> std::result::Result<(), ConsumerError> {
        Ok(())
    }
}
fn open(root: &Path, probe: Arc<Probe>) -> MobileSession<Consumer> {
    MobileSession::open(root, 44117, Some([640., 360.]), probe).unwrap()
}
fn tick(game: &mut MobileSession<Consumer>, events: &[InputEvent]) -> TickReceipt {
    let token = game.delivery_token().unwrap();
    let mut pcm = vec![99.; game.next_output_frames().unwrap() * 2];
    let receipt = game.tick(&token, events, &[], &mut pcm).unwrap();
    assert!(pcm.iter().all(|v| *v == 0.));
    receipt
}

#[test]
fn callbacks_retain_one_world_consumer_and_exact_fractional_pcm_clock() {
    let root = package();
    let probe = Arc::new(Probe::default());
    let mut game = open(root.path(), probe.clone());
    assert_eq!(probe.prepares.load(Ordering::Relaxed), 1);
    assert_eq!(probe.frames.load(Ordering::Relaxed), 0);
    assert!(matches!(
        game.delivery_token(),
        Err(MobileHostError::Suspended)
    ));
    game.set_active(true).unwrap();
    let identity = game.world().unwrap().incarnation();
    let mut frames = 0;
    for n in 1..=104 {
        let events = if n == 1 {
            vec![key(KeyCode::KeyD, true)]
        } else {
            vec![]
        };
        let result = tick(&mut game, &events);
        frames += result.stereo_frames;
        assert_eq!(result.simulation_tick, n);
        assert_eq!(game.world().unwrap().incarnation(), identity);
    }
    assert_eq!(frames, 104 * 44117 / 144);
    assert_eq!(probe.prepares.load(Ordering::Relaxed), 1);
    assert_eq!(probe.frames.load(Ordering::Relaxed), 104);
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 5);
    game.close();
    game.close();
    assert_eq!(probe.drops.load(Ordering::Relaxed), 1);
    assert!(matches!(game.world(), Err(MobileHostError::Failed)));
}

#[test]
fn suspend_resume_retires_deliveries_and_clears_held_input_without_hidden_ticks() {
    let root = package();
    let mut game = open(root.path(), Arc::default());
    game.set_active(true).unwrap();
    tick(&mut game, &[key(KeyCode::KeyD, true)]);
    let old = game.delivery_token().unwrap();
    let frames = game.next_output_frames().unwrap();
    let mut pcm = vec![17.; frames * 2];
    game.set_active(false).unwrap();
    game.set_active(false).unwrap();
    assert!(matches!(
        game.tick(&old, &[], &[], &mut pcm),
        Err(MobileHostError::Suspended)
    ));
    game.set_active(true).unwrap();
    assert!(matches!(
        game.tick(&old, &[], &[], &mut pcm),
        Err(MobileHostError::StaleDelivery)
    ));
    assert!(matches!(
        game.host(),
        Err(MobileHostError::PendingLifecycle)
    ));
    assert_eq!(game.world().unwrap().tick(), 1);
    assert_eq!(game.next_output_frames().unwrap(), frames);
    assert!(pcm.iter().all(|v| *v == 17.));
    let result = tick(&mut game, &[]);
    assert_eq!(result.simulation_tick, 2);
    assert!(
        game.game
            .as_ref()
            .unwrap()
            .input()
            .unwrap()
            .keyboard
            .held
            .is_empty()
    );
    assert!(game.host().is_ok());
}

#[test]
fn foreign_owner_resize_and_bad_buffer_callbacks_do_not_consume_output() {
    let root = package();
    let mut first = open(root.path(), Arc::default());
    let mut other = open(root.path(), Arc::default());
    first.set_active(true).unwrap();
    other.set_active(true).unwrap();
    tick(&mut first, &[]);
    let old = first.delivery_token().unwrap();
    first.resize_ui([320., 640.]).unwrap();
    assert_eq!(first.world().unwrap().tick(), 1);
    let mut pcm = vec![17.; first.next_output_frames().unwrap() * 2];
    for token in [old, other.delivery_token().unwrap()] {
        assert!(matches!(
            first.tick(&token, &[], &[], &mut pcm),
            Err(MobileHostError::StaleDelivery)
        ));
    }
    let current = first.delivery_token().unwrap();
    let n = pcm.len();
    assert!(matches!(
        first.tick(&current, &[], &[], &mut pcm[..n - 1]),
        Err(MobileHostError::Game(GameError::Buffer { .. }))
    ));
    assert!(pcm.iter().all(|v| *v == 17.));
    assert_eq!(first.world().unwrap().tick(), 1);
    assert!(matches!(
        first.resize_ui([f32::NAN, 1.]),
        Err(MobileHostError::Settings(_))
    ));
    first.tick(&current, &[], &[], &mut pcm).unwrap();
    assert_eq!(first.world().unwrap().tick(), 2);
}

#[test]
fn accepted_guest_and_mandatory_consumer_failures_silence_pcm_and_retire_game() {
    for fail_guest in [true, false] {
        let root = package();
        let probe = Arc::new(Probe::default());
        let mut game = open(root.path(), probe.clone());
        game.set_active(true).unwrap();
        tick(&mut game, &[]);
        probe.fail.store(!fail_guest, Ordering::Relaxed);
        let token = game.delivery_token().unwrap();
        let mut pcm = vec![17.; game.next_output_frames().unwrap() * 2];
        let events = if fail_guest {
            vec![key(KeyCode::KeyF, true)]
        } else {
            vec![]
        };
        assert!(game.tick(&token, &events, &[], &mut pcm).is_err());
        assert!(pcm.iter().all(|v| *v == 0.));
        assert!(matches!(game.world(), Err(MobileHostError::Failed)));
        assert_eq!(probe.drops.load(Ordering::Relaxed), 1);
        assert_eq!(probe.frames.load(Ordering::Relaxed), 1);
        assert!(matches!(
            game.tick(&token, &[], &[], &mut pcm),
            Err(MobileHostError::Failed)
        ));
    }
}

#[test]
fn startup_validation_and_epoch_exhaustion_fail_before_resource_reuse() {
    let root = package();
    let probe = Arc::new(Probe::default());
    assert!(matches!(
        MobileSession::<Consumer>::open(Path::new("missing"), 1, None, probe.clone()),
        Err(MobileHostError::Settings(_))
    ));
    let mut game = open(root.path(), probe.clone());
    game.set_active(true).unwrap();
    game.epoch = u64::MAX;
    assert!(matches!(
        game.set_active(false),
        Err(MobileHostError::EpochLimit)
    ));
    assert_eq!(probe.drops.load(Ordering::Relaxed), 1);
    assert!(matches!(game.world(), Err(MobileHostError::Failed)));
    fs::write(root.path().join("world.scene"), b"corrupt").unwrap();
    assert!(matches!(
        MobileSession::<Consumer>::open(root.path(), 48000, None, probe.clone()),
        Err(MobileHostError::Startup(_))
    ));
    assert_eq!(probe.prepares.load(Ordering::Relaxed), 1);
}

#[test]
fn resumed_ui_and_invalid_physical_packets_leave_neutralization_retryable() {
    let root = package();
    let mut game = open(root.path(), Arc::default());
    game.set_active(true).unwrap();
    tick(&mut game, &[key(KeyCode::KeyD, true)]);
    game.set_active(false).unwrap();
    game.set_active(true).unwrap();
    let token = game.delivery_token().unwrap();
    let frames = game.next_output_frames().unwrap();
    let mut pcm = vec![17.; frames * 2];
    assert!(matches!(
        game.tick(&token, &[], &[UiEvent::Activate], &mut pcm),
        Err(MobileHostError::PendingLifecycle)
    ));
    assert!(
        game.tick(
            &token,
            &[InputEvent::PointerPosition {
                position: [f64::NAN, 0.],
            }],
            &[],
            &mut pcm
        )
        .is_err()
    );
    assert_eq!(game.world().unwrap().tick(), 1);
    assert_eq!(game.next_output_frames().unwrap(), frames);
    assert!(pcm.iter().all(|v| *v == 17.));
    assert!(matches!(
        game.host(),
        Err(MobileHostError::PendingLifecycle)
    ));
    game.tick(&token, &[], &[], &mut pcm).unwrap();
    assert_eq!(game.world().unwrap().tick(), 2);
    assert!(
        game.game
            .as_ref()
            .unwrap()
            .input()
            .unwrap()
            .keyboard
            .held
            .is_empty()
    );
    assert!(game.host().is_ok());
}

#[test]
fn unavailable_ui_and_excess_input_refuse_without_revoking_current_delivery() {
    let root = package();
    let mut game =
        MobileSession::<Consumer>::open(root.path(), 48_000, None, Arc::default()).unwrap();
    game.set_active(true).unwrap();
    let token = game.delivery_token().unwrap();
    let mut pcm = vec![17.; game.next_output_frames().unwrap() * 2];
    assert!(game.resize_ui([640., 360.]).is_err());
    let events = vec![InputEvent::Focus { focused: true }; MAX_EVENTS_PER_TICK - 1];
    assert!(matches!(
        game.tick(&token, &events, &[], &mut pcm),
        Err(MobileHostError::InputLimit)
    ));
    assert_eq!(game.world().unwrap().tick(), 0);
    assert!(pcm.iter().all(|v| *v == 17.));
    game.tick(&token, &[], &[], &mut pcm).unwrap();
    assert_eq!(game.world().unwrap().tick(), 1);
}
