//! Driver caches are host-owned disposable data, never project content.
//! Loading happens once per renderer. Saving happens only after successful
//! explicit cold catalog preparation, never during submission or Drop.
#[cfg(all(
    feature = "persistent-pipeline-cache",
    unix,
    not(target_arch = "wasm32")
))]
mod private;
#[cfg(all(
    feature = "persistent-pipeline-cache",
    unix,
    not(target_arch = "wasm32")
))]
mod store;

#[cfg(all(feature = "metal-binary-archives", target_os = "macos"))]
mod darwin_acl;
#[cfg(all(feature = "metal-binary-archives", target_os = "macos"))]
mod metal;
#[cfg(all(feature = "metal-binary-archives", target_os = "macos"))]
mod metal_private;
#[cfg(all(
    feature = "metal-binary-archives",
    feature = "native-compilation",
    target_os = "macos"
))]
pub use metal::MetalArchiveDiagnosticError;
#[cfg(all(feature = "metal-binary-archives", target_os = "macos"))]
pub use metal::MetalArchiveReport;

/// Initial cache admission and the latest successful cold-preparation write attempt.
/// Supplying cache bytes does not prove that the driver had a cache hit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PipelineCacheReport {
    pub read: CacheRead,
    pub write: CacheWrite,
    #[cfg(all(feature = "metal-binary-archives", target_os = "macos"))]
    pub metal: Option<MetalArchiveReport>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CacheRead {
    Disabled,
    Unsupported,
    UnsupportedStorage,
    NoDirectory,
    Missing,
    Rejected,
    Io(std::io::ErrorKind),
    Supplied { bytes: usize },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CacheWrite {
    NotAttempted,
    NoData,
    Stored { bytes: usize },
    Io(std::io::ErrorKind),
}
pub(crate) struct Session {
    #[cfg(all(feature = "metal-binary-archives", target_os = "macos"))]
    metal: Option<metal::Session>,
    pub cache: Option<wgpu::PipelineCache>,
    pub report: PipelineCacheReport,
    #[cfg(all(
        feature = "persistent-pipeline-cache",
        unix,
        not(target_arch = "wasm32")
    ))]
    store: Option<store::Store>,
}
pub(crate) fn required_features(adapter: &wgpu::Adapter) -> wgpu::Features {
    if cfg!(all(
        feature = "persistent-pipeline-cache",
        any(target_os = "linux", target_os = "android"),
        not(target_arch = "wasm32")
    )) && adapter.get_info().backend == wgpu::Backend::Vulkan
    {
        adapter.features() & wgpu::Features::PIPELINE_CACHE
    } else {
        wgpu::Features::empty()
    }
}
impl Session {
    #[cfg(all(
        feature = "metal-binary-archives",
        feature = "native-compilation",
        target_os = "macos"
    ))]
    pub fn require_hits(device: &wgpu::Device, adapter: &wgpu::Adapter) -> crate::Result<Self> {
        let (metal, read) = metal::Session::require_hits(device, adapter)?;
        let mut session = Self::inactive(read);
        session.report.metal = Some(metal.report());
        session.metal = Some(metal);
        Ok(session)
    }
    pub fn new(device: &wgpu::Device, adapter: &wgpu::Adapter) -> Self {
        #[cfg(all(feature = "metal-binary-archives", target_os = "macos"))]
        if adapter.get_info().backend == wgpu::Backend::Metal {
            let (metal, read) = metal::Session::new(device, adapter);
            let mut session = Self::inactive(read);
            session.report.metal = Some(metal.report());
            session.metal = Some(metal);
            return session;
        }
        #[cfg(all(
            feature = "persistent-pipeline-cache",
            unix,
            not(target_arch = "wasm32")
        ))]
        {
            if adapter.get_info().backend != wgpu::Backend::Vulkan {
                return Self::inactive(CacheRead::Unsupported);
            }
            // POSIX mode ownership is not an ACL audit on Darwin. Windows also
            // needs a separate ACL implementation before saved driver bytes are trusted.
            if !cfg!(any(target_os = "linux", target_os = "android")) {
                return Self::inactive(CacheRead::UnsupportedStorage);
            }
            if !device.features().contains(wgpu::Features::PIPELINE_CACHE) {
                return Self::inactive(CacheRead::Unsupported);
            }
            let Some(directory) = store::application_directory() else {
                return Self::inactive(CacheRead::NoDirectory);
            };
            let store = store::Store::new(directory, &adapter.get_info());
            let (data, read) = store.load();
            // SAFETY: the only source is our application cache, outside projects.
            // Only get_data() output is written there, in a bounded, checksummed,
            // adapter/driver-bound envelope. The held directory and regular file
            // descriptors also enforce private ownership; hashes alone do not.
            // wgpu additionally validates its version and driver cache identity;
            // fallback permits a cold cache after upgrades. This is not an API
            // for accepting user/project/network-supplied driver bytecode.
            let cache = unsafe {
                device.create_pipeline_cache(&wgpu::PipelineCacheDescriptor {
                    label: Some("Incant startup pipeline cache"),
                    data: data.as_deref(),
                    fallback: true,
                })
            };
            Self {
                #[cfg(all(feature = "metal-binary-archives", target_os = "macos"))]
                metal: None,
                cache: Some(cache),
                report: PipelineCacheReport {
                    read,
                    write: CacheWrite::NotAttempted,
                    #[cfg(all(feature = "metal-binary-archives", target_os = "macos"))]
                    metal: None,
                },
                store: Some(store),
            }
        }
        #[cfg(not(all(
            feature = "persistent-pipeline-cache",
            unix,
            not(target_arch = "wasm32")
        )))]
        {
            let _ = (device, adapter);
            Self::inactive(if !cfg!(feature = "persistent-pipeline-cache") {
                CacheRead::Disabled
            } else if cfg!(target_arch = "wasm32")
                || adapter.get_info().backend != wgpu::Backend::Vulkan
            {
                CacheRead::Unsupported
            } else {
                CacheRead::UnsupportedStorage
            })
        }
    }
    fn inactive(read: CacheRead) -> Self {
        Self {
            #[cfg(all(feature = "metal-binary-archives", target_os = "macos"))]
            metal: None,
            cache: None,
            report: PipelineCacheReport {
                read,
                write: CacheWrite::NotAttempted,
                #[cfg(all(feature = "metal-binary-archives", target_os = "macos"))]
                metal: None,
            },
            #[cfg(all(
                feature = "persistent-pipeline-cache",
                unix,
                not(target_arch = "wasm32")
            ))]
            store: None,
        }
    }
    /// The caller has already closed the device validation scope. Rust-side
    /// preparation errors also refuse persistence; retain this device cache for
    /// later optional catalogs, with no implicit flush on Drop.
    pub fn complete<T>(&mut self, candidate: crate::Result<T>) -> crate::Result<T> {
        #[cfg(all(
            feature = "metal-binary-archives",
            feature = "native-compilation",
            target_os = "macos"
        ))]
        let candidate = candidate.and_then(|prepared| {
            if let Some(metal) = &self.metal {
                metal.check_strict_completion()?;
            }
            Ok(prepared)
        });
        complete(candidate, || self.persist())
    }
    fn persist(&mut self) {
        #[cfg(all(feature = "metal-binary-archives", target_os = "macos"))]
        if let Some(metal) = &mut self.metal {
            self.report.write = metal.persist();
            self.report.metal = Some(metal.report());
        }
        #[cfg(all(
            feature = "persistent-pipeline-cache",
            unix,
            not(target_arch = "wasm32")
        ))]
        if let (Some(cache), Some(store)) = (&self.cache, &self.store) {
            self.report.write = match cache.get_data() {
                Some(data) => match store.save(&data) {
                    Ok(()) => CacheWrite::Stored { bytes: data.len() },
                    Err(error) => CacheWrite::Io(error.kind()),
                },
                None => CacheWrite::NoData,
            };
        }
    }
}

// Keep publication after the complete preparation result, including errors
// discovered after some pipelines have already been constructed.
fn complete<T>(candidate: crate::Result<T>, persist: impl FnOnce()) -> crate::Result<T> {
    let prepared = candidate?;
    persist();
    Ok(prepared)
}

#[cfg(test)]
mod tests;
