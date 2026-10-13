#![cfg(feature = "typescript")]

use incant_cook::{
    compile_project_sources, compile_project_sources_for_target, prepare_script_scene,
};
use incant_doc::{Entity, Project, Scene, ScriptSource, new_id};
use incant_script::artifact;
use serde_json::json;
use sha2::{Digest, Sha256};

fn project() -> (Project, String, String) {
    let mut project = Project::empty("Authored mobile binding");
    let mut scene = Scene::new("Main");
    let mut entity = Entity::new("Mover");
    let source = new_id();
    entity.components.insert(
        "Script".into(),
        json!({"source":source,"props":{"speed":3,"label":"語"}}),
    );
    let entity_id = entity.id.clone();
    scene.entities.insert(entity.id.clone(), entity);
    let scene_id = scene.id.clone();
    project.scenes.insert(scene_id.clone(), scene);
    project.scripts.insert(
        source.clone(),
        ScriptSource {
            id: source,
            name: "Mover".into(),
            path: "main.ts".into(),
        },
    );
    (project, scene_id, entity_id)
}

#[test]
fn fresh_project_build_preserves_native_bytecode_and_exact_scene_attachment_binding() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("main.ts"),
        r#"
import type { NativeBehavior } from '@incant/runtime/native';
import { multiplier } from './shared';
export default { update({instances,chunks,dt}) {
  for (const instance of instances) for (const chunk of chunks)
    for (let row=0;row<chunk.ids.length;row++) if (chunk.ids[row]===instance.entity)
      chunk.transforms[row*10]+=Number(instance.props.speed)*multiplier*dt;
} } satisfies NativeBehavior;
"#,
    )
    .unwrap();
    std::fs::write(root.path().join("shared.ts"), "export const multiplier=2;").unwrap();
    let (mut project, scene_id, entity_id) = project();
    let development = compile_project_sources(
        prepare_script_scene(&project, &scene_id).unwrap(),
        root.path(),
    )
    .unwrap();
    let (native_scene, native_script) = development.program.into_parts();
    let targets = [
        artifact::TARGET,
        "aarch64-apple-ios-sim",
        "aarch64-apple-ios",
        "aarch64-linux-android",
    ];
    for target in targets {
        let prepared = prepare_script_scene(&project, &scene_id).unwrap();
        if !artifact::compatibility::admits(
            artifact::TARGET,
            target,
            artifact::BytecodeAbi::current(),
        ) {
            assert!(
                prepared
                    .compile_for_target("exports.default={};", target)
                    .is_err()
            );
            continue;
        }
        let built = compile_project_sources_for_target(prepared, root.path(), target).unwrap();
        assert_eq!(built.compiler_manifest, development.compiler_manifest);
        let (scene, script) = built.program.into_parts();
        assert_eq!(scene.to_bytes(), native_scene.to_bytes());
        let m = script.metadata();
        assert_eq!(m.target, target);
        assert_eq!(m.producer_target, artifact::TARGET);
        assert_eq!(m.bytecode_sha256, native_script.metadata().bytecode_sha256);
        assert_eq!(
            m.hardening_sha256,
            native_script.metadata().hardening_sha256
        );
        assert_eq!(m.bindings_sha256, native_script.metadata().bindings_sha256);
        assert!(m.bindings_sha256.is_some());
        assert!(script.embedded_rust().contains("bindings: Some("));
    }
    // A fresh source graph is read each time, including transitive dependencies.
    std::fs::write(root.path().join("shared.ts"), "export const multiplier=5;").unwrap();
    let changed = compile_project_sources_for_target(
        prepare_script_scene(&project, &scene_id).unwrap(),
        root.path(),
        artifact::TARGET,
    )
    .unwrap();
    let (_, changed) = changed.program.into_parts();
    assert_ne!(
        changed.metadata().source_sha256,
        native_script.metadata().source_sha256
    );
    assert_eq!(
        changed.metadata().bindings_sha256,
        native_script.metadata().bindings_sha256
    );
    project
        .scenes
        .get_mut(&scene_id)
        .unwrap()
        .entities
        .get_mut(&entity_id)
        .unwrap()
        .components
        .get_mut("Script")
        .unwrap()["props"]["speed"] = json!(7);
    let rebound = compile_project_sources_for_target(
        prepare_script_scene(&project, &scene_id).unwrap(),
        root.path(),
        artifact::TARGET,
    )
    .unwrap();
    let (_, rebound) = rebound.program.into_parts();
    assert_ne!(
        rebound.metadata().bindings_sha256,
        changed.metadata().bindings_sha256
    );
    assert_eq!(
        rebound.metadata().bytecode_sha256,
        changed.metadata().bytecode_sha256
    );
}

#[test]
fn mobile_project_producer_child() {
    let Ok(mode) = std::env::var("INCANT_TEST_PROJECT_PRODUCER") else {
        return;
    };
    let path = std::env::var("INCANT_TEST_PROJECT_PATH").unwrap();
    let result = if mode == "mobile" {
        incant_cook::embed_mobile_project(std::path::Path::new(&path), None)
    } else {
        incant_cook::embed_project(std::path::Path::new(&path), None)
    };
    match std::env::var("INCANT_TEST_PROJECT_ERROR") {
        Ok(expected) => assert_eq!(result.unwrap_err().to_string(), expected),
        Err(_) => result.unwrap(),
    }
}

#[test]
fn explicit_mobile_producer_retains_desktop_and_unverified_destination_refusals() {
    let root = tempfile::tempdir().unwrap();
    let mut project = Project::empty("Empty authored mobile scene");
    let scene = Scene::new("Main");
    project.scenes.insert(scene.id.clone(), scene);
    let path = root.path().join("game.incant");
    std::fs::write(&path, project.canonical_text().unwrap()).unwrap();
    let mobile_error =
        "authored mobile project requires the reviewed macOS arm64 mobile bytecode route";
    let reviewed = artifact::TARGET == "aarch64-apple-darwin"
        && artifact::compatibility::admits(
            artifact::TARGET,
            "aarch64-apple-ios-sim",
            artifact::BytecodeAbi::current(),
        );
    for (index, (mode, target, error)) in [
        (
            "native",
            "aarch64-apple-ios-sim",
            Some("authored project export requires native host == target"),
        ),
        (
            "native",
            "aarch64-linux-android",
            Some("authored project export requires native host == target"),
        ),
        (
            "mobile",
            "aarch64-linux-android",
            (!reviewed).then_some(mobile_error),
        ),
        ("mobile", "x86_64-linux-android", Some(mobile_error)),
        ("mobile", "armv7-linux-androideabi", Some(mobile_error)),
        ("mobile", "aarch64-apple-darwin", Some(mobile_error)),
        (
            "mobile",
            "aarch64-apple-ios-sim",
            (!reviewed).then_some(mobile_error),
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let output = root.path().join(format!("out-{index}"));
        std::fs::create_dir(&output).unwrap();
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .args(["--exact", "mobile_project_producer_child", "--nocapture"])
            .env("INCANT_TEST_PROJECT_PRODUCER", mode)
            .env("INCANT_TEST_PROJECT_PATH", &path)
            .env("HOST", artifact::TARGET)
            .env("TARGET", target)
            .env("OUT_DIR", &output);
        for (key, _) in std::env::vars_os() {
            if key.to_string_lossy().starts_with("INCANT_NATIVE_") || key == "INCANT_WASM_BUNDLE" {
                command.env_remove(key);
            }
        }
        if let Some(error) = error {
            command.env("INCANT_TEST_PROJECT_ERROR", error);
        } else {
            command.env_remove("INCANT_TEST_PROJECT_ERROR");
        }
        let result = command.output().unwrap();
        assert!(
            result.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        if error.is_some() {
            assert_eq!(std::fs::read_dir(output).unwrap().count(), 0);
        } else {
            let receipt: serde_json::Value =
                serde_json::from_slice(&std::fs::read(output.join("project-export.json")).unwrap())
                    .unwrap();
            assert_eq!(
                receipt["producer_kind"],
                if target == "aarch64-linux-android" {
                    "android-archive"
                } else {
                    "ios-archive"
                }
            );
            assert_eq!(receipt["producer_target"], artifact::TARGET);
            assert_eq!(receipt["script"]["target"], target);
            let scene = std::fs::read(output.join("world.scene")).unwrap();
            assert_eq!(
                receipt["scene_sha256"],
                format!("{:x}", Sha256::digest(scene))
            );
        }
    }
}
