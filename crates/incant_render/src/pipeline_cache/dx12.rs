//! Native D3D12 library ownership and cold-only persistence. Ordinary capture
//! falls back explicitly; the strict diagnostic never turns fallback into a hit.
use super::{
    CacheRead, CacheWrite, library_codec as codec,
    windows_private::{self, Directory},
};
use std::{io, path::Path, sync::Arc};
use wgpu_hal::dx12::pipeline_library::{self as native, Config, Mode};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Dx12LibraryReport {
    pub load_error: Option<native::Error>,
    pub initialization_error: Option<native::Error>,
    pub native: native::Counts,
}
#[derive(Debug, thiserror::Error)]
pub enum Dx12LibraryError {
    #[error("DX12 pipeline library is unavailable for this backend")]
    Backend,
    #[error("private DX12 cache admission: {0}")]
    Storage(#[from] io::Error),
    #[error("strict DX12 reuse requires an exact existing identity-bound library")]
    Missing,
    #[error("strict DX12 input admission refused: {0:?}")]
    Admission(CacheRead),
    #[error("strict DX12 pipeline membership refused: {0:?}")]
    Refused(Box<native::Counts>),
    #[error("strict DX12 device error: {0}")]
    Device(String),
    #[error(transparent)]
    Native(#[from] native::Error),
}
pub(crate) struct Session {
    library: Option<Arc<native::Session>>,
    store: Option<Store>,
    report: Dx12LibraryReport,
    strict: bool,
    strict_errors: Option<Arc<std::sync::Mutex<Option<String>>>>,
}
#[derive(Default)]
pub(crate) struct Options {
    pub config: Config,
    // Tests may use only their owned fixture, never project/native input. This
    // path override is absent from production APIs and all script interfaces.
    #[cfg(test)]
    pub directory: Option<std::path::PathBuf>,
    #[cfg(test)]
    pub observed: Option<Arc<std::sync::Mutex<Option<Arc<native::Session>>>>>,
}
impl Session {
    fn empty() -> Self {
        Self {
            library: None,
            store: None,
            report: Dx12LibraryReport::default(),
            strict: false,
            strict_errors: None,
        }
    }
    pub fn configured(
        device: &wgpu::Device,
        adapter: &wgpu::Adapter,
        options: Options,
    ) -> Result<(Self, CacheRead), Dx12LibraryError> {
        let application = || windows_private::application_directory();
        #[cfg(test)]
        let directory = options.directory.map_or_else(application, Ok);
        #[cfg(not(test))]
        let directory = application();
        let directory = match directory {
            Ok(path) => path,
            Err(error) if options.config.mode == Mode::Capture => {
                return Ok((Self::empty(), CacheRead::Io(error.kind())));
            }
            Err(error) => return Err(error.into()),
        };
        let result = if options.config.mode == Mode::Capture {
            Self::capture(device, adapter, &directory, options.config)
        } else {
            Self::require_hits(device, adapter, &directory, options.config)?
        };
        #[cfg(test)]
        if let Some(observer) = options.observed {
            *observer.lock().expect("diagnostic observer poisoned") = result.0.library.clone();
        }
        Ok(result)
    }
    fn capture(
        device: &wgpu::Device,
        adapter: &wgpu::Adapter,
        path: &Path,
        config: Config,
    ) -> (Self, CacheRead) {
        let mut session = Self::empty();
        let Some(hal) = (unsafe { device.as_hal::<wgpu::hal::api::Dx12>() }) else {
            return (session, CacheRead::Unsupported);
        };
        let identity = match identity(&adapter.get_info(), &hal) {
            Ok(id) => id,
            Err(error) => {
                session.report.initialization_error = Some(error);
                return (session, CacheRead::Unsupported);
            }
        };
        let store = match Store::open(path, identity, true) {
            Ok(store) => store,
            Err(error) => return (session, CacheRead::Io(error.kind())),
        };
        let (input, mut read) = store.load();
        if matches!(read, CacheRead::Io(_)) {
            return (session, read);
        }
        let supplied = input.is_some();
        // SAFETY: only the held owner/private-DACL, single-link store supplies
        // bytes, with exact source/adapter/driver identity and bounded checksum.
        let mut created = unsafe { hal.begin_pipeline_library(input.unwrap_or_default(), config) };
        if supplied && let Err(error) = created {
            session.report.load_error = Some(error);
            read = CacheRead::Rejected;
            // Empty cold fallback accepts no external native data.
            created = unsafe { hal.begin_pipeline_library(Vec::new(), config) };
        }
        match created {
            Ok(library) => {
                if let Err(error) = store.directory.validate() {
                    library.disable();
                    session.report.native = library.counts();
                    return (session, CacheRead::Io(error.kind()));
                }
                session.library = Some(library);
                session.store = Some(store);
            }
            Err(error) => session.report.initialization_error = Some(error),
        }
        (session, read)
    }
    pub fn require_hits(
        device: &wgpu::Device,
        adapter: &wgpu::Adapter,
        path: &Path,
        config: Config,
    ) -> Result<(Self, CacheRead), Dx12LibraryError> {
        if config.mode != Mode::RequireHits {
            return Err(Dx12LibraryError::Missing);
        }
        let Some(hal) = (unsafe { device.as_hal::<wgpu::hal::api::Dx12>() }) else {
            return Err(Dx12LibraryError::Backend);
        };
        let store = Store::open(path, identity(&adapter.get_info(), &hal)?, false)?;
        let (input, read) = store.load();
        let Some(input) = input else {
            return Err(Dx12LibraryError::Admission(read));
        };
        let library = unsafe { hal.begin_pipeline_library(input, config) }?;
        store.directory.validate()?;
        // Strict native failures can be wgpu Internal errors, outside the
        // existing validation scopes. Retain the first error on this diagnostic
        // device and turn it into a cold-completion refusal, never a frame panic.
        let errors = Arc::new(std::sync::Mutex::new(None));
        let recorded = Arc::clone(&errors);
        device.on_uncaptured_error(Arc::new(move |error: wgpu::Error| {
            let mut first = recorded.lock().unwrap_or_else(|p| p.into_inner());
            if first.is_none() {
                *first = Some(error.to_string());
            }
        }));
        Ok((
            Self {
                library: Some(library),
                store: Some(store),
                report: Dx12LibraryReport::default(),
                strict: true,
                strict_errors: Some(errors),
            },
            read,
        ))
    }
    pub fn report(&self) -> Dx12LibraryReport {
        let mut report = self.report.clone();
        if let Some(library) = &self.library {
            report.native = library.counts();
        }
        report
    }
    pub fn check_strict_completion(&self) -> crate::Result<()> {
        if !self.strict {
            return Ok(());
        }
        self.store
            .as_ref()
            .ok_or(Dx12LibraryError::Missing)?
            .directory
            .validate()
            .map_err(Dx12LibraryError::Storage)?;
        let library = self.library.as_ref().ok_or(Dx12LibraryError::Missing)?;
        library
            .check_strict_completion()
            .map_err(Dx12LibraryError::Native)?;
        check_counts(&library.counts())?;
        if let Some(error) = self
            .strict_errors
            .as_ref()
            .and_then(|errors| errors.lock().unwrap_or_else(|p| p.into_inner()).clone())
        {
            return Err(Dx12LibraryError::Device(error).into());
        }
        Ok(())
    }
    pub fn persist(&mut self) -> CacheWrite {
        if self.strict {
            return CacheWrite::NotAttempted;
        }
        let (Some(library), Some(store)) = (&self.library, &self.store) else {
            return CacheWrite::NotAttempted;
        };
        let result: io::Result<usize> = (|| {
            let bytes = library
                .serialize()
                .map_err(|e| io::Error::other(e.to_string()))?;
            store.save(&bytes)?;
            Ok(bytes.len())
        })();
        match result {
            Ok(bytes) => CacheWrite::Stored { bytes },
            Err(error) => {
                library.disable();
                CacheWrite::Io(error.kind())
            }
        }
    }
}
fn check_counts(counts: &native::Counts) -> Result<(), Dx12LibraryError> {
    let hits = counts
        .stream_hits
        .checked_add(counts.classic_hits)
        .and_then(|n| n.checked_add(counts.compute_hits));
    if !matches!(hits, Some(1..))
        || counts.disabled
        || counts.overflowed
        || counts.miss_or_mismatch != 0
        || counts.native_load_errors != 0
        || counts.fallback_attempts != 0
        || counts.stores != 0
        || counts.store_errors != 0
        || counts.strict_refusals != 0
        || counts.serialization_errors != 0
        || counts.last_error.is_some()
    {
        return Err(Dx12LibraryError::Refused(Box::new(counts.clone())));
    }
    Ok(())
}
fn identity(
    adapter: &wgpu::AdapterInfo,
    hal: &wgpu_hal::dx12::Device,
) -> Result<[u8; 32], native::Error> {
    let native = hal.pipeline_library_identity()?;
    Ok(codec::identity(&[
        "wgpu29.0.4",
        env!("INCANT_DX12_LIBRARY_SOURCE"),
        &native,
        &format!("{:?}", adapter.backend),
        &adapter.vendor.to_string(),
        &adapter.device.to_string(),
        &adapter.name,
        &adapter.driver,
        &adapter.driver_info,
    ]))
}
struct Store {
    directory: Directory,
    identity: [u8; 32],
}
impl Store {
    fn open(path: &Path, identity: [u8; 32], create: bool) -> io::Result<Self> {
        Ok(Self {
            directory: Directory::open(path, create)?,
            identity,
        })
    }
    fn load(&self) -> (Option<Vec<u8>>, CacheRead) {
        match self
            .directory
            .read("current.library", codec::HEADER + codec::MAX_BYTES)
        {
            Ok(bytes) => match codec::decode(&self.identity, &bytes) {
                Some(data) => (
                    Some(data.to_vec()),
                    CacheRead::Supplied { bytes: data.len() },
                ),
                None => (None, CacheRead::Rejected),
            },
            Err(error) if error.kind() == io::ErrorKind::NotFound => (None, CacheRead::Missing),
            Err(error) => (None, CacheRead::Io(error.kind())),
        }
    }
    fn save(&self, bytes: &[u8]) -> io::Result<()> {
        let envelope = codec::encode(&self.identity, bytes)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "DX12 library size"))?;
        self.directory.publish("current.library", &envelope)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strict_contract_accepts_only_real_hits_and_refuses_all_fallback_error_states() {
        let valid = native::Counts {
            stream_hits: 2,
            compute_hits: 1,
            ..Default::default()
        };
        assert!(check_counts(&valid).is_ok());
        assert!(check_counts(&native::Counts::default()).is_err());
        for field in [
            |c: &mut native::Counts| c.miss_or_mismatch = 1,
            |c: &mut native::Counts| c.native_load_errors = 1,
            |c: &mut native::Counts| c.fallback_attempts = 1,
            |c: &mut native::Counts| c.stores = 1,
            |c: &mut native::Counts| c.store_errors = 1,
            |c: &mut native::Counts| c.strict_refusals = 1,
            |c: &mut native::Counts| c.serialization_errors = 1,
            |c: &mut native::Counts| c.disabled = true,
            |c: &mut native::Counts| c.overflowed = true,
            |c: &mut native::Counts| c.last_error = Some(native::Error::Disabled),
            |c: &mut native::Counts| {
                c.stream_hits = u64::MAX;
                c.compute_hits = 1;
            },
        ] {
            let mut refused = valid.clone();
            field(&mut refused);
            assert!(matches!(
                check_counts(&refused),
                Err(Dx12LibraryError::Refused(_))
            ));
        }
    }
}
