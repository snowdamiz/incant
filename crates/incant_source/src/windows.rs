use std::{
    fs::{File, OpenOptions},
    io,
    os::windows::{ffi::OsStrExt, fs::OpenOptionsExt, io::AsRawHandle},
    path::{Path, PathBuf},
};
use windows_sys::Win32::Storage::FileSystem::*;

// Directory handles deny DELETE sharing for their lifetime. Thus no path prefix
// can be renamed or replaced with a junction between validation and an operation.
// Keep the entire chain alive, including while operating on a child directory.
pub struct Dir {
    file: File,
    path: PathBuf,
    ancestors: Vec<File>,
}
fn directory(path: &Path) -> io::Result<File> {
    let file = OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)?;
    let info = information(&file)?;
    if info.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
        || info.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY == 0
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "source directory is a reparse point or not a directory",
        ));
    }
    Ok(file)
}
impl Dir {
    pub fn root(path: &Path) -> io::Result<Self> {
        // A trusted caller supplies the root. Canonicalization is only used here;
        // project-controlled child components are opened individually, no-follow.
        let path = std::fs::canonicalize(path)?;
        Ok(Self {
            file: directory(&path)?,
            path,
            ancestors: Vec::new(),
        })
    }
    pub fn duplicate(&self) -> io::Result<Self> {
        Ok(Self {
            file: self.file.try_clone()?,
            path: self.path.clone(),
            ancestors: self
                .ancestors
                .iter()
                .map(File::try_clone)
                .collect::<io::Result<Vec<_>>>()?,
        })
    }
    pub fn child(&self, name: &str) -> io::Result<Self> {
        let path = self.path.join(name);
        let mut ancestors = self
            .ancestors
            .iter()
            .map(File::try_clone)
            .collect::<io::Result<Vec<_>>>()?;
        ancestors.push(self.file.try_clone()?);
        Ok(Self {
            file: directory(&path)?,
            path,
            ancestors,
        })
    }
    pub fn mkdir(&self, name: &str) -> io::Result<()> {
        std::fs::create_dir(self.path.join(name))
    }
    pub fn open(&self, name: &str) -> io::Result<File> {
        OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
            .open(self.path.join(name))
    }
    pub fn create(&self, name: &str) -> io::Result<File> {
        OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_WRITE_THROUGH)
            .open(self.path.join(name))
    }
    pub fn move_to(&self, name: &str, target: &Self, destination: &str) -> io::Result<()> {
        let from: Vec<_> = self
            .path
            .join(name)
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();
        let to: Vec<_> = target
            .path
            .join(destination)
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();
        // SAFETY: both paths are terminated UTF-16, parents are pinned no-reparse
        // directory handles. No REPLACE_EXISTING/COPY_ALLOWED: refuse a collision
        // or cross-volume move, never follow or overwrite a destination link.
        if unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), MOVEFILE_WRITE_THROUGH) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
    pub fn remove(&self, name: &str) -> io::Result<()> {
        std::fs::remove_file(self.path.join(name))
    }
    // Windows publishes moves through MOVEFILE_WRITE_THROUGH; files use
    // FILE_FLAG_WRITE_THROUGH + FlushFileBuffers (File::sync_all). Directory
    // FlushFileBuffers is not a supported portable Windows directory operation.
    pub fn sync(&self) -> io::Result<()> {
        Ok(())
    }
}
fn information(file: &File) -> io::Result<BY_HANDLE_FILE_INFORMATION> {
    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    // SAFETY: the live File owns the handle; info points to a fully sized struct.
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(info)
}
pub fn regular(file: &File) -> io::Result<()> {
    let info = information(file)?;
    if info.dwFileAttributes & (FILE_ATTRIBUTE_REPARSE_POINT | FILE_ATTRIBUTE_DIRECTORY) != 0
        || info.nNumberOfLinks != 1
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "source must be a regular file with one link and no reparse point",
        ));
    }
    Ok(())
}

pub fn identity(file: &File) -> io::Result<(u64, u64)> {
    let info = information(file)?;
    Ok((
        u64::from(info.dwVolumeSerialNumber),
        (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow),
    ))
}
