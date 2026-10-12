//! Trusted native-build producer for an authored project. Only engine Rust is
//! compiled; project input is bounded JSON and TypeScript source data.
use crate::{
    compile_project_sources, prepare_script_scene_with_resources, script::artifact_from_compiled,
};
use incant_doc::Project;
use incant_runtime::CookedScene;
use sha2::{Digest, Sha256};
use std::{env, fs, io::Read, path::Path};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub const PROJECT_ENV: &str = "INCANT_NATIVE_PROJECT";
pub const SCENE_ENV: &str = "INCANT_NATIVE_PROJECT_SCENE";

/// Called only from an engine-owned build.rs. Generated executable Rust is
/// written directly into its fresh OUT_DIR, never read back from project files.
pub fn embed_project(project_path: &Path, selected_scene: Option<&str>) -> Result<()> {
    let target = env::var("TARGET")?;
    if env::var("HOST")? != target || target != incant_script::artifact::TARGET {
        return Err("authored project export requires native host == target".into());
    }
    for key in [
        "INCANT_NATIVE_SCRIPT_SOURCE",
        "INCANT_NATIVE_SCRIPT_MAP",
        "INCANT_NATIVE_SCRIPT_NAME",
    ] {
        if env::var_os(key).is_some() {
            return Err("project export cannot also select standalone script inputs".into());
        }
    }
    changed(project_path)?;
    let project_path = project_path.canonicalize()?;
    let root = project_path
        .parent()
        .ok_or("project has no parent directory")?;
    let source_epoch = incant_source::check_recovery(root)?;
    let mut text = String::new();
    fs::File::open(&project_path)?
        .take(64 * 1024 * 1024 + 1)
        .read_to_string(&mut text)?;
    if text.len() > 64 * 1024 * 1024 {
        return Err("project text exceeds native export limit".into());
    }
    let project = Project::from_text(&text)?;
    let scene_id = match selected_scene {
        Some(id) => id.to_owned(),
        None if project.scenes.len() == 1 => project.scenes.keys().next().unwrap().clone(),
        None => return Err("choose a scene when the project does not contain exactly one".into()),
    };
    let authored = project
        .scenes
        .get(&scene_id)
        .ok_or("selected scene is missing")?;
    let attached = authored
        .entities
        .values()
        .any(|e| e.components.contains_key("Script"));
    let animations = crate::load_project_animation_resources(&project, &scene_id, root)?;
    for path in animations.cache_paths() {
        changed(path)?;
    }
    let (scene, artifact, compiler_manifest) = if attached {
        let compiled = compile_project_sources(
            prepare_script_scene_with_resources(
                &project,
                &scene_id,
                &Default::default(),
                &animations,
            )?,
            root,
        )?;
        let (scene, program) = compiled.program.into_parts();
        // Changes to either an entry or a transitive helper rerun the producer.
        // These names come from the fixed compiler's validated graph snapshot.
        for module in compiled.compiler_manifest["modules"]
            .as_array()
            .ok_or("compiler module list")?
        {
            let path = module["path"].as_str().ok_or("compiler module path")?;
            if path
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
                || path.contains(['\\', ':'])
                || Path::new(path).is_absolute()
            {
                return Err("compiler module path is not relative".into());
            }
            changed(&root.join(path))?;
        }
        (
            scene,
            artifact_from_compiled(&program, Some(&compiled.source_map)),
            compiled.compiler_manifest,
        )
    } else {
        let scene = CookedScene::from_bytes(&crate::cook_scene_with_resources(
            &project,
            &scene_id,
            &Default::default(),
            &animations,
        )?)?;
        let artifact = crate::cook_script(
            "incant-empty-project",
            "exports.default={update(){}};",
            None,
            &target,
        )?;
        (scene, artifact, serde_json::Value::Null)
    };
    let engine = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for path in [
        "tools/build_native_script.mjs",
        "tools/native_script",
        "sdk/ts/src",
    ] {
        changed(&engine.join(path))?;
    }
    let out = std::path::PathBuf::from(env::var_os("OUT_DIR").ok_or("OUT_DIR missing")?);
    let scene_bytes = scene.to_bytes();
    fs::write(out.join("world.scene"), &scene_bytes)?;
    fs::write(out.join("behavior.rs"), artifact.rust_module)?;
    fs::write(out.join("script-symbols.json"), artifact.symbol_manifest)?;
    if let Some(map) = artifact.source_map {
        fs::write(out.join("behavior.js.map"), map)?;
    } else if out.join("behavior.js.map").exists() {
        fs::remove_file(out.join("behavior.js.map"))?;
    }
    fs::write(
        out.join("input-actions.json"),
        serde_json::to_vec(&project.settings.input_actions)?,
    )?;
    let visual: Vec<_> = scene.visual_assets().iter().map(|asset| serde_json::json!({
        "id": ulid::Ulid::from_bytes(asset.id.0).to_string(),
        "kind": match asset.kind { incant_runtime::VisualAssetKind::Model => "model", incant_runtime::VisualAssetKind::Texture => "texture" },
        "fingerprint": hex(&asset.fingerprint),
    })).collect();
    let mut audio = std::collections::BTreeMap::new();
    for clip in scene
        .audio()
        .iter()
        .filter_map(|node| node.source.as_ref().map(|s| s.clip))
    {
        let asset = project
            .assets
            .values()
            .find(|asset| crate::stable_id(&asset.id).ok() == Some(clip))
            .ok_or("missing audio asset")?;
        if asset.kind != "audio" {
            return Err("audio reference has the wrong kind".into());
        }
        audio.insert(
            ulid::Ulid::from_bytes(clip.0).to_string(),
            asset.sha256.clone(),
        );
    }
    let manifest = serde_json::json!({
        "format": "incant-native-project-build", "version": 1,
        "project_id": project.id, "scene_id": scene_id, "tick_rate": scene.tick_rate(),
        "target": target, "scene_sha256": hex(&Sha256::digest(&scene_bytes)),
        "project_sha256": hex(&Sha256::digest(text.as_bytes())), "visual_assets": visual,
        "audio_clips": audio, "compiler": compiler_manifest,
        "cameras": scene.spatial().iter().filter(|n| n.camera.is_some())
            .map(|n| ulid::Ulid::from_bytes(n.entity.0).to_string()).collect::<Vec<_>>(),
        "has_spatial_content": !scene.spatial().is_empty() || !scene.visual_assets().is_empty(),
        "startup_source_parsing": false,
    });
    // This receipt is last. A failed producer cannot advertise a complete build.
    if incant_source::check_recovery(root)? != source_epoch {
        return Err("script sources changed during native project build; retry".into());
    }
    fs::write(
        out.join("project-export.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn changed(path: &Path) -> Result<()> {
    let path = path.to_str().ok_or("build input path must be UTF-8")?;
    if path.chars().any(char::is_control) {
        return Err("control character in build input path".into());
    }
    println!("cargo:rerun-if-changed={path}");
    Ok(())
}
