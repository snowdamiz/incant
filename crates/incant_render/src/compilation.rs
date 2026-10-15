//! Explicit cold Metal observation. Contains no renderer/native object ownership.
pub use wgpu_hal::metal::compilation::{
    ClockAnomalies, Context, Counts, ErrorDomain, KIND_COUNT, Kind, MAX_SAMPLES, Observer, Outcome,
    Phase, Row, Scope, Snapshot, Stage,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("native compilation capture capacity must be1..4096")]
    Capacity,
    #[error("native compilation capture requires the Metal backend")]
    Backend,
    #[error("native compilation capture is already attached")]
    AlreadyAttached,
    #[error("native compilation observer refused: {0:?}")]
    Observer(wgpu_hal::metal::compilation::Error),
}
/// Created by a trusted host before renderer construction. Retains a diagnostic
/// handle even when catalog construction returns an error; never retains Device.
pub struct Capture {
    capacity: usize,
    observer: Option<Observer>,
}
impl Capture {
    pub fn new(capacity: usize) -> Result<Self, Error> {
        if !(1..=MAX_SAMPLES).contains(&capacity) {
            return Err(Error::Capacity);
        }
        Ok(Self {
            capacity,
            observer: None,
        })
    }
    pub fn observer(&self) -> Option<Observer> {
        self.observer.clone()
    }
    pub(super) fn attach(&mut self, device: &crate::wgpu::Device) -> Result<Scope, Error> {
        if self.observer.is_some() {
            return Err(Error::AlreadyAttached);
        }
        // SAFETY: scoped borrow of the already-requested Metal device only.
        // No raw device mutation/ownership or second device is introduced.
        let device =
            unsafe { device.as_hal::<crate::wgpu::hal::api::Metal>() }.ok_or(Error::Backend)?;
        self.attach_observer(device.compilation_observer())
    }
    fn attach_observer(&mut self, observer: Observer) -> Result<Scope, Error> {
        if self.observer.is_some() {
            return Err(Error::AlreadyAttached);
        }
        self.observer = Some(observer.clone());
        observer.start(self.capacity).map_err(Error::Observer)?;
        observer
            .scope(Context {
                session_generation: 1,
                phase: Phase::Catalog,
                ..Context::default()
            })
            .map_err(Error::Observer)
    }
}
impl crate::Renderer {
    /// Fixed ordinary1x catalog, exact existing constructor semantics plus a
    /// default-off observation bound before catalog creation. No GPU timestamps.
    pub async fn new_with_compilation_capture(
        instance: &crate::wgpu::Instance,
        surface: Option<&crate::wgpu::Surface<'_>>,
        capture: &mut Capture,
    ) -> crate::Result<Self> {
        Self::new_configured(
            instance,
            surface,
            false,
            crate::SceneMsaa::One,
            None,
            Some(capture),
            #[cfg(all(
                feature = "metal-binary-archives",
                feature = "native-compilation",
                target_os = "macos"
            ))]
            wgpu_hal::metal::binary_archive::Mode::Capture,
        )
        .await
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn capacity_refuses_before_attaching_and_partial_handle_survives() {
        assert!(matches!(Capture::new(0), Err(Error::Capacity)));
        assert!(matches!(
            Capture::new(MAX_SAMPLES + 1),
            Err(Error::Capacity)
        ));
        let mut capture = Capture::new(1).unwrap();
        assert!(capture.observer().is_none());
        let observer = Observer::default();
        observer.start(1).unwrap();
        assert!(matches!(
            capture.attach_observer(observer),
            Err(Error::Observer(_))
        ));
        assert!(capture.observer().unwrap().snapshot().capture_started);
        assert!(matches!(
            capture.attach_observer(Observer::default()),
            Err(Error::AlreadyAttached)
        ));
    }
    #[test]
    fn catalog_scope_ends_without_discarding_diagnostic_handle() {
        let mut capture = Capture::new(1).unwrap();
        let scope = capture.attach_observer(Observer::default()).unwrap();
        assert!(capture.observer().unwrap().snapshot().scope_active);
        drop(scope);
        assert!(!capture.observer().unwrap().snapshot().scope_active);
        let handle = capture.observer().unwrap();
        drop(capture);
        assert!(handle.snapshot().capture_started);
    }
    #[test]
    fn catalog_unwind_clears_scope_and_keeps_the_failure_observer() {
        let mut capture = Capture::new(1).unwrap();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _scope = capture.attach_observer(Observer::default()).unwrap();
            panic!("catalog startup failure");
        }));
        assert!(result.is_err());
        let snapshot = capture.observer().unwrap().snapshot();
        assert!(snapshot.capture_started);
        assert!(!snapshot.scope_active);
    }
}
