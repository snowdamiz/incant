#[path = "../../incant_assets/tests/support/audio.rs"]
mod wave;
use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::{AudioBus, AudioSource, Entity, Project, Scene};
use incant_import::{ImportRequest, ImportSnapshot};
use serde_json::{Value, json};
use std::{
    fs,
    path::Path,
    process::{Command as Process, Output},
};

fn run(root: &Path, args: &[&str]) -> Output {
    Process::new(env!("CARGO_BIN_EXE_incant_headless"))
        .current_dir(root)
        .args(args)
        .output()
        .unwrap()
}
fn success(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn fixture(root: &Path) {
    let mut project = Project::empty("Audio game");
    let scene = Scene::new("Main");
    let sid = scene.id.clone();
    project.scenes.insert(sid.clone(), scene);
    let mut commands = CommandBus::new(project).unwrap();
    fs::write(
        root.join("tone.wav"),
        wave::wav(1, 48000, &vec![[8000; 2]; 48000]),
    )
    .unwrap();
    let imported = ImportSnapshot::capture(&commands)
        .prepare(
            root,
            None,
            &[ImportRequest {
                source: "tone.wav".into(),
                texture_usage: None,
            }],
        )
        .unwrap()
        .commit(&mut commands, Actor::import("test"), "Import audio")
        .unwrap();
    let mut bus = Entity::new("SFX");
    bus.components.insert(
        "AudioBus".into(),
        json!(AudioBus {
            parent: None,
            gain_db: 0.
        }),
    );
    let mut sound = Entity::new("Tone");
    sound.components.insert(
        "AudioSource".into(),
        json!(AudioSource {
            clip: imported.imports[0].asset.id.clone(),
            bus: Some(bus.id.clone()),
            gain_db: 0.,
            pan: 1.,
            rate: 1.,
            start_seconds: 0.,
            playing: true,
            looping: true,
            streaming: true,
            spatial: None
        }),
    );
    commands
        .execute(
            vec![
                Command::CreateEntity {
                    scene_id: sid.clone(),
                    entity: bus,
                },
                Command::CreateEntity {
                    scene_id: sid,
                    entity: sound,
                },
            ],
            Actor::import("test"),
            "Audio graph",
            None,
        )
        .unwrap();
    fs::write(
        root.join("game.incant.json"),
        commands.project().canonical_text().unwrap(),
    )
    .unwrap();
    fs::write(
        root.join("game.incant.journal.jsonl"),
        "do not open author journal",
    )
    .unwrap();
    // A packaged runtime must not depend on the import source.
    fs::remove_file(root.join("tone.wav")).unwrap();
}
fn samples(path: &Path) -> Vec<f32> {
    let bytes = fs::read(path).unwrap();
    assert_eq!(&bytes[..4], b"RIFF");
    assert_eq!(
        u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize + 8,
        bytes.len()
    );
    assert_eq!(&bytes[48..52], b"data");
    bytes[56..]
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| f32::from_le_bytes(*b))
        .collect()
}

#[test]
fn scripts_mix_pause_and_attenuate_cached_audio_through_the_shared_command_bus() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fixture(root);
    let before = fs::read(root.join("game.incant.json")).unwrap();
    fs::write(root.join("behavior.js"), r#"exports.default={initialState:{t:0},update(api,dt,s){
        s.t++;
        if(s.t===21||s.t===31)for(const e of api.query('AudioSource'))
            api.command({op:'set_component',scene_id:e.scene_id,entity_id:e.id,component:'AudioSource',value:{...e.components.AudioSource,playing:s.t===31}});
        if(s.t===41)for(const e of api.query('AudioBus'))
            api.command({op:'set_component',scene_id:e.scene_id,entity_id:e.id,component:'AudioBus',value:{...e.components.AudioBus,gain_db:-6}});
    }};"#).unwrap();
    let a = success(run(
        root,
        &[
            "play",
            "game.incant.json",
            "--ticks",
            "60",
            "--compiled-script",
            "behavior.js",
            "--audio-output",
            "mix.wav",
        ],
    ));
    let b = success(run(
        root,
        &[
            "play",
            "game.incant.json",
            "--ticks",
            "60",
            "--compiled-script",
            "behavior.js",
            "--audio-output",
            "repeat.wav",
        ],
    ));
    assert_eq!(a["audio"]["frames"], 48000);
    assert_eq!(a["audio"]["pcm_sha256"], b["audio"]["pcm_sha256"]);
    assert!(a["adapter"].is_null());
    assert_eq!(a["frames"], json!([]));
    assert_eq!(a["script_commands"], 3);
    let pcm = samples(&root.join("mix.wav"));
    assert_eq!(pcm.len(), 96000);
    assert!(pcm.as_chunks::<2>().0.iter().all(|f| f[0] == 0.));
    let at = |frame: usize| pcm[frame * 2 + 1];
    let full = at(1000);
    assert!((full - 8000_f32 / 32768. * 2_f32.sqrt()).abs() < 1e-6);
    assert!(pcm[17000 * 2..23000 * 2].iter().all(|s| *s == 0.));
    assert!((at(26000) - full).abs() < 1e-6);
    assert!((at(35000) / full - 10_f32.powf(-6. / 20.)).abs() < 1e-5);
    assert_eq!(fs::read(root.join("game.incant.json")).unwrap(), before);
    assert_eq!(
        fs::read_to_string(root.join("game.incant.journal.jsonl")).unwrap(),
        "do not open author journal"
    );
    // The exported float WAV is itself accepted by the import decoder.
    let cooked =
        incant_assets::cook_audio(root, Path::new("mix.wav"), &root.join("roundtrip")).unwrap();
    assert_eq!(cooked.metadata.frames, 48000);
    assert_eq!(cooked.read_all(48000).unwrap()[35000][1], at(35000));
}

#[test]
fn invalid_output_conflicts_and_runtime_failures_publish_no_audio_or_game_save() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fixture(root);
    let path = root.join("game.incant.json");
    let mut project: Project = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    project.settings.tick_rate = 30;
    fs::write(path, project.canonical_text().unwrap()).unwrap();
    for args in [
        vec!["--audio-output", "same.wav", "--log-output", "./same.wav"],
        vec!["--audio-output", "same.wav", "--save-output", "same.wav"],
        vec![
            "--audio-output",
            "huge.wav",
            "--ticks",
            "10000",
            "--audio-rate",
            "192000",
        ],
    ] {
        let mut all = vec!["play", "game.incant.json"];
        all.extend(args);
        assert!(!run(root, &all).status.success());
        assert!(!root.join("same.wav").exists());
        assert!(!root.join("huge.wav").exists());
    }
    fs::write(
        root.join("broken.js"),
        "exports.default={update(){throw Error('failed')}}",
    )
    .unwrap();
    assert!(
        !run(
            root,
            &[
                "play",
                "game.incant.json",
                "--compiled-script",
                "broken.js",
                "--audio-output",
                "failed.wav",
                "--save-output",
                "failed.save"
            ]
        )
        .status
        .success()
    );
    assert!(!root.join("failed.wav").exists());
    assert!(!root.join("failed.save").exists());
    fs::write(root.join("existing.wav"), "keep this").unwrap();
    assert!(
        !run(
            root,
            &["play", "game.incant.json", "--audio-output", "existing.wav"]
        )
        .status
        .success()
    );
    assert_eq!(
        fs::read_to_string(root.join("existing.wav")).unwrap(),
        "keep this"
    );
}

#[test]
fn nondivisible_sample_counts_and_failed_assertions_keep_completed_audio() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fixture(root);
    let report = success(run(
        root,
        &[
            "play",
            "game.incant.json",
            "--ticks",
            "7",
            "--audio-output",
            "odd.wav",
            "--audio-rate",
            "8001",
        ],
    ));
    assert_eq!(report["audio"]["frames"], 933);
    assert_eq!(samples(&root.join("odd.wav")).len(), 1866);
    fs::write(root.join("assert.json"), r#"{"format":"incant-play-assertions","version":1,"checks":[{"name":"wrong","tick":1,"path":"/state/tick","expect":{"type":"equals","value":99}}]}"#).unwrap();
    let output = run(
        root,
        &[
            "play",
            "game.incant.json",
            "--ticks",
            "1",
            "--audio-output",
            "assert.wav",
            "--assertions",
            "assert.json",
            "--save-output",
            "assert.save",
        ],
    );
    assert!(!output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["completed"], true);
    assert_eq!(report["passed"], false);
    assert!(root.join("assert.wav").exists());
    assert!(!root.join("assert.save").exists());
}
