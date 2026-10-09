use crate::{Result, print, read_project, save};
use incant_assets::{TextureUsage, cook_gltf, cook_texture};
use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::{Asset, AssetImportSettings, new_id};
use serde_json::json;
use std::path::{Component, Path};

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
    let extension = source
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let (kind, fingerprint, cache_hit, import_settings, details) = match extension.as_str() {
        "gltf" | "glb" => {
            if texture_usage.is_some() {
                return Err("Texture usage applies only to standalone images".into());
            }
            let default_cache = root.join(".incant/cache/models");
            let cooked = cook_gltf(root, source, cache.unwrap_or(&default_cache))?;
            (
                "model",
                cooked.metadata.fingerprint,
                cooked.cache_hit,
                None,
                json!({"meshes":cooked.meshes.len(),"textures":cooked.images.len(),"vertices":cooked.meshes.iter().map(|m|m.vertices.len()).sum::<usize>(),"dependencies":cooked.metadata.dependencies}),
            )
        }
        "png" | "jpg" | "jpeg" | "exr" => {
            let default_cache = root.join(".incant/cache/textures");
            let cache = cache.unwrap_or(&default_cache);
            let usage = match texture_usage {
                Some("color") => TextureUsage::Color,
                Some("linear") => TextureUsage::Linear,
                Some("normal") => TextureUsage::Normal,
                Some(_) => return Err("Unknown texture usage".into()),
                None => match &previous {
                    Some(old) => match old.import_settings {
                        Some(AssetImportSettings::Texture { usage }) => usage,
                        None => TextureUsage::Color,
                    },
                    _ => TextureUsage::Color,
                },
            };
            let cooked = cook_texture(root, source, cache, usage)?;
            (
                "texture",
                cooked.metadata.fingerprint,
                cooked.cache_hit,
                Some(AssetImportSettings::Texture { usage }),
                json!({"width":cooked.texture.width,"height":cooked.texture.height,"format":cooked.texture.format,"mip_levels":cooked.texture.levels.len(),"usage":usage,"dependencies":[cooked.metadata.dependency]}),
            )
        }
        _ => return Err("Import supports glTF, GLB, PNG, JPEG and EXR".into()),
    };
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
        kind: kind.into(),
        sha256: fingerprint,
        import_settings,
    };
    let changed = previous.as_ref() != Some(&asset);
    if changed {
        bus.execute(
            vec![Command::UpsertAsset {
                asset: asset.clone(),
            }],
            Actor::import(kind),
            format!("Import {}", asset.name),
            None,
        )?;
    }
    save(project, &bus.project().canonical_text()?)?;
    print(
        json!({"asset":asset,"changed":changed,"cache_hit":cache_hit,"details":details,"revision":bus.revision()}),
    )
}
