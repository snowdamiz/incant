use super::support::{Fixture, frame, rules};
use incant_assets::{CacheKind, project_cache_directory};
use incant_import::{
    MAX_TILEMAP_PAINT_CELLS, TilemapBrush as B, TilemapPaintError as E, TilemapPaintRequest,
    TilemapTerrainAction as A,
};
use serde_json::json;
use std::fs;

#[test]
fn strict_request_grammar_rejects_incomplete_rules_unknown_fields_and_invalid_actions() {
    let f = Fixture::new([1, 1]);
    let request = f.request(
        vec![[0, 0]],
        B::Autotile {
            action: A::Paint,
            frames: rules(),
        },
    );
    let value = serde_json::to_value(request).unwrap();
    for bad in [
        {
            let mut x = value.clone();
            x["root"] = json!("elsewhere");
            x
        },
        {
            let mut x = value.clone();
            x["brush"]["frames"] = json!([frame(0)]);
            x
        },
        {
            let mut x = value.clone();
            x["brush"]["frames"] = json!(vec![frame(0); 17]);
            x
        },
        {
            let mut x = value.clone();
            x["brush"]["action"] = json!("flood");
            x
        },
        {
            let mut x = value.clone();
            x["brush"]["ignored"] = json!(true);
            x
        },
        {
            let mut x = value.clone();
            x["cells"] = json!([[-1, 0]]);
            x
        },
        {
            let mut x = value.clone();
            x["cells"] = json!([[1.5, 0]]);
            x
        },
    ] {
        assert!(serde_json::from_value::<TilemapPaintRequest>(bad).is_err());
    }
}

#[test]
fn empty_oversized_duplicate_and_outside_cells_refuse_before_cooked_reads() {
    let f = Fixture::new([2, 2]);
    let before = f.bus.project().clone();
    let history = f.bus.history().to_vec();
    let cache = project_cache_directory(f.root.path(), CacheKind::Textures).unwrap();
    fs::remove_file(cache.join(format!(
        "{}.json",
        f.bus.project().assets[&f.texture].sha256
    )))
    .unwrap();
    for cells in [
        vec![],
        vec![[0, 0]; MAX_TILEMAP_PAINT_CELLS + 1],
        vec![[0, 0], [0, 0]],
        vec![[2, 0]],
        vec![[0, 2]],
        vec![[u32::MAX, u32::MAX]],
    ] {
        assert!(matches!(
            f.prepare(&f.request(cells, B::Erase)),
            Err(E::Cells)
        ));
    }
    assert_eq!(f.bus.project(), &before);
    assert_eq!(f.bus.history(), history);
}

#[test]
fn full_admissible_stroke_and_missing_brush_or_existing_map_frames_are_explicit() {
    let f = Fixture::new([64, 64]);
    let cells = (0..64).flat_map(|y| (0..64).map(move |x| [x, y])).collect();
    let p = f
        .prepare(&f.request(cells, B::Paint { frame: frame(0) }))
        .unwrap();
    assert_eq!(p.preview().changes.len(), MAX_TILEMAP_PAINT_CELLS);
    assert!(matches!(
        f.prepare(&f.request(
            vec![[0, 0]],
            B::Paint {
                frame: "invalid".into()
            }
        )),
        Err(E::Frame)
    ));
    let mut missing = rules();
    missing[15] = incant_doc::new_id();
    assert!(matches!(
        f.prepare(&f.request(
            vec![[0, 0]],
            B::Autotile {
                action: A::Paint,
                frames: missing
            }
        )),
        Err(E::Frame)
    ));
    let mut f = Fixture::new([1, 1]);
    f.set(vec![Some(incant_doc::new_id())]);
    assert!(
        matches!(f.prepare(&f.request(vec![[0, 0]], B::Erase)), Err(E::Frame)),
        "invalid existing binding cannot be hidden by an erase"
    );
}

#[test]
fn stable_selection_revision_and_foreign_terrain_refuse_without_editing() {
    let mut f = Fixture::new([1, 1]);
    f.set(vec![Some(frame(16))]);
    let request = f.request(
        vec![[0, 0]],
        B::Autotile {
            action: A::Erase,
            frames: rules(),
        },
    );
    assert!(matches!(f.prepare(&request), Err(E::ForeignTerrain)));
    for field in ["scene_id", "entity_id", "layer_id"] {
        let mut v = serde_json::to_value(&request).unwrap();
        v[field] = json!(incant_doc::new_id());
        assert!(matches!(
            f.prepare(&serde_json::from_value(v).unwrap()),
            Err(E::Selection)
        ));
    }
    let mut stale = request.clone();
    stale.expected_revision -= 1;
    assert!(matches!(f.prepare(&stale), Err(E::StaleProject)));
    // Ordinary erase explicitly permits removing foreign terrain.
    let p = f.prepare(&f.request(vec![[0, 0]], B::Erase)).unwrap();
    assert_eq!(p.preview().changes.len(), 1);
}

#[test]
fn corrupted_and_matching_digest_malformed_cooked_payloads_refuse_atomically() {
    for matching_digest in [false, true] {
        let f = Fixture::new([1, 1]);
        let before = f.bus.project().clone();
        let history = f.bus.history().to_vec();
        let request = f.request(vec![[0, 0]], B::Paint { frame: frame(0) });
        let preview = f.prepare(&request).unwrap();
        let d = preview.preview();
        let cache = project_cache_directory(f.root.path(), CacheKind::Textures).unwrap();
        let bytes = b"not a KTX2 header";
        if matching_digest {
            let p = cache.join(format!("{}.json", d.texture_key));
            let mut metadata: incant_assets::TextureMetadata =
                serde_json::from_slice(&fs::read(&p).unwrap()).unwrap();
            let hash = incant_assets::sha256(bytes);
            fs::write(cache.join(format!("{hash}.ktx2")), bytes).unwrap();
            metadata.content_sha256 = hash;
            assert_eq!(metadata.expected_key().unwrap(), d.texture_key);
            fs::write(p, serde_json::to_vec(&metadata).unwrap()).unwrap();
        } else {
            fs::write(
                cache.join(format!("{}.ktx2", d.texture_content_sha256)),
                bytes,
            )
            .unwrap();
        }
        let error = f.prepare(&request).err().unwrap();
        assert!(matches!(error, E::Cache(_)));
        assert!(error.to_string().contains(if matching_digest {
            "truncated KTX2 header"
        } else {
            "checksum mismatch"
        }));
        assert_eq!(f.bus.project(), &before);
        assert_eq!(f.bus.history(), history);
    }
}

#[test]
fn first_solid_cell_refuses_invalid_planar_geometry_before_returning_a_preview() {
    let mut f = Fixture::new([1, 1]);
    let before = f.map();
    assert!(before.layers[0].cells.iter().all(Option::is_none));
    f.bus
        .execute(
            vec![incant_cmd::Command::SetComponent {
                scene_id: f.scene.clone(),
                entity_id: f.entity.clone(),
                component: "Transform".into(),
                value: json!(incant_doc::Transform {
                    rotation: [1., 0., 0., 0.],
                    ..Default::default()
                }),
            }],
            incant_cmd::Actor::user("test"),
            "Nonplanar visual-only map",
            None,
        )
        .unwrap();
    let before = f.bus.project().clone();
    let revision = f.bus.revision();
    let history = f.bus.history().to_vec();
    assert!(matches!(
        f.prepare(&f.request(vec![[0, 0]], B::Paint { frame: frame(0) })),
        Err(E::Document(_))
    ));
    assert_eq!(f.bus.project(), &before);
    assert_eq!(f.bus.revision(), revision);
    assert_eq!(f.bus.history(), history);
}

#[test]
fn ordinary_texture_without_atlas_is_not_a_tileset() {
    let mut f = Fixture::new([1, 1]);
    let result = incant_import::ImportSnapshot::capture(&f.bus)
        .prepare(
            f.root.path(),
            None,
            &[incant_import::ImportRequest {
                source: "tile.png".into(),
                texture_usage: None,
            }],
        )
        .unwrap()
        .commit(
            &mut f.bus,
            incant_cmd::Actor::import("test"),
            "Ordinary image",
        )
        .unwrap();
    let mut map = f.map();
    map.tileset = result.imports[0].asset.id.clone();
    f.bus
        .execute(
            vec![incant_cmd::Command::SetComponent {
                scene_id: f.scene.clone(),
                entity_id: f.entity.clone(),
                component: "Tilemap".into(),
                value: json!(map),
            }],
            incant_cmd::Actor::user("test"),
            "Different texture",
            None,
        )
        .unwrap();
    assert!(matches!(
        f.prepare(&f.request(vec![[0, 0]], B::Erase)),
        Err(E::Atlas)
    ));
}
