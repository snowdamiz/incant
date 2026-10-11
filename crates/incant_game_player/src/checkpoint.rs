//! Data-only ordinary-load options and create-only save-file publication.
use crate::session::Result;
use std::{io::Write, path::Path};
#[derive(Default, Debug)]
pub struct Options {
    pub load: Option<String>,
    pub save: Option<String>,
}
impl Options {
    pub fn split(args: Vec<String>) -> Result<(Vec<String>, Self)> {
        let mut result = Vec::new();
        let mut options = Self::default();
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            let slot = match arg.as_str() {
                "--load-checkpoint" => &mut options.load,
                "--save-checkpoint" => &mut options.save,
                _ => {
                    result.push(arg);
                    continue;
                }
            };
            if slot.is_some() {
                return Err("duplicate checkpoint option".into());
            }
            let path = args.next().ok_or("checkpoint option requires a path")?;
            if path.is_empty() || path.starts_with("--") || path.chars().any(char::is_control) {
                return Err("invalid checkpoint path".into());
            }
            *slot = Some(path);
        }
        if (options.load.is_some() || options.save.is_some())
            && result.iter().any(|arg| {
                matches!(
                    arg.as_str(),
                    "--window" | "--gpu-offscreen" | "--static-module" | "--build-info"
                )
            })
        {
            return Err(
                "checkpoints require the offline bytecode plain/CPU-render-input player".into(),
            );
        }
        Ok((result, options))
    }
}
/// N means additional ticks. The input recording starts a new neutral relative
/// timeline; observation checks continue to use the absolute native world tick.
pub fn frame_range(start: u64, ticks: u64, rate: u32, tick_rate: u32) -> Result<(u64, u128)> {
    if tick_rate == 0 {
        return Err("invalid tick rate".into());
    }
    let end = start.checked_add(ticks).ok_or("checkpoint tick overflow")?;
    let frames = u128::from(end) * u128::from(rate) / u128::from(tick_rate)
        - u128::from(start) * u128::from(rate) / u128::from(tick_rate);
    if frames > 1_000_000 {
        return Err("output frame bound".into());
    }
    Ok((end, frames))
}
/// Same-directory staging + atomic no-clobber publication. The file is synced;
/// this is not a universal power-loss durability or multi-file PCM transaction.
pub fn publish(path: &Path, bytes: &[u8]) -> Result<()> {
    if bytes.len() > incant_script::MAX_CHECKPOINT_BYTES {
        return Err("checkpoint size limit".into());
    }
    publish_with(path, |file| {
        file.write_all(bytes)?;
        Ok(())
    })
}
fn publish_with(
    path: &Path,
    write: impl FnOnce(&mut std::fs::File) -> std::io::Result<()>,
) -> Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut file = tempfile::Builder::new()
        .prefix(".incant-checkpoint-")
        .tempfile_in(parent)?;
    write(file.as_file_mut())?;
    file.as_file().sync_all()?;
    file.persist_noclobber(path).map_err(|e| e.error)?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fractional_output_continues_absolute_phase_and_checks_overflow() {
        assert_eq!(frame_range(13, 27, 44100, 144).unwrap(), (40, 8269));
        assert_eq!(
            frame_range(0, 13, 44100, 144).unwrap().1 + frame_range(13, 27, 44100, 144).unwrap().1,
            frame_range(0, 40, 44100, 144).unwrap().1
        );
        assert!(frame_range(u64::MAX, 1, 48000, 60).is_err());
        assert!(frame_range(0, 3600, 192000, 1).is_err());
    }
    #[test]
    fn unsupported_routes_and_ambiguous_options_refuse_before_startup() {
        for mode in [
            "--window",
            "--gpu-offscreen",
            "--static-module",
            "--build-info",
        ] {
            assert!(
                Options::split(vec![
                    mode.into(),
                    "--load-checkpoint".into(),
                    "absent".into()
                ])
                .is_err()
            );
        }
        for args in [
            vec!["--load-checkpoint"],
            vec!["--load-checkpoint", "a", "--load-checkpoint", "b"],
            vec!["--save-checkpoint", "--window"],
        ] {
            assert!(Options::split(args.into_iter().map(String::from).collect()).is_err());
        }
        let (args, options) = Options::split(
            vec![
                "--package",
                "data",
                "--ticks",
                "2",
                "--load-checkpoint",
                "before",
                "--save-checkpoint",
                "after",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
        )
        .unwrap();
        assert_eq!(args, ["--package", "data", "--ticks", "2"]);
        assert_eq!(options.load.as_deref(), Some("before"));
    }
    #[test]
    fn publication_is_complete_create_only_and_cleans_failed_staging() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("game.save");
        assert!(
            publish_with(&path, |f| {
                f.write_all(b"partial")?;
                Err(std::io::Error::other("injected write failure"))
            })
            .is_err()
        );
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
        publish(&path, b"complete").unwrap();
        assert!(publish(&path, b"replacement").is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"complete");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
        assert!(publish(&dir.path().join("missing/game.save"), b"new").is_err());
    }
}
