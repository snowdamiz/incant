//! Real disposable game measurements; no device, network or performance gate.
#[cfg(feature = "agent-profiler")]
use incant_agent::ProjectHost;
use incant_agent::{AgentError, EngineHost, NoViewport, dispatch};
use incant_agent_native::Host;
use incant_cmd::{Actor, CommandBus};
use incant_doc::{Entity, Project, Scene, Transform};
use incant_script::{CompiledScript, ScriptError};
use serde_json::{Value, json};
use std::cell::Cell;
#[cfg(feature = "agent-profiler")]
use std::sync::atomic::{AtomicBool, Ordering};

thread_local! { static FAIL: Cell<bool> = const { Cell::new(false) }; }
fn fallback() -> Result<CompiledScript, ScriptError> {
    CompiledScript::compile(if FAIL.get() {
        "exports.default={update(){throw Error('candidate failure')}};"
    } else {
        "exports.default={update({chunks}){for(const c of chunks)c.transforms[0]+=1}};"
    })
}
fn fixture() -> (CommandBus, String) {
    let mut project = Project::empty("Profile owner");
    project.settings.tick_rate = 10;
    let mut scene = Scene::new("Main");
    let mut entity = Entity::new("Mover");
    entity
        .components
        .insert("Transform".into(), json!(Transform::default()));
    let id = entity.id.clone();
    scene.entities.insert(id.clone(), entity);
    project.scenes.insert(scene.id.clone(), scene);
    (CommandBus::new(project).unwrap(), id)
}
fn call(
    bus: &mut CommandBus,
    host: &mut dyn EngineHost,
    name: &str,
    value: Value,
) -> Result<Value, AgentError> {
    dispatch(bus, host, name, value, Actor::agent("test", "session"))
}

#[cfg(feature = "agent-profiler")]
#[test]
fn real_measurements_are_project_bound_read_only_and_publish_with_successful_game() {
    FAIL.set(false);
    let root = tempfile::tempdir().unwrap();
    let (mut bus, entity) = fixture();
    let authored = bus.project().canonical_text().unwrap();
    let host = Host::new(root.path().to_owned(), NoViewport, fallback);
    let mut host = ProjectHost::new(bus.project(), root.path(), host).unwrap();
    let process_capture = incant_profile::Capture::start().unwrap();
    assert!(host.profiler_available());
    assert!(call(&mut bus, &mut host, "profiler_read", json!({"frames":1})).is_err());
    let run = incant_profile::frame(|| {
        call(
            &mut bus,
            &mut host,
            "play_run",
            json!({"seconds":0.5,"profile":true}),
        )
    })
    .unwrap();
    let report = call(&mut bus, &mut host, "profiler_read", json!({"frames":3})).unwrap();
    assert_eq!(report["profile_id"], run["profile"]["id"]);
    assert_eq!(report["project_id"], bus.project().id);
    assert_eq!(report["measurements"]["selected_frames"], 3);
    assert_eq!(report["measurements"]["frames"][0]["tick"], 3);
    assert_eq!(report["measurements"]["frames"][2]["tick"], 5);
    let zones = report["measurements"]["zones"].as_array().unwrap();
    for name in [
        "player.frame",
        "native.tick",
        "script.tick",
        "script.source",
        "game.audio_render",
    ] {
        let zone = zones.iter().find(|v| v["name"] == name).unwrap();
        assert_eq!(zone["valid_frames"], 3, "{name}");
        assert!(zone["totals"]["calls"].as_u64().unwrap() >= 3, "{name}");
        assert!(zone["p95_wall_ns"].is_u64(), "{name}");
    }
    assert!(report["measurements"].get("rust_allocations").is_none());
    assert!(report["measurements"].get("quickjs_heap").is_none());
    let path = format!("/entities/{entity}/transform/translation");
    assert_eq!(
        host.play_state_query(&path).unwrap()["untrusted_runtime_data"],
        json!([5., 0., 0.])
    );
    assert_eq!(
        host.profiler_read(&bus.project().id, bus.revision(), 3)
            .unwrap(),
        report
    );
    let process = process_capture.report();
    assert_eq!(process.frame_wall.attempted_frames, 1);
    assert_eq!(process.frame_wall.invalid_frames, 0);
    let process_frame = &process.frame_wall.phases[incant_profile::Phase::Frame as usize];
    assert_eq!(process_frame.invalid_frames, 0);
    assert_eq!(process_frame.samples + process_frame.overflow_samples, 1);
    // Cooking may exceed the process histogram's 100 ms range on a busy host;
    // that overflow is truthful and is not a collector-isolation failure.
    assert_eq!(report["measurements"]["failed_frames"], 0);
    assert_eq!(report["measurements"]["invalid_frames"], 0);
    assert_eq!(
        zones.iter().find(|v| v["name"] == "player.frame").unwrap()["totals"]["calls"],
        3
    );
    FAIL.set(true);
    assert!(
        call(
            &mut bus,
            &mut host,
            "play_run",
            json!({"seconds":0.3,"profile":true})
        )
        .is_err()
    );
    FAIL.set(false);
    assert_eq!(
        host.profiler_read(&bus.project().id, bus.revision(), 3)
            .unwrap(),
        report
    );
    for bad in [
        json!({"frames":0}),
        json!({"frames":257}),
        json!({"frames":6}),
        json!({"frames":1,"path":"other"}),
        json!({"frames":null}),
        json!({"frames":1.5}),
    ] {
        assert!(call(&mut bus, &mut host, "profiler_read", bad).is_err());
    }
    assert!(host.profiler_read(&incant_doc::new_id(), 0, 1).is_err());
    assert_eq!(bus.project().canonical_text().unwrap(), authored);
    assert!(bus.history().is_empty());
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[cfg(feature = "agent-profiler")]
#[test]
fn failed_cancelled_and_snapshot_rejected_candidates_preserve_previous_measurements() {
    FAIL.set(false);
    let root = tempfile::tempdir().unwrap();
    let (bus, entity) = fixture();
    let mut host = Host::new(root.path().to_owned(), NoViewport, fallback);
    let request = || serde_json::from_value(json!({"seconds":0.3,"profile":true})).unwrap();
    let cancel = AtomicBool::new(false);
    host.play_run(bus.project(), 7, request(), &cancel).unwrap();
    let report = host.profiler_read(&bus.project().id, 7, 3).unwrap();
    let path = format!("/entities/{entity}/transform/translation");
    let state = host.play_state_query(&path).unwrap();
    FAIL.set(true);
    assert!(host.play_run(bus.project(), 8, request(), &cancel).is_err());
    FAIL.set(false);
    assert_eq!(host.profiler_read(&bus.project().id, 7, 3).unwrap(), report);
    assert_eq!(host.play_state_query(&path).unwrap(), state);
    assert!(
        host.play_run_checked(bus.project(), 8, request(), &cancel, &mut || {
            Err(AgentError::ProjectChanged)
        })
        .is_err()
    );
    assert!(
        host.play_run_checked(bus.project(), 8, request(), &cancel, &mut || {
            cancel.store(true, Ordering::Relaxed);
            Ok(())
        })
        .is_err()
    );
    assert_eq!(host.profiler_read(&bus.project().id, 7, 3).unwrap(), report);
    assert_eq!(host.play_state_query(&path).unwrap(), state);
    cancel.store(false, Ordering::Relaxed);
    let unprofiled = serde_json::from_value(json!({"seconds":0.1})).unwrap();
    host.play_run(bus.project(), 8, unprofiled, &cancel)
        .unwrap();
    assert!(host.profiler_read(&bus.project().id, 8, 1).is_err());
}

#[cfg(feature = "agent-profiler")]
#[test]
fn stopping_the_controller_retires_its_measurements() {
    FAIL.set(false);
    let root = tempfile::tempdir().unwrap();
    let (bus, _) = fixture();
    let mut controller = incant_agent_native::Controller::new(fallback);
    controller
        .run(
            bus.project(),
            0,
            root.path(),
            serde_json::from_value(json!({"seconds":0.1,"profile":true})).unwrap(),
            &AtomicBool::new(false),
        )
        .unwrap();
    assert!(controller.profiler_read(&bus.project().id, 0, 1).is_ok());
    controller.stop(json!({})).unwrap();
    assert!(controller.profiler_read(&bus.project().id, 0, 1).is_err());
}

#[cfg(not(feature = "agent-profiler"))]
#[test]
fn ordinary_host_refuses_profile_before_running_and_retains_normal_play() {
    FAIL.set(false);
    let root = tempfile::tempdir().unwrap();
    let (mut bus, entity) = fixture();
    let mut host = Host::new(root.path().to_owned(), NoViewport, fallback);
    assert!(!host.profiler_available());
    call(&mut bus, &mut host, "play_run", json!({"seconds":0.2})).unwrap();
    let path = format!("/entities/{entity}/transform/translation");
    let before = host.play_state_query(&path).unwrap();
    assert!(
        call(
            &mut bus,
            &mut host,
            "play_run",
            json!({"seconds":0.3,"profile":true})
        )
        .is_err()
    );
    assert!(call(&mut bus, &mut host, "profiler_read", json!({"frames":1})).is_err());
    assert_eq!(host.play_state_query(&path).unwrap(), before);
}
