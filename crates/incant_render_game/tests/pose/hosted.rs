use super::*;
use incant_render_data::{SceneInput, UiInput};
use incant_render_game::{
    RenderInputConsumer,
    pose::{PoseConsumer, PoseRenderHost, PoseReply, PoseRequest},
};
use incant_render_native::{UpdateKind, interpolation::PoseReceipt};
use std::{
    cell::RefCell,
    panic::{AssertUnwindSafe, catch_unwind},
    rc::Rc,
    sync::Arc,
};

#[derive(Default)]
struct Record {
    ticks: Vec<u64>,
    displayed: Vec<PoseReceipt>,
    mesh_positions: Vec<f64>,
    fail: bool,
    panic: bool,
    dropped: bool,
}
struct Consumer(Rc<RefCell<Record>>);
impl Drop for Consumer {
    fn drop(&mut self) {
        self.0.borrow_mut().dropped = true;
    }
}
impl RenderInputConsumer for Consumer {
    type Settings = Rc<RefCell<Record>>;
    type Error = std::io::Error;
    fn prepare(
        _: &SceneInput,
        _: &str,
        ui: Option<&Arc<UiInput>>,
        settings: Self::Settings,
    ) -> Result<Self, Self::Error> {
        assert!(ui.is_none());
        Ok(Self(settings))
    }
    fn accept(
        &mut self,
        _: &SceneInput,
        _: &str,
        _: UpdateKind,
        _: Option<&Arc<UiInput>>,
    ) -> Result<(), Self::Error> {
        panic!("pose host must use explicit admission, not ordinary presentation");
    }
    fn admit_ui(&mut self, _: &Arc<UiInput>) -> Result<(), Self::Error> {
        Err(std::io::Error::other("unsupported UI"))
    }
}
impl PoseConsumer for Consumer {
    fn admit_pose(
        &mut self,
        _: &SceneInput,
        _: &str,
        _: UpdateKind,
        tick: u64,
    ) -> Result<(), Self::Error> {
        self.0.borrow_mut().ticks.push(tick);
        Ok(())
    }
    fn present_pose(
        &mut self,
        input: &SceneInput,
        _: &str,
        receipt: PoseReceipt,
    ) -> Result<(), Self::Error> {
        let record = self.0.borrow();
        assert!(!record.panic, "injected presentation unwind");
        if record.fail {
            return Err(std::io::Error::other("injected presentation refusal"));
        }
        drop(record);
        let mut record = self.0.borrow_mut();
        record.displayed.push(receipt);
        record.mesh_positions.push(
            input
                .nodes
                .iter()
                .find(|n| n.mesh.is_some())
                .unwrap()
                .world
                .w_axis
                .x,
        );
        Ok(())
    }
}
type PoseGame = HostedOfflineGameSession<PoseRenderHost<Consumer>>;

fn settings(
    f: &Fixture,
    consumer: Rc<RefCell<Record>>,
) -> NativeRenderSettings<Rc<RefCell<Record>>> {
    let base = f.settings();
    NativeRenderSettings {
        model_cache: base.model_cache,
        texture_cache: base.texture_cache,
        camera: base.camera,
        text: base.text,
        ui: base.ui,
        consumer,
    }
}

#[test]
fn direct_presentation_refuses_foreign_world_and_unadmitted_same_tick_motion() {
    use incant_game_runtime::{SceneHost, SceneHostPresentation};
    let fixture = Fixture::new(false);
    let mut world = incant_runtime::NativeWorld::new(fixture.scene.clone());
    let other = incant_runtime::NativeWorld::new(fixture.scene.clone());
    let record = Rc::new(RefCell::new(Record::default()));
    let settings = settings(&fixture, record.clone());
    let mut host = PoseRenderHost::<Consumer>::prepare(&world, settings).unwrap();
    assert!(
        host.present(&other, PoseRequest::Display { fraction: 0.5 })
            .is_err()
    );
    world.with_numeric_columns(|columns| columns.for_each_chunk(|chunk| chunk.transforms[0] += 1.));
    assert!(
        host.present(&world, PoseRequest::Display { fraction: 0.5 })
            .is_err()
    );
    assert!(record.borrow().displayed.is_empty());
    host.present(
        &world,
        PoseRequest::Reset {
            continuity_epoch: 1,
        },
    )
    .unwrap();
    host.present(&world, PoseRequest::Display { fraction: 0.5 })
        .unwrap();
    assert_eq!(record.borrow().displayed.len(), 1);
}
fn game(script: CompiledScript) -> (PoseGame, Rc<RefCell<Record>>, tempfile::TempDir) {
    let f = Fixture::new(false);
    let r = Rc::new(RefCell::new(Record::default()));
    let settings = settings(&f, r.clone());
    let g = PoseGame::new(f.scene, script, actions(), vec![f.clip], 48000, settings).unwrap();
    (g, r, f.temp)
}

#[test]
fn reset_after_teleport_admission_snaps_current_pose_then_resumes_contiguous_history() {
    let (mut game, record, _temp) = game(code(
        "for(const c of chunks)for(let i=0;i<c.ids.length;i++)c.transforms[i*10]+=100;",
    ));
    let mut pcm = vec![0.; game.next_output_frames().unwrap() * 2];
    game.tick(&[], &mut pcm).unwrap();
    let position = game
        .host()
        .unwrap()
        .committed()
        .input()
        .nodes
        .iter()
        .find(|n| n.mesh.is_some())
        .unwrap()
        .world
        .w_axis
        .x;
    game.present_host(PoseRequest::Reset {
        continuity_epoch: 1,
    })
    .unwrap();
    for fraction in [0., 0.5, 1.] {
        game.present_host(PoseRequest::Display { fraction })
            .unwrap();
    }
    assert_eq!(record.borrow().mesh_positions, [position; 3]);
    assert!(
        record
            .borrow()
            .displayed
            .iter()
            .all(|r| r.previous_tick == 1 && r.current_tick == 1)
    );
    let mut pcm = vec![0.; game.next_output_frames().unwrap() * 2];
    game.tick(&[], &mut pcm).unwrap();
    game.present_host(PoseRequest::Display { fraction: 0. })
        .unwrap();
    assert_eq!(record.borrow().mesh_positions.last(), Some(&position));
    assert_eq!(record.borrow().displayed.last().unwrap().previous_tick, 1);
    assert_eq!(record.borrow().displayed.last().unwrap().current_tick, 2);
}

#[test]
fn direct_host_refuses_ui_ownership_before_prepare_admit_or_display() {
    use incant_game_runtime::{SceneHost, SceneHostPresentation};
    use incant_runtime::{CookedUi, FrameCommands, NativeWorld, StructuralCommand};
    use incant_types::{UiKind, UiNode};
    let fixture = Fixture::new(false);
    let record = Rc::new(RefCell::new(Record::default()));
    let node = Arc::new(UiNode {
        kind: UiKind::Root,
        style: Default::default(),
        order: 0,
        disabled: false,
    });
    let ui_world = NativeWorld::new(
        fixture
            .scene
            .clone()
            .with_ui(vec![CookedUi {
                entity: id(1),
                node: node.clone(),
            }])
            .unwrap(),
    );
    assert!(
        PoseRenderHost::<Consumer>::prepare(&ui_world, settings(&fixture, record.clone())).is_err()
    );
    let mut world = NativeWorld::new(fixture.scene.clone());
    let mut host =
        PoseRenderHost::<Consumer>::prepare(&world, settings(&fixture, record.clone())).unwrap();
    let mut commands = FrameCommands::default();
    commands
        .push(StructuralCommand::SetUi {
            entity: id(1),
            node: Some(node),
        })
        .unwrap();
    world.apply(&mut commands).unwrap();
    assert!(host.sync(&world).is_err());
    assert!(
        host.present(&world, PoseRequest::Display { fraction: 1. })
            .is_err()
    );
    assert!(
        host.present(
            &world,
            PoseRequest::Reset {
                continuity_epoch: 1
            }
        )
        .is_err()
    );
    commands
        .push(StructuralCommand::SetUi {
            entity: id(1),
            node: None,
        })
        .unwrap();
    world.apply(&mut commands).unwrap();
    assert!(world.requires_ui_checkpoint());
    assert!(host.sync(&world).is_err());
    assert!(record.borrow().ticks.is_empty());
    assert!(record.borrow().displayed.is_empty());
}
fn script() -> CompiledScript {
    code(
        "for(const c of chunks)for(let i=0;i<c.ids.length;i++)if(c.ids[i]==='00000000000000000000000001'&&input.actions.jump.pressed)c.transforms[i*10]+=0.25;const targets=commands.hapticCapabilities();if(targets.length)commands.playHaptic(targets[0],{kind:'rumble',strong:0.5,weak:1,durationMs:125});",
    )
}

#[test]
fn presentation_between_fixed_steps_preserves_complete_gameplay_input_audio_and_haptics() {
    let (mut baseline, base_temp) = Fixture::new(false).game(script());
    let (mut game, record, temp) = game(script());
    base_temp.close().unwrap();
    temp.close().unwrap();
    let caps = incant_input::haptics::HapticCapabilities::default()
        .with_gamepad(0, 3)
        .unwrap();
    baseline.set_haptic_capabilities(caps).unwrap();
    game.set_haptic_capabilities(caps).unwrap();
    let mut total = 0;
    let mut nonzero = false;
    for count in [0, 1, 2, 0, 4, 1] {
        for _ in 0..count {
            total += 1;
            let events = [key(total % 2 == 0)];
            let mut a = vec![0.; baseline.next_output_frames().unwrap() * 2];
            let mut b = vec![0.; game.next_output_frames().unwrap() * 2];
            baseline.tick(&events, &mut a).unwrap();
            game.tick(&events, &mut b).unwrap();
            assert_eq!(a, b);
            nonzero |= a.iter().any(|x| x.abs() > 0.001);
            let bytes = game.checkpoint_bytes().unwrap();
            let frames = game.rendered_frames().unwrap();
            for fraction in [0., 0.3, 0.7, 1.] {
                let reply = game
                    .present_host(PoseRequest::Display { fraction })
                    .unwrap();
                assert!(
                    matches!(reply,PoseReply::Displayed(r) if r.current_tick==total && r.previous_tick==total-1)
                );
                assert_eq!(game.checkpoint_bytes().unwrap(), bytes);
                assert_eq!(game.rendered_frames().unwrap(), frames);
            }
            let h = game.take_haptics().unwrap();
            assert_eq!(h.len(), 1);
            assert_eq!(h, baseline.take_haptics().unwrap());
            assert_eq!(
                game.checkpoint_bytes().unwrap(),
                baseline.checkpoint_bytes().unwrap()
            );
        }
        let before = game.checkpoint_bytes().unwrap();
        game.present_host(PoseRequest::Display { fraction: 0.6 })
            .unwrap();
        assert_eq!(game.checkpoint_bytes().unwrap(), before);
    }
    assert!(nonzero);
    assert_eq!(record.borrow().ticks, (1..=8).collect::<Vec<_>>());
    game.present_host(PoseRequest::Reset {
        continuity_epoch: 1,
    })
    .unwrap();
    let reply = game
        .present_host(PoseRequest::Display { fraction: 0.25 })
        .unwrap();
    assert!(
        matches!(reply,PoseReply::Displayed(r) if r.previous_tick==8 && r.current_tick==8 && r.continuity_epoch==1)
    );
    assert_eq!(
        game.checkpoint_bytes().unwrap(),
        baseline.checkpoint_bytes().unwrap()
    );
}

#[test]
fn presentation_refusal_and_unwind_destroy_composite_without_reaccepting_tick() {
    for panic in [false, true] {
        let (mut game, record, _temp) = game(code(""));
        let mut pcm = vec![0.; game.next_output_frames().unwrap() * 2];
        game.tick(&[], &mut pcm).unwrap();
        {
            let mut r = record.borrow_mut();
            r.fail = !panic;
            r.panic = panic;
        }
        let result = catch_unwind(AssertUnwindSafe(|| {
            game.present_host(PoseRequest::Display { fraction: 0.5 })
        }));
        if panic {
            assert!(result.is_err());
        } else {
            assert!(result.unwrap().is_err());
        }
        assert!(game.is_failed());
        assert!(game.world().is_err());
        assert!(game.host().is_err());
        assert_eq!(record.borrow().ticks, [1]);
        assert!(record.borrow().dropped);
    }
}
