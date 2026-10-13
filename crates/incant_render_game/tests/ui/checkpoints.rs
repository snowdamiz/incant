use super::*;
use incant_script::{CheckpointError, NativeSession, PendingSessionCheckpoint};

const SOURCE: &str = r#"exports.default={persistence:{version:1},update(t){
 let s=t.state.get()||{ticks:0,timers:0,edits:0};
 s={ticks:s.ticks+1,timers:s.timers+t.timers.events.length,edits:s.edits+t.input.ui.events.filter(e=>e.kind==='value_changed').length};
 if(t.clock.tick===1)t.timers.set('beat',3,3);
 if(t.input.ui.events.some(e=>e.kind==='value_changed'))t.commands.setParent('00000000000000000000000003','00000000000000000000000004');
 t.state.set(s);
 for(const c of t.chunks)for(let i=0;i<c.ids.length;i++)if(c.ids[i]==='00000000000000000000000005')c.transforms[i*c.transformStride]=s.ticks*10+s.timers+s.edits;
}};"#;
fn program() -> CompiledScript {
    CompiledScript::compile(SOURCE).unwrap()
}
fn settings(root: &Path, viewport: [f32; 2]) -> NativeRenderSettings {
    NativeRenderSettings {
        model_cache: root.join("models"),
        texture_cache: root.join("textures"),
        camera: id(8),
        text: Some(TextSettings::new(root.join("fonts"))),
        ui: Some(UiSettings::new(viewport)),
        consumer: (),
    }
}
fn start(scene: CookedScene, root: &Path) -> Game {
    Game::new(
        scene,
        program(),
        Default::default(),
        vec![],
        44100,
        settings(root, [320., 200.]),
    )
    .unwrap()
}
fn load(
    bytes: &[u8],
    scene: &CookedScene,
    root: &Path,
) -> Result<Game, GameError<RenderHostError>> {
    Game::from_checkpoint_bytes(
        bytes,
        scene,
        program(),
        Default::default(),
        vec![],
        44100,
        settings(root, [600., 300.]),
    )
}
fn edit(game: &mut Game) {
    tick(game, &[], &[UiEvent::Focus(id(3))]);
    let target = game.host().unwrap().ui().unwrap().focus().unwrap();
    tick(
        game,
        &[],
        &[UiEvent::Commit {
            target: target.clone(),
            text: "e\u{301}日本".into(),
        }],
    );
    let target = game.host().unwrap().ui().unwrap().focus().unwrap();
    let mut selection = game
        .host()
        .unwrap()
        .ui()
        .unwrap()
        .text(id(3))
        .unwrap()
        .selection();
    selection.anchor = 13;
    selection.focus = 4;
    tick(
        game,
        &[InputEvent::Key {
            code: KeyCode::KeyA,
            down: true,
        }],
        &[UiEvent::Select { target, selection }],
    );
}
#[test]
fn actual_host_restores_text_selection_focus_timers_draw_input_and_fractional_clock() {
    let (temp, scene, _) = fixture();
    let mut original = start(scene.clone(), temp.path());
    edit(&mut original);
    for _ in 0..4 {
        tick(&mut original, &[], &[]);
    }
    let saved = original.checkpoint_bytes().unwrap();
    assert_eq!(&saved[8..12], &2u32.to_le_bytes());
    let old_focus = original.host().unwrap().ui().unwrap().focus().unwrap();
    let mut resumed = load(&saved, &scene, temp.path()).unwrap();
    assert_ne!(
        original.world().unwrap().incarnation(),
        resumed.world().unwrap().incarnation()
    );
    assert_eq!(saved, resumed.checkpoint_bytes().unwrap());
    assert_ne!(
        old_focus,
        resumed.host().unwrap().ui().unwrap().focus().unwrap()
    );
    let widget = resumed
        .host()
        .unwrap()
        .ui_input()
        .unwrap()
        .widgets
        .iter()
        .find(|w| w.id == id(3))
        .unwrap();
    assert!(widget.focused);
    assert_eq!(widget.layout.source(), "Namee\u{301}日本");
    assert_eq!(
        widget.edit.as_ref().unwrap().selection.unwrap().range(),
        4..13
    );
    assert!(resumed.input().unwrap().keyboard.held.is_empty());
    assert!(resumed.input().unwrap().keyboard.pressed.is_empty());
    assert!(resumed.input().unwrap().keyboard.released.is_empty());
    assert!(resumed.input().unwrap().ui.events.is_empty());
    assert_eq!(
        resumed.rendered_frames().unwrap(),
        original.rendered_frames().unwrap()
    );
    // Both real hosts retain the packaged font bytes after all source/cache removal.
    temp.close().unwrap();
    for n in 0..17 {
        tick(&mut original, &[], &[]);
        let stale = if n == 0 {
            vec![UiEvent::Commit {
                target: old_focus.clone(),
                text: "stale".into(),
            }]
        } else {
            vec![]
        };
        tick(&mut resumed, &[], &stale);
        if n == 0 {
            assert_eq!(resumed.input().unwrap().ui.discarded_events, 1);
        }
        assert_eq!(
            original.checkpoint_bytes().unwrap(),
            resumed.checkpoint_bytes().unwrap()
        );
        assert_eq!(
            original.rendered_frames().unwrap(),
            resumed.rendered_frames().unwrap()
        );
    }
    assert!(
        resumed
            .world()
            .unwrap()
            .inspect(id(5))
            .unwrap()
            .transform
            .translation[0]
            > 240.
    );
    assert_eq!(
        resumed
            .host()
            .unwrap()
            .ui()
            .unwrap()
            .text(id(3))
            .unwrap()
            .text(),
        "Namee\u{301}日本"
    );
}
fn sha(bytes: &[u8]) -> [u8; 32] {
    let s = incant_assets_runtime::sha256(bytes);
    std::array::from_fn(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).unwrap())
}
fn seal(bytes: &mut [u8], header: usize) {
    let mut joined = bytes[..header - 32].to_vec();
    joined.extend(&bytes[header..]);
    bytes[header - 32..header].copy_from_slice(&sha(&joined));
}
fn ui_start(bytes: &[u8]) -> usize {
    132 + (0..4)
        .map(|i| u32::from_le_bytes(bytes[80 + 4 * i..84 + 4 * i].try_into().unwrap()) as usize)
        .sum::<usize>()
}
#[test]
fn malformed_or_wrong_program_data_refuses_before_consumer_and_preserves_live_game() {
    let (temp, scene, _) = fixture();
    let mut live = start(scene.clone(), temp.path());
    edit(&mut live);
    let saved = live.checkpoint_bytes().unwrap();
    let old = live.world().unwrap().incarnation();
    let start = ui_start(&saved);
    let mut cases = Vec::new();
    for offset in [8, 12, 80, 84, 88, 92, 96] {
        let mut b = saved.clone();
        b[offset..offset + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        seal(&mut b, 132);
        cases.push(b);
    }
    let mut missing = saved[..start].to_vec();
    missing.drain(96..100);
    missing[8..12].copy_from_slice(&1u32.to_le_bytes());
    seal(&mut missing, 128);
    cases.push(missing);
    for offset in [16, 20, 44, 80, 80 + 32, 80 + 34] {
        let mut b = saved.clone();
        b[start + offset] = 255;
        seal(&mut b[start..], 80);
        seal(&mut b, 132);
        cases.push(b);
    }
    let mut short = saved.clone();
    short.pop();
    cases.push(short);
    for bad in cases {
        // A consumer counter also catches late refusal after resource publication.
        let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
        let base = settings(temp.path(), [320., 200.]);
        let gate = NativeRenderSettings {
            model_cache: base.model_cache,
            texture_cache: base.texture_cache,
            camera: base.camera,
            text: base.text,
            ui: base.ui,
            consumer: Gate {
                refuse: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                seen: Arc::clone(&seen),
            },
        };
        assert!(
            HostedOfflineGameSession::<NativeRenderHost<Gate>>::from_checkpoint_bytes(
                &bad,
                &scene,
                program(),
                Default::default(),
                vec![],
                44100,
                gate
            )
            .is_err()
        );
        assert!(seen.lock().unwrap().is_empty());
        assert_eq!(live.checkpoint_bytes().unwrap(), saved);
        assert_eq!(live.world().unwrap().incarnation(), old);
    }
    let wrong =
        CompiledScript::compile("exports.default={persistence:{version:1},update(){}};").unwrap();
    assert!(matches!(
        Game::from_checkpoint_bytes(
            &saved,
            &scene,
            wrong,
            Default::default(),
            vec![],
            44100,
            settings(temp.path(), [320., 200.])
        ),
        Err(GameError::Checkpoint(CheckpointError::ProgramMismatch))
    ));
    assert!(
        NativeSession::from_checkpoint_bytes(&saved, &scene, program(), Default::default())
            .is_err()
    );
    let pending =
        PendingSessionCheckpoint::new(&saved, &scene, program(), Default::default()).unwrap();
    assert!(
        pending
            .start_with_ui(live.host().unwrap().ui().unwrap())
            .is_err()
    );
    tick(&mut live, &[], &[]);
    assert_eq!(live.world().unwrap().tick(), 4);
}
#[test]
fn active_preedit_refuses_and_cancel_preserves_the_previous_save() {
    let (temp, scene, _) = fixture();
    let mut live = start(scene, temp.path());
    edit(&mut live);
    let focus = live.host().unwrap().ui().unwrap().focus().unwrap();
    tick(
        &mut live,
        &[],
        &[UiEvent::Preedit {
            target: focus.clone(),
            text: "候補".into(),
            cursor: Some(0..3),
        }],
    );
    assert!(matches!(
        live.checkpoint_bytes(),
        Err(GameError::Checkpoint(CheckpointError::Ui(
            incant_ui::UiCheckpointError::CompositionActive
        )))
    ));
    assert_eq!(
        live.host()
            .unwrap()
            .ui()
            .unwrap()
            .text(id(3))
            .unwrap()
            .text(),
        "Namee\u{301}日本"
    );
    tick(&mut live, &[], &[UiEvent::Cancel { target: focus }]);
    assert!(live.checkpoint_bytes().is_ok());
}
#[test]
fn consumer_restore_failure_is_atomic_and_default_host_cannot_omit_ui() {
    let (temp, scene, _) = fixture();
    let mut live = start(scene.clone(), temp.path());
    edit(&mut live);
    let saved = live.checkpoint_bytes().unwrap();
    let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    let base = settings(temp.path(), [640., 400.]);
    let gate = NativeRenderSettings {
        model_cache: base.model_cache,
        texture_cache: base.texture_cache,
        camera: base.camera,
        text: base.text,
        ui: base.ui,
        consumer: Gate {
            refuse: Arc::new(std::sync::atomic::AtomicBool::new(true)),
            seen: Arc::clone(&seen),
        },
    };
    assert!(matches!(
        HostedOfflineGameSession::<NativeRenderHost<Gate>>::from_checkpoint_bytes(
            &saved,
            &scene,
            program(),
            Default::default(),
            vec![],
            44100,
            gate
        ),
        Err(GameError::Host(RenderHostError::Consumer(_)))
    ));
    assert_eq!(
        seen.lock().unwrap()[0],
        ("prepare", [640., 400.], vec![id(3)])
    );
    assert_eq!(live.checkpoint_bytes().unwrap(), saved);
    struct DefaultHost;
    impl SceneHost for DefaultHost {
        const SUPPORTS_UI: bool = true;
        type Settings = ();
        type Error = std::convert::Infallible;
        fn prepare(_: &NativeWorld, _: ()) -> Result<Self, Self::Error> {
            panic!("default UI restore must refuse before prepare")
        }
        fn sync(&mut self, _: &NativeWorld) -> Result<(), Self::Error> {
            Ok(())
        }
    }
    assert!(matches!(
        HostedOfflineGameSession::<DefaultHost>::from_checkpoint_bytes(
            &saved,
            &scene,
            program(),
            Default::default(),
            vec![],
            44100,
            ()
        ),
        Err(GameError::UiHost)
    ));
    let mut no_ui = settings(temp.path(), [320., 200.]);
    no_ui.ui = None;
    assert!(load(&saved, &scene, temp.path()).is_ok());
    assert!(
        Game::from_checkpoint_bytes(
            &saved,
            &scene,
            program(),
            Default::default(),
            vec![],
            44100,
            no_ui
        )
        .is_err()
    );
}

#[test]
fn fresh_process_restore_matches_uninterrupted_hosted_state() {
    let (temp, scene, _) = fixture();
    let mut game = start(scene.clone(), temp.path());
    edit(&mut game);
    for _ in 0..4 {
        tick(&mut game, &[], &[]);
    }
    std::fs::write(temp.path().join("world.scene"), scene.to_bytes()).unwrap();
    std::fs::write(
        temp.path().join("start.save"),
        game.checkpoint_bytes().unwrap(),
    )
    .unwrap();
    std::fs::remove_file(temp.path().join("font.ttf")).unwrap();
    for _ in 0..17 {
        tick(&mut game, &[], &[]);
    }
    let expected = game.checkpoint_bytes().unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "checkpoints::fresh_process_child", "--nocapture"])
        .env("INCANT_UI_CHECKPOINT_CHILD", temp.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        std::fs::read(temp.path().join("final.save")).unwrap(),
        expected
    );
}
#[test]
fn fresh_process_child() {
    let Some(path) = std::env::var_os("INCANT_UI_CHECKPOINT_CHILD") else {
        return;
    };
    let root = std::path::PathBuf::from(path);
    assert!(!root.join("font.ttf").exists());
    let scene = CookedScene::from_bytes(&std::fs::read(root.join("world.scene")).unwrap()).unwrap();
    let bytes = std::fs::read(root.join("start.save")).unwrap();
    // Known fixed test behavior is compiled in this new process. This is an
    // actual process continuation control, not a shipped embedded-code proof.
    let mut game = load(&bytes, &scene, &root).unwrap();
    assert_eq!(game.world().unwrap().tick(), 7);
    for _ in 0..17 {
        tick(&mut game, &[], &[]);
    }
    std::fs::write(root.join("final.save"), game.checkpoint_bytes().unwrap()).unwrap();
}

#[test]
fn restart_recreates_actual_ui_host_buffers_focus_router_and_canonical_paint() {
    let (temp, scene, _) = fixture();
    let seed =
        incant_game_runtime::GameSeed::new(scene, program(), Default::default(), vec![]).unwrap();
    let mut game = Game::from_seed(&seed, 44100, settings(temp.path(), [320., 200.])).unwrap();
    let initial = game.host().unwrap().ui_input().unwrap().clone();
    edit(&mut game);
    let old_focus = game.host().unwrap().ui().unwrap().focus().unwrap();
    let old_id = game.world().unwrap().incarnation();
    std::fs::remove_file(temp.path().join("font.ttf")).unwrap();
    game.restart_from(&seed, settings(temp.path(), [600., 300.]))
        .unwrap();
    assert_ne!(old_id, game.world().unwrap().incarnation());
    assert_eq!(game.world().unwrap().tick(), 0);
    let ui = game.host().unwrap().ui().unwrap();
    assert_eq!(ui.text(id(3)).unwrap().text(), "Name");
    assert!(ui.focus().is_none());
    let draw = game.host().unwrap().ui_input().unwrap();
    assert_eq!(draw.viewport, [600., 300.]);
    assert!(!Arc::ptr_eq(draw, &initial));
    assert!(
        game.host()
            .unwrap()
            .ui_paint()
            .unwrap()
            .atlas
            .pixels()
            .iter()
            .any(|p| *p != 0)
    );
    tick(
        &mut game,
        &[],
        &[UiEvent::Commit {
            target: old_focus,
            text: "stale".into(),
        }],
    );
    assert_eq!(game.input().unwrap().ui.discarded_events, 1);
    assert_eq!(
        game.host()
            .unwrap()
            .ui()
            .unwrap()
            .text(id(3))
            .unwrap()
            .text(),
        "Name"
    );
    assert!(game.input().unwrap().keyboard.held.is_empty());
}
