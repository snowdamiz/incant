//! Actual app/import/watch/history convergence. No surface pixel capture or design review.
use super::{Options, project, require, wait_observed};
use crate::{
    Editor, assets,
    play::{Action, Request, StartOptions, execute},
};
use incant_runtime::{StableId, WorldIdentity};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    fs,
    path::Path,
    sync::{Arc, mpsc},
    thread,
    time::{Duration, Instant},
};
use tauri::Manager;

mod animation;
mod kind;
pub(in crate::play) use animation::AnimationObservation;
pub(super) use kind::Kind;
const ENTITY: &str = "00000000000000000000000004";

#[derive(Clone, Copy)]
struct ObservedAsset {
    id: StableId,
    kind: incant_runtime::VisualAssetKind,
    animation_entity: Option<StableId>,
}
const CAMERA: &str = "00000000000000000000000003";

pub(in crate::play) struct Observation {
    pub identity: WorldIdentity,
    pub tick: u64,
    pub frames: u64,
    pub recipe_matches: bool,
    pub adapter: String,
    pub animation: Option<AnimationObservation>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Fixture {
    format: String,
    version: u32,
    project_id: String,
}
fn owned_fixture(root: &Path, document: &Value, kind: Kind) -> Result<(), String> {
    for name in [kind.fixture(), kind.source(), kind.replacement()] {
        let metadata = fs::symlink_metadata(root.join(name)).map_err(|e| e.to_string())?;
        require(
            metadata.is_file() && !metadata.file_type().is_symlink() && metadata.len() <= 65536,
            "asset probe requires its bounded regular fixture files",
        )?;
    }
    if kind.animated() {
        let metadata = fs::symlink_metadata(root.join("refresh-incompatible.gltf"))
            .map_err(|e| e.to_string())?;
        require(
            metadata.is_file() && !metadata.file_type().is_symlink() && metadata.len() <= 65536,
            "animation probe requires its bounded incompatible fixture",
        )?;
    }
    let fixture: Fixture =
        serde_json::from_slice(&fs::read(root.join(kind.fixture())).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    require(
        fixture.format == kind.format()
            && fixture.version == 1
            && document["id"].as_str() == Some(fixture.project_id.as_str())
            && document["assets"].as_object().is_some_and(|a| a.is_empty()),
        "asset probe requires a fresh matching fixture project",
    )
}
fn observe(
    editor: &Editor,
    generation: u32,
    asset: ObservedAsset,
    fingerprint: &str,
) -> Result<Observation, String> {
    let (reply, result) = mpsc::channel();
    editor.play.call(Request::ObserveAsset {
        generation,
        asset: asset.id,
        kind: asset.kind,
        fingerprint: fingerprint.into(),
        animation_entity: asset.animation_entity,
        reply,
    })?;
    result
        .recv_timeout(Duration::from_secs(20))
        .map_err(|_| "asset observer deadline")?
}
fn stable(observed: &Observation, baseline: &Observation) -> Result<(), String> {
    require(
        observed.identity == baseline.identity
            && observed.tick == baseline.tick
            && observed.animation == baseline.animation,
        "asset workflow replaced or advanced the paused native world/animation",
    )
}
fn catalog_frame(
    editor: &Editor,
    generation: u32,
    asset: ObservedAsset,
    fingerprint: &str,
    baseline: Option<&Observation>,
) -> Result<Observation, String> {
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut accepted_frames = None;
    loop {
        let status = editor.play.snapshot()?;
        require(
            status.generation == generation && status.phase == "paused",
            "Play session changed during asset workflow",
        )?;
        let observed = observe(editor, generation, asset, fingerprint)?;
        if let Some(baseline) = baseline {
            stable(&observed, baseline)?;
        }
        if observed.recipe_matches {
            match accepted_frames {
                Some(frames) if observed.frames > frames => return Ok(observed),
                None => accepted_frames = Some(observed.frames),
                _ => {}
            }
        } else {
            accepted_frames = None;
        }
        require(
            Instant::now() < deadline,
            "live catalog/next-presentation deadline",
        )?;
        thread::sleep(Duration::from_millis(20));
    }
}
fn asset_fingerprint(editor: &Editor, asset: &str) -> Result<String, String> {
    project(editor)?.0["assets"][asset]["sha256"]
        .as_str()
        .map(str::to_owned)
        .ok_or("asset fingerprint absent".into())
}
fn history(editor: &Editor) -> Result<Value, String> {
    let state = editor.bus.lock().map_err(|_| "project lock")?;
    let crate::project::LoadState::Ready(bus) = &*state else {
        return Err("project not ready".into());
    };
    Ok(
        json!({"revision":bus.revision(), "applied":bus.history().len(),
        "redo":bus.redo_history().count(), "transactions":bus.history().iter().map(|t|
        json!({"id":t.id, "description":t.description, "actor":t.actor})).collect::<Vec<_>>()}),
    )
}
fn wait_source(editor: &Editor, kind: Kind, failed: bool) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let diagnostics = editor
            .source_diagnostics
            .lock()
            .map_err(|_| "source diagnostics lock")?;
        let present = diagnostics.iter().any(|d| d.source == kind.source());
        drop(diagnostics);
        if present == failed {
            return Ok(());
        }
        require(
            Instant::now() < deadline,
            "real source watcher diagnostic deadline",
        )?;
        thread::sleep(Duration::from_millis(20));
    }
}
fn point(name: &str, observation: &Observation, fingerprint: &str, history: Value) -> Value {
    json!({"stage":name,"tick":observation.tick,"frames":observation.frames,
        "accepted_fingerprint":fingerprint,"recipe_matches":observation.recipe_matches,"history":history,
        "animation":observation.animation})
}
fn initial_play_ready(status: &super::Status) -> bool {
    status.spatial && status.tick >= 2 && status.presented_frames >= 2
}
fn startup_deadline(
    status: &super::Status,
    window: Value,
    viewport: Value,
    render_error: Value,
) -> String {
    json!({"stage":"initial_live_play", "error":"native Play probe deadline",
        "last_play_status":status, "window":window, "viewport":viewport,
        "viewport_error":render_error})
    .to_string()
}

pub(super) fn run(
    app: &tauri::AppHandle,
    editor: &Editor,
    options: &Options,
    kind: Kind,
) -> Result<Value, String> {
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
        require(Instant::now() < deadline, "asset app startup deadline")?;
        thread::sleep(Duration::from_millis(20));
    }
    let root = editor
        .asset_root
        .as_ref()
        .ok_or("asset probe requires saved fixture")?;
    let before = project(editor)?;
    owned_fixture(root, &before.0, kind)?;
    let blue = fs::read(root.join(kind.replacement())).map_err(|e| e.to_string())?;
    let imported = tauri::async_runtime::block_on(assets::engine_import(
        app.state::<Arc<Editor>>(),
        app.clone(),
        vec![incant_import::ImportRequest {
            source: kind.source().into(),
            texture_usage: kind.texture_usage(),
        }],
        before.1,
    ))
    .map_err(|e| serde_json::to_value(e).map_or_else(|e| e.to_string(), |v| v.to_string()))?;
    let imported = serde_json::to_value(imported).map_err(|e| e.to_string())?;
    let revision = imported["commit"]["revision"]
        .as_u64()
        .ok_or("import revision absent")?;
    require(
        revision == before.1 + 1 && imported["commit"]["transaction"].is_string(),
        "manual import did not commit once",
    )?;
    let assets = imported["snapshot"]["project"]["assets"]
        .as_object()
        .ok_or("import assets absent")?;
    let asset = assets
        .iter()
        .find(|(_, a)| a["path"] == kind.source())
        .map(|(id, _)| id.clone())
        .ok_or("imported asset absent")?;
    let stable_asset = ObservedAsset {
        kind: kind.visual_kind(),
        id: StableId(
            asset
                .parse::<ulid::Ulid>()
                .map_err(|e| e.to_string())?
                .to_bytes(),
        ),
        animation_entity: kind.animated().then_some(StableId(
            ENTITY
                .parse::<ulid::Ulid>()
                .map_err(|e| e.to_string())?
                .to_bytes(),
        )),
    };
    let scene = before.0["scenes"]
        .as_object()
        .and_then(|s| (s.len() == 1).then(|| s.keys().next().unwrap().clone()))
        .ok_or("fixture requires one scene")?;
    let command = kind.entity_command(&scene, ENTITY, &asset)?;
    let bound = tauri::async_runtime::block_on(crate::engine_execute(
        app.state::<Arc<Editor>>(),
        vec![command],
        "Bind native asset workflow entity".into(),
        revision,
    ))?;
    let revision = bound["revision"]
        .as_u64()
        .ok_or("binding revision absent")?;
    let red = asset_fingerprint(editor, &asset)?;
    let initial_history = history(editor)?;
    require(
        initial_history["transactions"].as_array().is_some_and(|h| {
            h.len() >= 2
                && h[h.len() - 2]["actor"]["origin"] == "user"
                && h[h.len() - 1]["actor"]["origin"] == "user"
        }),
        "manual import/binding lost user provenance",
    )?;
    execute(
        editor,
        Action::Start {
            generation: editor.play.snapshot()?.generation,
            expected_revision: revision,
            options: StartOptions {
                camera: Some(CAMERA.into()),
                ..Default::default()
            },
        },
    )?;
    let running = wait_observed(editor, initial_play_ready, |status| {
        let window = json!({
            "focused":window.is_focused().map_err(|e| e.to_string()),
            "visible":window.is_visible().map_err(|e| e.to_string()),
            "minimized":window.is_minimized().map_err(|e| e.to_string()),
            "physical_size":window.inner_size().map(|s| json!({"width":s.width,"height":s.height})).map_err(|e|e.to_string()),
        });
        let viewport = match editor.viewport.lock() {
            Ok(value) => json!(value.map(|v| json!({"rect":v.rect,"corner_radii":v.corner_radii}))),
            Err(error) => json!({"error":error.to_string()}),
        };
        let render_error = match editor.viewport_error.lock() {
            Ok(value) => json!(*value),
            Err(error) => json!({"error":error.to_string()}),
        };
        startup_deadline(status, window, viewport, render_error)
    })?;
    execute(
        editor,
        Action::Pause {
            generation: running.generation,
        },
    )?;
    let generation = editor.play.snapshot()?.generation;
    let baseline = catalog_frame(editor, generation, stable_asset, &red, None)?;
    if kind.animated() {
        animation::validate_at_tick(baseline.animation.as_ref(), baseline.tick)?;
    }
    require(
        baseline.adapter.contains("backend: Metal"),
        "this control requires the actual Mac Metal adapter",
    )?;
    let at_pause = execute(
        editor,
        Action::Inspect {
            generation,
            entity: options.entity.clone(),
        },
    )?;
    let mut stages = vec![point(kind.stage(0), &baseline, &red, initial_history)];
    // Let the real watcher establish its baseline before the controlled source edit.
    thread::sleep(Duration::from_millis(1500));
    let red_document = project(editor)?.0;
    fs::write(root.join(kind.source()), &blue).map_err(|e| e.to_string())?;
    let deadline = Instant::now() + Duration::from_secs(20);
    let blue_fingerprint = loop {
        let value = asset_fingerprint(editor, &asset)?;
        if value != red {
            break value;
        }
        require(
            Instant::now() < deadline,
            "real source watcher reimport deadline",
        )?;
        thread::sleep(Duration::from_millis(20));
    };
    let blue_frame = catalog_frame(
        editor,
        generation,
        stable_asset,
        &blue_fingerprint,
        Some(&baseline),
    )?;
    let watched_history = history(editor)?;
    require(
        watched_history["revision"].as_u64() == Some(revision + 1)
            && watched_history["transactions"]
                .as_array()
                .and_then(|h| h.last())
                .is_some_and(|t| t["actor"]["origin"] == "import"),
        "source watcher did not use one real import transaction",
    )?;
    stages.push(point(
        kind.stage(1),
        &blue_frame,
        &blue_fingerprint,
        watched_history,
    ));
    let blue_document = project(editor)?.0;
    let undone =
        tauri::async_runtime::block_on(crate::engine_history(app.state::<Arc<Editor>>(), false))?;
    require(
        undone["project"] == red_document,
        "Undo did not restore the original authored catalog",
    )?;
    let undo_frame = catalog_frame(editor, generation, stable_asset, &red, Some(&baseline))?;
    stages.push(point(kind.stage(2), &undo_frame, &red, history(editor)?));
    // This outlasts two watcher polls plus debounce; Undo must not be overwritten.
    thread::sleep(Duration::from_millis(1500));
    require(
        project(editor)?.0 == red_document,
        "watcher overwrote Undo without a new source edit",
    )?;
    stable(&observe(editor, generation, stable_asset, &red)?, &baseline)?;
    let redone =
        tauri::async_runtime::block_on(crate::engine_history(app.state::<Arc<Editor>>(), true))?;
    require(
        redone["project"] == blue_document,
        "Redo did not restore the replacement authored catalog",
    )?;
    let redo_frame = catalog_frame(
        editor,
        generation,
        stable_asset,
        &blue_fingerprint,
        Some(&baseline),
    )?;
    stages.push(point(
        kind.stage(3),
        &redo_frame,
        &blue_fingerprint,
        history(editor)?,
    ));
    let (refusal, stable_authoring) = if kind.animated() {
        let recovered = animation::refuse_and_recover(
            editor,
            generation,
            stable_asset,
            &blue_fingerprint,
            &baseline,
            &blue,
            &mut stages,
        )?;
        (recovered.refusal, recovered.authoring)
    } else {
        let before_bad = project(editor)?;
        fs::write(
            root.join(kind.source()),
            b"invalid source owned by the native asset workflow fixture",
        )
        .map_err(|e| e.to_string())?;
        let refusal = match tauri::async_runtime::block_on(assets::engine_import(
            app.state::<Arc<Editor>>(),
            app.clone(),
            vec![incant_import::ImportRequest {
                source: kind.source().into(),
                texture_usage: kind.texture_usage(),
            }],
            before_bad.1,
        )) {
            Ok(_) => return Err("malformed manual import was accepted".into()),
            Err(error) => serde_json::to_value(error).map_err(|e| e.to_string())?,
        };
        require(
            refusal["code"] == "asset.import",
            "manual refusal lost its structured import diagnostic",
        )?;
        wait_source(editor, kind, true)?;
        require(
            project(editor)? == before_bad,
            "bad source changed project/revision/history",
        )?;
        let retained = catalog_frame(
            editor,
            generation,
            stable_asset,
            &blue_fingerprint,
            Some(&baseline),
        )?;
        stages.push(point(
            kind.stage(4),
            &retained,
            &blue_fingerprint,
            history(editor)?,
        ));
        fs::write(root.join(kind.source()), &blue).map_err(|e| e.to_string())?;
        wait_source(editor, kind, false)?;
        require(
            project(editor)? == before_bad,
            "identical source recovery appended a transaction",
        )?;
        let recovered = catalog_frame(
            editor,
            generation,
            stable_asset,
            &blue_fingerprint,
            Some(&baseline),
        )?;
        stages.push(point(
            kind.stage(5),
            &recovered,
            &blue_fingerprint,
            history(editor)?,
        ));
        (refusal, before_bad)
    };
    require(
        execute(
            editor,
            Action::Inspect {
                generation,
                entity: options.entity.clone(),
            },
        )? == at_pause,
        "paused VM/world changed",
    )?;
    let stale = generation
        .checked_sub(1)
        .ok_or("fixture generation was not advanced")?;
    require(
        observe(editor, stale, stable_asset, &blue_fingerprint).is_err(),
        "old-generation observer accepted",
    )?;
    require(
        execute(editor, Action::Step { generation: stale }).is_err(),
        "stale generation step accepted",
    )?;
    execute(editor, Action::Step { generation })?;
    let stepped = execute(
        editor,
        Action::Inspect {
            generation,
            entity: options.entity.clone(),
        },
    )?;
    let x = |v: &Value| {
        v["transform"]["translation"][0]
            .as_f64()
            .ok_or("fixture mover coordinate absent")
    };
    let before_translation = x(&at_pause)?;
    let observed_delta = x(&stepped)? - before_translation;
    let expected_delta = 2. * (baseline.tick + 1) as f64;
    require(
        stepped["tick"].as_u64() == Some(baseline.tick + 1)
            && (before_translation - (baseline.tick * (baseline.tick + 1)) as f64).abs() < 1e-9
            && (observed_delta - expected_delta).abs() < 1e-9,
        "module update counter did not continue on the next step",
    )?;
    let final_observation = observe(editor, generation, stable_asset, &blue_fingerprint)?;
    require(
        final_observation.identity == baseline.identity && final_observation.recipe_matches,
        "step replaced world/catalog",
    )?;
    require(
        project(editor)? == stable_authoring,
        "Play continuation changed authoring",
    )?;
    if kind.animated() {
        animation::continued(
            baseline.animation.as_ref(),
            final_observation.animation.as_ref(),
        )?;
    }
    execute(editor, Action::Stop { generation })?;
    require(
        observe(editor, generation, stable_asset, &blue_fingerprint).is_err(),
        "stopped world remained observable",
    )?;
    let mut report = json!({"passed":true,"asset_kind":kind.label(),"platform":std::env::consts::OS,"adapter":baseline.adapter,
        "play_generation":generation,"paused_tick":baseline.tick,"final_tick":final_observation.tick,
        "world_identity_unchanged":true,"module_counter_continued":true,"actual_window_presentations":true,
        "vm_counter":{"before_translation":before_translation,"observed_delta":observed_delta,"expected_delta":expected_delta},
        "manual_import_execute_history_handlers":true,"real_source_watcher":true,"undo_not_overwritten":true,
        "malformed_source_refusal":refusal,"source_recovery":true,"stale_generation_refused":true,
        "stop_releases_world":true,"stages":stages,"pixel_capture":false,"appearance_review":false,
        "performance_or_phase_gate":false});
    if kind.animated() {
        report
            .as_object_mut()
            .unwrap()
            .remove("malformed_source_refusal");
        report["incompatible_refresh_refusal"] = refusal;
        report["animation_continued"] = json!(true);
        report["committed_incompatible_refused"] = json!(true);
        report["animation_step"] =
            json!({"before":baseline.animation,"after":final_observation.animation});
    }
    Ok(report)
}

#[cfg(test)]
mod startup_diagnostic_tests {
    use super::*;

    #[test]
    fn initial_wait_requires_spatial_ticks_and_actual_frames() {
        let (service, _) = crate::play::Service::new();
        let mut status = service.snapshot().unwrap();
        status.spatial = true;
        status.tick = 2;
        status.presented_frames = 2;
        assert!(initial_play_ready(&status));
        status.spatial = false;
        assert!(!initial_play_ready(&status));
        status.spatial = true;
        status.tick = 0;
        assert!(!initial_play_ready(&status));
        status.tick = 2;
        status.presented_frames = 0;
        assert!(!initial_play_ready(&status));
    }

    #[test]
    fn failed_startup_retains_last_status_and_native_eligibility_without_claiming_pass() {
        let (service, _) = crate::play::Service::new();
        let mut status = service.snapshot().unwrap();
        status.phase = "running";
        status.spatial = true;
        status.tick = 0;
        status.presented_frames = 7;
        status.suspended = true;
        let window =
            json!({"focused":{"Ok":false},"physical_size":{"Ok":{"width":1440,"height":900}}});
        let viewport = json!({"rect":[0,0,1200,800]});
        let diagnostic: Value = serde_json::from_str(&startup_deadline(
            &status,
            window.clone(),
            viewport.clone(),
            Value::Null,
        ))
        .unwrap();
        assert_eq!(diagnostic["stage"], "initial_live_play");
        assert_eq!(diagnostic["last_play_status"]["tick"], 0);
        assert_eq!(diagnostic["last_play_status"]["presentedFrames"], 7);
        assert_eq!(diagnostic["last_play_status"]["suspended"], true);
        assert_eq!(diagnostic["window"], window);
        assert_eq!(diagnostic["viewport"], viewport);
        assert!(diagnostic.get("passed").is_none());
    }
}
