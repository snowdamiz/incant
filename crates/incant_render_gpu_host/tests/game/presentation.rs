//! Actual GPU admission and composed gameplay/audio parity. Not window pacing.
use super::*;
use incant_render_game::RenderInputConsumer;
use incant_render_gpu_host::PresentationMode;
use incant_render_native::UpdateKind;
use std::sync::atomic::AtomicBool;

struct BatchTargets<P> {
    inner: P,
    defer: Arc<AtomicBool>,
    acquisitions: Arc<AtomicUsize>,
}
impl<P: TargetProvider> TargetProvider for BatchTargets<P> {
    type Error = P::Error;
    type Frame = P::Frame;
    fn acquire(&mut self, renderer: &Renderer) -> Result<Self::Frame, Self::Error> {
        assert!(
            !self.defer.load(Ordering::SeqCst),
            "intermediate acquisition"
        );
        self.acquisitions.fetch_add(1, Ordering::SeqCst);
        self.inner.acquire(renderer)
    }
    fn presentation_mode(&self) -> PresentationMode {
        if self.defer.load(Ordering::SeqCst) {
            PresentationMode::AdmitOnly
        } else {
            PresentationMode::Present
        }
    }
}

#[test]
#[ignore = "requires coordinated native GPU; no performance claim"]
fn batches_preserve_every_tick_input_and_pcm_but_only_display_final_state() {
    let (mut ordinary, reference) = game_with(true);
    let defer = Arc::new(AtomicBool::new(false));
    let acquisitions = Arc::new(AtomicUsize::new(0));
    let (mut batched, probe) = game_with_targets(true, |s| GpuRenderSettings {
        renderer: s.renderer,
        targets: BatchTargets {
            inner: s.targets,
            defer: Arc::clone(&defer),
            acquisitions: Arc::clone(&acquisitions),
        },
    });
    let mut tick = 0;
    let mut displays = 0;
    let mut last_image = None;
    for due in [0, 1, 2, 0, 4, 1] {
        for ordinal in 0..due {
            tick += 1;
            let events = if tick == 2 || tick == 3 {
                vec![InputEvent::Key {
                    code: KeyCode::Space,
                    down: tick == 2,
                }]
            } else {
                vec![]
            };
            let intermediate = ordinal + 1 < due;
            defer.store(intermediate, Ordering::SeqCst);
            let mut a = vec![0.; ordinary.next_output_frames().unwrap() * 2];
            let mut b = vec![0.; batched.next_output_frames().unwrap() * 2];
            ordinary.tick(&events, &mut a).unwrap();
            batched.tick(&events, &mut b).unwrap();
            assert_eq!(a, b, "complete PCM at tick {tick}");
            assert!(b.iter().any(|v| v.abs() > 0.01));
            assert_eq!(
                ordinary.world().unwrap().snapshot(),
                batched.world().unwrap().snapshot()
            );
            assert_eq!(ordinary.input().unwrap(), batched.input().unwrap());
            assert_eq!(batched.host().unwrap().render_input().tick(), tick);
            if intermediate {
                assert_eq!(
                    batched.host().unwrap().consumer().accepted_frames(),
                    displays
                );
                assert_eq!(acquisitions.load(Ordering::SeqCst), displays as usize);
                if let Some(image) = &last_image {
                    assert_eq!(&pixels(&probe), image);
                }
            } else {
                displays += 1;
                let image = pixels(&probe);
                assert_eq!(image, pixels(&reference), "final state at tick {tick}");
                last_image = Some(image);
            }
        }
        assert_eq!(batched.world().unwrap().tick(), tick);
        assert_eq!(
            batched.host().unwrap().consumer().accepted_frames(),
            displays
        );
        assert_eq!(probe.presents.load(Ordering::SeqCst), displays as usize);
    }
    assert_eq!((tick, displays), (8, 4));
    assert_eq!(
        batched.rendered_frames().unwrap(),
        ordinary.rendered_frames().unwrap()
    );
}

#[test]
#[ignore = "requires coordinated native GPU; no performance claim"]
fn early_batch_boundary_refreshes_without_extra_gameplay_or_audio_and_failure_is_terminal() {
    let defer = Arc::new(AtomicBool::new(true));
    let acquisitions = Arc::new(AtomicUsize::new(0));
    let (mut game, probe) = game_with_targets(false, |s| GpuRenderSettings {
        renderer: s.renderer,
        targets: BatchTargets {
            inner: s.targets,
            defer: Arc::clone(&defer),
            acquisitions: Arc::clone(&acquisitions),
        },
    });
    let mut pcm = vec![0.; game.next_output_frames().unwrap() * 2];
    game.tick(&[], &mut pcm).unwrap();
    assert!(pcm.iter().any(|v| v.abs() > 0.01));
    assert_eq!(acquisitions.load(Ordering::SeqCst), 0);
    let world = game.world().unwrap().snapshot();
    let input = game.input().unwrap().clone();
    let audio_frames = game.rendered_frames().unwrap();
    defer.store(false, Ordering::SeqCst);
    game.refresh_host().unwrap();
    assert_eq!(probe.presents.load(Ordering::SeqCst), 1);
    assert_eq!(game.world().unwrap().snapshot(), world);
    assert_eq!(game.input().unwrap(), &input);
    assert_eq!(game.rendered_frames().unwrap(), audio_frames);
    defer.store(true, Ordering::SeqCst);
    game.tick(&[], &mut pcm).unwrap();
    probe.fault.store(1, Ordering::SeqCst);
    defer.store(false, Ordering::SeqCst);
    assert!(game.refresh_host().is_err());
    assert!(game.is_failed());
    assert_eq!(probe.dropped.load(Ordering::SeqCst), 1);
    pcm.fill(0.7);
    assert!(game.tick(&[], &mut pcm).is_err());
    assert!(pcm.iter().all(|v| *v == 0.));
}

fn targets(probe: &Probe, defer: &Arc<AtomicBool>) -> BatchTargets<Targets> {
    BatchTargets {
        inner: Targets {
            texture: probe.texture.clone(),
            presents: Arc::clone(&probe.presents),
            fault: Arc::clone(&probe.fault),
            dropped: Arc::clone(&probe.dropped),
        },
        defer: Arc::clone(defer),
        acquisitions: Arc::new(AtomicUsize::new(0)),
    }
}

#[test]
#[ignore = "requires coordinated native GPU; no performance claim"]
fn intermediate_rebuild_survives_final_motion_and_invalid_intermediate_is_not_skipped() {
    let (game, probe) = game();
    let original = game.host().unwrap().render_input();
    let input = original.input();
    let camera = original.camera_label();
    let defer = Arc::new(AtomicBool::new(true));
    let mut consumer = GpuRenderConsumer::prepare(
        input,
        camera,
        None,
        GpuRenderSettings {
            renderer: Arc::clone(&probe.renderer),
            targets: targets(&probe, &defer),
        },
    )
    .unwrap();
    let mut rebuilt = incant_render_data::SceneInput::default();
    for node in &input.nodes {
        rebuilt.nodes.push(incant_render_data::SceneNode {
            id: node.id.clone(),
            world: node.world,
            camera: node.camera.clone(),
            light: node.light.clone(),
            mesh: None,
            sprite: None,
            tilemap: None,
            diagnostic: false,
        });
    }
    // Removing the mesh changes bindings; final camera motion must use this
    // intermediate replacement, not the original model bindings.
    consumer
        .accept(&rebuilt, camera, UpdateKind::Rebuilt, None)
        .unwrap();
    assert_eq!(consumer.accepted_frames(), 0);
    rebuilt
        .nodes
        .iter_mut()
        .find(|n| n.camera.is_some())
        .unwrap()
        .world
        .w_axis
        .x += 0.25;
    consumer
        .accept(&rebuilt, camera, UpdateKind::Motion, None)
        .unwrap();
    assert_eq!(probe.presents.load(Ordering::SeqCst), 0);
    defer.store(false, Ordering::SeqCst);
    consumer
        .accept(&rebuilt, camera, UpdateKind::Motion, None)
        .unwrap();
    let expected = pixels(&probe);
    let mut fresh = GpuRenderConsumer::prepare(
        &rebuilt,
        camera,
        None,
        GpuRenderSettings {
            renderer: Arc::clone(&probe.renderer),
            targets: targets(&probe, &defer),
        },
    )
    .unwrap();
    fresh
        .accept(&rebuilt, camera, UpdateKind::Motion, None)
        .unwrap();
    assert_eq!(pixels(&probe), expected);
    assert_eq!(consumer.accepted_frames(), 1);
    defer.store(true, Ordering::SeqCst);
    rebuilt.nodes[0].world.w_axis.x = f64::NAN;
    assert!(
        consumer
            .accept(&rebuilt, camera, UpdateKind::Motion, None)
            .is_err()
    );
    assert!(consumer.is_failed());
    rebuilt.nodes[0].world.w_axis.x = 0.;
    defer.store(false, Ordering::SeqCst);
    assert!(
        consumer
            .accept(&rebuilt, camera, UpdateKind::Motion, None)
            .is_err()
    );
    assert_eq!(consumer.accepted_frames(), 1);
}

#[test]
#[ignore = "requires coordinated native GPU; no performance claim"]
fn unqualified_resolution_policy_refuses_before_target_acquisition() {
    use incant_render::resolution::{ResolutionConfig, ResolutionScale};
    let (game, probe) = game();
    let original = game.host().unwrap().render_input();
    let defer = Arc::new(AtomicBool::new(true));
    let mut consumer = GpuRenderConsumer::prepare(
        original.input(),
        original.camera_label(),
        None,
        GpuRenderSettings {
            renderer: Arc::clone(&probe.renderer),
            targets: targets(&probe, &defer),
        }
        .with_dynamic_resolution(ResolutionConfig {
            target_ms: 16.0,
            deadband: 0.1,
            minimum: ResolutionScale::FULL,
            maximum: ResolutionScale::FULL,
            initial: ResolutionScale::FULL,
            overload_samples: 2,
            recovery_samples: 2,
            cooldown_samples: 2,
            slew: 1,
        })
        .unwrap(),
    )
    .unwrap();
    assert!(matches!(
        consumer.accept(
            original.input(),
            original.camera_label(),
            UpdateKind::Motion,
            None
        ),
        Err(incant_render_gpu_host::GpuHostError::BatchPolicy)
    ));
    assert!(consumer.is_failed());
    assert_eq!(probe.presents.load(Ordering::SeqCst), 0);
    assert_eq!(consumer.accepted_frames(), 0);
}
