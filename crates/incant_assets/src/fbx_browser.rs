use crate::{AssetError, ImportedModel, Result};
use std::path::Path;

/// Browser hosts can consume cooked models, but cannot cook FBX sources with
/// the pinned native C parser. Refuse before any source/cache access.
pub fn import_fbx(_root: &Path, _source: &Path) -> Result<ImportedModel> {
    Err(AssetError::Unsupported(
        "FBX source import is unavailable on WebAssembly; cook the model on a native host".into(),
    ))
}
