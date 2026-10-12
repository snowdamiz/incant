#[path = "../../incant_assets/tests/skeletal/fixture.rs"]
mod fixture;
use incant_doc::{Asset, Entity, Project, Scene, ScriptSource, new_id};
use incant_play::{PreparedPlay, StartOptions};
use incant_script::CompiledScript;
use serde_json::json;
use std::{fs, path::Path};
#[test]
fn actual_editor_and_headless_startup_binds_animation_for_both_script_routes() {
    for authored in [false, true] {
        let root = tempfile::tempdir().unwrap();
        fixture::fixture(root.path());
        let model = incant_assets::cook_gltf(
            root.path(),
            Path::new("model.gltf"),
            &root.path().join(".incant/cache/models"),
        )
        .unwrap();
        let mut p = Project::empty("Prepared animation");
        p.settings.physics_mode = incant_runtime::PHYSICS_VARIANT.mode();
        let aid = new_id();
        p.assets.insert(
            aid.clone(),
            Asset {
                id: aid.clone(),
                name: "Rig".into(),
                path: "model.gltf".into(),
                kind: "model".into(),
                sha256: model.metadata.fingerprint,
                import_settings: None,
            },
        );
        let mut s = Scene::new("Main");
        let mut e = Entity::new("Camera and rig");
        let eid = e.id.clone();
        e.components.insert(
            "Camera".into(),
            json!({"fov_degrees":60,"near":0.1,"far":100}),
        );
        e.components.insert(
            "MeshRenderer".into(),
            json!({"mesh":aid,"materials":[],"cast_shadows":false}),
        );
        e.components.insert(
            "Animation".into(),
            json!({"clip":0,"seconds":0,"speed":1,"playing":true,"looping":false}),
        );
        if authored {
            let source = new_id();
            p.scripts.insert(
                source.clone(),
                ScriptSource {
                    id: source.clone(),
                    name: "Behavior".into(),
                    path: "behavior.ts".into(),
                },
            );
            e.components
                .insert("Script".into(), json!({"source":source,"props":{}}));
            fs::write(root.path().join("behavior.ts"),"import type { NativeBehavior } from '@incant/runtime/native'; export default { update() {} } satisfies NativeBehavior;").unwrap();
        }
        s.entities.insert(eid.clone(), e);
        p.scenes.insert(s.id.clone(), s);
        fs::remove_file(root.path().join("model.gltf")).unwrap();
        fs::remove_file(root.path().join("model.bin")).unwrap();
        let before = p.canonical_text().unwrap();
        let prepared = PreparedPlay::load(
            &p,
            root.path(),
            &StartOptions {
                camera: Some(eid.clone()),
                ..Default::default()
            },
            48000,
            || {
                assert!(!authored);
                CompiledScript::compile("exports.default={update(){}};")
            },
        )
        .unwrap();
        assert_eq!(prepared.authored_scripts, authored);
        assert!(prepared.render.is_some());
        assert_eq!(p.canonical_text().unwrap(), before);
        drop(p);
        root.close().unwrap();
        let mut game =
            incant_script::NativeSession::new(prepared.cooked, prepared.code, prepared.actions)
                .unwrap();
        for _ in 0..30 {
            game.tick(&[]).unwrap();
        }
        let id = incant_runtime::StableId(eid.parse::<ulid::Ulid>().unwrap().to_bytes());
        assert!(
            (game
                .world()
                .unwrap()
                .animation_state(id)
                .unwrap()
                .sample
                .seconds
                - 0.5)
                .abs()
                < 1e-10
        );
    }
}
