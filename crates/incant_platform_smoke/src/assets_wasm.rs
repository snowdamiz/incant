//! Actual browser smoke coverage of the explicit native-only source importer.
use incant_assets::{AssetError, cook_fbx, cook_gltf, cook_model, import_fbx};
use std::path::Path;

fn refused<T>(result: incant_assets::Result<T>, expected: &str) -> Result<(), String> {
    match result {
        Err(AssetError::Unsupported(message)) if message.contains(expected) => Ok(()),
        Err(error) => Err(format!(
            "FBX browser refusal used the wrong boundary: {error}"
        )),
        Ok(_) => Err("FBX browser source import unexpectedly succeeded".into()),
    }
}

pub fn check() -> Result<(), String> {
    // No source/cache exists. Every API must refuse the capability before any
    // filesystem access, including the extension-dispatch entry point.
    let root = Path::new("missing-fbx-project");
    let source = Path::new("model.fbx");
    let cache = Path::new("missing-fbx-cache");
    let fbx = "FBX source import is unavailable on WebAssembly";
    refused(import_fbx(root, source), fbx)?;
    refused(cook_fbx(root, source, cache), fbx)?;
    refused(cook_model(root, source, cache), fbx)?;
    refused(cook_model(root, Path::new("model.FBX"), cache), fbx)?;
    let model = "model source cooking is unavailable on WebAssembly";
    refused(cook_gltf(root, Path::new("model.gltf"), cache), model)?;
    refused(cook_model(root, Path::new("model.gltf"), cache), model)?;
    refused(cook_model(root, Path::new("model.GLB"), cache), model)
}
