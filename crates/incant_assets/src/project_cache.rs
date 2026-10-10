//! Default project caches are capabilities, not arbitrary filesystem paths.
use crate::{Result, invalid};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy)]
pub enum CacheKind {
    Models,
    Textures,
    Audio,
}
impl CacheKind {
    pub(crate) fn directory(self) -> &'static str {
        match self {
            Self::Models => "models",
            Self::Textures => "textures",
            Self::Audio => "audio",
        }
    }
}

/// Resolve an existing project-owned cache directory without creating anything.
/// Check every component before descending, so a repository-supplied directory
/// symlink cannot redirect runtime or watcher reads outside the granted project.
pub fn project_cache_directory(root: &Path, kind: CacheKind) -> Result<PathBuf> {
    let root = root.canonicalize()?;
    let mut path = root.clone();
    for part in [".incant", "cache", kind.directory()] {
        path.push(part);
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink()
            || !metadata.is_dir()
            || !path.canonicalize()?.starts_with(&root)
        {
            return Err(invalid(
                "default cache directories must be real project directories, not symlinks or files",
            ));
        }
    }
    Ok(path)
}
