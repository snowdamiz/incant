//! No filesystem fallback on unsupported authoring targets (including browsers).
use std::{fs::File, io, path::Path};
pub struct Dir;
fn unavailable<T>() -> io::Result<T> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "confined source filesystem is unavailable on this target",
    ))
}
impl Dir {
    pub fn root(_: &Path) -> io::Result<Self> {
        unavailable()
    }
    pub fn duplicate(&self) -> io::Result<Self> {
        unavailable()
    }
    pub fn identity(&self) -> io::Result<[u64; 2]> {
        unavailable()
    }
    pub fn only_child(&self, _: &str) -> io::Result<bool> {
        unavailable()
    }
    pub fn child(&self, _: &str) -> io::Result<Self> {
        unavailable()
    }
    pub fn mkdir(&self, _: &str) -> io::Result<()> {
        unavailable()
    }
    pub fn open(&self, _: &str) -> io::Result<File> {
        unavailable()
    }
    pub fn open_rw(&self, _: &str) -> io::Result<File> {
        unavailable()
    }
    pub fn create(&self, _: &str) -> io::Result<File> {
        unavailable()
    }
    pub fn move_to(&self, _: &str, _: &Self, _: &str) -> io::Result<()> {
        unavailable()
    }
    pub fn remove(&self, _: &str) -> io::Result<()> {
        unavailable()
    }
    pub fn sync(&self) -> io::Result<()> {
        unavailable()
    }
}
pub fn regular(_: &File) -> io::Result<()> {
    unavailable()
}
pub fn identity(_: &File) -> io::Result<(u64, u64)> {
    unavailable()
}
