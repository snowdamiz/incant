//! Actual app/import/watch/history convergence. No surface pixel capture or design review.
use super::{Options, project, require, wait};
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

const SOURCE: &str = "refresh.png";
const BLUE: &str = "refresh-blue.png";
const SPRITE: &str = "00000000000000000000000004";
const CAMERA: &str = "00000000000000000000000003";

pub(in crate::play) struct Observation {
    pub identity: WorldIdentity,
    pub tick: u64,
    pub frames: u64,
    pub recipe_matches: bool,
    pub adapter: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Fixture {
    format: String,
    version: u32,
    project_id: String,
}
fn owned_fixture(root: &Path, document: &Value) -> Result<(), String> {
    for name in ["texture-workflow.fixture.json", SOURCE, BLUE] {
        let metadata = fs::symlink_metadata(root.join(name)).map_err(|e| e.to_string())?;
        require(
            metadata.is_file() && !metadata.file_type().is_symlink() && metadata.len() <= 65536,
            "texture probe requires its bounded regular fixture files",
        )?;
    }
    let fixture: Fixture = serde_json::from_slice(
        &fs::read(root.join("texture-workflow.fixture.json")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    require(
        fixture.format == "incant-native-texture-workflow"
            && fixture.version == 1
            && document["id"].as_str() == Some(fixture.project_id.as_str())
            && document["assets"].as_object().is_some_and(|a| a.is_empty()),
        "texture probe requires a fresh matching fixture project",
    )
}
fn observe(
    editor: &Editor,
    generation: u32,
    asset: StableId,
    fingerprint: &str,
) -> Result<Observation, String> {
    let (reply, result) = mpsc::channel();
    editor.play.call(Request::ObserveTexture {
        generation,
        asset,
        fingerprint: fingerprint.into(),
        reply,
    })?;
    result
        .recv_timeout(Duration::from_secs(20))
        .map_err(|_| "texture observer deadline")?
}
fn stable(observed: &Observation, baseline: &Observation) -> Result<(), String> {
    require(
        observed.identity == baseline.identity && observed.tick == baseline.tick,
        "texture workflow replaced or advanced the paused native world",
    )
}
fn catalog_frame(
    editor: &Editor,
    generation: u32,
    asset: StableId,
    fingerprint: &str,
    baseline: Option<&Observation>,
) -> Result<Observation, String> {
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut accepted_frames = None;
    loop {
        let status = editor.play.snapshot()?;
        require(
            status.generation == generation && status.phase == "paused",
            "Play session changed during texture workflow",
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
        .ok_or("texture fingerprint absent".into())
}
fn history(editor: &Editor) -> Result<Value, String> {
    let state = editor.bus.lock().map_err(|_| "project lock")?;
    let crate::project::LoadState::Ready(bus) = &*state else {
        return Err("project not ready".into());
    };
    Ok(
        json!({"revision":bus.revision(), "applied":bus.history().len(),
        "redo":bus.redo_history().len(), "transactions":bus.history().iter().map(|t|
        json!({"id":t.id, "description":t.description, "actor":t.actor})).collect::<Vec<_>>()}),
    )
}
fn wait_source(editor: &Editor, failed: bool) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let diagnostics = editor
            .source_diagnostics
            .lock()
            .map_err(|_| "source diagnostics lock")?;
        let present = diagnostics.iter().any(|d| d.source == SOURCE);
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
        "accepted_fingerprint":fingerprint,"recipe_matches":observation.recipe_matches,"history":history})
}

pub(super) fn run(
    app: &tauri::AppHandle,
    editor: &Editor,
    options: &Options,
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
        require(Instant::now() < deadline, "texture app startup deadline")?;
        thread::sleep(Duration::from_millis(20));
    }
    let root = editor
        .asset_root
        .as_ref()
        .ok_or("texture probe requires saved fixture")?;
    let before = project(editor)?;
    owned_fixture(root, &before.0)?;
    let blue = fs::read(root.join(BLUE)).map_err(|e| e.to_string())?;
    let imported = tauri::async_runtime::block_on(assets::engine_import(
        app.state::<Arc<Editor>>(),
        app.clone(),
        vec![incant_import::ImportRequest {
            source: SOURCE.into(),
            texture_usage: Some(incant_assets::TextureUsage::Color),
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
        .find(|(_, a)| a["path"] == SOURCE)
        .map(|(id, _)| id.clone())
        .ok_or("imported texture absent")?;
    let stable_asset = StableId(
        asset
            .parse::<ulid::Ulid>()
            .map_err(|e| e.to_string())?
            .to_bytes(),
    );
    let scene = before.0["scenes"]
        .as_object()
        .and_then(|s| (s.len() == 1).then(|| s.keys().next().unwrap().clone()))
        .ok_or("fixture requires one scene")?;
    let command = serde_json::from_value::<incant_cmd::Command>(json!({"op":"create_entity","scene_id":scene,"entity":{
        "id":SPRITE,"name":"Refresh sprite","parent":null,"provenance":null,"components":{
            "Transform":{"translation":[0,0,-2],"rotation":[0,0,0,1],"scale":[1,1,1]},
            "SpriteRenderer":{"texture":asset,"region":[0,0,1,1],"size":[1,1],"pivot":[0.5,0.5],"tint":[1,1,1,1],"sampling":"nearest","mode":{"kind":"simple"}}
        }}})).map_err(|e| e.to_string())?;
    let bound = tauri::async_runtime::block_on(crate::engine_execute(
        app.state::<Arc<Editor>>(),
        vec![command],
        "Bind native texture workflow sprite".into(),
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
    let running = wait(editor, |s| {
        s.spatial && s.tick >= 2 && s.presented_frames >= 2
    })?;
    execute(
        editor,
        Action::Pause {
            generation: running.generation,
        },
    )?;
    let generation = editor.play.snapshot()?.generation;
    let baseline = catalog_frame(editor, generation, stable_asset, &red, None)?;
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
    let mut stages = vec![point("manual_import_red", &baseline, &red, initial_history)];
    // Let the real watcher establish its baseline before the controlled source edit.
    thread::sleep(Duration::from_millis(1500));
    let red_document = project(editor)?.0;
    fs::write(root.join(SOURCE), &blue).map_err(|e| e.to_string())?;
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
        "watched_blue",
        &blue_frame,
        &blue_fingerprint,
        watched_history,
    ));
    let blue_document = project(editor)?.0;
    let undone =
        tauri::async_runtime::block_on(crate::engine_history(app.state::<Arc<Editor>>(), false))?;
    require(
        undone["project"] == red_document,
        "Undo did not restore the red authored catalog",
    )?;
    let undo_frame = catalog_frame(editor, generation, stable_asset, &red, Some(&baseline))?;
    stages.push(point("undo_red", &undo_frame, &red, history(editor)?));
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
        "Redo did not restore the blue authored catalog",
    )?;
    let redo_frame = catalog_frame(
        editor,
        generation,
        stable_asset,
        &blue_fingerprint,
        Some(&baseline),
    )?;
    stages.push(point(
        "redo_blue",
        &redo_frame,
        &blue_fingerprint,
        history(editor)?,
    ));
    let before_bad = project(editor)?;
    fs::write(
        root.join(SOURCE),
        b"invalid PNG owned by the texture workflow fixture",
    )
    .map_err(|e| e.to_string())?;
    let refusal = match tauri::async_runtime::block_on(assets::engine_import(
        app.state::<Arc<Editor>>(),
        app.clone(),
        vec![incant_import::ImportRequest {
            source: SOURCE.into(),
            texture_usage: Some(incant_assets::TextureUsage::Color),
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
    wait_source(editor, true)?;
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
        "malformed_source_retained_blue",
        &retained,
        &blue_fingerprint,
        history(editor)?,
    ));
    fs::write(root.join(SOURCE), &blue).map_err(|e| e.to_string())?;
    wait_source(editor, false)?;
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
        "source_recovered_blue",
        &recovered,
        &blue_fingerprint,
        history(editor)?,
    ));
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
    require(
        stepped["tick"].as_u64() == Some(baseline.tick + 1)
            && (x(&stepped)? - x(&at_pause)? - 2.).abs() < 1e-9,
        "same VM did not continue on the next step",
    )?;
    let final_observation = observe(editor, generation, stable_asset, &blue_fingerprint)?;
    require(
        final_observation.identity == baseline.identity && final_observation.recipe_matches,
        "step replaced world/catalog",
    )?;
    require(
        project(editor)? == before_bad,
        "Play continuation changed authoring",
    )?;
    execute(editor, Action::Stop { generation })?;
    require(
        observe(editor, generation, stable_asset, &blue_fingerprint).is_err(),
        "stopped world remained observable",
    )?;
    Ok(
        json!({"passed":true,"platform":std::env::consts::OS,"adapter":baseline.adapter,
        "play_generation":generation,"paused_tick":baseline.tick,"final_tick":final_observation.tick,
        "world_identity_unchanged":true,"same_vm_continued":true,"actual_window_presentations":true,
        "manual_import_execute_history_handlers":true,"real_source_watcher":true,"undo_not_overwritten":true,
        "malformed_source_refusal":refusal,"source_recovery":true,"stale_generation_refused":true,
        "stop_releases_world":true,"stages":stages,"pixel_capture":false,"appearance_review":false,
        "performance_or_phase_gate":false}),
    )
}
