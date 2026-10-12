#[path = "../../incant_assets_runtime/tests/support/mod.rs"]
mod pcm;
use incant_audio_runtime::{ClipBinding, SourceState};
use incant_game_runtime::{GameError, HostedOfflineGameSession, OfflineGameSession};
use incant_input::{GamepadAxis, InputActions, InputEvent, KeyCode};
use incant_render_game::{NativeRenderHost, NativeRenderSettings, RenderHostError};
use incant_render_native::NativeRenderError;
use incant_runtime::{
    CookedAudio, CookedEntity, CookedPhysics, CookedScene, CookedSpatial, NativeAudioSource,
    NativeMeshRenderer, StableId, VisualAssetKind, VisualAssetRef,
};
use incant_script::{CompiledScript, SessionError};
use incant_types::{
    AudioSpatial, BodyMotion, Camera, CameraProjection, Collider, RigidBody, Transform, Velocity,
};
use serde_json::json;
use std::{fs, path::Path};
type Game = HostedOfflineGameSession<NativeRenderHost>;
fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
fn code(body: &str) -> CompiledScript {
    CompiledScript::compile(&format!(
        "exports.default={{update({{chunks,commands,input}}){{{body}}}}};"
    ))
    .unwrap()
}
fn actions() -> InputActions {
    serde_json::from_value(
        json!({"jump":{"kind":"button","bindings":[{"type":"key","code":"Space"}]}}),
    )
    .unwrap()
}
fn key(down: bool) -> InputEvent {
    InputEvent::Key {
        code: KeyCode::Space,
        down,
    }
}
fn bad_input() -> InputEvent {
    InputEvent::GamepadAxis {
        id: 99,
        axis: GamepadAxis::LeftX,
        value: 0.5,
    }
}
fn camera() -> Camera {
    Camera {
        fov_degrees: 60.,
        near: 0.1,
        far: 100.,
        projection: CameraProjection::Perspective {},
    }
}
struct Fixture {
    temp: tempfile::TempDir,
    scene: CookedScene,
    clip: ClipBinding,
}
impl Fixture {
    fn new(physics: bool) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let source = root.join("source");
        fs::create_dir(&source).unwrap();
        let vertices: [f32; 9] = [0., 0., 0., 1., 0., 0., 0., 1., 0.];
        fs::write(
            source.join("tri.bin"),
            vertices
                .into_iter()
                .flat_map(f32::to_le_bytes)
                .collect::<Vec<_>>(),
        )
        .unwrap();
        fs::write(source.join("tri.gltf"),json!({"asset":{"version":"2.0"},"buffers":[{"byteLength":36,"uri":"tri.bin"}],"bufferViews":[{"buffer":0,"byteLength":36}],"accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[0,0,0],"max":[1,1,0]}],"meshes":[{"primitives":[{"attributes":{"POSITION":0}}]}],"nodes":[{"mesh":0}],"scenes":[{"nodes":[0]}],"scene":0}).to_string()).unwrap();
        let model =
            incant_assets::cook_gltf(&source, Path::new("tri.gltf"), &root.join("models")).unwrap();
        let asset = VisualAssetRef {
            id: id(91),
            kind: VisualAssetKind::Model,
            fingerprint: std::array::from_fn(|i| {
                u8::from_str_radix(&model.metadata.fingerprint[i * 2..i * 2 + 2], 16).unwrap()
            }),
        };
        fs::remove_dir_all(source).unwrap();
        let key = pcm::cooked(&root.join("audio"), &[[0.2; 2]; 24000]);
        let clip = ClipBinding::load(id(90), &root.join("audio"), &key).unwrap();
        let mut scene = CookedScene::new(
            id(99),
            64,
            vec![
                CookedEntity {
                    id: id(1),
                    parent: None,
                    transform: Transform {
                        translation: [-2., 0., 0.],
                        ..Default::default()
                    },
                    velocity: Some(Velocity {
                        linear: [1., 0., 0.],
                    }),
                },
                CookedEntity {
                    id: id(2),
                    parent: None,
                    transform: Transform::default(),
                    velocity: None,
                },
            ],
        )
        .unwrap();
        if physics {
            scene = scene
                .with_physics(vec![CookedPhysics {
                    entity: id(1),
                    collider: Collider::default(),
                    body: Some(RigidBody {
                        motion: BodyMotion::Kinematic,
                        ..Default::default()
                    }),
                    angular_velocity: None,
                }])
                .unwrap();
        }
        scene = scene
            .with_audio(vec![
                CookedAudio {
                    entity: id(1),
                    bus: None,
                    listener: false,
                    source: Some(NativeAudioSource {
                        clip: id(90),
                        bus: None,
                        gain_db: 0.,
                        pan: 0.,
                        rate: 1.,
                        start_seconds: 0.,
                        playing: true,
                        looping: true,
                        streaming: false,
                        spatial: Some(AudioSpatial {
                            min_distance: 1.,
                            max_distance: 100.,
                        }),
                    }),
                },
                CookedAudio {
                    entity: id(2),
                    bus: None,
                    listener: true,
                    source: None,
                },
            ])
            .unwrap();
        let mut mesh = CookedSpatial::empty(id(1));
        mesh.mesh = Some(NativeMeshRenderer {
            mesh: id(91),
            cast_shadows: true,
        });
        let mut view = CookedSpatial::empty(id(2));
        view.camera = Some(camera());
        let bytes = scene
            .with_spatial(vec![asset], vec![mesh, view])
            .unwrap()
            .to_bytes();
        assert_eq!(&bytes[8..12], &4u32.to_le_bytes());
        Self {
            temp,
            scene: CookedScene::from_bytes(&bytes).unwrap(),
            clip,
        }
    }
    fn settings(&self) -> NativeRenderSettings {
        NativeRenderSettings {
            text: None,
            model_cache: self.temp.path().join("models"),
            texture_cache: self.temp.path().join("textures"),
            camera: id(2),
            consumer: (),
        }
    }
    fn game(self, script: CompiledScript) -> (Game, tempfile::TempDir) {
        let settings = self.settings();
        (
            Game::new(
                self.scene,
                script,
                actions(),
                vec![self.clip],
                48000,
                settings,
            )
            .unwrap(),
            self.temp,
        )
    }
}
fn position(game: &Game) -> f64 {
    game.host()
        .unwrap()
        .render_input()
        .input()
        .nodes
        .iter()
        .find(|n| n.mesh.is_some())
        .unwrap()
        .world
        .w_axis
        .x
}
fn next(game: &Game) -> Vec<f32> {
    vec![0.7; game.next_output_frames().unwrap() * 2]
}
fn failed(game: &mut Game) {
    assert!(game.is_failed());
    assert!(matches!(game.host(), Err(GameError::Failed)));
    assert!(matches!(game.world(), Err(GameError::Failed)));
    assert!(matches!(game.input(), Err(GameError::Failed)));
    assert!(matches!(game.source_state(id(1)), Err(GameError::Failed)));
    assert!(matches!(game.poll_audio(), Err(GameError::Failed)));
    assert!(matches!(game.rebind(actions()), Err(GameError::Failed)));
    assert!(matches!(game.tick(&[], &mut []), Err(GameError::Failed)));
}

#[test]
fn accepted_input_script_physics_audio_and_render_input_share_one_tick_after_source_and_cache_deletion()
 {
    let (mut game,temp)=Fixture::new(true).game(code("for(const c of chunks) for(let i=0;i<c.ids.length;i++) if(c.ids[i]==='00000000000000000000000001' && input.actions.jump.pressed) c.transforms[i*10]+=4;"));
    let input = game.host().unwrap().render_input();
    assert_eq!(input.tick(), 0);
    assert_eq!(position(&game), -2.);
    let storage = input.input().nodes.as_ptr();
    let asset = input
        .input()
        .nodes
        .iter()
        .find_map(|n| n.mesh.as_ref())
        .unwrap()
        .source
        .version_id();
    temp.close().unwrap();
    let mut output = next(&game);
    game.tick(&[key(true)], &mut output).unwrap();
    assert_eq!(position(&game), 2. + 1. / 64.);
    assert_eq!(
        game.world()
            .unwrap()
            .physics_state(id(1))
            .unwrap()
            .translation[0],
        position(&game)
    );
    assert_eq!(game.host().unwrap().render_input().tick(), 1);
    assert_eq!(game.rendered_frames().unwrap(), 750);
    let balance: f32 = output[128..]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|f| f[0] * f[0] - f[1] * f[1])
        .sum();
    assert!(
        balance < -0.01,
        "audio did not see accepted right-side pose: {balance}"
    );
    for tick in 2..=8 {
        game.tick(&[], &mut output).unwrap();
        assert_eq!(position(&game), 2. + tick as f64 / 64.);
        assert_eq!(game.host().unwrap().render_input().tick(), tick);
        let input = game.host().unwrap().render_input().input();
        assert_eq!(input.nodes.as_ptr(), storage);
        assert_eq!(
            input
                .nodes
                .iter()
                .find_map(|n| n.mesh.as_ref())
                .unwrap()
                .source
                .version_id(),
            asset
        );
    }
}

#[test]
fn invalid_input_and_wrong_output_size_keep_render_audio_input_and_vm_retryable() {
    let (mut game,_temp)=Fixture::new(false).game(code("for(const c of chunks) for(let i=0;i<c.ids.length;i++) if(c.ids[i]==='00000000000000000000000001' && input.actions.jump.released) c.transforms[i*10]+=1;"));
    let mut output = next(&game);
    game.tick(&[key(true)], &mut output).unwrap();
    let old = position(&game);
    let actions = game.input().unwrap().clone();
    let mut short = vec![0.7; output.len() - 1];
    assert!(matches!(
        game.tick(&[key(false)], &mut short),
        Err(GameError::Buffer { .. })
    ));
    assert!(short.iter().all(|v| *v == 0.7));
    output.fill(0.7);
    assert!(matches!(
        game.tick(&[key(false), bad_input()], &mut output),
        Err(GameError::Session(SessionError::Input(_)))
    ));
    assert!(output.iter().all(|v| *v == 0.7));
    assert_eq!(position(&game), old);
    assert_eq!(game.host().unwrap().render_input().tick(), 1);
    assert_eq!(game.input().unwrap(), &actions);
    assert!(!game.is_failed());
    game.tick(&[key(false)], &mut output).unwrap();
    assert_eq!(position(&game), old + 1. + 1. / 64.);
    assert_eq!(game.host().unwrap().render_input().tick(), 2);
}

#[test]
fn script_error_and_missing_camera_after_acceptance_destroy_the_composite_and_silence_pcm() {
    for (body, host_error) in [
        ("throw Error('accepted');", false),
        ("commands.despawn('00000000000000000000000002');", true),
    ] {
        let mut fixture = Fixture::new(false);
        if host_error {
            let mut audio = fixture.scene.audio().to_vec();
            audio[0].source.as_mut().unwrap().spatial = None;
            fixture.scene = fixture.scene.with_audio(audio).unwrap();
        }
        let (mut game, _temp) = fixture.game(code(body));
        let mut output = next(&game);
        let error = game.tick(&[], &mut output).unwrap_err();
        if host_error {
            assert!(
                matches!(error,GameError::Host(RenderHostError::Input(NativeRenderError::MissingCamera(x))) if x==id(2))
            );
        } else {
            match error {
                GameError::Session(SessionError::Script(_)) => {}
                other => panic!("unexpected failure: {other}"),
            }
        }
        assert!(output.iter().all(|v| *v == 0.));
        failed(&mut game);
    }
}

#[test]
fn retained_assets_survive_script_topology_changes_and_new_input_uses_committed_parent_pose() {
    let (mut game,temp)=Fixture::new(false).game(code("for(const c of chunks) for(let i=0;i<c.ids.length;i++) if(c.ids[i]==='00000000000000000000000002') c.transforms[i*10]=4; commands.setParent('00000000000000000000000001','00000000000000000000000002');"));
    temp.close().unwrap();
    let mut output = next(&game);
    game.tick(&[], &mut output).unwrap();
    assert_eq!(position(&game), 2. + 1. / 64.);
    let input = game.host().unwrap().render_input();
    assert_eq!(input.topology_generation(), 1);
    assert_eq!(input.tick(), 1);
    assert!(matches!(
        game.source_state(id(1)).unwrap(),
        SourceState::Playing
    ));
}

#[test]
fn bad_binding_refuses_before_guest_execution_and_plain_game_remains_spatial_strict() {
    let fixture = Fixture::new(false);
    let mut settings = fixture.settings();
    settings.model_cache = fixture.temp.path().join("missing");
    let script = || {
        CompiledScript::compile("throw Error('guest must not run before spatial binding');")
            .unwrap()
    };
    assert!(matches!(
        OfflineGameSession::new(fixture.scene.clone(), script(), actions(), vec![], 48000),
        Err(GameError::SpatialHost)
    ));
    let result = Game::new(
        fixture.scene,
        script(),
        actions(),
        vec![fixture.clip],
        48000,
        settings,
    );
    assert!(
        matches!(result,Err(GameError::Host(RenderHostError::Input(NativeRenderError::Asset{asset,..}))) if asset==id(91))
    );
}

#[derive(Debug, thiserror::Error)]
#[error("consumer rejected the frame")]
struct ConsumerFailure;
#[derive(Clone, Copy)]
enum Failure {
    None,
    Prepare,
    Accept,
    Unwind,
}
type Observation = (String, f64, Option<incant_render_native::UpdateKind>);
struct ConsumerSettings {
    failure: Failure,
    observations: std::rc::Rc<std::cell::RefCell<Vec<Observation>>>,
    drops: std::rc::Rc<std::cell::Cell<usize>>,
}
struct Consumer(ConsumerSettings);
impl Drop for Consumer {
    fn drop(&mut self) {
        self.0.drops.set(self.0.drops.get() + 1);
    }
}
impl incant_render_game::RenderInputConsumer for Consumer {
    type Settings = ConsumerSettings;
    type Error = ConsumerFailure;
    fn prepare(
        input: &incant_render_data::SceneInput,
        camera: &str,
        settings: Self::Settings,
    ) -> Result<Self, Self::Error> {
        if matches!(settings.failure, Failure::Prepare) {
            return Err(ConsumerFailure);
        }
        settings.observations.borrow_mut().push((
            camera.to_owned(),
            input
                .nodes
                .iter()
                .find(|n| n.mesh.is_some())
                .unwrap()
                .world
                .w_axis
                .x,
            None,
        ));
        Ok(Self(settings))
    }
    fn accept(
        &mut self,
        input: &incant_render_data::SceneInput,
        camera: &str,
        kind: incant_render_native::UpdateKind,
    ) -> Result<(), Self::Error> {
        self.0.observations.borrow_mut().push((
            camera.to_owned(),
            input
                .nodes
                .iter()
                .find(|n| n.mesh.is_some())
                .unwrap()
                .world
                .w_axis
                .x,
            Some(kind),
        ));
        match self.0.failure {
            Failure::Accept => Err(ConsumerFailure),
            Failure::Unwind => panic!("consumer unwind"),
            _ => Ok(()),
        }
    }
}
fn consumer_settings(
    fixture: &Fixture,
    failure: Failure,
) -> NativeRenderSettings<ConsumerSettings> {
    NativeRenderSettings {
        text: None,
        model_cache: fixture.temp.path().join("models"),
        texture_cache: fixture.temp.path().join("textures"),
        camera: id(2),
        consumer: ConsumerSettings {
            failure,
            observations: Default::default(),
            drops: Default::default(),
        },
    }
}
type ConsumingGame = HostedOfflineGameSession<NativeRenderHost<Consumer>>;

#[test]
fn consumer_preparation_precedes_guest_and_accepted_failures_drop_all_state_and_clear_pcm() {
    for failure in [Failure::Prepare, Failure::Accept, Failure::Unwind] {
        let fixture = Fixture::new(false);
        let settings = consumer_settings(&fixture, failure);
        let observations = settings.consumer.observations.clone();
        let drops = settings.consumer.drops.clone();
        let script = if matches!(failure, Failure::Prepare) {
            CompiledScript::compile("throw Error('must not run');").unwrap()
        } else {
            code("")
        };
        let result = ConsumingGame::new(
            fixture.scene,
            script,
            actions(),
            vec![fixture.clip],
            48000,
            settings,
        );
        if matches!(failure, Failure::Prepare) {
            assert!(matches!(
                result,
                Err(GameError::Host(RenderHostError::Consumer(ConsumerFailure)))
            ));
            assert!(observations.borrow().is_empty());
            continue;
        }
        let mut game = result.unwrap();
        let mut output = vec![0.7; game.next_output_frames().unwrap() * 2];
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| game.tick(&[], &mut output)));
        match failure {
            Failure::Accept => assert!(matches!(
                result.unwrap(),
                Err(GameError::Host(RenderHostError::Consumer(ConsumerFailure)))
            )),
            Failure::Unwind => assert!(result.is_err()),
            _ => unreachable!(),
        }
        assert!(game.is_failed());
        assert!(matches!(game.host(), Err(GameError::Failed)));
        assert!(matches!(game.world(), Err(GameError::Failed)));
        assert!(matches!(game.source_state(id(1)), Err(GameError::Failed)));
        assert!(output.iter().all(|v| *v == 0.));
        assert_eq!(drops.get(), 1);
        assert_eq!(observations.borrow()[1].1, -2. + 1. / 64.);
    }
}

#[test]
fn mandatory_consumer_observes_motion_and_rebuild_but_not_rejected_input() {
    let fixture = Fixture::new(false);
    let settings = consumer_settings(&fixture, Failure::None);
    let observations = settings.consumer.observations.clone();
    let mut game = ConsumingGame::new(fixture.scene, code("if(input.actions.jump.pressed) commands.setParent('00000000000000000000000001','00000000000000000000000002');"), actions(), vec![fixture.clip], 48000, settings).unwrap();
    fixture.temp.close().unwrap();
    let mut output = vec![0.; game.next_output_frames().unwrap() * 2];
    game.tick(&[], &mut output).unwrap();
    assert!(game.tick(&[bad_input()], &mut output).is_err());
    game.tick(&[key(true)], &mut output).unwrap();
    use incant_render_native::UpdateKind;
    assert_eq!(
        &*observations.borrow(),
        &[
            ("00000000000000000000000000000002".to_owned(), -2., None),
            (
                "00000000000000000000000000000002".to_owned(),
                -2. + 1. / 64.,
                Some(UpdateKind::Motion)
            ),
            (
                "00000000000000000000000000000002".to_owned(),
                -2. + 2. / 64.,
                Some(UpdateKind::Rebuilt)
            ),
        ]
    );
    assert_eq!(game.host().unwrap().consumer().0.drops.get(), 0);
    assert_eq!(game.host().unwrap().render_input().tick(), 2);
}

#[test]
fn mixed_game_retains_numeric_text_and_atlas_with_script_models_and_audible_pcm() {
    use incant_runtime::{CookedText, FontAssetRef, NativeText, TextLayout, TextWrap};
    use std::sync::Arc;
    let mut fixture = Fixture::new(false);
    let root = fixture.temp.path();
    let mut fonts = Vec::new();
    for (i, bytes) in [
        include_bytes!("../../incant_text/tests/fonts/NotoSans-Regular.ttf").as_slice(),
        include_bytes!("../../incant_text/tests/fonts/NotoSansArabic.ttf").as_slice(),
        include_bytes!("../../incant_text/tests/fonts/IncantCJKFixture.ttf").as_slice(),
    ]
    .into_iter()
    .enumerate()
    {
        fs::write(root.join("font.ttf"), bytes).unwrap();
        let font =
            incant_assets::cook_font(root, Path::new("font.ttf"), &root.join("fonts")).unwrap();
        fonts.push(FontAssetRef {
            id: id(80 + i as u128),
            fingerprint: std::array::from_fn(|n| {
                u8::from_str_radix(&font.metadata().fingerprint[2 * n..2 * n + 2], 16).unwrap()
            }),
        });
    }
    fixture.scene = fixture
        .scene
        .with_text(
            fonts.clone(),
            vec![CookedText {
                entity: id(1),
                text: Arc::new(NativeText {
                    content: "ffi سلام 中文".into(),
                    fonts: fonts.iter().map(|f| f.id).collect(),
                    layout: TextLayout {
                        font_size: 20.,
                        line_height: 28.,
                        width: 500.,
                        wrap: TextWrap::WordOrGlyph,
                    },
                }),
            }],
        )
        .unwrap();
    let mut settings = fixture.settings();
    settings.text = Some(incant_render_native::TextSettings::new(root.join("fonts")));
    let mut game = Game::new(
        fixture.scene,
        code("for (const c of chunks) for(let i=0;i<c.ids.length;i++) c.transforms[i*10] += 2;"),
        actions(),
        vec![fixture.clip],
        48000,
        settings,
    )
    .unwrap();
    fixture.temp.close().unwrap();
    let original = game
        .host()
        .unwrap()
        .render_input()
        .input()
        .text
        .as_ref()
        .unwrap();
    let atlas = original.atlas.clone();
    let layout = original.nodes[0].layout.clone();
    assert!(layout.missing_clusters().is_empty());
    assert!(layout.glyphs().iter().any(|g| g.bidi_level % 2 == 1));
    assert!(atlas.pixels().iter().any(|&p| p > 0));
    for tick in 1..=8 {
        let mut pcm = next(&game);
        game.tick(&[], &mut pcm).unwrap();
        assert!(pcm.iter().any(|&v| v != 0.));
        let input = game.host().unwrap().render_input();
        assert_eq!(input.tick(), tick);
        let text = input.input().text.as_ref().unwrap();
        assert!(Arc::ptr_eq(&text.atlas, &atlas));
        assert!(Arc::ptr_eq(&text.nodes[0].layout, &layout));
        assert_eq!(text.nodes[0].world.w_axis.x, position(&game));
        assert_eq!(position(&game), -2. + tick as f64 * (2. + 1. / 64.));
    }
}
