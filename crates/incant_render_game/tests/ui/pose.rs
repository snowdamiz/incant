use super::*;
use incant_render_data::{SceneInput, UiInput};
use incant_render_game::pose::{PoseConsumer, PoseRenderHost, PoseRequest};
use incant_render_native::interpolation::PoseReceipt;
use std::{cell::RefCell, rc::Rc};
#[derive(Default)]
struct Record {
    displayed: Vec<Arc<UiInput>>,
    admitted: Option<Arc<UiInput>>,
    fail: bool,
}
struct Consumer(Rc<RefCell<Record>>);
impl RenderInputConsumer for Consumer {
    const SUPPORTS_UI: bool = true;
    type Settings = Rc<RefCell<Record>>;
    type Error = std::io::Error;
    fn prepare(
        _: &SceneInput,
        _: &str,
        ui: Option<&Arc<UiInput>>,
        settings: Self::Settings,
    ) -> Result<Self, Self::Error> {
        settings.borrow_mut().admitted = Some(ui.expect("real HUD").clone());
        Ok(Self(settings))
    }
    fn accept(
        &mut self,
        _: &SceneInput,
        _: &str,
        _: UpdateKind,
        _: Option<&Arc<UiInput>>,
    ) -> Result<(), Self::Error> {
        panic!("explicit pose admission required")
    }
    fn admit_ui(&mut self, ui: &Arc<UiInput>) -> Result<(), Self::Error> {
        self.0.borrow_mut().admitted = Some(ui.clone());
        Ok(())
    }
}
impl PoseConsumer for Consumer {
    fn admit_pose(
        &mut self,
        _: &SceneInput,
        _: &str,
        _: UpdateKind,
        _: u64,
        ui: Option<&Arc<UiInput>>,
    ) -> Result<(), Self::Error> {
        let mut record = self.0.borrow_mut();
        if record.fail {
            return Err(std::io::Error::other("candidate refused"));
        }
        record.admitted = Some(ui.expect("real HUD").clone());
        Ok(())
    }
    fn present_pose(
        &mut self,
        _: &SceneInput,
        _: &str,
        _: PoseReceipt,
        ui: Option<&Arc<UiInput>>,
    ) -> Result<(), Self::Error> {
        let mut record = self.0.borrow_mut();
        let ui = ui.expect("real HUD");
        assert!(Arc::ptr_eq(record.admitted.as_ref().unwrap(), ui));
        record.displayed.push(ui.clone());
        Ok(())
    }
}
type PoseGame = HostedOfflineGameSession<PoseRenderHost<Consumer>>;
fn setup(body: &str) -> (PoseGame, Game, Rc<RefCell<Record>>, tempfile::TempDir) {
    let (temp, scene, settings) = fixture();
    let r = Rc::new(RefCell::new(Record::default()));
    let p = NativeRenderSettings {
        model_cache: settings.model_cache.clone(),
        texture_cache: settings.texture_cache.clone(),
        camera: settings.camera,
        text: settings.text.clone(),
        ui: settings.ui,
        consumer: r.clone(),
    };
    let actions: incant_input::InputActions = serde_json::from_str(
        r#"{"jump":{"kind":"button","bindings":[{"type":"key","code":"Space"}]}}"#,
    )
    .unwrap();
    let ordinary = Game::new(
        scene.clone(),
        code(body),
        actions.clone(),
        vec![],
        48000,
        settings,
    )
    .unwrap();
    let pose = PoseGame::new(scene, code(body), actions, vec![], 48000, p).unwrap();
    (pose, ordinary, r, temp)
}
#[test]
fn font_only_hud_routes_fixed_step_input_and_pcm_identically_while_pose_display_never_ticks() {
    let (mut pose, mut ordinary, r, temp) = setup(
        "if(input.ui.events.some(e=>e.kind==='activated'))commands.despawn('00000000000000000000000005');",
    );
    assert!(pose.world().unwrap().spatial_view().unwrap().has_text());
    assert!(
        !pose
            .world()
            .unwrap()
            .spatial_view()
            .unwrap()
            .has_world_text()
    );
    temp.close().unwrap();
    let events = [
        InputEvent::Key {
            code: KeyCode::Tab,
            down: true,
        },
        InputEvent::Key {
            code: KeyCode::Space,
            down: true,
        },
    ];
    let mut a = vec![9.; pose.next_output_frames().unwrap() * 2];
    let mut b = a.clone();
    pose.tick_with_ui(&events, &[], &mut a).unwrap();
    ordinary.tick_with_ui(&events, &[], &mut b).unwrap();
    assert_eq!(a, b);
    assert_eq!(pose.input().unwrap(), ordinary.input().unwrap());
    assert_eq!(
        pose.world().unwrap().entity_count(),
        ordinary.world().unwrap().entity_count()
    );
    assert!(pose.world().unwrap().entity_identity(id(5)).is_none());
    let accepted = pose.host().unwrap().ui_input().unwrap().clone();
    assert!(accepted.widgets.iter().any(|w| !w.glyphs.is_empty()));
    let input = pose.input().unwrap().clone();
    let frames = pose.rendered_frames().unwrap();
    let next = pose.next_output_frames().unwrap();
    for fraction in [0., 0.5, 1.] {
        pose.present_host(PoseRequest::Display { fraction })
            .unwrap();
    }
    assert_eq!(pose.world().unwrap().tick(), 1);
    assert_eq!(pose.input().unwrap(), &input);
    assert_eq!(pose.rendered_frames().unwrap(), frames);
    assert_eq!(pose.next_output_frames().unwrap(), next);
    assert!(
        r.borrow()
            .displayed
            .iter()
            .all(|ui| Arc::ptr_eq(ui, &accepted))
    );
    pose.resize_ui([640., 300.]).unwrap();
    pose.present_host(PoseRequest::Reset {
        continuity_epoch: 1,
    })
    .unwrap();
    pose.present_host(PoseRequest::Display { fraction: 0. })
        .unwrap();
    assert_eq!(r.borrow().displayed.last().unwrap().viewport, [640., 300.]);
    assert_eq!(pose.world().unwrap().tick(), 1);
    assert_eq!(pose.input().unwrap(), &input);
}
#[test]
fn malformed_ui_is_retryable_but_candidate_admission_failure_destroys_composition() {
    let (mut pose, _, r, _temp) = setup("");
    let old = pose.host().unwrap().ui_input().unwrap().clone();
    assert!(pose.resize_ui([f32::NAN, 100.]).is_err());
    assert!(!pose.is_failed());
    assert!(Arc::ptr_eq(&old, pose.host().unwrap().ui_input().unwrap()));
    r.borrow_mut().fail = true;
    let mut pcm = vec![9.; pose.next_output_frames().unwrap() * 2];
    assert!(pose.tick_with_ui(&[], &[], &mut pcm).is_err());
    assert!(pose.is_failed());
    assert!(pose.host().is_err());
    assert!(Arc::ptr_eq(&old, r.borrow().admitted.as_ref().unwrap()));
    assert!(r.borrow().displayed.is_empty());
}

#[test]
fn direct_display_refuses_ui_generation_not_yet_admitted_with_the_spatial_owner() {
    use incant_game_runtime::SceneHostPresentation;
    let (_temp, scene, settings) = fixture();
    let mut changed = (*scene.ui().iter().find(|n| n.entity == id(2)).unwrap().node).clone();
    let UiKind::Button { text } = &mut changed.kind else {
        panic!()
    };
    text.content = "Changed".into();
    let mut world = NativeWorld::new(scene);
    let r = Rc::new(RefCell::new(Record::default()));
    let mut host = PoseRenderHost::<Consumer>::prepare(
        &world,
        NativeRenderSettings {
            model_cache: settings.model_cache,
            texture_cache: settings.texture_cache,
            camera: settings.camera,
            text: settings.text,
            ui: settings.ui,
            consumer: r.clone(),
        },
    )
    .unwrap();
    let old = host.ui_input().unwrap().clone();
    let mut commands = FrameCommands::default();
    commands
        .push(StructuralCommand::SetUi {
            entity: id(2),
            node: Some(Arc::new(changed)),
        })
        .unwrap();
    world.apply(&mut commands).unwrap();
    assert!(
        host.present(&world, PoseRequest::Display { fraction: 0.5 })
            .is_err()
    );
    assert!(r.borrow().displayed.is_empty());
    assert!(Arc::ptr_eq(&old, host.ui_input().unwrap()));
    host.sync(&world).unwrap();
    host.present(&world, PoseRequest::Display { fraction: 0.5 })
        .unwrap();
    assert!(!Arc::ptr_eq(&old, host.ui_input().unwrap()));
    assert_eq!(
        host.ui_input()
            .unwrap()
            .widgets
            .iter()
            .find(|w| w.id == id(2))
            .unwrap()
            .layout
            .source(),
        "Changed"
    );
    assert_eq!(world.tick(), 0);
}

#[test]
fn ordinary_and_pose_checkpoint_preparation_restore_the_same_ui_and_pcm_owner() {
    let (temp, scene, settings) = fixture();
    let program = || {
        CompiledScript::compile("exports.default={persistence:{version:1},update(t){t.state.set({tick:t.clock.tick});}};").unwrap()
    };
    let mut original = Game::new(
        scene.clone(),
        program(),
        Default::default(),
        vec![],
        48000,
        settings,
    )
    .unwrap();
    tick(&mut original, &[], &[UiEvent::Focus(id(3))]);
    let old_focus = original.host().unwrap().ui().unwrap().focus().unwrap();
    tick(
        &mut original,
        &[],
        &[UiEvent::Commit {
            target: old_focus.clone(),
            text: "e\u{301}日本".into(),
        }],
    );
    let saved = original.checkpoint_bytes().unwrap();
    let settings = |consumer| NativeRenderSettings {
        model_cache: temp.path().join("models"),
        texture_cache: temp.path().join("textures"),
        camera: id(8),
        text: Some(TextSettings::new(temp.path().join("fonts"))),
        ui: Some(UiSettings::new([600., 300.])),
        consumer,
    };
    let record = Rc::new(RefCell::new(Record::default()));
    let p = settings(record.clone());
    let mut ordinary = Game::from_checkpoint_bytes(
        &saved,
        &scene,
        program(),
        Default::default(),
        vec![],
        48000,
        NativeRenderSettings {
            model_cache: p.model_cache.clone(),
            texture_cache: p.texture_cache.clone(),
            camera: p.camera,
            text: p.text.clone(),
            ui: p.ui,
            consumer: (),
        },
    )
    .unwrap();
    let mut pose = PoseGame::from_checkpoint_bytes(
        &saved,
        &scene,
        program(),
        Default::default(),
        vec![],
        48000,
        p,
    )
    .unwrap();
    assert_eq!(pose.checkpoint_bytes().unwrap(), saved);
    assert_eq!(ordinary.checkpoint_bytes().unwrap(), saved);
    assert_ne!(
        pose.host().unwrap().ui().unwrap().focus().unwrap(),
        old_focus
    );
    assert_eq!(
        pose.host()
            .unwrap()
            .ui()
            .unwrap()
            .text(id(3))
            .unwrap()
            .text(),
        "Namee\u{301}日本"
    );
    temp.close().unwrap();
    for _ in 0..3 {
        let mut a = vec![9.; pose.next_output_frames().unwrap() * 2];
        let mut b = a.clone();
        pose.tick_with_ui(&[], &[], &mut a).unwrap();
        ordinary.tick_with_ui(&[], &[], &mut b).unwrap();
        assert_eq!(a, b);
        assert_eq!(
            pose.checkpoint_bytes().unwrap(),
            ordinary.checkpoint_bytes().unwrap()
        );
        let accepted = pose.host().unwrap().ui_input().unwrap().clone();
        pose.present_host(PoseRequest::Display { fraction: 0.5 })
            .unwrap();
        assert!(Arc::ptr_eq(
            record.borrow().displayed.last().unwrap(),
            &accepted
        ));
    }
}
