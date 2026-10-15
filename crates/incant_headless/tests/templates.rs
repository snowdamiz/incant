//! Actual startup cooking and disposable native sessions, with no GPU/window.
use incant_agent_native::{Session, StartOptions};
use incant_doc::Project;
use incant_input::{InputEvent, KeyCode};
use incant_runtime::StableId;
use incant_script::{CompiledScript, ScriptError};
use incant_templates::{CreatedProject, Template};
use std::{collections::BTreeSet, fs, path::PathBuf, process::Command, sync::Mutex};

// Startup may spawn the trusted TS cook. Serialize these tests so a fork cannot
// inherit a sibling test's live advisory journal lock during create/reopen.
static SERIAL: Mutex<()> = Mutex::new(());

struct Fixture {
    _temp: tempfile::TempDir,
    created: CreatedProject,
    document: Project,
    authored: Vec<(PathBuf, Vec<u8>)>,
}
impl Fixture {
    fn new(kind: Template) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap().join("starter");
        let created = incant_templates::create(&root, kind, "Behavior test").unwrap();
        let document = Project::from_text(&fs::read_to_string(&created.project).unwrap()).unwrap();
        let authored = [
            created.project.clone(),
            created.journal.clone(),
            root.join("controller.ts"),
        ]
        .into_iter()
        .filter(|p| p.exists())
        .map(|p| {
            let bytes = fs::read(&p).unwrap();
            (p, bytes)
        })
        .collect();
        Self {
            _temp: temp,
            created,
            document,
            authored,
        }
    }
    fn start(&self) -> Session {
        let fallback = if self.created.template == Template::Empty {
            no_op
        } else {
            forbidden_fallback
        };
        let session = Session::new(
            &self.document,
            self.created.project.parent().unwrap(),
            &StartOptions {
                scene: Some(self.created.roles["scene"].clone()),
                camera: Some(self.created.roles["camera"].clone()),
                ..StartOptions::default()
            },
            8000,
            fallback,
        )
        .unwrap();
        assert_eq!(
            session.authored_scripts,
            self.created.template != Template::Empty
        );
        session
    }
    fn unchanged(&self) {
        for (path, bytes) in &self.authored {
            assert_eq!(
                fs::read(path).unwrap(),
                *bytes,
                "{} changed",
                path.display()
            );
        }
    }
    fn id(&self, role: &str) -> StableId {
        StableId(
            self.created.roles[role]
                .parse::<ulid::Ulid>()
                .unwrap()
                .to_bytes(),
        )
    }
    fn position(&self, game: &Session, role: &str) -> [f64; 3] {
        game.world()
            .unwrap()
            .inspect(self.id(role))
            .unwrap()
            .transform
            .translation
    }
}
fn no_op() -> Result<CompiledScript, ScriptError> {
    CompiledScript::compile("exports.default={update(){}};")
}
fn forbidden_fallback() -> Result<CompiledScript, ScriptError> {
    panic!("authored starter invoked fallback")
}
fn tick(game: &mut Session, events: &[InputEvent]) {
    let mut pcm = vec![0.; game.next_frames().unwrap() * 2];
    game.tick(events, &mut pcm).unwrap();
}
fn key(code: KeyCode, down: bool) -> InputEvent {
    InputEvent::Key { code, down }
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-8, "{a} != {b}");
}

#[test]
fn empty_template_is_stationary_and_cli_catalogue_and_creation_are_real() {
    let _serial = SERIAL.lock().unwrap_or_else(|error| error.into_inner());
    let f = Fixture::new(Template::Empty);
    let mut game = f.start();
    let before = f.position(&game, "camera");
    for _ in 0..4 {
        tick(&mut game, &[]);
    }
    assert_eq!(f.position(&game, "camera"), before);
    f.unchanged();
    let result = Command::new(env!("CARGO_BIN_EXE_incant_headless"))
        .arg("templates")
        .output()
        .unwrap();
    assert!(result.status.success());
    let catalogue: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(catalogue.as_array().unwrap().len(), 5);
    let path = f._temp.path().canonicalize().unwrap().join("cli");
    let result = Command::new(env!("CARGO_BIN_EXE_incant_headless"))
        .args(["new", path.to_str().unwrap(), "--template", "empty"])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let receipt: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert!(
        Project::from_text(&fs::read_to_string(receipt["project"].as_str().unwrap()).unwrap())
            .is_ok()
    );
    let before = fs::read(path.join("project.incant.json")).unwrap();
    let refused = Command::new(env!("CARGO_BIN_EXE_incant_headless"))
        .args(["new", path.to_str().unwrap(), "--template", "first-person"])
        .output()
        .unwrap();
    assert!(!refused.status.success());
    assert_eq!(fs::read(path.join("project.incant.json")).unwrap(), before);
}

#[test]
fn spatial_starters_jump_collide_follow_and_look_without_mutating_authoring() {
    let _serial = SERIAL.lock().unwrap_or_else(|error| error.into_inner());
    for kind in [Template::ThirdPerson, Template::FirstPerson] {
        let f = Fixture::new(kind);
        let mut game = f.start();
        for _ in 0..60 {
            tick(&mut game, &[]);
        }
        let ground = f.position(&game, "player");
        assert!((0.75..0.95).contains(&ground[1]), "ground {ground:?}");
        tick(&mut game, &[key(KeyCode::Space, true)]);
        for _ in 0..20 {
            tick(&mut game, &[]);
        }
        assert!(
            f.position(&game, "player")[1] > ground[1] + 0.5,
            "jump did not leave ground"
        );
        tick(&mut game, &[key(KeyCode::Space, false)]);
        for _ in 0..100 {
            tick(&mut game, &[]);
        }
        tick(&mut game, &[key(KeyCode::KeyD, true)]);
        for _ in 0..180 {
            tick(&mut game, &[]);
        }
        let player = f.position(&game, "player");
        assert!((5.0..5.35).contains(&player[0]), "wall failed {player:?}");
        near(player[2], 0.);
        let camera = f.position(&game, "camera");
        near(camera[0], player[0]);
        if kind == Template::FirstPerson {
            near(camera[1], player[1] + 0.55);
            near(camera[2], player[2]);
        } else {
            near(
                ((camera[1] - player[1] - 0.55).powi(2) + (camera[2] - player[2]).powi(2)).sqrt(),
                6.,
            );
        }
        tick(
            &mut game,
            &[key(KeyCode::KeyD, false), key(KeyCode::ArrowRight, true)],
        );
        for _ in 0..30 {
            tick(&mut game, &[]);
        }
        let camera = game.world().unwrap().inspect(f.id("camera")).unwrap();
        assert!(
            camera.transform.rotation[1].abs() > 0.2,
            "look did not rotate camera"
        );
        f.unchanged();
    }
}

#[test]
fn planar_starters_use_solid_tiles_animation_and_following_camera() {
    let _serial = SERIAL.lock().unwrap_or_else(|error| error.into_inner());
    for kind in [Template::Platformer2d, Template::TopDown] {
        let f = Fixture::new(kind);
        let mut game = f.start();
        for _ in 0..60 {
            tick(&mut game, &[]);
        }
        let ground = f.position(&game, "player");
        if kind == Template::Platformer2d {
            assert!((1.4..1.5).contains(&ground[1]), "floor failed {ground:?}");
            tick(&mut game, &[key(KeyCode::Space, true)]);
            for _ in 0..20 {
                tick(&mut game, &[]);
            }
            assert!(
                f.position(&game, "player")[1] > ground[1] + 0.5,
                "planar jump failed"
            );
            tick(&mut game, &[key(KeyCode::Space, false)]);
            for _ in 0..100 {
                tick(&mut game, &[]);
            }
        } else {
            near(ground[1], 2.);
        }
        tick(&mut game, &[key(KeyCode::KeyD, true)]);
        let mut frames = BTreeSet::new();
        for _ in 0..260 {
            tick(&mut game, &[]);
            let state = game
                .world()
                .unwrap()
                .flipbook_state(f.id("player"))
                .unwrap();
            assert!(state.playing());
            frames.insert(state.frame());
        }
        assert_eq!(frames.len(), 3, "native animation did not visit all frames");
        let player = f.position(&game, "player");
        let wall = if kind == Template::Platformer2d {
            17.6
        } else {
            9.6
        };
        assert!(
            (player[0] - wall).abs() < 0.1,
            "tile collision failed {player:?}"
        );
        let camera = f.position(&game, "camera");
        near(camera[0], player[0]);
        near(
            camera[1],
            player[1]
                + if kind == Template::Platformer2d {
                    1.5
                } else {
                    0.
                },
        );
        tick(&mut game, &[key(KeyCode::KeyD, false)]);
        assert!(
            !game
                .world()
                .unwrap()
                .flipbook_state(f.id("player"))
                .unwrap()
                .playing()
        );
        if kind == Template::TopDown {
            let before = f.position(&game, "player");
            tick(&mut game, &[key(KeyCode::KeyW, true)]);
            for _ in 0..30 {
                tick(&mut game, &[]);
            }
            let after = f.position(&game, "player");
            assert!(
                after[1] > before[1] + 1.5,
                "top-down second axis did not move"
            );
            near(f.position(&game, "camera")[1], after[1]);
            tick(&mut game, &[key(KeyCode::KeyW, false)]);
        }
        f.unchanged();
    }
}
