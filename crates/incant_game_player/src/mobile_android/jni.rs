//! JNI runs on the SurfaceView thread. Handles are generation IDs, never addresses.
use super::{
    host::{AndroidMobileHost, NativeHostError},
    lifecycle::{Gate, LifecycleError, Slot, TouchSlots},
};
use incant_input::TouchPhase;
use jni::{
    Env, EnvUnowned, JavaVM,
    errors::ThrowRuntimeExAndDefault,
    jni_sig, jni_str,
    objects::{JClass, JObject, JString},
    refs::Global,
};
use ndk::native_window::NativeWindow;
use std::{
    cell::RefCell,
    panic::{AssertUnwindSafe, catch_unwind},
    path::Path,
    sync::Mutex,
};

type Result<T> = std::result::Result<T, BridgeError>;
#[derive(Debug, thiserror::Error)]
enum BridgeError {
    #[error(transparent)]
    Jni(#[from] jni::errors::Error),
    #[error(transparent)]
    Lifecycle(#[from] LifecycleError),
    #[error(transparent)]
    Host(#[from] NativeHostError),
    #[error("Android application context, surface or package path is invalid")]
    Input,
    #[error("Android native callback panicked; owner retired")]
    Panic,
}
struct Entry {
    host: AndroidMobileHost,
    touches: TouchSlots,
}
static GATE: Gate = Gate::new();
// Application, not Activity: CPAL callbacks can outlive a surface. Never reset the
// ndk-context singleton or delete this global reference during Activity teardown.
static CONTEXT: Mutex<Option<(JavaVM, Global<JObject<'static>>)>> = Mutex::new(None);
thread_local! { static OWNER: RefCell<Slot<Entry>> = RefCell::new(Slot::default()); }
fn retire() {
    OWNER.with(|cell| {
        if let Ok(mut slot) = cell.try_borrow_mut()
            && slot.retire()
        {
            GATE.release();
        }
    });
}
fn boundary<T>(f: impl FnOnce() -> Result<T>) -> Result<T> {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(value) => value,
        Err(_) => {
            retire();
            Err(BridgeError::Panic)
        }
    }
}
fn with<T>(id: i64, f: impl FnOnce(&mut Entry) -> Result<T>) -> Result<T> {
    OWNER.with(|cell| {
        let mut slot = cell.try_borrow_mut().map_err(|_| LifecycleError::Busy)?;
        f(slot.get(id)?)
    })
}
fn context(env: &mut Env<'_>, supplied: &JObject<'_>) -> Result<()> {
    let app = env
        .call_method(
            supplied,
            jni_str!("getApplicationContext"),
            jni_sig!("()Landroid/content/Context;"),
            &[],
        )?
        .l()?;
    if app.is_null() || !env.is_instance_of(&app, jni_str!("android/app/Application"))? {
        return Err(BridgeError::Input);
    }
    let vm = env.get_java_vm()?;
    let mut stored = CONTEXT.lock().map_err(|_| BridgeError::Input)?;
    if let Some((old_vm, old_app)) = stored.as_ref() {
        if old_vm.get_raw() != vm.get_raw() || !env.is_same_object(old_app, &app)? {
            return Err(BridgeError::Input);
        }
        return Ok(());
    }
    let global = env.new_global_ref(app)?;
    // SAFETY: this standalone host is the sole ndk-context initializer. The
    // mutex serializes initialization before CPAL startup; both pointers remain
    // valid for process life. No ndk-glue/android-activity host is linked.
    unsafe {
        ndk_context::initialize_android_context(vm.get_raw().cast(), global.as_raw().cast());
    }
    *stored = Some((vm, global));
    Ok(())
}
fn window(env: &mut Env<'_>, object: &JObject<'_>) -> Result<NativeWindow> {
    if object.is_null() || !env.is_instance_of(object, jni_str!("android/view/Surface"))? {
        return Err(BridgeError::Input);
    }
    // SAFETY: JNI has checked Surface type. ndk owns an acquired native-window
    // reference. jni-sys 0.3/0.4 pointers share the Android JNI C ABI.
    unsafe { NativeWindow::from_surface(env.get_raw().cast(), object.as_raw().cast()) }
        .ok_or(BridgeError::Input)
}
fn size(w: i32, h: i32) -> Result<[u32; 2]> {
    Ok([
        u32::try_from(w).map_err(|_| BridgeError::Input)?,
        u32::try_from(h).map_err(|_| BridgeError::Input)?,
    ])
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_incant_player_NativeBridge_open<'a>(
    mut env: EnvUnowned<'a>,
    _: JClass<'a>,
    app: JObject<'a>,
    surface: JObject<'a>,
    package: JString<'a>,
    w: i32,
    h: i32,
    lw: f32,
    lh: f32,
) -> i64 {
    env.with_env(|env| {
        boundary(|| {
            let len = env
                .call_method(&package, jni_str!("length"), jni_sig!("()I"), &[])?
                .i()?;
            if !(1..=4096).contains(&len) {
                return Err(BridgeError::Input);
            }
            let package = package.try_to_string(env)?;
            if package.contains('\0') || !Path::new(&package).is_absolute() {
                return Err(BridgeError::Input);
            }
            context(env, &app)?;
            let claim = GATE.reserve()?;
            let id = claim.id();
            let result = (|| {
                let host = AndroidMobileHost::open(
                    window(env, &surface)?,
                    Path::new(&package),
                    size(w, h)?,
                    [lw, lh],
                )?;
                OWNER.with(|cell| {
                    cell.try_borrow_mut()
                        .map_err(|_| LifecycleError::Busy)?
                        .install(
                            id,
                            Entry {
                                host,
                                touches: TouchSlots::default(),
                            },
                        )
                })?;
                Ok(id)
            })();
            if result.is_ok() {
                claim.publish();
            }
            result
        })
    })
    .resolve::<ThrowRuntimeExAndDefault>()
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_incant_player_NativeBridge_close<'a>(
    mut env: EnvUnowned<'a>,
    _: JClass<'a>,
    id: i64,
) {
    env.with_env(|_| {
        boundary(|| {
            OWNER.with(|cell| {
                if cell
                    .try_borrow_mut()
                    .map_err(|_| LifecycleError::Busy)?
                    .close(id)?
                {
                    GATE.release();
                }
                Ok(())
            })
        })
    })
    .resolve::<ThrowRuntimeExAndDefault>()
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_incant_player_NativeBridge_active<'a>(
    mut env: EnvUnowned<'a>,
    _: JClass<'a>,
    id: i64,
    active: bool,
) {
    env.with_env(|_| {
        boundary(|| {
            with(id, |entry| {
                let old = entry.host.delivery_epoch();
                entry.host.set_active(active)?;
                if !active || old != entry.host.delivery_epoch() {
                    entry.touches.reset();
                }
                Ok(())
            })
        })
    })
    .resolve::<ThrowRuntimeExAndDefault>()
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_incant_player_NativeBridge_surface<'a>(
    mut env: EnvUnowned<'a>,
    _: JClass<'a>,
    id: i64,
    surface: JObject<'a>,
    w: i32,
    h: i32,
    lw: f32,
    lh: f32,
) {
    env.with_env(|env| {
        boundary(|| {
            with(id, |entry| {
                if surface.is_null() {
                    entry.host.detach()?;
                } else {
                    entry
                        .host
                        .attach(window(env, &surface)?, size(w, h)?, [lw, lh])?;
                }
                entry.touches.reset();
                Ok(())
            })
        })
    })
    .resolve::<ThrowRuntimeExAndDefault>()
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_incant_player_NativeBridge_frame<'a>(
    mut env: EnvUnowned<'a>,
    _: JClass<'a>,
    id: i64,
) -> i64 {
    env.with_env(|_| {
        boundary(|| {
            with(id, |entry| {
                let old = entry.host.delivery_epoch();
                entry.host.frame()?;
                let current = entry.host.delivery_epoch();
                if old != current {
                    entry.touches.reset();
                }
                i64::try_from(current).map_err(|_| LifecycleError::Exhausted.into())
            })
        })
    })
    .resolve::<ThrowRuntimeExAndDefault>()
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_incant_player_NativeBridge_touch<'a>(
    mut env: EnvUnowned<'a>,
    _: JClass<'a>,
    id: i64,
    epoch: i64,
    pointer: i32,
    phase: i32,
    x: f32,
    y: f32,
) {
    env.with_env(|_| {
        boundary(|| {
            with(id, |entry| {
                let epoch = u64::try_from(epoch).map_err(|_| LifecycleError::Stale)?;
                if epoch == 0 || epoch != entry.host.delivery_epoch() {
                    return Err(LifecycleError::Stale.into());
                }
                let phase = match phase {
                    0 => TouchPhase::Down,
                    1 => TouchPhase::Move,
                    2 => TouchPhase::Up,
                    3 => TouchPhase::Cancel,
                    _ => return Err(BridgeError::Input),
                };
                let slot = if phase == TouchPhase::Down {
                    entry.touches.start(pointer, epoch)?
                } else {
                    entry.touches.get(pointer, epoch)?
                };
                let result = entry.host.touch(epoch, slot, phase, [x, y]);
                if matches!(phase, TouchPhase::Up | TouchPhase::Cancel)
                    || (result.is_err() && phase == TouchPhase::Down)
                {
                    entry.touches.end(slot);
                }
                result.map_err(Into::into)
            })
        })
    })
    .resolve::<ThrowRuntimeExAndDefault>()
}
