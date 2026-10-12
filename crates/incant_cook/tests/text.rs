#![cfg(feature = "scene")]
use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::{Entity, Project, Scene};
use incant_import::{ImportRequest, ImportSnapshot};
use incant_runtime::{CookedScene, NativeWorld};
use incant_text::{AtlasConfig, GlyphAtlas, TextEngine, TextStyle, Wrap};
use serde_json::json;
use std::fs;
const LATIN: &[u8] = include_bytes!("../../incant_text/tests/fonts/NotoSans-Regular.ttf");
const ARABIC: &[u8] = include_bytes!("../../incant_text/tests/fonts/NotoSansArabic.ttf");
const CJK: &[u8] = include_bytes!("../../incant_text/tests/fonts/IncantCJKFixture.ttf");
fn fixture(root: &std::path::Path) -> (CommandBus, String, String) {
    let mut bus = CommandBus::new(Project::empty("Text")).unwrap();
    let sources = [
        ("latin.ttf", LATIN),
        ("arabic.ttf", ARABIC),
        ("cjk.ttf", CJK),
    ];
    for (name, bytes) in sources {
        fs::write(root.join(name), bytes).unwrap();
    }
    let requests: Vec<_> = sources
        .iter()
        .map(|(name, _)| ImportRequest {
            source: (*name).into(),
            texture_usage: None,
        })
        .collect();
    let imported = ImportSnapshot::capture(&bus)
        .prepare(root, None, &requests)
        .unwrap()
        .commit(&mut bus, Actor::import("font fixture"), "Import fonts")
        .unwrap();
    let fonts: Vec<_> = imported
        .imports
        .iter()
        .map(|i| i.asset.id.clone())
        .collect();
    let scene = Scene::new("Main");
    let sid = scene.id.clone();
    let mut entity = Entity::new("Text");
    let eid = entity.id.clone();
    entity.components.insert("Text".into(),json!({"content":"ffi سلام 中文", "fonts":fonts,"font_size":20,"line_height":28,"width":500,"wrap":"word_or_glyph"}));
    bus.execute(
        vec![
            Command::CreateScene { scene },
            Command::CreateEntity {
                scene_id: sid.clone(),
                entity,
            },
        ],
        Actor::agent("test", "conversation"),
        "Create text",
        None,
    )
    .unwrap();
    (bus, sid, eid)
}
#[test]
fn public_import_command_cook_and_source_deleted_font_catalog_produce_real_glyph_pixels() {
    let temp = tempfile::tempdir().unwrap();
    let (mut bus, sid, eid) = fixture(temp.path());
    let authored = bus.project().canonical_text().unwrap();
    let bytes = incant_cook::cook_scene(bus.project(), &sid).unwrap();
    assert_eq!(u32::from_le_bytes(bytes[8..12].try_into().unwrap()), 12);
    bus.undo().unwrap();
    assert!(bus.project().scenes.is_empty());
    bus.redo().unwrap();
    assert_eq!(bus.project().canonical_text().unwrap(), authored);
    let scene = CookedScene::from_bytes(&bytes).unwrap();
    assert_eq!(
        scene.texts()[0].entity,
        incant_cook::stable_id(&eid).unwrap()
    );
    assert_eq!(scene.font_assets().len(), 3);
    for name in ["latin.ttf", "arabic.ttf", "cjk.ttf"] {
        fs::remove_file(temp.path().join(name)).unwrap();
    }
    drop(bus);
    let catalog = incant_assets_runtime::FontCatalog::load(
        &temp.path().join(".incant/cache/fonts"),
        scene.font_assets(),
    )
    .unwrap();
    assert_eq!(catalog.bytes(), LATIN.len() + ARABIC.len() + CJK.len());
    let mut fonts: std::collections::BTreeMap<_, _> = catalog.into_fonts().collect();
    let declaration = &scene.texts()[0].text;
    let sources = declaration
        .fonts
        .iter()
        .map(|id| {
            fonts
                .remove(id)
                .unwrap()
                .into_source(ulid::Ulid::from_bytes(id.0))
        })
        .collect();
    let mut engine = TextEngine::new(sources).unwrap();
    temp.close().unwrap(); // No source, project or cache remains for shaping/rasterization.
    let mut world = NativeWorld::new(scene);
    for _ in 0..3 {
        world.step().unwrap();
    }
    let view = world.text_view().unwrap();
    let (_, text) = view.nodes().next().unwrap();
    let shaped = engine
        .shape(
            &text.content,
            TextStyle {
                font_size: text.layout.font_size,
                line_height: text.layout.line_height,
                width: text.layout.width,
                wrap: Wrap::WordOrGlyph,
            },
        )
        .unwrap();
    assert!(shaped.missing_clusters().is_empty());
    assert!(shaped.glyphs().len() > 5);
    assert!(shaped.glyphs().iter().any(|g| g.bidi_level % 2 == 1));
    let mut atlas = GlyphAtlas::new(AtlasConfig {
        width: 512,
        height: 512,
        max_glyphs: 128,
    })
    .unwrap();
    for i in 0..shaped.glyphs().len() {
        engine.cache_glyph(&shaped, i, &mut atlas).unwrap();
    }
    assert!(atlas.pixels().iter().any(|&v| v > 0));
    assert!(atlas.len() > 5);
}
#[test]
fn invalid_font_edits_are_atomic_and_catalog_limit_precedes_blob_reads() {
    let temp = tempfile::tempdir().unwrap();
    let (mut bus, sid, eid) = fixture(temp.path());
    let before = bus.project().clone();
    let revision = bus.revision();
    let mut value = before.scenes[&sid].entities[&eid].components["Text"].clone();
    value["fonts"] = json!([incant_doc::new_id()]);
    assert!(
        bus.execute(
            vec![Command::SetComponent {
                scene_id: sid.clone(),
                entity_id: eid,
                component: "Text".into(),
                value
            }],
            Actor::user("test"),
            "Invalid text",
            None
        )
        .is_err()
    );
    assert_eq!(bus.project(), &before);
    assert_eq!(bus.revision(), revision);
    let bytes = incant_cook::cook_scene(&before, &sid).unwrap();
    let scene = CookedScene::from_bytes(&bytes).unwrap();
    let cache = temp.path().join(".incant/cache/fonts");
    // Recipe binds source content; byte count is independently checked. Bogus but
    // individually bounded declarations must trigger aggregate admission before
    // the missing blobs can be opened.
    for reference in scene.font_assets() {
        let key: String = reference
            .fingerprint
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let file = cache.join(format!("{key}.json"));
        let mut m: serde_json::Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
        m["bytes"] = json!(32 * 1024 * 1024);
        fs::write(file, serde_json::to_vec(&m).unwrap()).unwrap();
        fs::remove_file(cache.join(format!("{}.ifont", m["content_sha256"].as_str().unwrap())))
            .unwrap();
    }
    assert!(matches!(
        incant_assets_runtime::FontCatalog::load(&cache, scene.font_assets()),
        Err(incant_assets_runtime::AssetError::Limit("total font bytes"))
    ));
}

#[cfg(feature = "typescript")]
#[test]
fn text_export_producer_child() {
    if let Some(path) = std::env::var_os("INCANT_TEXT_EXPORT_TEST") {
        incant_cook::embed_project(std::path::Path::new(&path), None).unwrap();
    }
}
#[cfg(feature = "typescript")]
#[test]
fn text_packaged_consumer_child() {
    if let Some(path) = std::env::var_os("INCANT_TEXT_PACKAGE_TEST") {
        let path = std::path::PathBuf::from(path);
        let scene = CookedScene::from_bytes(&fs::read(path.join("world.scene")).unwrap()).unwrap();
        let fonts =
            incant_assets_runtime::FontCatalog::load(&path.join("fonts"), scene.font_assets())
                .unwrap();
        let mut fonts: std::collections::BTreeMap<_, _> = fonts.into_fonts().collect();
        let text = &scene.texts()[0].text;
        let mut engine = TextEngine::new(
            text.fonts
                .iter()
                .map(|id| {
                    fonts
                        .remove(id)
                        .unwrap()
                        .into_source(ulid::Ulid::from_bytes(id.0))
                })
                .collect(),
        )
        .unwrap();
        let shape = engine
            .shape(
                &text.content,
                TextStyle {
                    font_size: text.layout.font_size,
                    line_height: text.layout.line_height,
                    width: text.layout.width,
                    wrap: Wrap::WordOrGlyph,
                },
            )
            .unwrap();
        assert!(shape.missing_clusters().is_empty());
        assert!(shape.glyphs().iter().any(|g| g.bidi_level % 2 == 1));
        let mut atlas = GlyphAtlas::new(AtlasConfig {
            width: 512,
            height: 512,
            max_glyphs: 128,
        })
        .unwrap();
        for n in 0..shape.glyphs().len() {
            engine.cache_glyph(&shape, n, &mut atlas).unwrap();
        }
        assert!(atlas.pixels().iter().any(|&b| b != 0));
        println!(
            "source-free packaged text: {} glyphs, {} atlas entries",
            shape.glyphs().len(),
            atlas.len()
        );
    }
}
#[cfg(feature = "typescript")]
#[test]
fn actual_public_producer_and_packager_feed_fresh_source_deleted_numeric_consumer() {
    let root = tempfile::tempdir().unwrap();
    let root_path = root.path().canonicalize().unwrap();
    let (bus, _, _) = fixture(&root_path);
    let project = root_path.join("project.json");
    fs::write(&project, bus.project().canonical_text().unwrap()).unwrap();
    drop(bus);
    for name in ["latin.ttf", "arabic.ttf", "cjk.ttf"] {
        fs::remove_file(root_path.join(name)).unwrap();
    }
    let producer = root_path.join("build");
    fs::create_dir(&producer).unwrap();
    let exe = std::env::current_exe().unwrap();
    let result = std::process::Command::new(&exe)
        .args(["--exact", "text_export_producer_child", "--nocapture"])
        .env("INCANT_TEXT_EXPORT_TEST", &project)
        .env("OUT_DIR", &producer)
        .env("HOST", incant_script::artifact::TARGET)
        .env("TARGET", incant_script::artifact::TARGET)
        .env_remove("INCANT_NATIVE_SCRIPT_SOURCE")
        .env_remove("INCANT_NATIVE_SCRIPT_MAP")
        .env_remove("INCANT_NATIVE_SCRIPT_NAME")
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(producer.join("project-export.json")).unwrap()).unwrap();
    assert_eq!(manifest["font_assets"].as_array().unwrap().len(), 3);
    assert_eq!(manifest["startup_source_parsing"], false);
    let packaged = tempfile::tempdir().unwrap();
    let output = packaged.path().canonicalize().unwrap().join("data");
    let engine = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let result=std::process::Command::new(if cfg!(windows){"python"}else{"python3"})
  .args(["-c","import json,sys; from pathlib import Path; sys.path.insert(0,str(Path(sys.argv[1])/'tools')); from native_package import data_package; data_package(Path(sys.argv[2]),Path(sys.argv[3]),json.loads((Path(sys.argv[3])/'project-export.json').read_text()),Path(sys.argv[4]),None)"])
  .arg(engine).arg(&root_path).arg(&producer).arg(&output).output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    root.close().unwrap(); // source, authoring project, cache and producer deleted.
    let result = std::process::Command::new(&exe)
        .args(["--exact", "text_packaged_consumer_child", "--nocapture"])
        .env("INCANT_TEXT_PACKAGE_TEST", &output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(String::from_utf8_lossy(&result.stdout).contains("source-free packaged text:"));
}
