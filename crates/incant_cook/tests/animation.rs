#![cfg(feature = "scene")]
#[path = "../../incant_assets/tests/skeletal/fixture.rs"]
mod fixture;
use incant_cook::{
    cook_scene, cook_scene_with_resources, load_project_animation_resources, stable_id,
};
use incant_doc::{Asset, Entity, Project, Scene, new_id};
use incant_runtime::NativeWorld;
use serde_json::json;
use std::{fs, path::Path};

fn project(root: &Path, animated: bool) -> (Project, String, String, String) {
    let mut input = fixture::fixture(root);
    if !animated {
        input.as_object_mut().unwrap().remove("animations");
        input.as_object_mut().unwrap().remove("skins");
        for node in input["nodes"].as_array_mut().unwrap() {
            node.as_object_mut().unwrap().remove("skin");
        }
        let attributes = input["meshes"][0]["primitives"][0]["attributes"]
            .as_object_mut()
            .unwrap();
        attributes.remove("JOINTS_0");
        attributes.remove("WEIGHTS_0");
        fixture::write(root, &input);
    }
    let cache = root.join(".incant/cache/models");
    let cooked = incant_assets::cook_gltf(root, Path::new("model.gltf"), &cache).unwrap();
    let asset = new_id();
    let mut p = Project::empty("Cook native animation");
    p.assets.insert(
        asset.clone(),
        Asset {
            id: asset.clone(),
            name: "Rig".into(),
            path: "model.gltf".into(),
            kind: "model".into(),
            sha256: cooked.metadata.fingerprint,
            import_settings: None,
        },
    );
    let mut scene = Scene::new("Main");
    let mut entity = Entity::new("Rig");
    entity.components.insert(
        "MeshRenderer".into(),
        json!({"mesh":asset,"materials":[],"cast_shadows":true}),
    );
    entity.components.insert(
        "Animation".into(),
        json!({"clip":0,"seconds":0.5,"speed":1,"playing":true,"looping":false}),
    );
    let eid = entity.id.clone();
    let sid = scene.id.clone();
    scene.entities.insert(eid.clone(), entity);
    p.scenes.insert(sid.clone(), scene);
    (p, sid, eid, asset)
}
#[test]
fn checked_cooked_asset_defines_clock_bounds_and_source_free_world() {
    let root = tempfile::tempdir().unwrap();
    let (p, sid, eid, _) = project(root.path(), true);
    let before = p.canonical_text().unwrap();
    fs::remove_file(root.path().join("model.gltf")).unwrap();
    fs::remove_file(root.path().join("model.bin")).unwrap();
    assert!(
        cook_scene(&p, &sid).is_err(),
        "unbound compatibility entry point must refuse"
    );
    let resources = load_project_animation_resources(&p, &sid, root.path()).unwrap();
    assert_eq!(resources.cache_paths().count(), 1);
    let bytes = cook_scene_with_resources(&p, &sid, &Default::default(), &resources).unwrap();
    assert_eq!(p.canonical_text().unwrap(), before);
    drop(p);
    drop(resources);
    root.close().unwrap();
    let mut world = NativeWorld::from_bytes(&bytes).unwrap();
    for _ in 0..30 {
        world.step().unwrap();
    }
    let state = world.animation_state(stable_id(&eid).unwrap()).unwrap();
    assert!((state.sample.seconds - 1.0).abs() < 1e-10);
    assert!(state.playing);
    let scene = world.spatial_view().unwrap();
    let node = scene
        .nodes()
        .find(|n| n.id == stable_id(&eid).unwrap())
        .unwrap();
    assert_eq!(
        &*node
            .declaration
            .unwrap()
            .animation
            .as_ref()
            .unwrap()
            .clip_durations,
        &[2.0, 2.0, 2.0]
    );
}
#[test]
fn missing_stale_static_and_out_of_range_animation_assets_fail_before_publication() {
    let root = tempfile::tempdir().unwrap();
    let (mut p, sid, eid, asset) = project(root.path(), true);
    let resources = load_project_animation_resources(&p, &sid, root.path()).unwrap();
    for change in [
        json!({"clip":3,"seconds":0,"speed":1,"playing":true,"looping":false}),
        json!({"clip":0,"seconds":2.01,"speed":1,"playing":true,"looping":false}),
    ] {
        p.scenes
            .get_mut(&sid)
            .unwrap()
            .entities
            .get_mut(&eid)
            .unwrap()
            .components
            .insert("Animation".into(), change);
        p.validate().unwrap();
        assert!(cook_scene_with_resources(&p, &sid, &Default::default(), &resources).is_err());
    }
    p.scenes
        .get_mut(&sid)
        .unwrap()
        .entities
        .get_mut(&eid)
        .unwrap()
        .components
        .get_mut("Animation")
        .unwrap()["seconds"] = json!(0);
    p.assets.get_mut(&asset).unwrap().sha256 = "00".repeat(32);
    assert!(cook_scene_with_resources(&p, &sid, &Default::default(), &resources).is_err());
    assert!(load_project_animation_resources(&p, &sid, root.path()).is_err());
    let other = tempfile::tempdir().unwrap();
    let (p, sid, _, _) = project(other.path(), false);
    assert!(load_project_animation_resources(&p, &sid, other.path()).is_err());
}
#[cfg(unix)]
#[test]
fn cache_directory_and_entry_symlinks_are_not_animation_capabilities() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    let (p, sid, _, _) = project(root.path(), true);
    let resources = load_project_animation_resources(&p, &sid, root.path()).unwrap();
    let path = resources.cache_paths().next().unwrap();
    let other = tempfile::tempdir().unwrap();
    let copied = other.path().join("model");
    fs::rename(path, &copied).unwrap();
    symlink(&copied, path).unwrap();
    assert!(load_project_animation_resources(&p, &sid, root.path()).is_err());
    fs::remove_file(path).unwrap();
    fs::rename(copied, path).unwrap();
    let cache = root.path().join(".incant/cache/models");
    let moved = other.path().join("models");
    fs::rename(&cache, &moved).unwrap();
    symlink(&moved, &cache).unwrap();
    assert!(load_project_animation_resources(&p, &sid, root.path()).is_err());
}
#[test]
fn no_animation_needs_no_cache_and_script_binding_uses_the_same_sealed_resources() {
    let mut empty = Project::empty("No assets");
    let s = Scene::new("Main");
    let sid = s.id.clone();
    empty.scenes.insert(sid.clone(), s);
    assert!(load_project_animation_resources(&empty, &sid, Path::new("/does-not-exist")).is_ok());
    #[cfg(feature = "script")]
    {
        let root = tempfile::tempdir().unwrap();
        let (mut p, sid, eid, _) = project(root.path(), true);
        let source = new_id();
        p.scripts.insert(
            source.clone(),
            incant_doc::ScriptSource {
                id: source.clone(),
                name: "Behavior".into(),
                path: "behavior.ts".into(),
            },
        );
        p.scenes
            .get_mut(&sid)
            .unwrap()
            .entities
            .get_mut(&eid)
            .unwrap()
            .components
            .insert("Script".into(), json!({"source":source,"props":{}}));
        assert!(incant_cook::prepare_script_scene(&p, &sid).is_err());
        let resources = load_project_animation_resources(&p, &sid, root.path()).unwrap();
        let prepared = incant_cook::prepare_script_scene_with_resources(
            &p,
            &sid,
            &Default::default(),
            &resources,
        )
        .unwrap();
        assert_eq!(prepared.source_catalog().as_array().unwrap().len(), 1);
    }
}

#[cfg(feature = "typescript")]
#[test]
fn animation_export_child() {
    // Child-only environment isolates HOST/TARGET/OUT_DIR from parallel tests.
    // No keychain, provider, network request or project native code is involved.
    if let Some(project) = std::env::var_os("INCANT_ANIMATION_EXPORT_TEST") {
        incant_cook::embed_project(Path::new(&project), None).unwrap();
    }
}
#[cfg(feature = "typescript")]
#[test]
fn actual_export_producer_binds_animations_and_tracks_cooked_cache_inputs() {
    for attached in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let (mut p, sid, eid, _) = project(root.path(), true);
        if attached {
            let source = new_id();
            p.scripts.insert(
                source.clone(),
                incant_doc::ScriptSource {
                    id: source.clone(),
                    name: "Idle".into(),
                    path: "idle.ts".into(),
                },
            );
            p.scenes
                .get_mut(&sid)
                .unwrap()
                .entities
                .get_mut(&eid)
                .unwrap()
                .components
                .insert("Script".into(), json!({"source":source,"props":{}}));
            fs::write(root.path().join("idle.ts"),"import type { NativeBehavior } from '@incant/runtime/native'; export default {update(){}} satisfies NativeBehavior;").unwrap();
        }
        let metadata = load_project_animation_resources(&p, &sid, root.path()).unwrap();
        let cache = metadata.cache_paths().next().unwrap().to_owned();
        let input = root.path().join("project.json");
        fs::write(&input, p.canonical_text().unwrap()).unwrap();
        fs::remove_file(root.path().join("model.gltf")).unwrap();
        fs::remove_file(root.path().join("model.bin")).unwrap();
        let output = root.path().join("out");
        fs::create_dir(&output).unwrap();
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "animation_export_child", "--nocapture"])
            .env("INCANT_ANIMATION_EXPORT_TEST", &input)
            .env("OUT_DIR", &output)
            .env("HOST", incant_script::artifact::TARGET)
            .env("TARGET", incant_script::artifact::TARGET)
            .env_remove("INCANT_NATIVE_SCRIPT_SOURCE")
            .env_remove("INCANT_NATIVE_SCRIPT_MAP")
            .env_remove("INCANT_NATIVE_SCRIPT_NAME")
            .output()
            .unwrap();
        assert!(
            child.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&child.stdout),
            String::from_utf8_lossy(&child.stderr)
        );
        assert!(
            String::from_utf8_lossy(&child.stdout)
                .contains(&format!("cargo:rerun-if-changed={}", cache.display()))
        );
        assert!(output.join("behavior.rs").is_file());
        let bytes = fs::read(output.join("world.scene")).unwrap();
        drop(metadata);
        drop(p);
        root.close().unwrap();
        let mut world = NativeWorld::from_bytes(&bytes).unwrap();
        for _ in 0..30 {
            world.step().unwrap();
        }
        assert!(
            (world
                .animation_state(stable_id(&eid).unwrap())
                .unwrap()
                .sample
                .seconds
                - 1.0)
                .abs()
                < 1e-10
        );
    }
}
