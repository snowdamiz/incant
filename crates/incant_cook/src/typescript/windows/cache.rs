//! Installed SWC native bytes are reusable host dependencies, never project output.
//! Windows' known folder selects the root. The pinned carrier supplies private
//! SID namespaces, DACL checks, full-byte validation, locks and atomic repair.
#[cfg(test)]
mod diagnostic;
#[cfg(test)]
mod tests;
use std::{
    ffi::OsString,
    fs::{File, OpenOptions},
    io,
    os::windows::{
        ffi::{OsStrExt, OsStringExt},
        fs::OpenOptionsExt,
        io::AsRawHandle,
    },
    path::{Component, Path, PathBuf, Prefix},
    ptr,
};
use windows_sys::Win32::{
    Globalization::{CSTR_EQUAL, CompareStringOrdinal},
    Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT,
        FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ,
        FILE_SHARE_WRITE, GetFileInformationByHandle,
    },
    System::Com::CoTaskMemFree,
    UI::Shell::{FOLDERID_LocalAppData, SHGetKnownFolderPath},
};

fn refused(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, message)
}
fn local_app_data() -> io::Result<PathBuf> {
    let mut value = ptr::null_mut();
    // SAFETY: OS-owned known-folder allocation; freed on every result below.
    let status =
        unsafe { SHGetKnownFolderPath(&FOLDERID_LocalAppData, 0, ptr::null_mut(), &mut value) };
    struct ShellString(*mut u16);
    impl Drop for ShellString {
        fn drop(&mut self) {
            unsafe {
                CoTaskMemFree(self.0.cast());
            }
        }
    }
    let _owner = ShellString(value);
    if status < 0 {
        return Err(io::Error::other(format!(
            "LocalAppData HRESULT {status:#x}"
        )));
    }
    if value.is_null() {
        return Err(refused("LocalAppData is absent"));
    }
    let mut length = 0;
    while length < 32767 && unsafe { *value.add(length) } != 0 {
        length += 1;
    }
    if length == 32767 {
        return Err(refused("LocalAppData exceeds path bound"));
    }
    Ok(PathBuf::from(OsString::from_wide(unsafe {
        std::slice::from_raw_parts(value, length)
    })))
}
fn same_path(left: &Path, right: &Path) -> bool {
    let a: Vec<_> = left.as_os_str().encode_wide().collect();
    let b: Vec<_> = right.as_os_str().encode_wide().collect();
    if a.len() > 32767 || b.len() > 32767 {
        return false;
    }
    // Paths are canonical and separator-normalized. Use Windows ordinal case
    // semantics, including non-ASCII names, rather than Rust's case-sensitive Eq.
    unsafe {
        CompareStringOrdinal(a.as_ptr(), a.len() as i32, b.as_ptr(), b.len() as i32, 1)
            == CSTR_EQUAL
    }
}
fn overlaps(left: &Path, right: &Path) -> bool {
    left.ancestors().any(|p| same_path(p, right)) || right.ancestors().any(|p| same_path(p, left))
}
fn pin(path: &Path) -> io::Result<File> {
    let file = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)?;
    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0 {
        return Err(io::Error::last_os_error());
    }
    if info.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY == 0
        || info.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
    {
        return Err(refused("SWC cache ancestor must be a no-reparse directory"));
    }
    Ok(file)
}
pub(crate) struct Cache {
    path: PathBuf,
    // No DELETE sharing pins the admitted parents until the cook child is reaped.
    _ancestors: Vec<File>,
}
impl Cache {
    pub fn path(&self) -> &Path {
        &self.path
    }
}
pub(crate) fn dependency_cache(project: &Path) -> io::Result<Cache> {
    resolve(&local_app_data()?, project)
}
fn resolve(known_folder: &Path, project: &Path) -> io::Result<Cache> {
    let mut components = known_folder.components();
    let Some(Component::Prefix(prefix)) = components.next() else {
        return Err(refused("LocalAppData must be an absolute local path"));
    };
    if !matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_))
        || components.next() != Some(Component::RootDir)
    {
        return Err(refused("LocalAppData must be an absolute local path"));
    }
    let mut path = PathBuf::from(prefix.as_os_str());
    path.push(Path::new("\\"));
    let mut held = vec![pin(&path)?];
    let normal: Vec<_> = components.collect();
    if normal.is_empty() || normal.len() > 64 {
        return Err(refused("LocalAppData ancestor count"));
    }
    for component in normal {
        let Component::Normal(name) = component else {
            return Err(refused("ambiguous LocalAppData component"));
        };
        path.push(name);
        held.push(pin(&path)?);
    }
    let canonical = path.canonicalize()?;
    let project = project.canonicalize()?;
    let cache = canonical.join("swc");
    if overlaps(&cache, &project) {
        return Err(refused("SWC dependency cache overlaps project content"));
    }
    match pin(&cache) {
        Ok(file) => held.push(file),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    // An existing leaf must not hide an alias into the project. The carrier
    // independently rejects unsafe ACLs and all reparse components before load.
    if cache.exists() && overlaps(&cache.canonicalize()?, &project) {
        return Err(refused("SWC dependency cache overlaps project content"));
    }
    Ok(Cache {
        path: cache,
        _ancestors: held,
    })
}
