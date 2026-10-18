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
fn information(
    file: &File,
) -> io::Result<windows_sys::Win32::Storage::FileSystem::BY_HANDLE_FILE_INFORMATION> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle,
    };
    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    // SAFETY: the File owns a live handle and info has the API's complete layout.
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(info)
}
#[cfg(windows)]
fn identity(file: &File) -> io::Result<(u64, u64)> {
    use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
    let info = information(file)?;
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
pub(crate) fn regular(file: &File) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let info = file.metadata()?;
        if info.is_file() && info.nlink() == 1 {
            return Ok(());
        }
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT,
        };
        let info = information(file)?;
        if info.dwFileAttributes & (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT) == 0
            && info.nNumberOfLinks == 1
        {
            return Ok(());
        }
    }
    #[cfg(not(any(unix, windows)))]
    let _ = file;
    Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        "opened node must be a single-link regular file",
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
    #[test]
    fn opened_file_rechecks_single_link_admission_after_namespace_change() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("original");
        let alias = root.path().join("alias");
        std::fs::write(&path, b"bounded data").unwrap();
        let held = read(&path).unwrap();
        regular(&held).unwrap();
        std::fs::hard_link(&path, &alias).unwrap();
        assert!(regular(&held).is_err());
        assert!(crate::backend::bounded(&path, 1024).is_err());
        // Windows no-DELETE-sharing applies to every name of the held file.
        drop(held);
        std::fs::remove_file(alias).unwrap();
        let held = read(&path).unwrap();
        regular(&held).unwrap();
        assert_eq!(
            crate::backend::bounded(&path, 1024).unwrap(),
            b"bounded data"
        );
    }
}
