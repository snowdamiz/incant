//! Windows cache storage is private host state, never project content.
//! Hold every no-reparse ancestor without DELETE sharing. Privacy is a separate
//! handle-based owner/protected-DACL check on the cache directory and each file.
mod security;
#[cfg(test)]
mod tests;
use std::{
    ffi::OsString,
    fs::{File, OpenOptions},
    io::{self, Read, Write},
    os::windows::{
        ffi::{OsStrExt, OsStringExt},
        fs::OpenOptionsExt,
        io::{AsRawHandle, FromRawHandle},
    },
    path::{Component, Path, PathBuf, Prefix},
    ptr,
};
use windows_sys::Win32::{
    Foundation::INVALID_HANDLE_VALUE,
    Storage::FileSystem::*,
    System::Com::CoTaskMemFree,
    UI::Shell::{FOLDERID_LocalAppData, SHGetKnownFolderPath},
};

fn refused(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, message)
}
fn wide(path: &Path) -> io::Result<Vec<u16>> {
    let mut value: Vec<_> = path.as_os_str().encode_wide().collect();
    if value.contains(&0) || value.len() > 32767 {
        return Err(refused("invalid cache path"));
    }
    value.push(0);
    Ok(value)
}
pub(super) fn application_directory() -> io::Result<PathBuf> {
    let mut value = ptr::null_mut();
    let status =
        unsafe { SHGetKnownFolderPath(&FOLDERID_LocalAppData, 0, ptr::null_mut(), &mut value) };
    if status < 0 {
        return Err(io::Error::other(format!(
            "LocalAppData HRESULT {status:#x}"
        )));
    }
    struct ShellString(*mut u16);
    impl Drop for ShellString {
        fn drop(&mut self) {
            unsafe {
                CoTaskMemFree(self.0.cast());
            }
        }
    }
    let owner = ShellString(value);
    if value.is_null() {
        return Err(refused("LocalAppData is absent"));
    }
    let mut length = 0;
    while unsafe { *value.add(length) } != 0 {
        length += 1;
        if length > 32767 {
            return Err(refused("LocalAppData exceeds path bound"));
        }
    }
    let path = PathBuf::from(OsString::from_wide(unsafe {
        std::slice::from_raw_parts(value, length)
    }));
    drop(owner);
    // One protected leaf, avoiding inherited permissions on application parents.
    Ok(path.join("Incant-Dx12PipelineLibrary-v1"))
}
fn information(file: &File) -> io::Result<BY_HANDLE_FILE_INFORMATION> {
    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(info)
}
fn pin(path: &Path) -> io::Result<File> {
    let file = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)?;
    let info = information(&file)?;
    if info.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY == 0
        || info.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
    {
        return Err(refused("cache ancestor is not a no-reparse directory"));
    }
    Ok(file)
}
pub(super) struct Directory {
    path: PathBuf,
    chain: Vec<File>,
    user: Vec<u8>,
}
impl Directory {
    pub fn open(path: &Path, create: bool) -> io::Result<Self> {
        let user = security::user()?;
        let mut components = path.components();
        let Some(Component::Prefix(prefix)) = components.next() else {
            return Err(refused("cache path requires an absolute drive anchor"));
        };
        if !matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_))
            || !matches!(components.next(), Some(Component::RootDir))
        {
            return Err(refused("cache path requires a local rooted drive"));
        }
        // Never query the bare C: or verbatim prefix as a separate ancestor.
        let mut current = PathBuf::from(prefix.as_os_str());
        current.push(Path::new("\\"));
        let mut chain = vec![pin(&current)?];
        let normal: Vec<_> = components.collect();
        if normal.is_empty() || normal.len() > 64 {
            return Err(refused("private cache requires 1..64 normal ancestors"));
        }
        for (index, component) in normal.iter().enumerate() {
            let Component::Normal(name) = component else {
                return Err(refused("cache ancestor has an ambiguous component"));
            };
            current.push(name);
            if create && index + 1 == normal.len() && !current.exists() {
                let path = wide(&current)?;
                let result = security::create(&user, |attributes| {
                    if unsafe { CreateDirectoryW(path.as_ptr(), attributes) } == 0 {
                        Err(io::Error::last_os_error())
                    } else {
                        Ok(())
                    }
                });
                if let Err(error) = result
                    && error.kind() != io::ErrorKind::AlreadyExists
                {
                    return Err(error);
                }
            }
            chain.push(pin(&current)?);
        }
        let result = Self {
            path: current,
            chain,
            user,
        };
        result.validate()?;
        Ok(result)
    }
    pub fn validate(&self) -> io::Result<()> {
        for file in &self.chain {
            let info = information(file)?;
            if info.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
                || info.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY == 0
            {
                return Err(refused("held cache ancestor changed kind"));
            }
        }
        security::validate(self.chain.last().expect("nonempty held chain"), &self.user)
    }
    fn child(&self, name: &str) -> io::Result<PathBuf> {
        if name.is_empty()
            || name.len() > 64
            || !name
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'.' || c == b'-')
            || matches!(name, "." | "..")
        {
            return Err(refused("invalid fixed cache filename"));
        }
        Ok(self.path.join(name))
    }
    fn regular(&self, file: &File) -> io::Result<()> {
        let info = information(file)?;
        if info.dwFileAttributes & (FILE_ATTRIBUTE_REPARSE_POINT | FILE_ATTRIBUTE_DIRECTORY) != 0
            || info.nNumberOfLinks != 1
        {
            return Err(refused(
                "cache file must be regular, no-reparse and single-link",
            ));
        }
        security::validate(file, &self.user)
    }
    fn open_file(&self, name: &str) -> io::Result<File> {
        let file = OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_DELETE)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
            .open(self.child(name)?)?;
        self.regular(&file)?;
        Ok(file)
    }
    pub fn read(&self, name: &str, limit: usize) -> io::Result<Vec<u8>> {
        self.validate()?;
        let mut file = self.open_file(name)?;
        let length = file.metadata()?.len();
        if length > limit as u64 {
            return Err(refused("cache file exceeds bound"));
        }
        let mut bytes = Vec::new();
        (&mut file).take(limit as u64 + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 != length {
            return Err(refused("cache file changed length"));
        }
        self.regular(&file)?;
        self.validate()?;
        Ok(bytes)
    }
    pub fn publish(&self, name: &str, bytes: &[u8]) -> io::Result<()> {
        self.validate()?;
        let target = self.child(name)?;
        let temporary = self.child("pending.library")?;
        // A fixed exclusive staging name bounds crash leftovers and concurrent
        // writers. Never remove another writer's existing temporary file.
        let path = wide(&temporary)?;
        let mut file = security::create(&self.user, |attributes| {
            let raw = unsafe {
                CreateFileW(
                    path.as_ptr(),
                    FILE_GENERIC_READ | FILE_GENERIC_WRITE,
                    0,
                    attributes,
                    CREATE_NEW,
                    FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_WRITE_THROUGH,
                    ptr::null_mut(),
                )
            };
            if raw == INVALID_HANDLE_VALUE {
                Err(io::Error::last_os_error())
            } else {
                Ok(unsafe { File::from_raw_handle(raw) })
            }
        })?;
        let result = (|| {
            self.regular(&file)?;
            file.write_all(bytes)?;
            file.sync_all()?;
            self.regular(&file)?;
            self.validate()?;
            match self.open_file(name) {
                Ok(_) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
            drop(file);
            let destination = wide(&target)?;
            // Parent directory is pinned; target was admitted or absent. The
            // private protected DACL excludes other untrusted writers. Hostile
            // same-user/admin code is outside the private-cache threat model.
            if unsafe {
                MoveFileExW(
                    path.as_ptr(),
                    destination.as_ptr(),
                    MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
                )
            } == 0
            {
                return Err(io::Error::last_os_error());
            }
            self.validate()
        })();
        if let Err(error) = result {
            // Only the file created by this call is retired, never an existing
            // staging file; no filesystem work occurs in Drop.
            if temporary.exists()
                && let Err(cleanup) = std::fs::remove_file(&temporary)
            {
                return Err(io::Error::new(
                    error.kind(),
                    format!("{error}; staging cleanup: {cleanup}"),
                ));
            }
            return Err(error);
        }
        Ok(())
    }
}
