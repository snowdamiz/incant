//! Same-thread, non-reentrant UIKit bridge. Handles are unique integers, never
//! trusted pointers. Rust panics retire the owner in unwind-enabled builds;
//! shipping's abort-on-panic policy is unchanged.
use super::{AppleMobileHost, NativeHostError};
use incant_input::TouchPhase;
use std::{
    cell::RefCell,
    ffi::c_void,
    panic::{AssertUnwindSafe, catch_unwind},
    path::Path,
    ptr::NonNull,
    sync::atomic::{AtomicU64, Ordering},
};

const OK: i32 = 0;
const INVALID: i32 = 1;
const BUSY: i32 = 2;
const STALE: i32 = 3;
const FAILED: i32 = 4;
const WRONG_THREAD: i32 = 5;
const EXHAUSTED: i32 = 6;
const MAX_ERROR: usize = 4096;
static NEXT: AtomicU64 = AtomicU64::new(1);

struct Slot<T> {
    owner: Option<(u64, T)>,
    retired: u64,
}
impl<T> Default for Slot<T> {
    fn default() -> Self {
        Self {
            owner: None,
            retired: 0,
        }
    }
}
impl<T> Slot<T> {
    fn get(&mut self, handle: u64) -> Result<&mut T, i32> {
        self.owner
            .as_mut()
            .filter(|(id, _)| handle != 0 && *id == handle)
            .map(|(_, value)| value)
            .ok_or(STALE)
    }
    fn close(&mut self, handle: u64) -> Result<(), i32> {
        if handle != 0 && self.retired == handle {
            return Ok(());
        }
        self.get(handle)?;
        self.retire();
        Ok(())
    }
    fn retire(&mut self) {
        if let Some((id, _value)) = self.owner.take() {
            self.retired = id;
        }
    }
}
fn allocate(next: &AtomicU64) -> Result<u64, i32> {
    next.try_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
        .map_err(|_| EXHAUSTED)
}
#[derive(Default)]
struct State {
    slot: Slot<AppleMobileHost>,
    error: String,
}
thread_local! { static STATE: RefCell<State> = RefCell::new(State::default()); }

fn call(operation: impl FnOnce(&mut State) -> i32) -> i32 {
    STATE.with(|cell| {
        let Ok(mut state) = cell.try_borrow_mut() else {
            return BUSY;
        };
        match catch_unwind(AssertUnwindSafe(|| operation(&mut state))) {
            Ok(status) => status,
            Err(_) => {
                state.slot.retire();
                state.error = "native host panicked and was retired".into();
                FAILED
            }
        }
    })
}
fn status(state: &mut State, result: Result<(), NativeHostError>) -> i32 {
    match result {
        Ok(()) => {
            state.error.clear();
            OK
        }
        Err(error) => {
            let status = match &error {
                NativeHostError::Stale | NativeHostError::Closed => STALE,
                NativeHostError::Input | NativeHostError::Touch(_) => INVALID,
                NativeHostError::Epoch => EXHAUSTED,
                _ => FAILED,
            };
            let mut message = error.to_string();
            let mut end = message.len().min(MAX_ERROR);
            while !message.is_char_boundary(end) {
                end -= 1;
            }
            message.truncate(end);
            state.error = message;
            status
        }
    }
}
fn with_host(
    handle: u64,
    operation: impl FnOnce(&mut AppleMobileHost) -> Result<(), NativeHostError>,
) -> i32 {
    call(|state| {
        let result = match state.slot.get(handle) {
            Ok(host) => operation(host),
            Err(status) => return status,
        };
        status(state, result)
    })
}

unsafe extern "C" {
    fn pthread_main_np() -> i32;
}

/// Creates one native owner on the main thread, initially suspended. The package
/// path is absolute bounded UTF-8; no NUL terminator is needed.
///
/// # Safety
/// `layer` must be a real CAMetalLayer, retained through successful close. The
/// platform must serialize layer mutations and every call on the main thread.
/// `package` points to `length` stable readable bytes (1..=4096), and `output`
/// points to a writable, aligned u64. The two allocations must not overlap.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn incant_apple_open_v1(
    layer: *mut c_void,
    package: *const u8,
    length: usize,
    width: u32,
    height: u32,
    logical_width: f32,
    logical_height: f32,
    output: *mut u64,
) -> i32 {
    if output.is_null() {
        return INVALID;
    }
    // SAFETY: the bounded writable output is caller-owned, including on failure.
    unsafe {
        output.write(0);
    }
    if layer.is_null() || package.is_null() || !(1..=4096).contains(&length) {
        return INVALID;
    }
    // SAFETY: pthread_main_np has no preconditions on Apple platforms.
    if unsafe { pthread_main_np() } == 0 {
        return WRONG_THREAD;
    }
    // SAFETY: length and allocation contract are checked/required above.
    let bytes = unsafe { std::slice::from_raw_parts(package, length) };
    let Ok(path) = std::str::from_utf8(bytes) else {
        return INVALID;
    };
    if !Path::new(path).is_absolute() || path.chars().any(char::is_control) {
        return INVALID;
    }
    call(|state| {
        if state.slot.owner.is_some() {
            return BUSY;
        }
        let handle = match allocate(&NEXT) {
            Ok(id) => id,
            Err(status) => return status,
        };
        // SAFETY: the caller retains the valid layer and serializes its access.
        match unsafe {
            AppleMobileHost::open(
                NonNull::new(layer).expect("checked above"),
                Path::new(path),
                [width, height],
                [logical_width, logical_height],
            )
        } {
            Ok(host) => {
                state.slot.owner = Some((handle, host));
                state.error.clear();
                // SAFETY: output remains exclusively writable through return.
                unsafe {
                    output.write(handle);
                }
                OK
            }
            Err(error) => status(state, Err(error)),
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn incant_apple_close_v1(handle: u64) -> i32 {
    call(|state| {
        state
            .slot
            .close(handle)
            .map_or_else(|status| status, |()| OK)
    })
}
#[unsafe(no_mangle)]
pub extern "C" fn incant_apple_active_v1(handle: u64, active: i32) -> i32 {
    if !(0..=1).contains(&active) {
        return INVALID;
    }
    with_host(handle, |host| host.set_active(active != 0))
}
#[unsafe(no_mangle)]
pub extern "C" fn incant_apple_resize_v1(
    handle: u64,
    width: u32,
    height: u32,
    logical_width: f32,
    logical_height: f32,
) -> i32 {
    with_host(handle, |host| {
        host.resize([width, height], [logical_width, logical_height])
    })
}
#[unsafe(no_mangle)]
pub extern "C" fn incant_apple_frame_v1(handle: u64) -> i32 {
    with_host(handle, |host| host.frame().map(|_| ()))
}
/// Zero means no input delivery is currently accepted. Store nonzero epochs
/// with the touch at Down; never refresh the epoch of a continuing gesture.
#[unsafe(no_mangle)]
pub extern "C" fn incant_apple_epoch_v1(handle: u64) -> u64 {
    STATE.with(|cell| {
        cell.try_borrow_mut()
            .ok()
            .and_then(|mut state| {
                state
                    .slot
                    .get(handle)
                    .ok()
                    .map(|host| host.delivery_epoch())
            })
            .unwrap_or(0)
    })
}
#[unsafe(no_mangle)]
pub extern "C" fn incant_apple_touch_v1(
    handle: u64,
    epoch: u64,
    id: u32,
    phase: u32,
    x: f32,
    y: f32,
) -> i32 {
    let phase = match phase {
        0 => TouchPhase::Down,
        1 => TouchPhase::Move,
        2 => TouchPhase::Up,
        3 => TouchPhase::Cancel,
        _ => return INVALID,
    };
    with_host(handle, |host| host.touch(epoch, id, phase, [x, y]))
}
/// Copies up to 4096 bytes of the last native error on this thread, without NUL
/// termination. Returns bytes copied. Handle/argument status codes are sufficient
/// for admission failures; those do not replace the last native error.
///
/// # Safety
/// For nonzero capacity (at most 4096), `output` points to that many writable
/// bytes, exclusively accessible during this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn incant_apple_error_v1(output: *mut u8, capacity: usize) -> usize {
    if output.is_null() || capacity == 0 || capacity > MAX_ERROR {
        return 0;
    }
    STATE.with(|cell| {
        let Ok(state) = cell.try_borrow() else {
            return 0;
        };
        let count = state.error.len().min(capacity);
        // SAFETY: output is a bounded caller allocation, separate from TLS text.
        unsafe {
            std::ptr::copy_nonoverlapping(state.error.as_ptr(), output, count);
        }
        count
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ids_never_reuse_and_exhaustion_does_not_wrap() {
        let next = AtomicU64::new(1);
        assert_eq!(allocate(&next), Ok(1));
        assert_eq!(allocate(&next), Ok(2));
        next.store(u64::MAX, Ordering::Relaxed);
        assert_eq!(allocate(&next), Err(EXHAUSTED));
        assert_eq!(next.load(Ordering::Relaxed), u64::MAX);
    }
    #[test]
    fn stale_handles_cannot_mutate_or_close_replacement_and_close_is_idempotent() {
        let mut slot = Slot {
            owner: Some((10, 1)),
            retired: 0,
        };
        assert_eq!(slot.get(0), Err(STALE));
        assert_eq!(slot.close(9), Err(STALE));
        *slot.get(10).unwrap() = 2;
        slot.close(10).unwrap();
        slot.close(10).unwrap();
        slot.owner = Some((11, 3));
        assert_eq!(slot.get(10), Err(STALE));
        slot.close(10).unwrap();
        assert_eq!(slot.get(11), Ok(&mut 3));
    }
    #[test]
    fn failed_open_clears_valid_output_before_rejecting_arguments() {
        let mut handle = 42;
        // SAFETY: output is writable; null resource pointers are rejected before use.
        let status = unsafe {
            incant_apple_open_v1(
                std::ptr::null_mut(),
                std::ptr::null(),
                0,
                0,
                0,
                0.,
                0.,
                &mut handle,
            )
        };
        assert_eq!(status, INVALID);
        assert_eq!(handle, 0);
    }
    #[test]
    fn ffi_admission_and_reentrancy_do_not_acquire_native_resources() {
        assert_eq!(incant_apple_frame_v1(99), STALE);
        assert_eq!(incant_apple_active_v1(99, 2), INVALID);
        assert_eq!(incant_apple_touch_v1(99, 1, 0, 99, 0., 0.), INVALID);
        assert_eq!(incant_apple_epoch_v1(99), 0);
        assert_eq!(call(|_| incant_apple_frame_v1(99)), BUSY);
        assert_eq!(call(|_| panic!("contained native panic")), FAILED);
        assert_eq!(incant_apple_frame_v1(99), STALE);
    }
}
