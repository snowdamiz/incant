//! Opt-in host-owned D3D12 pipeline libraries, not wgpu PipelineCache.
//! No storage I/O here. The host admits native bytes and explicitly serializes
//! after successful cold catalog preparation. All same-name loads are locked.
use super::{library_key, pipeline_desc::RenderPipelineStateStreamDesc};
use alloc::{boxed::Box, format, string::String, sync::Arc, vec::Vec};
use core::mem::size_of;
use parking_lot::Mutex;
use windows::{
    core::{Interface, PCWSTR},
    Win32::Graphics::{Direct3D12::*, Dxgi},
};

pub const MAX_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Capture,
    /// No normal creation or StorePipeline, including after cache failure.
    RequireHits,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderPath {
    Automatic,
    /// Trusted diagnostic to exercise the classic descriptor on modern hosts.
    Classic,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Config {
    pub mode: Mode,
    pub render_path: RenderPath,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            mode: Mode::Capture,
            render_path: RenderPath::Automatic,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("D3D12 pipeline libraries unsupported")]
    Unsupported,
    #[error("a D3D12 library is already attached to this device")]
    AlreadyAttached,
    #[error("D3D12 pipeline library disabled")]
    Disabled,
    #[error("classic descriptor cannot represent mesh/task/view-instancing state")]
    UnsupportedClassicDescriptor,
    #[error("D3D12 library1 stream loading unsupported")]
    UnsupportedStreamLibrary,
    #[error("D3D12 native operation {operation} failed: HRESULT {code:#x}")]
    Native { operation: &'static str, code: i32 },
    #[error("D3D12 library size is empty or exceeds its bound")]
    Size,
    #[error("D3D12 library evidence counter overflow")]
    CounterOverflow,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub stream_hits: u64,
    pub classic_hits: u64,
    pub compute_hits: u64,
    /// E_INVALIDARG means missing name OR native descriptor mismatch.
    pub miss_or_mismatch: u64,
    pub native_load_errors: u64,
    pub fallback_attempts: u64,
    pub stores: u64,
    pub store_errors: u64,
    pub strict_refusals: u64,
    pub serialization_errors: u64,
    pub disabled: bool,
    pub overflowed: bool,
    pub last_error: Option<Error>,
}
struct Library {
    // Declaration/drop order is deliberate: EVERY COM interface dies before
    // backing, which CreatePipelineLibrary retains by pointer without copying.
    extended: Option<ID3D12PipelineLibrary1>,
    raw: ID3D12PipelineLibrary,
    _backing: Box<[u8]>,
}
struct State {
    library: Library,
    config: Config,
    counts: Counts,
}
pub struct Session(Mutex<State>);
impl State {
    fn count(&mut self, field: fn(&mut Counts) -> &mut u64) {
        count(&mut self.counts, field);
    }
}
fn count(counts: &mut Counts, field: fn(&mut Counts) -> &mut u64) {
    let slot = field(counts);
    if let Some(next) = slot.checked_add(1) {
        *slot = next;
    } else {
        counts.overflowed = true;
        counts.disabled = true;
        counts.last_error = Some(Error::CounterOverflow);
    }
}
fn native(operation: &'static str, error: windows::core::Error) -> Error {
    Error::Native {
        operation,
        code: error.code().0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn counters_never_wrap_and_disable_reuse_after_overflow() {
        let mut counts = Counts {
            stream_hits: u64::MAX,
            ..Default::default()
        };
        count(&mut counts, |c| &mut c.stream_hits);
        assert_eq!(counts.stream_hits, u64::MAX);
        assert!(counts.disabled && counts.overflowed);
        assert_eq!(counts.last_error, Some(Error::CounterOverflow));
        count(&mut counts, |c| &mut c.strict_refusals);
        assert_eq!(counts.strict_refusals, 1);
        assert_eq!(
            name([0xab; 32]),
            alloc::format!("{}\0", "ab".repeat(32))
                .encode_utf16()
                .collect::<Vec<_>>()
        );
    }
}
fn name(key: [u8; 32]) -> Vec<u16> {
    let mut name = Vec::with_capacity(65);
    for byte in key {
        name.push(u16::from(b"0123456789abcdef"[usize::from(byte >> 4)]));
        name.push(u16::from(b"0123456789abcdef"[usize::from(byte & 15)]));
    }
    name.push(0);
    name
}
impl super::Device {
    fn check_library_support(&self) -> Result<(), Error> {
        let mut support = D3D12_FEATURE_DATA_SHADER_CACHE::default();
        unsafe {
            self.raw.CheckFeatureSupport(
                D3D12_FEATURE_SHADER_CACHE,
                (&mut support as *mut D3D12_FEATURE_DATA_SHADER_CACHE).cast(),
                size_of::<D3D12_FEATURE_DATA_SHADER_CACHE>() as u32,
            )
        }
        .map_err(|_| Error::Unsupported)?;
        if support
            .SupportFlags
            .contains(D3D12_SHADER_CACHE_SUPPORT_LIBRARY)
            && self.raw.cast::<ID3D12Device1>().is_ok()
        {
            Ok(())
        } else {
            Err(Error::Unsupported)
        }
    }
    pub fn pipeline_library_identity(&self) -> Result<String, Error> {
        self.check_library_support()?;
        // Query the real driver version; never silently key an unknown driver
        // as zero. Native CreatePipelineLibrary also validates adapter/driver.
        let adapter = &self.shared.adapter;
        let desc = unsafe { adapter.GetDesc2() }.map_err(|e| native("GetDesc2", e))?;
        let version = unsafe { adapter.CheckInterfaceSupport(&Dxgi::IDXGIDevice::IID) }
            .map_err(|e| native("DriverVersion", e))?;
        Ok(format!(
            "{}:{}:{}:{}:{}:{}:{}",
            desc.VendorId,
            desc.DeviceId,
            desc.SubSysId,
            desc.Revision,
            desc.AdapterLuid.HighPart,
            desc.AdapterLuid.LowPart,
            version
        ))
    }
    /// # Safety
    /// `input` is only engine-produced, identity/checksum-bound private cache
    /// data, or a fixed trusted diagnostic. Never project/downloaded bytes.
    pub unsafe fn begin_pipeline_library(
        &self,
        input: Vec<u8>,
        config: Config,
    ) -> Result<Arc<Session>, Error> {
        if input.len() > MAX_BYTES {
            return Err(Error::Size);
        }
        let mut installed = self.pipeline_library.lock();
        if installed.upgrade().is_some() {
            return Err(Error::AlreadyAttached);
        }
        self.check_library_support()?;
        let device = self
            .raw
            .cast::<ID3D12Device1>()
            .map_err(|_| Error::Unsupported)?;
        let backing = input.into_boxed_slice();
        // The slice binding supplies a dangling nonnull pointer for []. Use the
        // documented null/zero pair for a new empty library instead.
        let mut result = core::ptr::null_mut();
        unsafe {
            (Interface::vtable(&device).CreatePipelineLibrary)(
                Interface::as_raw(&device),
                if backing.is_empty() {
                    core::ptr::null()
                } else {
                    backing.as_ptr().cast()
                },
                backing.len(),
                &ID3D12PipelineLibrary::IID,
                &mut result,
            )
            .ok()
        }
        .map_err(|e| native("CreatePipelineLibrary", e))?;
        if result.is_null() {
            return Err(Error::Native {
                operation: "CreatePipelineLibrary",
                code: -2147467261,
            });
        }
        // SAFETY: successful COM creation transferred one nonnull owned ref.
        let raw = unsafe { ID3D12PipelineLibrary::from_raw(result) };
        let extended = raw.cast().ok();
        let session = Arc::new(Session(Mutex::new(State {
            library: Library {
                extended,
                raw,
                _backing: backing,
            },
            config,
            counts: Counts::default(),
        })));
        *installed = Arc::downgrade(&session);
        Ok(session)
    }
    pub(super) fn pipeline_library(&self) -> Option<Arc<Session>> {
        self.pipeline_library.lock().upgrade()
    }
}
impl Session {
    pub fn counts(&self) -> Counts {
        self.0.lock().counts.clone()
    }
    pub fn disable(&self) {
        self.0.lock().counts.disabled = true;
    }
    /// Native counters are meaningful only if the complete cold preparation
    /// succeeded; this also refuses a strict route after evidence overflow.
    pub fn check_strict_completion(&self) -> Result<(), Error> {
        let state = self.0.lock();
        if state.config.mode == Mode::RequireHits
            && (state.counts.disabled || state.counts.overflowed)
        {
            Err(state.counts.last_error.clone().unwrap_or(Error::Disabled))
        } else {
            Ok(())
        }
    }
    pub fn serialize(&self) -> Result<Vec<u8>, Error> {
        let mut state = self.0.lock();
        if state.counts.disabled || state.config.mode != Mode::Capture {
            return Err(Error::Disabled);
        }
        let size = unsafe { state.library.raw.GetSerializedSize() };
        if size == 0 || size > MAX_BYTES {
            return Err(Error::Size);
        }
        let mut bytes = alloc::vec![0; size];
        if let Err(error) = unsafe { state.library.raw.Serialize(&mut bytes) } {
            let error = native("Serialize", error);
            state.count(|c| &mut c.serialization_errors);
            state.counts.last_error = Some(error.clone());
            return Err(error);
        }
        Ok(bytes)
    }
    fn pipeline(
        &self,
        key: [u8; 32],
        hit: fn(&mut Counts) -> &mut u64,
        load: impl FnOnce(&Library, PCWSTR) -> Result<ID3D12PipelineState, Error>,
        create: impl FnOnce() -> Result<ID3D12PipelineState, Error>,
    ) -> Result<ID3D12PipelineState, Error> {
        // Serialize load/create/store and serialization, including concurrent
        // requests for the same name. No root/authoring lock is acquired here.
        let mut state = self.0.lock();
        if state.counts.disabled {
            if state.config.mode == Mode::RequireHits {
                state.count(|c| &mut c.strict_refusals);
                return Err(Error::Disabled);
            }
            state.count(|c| &mut c.fallback_attempts);
            return create();
        }
        let name = name(key);
        let name = PCWSTR(name.as_ptr());
        match load(&state.library, name) {
            Ok(pipeline) => {
                state.count(hit);
                return Ok(pipeline);
            }
            Err(error) => {
                if matches!(
                    error,
                    Error::Native {
                        code: -2147024809,
                        ..
                    }
                ) {
                    state.count(|c| &mut c.miss_or_mismatch);
                } else {
                    state.count(|c| &mut c.native_load_errors);
                }
                state.counts.last_error = Some(error.clone());
                if state.config.mode == Mode::RequireHits {
                    state.count(|c| &mut c.strict_refusals);
                    return Err(error);
                }
            }
        }
        state.count(|c| &mut c.fallback_attempts);
        let pipeline = create()?;
        // Names cannot be overwritten. Store failure is retained independently
        // of successful normal creation and is never described as a cache hit.
        match unsafe { state.library.raw.StorePipeline(name, &pipeline) } {
            Ok(()) => state.count(|c| &mut c.stores),
            Err(error) => {
                state.count(|c| &mut c.store_errors);
                state.counts.last_error = Some(native("StorePipeline", error));
            }
        }
        Ok(pipeline)
    }
    pub(super) unsafe fn render(
        &self,
        device: &ID3D12Device,
        descriptor: &RenderPipelineStateStreamDesc<'_>,
        root: &[u8; 32],
    ) -> Result<ID3D12PipelineState, Error> {
        let path = self.0.lock().config.render_path;
        let stream_device = if path == RenderPath::Automatic {
            device.cast::<ID3D12Device2>().ok()
        } else {
            None
        };
        let key = unsafe { library_key::render(descriptor, root) };
        if let Some(device) = stream_device {
            self.pipeline(
                key,
                |c| &mut c.stream_hits,
                |library, name| {
                    let library = library
                        .extended
                        .as_ref()
                        .ok_or(Error::UnsupportedStreamLibrary)?;
                    unsafe { descriptor.to_stream().load_pipeline_state(library, name) }
                        .map_err(|e| native("LoadPipeline", e))
                },
                || {
                    unsafe { descriptor.to_stream().create_pipeline_state(&device) }
                        .map_err(|e| native("CreatePipelineState", e))
                },
            )
        } else {
            if descriptor.mesh_shader.BytecodeLength != 0
                || descriptor.task_shader.BytecodeLength != 0
                || descriptor.view_instancing.is_some()
            {
                return Err(Error::UnsupportedClassicDescriptor);
            }
            let classic = unsafe { descriptor.to_graphics_pipeline_descriptor() };
            self.pipeline(
                key,
                |c| &mut c.classic_hits,
                |library, name| {
                    unsafe { library.raw.LoadGraphicsPipeline(name, &classic) }
                        .map_err(|e| native("LoadGraphicsPipeline", e))
                },
                || {
                    unsafe { device.CreateGraphicsPipelineState(&classic) }
                        .map_err(|e| native("CreateGraphicsPipelineState", e))
                },
            )
        }
    }
    pub(super) unsafe fn compute(
        &self,
        device: &ID3D12Device,
        descriptor: &D3D12_COMPUTE_PIPELINE_STATE_DESC,
        root: &[u8; 32],
    ) -> Result<ID3D12PipelineState, Error> {
        let key = unsafe { library_key::compute(descriptor, root) };
        self.pipeline(
            key,
            |c| &mut c.compute_hits,
            |library, name| {
                unsafe { library.raw.LoadComputePipeline(name, descriptor) }
                    .map_err(|e| native("LoadComputePipeline", e))
            },
            || {
                unsafe { device.CreateComputePipelineState(descriptor) }
                    .map_err(|e| native("CreateComputePipelineState", e))
            },
        )
    }
}
