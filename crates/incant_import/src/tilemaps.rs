//! The existing atomic import/cache-validation path also protects tilemap IDs.
use crate::sprites::{
    SpriteDiagnostic, SpriteDiagnosticCode as Code, SpriteDiagnosticSeverity as Severity,
    TextureInfo,
};
use incant_assets::TextureUsage;
use incant_doc::{Project, Tilemap};
use std::collections::BTreeMap;

pub(crate) fn check(
    project: &Project,
    textures: &BTreeMap<String, Result<TextureInfo, String>>,
) -> Vec<SpriteDiagnostic> {
    let mut output = Vec::new();
    for (sid, scene) in &project.scenes {
        for (eid, entity) in &scene.entities {
            let Some(value) = entity.components.get("Tilemap") else {
                continue;
            };
            let map: Tilemap = serde_json::from_value(value.clone()).expect("validated tilemap");
            let Some(texture) = textures.get(&map.tileset) else {
                continue;
            };
            let mut issue = |frame, field: &str, code, message: String| {
                output.push(SpriteDiagnostic {
                    scene_id: sid.clone(),
                    entity_id: eid.clone(),
                    texture_id: map.tileset.clone(),
                    frame,
                    path: format!("/scenes/{sid}/entities/{eid}/components/Tilemap/{field}"),
                    severity: Severity::Error,
                    code,
                    message,
                });
            };
            let texture = match texture {
                Ok(texture) => texture,
                Err(error) => {
                    issue(None, "tileset", Code::CacheUnavailable, error.clone());
                    continue;
                }
            };
            if texture.usage != TextureUsage::Color {
                issue(
                    None,
                    "tileset",
                    Code::TextureUsage,
                    "tilemaps require a cooked color atlas".into(),
                );
                continue;
            }
            let Some(atlas) = &texture.atlas else {
                issue(
                    None,
                    "tileset",
                    Code::NotAtlas,
                    "tilemap frame IDs require a cooked atlas".into(),
                );
                continue;
            };
            // One diagnostic per entity/texture/frame; the first cell supplies
            // its precise typed document path, even if thousands share a frame.
            let mut frames = BTreeMap::new();
            for (layer, value) in map.layers.iter().enumerate() {
                for (cell, frame) in value.cells.iter().enumerate() {
                    if let Some(frame) = frame {
                        frames.entry(frame).or_insert((layer, cell));
                    }
                }
            }
            for (frame, (layer, cell)) in frames {
                if !atlas.frames.contains_key(frame) {
                    issue(
                        Some(frame.clone()),
                        &format!("layers/{layer}/cells/{cell}"),
                        Code::MissingFrame,
                        "referenced tile frame does not exist in the cooked atlas".into(),
                    );
                }
            }
        }
    }
    output
}
