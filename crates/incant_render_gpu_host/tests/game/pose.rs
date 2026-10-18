//! Actual composed static pose rendering; no window or performance claim.
use super::*;
use incant_render_data::{MeshInput, SceneInput, SceneNode};
use incant_render_game::{
    RenderInputConsumer,
    pose::{PoseConsumer, PoseRenderHost, PoseRequest},
};
use incant_render_gpu_host::PoseTargets;
use incant_render_native::{UpdateKind, interpolation::PoseReceipt};
use std::panic::{AssertUnwindSafe, catch_unwind};

type PoseGame = HostedOfflineGameSession<PoseRenderHost<GpuRenderConsumer<PoseTargets<Targets>>>>;
fn fixture() -> (PoseGame, Probe) {
    game_with_host::<_, PoseRenderHost<_>>(false, |s| GpuRenderSettings {
        renderer: s.renderer,
        targets: PoseTargets::new(s.targets),
    })
}
fn targets(probe: &Probe) -> Targets {
    Targets {
        texture: probe.texture.clone(),
        presents: Arc::clone(&probe.presents),
        fault: Arc::clone(&probe.fault),
        dropped: Arc::clone(&probe.dropped),
    }
}
fn expected(input: &SceneInput, x: f64) -> SceneInput {
    assert!(input.text.is_none() && input.environment.is_none());
    let nodes = input
        .nodes
        .iter()
        .map(|n| {
            assert!(n.sprite.is_none() && n.tilemap.is_none());
            let mut world = n.world;
            if n.mesh.is_some() {
                world.w_axis.x = x;
            }
            SceneNode {
                id: n.id.clone(),
                world,
                camera: n.camera.clone(),
                light: n.light.clone(),
                mesh: n.mesh.as_ref().map(|m| {
                    assert!(
                        m.animation.is_none()
                            && m.animation_blend.is_none()
                            && m.animation_pose.is_none()
                            && m.animation_ik.is_none()
                            && m.animation_root.is_none()
                    );
                    MeshInput {
                        source: Arc::clone(&m.source),
                        cast_shadows: m.cast_shadows,
                        animation: None,
                        animation_blend: None,
                        animation_pose: None,
                        animation_ik: None,
                        animation_root: None,
                    }
                }),
                sprite: None,
                tilemap: None,
                diagnostic: n.diagnostic,
            }
        })
        .collect();
    SceneInput {
        nodes,
        ..Default::default()
    }
}

#[test]
#[ignore = "requires coordinated native GPU; no performance claim"]
fn pose_endpoints_and_midpoint_match_independent_fresh_render_after_all_tick_admissions() {
    let (mut game, probe) = fixture();
    for tick in 1..=2 {
        let mut pcm = vec![0.0_f32; game.next_output_frames().unwrap() * 2];
        let events = if tick == 2 {
            vec![InputEvent::Key {
                code: KeyCode::Space,
                down: true,
            }]
        } else {
            vec![]
        };
        game.tick(&events, &mut pcm).unwrap();
        assert!(pcm.iter().any(|v| v.abs() > 0.01));
        assert_eq!(probe.presents.load(Ordering::SeqCst), 0);
        assert_eq!(game.host().unwrap().consumer().accepted_frames(), 0);
    }
    let frames = game.rendered_frames().unwrap();
    let state = game.world().unwrap().snapshot();
    let input = game.input().unwrap().clone();
    let canonical = game
        .host()
        .unwrap()
        .committed()
        .input()
        .nodes
        .iter()
        .find(|n| n.mesh.is_some())
        .unwrap()
        .world;
    let previous_x = -2. + 1. / 64.;
    let current_x = -2. + 2. / 64. + 0.5;
    assert_eq!(canonical.w_axis.x, current_x);
    let mut images = Vec::new();
    for fraction in [0., 0.5, 1.] {
        game.present_host(PoseRequest::Display { fraction })
            .unwrap();
        let actual = pixels(&probe);
        let committed = game.host().unwrap().committed();
        let reference = expected(
            committed.input(),
            previous_x + fraction * (current_x - previous_x),
        );
        let mut fresh = GpuRenderConsumer::prepare(
            &reference,
            committed.camera_label(),
            None,
            GpuRenderSettings {
                renderer: Arc::clone(&probe.renderer),
                targets: targets(&probe),
            },
        )
        .unwrap();
        fresh
            .accept(
                &reference,
                committed.camera_label(),
                UpdateKind::Motion,
                None,
            )
            .unwrap();
        assert_eq!(
            actual,
            pixels(&probe),
            "independent pose at fraction {fraction}"
        );
        images.push(actual);
        assert_eq!(
            committed
                .input()
                .nodes
                .iter()
                .find(|n| n.mesh.is_some())
                .unwrap()
                .world,
            canonical
        );
        assert_eq!(game.rendered_frames().unwrap(), frames);
        assert_eq!(game.world().unwrap().snapshot(), state);
        assert_eq!(game.input().unwrap(), &input);
    }
    assert_ne!(images[0], images[1]);
    assert_ne!(images[1], images[2]);
    assert_eq!(probe.presents.load(Ordering::SeqCst), 6);
    assert_eq!(game.host().unwrap().consumer().accepted_frames(), 3);
}

#[test]
#[ignore = "requires coordinated native GPU; no performance claim"]
fn pose_target_failure_and_unwind_destroy_composed_owner() {
    for fault in [1, 2, 3] {
        let (mut game, probe) = fixture();
        let mut pcm = vec![0.; game.next_output_frames().unwrap() * 2];
        game.tick(&[], &mut pcm).unwrap();
        probe.fault.store(fault, Ordering::SeqCst);
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            game.present_host(PoseRequest::Display { fraction: 0.5 })
        }));
        if fault == 3 {
            assert!(outcome.is_err());
        } else {
            assert!(outcome.unwrap().is_err());
        }
        assert!(game.is_failed());
        assert!(game.world().is_err() && game.host().is_err());
        assert_eq!(probe.presents.load(Ordering::SeqCst), 0);
        assert_eq!(probe.dropped.load(Ordering::SeqCst), 1);
    }
}

struct PolicyTargets {
    _inner: Targets,
    motion: bool,
}
impl TargetProvider for PolicyTargets {
    type Error = TargetError;
    type Frame = Frame;
    fn acquire(&mut self, _: &Renderer) -> Result<Frame, TargetError> {
        panic!("unsupported pose history policy reached target acquisition")
    }
    fn motion_epoch(&self) -> Option<u64> {
        self.motion.then_some(1)
    }
    fn resolution_config(&self) -> Option<incant_render::resolution::ResolutionConfig> {
        use incant_render::resolution::{ResolutionConfig, ResolutionScale};
        (!self.motion).then_some(ResolutionConfig {
            target_ms: 16.,
            deadband: 0.1,
            minimum: ResolutionScale::FULL,
            maximum: ResolutionScale::FULL,
            initial: ResolutionScale::FULL,
            overload_samples: 2,
            recovery_samples: 2,
            cooldown_samples: 2,
            slew: 1,
        })
    }
}
#[test]
#[ignore = "requires coordinated native GPU; no performance claim"]
fn pose_startup_display_refuses_unqualified_motion_and_resolution_policies() {
    let (game, probe) = fixture();
    let input = game.host().unwrap().committed();
    let mut renderer = models::checked(Renderer::headless().unwrap());
    pollster::block_on(renderer.enable_motion_vectors()).unwrap();
    let renderer = Arc::new(renderer);
    for motion in [false, true] {
        let mut consumer = GpuRenderConsumer::prepare(
            input.input(),
            input.camera_label(),
            None,
            GpuRenderSettings {
                renderer: Arc::clone(&renderer),
                targets: PoseTargets::new(PolicyTargets {
                    _inner: targets(&probe),
                    motion,
                }),
            },
        )
        .unwrap();
        let result = consumer.present_pose(
            input.input(),
            input.camera_label(),
            PoseReceipt {
                previous_tick: 0,
                current_tick: 0,
                continuity_epoch: 0,
                fraction: 0.5,
            },
            None,
        );
        assert!(matches!(
            result,
            Err(incant_render_gpu_host::GpuHostError::BatchPolicy)
        ));
        assert!(consumer.is_failed());
        assert_eq!(probe.presents.load(Ordering::SeqCst), 0);
    }
}
