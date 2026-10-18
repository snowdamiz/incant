//! Audited static C boundary. Traps must be caught before returning from C; C
//! import wrappers must wait for Rust callbacks to return before trapping.
use crate::{BoundaryError, Capability, limits};
use incant_runtime::NativeWorld;
use std::{ffi::c_void, ptr::NonNull};
use thiserror::Error;

pub type Import = unsafe extern "C" fn(
    *mut c_void,
    *mut u8,
    usize,
    u32,
    i64,
    i32,
    i32,
    i32,
    i32,
    *mut i32,
) -> i32;
type Create = unsafe extern "C" fn(Import, *mut c_void, *mut *mut c_void) -> i32;
type Update = unsafe extern "C" fn(*mut c_void, Import, *mut c_void, i64, f64) -> i32;
type Free = unsafe extern "C" fn(*mut c_void);
/// Trusted, build-linked module functions. Never deserialize these from a file.
#[derive(Clone, Copy)]
pub struct ModuleExports {
    create: Create,
    update: Update,
    free: Free,
    persistent: Option<crate::persistent::PersistentProgram>,
    coroutines: &'static [incant_wasm_abi::coroutines::Routine],
}
impl ModuleExports {
    /// # Safety
    /// Functions must be the reviewed C facade for validated/metered wasm2c
    /// output, compiled with compatible runtime flags. Create yields a unique
    /// instance or frees partial allocation on error; free consumes it. Update
    /// cannot retain callback/context, run concurrently or reenter the host.
    /// Calls return normally with status; no longjmp/unwind may cross Rust.
    /// Callback memory must be exclusive initialized bytes for the whole call,
    /// bounded by limits::MEMORY and unrelated to context/output. Output points
    /// to a writable i32. This grants trusted native executable code, not Wasm.
    pub const unsafe fn from_linked(create: Create, update: Update, free: Free) -> Self {
        Self {
            create,
            update,
            free,
            persistent: None,
            coroutines: &[],
        }
    }
    /// # Safety
    /// All `from_linked` obligations apply. `program` must describe the exact
    /// validated original/prepared module and trusted generated native linkage.
    /// Never obtain this descriptor or its fingerprint from checkpoint bytes.
    pub const unsafe fn from_linked_persistent(
        create: Create,
        update: Update,
        free: Free,
        program: crate::persistent::PersistentProgram,
    ) -> Self {
        Self {
            create,
            update,
            free,
            persistent: Some(program),
            coroutines: &[],
        }
    }
    /// The trusted generated linker copies declarations from the validated module.
    pub const fn with_coroutines(
        mut self,
        routines: &'static [incant_wasm_abi::coroutines::Routine],
    ) -> Self {
        self.coroutines = routines;
        self
    }
    pub const fn persistent_program(&self) -> Option<crate::persistent::PersistentProgram> {
        self.persistent
    }
}
#[derive(Debug, Error)]
pub enum StaticError {
    #[error(transparent)]
    Boundary(#[from] BoundaryError),
    #[error("static Wasm trapped (code {0})")]
    Trap(i32),
    #[error("static Wasm instruction fuel exhausted")]
    Fuel,
    #[error("static Wasm callback panicked")]
    CallbackPanic,
    #[error("invalid trusted static module facade")]
    Facade,
}
pub struct StaticModule {
    exports: ModuleExports,
    instance: NonNull<c_void>,
    capability: Capability,
}
impl StaticModule {
    pub fn new(exports: ModuleExports) -> Result<Self, StaticError> {
        let mut capability =
            Capability::with_persistence(exports.persistent.map(|p| p.declaration))
                .with_coroutines(exports.coroutines.to_vec())?;
        let mut context = Context {
            capability: &mut capability,
            error: None,
        };
        let mut instance = std::ptr::null_mut();
        // SAFETY: descriptor construction asserts the C ABI/ownership contract;
        // context is alive and exclusively borrowed through this synchronous call.
        let status = unsafe {
            (exports.create)(
                dispatch,
                (&mut context as *mut Context<'_>).cast(),
                &mut instance,
            )
        };
        if status != 0 {
            return Err(context.error.unwrap_or(if status == -3 {
                StaticError::Fuel
            } else {
                StaticError::Trap(status)
            }));
        }
        if context.error.is_some() {
            if let Some(instance) = NonNull::new(instance) {
                unsafe { (exports.free)(instance.as_ptr()) };
            }
            return Err(StaticError::Facade);
        }
        Ok(Self {
            exports,
            instance: NonNull::new(instance).ok_or(StaticError::Facade)?,
            capability,
        })
    }
    pub fn exports(&self) -> ModuleExports {
        self.exports
    }
    pub fn instance_catalog(&self) -> Option<std::sync::Arc<crate::instances::Catalog>> {
        self.capability.instance_catalog()
    }
    pub fn configure_instances(
        &mut self,
        catalog: std::sync::Arc<crate::instances::Catalog>,
        world: &NativeWorld,
    ) -> Result<(), StaticError> {
        catalog.validate_world(world)?;
        self.capability.configure_instances(catalog)?;
        if world.tick() == 0 {
            self.capability.bind_instances(world)?;
        }
        Ok(())
    }
    pub fn persistent_checkpoint(
        &self,
        world: &NativeWorld,
    ) -> Result<crate::persistent::PersistentSnapshot, StaticError> {
        Ok(self.capability.persistent_checkpoint(world)?)
    }
    pub fn checkpoint_authority(&self, world: &NativeWorld) -> Result<(i64, bool), StaticError> {
        Ok(self.capability.checkpoint_authority(world)?)
    }
    pub fn restore_persistent(
        &mut self,
        world: &NativeWorld,
        snapshot: &crate::persistent::PersistentSnapshot,
        watermark: i64,
        restart: bool,
    ) -> Result<(), StaticError> {
        Ok(self
            .capability
            .restore_persistent(world, snapshot, watermark, restart)?)
    }
    /// Construct and validate before replacing. Only explicit same-layout host
    /// bytes survive; all guest globals, memory and tables start fresh.
    pub fn reload(&mut self, exports: ModuleExports) -> Result<(), StaticError> {
        let mut candidate = Self::new(exports)?;
        if let Some(catalog) = self.instance_catalog() {
            candidate.capability.configure_instances(catalog)?;
        }
        self.capability.prepare_reload(&candidate.capability)?;
        candidate.capability = self.capability.replacement();
        *self = candidate;
        Ok(())
    }
    pub fn is_failed(&self) -> bool {
        self.capability.is_failed()
    }
    pub fn enable_restart_requests(&mut self) -> Result<(), StaticError> {
        Ok(self.capability.enable_restart_requests()?)
    }
    pub fn set_haptic_capabilities(
        &mut self,
        value: incant_input::haptics::HapticCapabilities,
    ) -> Result<(), StaticError> {
        Ok(self.capability.set_haptic_capabilities(value)?)
    }
    pub fn haptics_ready(&self) -> Result<(), StaticError> {
        Ok(self.capability.haptics_ready()?)
    }
    pub fn take_haptics(&mut self) -> Result<incant_input::haptics::HapticRequests, StaticError> {
        Ok(self.capability.take_haptics()?)
    }
    pub fn clear_haptics(&mut self) {
        self.capability.clear_haptics();
    }
    /// Script acceptance only; defer consumption until composed acceptance.
    pub fn take_restart_request(&mut self) -> Result<bool, StaticError> {
        Ok(self.capability.take_restart_request()?)
    }
    /// Bounded committed prefix; a failed instance exposes no live diagnostics.
    pub fn logs(&self) -> Result<&crate::NativeLogs, StaticError> {
        Ok(self.capability.logs()?)
    }
    pub fn tick(&mut self, world: &mut NativeWorld, dt: f64) -> Result<(), StaticError> {
        let epoch = self.capability.begin(world, dt)?;
        self.run(world, dt, epoch)
    }
    /// Run against an engine-validated snapshot through `incant_input_v1`.
    /// The owner must advance this input and native simulation on the same fixed
    /// timeline. This method does not advance either clock. `tick` intentionally
    /// supplies no input authority; imports there fail explicitly.
    pub fn tick_with_input(
        &mut self,
        world: &mut NativeWorld,
        dt: f64,
        input: &incant_input::InputRuntime,
    ) -> Result<(), StaticError> {
        let epoch = self.capability.begin_with_input(world, dt, input)?;
        self.run(world, dt, epoch)
    }
    fn run(&mut self, world: &mut NativeWorld, dt: f64, epoch: i64) -> Result<(), StaticError> {
        let result = world
            .with_numeric_physics(|_, queries| -> Result<(), StaticError> {
                self.capability.bind_physics(epoch, queries)?;
                let mut context = Context {
                    capability: &mut self.capability,
                    error: None,
                };
                // SAFETY: same live instance and call-local exclusive context; C catches
                // all traps and only invokes Rust callbacks synchronously.
                let status = unsafe {
                    (self.exports.update)(
                        self.instance.as_ptr(),
                        dispatch,
                        (&mut context as *mut Context<'_>).cast(),
                        epoch,
                        dt,
                    )
                };
                let error = context.error.take();
                self.capability.revoke_physics();
                if status != 0 || error.is_some() {
                    let error = error.unwrap_or(if status == -3 {
                        StaticError::Fuel
                    } else {
                        StaticError::Trap(status)
                    });
                    self.capability.abort();
                    return Err(error);
                }
                Ok(())
            })
            .map_err(|e| StaticError::Boundary(e.into()))
            .and_then(|r| r);
        if let Err(error) = result {
            self.capability.abort();
            return Err(error);
        }
        Ok(self.capability.finish(world)?)
    }
}
impl Drop for StaticModule {
    fn drop(&mut self) {
        // SAFETY: unique live instance; no guest frame/callback remains active.
        unsafe { (self.exports.free)(self.instance.as_ptr()) };
    }
}
struct Context<'a> {
    capability: &'a mut Capability,
    error: Option<StaticError>,
}
unsafe extern "C" fn dispatch(
    raw: *mut c_void,
    memory: *mut u8,
    len: usize,
    op: u32,
    epoch: i64,
    a: i32,
    b: i32,
    c: i32,
    d: i32,
    out: *mut i32,
) -> i32 {
    // SAFETY: trusted facade passes this call's exclusive Context. No reference
    // survives the callback and generated guest code is suspended until return.
    let context = unsafe { &mut *raw.cast::<Context<'_>>() };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if len > limits::MEMORY || (len != 0 && memory.is_null()) || out.is_null() {
            return Err(StaticError::Facade);
        }
        let call = crate::generated::decode(op, [a, b, c, d]).ok_or(StaticError::Facade)?;
        // SAFETY: validated trusted descriptor contract guarantees initialized,
        // exclusive, bounded guest bytes. Empty Wasm memory may have null data.
        let bytes = if len == 0 {
            &mut []
        } else {
            unsafe { std::slice::from_raw_parts_mut(memory, len) }
        };
        let value = context.capability.invoke(bytes, epoch, call)?;
        unsafe { *out = value };
        Ok(())
    }));
    match result {
        Ok(Ok(())) => 0,
        Ok(Err(error)) => {
            context.error = Some(error);
            1
        }
        Err(_) => {
            context.error = Some(StaticError::CallbackPanic);
            1
        }
    }
}
