//! Explicit native Metal archives. This does not implement wgpu's PipelineCache.
//! The trusted host owns URL admission and cold-only persistence. Mesh and tile
//! descriptors are intentionally outside this first route.
use alloc::{
    string::{String, ToString},
    sync::Arc,
};
use objc2::{available, rc::Retained, runtime::ProtocolObject};
use objc2_foundation::{NSArray, NSError, NSProcessInfo, NSString, NSURL};
use objc2_metal::{
    MTLBinaryArchive, MTLBinaryArchiveDescriptor, MTLComputePipelineDescriptor,
    MTLComputePipelineState, MTLDevice, MTLPipelineOption, MTLRenderPipelineDescriptor,
    MTLRenderPipelineState,
};
use parking_lot::Mutex;
use std::path::Path;

type Native<T> = Result<Retained<ProtocolObject<T>>, Retained<NSError>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Ordinary compilation fallback plus explicit function capture.
    Capture,
    /// Trusted diagnostic: no capture and no fallback. Never a production default.
    RequireHit,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeError {
    pub domain: String,
    pub code: i64,
}
impl From<&Retained<NSError>> for NativeError {
    fn from(error: &Retained<NSError>) -> Self {
        Self {
            domain: error.domain().to_string(),
            code: error.code() as i64,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Unsupported,
    AlreadyAttached,
    InvalidPath,
    Disabled,
    Native(NativeError),
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub render_attached: u64,
    pub compute_attached: u64,
    pub render_added: u64,
    pub compute_added: u64,
    pub require_succeeded: u64,
    pub require_failed: u64,
    pub disabled_require_refusals: u64,
    pub fallback_attempts: u64,
    pub addition_failures: u64,
    pub serialization_failures: u64,
    pub disabled: bool,
    pub last_error: Option<NativeError>,
    pub render_error: Option<NativeError>,
    pub compute_error: Option<NativeError>,
}
struct State {
    raw: Retained<ProtocolObject<dyn MTLBinaryArchive>>,
    mode: Mode,
    counts: Counts,
}
/// Every native archive operation, including attached creation, is serialized.
/// The native device keeps only a Weak reference; this owner does no file I/O in Drop.
pub struct Session(Mutex<State>);
// SAFETY: Metal objects support cross-thread use. All calls involving this
// archive are serialized by the mutex; descriptors remain local to the caller.
unsafe impl Send for State {}

fn url(path: &Path) -> Result<Retained<NSURL>, Error> {
    let path = path
        .to_str()
        .filter(|s| path.is_absolute() && !s.contains('\0'))
        .ok_or(Error::InvalidPath)?;
    Ok(NSURL::fileURLWithPath(&NSString::from_str(path)))
}
impl super::Device {
    /// Device/OS identity supplements the host's source, feature and adapter key.
    pub fn binary_archive_identity(&self) -> String {
        format!(
            "{}:{}",
            self.shared.device.registryID(),
            NSProcessInfo::processInfo().operatingSystemVersionString()
        )
    }
    /// Install one native session before cold pipeline construction.
    ///
    /// # Safety
    /// `input`, when present, must be an owner-private, bounded, identity-bound
    /// archive produced by Metal (or a fixed host-authored invalid-cache control).
    /// The host must validate its complete URL path
    /// and file ownership/ACLs, and maintain that admission during this call.
    /// Never pass project, downloaded, or arbitrary user-supplied native bytes.
    pub unsafe fn begin_binary_archive(
        &self,
        input: Option<&Path>,
        mode: Mode,
    ) -> Result<Arc<Session>, Error> {
        if !available!(macos = 11.0) {
            return Err(Error::Unsupported);
        }
        let mut installed = self.binary_archive.lock();
        if installed.upgrade().is_some() {
            return Err(Error::AlreadyAttached);
        }
        let descriptor = MTLBinaryArchiveDescriptor::new();
        if let Some(input) = input {
            let input_url = url(input)?;
            descriptor.setUrl(Some(&input_url));
        }
        let raw = self
            .shared
            .device
            .newBinaryArchiveWithDescriptor_error(&descriptor)
            .map_err(|e| Error::Native((&e).into()))?;
        let session = Arc::new(Session(Mutex::new(State {
            raw,
            mode,
            counts: Counts::default(),
        })));
        *installed = Arc::downgrade(&session);
        Ok(session)
    }
    pub(super) fn archive_render(
        &self,
        descriptor: &MTLRenderPipelineDescriptor,
        mut create: impl FnMut(MTLPipelineOption) -> Native<dyn MTLRenderPipelineState>,
    ) -> Native<dyn MTLRenderPipelineState> {
        let session = self.binary_archive.lock().upgrade();
        match session {
            Some(session) => session.create(
                true,
                |archives| descriptor.setBinaryArchives(archives),
                |archive| archive.addRenderPipelineFunctionsWithDescriptor_error(descriptor),
                create,
            ),
            None => create(MTLPipelineOption::empty()),
        }
    }
    pub(super) fn archive_compute(
        &self,
        descriptor: &MTLComputePipelineDescriptor,
        mut create: impl FnMut(MTLPipelineOption) -> Native<dyn MTLComputePipelineState>,
    ) -> Native<dyn MTLComputePipelineState> {
        let session = self.binary_archive.lock().upgrade();
        match session {
            Some(session) => session.create(
                false,
                |archives| descriptor.setBinaryArchives(archives),
                |archive| archive.addComputePipelineFunctionsWithDescriptor_error(descriptor),
                create,
            ),
            None => create(MTLPipelineOption::empty()),
        }
    }
}
impl Session {
    pub fn counts(&self) -> Counts {
        self.0.lock().counts.clone()
    }
    /// Disables future attachment/capture without any native or filesystem call.
    pub fn disable(&self) {
        self.0.lock().counts.disabled = true;
    }
    /// Serialize only at an explicit successful cold completion.
    ///
    /// # Safety
    /// The host must exclusively create and admit this output URL in a private
    /// directory, then validate/read it with a bound before publishing. Neither
    /// file permissions nor output size are guaranteed by this native call.
    pub unsafe fn serialize(&self, output: &Path) -> Result<(), Error> {
        let mut state = self.0.lock();
        if state.counts.disabled || state.mode != Mode::Capture {
            return Err(Error::Disabled);
        }
        let output_url = url(output)?;
        if let Err(error) = state.raw.serializeToURL_error(&output_url) {
            state.counts.serialization_failures += 1;
            state.counts.last_error = Some((&error).into());
            state.counts.disabled = true;
            return Err(Error::Native((&error).into()));
        }
        Ok(())
    }
    fn create<T: ?Sized>(
        &self,
        render: bool,
        attach: impl Fn(Option<&NSArray<ProtocolObject<dyn MTLBinaryArchive>>>),
        add: impl Fn(&ProtocolObject<dyn MTLBinaryArchive>) -> Result<(), Retained<NSError>>,
        mut create: impl FnMut(MTLPipelineOption) -> Native<T>,
    ) -> Native<T> {
        let mut state = self.0.lock();
        if state.counts.disabled {
            attach(None);
            if state.mode == Mode::RequireHit {
                state.counts.disabled_require_refusals += 1;
                // SAFETY: no userInfo dictionary is supplied. This domain is
                // explicitly ours: no native call or native miss is claimed.
                return Err(unsafe {
                    NSError::errorWithDomain_code_userInfo(
                        &NSString::from_str("IncantMetalArchiveDisabled"),
                        1,
                        None,
                    )
                });
            }
            return create(MTLPipelineOption::empty());
        }
        let archives = NSArray::from_slice(&[&*state.raw]);
        attach(Some(&archives));
        if render {
            state.counts.render_attached += 1;
        } else {
            state.counts.compute_attached += 1;
        }
        let options = match state.mode {
            Mode::Capture => MTLPipelineOption::empty(),
            Mode::RequireHit => MTLPipelineOption::FailOnBinaryArchiveMiss,
        };
        let result = create(options);
        attach(None);
        if state.mode == Mode::RequireHit {
            match &result {
                Ok(_) => state.counts.require_succeeded += 1,
                Err(error) => {
                    state.counts.require_failed += 1;
                    state.counts.last_error = Some(error.into());
                    if render {
                        state.counts.render_error = Some(error.into());
                    } else {
                        state.counts.compute_error = Some(error.into());
                    }
                }
            }
            return result;
        }
        if let Err(error) = &result {
            state.counts.last_error = Some(error.into());
            if render {
                state.counts.render_error = Some(error.into());
            } else {
                state.counts.compute_error = Some(error.into());
            }
            state.counts.disabled = true;
            state.counts.fallback_attempts += 1;
            // An archive problem cannot prevent ordinary pipeline construction.
            // Its actual ordinary error, if any, remains visible to the caller.
            return create(MTLPipelineOption::empty());
        }
        match add(&state.raw) {
            Ok(()) => {
                if render {
                    state.counts.render_added += 1;
                } else {
                    state.counts.compute_added += 1;
                }
            }
            Err(error) => {
                state.counts.addition_failures += 1;
                state.counts.last_error = Some((&error).into());
                state.counts.disabled = true;
            }
        }
        result
    }
}
