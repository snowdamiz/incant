//! The COM library, rather than our interface wrappers, owns its immutable input.
use alloc::{
    boxed::Box,
    sync::{Arc, Weak},
};
use core::ffi::c_void;
use windows_core::IUnknown;

#[windows_core::implement()]
struct BlobOwner {
    // Drop the allocation before releasing the only strong lifetime marker.
    _bytes: Box<[u8]>,
    _alive: Arc<()>,
}

/// A read-only witness; it cannot upgrade or extend the backing lifetime.
#[derive(Clone)]
pub struct Lifetime(Weak<()>);
impl Lifetime {
    pub fn is_alive(&self) -> bool {
        self.0.strong_count() != 0
    }
}

pub(super) struct Prepared {
    owner: IUnknown,
    pointer: *const c_void,
    length: usize,
    lifetime: Lifetime,
}
struct Attachment<T> {
    // On HRESULT error or unwind, release the unreturned native library before
    // its last local blob owner. Success takes the library out after native
    // SetPrivateDataInterface has added its own reference to the owner.
    library: Option<T>,
    owner: IUnknown,
}
impl Prepared {
    pub fn new(bytes: Box<[u8]>) -> Self {
        let alive = Arc::new(());
        let lifetime = Lifetime(Arc::downgrade(&alive));
        let length = bytes.len();
        let pointer = if bytes.is_empty() {
            core::ptr::null()
        } else {
            bytes.as_ptr().cast()
        };
        // Moving Box into a COM object preserves the allocation and alignment.
        let owner = BlobOwner {
            _bytes: bytes,
            _alive: alive,
        }
        .into();
        Self {
            owner,
            pointer,
            length,
            lifetime,
        }
    }
    pub fn pointer(&self) -> *const c_void {
        self.pointer
    }
    pub fn length(&self) -> usize {
        self.length
    }
    pub fn lifetime(&self) -> Lifetime {
        self.lifetime.clone()
    }
    // Private trusted plumbing only, not a mutable host/script escape. The only
    // production binder is ID3D12Object::SetPrivateDataInterface.
    pub fn attach<T>(
        self,
        library: T,
        bind: impl FnOnce(&T, &IUnknown) -> windows_core::Result<()>,
    ) -> windows_core::Result<T> {
        let mut attachment = Attachment {
            library: Some(library),
            owner: self.owner,
        };
        bind(attachment.library.as_ref().unwrap(), &attachment.owner)?;
        Ok(attachment.library.take().unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::{AtomicBool, Ordering};

    #[test]
    fn prepared_pointer_remains_stable_and_failed_creation_releases_backing() {
        let bytes = alloc::vec![1_u8, 2, 3].into_boxed_slice();
        let pointer = bytes.as_ptr();
        let prepared = Prepared::new(bytes);
        let life = prepared.lifetime();
        assert_eq!(prepared.pointer(), pointer.cast());
        assert_eq!(prepared.length(), 3);
        assert_eq!(
            unsafe { core::slice::from_raw_parts(prepared.pointer().cast::<u8>(), 3) },
            &[1, 2, 3]
        );
        assert!(life.is_alive());
        drop(prepared); // CreatePipelineLibrary failure before an interface exists.
        assert!(!life.is_alive());
        assert!(Prepared::new(Box::default()).pointer().is_null());
    }
    struct Unreturned {
        life: Lifetime,
        dropped: Arc<AtomicBool>,
    }
    impl Drop for Unreturned {
        fn drop(&mut self) {
            assert!(
                self.life.is_alive(),
                "native object must retire before its backing"
            );
            self.dropped.store(true, Ordering::SeqCst);
        }
    }
    #[test]
    fn failed_attach_and_unwind_release_unreturned_object_before_backing() {
        for unwind in [false, true] {
            let prepared = Prepared::new(alloc::vec![1_u8; 32].into_boxed_slice());
            let life = prepared.lifetime();
            let dropped = Arc::new(AtomicBool::new(false));
            let library = Unreturned {
                life: life.clone(),
                dropped: Arc::clone(&dropped),
            };
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                prepared.attach(library, |_, _| {
                    if unwind {
                        panic!("synthetic native attachment panic");
                    }
                    Err(windows_core::Error::from_hresult(windows_core::HRESULT(
                        -2147467259,
                    )))
                })
            }));
            if unwind {
                assert!(result.is_err());
            } else {
                assert!(result.unwrap().is_err());
            }
            assert!(dropped.load(Ordering::SeqCst));
            assert!(!life.is_alive());
        }
    }
    #[test]
    fn successful_com_attachment_keeps_backing_until_last_retained_interface() {
        let prepared = Prepared::new(alloc::vec![2_u8; 32].into_boxed_slice());
        let life = prepared.lifetime();
        let library = prepared
            .attach(core::cell::RefCell::new(None), |slot, owner| {
                *slot.borrow_mut() = Some(owner.clone());
                Ok(())
            })
            .unwrap();
        let retained = library.borrow().as_ref().unwrap().clone();
        let final_ref = retained.clone();
        drop(library);
        assert!(life.is_alive());
        drop(retained);
        assert!(life.is_alive());
        drop(final_ref);
        assert!(!life.is_alive());
    }
}
