//! Node launch spelling only. Canonical paths remain the authority for checks.
pub(super) mod cache;
use super::{Result, invalid};
use std::{
    ffi::OsString,
    path::{Component, Path, PathBuf, Prefix},
};

pub(super) fn launch_directory(canonical: &Path) -> Result<PathBuf> {
    let candidate = regular_spelling(canonical)?;
    // Removing a verbatim prefix can change Win32 interpretation (for example
    // trailing dots or spaces). Never launch from a different directory.
    if candidate.canonicalize()? != canonical {
        return Err(invalid(
            "Node launch directory does not match the canonical engine",
        ));
    }
    Ok(candidate)
}
fn regular_spelling(canonical: &Path) -> Result<PathBuf> {
    let mut components = canonical.components();
    let Some(Component::Prefix(prefix)) = components.next() else {
        return Err(invalid("unsupported canonical Windows engine path"));
    };
    let mut value = match prefix.kind() {
        Prefix::VerbatimDisk(drive) => OsString::from(format!("{}:", char::from(drive))),
        Prefix::VerbatimUNC(server, share) => {
            let mut value = OsString::from(r"\\");
            value.push(server);
            value.push(r"\");
            value.push(share);
            value
        }
        Prefix::Disk(_) | Prefix::UNC(_, _) if canonical.is_absolute() => {
            return Ok(canonical.to_path_buf());
        }
        _ => return Err(invalid("unsupported canonical Windows engine namespace")),
    };
    if components.next() != Some(Component::RootDir) {
        return Err(invalid("Node launch directory must be absolute"));
    }
    value.push(r"\");
    let mut candidate = PathBuf::from(value);
    for component in components {
        let Component::Normal(part) = component else {
            return Err(invalid("noncanonical Windows engine path component"));
        };
        candidate.push(part);
    }
    Ok(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn drive_and_unc_preserve_components_and_unsupported_namespaces_refuse() {
        for (input, expected) in [
            (r"\\?\D:\engine Ω\checkout", r"D:\engine Ω\checkout"),
            (r"\\?\UNC\server\share\engine Ω", r"\\server\share\engine Ω"),
            (r"\\?\D:\", r"D:\"),
            (r"D:\engine", r"D:\engine"),
        ] {
            assert_eq!(
                regular_spelling(Path::new(input)).unwrap(),
                PathBuf::from(expected)
            );
        }
        for input in [
            r"\\?\Volume{01234567}\engine",
            r"\\.\pipe\engine",
            r"D:engine",
            r"engine",
            r"\\?\D:\engine\..\other",
        ] {
            assert!(regular_spelling(Path::new(input)).is_err(), "{input}");
        }
    }
    #[test]
    fn actual_canonical_directory_round_trips_before_launch() {
        let parent = tempfile::tempdir().unwrap();
        let root = parent.path().join("engine Ω checkout");
        std::fs::create_dir(&root).unwrap();
        let canonical = root.canonicalize().unwrap();
        let launch = launch_directory(&canonical).unwrap();
        assert_eq!(launch.canonicalize().unwrap(), canonical);
        assert!(
            matches!(launch.components().next(), Some(Component::Prefix(p)) if !p.kind().is_verbatim())
        );
    }
}
