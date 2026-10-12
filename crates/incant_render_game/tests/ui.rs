use incant_game_runtime::{GameError, HostedOfflineGameSession, OfflineGameSession, SceneHost};
use incant_input::{InputEvent, KeyCode};
use incant_render_game::{
    NativeRenderHost, NativeRenderSettings, RenderHostError, RenderInputConsumer, UiSettings,
};
use incant_render_native::{TextSettings, UpdateKind};
use incant_runtime::*;
use incant_script::CompiledScript;
use incant_types::{Camera, CameraProjection, TextWrap, UiKind, UiLength, UiNode, UiStyle, UiText};
use incant_ui::UiEvent;
use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
const FONT: &[u8] = include_bytes!("../../incant_text/tests/fonts/NotoSans-Regular.ttf");
fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
fn code(body: &str) -> CompiledScript {
    CompiledScript::compile(&format!(
        "exports.default={{update({{commands,input}}){{{body}}}}};"
    ))
    .unwrap()
}
fn fixture() -> (tempfile::TempDir, CookedScene, NativeRenderSettings) {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(temp.path().join("font.ttf"), FONT).unwrap();
    let font = incant_assets::cook_font(
        temp.path(),
        Path::new("font.ttf"),
        &temp.path().join("fonts"),
    )
    .unwrap();
    let fingerprint = std::array::from_fn(|i| {
        u8::from_str_radix(&font.metadata().fingerprint[2 * i..2 * i + 2], 16).unwrap()
    });
    let mut camera = CookedSpatial::empty(id(8));
    camera.camera = Some(Camera {
        fov_degrees: 60.,
        near: 0.1,
        far: 100.,
        projection: CameraProjection::Perspective {},
    });
    let entities = (1..=8)
        .map(|n| CookedEntity {
            id: id(n),
            parent: matches!(n, 2..=4).then(|| id(1)),
            transform: Default::default(),
            velocity: None,
        })
        .collect();
    let text = UiText {
        content: "Name".into(),
        fonts: vec![id(9)],
        font_size: 16.,
        line_height: 24.,
        wrap: TextWrap::WordOrGlyph,
    };
    let kinds = [
        UiKind::Root,
        UiKind::Button { text: text.clone() },
        UiKind::TextInput { text },
        UiKind::Container,
    ];
    let ui = kinds
        .into_iter()
        .enumerate()
        .map(|(n, kind)| CookedUi {
            entity: id(n as u128 + 1),
            node: Arc::new(UiNode {
                kind,
                style: if n == 0 {
                    UiStyle::default()
                } else {
                    UiStyle {
                        size: [UiLength::Pixels(160.), UiLength::Pixels(32.)],
                        ..Default::default()
                    }
                },
                order: 0,
                disabled: false,
            }),
        })
        .collect();
    let scene = CookedScene::new(id(99), 60, entities)
        .unwrap()
        .with_spatial(vec![], vec![camera])
        .unwrap()
        .with_text(
            vec![FontAssetRef {
                id: id(9),
                fingerprint,
            }],
            vec![],
        )
        .unwrap()
        .with_ui(ui)
        .unwrap();
    let settings = NativeRenderSettings {
        model_cache: temp.path().join("models"),
        texture_cache: temp.path().join("textures"),
        camera: id(8),
        text: Some(TextSettings::new(temp.path().join("fonts"))),
        ui: Some(UiSettings::new([320., 200.])),
        consumer: (),
    };
    (temp, scene, settings)
}
type Game = HostedOfflineGameSession<NativeRenderHost>;
fn game(body: &str) -> (Game, tempfile::TempDir) {
    let (temp, scene, settings) = fixture();
    let actions = serde_json::from_str(
        r#"{"jump":{"kind":"button","bindings":[{"type":"key","code":"Space"}]}}"#,
    )
    .unwrap();
    (
        Game::new(scene, code(body), actions, vec![], 48000, settings).unwrap(),
        temp,
    )
}
fn tick(game: &mut Game, keys: &[InputEvent], ui: &[UiEvent]) {
    let mut pcm = vec![1.; game.next_output_frames().unwrap() * 2];
    game.tick_with_ui(keys, ui, &mut pcm).unwrap();
    assert!(pcm.iter().all(|n| *n == 0.));
}
#[test]
fn actual_composed_ui_is_seen_once_by_guest_with_consumed_gameplay_keys_and_source_deleted_fonts() {
    let (mut game, temp) = game(
        r#"if(input.ui.focused!==null && typeof input.ui.focused!=='string')throw Error('focus');if(input.ui.events.length){if(input.actions.jump.pressed||input.keyboard.held.includes('Space'))throw Error('leak');if(input.ui.events[0].kind==='activated')commands.despawn('00000000000000000000000005');}"#,
    );
    let initial_paint = Arc::clone(game.host().unwrap().ui_paint().unwrap());
    assert!(initial_paint.atlas.pixels().iter().any(|v| *v != 0));
    temp.close().unwrap(); // layout/IME must use the actual retained packaged font bytes
    tick(
        &mut game,
        &[
            InputEvent::Key {
                code: KeyCode::Tab,
                down: true,
            },
            InputEvent::Key {
                code: KeyCode::Space,
                down: true,
            },
        ],
        &[],
    );
    assert!(Arc::ptr_eq(
        &initial_paint,
        game.host().unwrap().ui_paint().unwrap()
    ));
    assert_eq!(game.world().unwrap().tick(), 1);
    assert!(game.world().unwrap().entity_identity(id(5)).is_none());
    assert_eq!(game.input().unwrap().ui.events.len(), 1);
    assert!(!game.input().unwrap().actions["jump"].pressed);
    tick(&mut game, &[], &[UiEvent::Focus(id(3))]);
    let target = game.host().unwrap().ui().unwrap().focus().unwrap();
    tick(
        &mut game,
        &[],
        &[UiEvent::Commit {
            target,
            text: "日本語".into(),
        }],
    );
    assert_eq!(
        game.host()
            .unwrap()
            .ui()
            .unwrap()
            .text(id(3))
            .unwrap()
            .text(),
        "Name日本語"
    );
    assert!(game.checkpoint_bytes().is_err());
}

#[test]
fn guest_numeric_ui_motion_rejects_and_restores_all_rows_before_publication() {
    let (_temp, scene, _) = fixture();
    let mut world = NativeWorld::new(scene);
    let before = world.snapshot();
    let mut host = incant_script::ScriptHost::new(CompiledScript::compile("exports.default={update({chunks}){for(const c of chunks)for(let i=0;i<c.ids.length;i++)c.transforms[i*c.transformStride]+=1;}};").unwrap()).unwrap();
    assert!(host.tick(&mut world, 1. / 60.).is_err());
    assert_eq!(world.snapshot(), before);
    assert_eq!(world.ui_view().unwrap().generation(), 0);
    assert_eq!(world.tick(), 0);
}
#[test]
fn failed_guest_is_terminal_and_malformed_input_or_resize_preserves_the_accepted_frame() {
    let (mut game, _) = game("if(input.ui.events.length)throw Error('trap');");
    tick(&mut game, &[], &[UiEvent::Focus(id(3))]);
    let original = Arc::clone(game.host().unwrap().ui().unwrap().layout().unwrap());
    let before = game.world().unwrap().tick();
    let input = game.input().unwrap().clone();
    let frames = game.rendered_frames().unwrap();
    assert!(game.resize_ui([f32::NAN, 200.]).is_err());
    assert!(!game.is_failed());
    assert!(Arc::ptr_eq(
        &original,
        game.host().unwrap().ui().unwrap().layout().unwrap()
    ));
    let target = game.host().unwrap().ui().unwrap().focus().unwrap();
    let bad = UiEvent::Preedit {
        target: target.clone(),
        text: "é".into(),
        cursor: Some(1..2),
    };
    let mut pcm = vec![9.; game.next_output_frames().unwrap() * 2];
    assert!(game.tick_with_ui(&[], &[bad], &mut pcm).is_err());
    assert!(!game.is_failed());
    assert!(pcm.iter().all(|n| *n == 9.));
    assert_eq!(game.world().unwrap().tick(), before);
    assert_eq!(game.input().unwrap(), &input);
    assert_eq!(game.rendered_frames().unwrap(), frames);
    game.resize_ui([640., 300.]).unwrap();
    assert_eq!(game.world().unwrap().tick(), before);
    assert_eq!(game.input().unwrap(), &input);
    assert_eq!(game.rendered_frames().unwrap(), frames);
    assert!(!Arc::ptr_eq(
        &original,
        game.host().unwrap().ui().unwrap().layout().unwrap()
    ));
    assert!(
        game.tick_with_ui(
            &[],
            &[UiEvent::Commit {
                target,
                text: "X".into()
            }],
            &mut pcm
        )
        .is_err()
    );
    assert!(game.is_failed());
    assert!(game.host().is_err());
    assert!(game.input().is_err());
    assert!(pcm.iter().all(|n| *n == 0.));
}
#[test]
fn same_tick_reparent_preserves_typed_value_and_expires_old_focus_receipts() {
    let (mut game, _) = game(
        "if(input.ui.events.some(e=>e.kind==='value_changed'))commands.setParent('00000000000000000000000003','00000000000000000000000004');",
    );
    tick(&mut game, &[], &[UiEvent::Focus(id(3))]);
    let target = game.host().unwrap().ui().unwrap().focus().unwrap();
    tick(
        &mut game,
        &[],
        &[UiEvent::Commit {
            target: target.clone(),
            text: "X".into(),
        }],
    );
    assert_eq!(
        game.host()
            .unwrap()
            .ui()
            .unwrap()
            .text(id(3))
            .unwrap()
            .text(),
        "NameX"
    );
    tick(
        &mut game,
        &[InputEvent::Focus { focused: false }],
        &[UiEvent::Commit {
            target,
            text: "stale".into(),
        }],
    );
    assert_eq!(game.input().unwrap().ui.discarded_events, 1);
    assert!(!game.input().unwrap().focused);
    assert!(game.host().unwrap().ui().unwrap().focus().is_none());
    assert_eq!(
        game.host()
            .unwrap()
            .ui()
            .unwrap()
            .text(id(3))
            .unwrap()
            .text(),
        "NameX"
    );
}
#[test]
fn ordinary_despawn_of_complete_tree_clears_retained_owner_without_stale_parent_links() {
    let (mut game, _) = game(
        "if(input.ui.events.length){commands.despawn('00000000000000000000000002');commands.despawn('00000000000000000000000003');commands.despawn('00000000000000000000000004');commands.despawn('00000000000000000000000001');}",
    );
    tick(&mut game, &[], &[UiEvent::Focus(id(2)), UiEvent::Activate]);
    assert!(game.host().unwrap().ui().unwrap().widgets().is_empty());
    assert!(
        game.host()
            .unwrap()
            .ui()
            .unwrap()
            .layout()
            .unwrap()
            .nodes
            .is_empty()
    );
    tick(
        &mut game,
        &[InputEvent::Key {
            code: KeyCode::Tab,
            down: true,
        }],
        &[],
    );
    assert!(
        game.input()
            .unwrap()
            .keyboard
            .pressed
            .contains(&KeyCode::Tab)
    );
}
#[test]
fn unsupported_consumers_refuse_before_prepare_vm_and_audio() {
    static CALLS: AtomicUsize = AtomicUsize::new(0);
    struct Consumer;
    impl RenderInputConsumer for Consumer {
        type Settings = ();
        type Error = std::convert::Infallible;
        fn prepare(
            _: &incant_render_data::SceneInput,
            _: &str,
            _: (),
        ) -> Result<Self, Self::Error> {
            CALLS.fetch_add(1, Ordering::SeqCst);
            Ok(Self)
        }
        fn accept(
            &mut self,
            _: &incant_render_data::SceneInput,
            _: &str,
            _: UpdateKind,
        ) -> Result<(), Self::Error> {
            panic!("unsupported")
        }
    }
    let (_, scene, settings) = fixture();
    let program = CompiledScript::compile("throw Error('VM must not start');").unwrap();
    let result = HostedOfflineGameSession::<NativeRenderHost<Consumer>>::new(
        scene.clone(),
        program,
        Default::default(),
        vec![],
        48000,
        settings,
    );
    assert!(matches!(result, Err(GameError::UiHost)));
    assert_eq!(CALLS.load(Ordering::SeqCst), 0);
    assert!(matches!(
        OfflineGameSession::new(scene.clone(), code(""), Default::default(), vec![], 48000),
        Err(GameError::UiHost)
    ));
    let world = NativeWorld::new(scene);
    let (_, _, settings) = fixture();
    assert!(matches!(
        NativeRenderHost::<Consumer>::prepare(&world, settings),
        Err(RenderHostError::UiUnsupported)
    ));
    assert_eq!(CALLS.load(Ordering::SeqCst), 0);
}
