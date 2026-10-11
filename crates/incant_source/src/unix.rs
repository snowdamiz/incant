use rustix::fs::{self, AtFlags, Mode, OFlags, RenameFlags};
use std::{fs::File, io, path::Path};

pub struct Dir(File);
impl Dir {
    pub fn root(path: &Path) -> io::Result<Self> {
        Ok(Self(
            fs::open(
                path,
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )?
            .into(),
        ))
    }
    pub fn duplicate(&self) -> io::Result<Self> {
        Ok(Self(self.0.try_clone()?))
    }
    pub fn child(&self, name: &str) -> io::Result<Self> {
        Ok(Self(
            fs::openat(
                &self.0,
                name,
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )?
            .into(),
        ))
    }
    pub fn mkdir(&self, name: &str) -> io::Result<()> {
        fs::mkdirat(&self.0, name, Mode::RUSR | Mode::WUSR | Mode::XUSR)?;
        self.sync()
    }
    pub fn open(&self, name: &str) -> io::Result<File> {
        Ok(fs::openat(
            &self.0,
            name,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
        )?
        .into())
    }
    pub fn create(&self, name: &str) -> io::Result<File> {
        Ok(fs::openat(
            &self.0,
            name,
            OFlags::RDWR | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::RUSR | Mode::WUSR,
        )?
        .into())
    }
    pub fn move_to(&self, name: &str, target: &Self, destination: &str) -> io::Result<()> {
        // Never fall back to a clobbering rename or a check-then-rename sequence.
        fs::renameat_with(
            &self.0,
            name,
            &target.0,
            destination,
            RenameFlags::NOREPLACE,
        )?;
        self.sync()?;
        target.sync()
    }
    pub fn remove(&self, name: &str) -> io::Result<()> {
        fs::unlinkat(&self.0, name, AtFlags::empty())?;
        self.sync()
    }
    pub fn sync(&self) -> io::Result<()> {
        self.0.sync_all()
    }
}
pub fn regular(file: &File) -> io::Result<()> {
    use std::os::unix::fs::MetadataExt;
    let meta = file.metadata()?;
    if !meta.is_file() || meta.nlink() != 1 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "source must be a regular file with one link",
        ));
    }
    Ok(())
}

pub fn identity(file: &File) -> io::Result<(u64, u64)> {
    use std::os::unix::fs::MetadataExt;
    let meta = file.metadata()?;
    Ok((meta.dev(), meta.ino()))
}
