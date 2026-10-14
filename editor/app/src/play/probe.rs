//! Opt-in numerical acceptance over the actual editor worker/window. This is not
//! a rendered-pixel or physical-device performance review. The lifecycle mode
//! makes no project mutations; the texture mode uses real editor command paths.
pub(super) mod textures;
use super::{Action, StartOptions, Status, execute};
use crate::{Editor, project};
use serde_json::{Value, json};
use std::{
    ffi::{OsStr, OsString},
    fs::OpenOptions,
    io::Write,
    path::PathBuf,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};
use tauri::Manager;
pub struct Options {
    output: PathBuf,
    entity: String,
    textures: bool,
}
impl Options {
    fn texture_mode(flag: &OsStr) -> Option<bool> {
        match flag.to_str() {
            Some("--play-probe") => Some(false),
            Some("--texture-probe") => Some(true),
            _ => None,
        }
    }
    pub fn accepts_mode(flag: &OsStr) -> bool {
        Self::texture_mode(flag).is_some()
    }
    pub fn from_args() -> Option<Self> {
        Self::from_probe_args(std::env::args_os().skip(2))
    }
    fn from_probe_args(mut args: impl Iterator<Item = OsString>) -> Option<Self> {
        match args.next() {
            None => None,
            Some(flag) if Self::accepts_mode(&flag) => Some(Self {
                textures: Self::texture_mode(&flag).expect("accepted probe mode"),
                output: args.next().expect("probe output path").into(),
                entity: args
                    .next()
                    .expect("probe entity ULID")
                    .into_string()
                    .expect("UTF-8 entity"),
            }),
            _ => panic!(
                "expected --play-probe or --texture-probe OUTPUT ENTITY after the project path"
            ),
        }
    }
}
fn require(value: bool, message: &str) -> Result<(), String> {
    value.then_some(()).ok_or_else(|| message.to_string())
}
fn project(editor: &Editor) -> Result<(Value, u64, usize), String> {
    let state = editor.bus.lock().map_err(|_| "project lock")?;
    let project::LoadState::Ready(bus) = &*state else {
        return Err("project not ready".into());
    };
    Ok((
        serde_json::to_value(bus.project()).map_err(|e| e.to_string())?,
        bus.revision(),
        bus.history().len(),
    ))
}
fn wait(editor: &Editor, condition: impl Fn(&Status) -> bool) -> Result<Status, String> {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let state = editor.play.snapshot()?;
        if state.phase == "error" {
            return Err(state.error.unwrap_or("native Play failed".into()));
        }
        if condition(&state) {
            return Ok(state);
        }
        require(Instant::now() < deadline, "native Play probe deadline")?;
        thread::sleep(Duration::from_millis(10));
    }
}
fn run(app: &tauri::AppHandle, editor: &Editor, options: &Options) -> Result<Value, String> {
    let window = app.get_window("main").ok_or("main window absent")?;
    window.set_focus().map_err(|e| e.to_string())?;
    let deadline = Instant::now() + Duration::from_secs(30);
    while project(editor).is_err()
        || editor
            .viewport
            .lock()
            .map_err(|_| "viewport lock")?
            .is_none()
    {
        require(
            Instant::now() < deadline,
            "project/viewport startup deadline",
        )?;
        thread::sleep(Duration::from_millis(20));
    }
    let before = project(editor)?;
    let start = |generation| {
        execute(
            editor,
            Action::Start {
                generation,
                expected_revision: before.1,
                options: StartOptions {
                    camera: Some("00000000000000000000000003".into()),
                    ..Default::default()
                },
            },
        )
    };
    let first = start(editor.play.snapshot()?.generation)?;
    require(first["tick"] == 0, "new world did not start at zero")?;
    let running = wait(editor, |s| s.tick >= 5 && s.presented_frames >= 5)?;
    require(
        running.spatial,
        "probe requires the real spatial GPU consumer",
    )?;
    execute(
        editor,
        Action::Pause {
            generation: running.generation,
        },
    )?;
    let paused = editor.play.snapshot()?;
    let inspect = |generation| {
        execute(
            editor,
            Action::Inspect {
                generation,
                entity: options.entity.clone(),
            },
        )
    };
    let at_pause = inspect(paused.generation)?;
    thread::sleep(Duration::from_millis(200));
    let redraw = editor.play.snapshot()?;
    require(
        redraw.tick == paused.tick && redraw.presented_frames > paused.presented_frames,
        "pause did not redraw without ticking",
    )?;
    require(
        inspect(paused.generation)? == at_pause,
        "paused world moved",
    )?;
    execute(
        editor,
        Action::Step {
            generation: paused.generation,
        },
    )?;
    let stepped = editor.play.snapshot()?;
    let after_step = inspect(paused.generation)?;
    require(
        stepped.phase == "paused" && stepped.tick == paused.tick + 1,
        "step did not accept exactly one tick",
    )?;
    let x = |v: &Value| {
        v["transform"]["translation"][0]
            .as_f64()
            .ok_or("missing native transform")
    };
    require(
        (x(&after_step)? - x(&at_pause)? - 2.).abs() < 1e-9,
        "authored script did not move two units in one step",
    )?;
    let camera = execute(
        editor,
        Action::Inspect {
            generation: paused.generation,
            entity: "00000000000000000000000003".into(),
        },
    )?;
    require(
        camera["transform"]["translation"] == json!([0.0, 0.0, 0.0]),
        "movement escaped its attached entity and changed the camera",
    )?;
    execute(
        editor,
        Action::Resume {
            generation: paused.generation,
        },
    )?;
    wait(editor, |s| s.tick > stepped.tick)?;
    window.minimize().map_err(|e| e.to_string())?;
    let hidden = wait(editor, |s| s.suspended)?;
    thread::sleep(Duration::from_millis(200));
    require(
        editor.play.snapshot()?.tick == hidden.tick,
        "minimized world kept ticking",
    )?;
    window.unminimize().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    wait(editor, |s| s.tick > hidden.tick && !s.suspended)?;
    execute(
        editor,
        Action::Stop {
            generation: paused.generation,
        },
    )?;
    let stopped = editor.play.snapshot()?;
    require(
        stopped.phase == "stopped" && stopped.tick == 0,
        "stop did not discard telemetry",
    )?;
    require(
        inspect(stopped.generation).is_err(),
        "stop retained a runtime world",
    )?;
    require(
        execute(
            editor,
            Action::Step {
                generation: paused.generation,
            },
        )
        .is_err(),
        "stale generation was accepted",
    )?;
    let restart = start(stopped.generation)?;
    require(restart["tick"] == 0, "restart inherited old runtime time")?;
    let fresh = wait(editor, |s| s.tick >= 2)?;
    execute(
        editor,
        Action::Pause {
            generation: fresh.generation,
        },
    )?;
    let fresh_entity = inspect(fresh.generation)?;
    require(
        (x(&fresh_entity)? - 2. * fresh_entity["tick"].as_u64().ok_or("missing tick")? as f64)
            .abs()
            < 1e-9,
        "restart inherited runtime changes",
    )?;
    execute(
        editor,
        Action::Stop {
            generation: fresh.generation,
        },
    )?;
    require(
        project(editor)? == before,
        "Play modified project/revision/history",
    )?;
    Ok(
        json!({"passed":true,"platform":std::env::consts::OS,"running":running,"paused":paused,"redraw":redraw,"stepped":stepped,"minimized":hidden,"paused_entity":at_pause,"stepped_entity":after_step,"restarted_entity":fresh_entity,"project_revision_history_unchanged":true,"limits":["Numerical native window proof on this host, not pixel review or physical-phone measurement","Device audio commands apply at callback boundaries, not sample-exact stepping"]}),
    )
}
pub fn start(app: tauri::AppHandle, editor: Arc<Editor>, options: Options) {
    thread::spawn(move || {
        let result = (|| -> Result<(), String> {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&options.output)
                .map_err(|e| e.to_string())?;
            let result = if options.textures {
                textures::run(&app, &editor, &options)
            } else {
                run(&app, &editor, &options)
            };
            let report = match &result {
                Ok(value) => value.clone(),
                Err(error) => json!({"passed":false,"error":error}),
            };
            file.write_all(
                serde_json::to_string_pretty(&report)
                    .map_err(|e| e.to_string())?
                    .as_bytes(),
            )
            .map_err(|e| e.to_string())?;
            file.sync_all().map_err(|e| e.to_string())?;
            result.map(|_| ())
        })();
        if let Ok(status) = editor.play.snapshot() {
            let _ = execute(
                &editor,
                Action::Stop {
                    generation: status.generation,
                },
            );
        }
        app.exit(if result.is_ok() { 0 } else { 1 });
    });
}

#[cfg(test)]
mod options_tests {
    use super::*;

    #[test]
    fn outer_dispatch_and_parser_accept_both_probe_modes() {
        for (flag, textures) in [("--play-probe", false), ("--texture-probe", true)] {
            assert!(Options::accepts_mode(OsStr::new(flag)));
            let options = Options::from_probe_args(
                [flag, "receipt.json", "00000000000000000000000002"]
                    .map(OsString::from)
                    .into_iter(),
            )
            .expect("recognized probe mode");
            assert_eq!(options.textures, textures);
            assert_eq!(options.output, PathBuf::from("receipt.json"));
            assert_eq!(options.entity, "00000000000000000000000002");
        }
    }

    #[test]
    fn unknown_modes_are_not_dispatched_as_probes() {
        for flag in ["", "--unknown-probe", "--texture-probe-extra"] {
            assert!(!Options::accepts_mode(OsStr::new(flag)));
        }
        assert!(Options::from_probe_args(std::iter::empty()).is_none());
    }

    #[test]
    #[should_panic(expected = "expected --play-probe or --texture-probe")]
    fn explicit_probe_parser_refuses_unknown_mode() {
        Options::from_probe_args([OsString::from("--unknown-probe")].into_iter());
    }
}
