//! Read-only held node identities. Windows directory handles need backup semantics
//! and exclude DELETE sharing so a project root cannot be rebound underneath us.
use std::{
    fs::{File, OpenOptions},
    io,
    path::Path,
};
pub(crate) fn read(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ,
            FILE_SHARE_WRITE,
        };
        options
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT);
    }
    let file = options.open(path)?;
    identity(&file)?;
    Ok(file)
}
pub(crate) fn directory(path: &Path) -> io::Result<File> {
    let file = read(path)?;
    if !file.metadata()?.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "held project root is not a directory",
        ));
    }
    Ok(file)
}
pub(crate) fn matches(file: &File, path: &Path) -> io::Result<bool> {
    Ok(identity(file)? == identity(&read(path)?)?)
}
#[cfg(unix)]
fn identity(file: &File) -> io::Result<(u64, u64)> {
    use std::os::unix::fs::MetadataExt;
    let m = file.metadata()?;
    Ok((m.dev(), m.ino()))
}
#[cfg(windows)]
fn identity(file: &File) -> io::Result<(u64, u64)> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, FILE_ATTRIBUTE_REPARSE_POINT, GetFileInformationByHandle,
    };
    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    // SAFETY: the File owns a live handle and info has the API's complete layout.
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0 {
        return Err(io::Error::last_os_error());
    }
    if info.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "held node is a reparse point",
        ));
    }
    Ok((
        u64::from(info.dwVolumeSerialNumber),
        (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow),
    ))
}
#[cfg(not(any(unix, windows)))]
fn identity(_: &File) -> io::Result<(u64, u64)> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "held filesystem identity unavailable",
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn held_directory_matches_only_its_original_node() {
        let root = tempfile::tempdir().unwrap();
        let first = root.path().join("first");
        let other = root.path().join("other");
        std::fs::create_dir(&first).unwrap();
        std::fs::create_dir(&other).unwrap();
        let held = directory(&first).unwrap();
        assert!(matches(&held, &first).unwrap());
        assert!(!matches(&held, &other).unwrap());
        let file = root.path().join("regular");
        std::fs::write(&file, b"data").unwrap();
        assert!(directory(&file).is_err());
        #[cfg(unix)]
        {
            let link = root.path().join("alias");
            std::os::unix::fs::symlink(&first, &link).unwrap();
            assert!(directory(&link).is_err());
        }
    }
}
