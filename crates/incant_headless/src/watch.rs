use crate::Result;
use incant_assets::AssetStore;
use incant_cmd::{Actor, CommandBus};
use incant_import::SourceWatcher;
use serde_json::{Value, json};
use std::{
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

fn check_project_file(bus: &CommandBus) -> Result<()> {
    bus.check_project_file().map_err(|error| {
        format!("project file changed outside command bus; asset watch stopped without overwriting it: {error}").into()
    })
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
    let mut bus = CommandBus::open_project_file_with_journal(
        project,
        project.with_extension("journal.jsonl"),
    )?;
    let mut watcher = SourceWatcher::new(root, debounce);
    let mut runtime = AssetStore::default();
    let mut runtime_error = None;
    // Persist recovered journal state as well, but never overwrite an external edit.
    bus.save_project_file(bus.revision(), Actor::user("asset-watch"))?;
    emit(json!({"event":"watching","project_id":bus.project().id,"revision":bus.revision()}))?;
    let mut count = 0_u64;
    loop {
        check_project_file(&bus)?;
        let report = watcher.poll(&mut bus, Instant::now())?;
        if report.imports.iter().any(|i| i.changed) {
            check_project_file(&bus)?;
            bus.save_project_file(bus.revision(), Actor::user("asset-watch"))?;
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
