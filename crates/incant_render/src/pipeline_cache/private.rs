//! Owner-only Unix cache storage. Driver data never comes from a project path.
//! Same-user hostile code and privileged OS access are outside this boundary.
use rustix::{
    fd::OwnedFd,
    fs::{self, AtFlags, FileType, Mode, OFlags},
    process::geteuid,
};
use std::{
    fs::{DirBuilder, File},
    io::{self, Read, Write},
    os::unix::fs::DirBuilderExt,
    path::{Component, Path},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

pub(super) struct Directory {
    fd: OwnedFd,
}
impl Directory {
    pub fn open(path: &Path, create: bool) -> io::Result<Self> {
        if !path.is_absolute() {
            return Err(invalid("cache directory must be absolute"));
        }
        if create {
            DirBuilder::new().recursive(true).mode(0o700).create(path)?;
        }
        let fd = fs::open(
            path,
            OFlags::RDONLY
                | OFlags::DIRECTORY
                | OFlags::NOFOLLOW
                | OFlags::NONBLOCK
                | OFlags::CLOEXEC,
            Mode::empty(),
        )?;
        let directory = Self { fd };
        directory.validate()?;
        Ok(directory)
    }

    fn validate(&self) -> io::Result<()> {
        let stat = fs::fstat(&self.fd)?;
        if FileType::from_raw_mode(stat.st_mode) != FileType::Directory
            || stat.st_uid != geteuid().as_raw()
            || stat.st_mode & 0o7777 != 0o700
        {
            return Err(untrusted("cache directory is not private and owned"));
        }
        Ok(())
    }

    pub fn read(&self, name: &str, max_bytes: usize) -> io::Result<Vec<u8>> {
        filename(name)?;
        self.validate()?;
        let limit = max_bytes
            .checked_add(1)
            .ok_or_else(|| invalid("cache read bound"))?;
        let fd = fs::openat(
            &self.fd,
            name,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
        )?;
        let mut file = File::from(fd);
        validate_file(&file, max_bytes)?;
        let mut bytes = Vec::new();
        (&mut file).take(limit as u64).read_to_end(&mut bytes)?;
        validate_file(&file, max_bytes)?;
        self.validate()?;
        if bytes.len() > max_bytes {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "cache grew beyond limit",
            ));
        }
        Ok(bytes)
    }

    pub fn publish(&self, name: &str, parts: &[&[u8]]) -> io::Result<()> {
        filename(name)?;
        self.validate()?;
        let sequence = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let temporary = format!(".{}-{nonce}-{sequence}.tmp", std::process::id());
        let fd = fs::openat(
            &self.fd,
            &temporary,
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::RUSR | Mode::WUSR,
        )?;
        let cleanup = Temporary {
            directory: &self.fd,
            name: temporary,
        };
        let mut file = File::from(fd);
        validate_file(&file, usize::MAX)?;
        for part in parts {
            file.write_all(part)?;
        }
        file.sync_all()?;
        validate_file(&file, usize::MAX)?;
        self.validate()?;
        fs::renameat(&self.fd, &cleanup.name, &self.fd, name)?;
        // The file and directory are both durable before reporting success.
        // Losing a cache on filesystems without directory sync is still safe:
        // the caller treats every I/O error as an optional cache miss.
        fs::fsync(&self.fd)?;
        Ok(())
    }
}

fn validate_file(file: &File, max_bytes: usize) -> io::Result<()> {
    let stat = fs::fstat(file)?;
    if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile
        || stat.st_uid != geteuid().as_raw()
        || stat.st_nlink != 1
        || stat.st_mode & 0o7077 != 0
    {
        return Err(untrusted(
            "cache file is not private, owned and singly linked",
        ));
    }
    if stat.st_size < 0 || stat.st_size as u64 > max_bytes as u64 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "cache file length",
        ));
    }
    Ok(())
}

fn filename(name: &str) -> io::Result<()> {
    let mut components = Path::new(name).components();
    if name.is_empty()
        || name.contains(['/', '\\', '\0'])
        || !matches!(components.next(), Some(Component::Normal(_)))
        || components.next().is_some()
    {
        return Err(invalid("cache key must be one filename"));
    }
    Ok(())
}
fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}
fn untrusted(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, message)
}
struct Temporary<'a> {
    directory: &'a OwnedFd,
    name: String,
}
impl Drop for Temporary<'_> {
    fn drop(&mut self) {
        let _ = fs::unlinkat(self.directory, &self.name, AtFlags::empty());
    }
}

#[cfg(test)]
#[path = "private_tests.rs"]
mod tests;
