//! Metal uses URLs, so admission covers every path component as well as open FDs.
//! Same-user hostile code and privileged OS mutation are outside this boundary.
use super::darwin_acl;
use rustix::{
    fd::{AsFd, OwnedFd},
    fs::{self, AtFlags, FileType, Mode, OFlags},
    process::geteuid,
};
use std::{
    fs::File,
    io::{self, Read, Write},
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);
const DIRECTORY_FLAGS: OFlags = OFlags::RDONLY
    .union(OFlags::DIRECTORY)
    .union(OFlags::NOFOLLOW)
    .union(OFlags::NONBLOCK)
    .union(OFlags::CLOEXEC);
const FILE_FLAGS: OFlags = OFlags::RDONLY
    .union(OFlags::NOFOLLOW)
    .union(OFlags::NONBLOCK)
    .union(OFlags::CLOEXEC);

pub(super) struct Directory {
    path: PathBuf,
    // Root first, leaf last. Holding every component also permits checking that
    // the URL still names the same tree before and after native file access.
    chain: Vec<(OwnedFd, std::ffi::OsString)>,
}
impl Directory {
    pub fn open(path: &Path, create: bool) -> io::Result<Self> {
        if !path.is_absolute() {
            return Err(invalid("archive directory must be absolute"));
        }
        let root = fs::open("/", DIRECTORY_FLAGS, Mode::empty())?;
        validate_directory(&root, false)?;
        let mut chain = vec![(root, std::ffi::OsString::from("/"))];
        for component in path.components().skip(1) {
            let Component::Normal(name) = component else {
                return Err(invalid("archive path component"));
            };
            let parent = &chain.last().expect("root").0;
            let fd = match fs::openat(parent, name, DIRECTORY_FLAGS, Mode::empty()) {
                Err(rustix::io::Errno::NOENT) if create => {
                    match fs::mkdirat(parent, name, Mode::RUSR | Mode::WUSR | Mode::XUSR) {
                        Ok(()) | Err(rustix::io::Errno::EXIST) => {}
                        Err(error) => return Err(error.into()),
                    }
                    fs::openat(parent, name, DIRECTORY_FLAGS, Mode::empty())?
                }
                result => result?,
            };
            validate_directory(&fd, false)?;
            chain.push((fd, name.to_owned()));
        }
        let directory = Self {
            path: path.to_owned(),
            chain,
        };
        directory.validate()?;
        Ok(directory)
    }
    fn fd(&self) -> &OwnedFd {
        &self.chain.last().expect("root").0
    }
    pub fn validate(&self) -> io::Result<()> {
        for (index, (fd, name)) in self.chain.iter().enumerate() {
            validate_directory(fd, index + 1 == self.chain.len())?;
            if index != 0 {
                let by_path =
                    fs::statat(&self.chain[index - 1].0, name, AtFlags::SYMLINK_NOFOLLOW)?;
                same_file(&fs::fstat(fd)?, &by_path)?;
            }
        }
        Ok(())
    }
    pub fn path(&self, name: &str) -> io::Result<PathBuf> {
        filename(name)?;
        self.validate()?;
        Ok(self.path.join(name))
    }
    pub fn read(&self, name: &str, max: usize) -> io::Result<Vec<u8>> {
        filename(name)?;
        self.validate()?;
        let mut file = File::from(fs::openat(self.fd(), name, FILE_FLAGS, Mode::empty())?);
        self.validate_file(&file, name, max)?;
        let mut bytes = Vec::new();
        (&mut file)
            .take(max.checked_add(1).ok_or_else(|| invalid("read bound"))? as u64)
            .read_to_end(&mut bytes)?;
        self.validate_file(&file, name, max)?;
        self.validate()?;
        if bytes.len() > max {
            return Err(invalid("archive exceeded read bound"));
        }
        Ok(bytes)
    }
    fn validate_file(&self, file: &File, name: &str, max: usize) -> io::Result<()> {
        let stat = fs::fstat(file)?;
        if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile
            || stat.st_uid != geteuid().as_raw()
            || stat.st_nlink != 1
            || stat.st_mode & 0o7777 != 0o600
        {
            return Err(untrusted(
                "archive file is not private, owned and singly linked",
            ));
        }
        if stat.st_size < 0 || stat.st_size as u64 > max as u64 {
            return Err(invalid("archive length"));
        }
        darwin_acl::deny_only(file.as_fd())?;
        same_file(
            &stat,
            &fs::statat(self.fd(), name, AtFlags::SYMLINK_NOFOLLOW)?,
        )
    }
    fn temporary(&self) -> io::Result<(String, File)> {
        self.validate()?;
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos();
        let name = format!(
            ".metal-{}-{nonce}-{}.tmp",
            std::process::id(),
            NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
        );
        let fd = fs::openat(
            self.fd(),
            &name,
            OFlags::RDWR | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::RUSR | Mode::WUSR,
        )?;
        let file = File::from(fd);
        if let Err(error) = self.validate_file(&file, &name, 0) {
            let _ = fs::unlinkat(self.fd(), &name, AtFlags::empty());
            return Err(error);
        }
        Ok((name, file))
    }
    /// Each temporary is explicitly cleaned in this cold call, never by Drop.
    /// Native writers can replace their temporary; re-open and admit the actual
    /// resulting regular file before publication instead of trusting the old FD.
    pub fn export(
        &self,
        name: &str,
        max: usize,
        write: impl FnOnce(&Path) -> io::Result<()>,
    ) -> io::Result<Vec<u8>> {
        filename(name)?;
        let (temporary, reserved) = self.temporary()?;
        let result = (|| {
            write(&self.path(&temporary)?)?;
            let bytes = self.read(&temporary, max)?;
            if bytes.is_empty() {
                return Err(invalid("empty native archive"));
            }
            let file = File::from(fs::openat(
                self.fd(),
                &temporary,
                FILE_FLAGS,
                Mode::empty(),
            )?);
            self.validate_file(&file, &temporary, max)?;
            file.sync_all()?;
            self.validate()?;
            fs::renameat(self.fd(), &temporary, self.fd(), name)?;
            fs::fsync(self.fd())?;
            Ok(bytes)
        })();
        drop(reserved);
        let cleanup = fs::unlinkat(self.fd(), &temporary, AtFlags::empty());
        match cleanup {
            Ok(()) | Err(rustix::io::Errno::NOENT) => result,
            Err(error) => Err(error.into()),
        }
    }
    pub fn publish(&self, name: &str, bytes: &[u8]) -> io::Result<()> {
        self.export(name, bytes.len(), |path| {
            // Open through the held directory; the URL is only used to recover
            // our generated filename, never an external path.
            let name = path.file_name().ok_or_else(|| invalid("temporary name"))?;
            let mut file = File::from(fs::openat(
                self.fd(),
                name,
                OFlags::WRONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )?);
            file.write_all(bytes)?;
            file.sync_all()
        })
        .map(|_| ())
    }
}
fn validate_directory(fd: &OwnedFd, leaf: bool) -> io::Result<()> {
    let stat = fs::fstat(fd)?;
    let uid = geteuid().as_raw();
    if FileType::from_raw_mode(stat.st_mode) != FileType::Directory
        || (stat.st_uid != uid && stat.st_uid != 0)
        || stat.st_mode & 0o022 != 0
        || (leaf && (stat.st_uid != uid || stat.st_mode & 0o7777 != 0o700))
    {
        return Err(untrusted("archive URL directory is not trusted"));
    }
    darwin_acl::deny_only(fd.as_fd())
}
fn same_file(a: &fs::Stat, b: &fs::Stat) -> io::Result<()> {
    if a.st_dev != b.st_dev || a.st_ino != b.st_ino {
        return Err(untrusted("archive URL changed identity"));
    }
    Ok(())
}
fn filename(name: &str) -> io::Result<()> {
    if name.is_empty() || name.contains(['/', '\\', '\0']) || name == "." || name == ".." {
        return Err(invalid("archive filename"));
    }
    Ok(())
}
fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
fn untrusted(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, message)
}
