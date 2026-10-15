use super::{CacheRead, CacheWrite, metal_private::Directory};
use sha2::{Digest, Sha256};
use std::{
    io,
    path::{Path, PathBuf},
    sync::Arc,
};
use wgpu_hal::metal::binary_archive::{self as native, Mode};
const MAGIC: &[u8; 8] = b"INCMARC1";
const HEADER: usize = 8 + 32 + 8 + 32;
pub(super) const MAX_BYTES: usize = 64 * 1024 * 1024;
#[cfg(feature = "native-compilation")]
#[path = "metal_diagnostic.rs"]
mod diagnostic;
#[cfg(feature = "native-compilation")]
pub use diagnostic::MetalArchiveDiagnosticError;

/// These are actual native outcomes. Ordinary attached creation is not a hit.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MetalArchiveReport {
    pub load_error: Option<native::Error>,
    pub initialization_error: Option<native::Error>,
    pub native: native::Counts,
}
pub(super) struct Session {
    archive: Option<Arc<native::Session>>,
    store: Option<Store>,
    report: MetalArchiveReport,
    #[cfg(feature = "native-compilation")]
    strict_errors: Option<Arc<std::sync::Mutex<Option<String>>>>,
    #[cfg(feature = "native-compilation")]
    strict_directory: Option<Directory>,
}
impl Session {
    pub fn new(device: &wgpu::Device, adapter: &wgpu::Adapter) -> (Self, CacheRead) {
        let mut session = Self {
            archive: None,
            store: None,
            report: MetalArchiveReport::default(),
            #[cfg(feature = "native-compilation")]
            strict_errors: None,
            #[cfg(feature = "native-compilation")]
            strict_directory: None,
        };
        let Some(directory) = application_directory() else {
            return (session, CacheRead::NoDirectory);
        };
        // SAFETY: scoped borrow of the requested device; no second native owner.
        let Some(hal) = (unsafe { device.as_hal::<wgpu::hal::api::Metal>() }) else {
            return (session, CacheRead::Unsupported);
        };
        let store = match Store::open(
            &directory,
            identity(&adapter.get_info(), &hal.binary_archive_identity()),
        ) {
            Ok(store) => store,
            Err(error) => return (session, CacheRead::Io(error.kind())),
        };
        let (input, mut read) = store.load();
        if matches!(read, CacheRead::Io(_)) {
            return (session, read);
        }
        // SAFETY: only this host store supplies native URLs. load() has checked
        // every ancestor, Darwin ACL, owner, mode, file kind/link count, bound,
        // source/device/OS identity and checksum. The tree is held and checked
        // again after the native call. Same-user hostile code is out of scope.
        let mut created = unsafe { hal.begin_binary_archive(input.as_deref(), Mode::Capture) };
        if input.is_some()
            && let Err(error) = created
        {
            session.report.load_error = Some(error);
            read = CacheRead::Rejected;
            // SAFETY: an empty in-memory archive accepts no native input bytes.
            created = unsafe { hal.begin_binary_archive(None, Mode::Capture) };
        }
        match created {
            Ok(archive) => {
                if let Err(error) = store.directory.validate() {
                    archive.disable();
                    session.report.native = archive.counts();
                    return (session, CacheRead::Io(error.kind()));
                }
                session.archive = Some(archive);
                session.store = Some(store);
            }
            Err(error) => session.report.initialization_error = Some(error),
        }
        (session, read)
    }
    pub fn report(&self) -> MetalArchiveReport {
        let mut report = self.report.clone();
        if let Some(archive) = &self.archive {
            report.native = archive.counts();
        }
        report
    }
    pub fn persist(&mut self) -> CacheWrite {
        let (Some(archive), Some(store)) = (&self.archive, &self.store) else {
            return CacheWrite::NotAttempted;
        };
        if archive.counts().disabled {
            return CacheWrite::NoData;
        }
        match store.save(|path| {
            // SAFETY: export reserves an exclusive private file under the held,
            // admitted URL tree and bounds/revalidates native output afterward.
            unsafe { archive.serialize(path) }
                .map_err(|error| io::Error::other(format!("Metal archive: {error:?}")))
        }) {
            Ok(bytes) => CacheWrite::Stored { bytes },
            Err(error) => {
                // Rejected paths, disk/permission errors and driver failures
                // disable this optimization; subsequent pipelines still work.
                archive.disable();
                CacheWrite::Io(error.kind())
            }
        }
    }
}
fn application_directory() -> Option<PathBuf> {
    let absolute = |key| {
        std::env::var_os(key)
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
    };
    absolute("XDG_CACHE_HOME")
        .or_else(|| absolute("HOME").map(|p| p.join("Library/Caches")))
        .map(|p| p.join("incant/metal-pipelines-v1"))
}
fn identity(adapter: &wgpu::AdapterInfo, native: &str) -> [u8; 32] {
    let mut digest = Sha256::new();
    for field in [
        "incant-metal-archive-v1",
        "wgpu29.0.4",
        env!("INCANT_METAL_ARCHIVE_SOURCE"),
        native,
        &format!("{:?}", adapter.backend),
        &adapter.vendor.to_string(),
        &adapter.device.to_string(),
        &adapter.name,
        &adapter.driver,
        &adapter.driver_info,
    ] {
        digest.update((field.len() as u64).to_le_bytes());
        digest.update(field.as_bytes());
    }
    digest.finalize().into()
}
struct Store {
    directory: Directory,
    identity: [u8; 32],
    raw: String,
    metadata: String,
}
impl Store {
    fn open(path: &Path, identity: [u8; 32]) -> io::Result<Self> {
        Self::open_with_create(path, identity, true)
    }
    fn open_with_create(path: &Path, identity: [u8; 32], create: bool) -> io::Result<Self> {
        let name: String = identity.iter().map(|b| format!("{b:02x}")).collect();
        Ok(Self {
            directory: Directory::open(path, create)?,
            identity,
            raw: format!("{name}.metalarc"),
            metadata: format!("{name}.identity"),
        })
    }
    fn load(&self) -> (Option<PathBuf>, CacheRead) {
        match self.read() {
            Ok(Some((path, bytes))) => (Some(path), CacheRead::Supplied { bytes }),
            Ok(None) => (None, CacheRead::Rejected),
            Err(error) if error.kind() == io::ErrorKind::NotFound => (None, CacheRead::Missing),
            Err(error) => (None, CacheRead::Io(error.kind())),
        }
    }
    fn read(&self) -> io::Result<Option<(PathBuf, usize)>> {
        let metadata = self.directory.read(&self.metadata, HEADER)?;
        let data = self.directory.read(&self.raw, MAX_BYTES)?;
        if !matches(&metadata, &self.identity, &data) {
            return Ok(None);
        }
        Ok(Some((self.directory.path(&self.raw)?, data.len())))
    }
    fn save(&self, write: impl FnOnce(&Path) -> io::Result<()>) -> io::Result<usize> {
        let data = self.directory.export(&self.raw, MAX_BYTES, write)?;
        // A crash or another publisher between these two atomic renames can
        // cause a hash mismatch. Admission then goes cold; it never trusts a
        // mismatched pair. Fixed names avoid unbounded archive generations.
        self.directory
            .publish(&self.metadata, &metadata(&self.identity, &data))?;
        Ok(data.len())
    }
}
fn metadata(identity: &[u8; 32], data: &[u8]) -> Vec<u8> {
    [
        MAGIC.as_slice(),
        identity,
        &(data.len() as u64).to_le_bytes(),
        &Sha256::digest(data),
    ]
    .concat()
}
fn matches(header: &[u8], identity: &[u8; 32], data: &[u8]) -> bool {
    !data.is_empty()
        && data.len() <= MAX_BYTES
        && header.len() == HEADER
        && &header[..8] == MAGIC
        && &header[8..40] == identity
        && header[40..48] == (data.len() as u64).to_le_bytes()
        && header[48..] == Sha256::digest(data)[..]
}

#[cfg(test)]
#[path = "metal_tests.rs"]
mod tests;
