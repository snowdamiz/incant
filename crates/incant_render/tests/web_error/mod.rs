use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Debug, thiserror::Error)]
#[error("backend validation detail")]
struct BackendSource {
    drops: Arc<AtomicUsize>,
    #[source]
    nested: std::io::Error,
}
impl Drop for BackendSource {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}
fn source(drops: &Arc<AtomicUsize>) -> wgpu::ErrorSource {
    Box::new(BackendSource {
        drops: Arc::clone(drops),
        nested: std::io::Error::other("nested driver detail"),
    })
}
#[test]
fn copied_error_preserves_category_display_and_source_chain_without_backend_owner() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<PreparationError>();
    for kind in [
        PreparationErrorKind::OutOfMemory,
        PreparationErrorKind::Validation,
        PreparationErrorKind::Internal,
    ] {
        let drops = Arc::new(AtomicUsize::new(0));
        let (error, message) = match kind {
            PreparationErrorKind::OutOfMemory => (
                wgpu::Error::OutOfMemory {
                    source: source(&drops),
                },
                "Out of Memory",
            ),
            PreparationErrorKind::Validation => (
                wgpu::Error::Validation {
                    source: source(&drops),
                    description: "invalid pipeline".into(),
                },
                "invalid pipeline",
            ),
            PreparationErrorKind::Internal => (
                wgpu::Error::Internal {
                    source: source(&drops),
                    description: "internal device failure".into(),
                },
                "internal device failure",
            ),
        };
        let error = PreparationError::from(error);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        assert_eq!(error.kind(), kind);
        assert_eq!(error.to_string(), message);
        let source = error.source().unwrap();
        assert_eq!(source.to_string(), "backend validation detail");
        let nested = source.source().unwrap();
        assert_eq!(nested.to_string(), "nested driver detail");
        assert!(nested.source().is_none());
    }
}
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn native_preparation_error_keeps_original_typed_source() {
    let drops = Arc::new(AtomicUsize::new(0));
    let error = crate::ResourceError::Preparation(wgpu::Error::Validation {
        source: source(&drops),
        description: "invalid pipeline".into(),
    });
    assert_eq!(
        error.to_string(),
        "renderer initialization failed: invalid pipeline"
    );
    let gpu = error
        .source()
        .unwrap()
        .downcast_ref::<wgpu::Error>()
        .unwrap();
    assert!(matches!(gpu, wgpu::Error::Validation { .. }));
    assert!(
        gpu.source()
            .unwrap()
            .downcast_ref::<BackendSource>()
            .is_some()
    );
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    drop(error);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}
