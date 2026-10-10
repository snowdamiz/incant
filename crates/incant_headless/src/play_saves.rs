//! Host-owned file IO. Sandboxed scripts receive no save-path capability.
use crate::play::PlayError;
use incant_script::{MAX_SAVE_BYTES, PlaySession, SaveError};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub fn load(path: &Path) -> Result<String, PlayError> {
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(MAX_SAVE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_SAVE_BYTES {
        return Err(SaveError::Size.into());
    }
    String::from_utf8(bytes).map_err(|_| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, "game save must be UTF-8").into()
    })
}

pub struct Output {
    destination: PathBuf,
    staged: tempfile::NamedTempFile,
}
impl Output {
    pub fn new(path: &Path, frames: Option<&Path>, logs: Option<&Path>) -> Result<Self, PlayError> {
        let parent = parent_dir(path);
        fs::create_dir_all(parent)?;
        if fs::symlink_metadata(path).is_ok() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "save output already exists",
            )
            .into());
        }
        let name = file_name(path);
        let directory = parent.canonicalize()?;
        if frames.is_some_and(|frames| {
            frames
                .canonicalize()
                .is_ok_and(|frames| frames == directory)
                && (name == "report.json" || (name.starts_with("frame-") && name.ends_with(".png")))
        }) {
            return Err(PlayError::SaveOutputConflict);
        }
        if let Some(logs) = logs {
            fs::create_dir_all(parent_dir(logs))?;
            if name == file_name(logs) && directory == parent_dir(logs).canonicalize()? {
                return Err(PlayError::SaveOutputConflict);
            }
        }
        Ok(Self {
            destination: path.to_path_buf(),
            staged: tempfile::NamedTempFile::new_in(parent)?,
        })
    }
    pub fn finish(mut self, play: &PlaySession) -> Result<(), PlayError> {
        self.staged.write_all(play.save_text()?.as_bytes())?;
        self.staged.as_file().sync_all()?;
        self.staged
            .persist_noclobber(self.destination)
            .map_err(|error| error.error)?;
        Ok(())
    }
}
fn parent_dir(path: &Path) -> &Path {
    path.parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
}
fn file_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_save_created_during_playback_is_not_overwritten() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("save.json");
        let output = Output::new(&path, None, None).unwrap();
        fs::write(&path, "another process won").unwrap();
        let play = PlaySession::new(
            &incant_doc::Project::empty("Save publication"),
            "exports.default={update(){}};",
        )
        .unwrap();
        assert!(output.finish(&play).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "another process won");
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    }
}
