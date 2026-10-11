//! Audited static C boundary. Traps must be caught before returning from C; C
//! import wrappers must wait for Rust callbacks to return before trapping.
use crate::{BoundaryError, Call, Capability, limits};
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
        }
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
        let mut capability = Capability::default();
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
    pub fn is_failed(&self) -> bool {
        self.capability.is_failed()
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
        if status != 0 || context.error.is_some() {
            let error = context.error.take().unwrap_or(if status == -3 {
                StaticError::Fuel
            } else {
                StaticError::Trap(status)
            });
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
        let call = match op {
            0 => Call::ChunkCount,
            1 => Call::ChunkLen(a),
            2 => Call::ChunkFlags(a),
            3 => Call::ReadIds(a, b),
            4..=7 => Call::Column {
                chunk: a,
                ptr: b,
                velocity: op >= 6,
                write: op % 2 == 1,
            },
            8 => Call::Spawn {
                id: a,
                parent: b,
                transform: c,
                velocity: d,
            },
            9 => Call::Despawn(a),
            10 => Call::SetVelocity(a, b),
            11 => Call::SetParent(a, b),
            12 => Call::InputLen,
            13 => Call::ReadInput {
                ptr: a,
                capacity: b,
            },
            14 => Call::FlipbookLen,
            15 => Call::ReadFlipbooks {
                ptr: a,
                capacity: b,
            },
            16 => Call::ControlFlipbook {
                id: a,
                kind: b,
                frame: c,
            },
            _ => return Err(StaticError::Facade),
        };
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
