//! Content polling for authoring workers. Only registered source dependencies
//! are inspected; cache writes and unrelated project files cannot trigger edits.
use crate::{ImportDetails, ImportOutcome, ImportRequest, ImportSnapshot, PreparedImports, Result};
use incant_assets::{Dependency, SourceSet, load_model, load_texture};
use incant_cmd::{Actor, CommandBus};
use incant_doc::Asset;
use serde::Serialize;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct WatchDiagnostic {
    pub asset_id: String,
    pub source: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct WatchReport {
    pub imports: Vec<ImportOutcome>,
    /// New or changed diagnostics only. Persistent errors are not spammed.
    pub diagnostics: Vec<WatchDiagnostic>,
    pub cleared: Vec<String>,
    pub pending: usize,
    pub revision: u64,
}

type Observation = std::result::Result<Vec<Dependency>, String>;
struct Entry {
    asset: Asset,
    baseline: Vec<Dependency>,
    observed: Option<Observation>,
    since: Instant,
    attempted: Option<Instant>,
    force: bool,
    diagnostic: Option<WatchDiagnostic>,
}

/// Run on an authoring worker that owns its command bus, never a UI/event thread.
/// Polling hashes content (including same-size writes) without decoding unchanged
/// geometry/images. Each asset's read is bounded by SourceSet's source limits.
/// Undo changes the document, not the observed source baseline: a live watcher
/// will not immediately overwrite Undo. A new watcher reconciles sources again.
pub struct SourceWatcher {
    root: PathBuf,
    project_id: Option<String>,
    entries: BTreeMap<String, Entry>,
    debounce: Duration,
    retry: Duration,
}
impl SourceWatcher {
    pub fn new(root: impl Into<PathBuf>, debounce: Duration) -> Self {
        Self {
            root: root.into(),
            project_id: None,
            entries: BTreeMap::new(),
            debounce,
            retry: Duration::from_secs(2),
        }
    }

    pub fn diagnostics(&self) -> impl Iterator<Item = &WatchDiagnostic> {
        self.entries.values().filter_map(|e| e.diagnostic.as_ref())
    }

    /// Successful changed sources publish together in one reversible transaction.
    /// A broken source does not prevent unrelated valid sources from reimporting.
    /// Failed attempts retry at most every two seconds (or after a new edit), so a
    /// newly created dependency can repair a model whose previous cook failed.
    pub fn poll(&mut self, bus: &mut CommandBus, now: Instant) -> Result<WatchReport> {
        let mut report = WatchReport {
            imports: vec![],
            diagnostics: vec![],
            cleared: vec![],
            pending: 0,
            revision: bus.revision(),
        };
        if self.project_id.as_ref() != Some(&bus.project().id) {
            self.entries.clear();
            self.project_id = Some(bus.project().id.clone());
        }
        self.entries.retain(|id, e| {
            if bus.project().assets.contains_key(id) {
                true
            } else {
                if e.diagnostic.is_some() {
                    report.cleared.push(id.clone());
                }
                false
            }
        });
        let mut ready = vec![];
        for (id, asset) in &bus.project().assets {
            let replace = self.entries.get(id).is_none_or(|e| {
                e.asset.path != asset.path
                    || e.asset.kind != asset.kind
                    || e.asset.import_settings != asset.import_settings
            });
            if replace {
                if let Some(previous) = self.entries.get_mut(id) {
                    clear(previous, &mut report);
                }
                let deps = cached_dependencies(&self.root, asset);
                self.entries.insert(
                    id.clone(),
                    Entry {
                        asset: asset.clone(),
                        force: deps.is_none(),
                        baseline: deps.unwrap_or_else(|| {
                            vec![Dependency {
                                path: asset.path.clone(),
                                sha256: String::new(),
                            }]
                        }),
                        observed: None,
                        since: now,
                        attempted: None,
                        diagnostic: None,
                    },
                );
            }
            let entry = self.entries.get_mut(id).unwrap();
            let observation = observe(&self.root, &entry.baseline);
            if entry.observed.as_ref() != Some(&observation) {
                entry.observed = Some(observation.clone());
                entry.since = now;
                entry.attempted = None;
            }
            if !entry.force && observation == Ok(entry.baseline.clone()) {
                clear(entry, &mut report);
                continue;
            }
            report.pending += 1;
            if now.saturating_duration_since(entry.since) >= self.debounce
                && entry
                    .attempted
                    .is_none_or(|at| now.saturating_duration_since(at) >= self.retry)
            {
                ready.push((entry.attempted, id.clone()));
            }
        }
        // Prefer never-attempted sources; a broken early entry cannot starve a
        // project larger than the maximum transaction batch.
        ready.sort();
        ready.truncate(crate::MAX_BATCH_IMPORTS);
        if ready.is_empty() {
            return Ok(report);
        }
        let snapshot = ImportSnapshot::capture(bus);
        let mut imports = vec![];
        for (_, id) in ready {
            let entry = self.entries.get_mut(&id).unwrap();
            entry.attempted = Some(now);
            let result = if matches!(entry.asset.kind.as_str(), "model" | "texture") {
                snapshot.clone().prepare(
                    &self.root,
                    None,
                    &[ImportRequest {
                        source: entry.asset.path.clone(),
                        texture_usage: None,
                    }],
                )
            } else {
                Err(crate::ImportError::Unsupported(entry.asset.path.clone()))
            };
            match result {
                Ok(prepared) => imports.extend(prepared.imports),
                Err(error) => {
                    let issue = WatchDiagnostic {
                        asset_id: id,
                        source: entry.asset.path.clone(),
                        message: error.to_string(),
                    };
                    if entry.diagnostic.as_ref() != Some(&issue) {
                        entry.diagnostic = Some(issue.clone());
                        report.diagnostics.push(issue);
                    }
                }
            }
        }
        if !imports.is_empty() {
            let committed = PreparedImports { snapshot, imports }.commit(
                bus,
                Actor::import("source-watch"),
                "Reimport changed sources",
            )?;
            for import in &committed.imports {
                let entry = self.entries.get_mut(&import.asset.id).unwrap();
                entry.baseline = dependencies(&import.details).to_vec();
                entry.baseline.sort_by(|a, b| a.path.cmp(&b.path));
                entry.observed = Some(Ok(entry.baseline.clone()));
                entry.force = false;
                entry.since = now;
                clear(entry, &mut report);
                report.pending -= 1;
            }
            report.imports = committed.imports;
            report.revision = committed.revision;
        }
        Ok(report)
    }
}

fn clear(entry: &mut Entry, report: &mut WatchReport) {
    if entry.diagnostic.take().is_some() {
        report.cleared.push(entry.asset.id.clone());
    }
}

fn dependencies(details: &ImportDetails) -> &[Dependency] {
    match details {
        ImportDetails::Model { dependencies, .. } | ImportDetails::Texture { dependencies, .. } => {
            dependencies
        }
    }
}

fn cached_dependencies(root: &Path, asset: &Asset) -> Option<Vec<Dependency>> {
    crate::validate_path(&asset.path).ok()?;
    let mut deps = match asset.kind.as_str() {
        "model" => {
            load_model(&root.join(".incant/cache/models"), &asset.sha256)
                .ok()?
                .metadata
                .dependencies
        }
        "texture" => {
            let texture = load_texture(&root.join(".incant/cache/textures"), &asset.sha256).ok()?;
            let usage = match asset.import_settings {
                Some(incant_doc::AssetImportSettings::Texture { usage }) => usage,
                None => incant_assets::TextureUsage::Color,
            };
            if usage != texture.metadata.usage {
                return None;
            }
            vec![texture.metadata.dependency]
        }
        _ => return None,
    };
    if deps.is_empty() || deps.len() > 1024 || !deps.iter().any(|d| d.path == asset.path) {
        return None;
    }
    deps.sort_by(|a, b| a.path.cmp(&b.path));
    for dep in &deps {
        crate::validate_path(&dep.path).ok()?;
    }
    if deps.windows(2).any(|d| d[0].path == d[1].path) {
        return None;
    }
    Some(deps)
}

fn observe(root: &Path, dependencies: &[Dependency]) -> Observation {
    (|| -> std::result::Result<_, Box<dyn std::error::Error>> {
        let mut sources = SourceSet::new(root)?;
        for dep in dependencies {
            crate::validate_path(&dep.path)?;
            sources.read(Path::new(&dep.path))?;
        }
        Ok(sources.dependencies())
    })()
    .map_err(|e| e.to_string())
}
