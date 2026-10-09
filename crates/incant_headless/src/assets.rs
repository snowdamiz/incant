use crate::{Result, print, read_project, save};
use incant_assets::cook_gltf;
use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::{Asset, new_id};
use serde_json::json;
use std::path::{Component, Path};

pub fn import(project: &Path, source: &Path, cache: Option<&Path>) -> Result<()> {
    let root = project
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let path = source.to_str().ok_or("Source path must be UTF-8")?;
    if path.is_empty()
        || path.contains(['\\', ':'])
        || source
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err("Source must be a portable project-relative path".into());
    }
    let initial = read_project(project)?;
    let mut bus = CommandBus::persistent(project.with_extension("journal.jsonl"), initial)?;
    let matching: Vec<_> = bus
        .project()
        .assets
        .values()
        .filter(|a| a.path == path)
        .collect();
    if matching.len() > 1 {
        return Err(
            "Multiple assets use this source path; resolve their identities before reimport".into(),
        );
    }
    let previous = matching.first().copied().cloned();
    let default_cache = root.join(".incant/cache/models");
    let cooked = cook_gltf(root, source, cache.unwrap_or(&default_cache))?;
    let asset = Asset {
        id: previous
            .as_ref()
            .map(|a| a.id.clone())
            .unwrap_or_else(new_id),
        name: previous
            .as_ref()
            .map(|a| a.name.clone())
            .unwrap_or_else(|| {
                source
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned()
            }),
        path: path.into(),
        kind: "model".into(),
        sha256: cooked.metadata.fingerprint.clone(),
    };
    // A repeated unchanged import is a no-op, so refresh checks don't fill Undo.
    let changed = previous.as_ref() != Some(&asset);
    if changed {
        bus.execute(
            vec![Command::UpsertAsset {
                asset: asset.clone(),
            }],
            Actor::import("glTF"),
            format!("Import {}", asset.name),
            None,
        )?;
    }
    save(project, &bus.project().canonical_text()?)?;
    print(
        json!({"asset":asset,"changed":changed,"cache_hit":cooked.cache_hit,"meshes":cooked.meshes.len(),"vertices":cooked.meshes.iter().map(|m|m.vertices.len()).sum::<usize>(),"dependencies":cooked.metadata.dependencies,"revision":bus.revision()}),
    )
}
