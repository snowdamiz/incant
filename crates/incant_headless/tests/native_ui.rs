//! Actual CLI/RPC UI through the same authored, cooked and retained native host.
use incant_cmd::{Actor, Command, CommandBus, SourceVersion};
use incant_doc::{Entity, Project, Scene, ScriptSource};
use incant_types::{TextWrap, UiKind, UiNode, UiText};
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command as Process, Stdio},
};
const FONT: &[u8] = include_bytes!("../../incant_text/tests/fonts/NotoSans-Regular.ttf");
fn id(n: u128) -> String {
    ulid::Ulid::from(n).to_string()
}
fn fixture(root: &Path) -> String {
    let mut p = Project::empty("Native UI");
    p.settings.tick_rate = 10;
    let s = Scene::new("Main");
    let sid = s.id.clone();
    p.scenes.insert(sid.clone(), s);
    let mut bus =
        CommandBus::persistent_with_sources(root.join("authoring.journal"), p, root).unwrap();
    fs::write(root.join("font.ttf"), FONT).unwrap();
    let imported = incant_import::ImportSnapshot::capture(&bus)
        .prepare(
            root,
            None,
            &[incant_import::ImportRequest {
                source: "font.ttf".into(),
                texture_usage: None,
            }],
        )
        .unwrap()
        .commit(&mut bus, Actor::import("test"), "Font")
        .unwrap();
    let text = UiText {
        content: "Name".into(),
        fonts: vec![imported.imports[0].asset.id.clone()],
        font_size: 16.,
        line_height: 24.,
        wrap: TextWrap::WordOrGlyph,
    };
    let mut commands = vec![];
    for n in 1..=4 {
        let mut e = Entity::new("Fixture");
        e.id = id(n);
        if n == 1 {
            e.components.insert(
                "Camera".into(),
                json!({"fov_degrees":60,"near":0.1,"far":100}),
            );
            e.components
                .insert("Script".into(), json!({"source":id(5),"props":{}}));
        } else {
            e.parent = (n > 2).then(|| id(2));
            e.components.insert(
                "UiNode".into(),
                json!(UiNode {
                    kind: match n {
                        2 => UiKind::Root,
                        3 => UiKind::Button { text: text.clone() },
                        _ => UiKind::TextInput { text: text.clone() },
                    },
                    style: Default::default(),
                    order: n as i32,
                    disabled: false
                }),
            );
        }
        commands.push(Command::CreateEntity {
            scene_id: sid.clone(),
            entity: e,
        });
    }
    commands.push(Command::UpsertScript {
        script: ScriptSource {
            id: id(5),
            name: "Observe".into(),
            path: "behavior.ts".into(),
        },
    });
    commands.push(Command::WriteScript {
        script_id: id(5),
        expected_source: SourceVersion::of(None),
        source: format!(
            r#"import type {{ NativeBehavior }} from '@incant/runtime/native';
export default {{update({{input,log,commands}}){{
if(input.ui.events.some(e=>'value' in e&&e.value.endsWith('TRAP')))throw Error('fixture trap');
log(JSON.stringify(input.ui));
if(input.ui.events.some(e=>e.kind==='submitted'))commands.setUiText('{button}','Done');
if(input.keyboard.held.includes('Enter')&&input.ui.text_input)throw Error('key leaked');
}}}} satisfies NativeBehavior;"#,
            button = id(3)
        ),
    });
    bus.execute(
        commands,
        Actor::user("test"),
        "Native UI",
        Some(bus.revision()),
    )
    .unwrap();
    fs::write(
        root.join("project.json"),
        bus.project().canonical_text().unwrap(),
    )
    .unwrap();
    fs::remove_file(root.join("font.ttf")).unwrap();
    sid
}
fn recording() -> Value {
    json!({"format":"incant-input","version":2,"tick_rate":10,"start_tick":0,"ticks":6,"frames":[
    {"tick":1,"events":[],"ui":[{"type":"focus","entity_id":id(4)}]},
    {"tick":2,"events":[],"ui":[{"type":"preedit","entity_id":id(4),"text":"é","cursor":[0,2]}],"viewport":[240,180]},
    {"tick":3,"events":[],"ui":[{"type":"cancel","entity_id":id(4)}]},
    {"tick":4,"events":[],"ui":[{"type":"commit","entity_id":id(4),"text":"ok"}]},
    {"tick":5,"events":[{"type":"key","code":"Enter","down":true}]},
    {"tick":6,"events":[{"type":"key","code":"Enter","down":false}]}
    ]})
}
fn cli(root: &Path, extra: &[&str]) -> std::process::Output {
    Process::new(env!("CARGO_BIN_EXE_incant_headless"))
        .current_dir(root)
        .args(["native-play", "project.json", "--ticks", "6"])
        .args(extra)
        .output()
        .unwrap()
}
fn rpc(root: &Path, calls: Vec<Value>) -> Vec<Value> {
    let mut p = Process::new(env!("CARGO_BIN_EXE_incant_headless"))
        .current_dir(root)
        .args(["rpc", "project.json"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = p.stdin.take().unwrap();
    for (n, mut c) in calls.into_iter().enumerate() {
        c["id"] = json!(n);
        writeln!(input, "{c}").unwrap();
    }
    drop(input);
    let out = p.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
fn ok(x: &Value) -> &Value {
    assert!(x.get("error").is_none(), "{x}");
    &x["result"]
}
#[test]
fn cli_rpc_run_and_incremental_steps_match_recorded_focus_ime_resize_and_filtered_keys() {
    let t = tempfile::tempdir().unwrap();
    let root = t.path();
    fixture(root);
    let before = fs::read(root.join("project.json")).unwrap();
    let source = fs::read(root.join("behavior.ts")).unwrap();
    let journal = fs::read(root.join("authoring.journal")).unwrap();
    let r = recording();
    fs::write(root.join("input.json"), r.to_string()).unwrap();
    let camera = id(1);
    let out = cli(
        root,
        &[
            "--camera",
            &camera,
            "--ui-viewport",
            "320x200",
            "--input-replay",
            "input.json",
            "--pcm-output",
            "a.pcm",
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let a: Value = serde_json::from_slice(&out.stdout).unwrap();
    let mut calls = vec![
        json!({"method":"play.run","params":{"seconds":0.6,"camera":camera,"ui_viewport":[320,200],"inputs_script":r}}),
        json!({"method":"play.stop","params":{}}),
        json!({"method":"play.start","params":{"camera":camera,"ui_viewport":[320,200]}}),
    ];
    for f in r["frames"].as_array().unwrap() {
        calls.push(json!({"method":"play.step","params":{"events":f["events"],"ui":f.get("ui").cloned().unwrap_or(json!([])),"viewport":f.get("viewport").cloned().unwrap_or(Value::Null),"observe":{"include_input":true,"include_logs":true}}}));
    }
    let b = rpc(root, calls);
    assert_eq!(ok(&b[0])["state"]["logs"], a["logs"]);
    assert_eq!(ok(b.last().unwrap())["logs"], a["logs"]);
    let entries = a["logs"]["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 6);
    let ui: Vec<Value> = entries
        .iter()
        .map(|e| serde_json::from_str(e["message"].as_str().unwrap()).unwrap())
        .collect();
    assert_eq!(ui[0]["focused"], id(4));
    assert_eq!(ui[3]["events"][0]["value"], "Nameok");
    assert_eq!(ui[4]["events"][0]["kind"], "submitted");
    assert!(ui[5]["events"].as_array().unwrap().is_empty());
    assert_eq!(a["cpu_render_input"], true);
    assert_eq!(a["gpu_submitted"], false);
    assert_eq!(
        fs::read(root.join("a.pcm")).unwrap(),
        vec![0; 48000 / 10 * 6 * 8]
    );
    assert_eq!(fs::read(root.join("project.json")).unwrap(), before);
    assert_eq!(fs::read(root.join("behavior.ts")).unwrap(), source);
    assert_eq!(fs::read(root.join("authoring.journal")).unwrap(), journal);
}
#[test]
fn refused_candidates_preserve_last_completed_ui_run_and_cli_creates_no_pcm() {
    let t = tempfile::tempdir().unwrap();
    let root = t.path();
    fixture(root);
    let camera = id(1);
    let good =
        json!({"seconds":0.6,"camera":camera,"ui_viewport":[320,200],"inputs_script":recording()});
    let mut wrong = recording();
    wrong["frames"][1]["ui"][0]["entity_id"] = json!(id(3));
    let mut trap = recording();
    trap["frames"][5]["ui"] = json!([{"type":"commit","entity_id":id(4),"text":"TRAP"}]);
    let mut calls = vec![
        json!({"method":"play.run","params":good}),
        json!({"method":"play.state_query","params":{"path":"/logs"}}),
    ];
    for rec in [wrong.clone(), trap] {
        let mut bad = good.clone();
        bad["inputs_script"] = rec;
        calls.push(json!({"method":"play.run","params":bad}));
        calls.push(json!({"method":"play.state_query","params":{"path":"/logs"}}));
    }
    let b = rpc(root, calls);
    ok(&b[0]);
    assert!(b[2].get("error").is_some());
    assert!(b[4].get("error").is_some());
    assert_eq!(ok(&b[1]), ok(&b[3]));
    assert_eq!(ok(&b[1]), ok(&b[5]));
    fs::write(root.join("wrong.json"), wrong.to_string()).unwrap();
    for opts in [
        vec!["--camera", &camera, "--input-replay", "wrong.json"],
        vec!["--ui-viewport", "320x200"],
        vec!["--camera", &camera, "--ui-viewport", "NaNx200"],
        vec![
            "--camera",
            &camera,
            "--ui-viewport",
            "320x200",
            "--input-replay",
            "wrong.json",
        ],
    ] {
        let mut opts = opts;
        opts.extend(["--pcm-output", "bad.pcm"]);
        let out = cli(root, &opts);
        assert!(!out.status.success());
        assert!(out.stdout.is_empty());
        assert!(!root.join("bad.pcm").exists());
    }
}
