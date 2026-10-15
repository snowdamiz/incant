//! Trusted, ignored CPU native-host proxy. No Capture, device, window or GPU.
//! Timing requires a separately frozen six-process plan and a quiet-host grant.
use incant_agent_native::{Session, StartOptions};
use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::{PhysicsMode, Project};
use incant_input::InputReplay;
use incant_observe::Plan;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::Instant,
};

const TICKS: u64 = 2156;
const RATE: u32 = 48_000;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Inputs {
    project: PathBuf,
    recording: PathBuf,
    observations: PathBuf,
    output: PathBuf,
}
fn read(path: &Path, cap: usize) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take(cap as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > cap {
        return Err("fixture input exceeds bound".into());
    }
    Ok(bytes)
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// The derived document is edited only through a reversible command transaction.
fn derivative(original: Project) -> Result<(Project, Value)> {
    if original.settings.physics_mode != PhysicsMode::Fast {
        return Err("expected immutable fast Core input".into());
    }
    let before = original.canonical_text()?;
    let mut bus = CommandBus::new(original.clone())?;
    bus.execute(
        vec![Command::SetPhysicsMode {
            mode: PhysicsMode::Deterministic,
        }],
        Actor::user("inactive-profiler-fixture"),
        "Derive deterministic Core without changing original files",
        Some(0),
    )?;
    let after = bus.project().canonical_text()?;
    let transaction = &bus.history()[0];
    let provenance = json!({
        "id":transaction.id,"actor":transaction.actor,"commands":transaction.commands,
        "before_sha256":hash(before.as_bytes()),"after_sha256":hash(after.as_bytes()),
        "before_canonical_text":before,"after_canonical_text":after
    });
    // Independent equality check: exactly one authored settings field changed.
    let mut expected = original.clone();
    expected.settings.physics_mode = PhysicsMode::Deterministic;
    if bus.project() != &expected {
        return Err("derivative changed more than physics_mode".into());
    }
    bus.undo()?;
    if bus.project() != &original {
        return Err("derivative undo did not restore exact original".into());
    }
    bus.redo()?;
    if bus.project().canonical_text()? != after || bus.revision() != 3 {
        return Err("derivative redo did not restore exact accepted document".into());
    }
    Ok((bus.project().clone(), provenance))
}
fn duration(start: Instant, end: Instant) -> Option<u64> {
    u64::try_from(end.checked_duration_since(start)?.as_nanos()).ok()
}
fn timing(rows: &[Option<u64>], expected: usize) -> Value {
    let mut sorted: Vec<_> = rows
        .iter()
        .copied()
        .collect::<Option<_>>()
        .unwrap_or_default();
    let total = sorted.iter().try_fold(0_u64, |sum, v| sum.checked_add(*v));
    let complete = rows.len() == expected && sorted.len() == expected && total.is_some();
    sorted.sort_unstable();
    let p95 = complete.then(|| sorted[(expected * 95).div_ceil(100) - 1]);
    json!({"expected_frames":expected,"recorded_frames":rows.len(),
        "clock_invalid_frames":rows.iter().filter(|v|v.is_none()).count(),
        "complete":complete,"p95_wall_ns":p95,"total_wall_ns":if complete {total} else {None},
        "raw_wall_ns":rows})
}
fn fallback() -> std::result::Result<incant_script::CompiledScript, incant_script::ScriptError> {
    panic!("Core must use its authored scripts, never a fixture fallback")
}
fn run(inputs: Inputs) -> Result<Value> {
    let mut destination = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&inputs.output)?;
    let original = read(&inputs.project, 64 * 1024 * 1024)?;
    let document = Project::from_text(std::str::from_utf8(&original)?)?;
    if document.settings.tick_rate != 60 {
        return Err("fixed Core tick rate differs".into());
    }
    let (document, derivative) = derivative(document)?;
    if serde_json::to_vec(&derivative)?.len() > 24 * 1024 * 1024 {
        return Err("canonical derivative proof exceeds startup bound".into());
    }
    let recording = read(&inputs.recording, incant_input::MAX_RECORDING_BYTES)?;
    let replay = InputReplay::from_text(std::str::from_utf8(&recording)?, 0, 60)?;
    if replay.end_tick() != TICKS {
        return Err("fixed Core replay length differs".into());
    }
    let observations = read(&inputs.observations, incant_observe::MAX_PLAN_BYTES)?;
    let mut plan = Plan::parse(&observations, TICKS)?;
    let options = StartOptions {
        scene: Some("01CSSN00000000000000000001".into()),
        camera: Some("01CSEN00000000000000000003".into()),
        ui_viewport: Some([640., 480.]),
        ..StartOptions::default()
    };
    let root = inputs.project.parent().ok_or("project requires a parent")?;
    let mut game = Session::new(&document, root, &options, RATE, fallback)?;
    if !game.authored_scripts || !game.is_spatial() {
        return Err("expected actual authored Core CPU host".into());
    }
    game.validate_ui_recording(replay.has_ui())?;
    drop(document);
    plan.observe(game.world()?, game.input()?)?;
    let mut samples = vec![0.; (RATE as usize / 60 + 1) * 2];
    let mut pcm = Sha256::new();
    let mut pcm_bytes = Vec::with_capacity(samples.len() * 4);
    let mut rows = Vec::with_capacity(TICKS as usize);
    let mut terminal_error = None;
    for tick in 1..=TICKS {
        let step = (|| -> Result<()> {
            let frames = game.next_frames()?;
            let output = &mut samples[..frames * 2];
            let events = replay.events_at(tick)?;
            let ui = replay.ui_at(tick)?;
            let viewport = replay.viewport_at(tick)?;
            // The only timed region. No process or session Capture is activated.
            let start = Instant::now();
            let result = game.tick_recorded(events, ui, viewport, output);
            let end = Instant::now();
            rows.push(duration(start, end));
            result?;
            pcm_bytes.clear();
            pcm_bytes.extend(output.iter().flat_map(|v| v.to_le_bytes()));
            pcm.update(&pcm_bytes);
            plan.observe(game.world()?, game.input()?)?;
            Ok(())
        })();
        if let Err(error) = step {
            terminal_error = Some(error.to_string());
            break;
        }
    }
    // Preserve raw durations even when the failed native world cannot be read.
    let (tick, entities, observation) = match game.world() {
        Ok(world) => {
            let observation = match plan.finish(world) {
                Ok(value) => serde_json::to_value(value)?,
                Err(error) => {
                    terminal_error.get_or_insert_with(|| error.to_string());
                    Value::Null
                }
            };
            (Some(world.tick()), Some(world.entity_count()), observation)
        }
        Err(error) => {
            terminal_error.get_or_insert_with(|| error.to_string());
            (None, None, Value::Null)
        }
    };
    let audio_frames = game
        .frames()
        .map_err(|error| {
            terminal_error.get_or_insert_with(|| error.to_string());
        })
        .ok();
    let logs = match game.logs() {
        Ok(logs) => serde_json::to_value(logs)?,
        Err(error) => {
            terminal_error.get_or_insert_with(|| error.to_string());
            Value::Null
        }
    };
    let timing = timing(&rows, TICKS as usize);
    let report = json!({
        "format":"incant-inactive-core-proxy","version":1,
        "agent_profiler_compiled":cfg!(feature="agent-profiler"),
        "capture_activated":false,"physics_mode":"deterministic",
        "build_profile":"external-cargo-artifact-binding",
        "ticks":tick,"tick_rate":60,"audio_frames":audio_frames,
        "pcm_sha256":format!("{:x}",pcm.finalize()),"script_source_sha256":game.script_source_sha256,
        "scene":game.scene,"entity_count":entities,
        "logs":logs,"observations":observation,
        "project_sha256":hash(&original),"recording_sha256":hash(&recording),
        "observations_sha256":hash(&observations),"derivative":derivative,
        "timing":timing,"terminal_error":terminal_error,
        "gpu_submitted":false,"window_launched":false,"eligible_for_shipping_gate":false,
        "limitations":["CPU native-host proxy, not GUI or shipping performance",
            "Inclusive tick wall time includes shared timer overhead and scheduler delays",
            "Deterministic derivative is not the historical fast4 numerical identity"]
    });
    let bytes = serde_json::to_vec(&report)?;
    if bytes.len() > 32 * 1024 * 1024 {
        return Err("fixture report exceeds bound".into());
    }
    destination.write_all(&bytes)?;
    if report["terminal_error"] != Value::Null
        || report["ticks"] != TICKS
        || report["timing"]["complete"] != true
        || report["observations"]["passed"] != true
    {
        return Err("full deterministic Core route/clock contract failed; report retained".into());
    }
    Ok(report)
}

#[test]
#[ignore = "requires frozen actual Core inputs and separately granted quiet CPU protocol"]
fn inactive_core_complete_route() {
    let input = std::env::var_os("INCANT_INACTIVE_CORE_INPUTS").expect("trusted fixed input plan");
    let input = read(Path::new(&input), 16 * 1024).unwrap();
    let input: Inputs = serde_json::from_slice(&input).unwrap();
    run(input).unwrap();
}
#[test]
fn deterministic_derivative_is_reversible_and_changes_only_one_setting() {
    let mut original = Project::empty("Derivative control");
    original.settings.physics_mode = PhysicsMode::Fast;
    let (derived, receipt) = derivative(original.clone()).unwrap();
    assert_eq!(original.settings.physics_mode, PhysicsMode::Fast);
    assert_eq!(derived.settings.physics_mode, PhysicsMode::Deterministic);
    assert_eq!(
        receipt["commands"],
        json!([{"op":"set_physics_mode","mode":"deterministic"}])
    );
    assert_ne!(receipt["before_sha256"], receipt["after_sha256"]);
    assert!(derivative(derived).is_err());
}
#[test]
fn missing_invalid_or_overflowed_clock_never_yields_complete_timing() {
    let start = Instant::now();
    assert_eq!(
        duration(start + std::time::Duration::from_nanos(1), start),
        None
    );
    for rows in [
        vec![],
        vec![Some(1)],
        vec![Some(1), None],
        vec![Some(u64::MAX), Some(1)],
    ] {
        let report = timing(&rows, 2);
        assert_eq!(report["complete"], false);
        assert_eq!(report["p95_wall_ns"], Value::Null);
        assert_eq!(report["total_wall_ns"], Value::Null);
    }
    let report = timing(&[Some(2), Some(1)], 2);
    assert_eq!(report["complete"], true);
    assert_eq!(report["p95_wall_ns"], 2);
    assert_eq!(report["total_wall_ns"], 3);
}
