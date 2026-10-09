use crate::{Result, print, read_project, save};
use incant_assets::TextureUsage;
use incant_cmd::{Actor, CommandBus};
use incant_import::{ImportRequest, ImportSnapshot};
use std::path::Path;

pub fn load_runtime(
    project: &Path,
    document: &incant_doc::Project,
) -> Result<incant_assets::AssetStore> {
    let root = project
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut store = incant_assets::AssetStore::default();
    store.sync(document, &root.join(".incant/cache"))?;
    Ok(store)
}

pub fn import(
    project: &Path,
    source: &Path,
    cache: Option<&Path>,
    texture_usage: Option<&str>,
) -> Result<()> {
    let root = project
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let request = ImportRequest {
        source: source.to_str().ok_or("Source path must be UTF-8")?.into(),
        texture_usage: match texture_usage {
            Some("color") => Some(TextureUsage::Color),
            Some("linear") => Some(TextureUsage::Linear),
            Some("normal") => Some(TextureUsage::Normal),
            Some(_) => return Err("Unknown texture usage".into()),
            None => None,
        },
    };
    let mut bus = CommandBus::persistent(
        project.with_extension("journal.jsonl"),
        read_project(project)?,
    )?;
    let prepared = ImportSnapshot::capture(&bus).prepare(root, cache, &[request])?;
    let outcome = &prepared.outcomes()[0];
    let actor = Actor::import(&outcome.asset.kind);
    let description = format!("Import {}", outcome.asset.name);
    let committed = prepared.commit(&mut bus, actor, description)?;
    save(project, &bus.project().canonical_text()?)?;
    let mut response = serde_json::to_value(&committed.imports[0])?;
    response["revision"] = serde_json::json!(committed.revision);
    print(response)
}
