use crate::{Result, save};
use incant_assets::AssetStore;
use incant_cmd::CommandBus;
use incant_doc::Project;
use incant_import::SourceWatcher;
use serde_json::{Value, json};
use std::{
    fs,
    io::{self, Write},
    path::Path,
    time::{Duration, Instant},
};

fn emit(value: Value) -> Result<()> {
    let mut out = io::stdout().lock();
    writeln!(out, "{value}")?;
    out.flush()?;
    Ok(())
}

pub fn run(
    project: &Path,
    interval: Duration,
    debounce: Duration,
    polls: Option<u64>,
) -> Result<()> {
    let root = project
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut saved = fs::read_to_string(project)?;
    let mut bus = CommandBus::persistent(
        project.with_extension("journal.jsonl"),
        Project::from_text(&saved)?,
    )?;
    let mut watcher = SourceWatcher::new(root, debounce);
    let mut runtime = AssetStore::default();
    let mut runtime_error = None;
    // Persist recovered journal state as well, but never overwrite an external edit.
    let recovered = bus.project().canonical_text()?;
    if saved != recovered {
        ensure_unchanged(project, &saved)?;
        save(project, &recovered)?;
        saved = recovered;
    }
    emit(json!({"event":"watching","project_id":bus.project().id,"revision":bus.revision()}))?;
    let mut count = 0_u64;
    loop {
        ensure_unchanged(project, &saved)?;
        let report = watcher.poll(&mut bus, Instant::now())?;
        if report.imports.iter().any(|i| i.changed) {
            ensure_unchanged(project, &saved)?;
            let text = bus.project().canonical_text()?;
            save(project, &text)?;
            saved = text;
        }
        let previous_error = runtime_error.clone();
        let changes = match runtime.sync_project(bus.project(), root) {
            Ok(changes) => {
                runtime_error = None;
                Some(changes)
            }
            Err(error) => {
                runtime_error = Some(error.to_string());
                None
            }
        };
        if !report.imports.is_empty()
            || !report.diagnostics.is_empty()
            || !report.cleared.is_empty()
            || previous_error != runtime_error
            || count == 0
        {
            emit(
                json!({"event":"assets","report":report,"runtime_changes":changes,
                "runtime_error":runtime_error,"assets":runtime.snapshot()}),
            )?;
        }
        count += 1;
        if polls.is_some_and(|limit| count >= limit) {
            let diagnostics: Vec<_> = watcher.diagnostics().collect();
            emit(
                json!({"event":"stopped","polls":count,"revision":bus.revision(),
                "pending":report.pending,"diagnostics":diagnostics,"runtime_error":runtime_error}),
            )?;
            if report.pending != 0 || !diagnostics.is_empty() || runtime_error.is_some() {
                return Err("asset watch stopped with unsettled sources or errors".into());
            }
            return Ok(());
        }
        std::thread::sleep(interval);
    }
}

fn ensure_unchanged(project: &Path, expected: &str) -> Result<()> {
    if fs::read_to_string(project)? != expected {
        return Err("project file changed outside the command bus; asset watch stopped without overwriting it".into());
    }
    Ok(())
}
