//! Trusted fixture producer only; authoring data and TS are never native code.
use incant_doc::{Entity, Project, Scene, ScriptSource};
use serde_json::json;
use std::{env, fs, io::Read, path::PathBuf};

fn bounded(path: PathBuf, limit: u64) -> Result<String, Box<dyn std::error::Error>> {
    let mut text = String::new();
    fs::File::open(path)?
        .take(limit + 1)
        .read_to_string(&mut text)?;
    if text.len() as u64 > limit {
        return Err("fixture input exceeds cap".into());
    }
    Ok(text)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if env::var("HOST")? != env::var("TARGET")? {
        return Err("project embedding proof requires native host == target".into());
    }
    println!("cargo:rerun-if-changed=fixtures");
    let fixtures = PathBuf::from("fixtures");
    let props: serde_json::Value =
        serde_json::from_str(&bounded(fixtures.join("props.json"), 4096)?)?;
    // Construct initial test data, never mutate a user's project. The real
    // authoring validation and program-aware cook consume this typed fixture.
    let mut project = Project::empty("Bound embedding fixture");
    project.id = format!("{:026}", 90);
    project.settings.tick_rate = 4;
    let mut scene = Scene::new("Main");
    scene.id = format!("{:026}", 91);
    let sources = ["0000000000000000000000000a", "0000000000000000000000000B"];
    for (source, path) in sources.iter().zip(["a.ts", "b.ts"]) {
        project.scripts.insert(
            (*source).into(),
            ScriptSource {
                id: (*source).into(),
                name: path.into(),
                path: path.into(),
            },
        );
    }
    for (number, source, speed) in [(1, sources[0], 2), (2, sources[0], 3), (3, sources[1], 4)] {
        let mut entity = Entity::new("Bound");
        entity.id = format!("{number:026}");
        let mut values = props.clone();
        values["speed"] = json!(speed);
        entity
            .components
            .insert("Script".into(), json!({"source":source,"props":values}));
        scene.entities.insert(entity.id.clone(), entity);
    }
    let scene_id = scene.id.clone();
    project.scenes.insert(scene.id.clone(), scene);
    let out = PathBuf::from(env::var_os("OUT_DIR").ok_or("missing OUT_DIR")?);
    // This real authoring representation is removed with all build storage.
    fs::write(out.join("project.incant.json"), project.canonical_text()?)?;
    let prepared = incant_cook::prepare_script_scene(&project, &scene_id)?;
    let manifest: serde_json::Value =
        serde_json::from_str(&bounded(fixtures.join("manifest.json"), 128 * 1024)?)?;
    if manifest["format"] != "incant-native-typescript-project"
        || &manifest["sources"] != prepared.source_catalog()
    {
        return Err("fresh Node catalog does not match the prepared project".into());
    }
    let source = bounded(fixtures.join("behavior.cjs"), 1_000_000)?;
    let (scene, compiled) = prepared.compile(&source)?.into_parts();
    fs::write(out.join("world.scene"), scene.to_bytes())?;
    fs::write(out.join("behavior.rs"), compiled.embedded_rust())?;
    fs::write(
        out.join("bindings.json"),
        json!({
            "format":compiled.metadata().format,
            "bindings_sha256":compiled.metadata().bindings_sha256,
            "source_sha256":compiled.metadata().source_sha256,
        })
        .to_string(),
    )?;
    Ok(())
}
