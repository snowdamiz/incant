//! Cooperative callbacks with explicit, versioned frame bytes; no suspended stack.
pub use incant_wasm_abi::coroutines::{Routine, encode_catalog};
pub use incant_wasm_abi::{COROUTINE_FRAME_BYTES, COROUTINE_MAX_BYTES};
pub struct Task<'a> {
    pub id: &'a str,
    pub entity: Option<crate::EntityId>,
    /// Decode fields explicitly. On Wait these bytes become the next saved frame.
    pub state: &'a mut [u8],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Next {
    Done,
    Wait(u32),
}

/// Declare the complete immutable routine catalog and dispatch after update.
/// Each callback is `fn(&mut Tick, Task) -> Result<Next>`. A new task first runs
/// on the next accepted simulation tick. Wait takes a positive tick count.
#[macro_export]
macro_rules! coroutine_module {
    ($update:path; $( $name:literal => ($schema:expr, $bytes:expr, $callback:path) ),+ $(,)?) => {
        const _: () = {
            const ROUTINES: &[$crate::coroutines::Routine] = &[$(
                match $crate::coroutines::Routine::new($name,$bytes,$schema) {
                    Some(value) => value,
                    None => panic!("invalid coroutine routine"),
                }
            ),+];
            #[cfg(target_arch = "wasm32")]
            #[used]
            #[unsafe(link_section = "incant_coroutines_v1")]
            static INCANT_COROUTINE_CATALOG: [u8; 16 + ROUTINES.len() * $crate::coroutines::Routine::SIZE] =
                $crate::coroutines::encode_catalog(ROUTINES);
        };
        #[unsafe(no_mangle)]
        pub extern "C" fn incant_abi() -> i32 { $crate::__private::ABI_VERSION }
        #[unsafe(no_mangle)]
        pub extern "C" fn update(epoch: i64, dt: f64) {
            $crate::__private::run(epoch,dt,|tick,dt| {
                $update(tick,dt)?;
                tick.resume_coroutines(|tick,routine,task| match routine {
                    $( $name => $callback(tick,task), )+
                    _ => Err($crate::Error::Malformed),
                })
            });
        }
    };
}
