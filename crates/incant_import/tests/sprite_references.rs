use image::{Rgba, RgbaImage};
use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::{Entity, Project, Scene, Transform};
use incant_import::{
    ImportError, ImportRequest, ImportSnapshot, SourceWatcher, SpriteDiagnosticCode as Code,
    SpriteDiagnosticSeverity as Severity, validate_cached_sprites,
};
use serde_json::{Value, json};
use std::{
    fs,
    path::Path,
    time::{Duration, Instant},
};

const A: &str = "00000000000000000000000001";
const B: &str = "00000000000000000000000002";
fn request(source: &str) -> ImportRequest {
    ImportRequest {
        source: source.into(),
        texture_usage: None,
    }
}
fn image(root: &Path, name: &str, width: u32, color: u8) {
    RgbaImage::from_pixel(width, 8, Rgba([color, 0, 0, 255]))
        .save(root.join(name))
        .unwrap();
}
fn descriptor() -> Value {
    json!({"version":1,"id":"00000000000000000000000009","maximum_size":[64,64],"padding":2,
        "frames":{(A):{"name":"Panel","path":"first.png"},(B):{"name":"Other","path":"second.png"}}})
}
fn write_descriptor(root: &Path, value: &Value) {
    fs::write(root.join("sheet.atlas.json"), value.to_string()).unwrap();
}
struct Fixture {
    root: tempfile::TempDir,
    bus: CommandBus,
    scene: String,
    atlas: String,
    ordinary: String,
}
impl Fixture {
    fn tilemap(&mut self, texture: String, frame: &str) -> String {
        let mut entity = Entity::new("Referenced tilemap");
        let id = entity.id.clone();
        entity
            .components
            .insert("Transform".into(), json!(Transform::default()));
        entity.components.insert("Tilemap".into(), json!({"tileset":texture,"size":[2,2],"cell_size":[1,1],"layers":[{"id":B,"cells":[frame,frame,null,frame],"solid":true}]}));
        self.bus
            .execute(
                vec![Command::CreateEntity {
                    scene_id: self.scene.clone(),
                    entity,
                }],
                Actor::user("test"),
                "Author tilemap",
                None,
            )
            .unwrap();
        id
    }
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        image(root.path(), "first.png", 8, 255);
        image(root.path(), "second.png", 8, 128);
        image(root.path(), "ordinary.png", 8, 32);
        write_descriptor(root.path(), &descriptor());
        let mut bus = CommandBus::persistent(
            root.path().join("history.jsonl"),
            Project::empty("Sprite references"),
        )
        .unwrap();
        let imports = ImportSnapshot::capture(&bus)
            .prepare(
                root.path(),
                None,
                &[request("sheet.atlas.json"), request("ordinary.png")],
            )
            .unwrap()
            .commit(&mut bus, Actor::import("test"), "Import textures")
            .unwrap();
        let atlas = imports.imports[0].asset.id.clone();
        let ordinary = imports.imports[1].asset.id.clone();
        let scene = Scene::new("Main");
        let id = scene.id.clone();
        bus.execute(
            vec![Command::CreateScene { scene }],
            Actor::user("test"),
            "Create scene",
            None,
        )
        .unwrap();
        Self {
            root,
            bus,
            scene: id,
            atlas,
            ordinary,
        }
    }
    fn sprite(&mut self, texture: String, region: Value, nine_slice: bool) -> String {
        let mut entity = Entity::new("Referenced sprite");
        let id = entity.id.clone();
        entity
            .components
            .insert("Transform".into(), json!(Transform::default()));
        let mut value = json!({"texture":texture,"region":region,"size":[1,1],"pivot":[0.5,0.5],"tint":[1,1,1,1]});
        if nine_slice {
            value["mode"] = json!({"kind":"nine_slice","border":[4,4,4,4],"pixels_per_unit":8});
        }
        entity.components.insert("SpriteRenderer".into(), value);
        self.bus
            .execute(
                vec![Command::CreateEntity {
                    scene_id: self.scene.clone(),
                    entity,
                }],
                Actor::user("test"),
                "Author sprite",
                None,
            )
            .unwrap();
        id
    }
    fn prepare(&self) -> Result<incant_import::PreparedImports, ImportError> {
        ImportSnapshot::capture(&self.bus).prepare(
            self.root.path(),
            None,
            &[request("sheet.atlas.json"), request("ordinary.png")],
        )
    }
    fn journal_bytes(&mut self) -> Vec<u8> {
        let project = self.bus.project().clone();
        let revision = self.bus.revision();
        // Windows denies independent reads while the persistent bus holds its lock.
        // Release it, inspect the exact bytes, then verify the replayed state.
        drop(std::mem::replace(
            &mut self.bus,
            CommandBus::new(project.clone()).unwrap(),
        ));
        let path = self.root.path().join("history.jsonl");
        let bytes = fs::read(&path).unwrap();
        self.bus = CommandBus::persistent(path, project.clone()).unwrap();
        assert_eq!(self.bus.project(), &project);
        assert_eq!(self.bus.revision(), revision);
        bytes
    }
}

#[test]
fn shared_tile_frames_reject_reimport_atomically_and_cached_validation_survives_undo_redo() {
    let mut f = Fixture::new();
    let first = f.tilemap(f.atlas.clone(), A);
    let second = f.tilemap(f.atlas.clone(), A);
    let sprite = f.sprite(f.atlas.clone(), json!({"frame":A}), false);
    let before = f.bus.project().clone();
    let revision = f.bus.revision();
    let journal = f.journal_bytes();
    image(f.root.path(), "ordinary.png", 8, 99);
    let mut dropped = descriptor();
    dropped["frames"].as_object_mut().unwrap().remove(A);
    write_descriptor(f.root.path(), &dropped);
    let ImportError::SpriteReferences(errors) = f.prepare().err().unwrap() else {
        panic!("typed references")
    };
    assert_eq!(errors.0.len(), 3); // Shared cells are grouped per entity/texture/frame.
    for entity in [&first, &second, &sprite] {
        let diagnostic = errors.0.iter().find(|d| &d.entity_id == entity).unwrap();
        assert_eq!(diagnostic.code, Code::MissingFrame);
        assert_eq!(diagnostic.texture_id, f.atlas);
        assert_eq!(diagnostic.frame.as_deref(), Some(A));
        if entity != &sprite {
            assert!(diagnostic.path.ends_with("/Tilemap/layers/0/cells/0"));
        }
    }
    assert_eq!(f.bus.project(), &before);
    assert_eq!(f.bus.revision(), revision);
    assert_eq!(f.journal_bytes(), journal);
    assert!(
        validate_cached_sprites(f.bus.project(), f.root.path(), None)
            .unwrap()
            .is_empty()
    );
    write_descriptor(f.root.path(), &descriptor());
    image(f.root.path(), "first.png", 8, 17);
    f.prepare()
        .unwrap()
        .commit(&mut f.bus, Actor::import("test"), "Repair tile atlas")
        .unwrap();
    let after = f.bus.project().clone();
    assert_ne!(
        before.assets[&f.atlas].sha256,
        after.assets[&f.atlas].sha256
    );
    for source in [
        "first.png",
        "second.png",
        "ordinary.png",
        "sheet.atlas.json",
    ] {
        fs::remove_file(f.root.path().join(source)).unwrap();
    }
    assert!(
        validate_cached_sprites(f.bus.project(), f.root.path(), None)
            .unwrap()
            .is_empty()
    );
    f.bus.undo().unwrap();
    assert_eq!(f.bus.project(), &before);
    assert!(
        validate_cached_sprites(f.bus.project(), f.root.path(), None)
            .unwrap()
            .is_empty()
    );
    f.bus.redo().unwrap();
    assert_eq!(f.bus.project(), &after);
    assert!(
        validate_cached_sprites(f.bus.project(), f.root.path(), None)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn ordinary_tile_textures_and_unknown_frames_have_typed_offline_diagnostics() {
    let mut f = Fixture::new();
    let ordinary = f.tilemap(f.ordinary.clone(), A);
    let unknown = f.tilemap(f.atlas.clone(), "00000000000000000000000003");
    f.bus.project().validate().unwrap(); // No filesystem IO in schema validation.
    for source in [
        "first.png",
        "second.png",
        "ordinary.png",
        "sheet.atlas.json",
    ] {
        fs::remove_file(f.root.path().join(source)).unwrap();
    }
    let diagnostics = validate_cached_sprites(f.bus.project(), f.root.path(), None).unwrap();
    assert_eq!(diagnostics.len(), 2);
    for (entity, code) in [(ordinary, Code::NotAtlas), (unknown, Code::MissingFrame)] {
        let diagnostic = diagnostics.iter().find(|d| d.entity_id == entity).unwrap();
        assert_eq!(diagnostic.code, code);
        assert_eq!(diagnostic.severity, Severity::Error);
        assert!(diagnostic.path.contains("/Tilemap/"));
    }
}

#[test]
fn broken_shared_frames_and_shrunken_slices_reject_the_entire_import_batch() {
    let mut f = Fixture::new();
    let first = f.sprite(f.atlas.clone(), json!({"frame":A}), false);
    let sliced = f.sprite(f.atlas.clone(), json!({"frame":A}), true);
    let before = f.bus.project().clone();
    let revision = f.bus.revision();
    let journal = f.journal_bytes();
    image(f.root.path(), "ordinary.png", 8, 99); // an unrelated successful cook in the same batch
    let mut dropped = descriptor();
    dropped["frames"].as_object_mut().unwrap().remove(A);
    write_descriptor(f.root.path(), &dropped);
    let error = f.prepare().err().unwrap();
    assert!(error.to_string().contains(&first) && error.to_string().contains(&sliced));
    let ImportError::SpriteReferences(errors) = error else {
        panic!("expected typed sprite references")
    };
    assert_eq!(errors.0.len(), 2);
    assert!(errors.0.iter().all(|d| d.code == Code::MissingFrame
        && d.texture_id == f.atlas
        && d.frame.as_deref() == Some(A)));
    let mut shrunk = descriptor();
    shrunk["frames"][A]["region"] = json!([0, 0, 4, 4]);
    write_descriptor(f.root.path(), &shrunk);
    let ImportError::SpriteReferences(errors) = f.prepare().err().unwrap() else {
        panic!("expected slice error")
    };
    assert_eq!(errors.0.len(), 1);
    assert_eq!(errors.0[0].code, Code::NineSliceBounds);
    assert_eq!(errors.0[0].entity_id, sliced);
    assert_eq!(f.bus.project(), &before);
    assert_eq!(f.bus.revision(), revision);
    assert_eq!(f.journal_bytes(), journal);
    assert!(
        validate_cached_sprites(f.bus.project(), f.root.path(), None)
            .unwrap()
            .is_empty()
    );

    // Repairing sources publishes once; Undo/Redo recover the exact cooked bindings.
    write_descriptor(f.root.path(), &descriptor());
    image(f.root.path(), "first.png", 8, 17);
    f.prepare()
        .unwrap()
        .commit(&mut f.bus, Actor::import("test"), "Repair source changes")
        .unwrap();
    let after = f.bus.project().clone();
    assert_ne!(
        after.assets[&f.atlas].sha256,
        before.assets[&f.atlas].sha256
    );
    assert_ne!(
        after.assets[&f.ordinary].sha256,
        before.assets[&f.ordinary].sha256
    );
    for source in [
        "first.png",
        "second.png",
        "ordinary.png",
        "sheet.atlas.json",
    ] {
        fs::remove_file(f.root.path().join(source)).unwrap();
    }
    assert!(
        validate_cached_sprites(f.bus.project(), f.root.path(), None)
            .unwrap()
            .is_empty()
    );
    f.bus.undo().unwrap();
    assert_eq!(f.bus.project(), &before);
    assert!(
        validate_cached_sprites(f.bus.project(), f.root.path(), None)
            .unwrap()
            .is_empty()
    );
    f.bus.redo().unwrap();
    assert_eq!(f.bus.project(), &after);
    assert!(
        validate_cached_sprites(f.bus.project(), f.root.path(), None)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn watcher_keeps_previous_asset_and_reports_then_clears_affected_entities() {
    let mut f = Fixture::new();
    let entity = f.sprite(f.atlas.clone(), json!({"frame":A}), false);
    let before = f.bus.project().clone();
    let mut watcher = SourceWatcher::new(f.root.path(), Duration::ZERO);
    assert!(
        watcher
            .poll(&mut f.bus, Instant::now())
            .unwrap()
            .imports
            .is_empty()
    );
    let mut dropped = descriptor();
    dropped["frames"].as_object_mut().unwrap().remove(A);
    write_descriptor(f.root.path(), &dropped);
    let report = watcher.poll(&mut f.bus, Instant::now()).unwrap();
    assert!(report.imports.is_empty());
    assert_eq!(report.diagnostics.len(), 1);
    assert_eq!(report.diagnostics[0].asset_id, f.atlas);
    assert!(report.diagnostics[0].message.contains(&entity));
    assert_eq!(f.bus.project(), &before);
    write_descriptor(f.root.path(), &descriptor());
    let repaired = watcher.poll(&mut f.bus, Instant::now()).unwrap();
    assert_eq!(repaired.cleared, vec![f.atlas]);
    assert!(repaired.imports.is_empty());
}

#[test]
fn cached_validation_identifies_ordinary_texture_frames_missing_frames_and_raw_rectangles() {
    let mut f = Fixture::new();
    let ordinary = f.sprite(f.ordinary.clone(), json!({"frame":A}), false);
    let missing = f.sprite(
        f.atlas.clone(),
        json!({"frame":"00000000000000000000000003"}),
        false,
    );
    let outside = f.sprite(f.ordinary.clone(), json!([7, 7, 2, 2]), false);
    let raw = f.sprite(f.atlas.clone(), json!([0, 0, 1, 1]), false);
    f.bus.project().validate().unwrap(); // Document validation deliberately performs no filesystem IO.
    for source in [
        "first.png",
        "second.png",
        "ordinary.png",
        "sheet.atlas.json",
    ] {
        fs::remove_file(f.root.path().join(source)).unwrap();
    }
    let diagnostics = validate_cached_sprites(f.bus.project(), f.root.path(), None).unwrap();
    for (id, code, severity) in [
        (ordinary, Code::NotAtlas, Severity::Error),
        (missing, Code::MissingFrame, Severity::Error),
        (outside, Code::RegionBounds, Severity::Error),
        (raw, Code::RawAtlasRegion, Severity::Warning),
    ] {
        let diagnostic = diagnostics.iter().find(|d| d.entity_id == id).unwrap();
        assert_eq!(diagnostic.code, code);
        assert_eq!(diagnostic.severity, severity);
        assert!(diagnostic.path.contains(&id));
    }
    assert_eq!(diagnostics.len(), 4);
    let manifest = f
        .root
        .path()
        .join(".incant/cache/textures")
        .join(format!("{}.json", f.bus.project().assets[&f.atlas].sha256));
    fs::write(manifest, b"corrupt cache").unwrap();
    let diagnostics = validate_cached_sprites(f.bus.project(), f.root.path(), None).unwrap();
    assert_eq!(
        diagnostics
            .iter()
            .filter(|d| d.texture_id == f.atlas && d.code == Code::CacheUnavailable)
            .count(),
        2
    );
}

#[test]
fn raw_atlas_rectangles_warn_in_both_import_outcomes_and_source_free_validation() {
    let mut f = Fixture::new();
    let raw = f.sprite(f.atlas.clone(), json!([0, 0, 1, 1]), false);
    let revision = f.bus.revision();
    let committed = f
        .prepare()
        .unwrap()
        .commit(&mut f.bus, Actor::import("test"), "Unchanged import")
        .unwrap();
    assert_eq!(committed.revision, revision);
    let atlas = &committed.imports[0];
    assert!(!atlas.changed);
    assert_eq!(atlas.diagnostics.len(), 1);
    assert_eq!(atlas.diagnostics[0].entity_id, raw);
    assert_eq!(atlas.diagnostics[0].code, Code::RawAtlasRegion);
    assert_eq!(atlas.diagnostics[0].severity, Severity::Warning);
    let serialized = serde_json::to_value(atlas).unwrap();
    assert_eq!(serialized["diagnostics"][0]["code"], "raw_atlas_region");
    assert_eq!(
        validate_cached_sprites(f.bus.project(), f.root.path(), None).unwrap(),
        atlas.diagnostics
    );
}
