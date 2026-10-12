#![cfg(feature = "scene")]
#[path = "support/scene_format.rs"]
mod format;
use format::scene_format;
#[path = "../../incant_assets/tests/skeletal/fixture.rs"]
mod fixture;
use incant_cmd::{Actor, Command, CommandBus};
use incant_cook::{load_project_root_motion_resources, stable_id};
use incant_doc::{Asset, Entity, Project, Scene, new_id};
use serde_json::{Value, json};
use std::{fs, path::Path, sync::Arc};

fn project(root: &Path, edit: impl FnOnce(&mut Value)) -> (Project, String, String, String) {
    let mut input = fixture::fixture(root);
    input["animations"].as_array_mut().unwrap().truncate(2);
    edit(&mut input);
    fixture::write(root, &input);
    let captured = incant_assets::cook_gltf(
        root,
        Path::new("model.gltf"),
        &root.join(".incant/cache/models"),
    )
    .unwrap();
    let aid = new_id();
    let mut p = Project::empty("Root catalogs");
    p.assets.insert(
        aid.clone(),
        Asset {
            id: aid.clone(),
            name: "Rig".into(),
            path: "model.gltf".into(),
            kind: "model".into(),
            sha256: captured.metadata.fingerprint,
            import_settings: None,
        },
    );
    let mut scene = Scene::new("Main");
    let mut entity = Entity::new("Rig");
    entity.components.insert(
        "MeshRenderer".into(),
        json!({"mesh":aid,"materials":[],"cast_shadows":false}),
    );
    entity.components.insert(
        "Animation".into(),
        json!({"clip":0,"seconds":0,"speed":1,"playing":true,"looping":true}),
    );
    let sid = scene.id.clone();
    let eid = entity.id.clone();
    scene.entities.insert(eid.clone(), entity);
    p.scenes.insert(sid.clone(), scene);
    (p, sid, eid, aid)
}
fn binding(p: &Project, asset: &str, node: u16) -> Value {
    json!({"model":asset,"fingerprint":p.assets[asset].sha256,"root":node,"application":"extract_only","applying":false})
}
fn set(scene: &str, entity: &str, value: Value) -> Command {
    Command::SetComponent {
        scene_id: scene.into(),
        entity_id: entity.into(),
        component: "AnimationRootMotion".into(),
        value,
    }
}

#[test]
fn public_root_binding_is_atomic_reversible_and_reopens_from_its_journal() {
    let root = tempfile::tempdir().unwrap();
    let (p, sid, eid, aid) = project(root.path(), |_| {});
    let journal = root.path().join("journal");
    let seed = p.clone();
    let value = binding(&p, &aid, 1);
    let mut bus = CommandBus::persistent(&journal, p).unwrap();
    let before = bus.project().canonical_text().unwrap();
    for variant in 0..5 {
        let mut bad = value.clone();
        match variant {
            0 => bad["fingerprint"] = json!("01".repeat(32)),
            1 => bad["root"] = json!(4096),
            2 => bad["application"] = json!("dynamic_force"),
            3 => bad["model"] = json!(new_id()),
            _ => bad["unknown"] = json!(1),
        }
        assert!(
            bus.execute(
                vec![set(&sid, &eid, value.clone()), set(&sid, &eid, bad)],
                Actor::agent("root-test", "fixture"),
                "invalid root binding",
                None
            )
            .is_err()
        );
        assert_eq!(bus.project().canonical_text().unwrap(), before);
        assert_eq!(bus.revision(), 0);
    }
    bus.execute(
        vec![set(&sid, &eid, value)],
        Actor::agent("root-test", "fixture"),
        "select root",
        None,
    )
    .unwrap();
    let after = bus.project().canonical_text().unwrap();
    assert_eq!(
        bus.project().scenes[&sid].entities[&eid]
            .provenance
            .as_ref()
            .unwrap()
            .model
            .as_deref(),
        Some("root-test")
    );
    bus.undo().unwrap();
    assert_eq!(bus.project().canonical_text().unwrap(), before);
    bus.redo().unwrap();
    assert_eq!(bus.project().canonical_text().unwrap(), after);
    drop(bus);
    let reopened = CommandBus::persistent(&journal, seed).unwrap();
    assert_eq!(reopened.project().canonical_text().unwrap(), after);
}

#[test]
fn captured_catalogs_survive_source_cache_and_project_deletion_and_share_data() {
    let root = tempfile::tempdir().unwrap();
    let (mut p, sid, eid, aid) = project(root.path(), |_| {});
    let value = binding(&p, &aid, 1);
    p.scenes
        .get_mut(&sid)
        .unwrap()
        .entities
        .get_mut(&eid)
        .unwrap()
        .components
        .insert("AnimationRootMotion".into(), value);
    let mut second = p.scenes[&sid].entities[&eid].clone();
    second.id = new_id();
    let second_id = second.id.clone();
    p.scenes
        .get_mut(&sid)
        .unwrap()
        .entities
        .insert(second_id.clone(), second);
    fs::remove_file(root.path().join("model.gltf")).unwrap();
    fs::remove_file(root.path().join("model.bin")).unwrap();
    let before = p.canonical_text().unwrap();
    let resources = load_project_root_motion_resources(&p, &sid, root.path()).unwrap();
    assert_eq!(before, p.canonical_text().unwrap());
    let catalog = Arc::clone(resources.catalog(stable_id(&eid).unwrap()).unwrap());
    assert!(Arc::ptr_eq(
        &catalog,
        resources.catalog(stable_id(&second_id).unwrap()).unwrap()
    ));
    assert_eq!(resources.retained_catalog_bytes(), catalog.retained_bytes());
    assert_eq!(resources.cache_paths().count(), 1);
    assert_eq!(catalog.model_reference().translation, [11., 0., 0.]);
    assert_eq!(catalog.clips().count(), 2);
    let bytes = catalog.to_bytes().unwrap();
    assert_eq!(
        catalog.as_ref(),
        &incant_anim::RootMotionCatalog::from_bytes(&bytes).unwrap()
    );
    drop(catalog);
    drop(resources);
    drop(p);
    root.close().unwrap();
    // Reopen only the root-only payload after both the source and captured
    // model cache are gone; no retained full model or project is available.
    let catalog = incant_anim::RootMotionCatalog::from_bytes(&bytes).unwrap();
    let delta = catalog
        .sampler(0)
        .unwrap()
        .loop_delta(
            incant_anim::RootMotionTime {
                cycle: 7,
                seconds: 1.5,
            },
            incant_anim::RootMotionTime {
                cycle: 9,
                seconds: 0.5,
            },
        )
        .unwrap();
    assert_eq!(delta.translation, [3., 0., 0.]);
    assert_eq!(delta.rotation, [0., 0., 0., 1.]);
}

#[test]
fn unsupported_topology_and_malformed_captures_refuse() {
    for variant in 0..4 {
        let root = tempfile::tempdir().unwrap();
        let (mut p, sid, eid, aid) = project(root.path(), |doc| match variant {
            0 => doc["animations"][0]["channels"][0]["target"]["node"] = json!(0),
            1 => doc["nodes"][0]["scale"] = json!([2, 2, 2]),
            _ => {}
        });
        let node = if variant == 2 { 2 } else { 1 };
        let value = binding(&p, &aid, node);
        p.scenes
            .get_mut(&sid)
            .unwrap()
            .entities
            .get_mut(&eid)
            .unwrap()
            .components
            .insert("AnimationRootMotion".into(), value);
        if variant == 3 {
            let path = root
                .path()
                .join(".incant/cache/models")
                .join(format!("{}.incmodel", p.assets[&aid].sha256));
            let mut bytes = fs::read(&path).unwrap();
            let last = bytes.len() - 1;
            bytes[last] ^= 1;
            fs::write(path, bytes).unwrap();
        }
        assert!(
            load_project_root_motion_resources(&p, &sid, root.path()).is_err(),
            "{variant}"
        );
    }
}

#[cfg(unix)]
#[test]
fn redirected_root_catalog_cache_and_entries_are_not_capabilities() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    let (mut p, sid, eid, aid) = project(root.path(), |_| {});
    let value = binding(&p, &aid, 1);
    p.scenes
        .get_mut(&sid)
        .unwrap()
        .entities
        .get_mut(&eid)
        .unwrap()
        .components
        .insert("AnimationRootMotion".into(), value);
    let resources = load_project_root_motion_resources(&p, &sid, root.path()).unwrap();
    let original = resources.cache_paths().next().unwrap();
    let moved = other.path().join("model");
    fs::rename(original, &moved).unwrap();
    symlink(&moved, original).unwrap();
    assert!(load_project_root_motion_resources(&p, &sid, root.path()).is_err());
    fs::remove_file(original).unwrap();
    fs::rename(&moved, original).unwrap();
    let cache = root.path().join(".incant/cache/models");
    fs::rename(&cache, &moved).unwrap();
    symlink(&moved, &cache).unwrap();
    assert!(load_project_root_motion_resources(&p, &sid, root.path()).is_err());
}

#[test]
fn ordinary_scene_cook_embeds_root_catalog_and_rejects_stale_or_malformed_bindings() {
    use incant_cook::{cook_scene_with_resources, load_project_animation_resources};
    use incant_runtime::CookedScene;
    use sha2::{Digest, Sha256};
    let root = tempfile::tempdir().unwrap();
    let (mut p, sid, eid, aid) = project(root.path(), |_| {});
    let value = binding(&p, &aid, 1);
    p.scenes
        .get_mut(&sid)
        .unwrap()
        .entities
        .get_mut(&eid)
        .unwrap()
        .components
        .insert("AnimationRootMotion".into(), value);
    let animations = load_project_animation_resources(&p, &sid, root.path()).unwrap();
    assert!(incant_cook::cook_scene(&p, &sid).is_err()); // Captured resources are required.
    let bytes = cook_scene_with_resources(&p, &sid, &Default::default(), &animations).unwrap();
    assert_eq!(&bytes[8..12], &scene_format(17).to_le_bytes());
    let catalog_size = load_project_root_motion_resources(&p, &sid, root.path())
        .unwrap()
        .catalog(stable_id(&eid).unwrap())
        .unwrap()
        .to_bytes()
        .unwrap()
        .len();
    let original = p.canonical_text().unwrap();
    p.scenes
        .get_mut(&sid)
        .unwrap()
        .entities
        .get_mut(&eid)
        .unwrap()
        .components
        .get_mut("AnimationRootMotion")
        .unwrap()["root"] = json!(0);
    assert!(cook_scene_with_resources(&p, &sid, &Default::default(), &animations).is_err());
    assert_ne!(p.canonical_text().unwrap(), original);
    drop(animations);
    drop(p);
    root.close().unwrap();
    let cooked = CookedScene::from_bytes(&bytes).unwrap();
    assert_eq!(cooked.to_bytes(), bytes);
    let catalog = &cooked.spatial()[0].root_motion.as_ref().unwrap().catalog;
    assert_eq!(
        catalog
            .sampler(0)
            .unwrap()
            .clamped_delta(0.0, 1.0)
            .unwrap()
            .translation,
        [1.0, 0.0, 0.0]
    );
    let payload = bytes.windows(8).position(|v| v == b"INCROOT\0").unwrap();
    let binding = payload + catalog_size + 4;
    for case in 0..10 {
        let mut bad = bytes.clone();
        match case {
            0 => bad[payload - 8..payload - 4].copy_from_slice(&0u32.to_le_bytes()),
            1 => bad[payload - 8..payload - 4].copy_from_slice(&1025u32.to_le_bytes()),
            2 => bad[payload - 4..payload].copy_from_slice(&u32::MAX.to_le_bytes()),
            3 => bad[payload + 12] ^= 1, // Fingerprint must match the animation.
            4 => bad[payload + 132..payload + 136].copy_from_slice(&0u32.to_le_bytes()),
            5 => bad[binding] ^= 1, // Unknown owner.
            6 => bad[binding + 16..binding + 20].copy_from_slice(&1u32.to_le_bytes()),
            7 => bad[binding + 20] = 4,
            8 => bad[binding + 21] = 2,
            _ => bad[binding + 22] = 1,
        }
        let mut hash = Sha256::new();
        hash.update(&bad[..40]);
        hash.update(&bad[72..]);
        bad[40..72].copy_from_slice(&hash.finalize());
        assert!(CookedScene::from_bytes(&bad).is_err(), "case {case}");
    }
    assert_eq!(CookedScene::from_bytes(&bytes).unwrap().to_bytes(), bytes);
}
