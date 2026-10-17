//! Single-threaded surface ownership. Reserve before simulation, consume inside
//! mandatory sync, present before returning to the event loop. No frame survives
//! a resize callback, and only this module can construct a reservation/provider.
use super::Error;
#[cfg(all(feature = "native-presentations", target_os = "macos"))]
use super::presentations::{Boundary, Capture, Failure, Role, Schedule, Status};
use incant_render::{Renderer, RetainedTarget, wgpu};
use incant_render_gpu_host::{RenderFrame, TargetProvider};
use std::{cell::RefCell, rc::Rc, sync::Arc};
use tao::window::Window;

pub struct SurfaceState {
    #[cfg(all(feature = "native-presentations", target_os = "macos"))]
    capture: Option<Capture>,
    instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    window: Arc<Window>,
    config: wgpu::SurfaceConfiguration,
    configured: bool,
    slot: Rc<RefCell<Option<Frame>>>,
    pub reconfigurations: u64,
    pub recreations: u64,
    pub skipped: u64,
}
pub struct SurfaceTargets {
    slot: Rc<RefCell<Option<Frame>>>,
}
pub struct Frame {
    texture: wgpu::SurfaceTexture,
    view: wgpu::TextureView,
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
}
impl SurfaceState {
    pub fn new(
        window: Arc<Window>,
        #[cfg(all(feature = "native-compilation", target_os = "macos"))] compilation: Option<
            &mut incant_render::compilation::Capture,
        >,
    ) -> Result<(Self, Arc<Renderer>, SurfaceTargets), Error> {
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(Arc::clone(&window))?;
        #[cfg(all(feature = "native-compilation", target_os = "macos"))]
        let renderer = match compilation {
            Some(capture) => pollster::block_on(Renderer::new_with_compilation_capture(
                &instance,
                Some(&surface),
                capture,
            )),
            None => pollster::block_on(Renderer::new(&instance, Some(&surface))),
        };
        #[cfg(not(all(feature = "native-compilation", target_os = "macos")))]
        let renderer = pollster::block_on(Renderer::new(&instance, Some(&surface)));
        let renderer = Arc::new(renderer.map_err(Error::Render)?);
        let capabilities = surface.get_capabilities(&renderer.adapter);
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .ok_or(Error::SurfaceCapabilities)?;
        let alpha_mode = *capabilities
            .alpha_modes
            .first()
            .ok_or(Error::SurfaceCapabilities)?;
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: 1,
            height: 1,
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode,
            view_formats: vec![],
        };
        let slot = Rc::new(RefCell::new(None));
        let targets = SurfaceTargets {
            slot: Rc::clone(&slot),
        };
        Ok((
            Self {
                #[cfg(all(feature = "native-presentations", target_os = "macos"))]
                capture: None,
                instance,
                surface,
                window,
                config,
                configured: false,
                slot,
                reconfigurations: 0,
                recreations: 0,
                skipped: 0,
            },
            renderer,
            targets,
        ))
    }
    #[cfg(all(feature = "native-presentations", target_os = "macos"))]
    pub fn install_capture(&mut self, capture: Capture) -> Result<(), Error> {
        capture.check_surface(&self.surface)?;
        self.capture = Some(capture);
        Ok(())
    }
    #[cfg(all(feature = "native-presentations", target_os = "macos"))]
    pub fn presentation_attempt(&mut self, role: Role, generation: u64, step: u64) {
        if let Some(capture) = &mut self.capture {
            capture.begin(role, generation, step);
        }
    }
    #[cfg(all(feature = "native-presentations", target_os = "macos"))]
    pub fn presentation_gameplay_attempt(
        &mut self,
        generation: u64,
        step: u64,
        schedule: Schedule,
    ) {
        if let Some(capture) = &mut self.capture {
            capture.gameplay_begin(generation, step, schedule);
        }
    }
    #[cfg(all(feature = "native-presentations", target_os = "macos"))]
    pub fn presentation_host_finished(&mut self) {
        if let Some(capture) = &mut self.capture {
            capture.host_finished();
        }
    }
    #[cfg(all(feature = "native-presentations", target_os = "macos"))]
    pub fn presentation_boundary(&mut self, reason: Boundary) {
        if let Some(capture) = &mut self.capture {
            capture.boundary(reason);
        }
    }
    #[cfg(all(feature = "native-presentations", target_os = "macos"))]
    pub fn presentation_complete(&mut self, status: Status) {
        if let Some(capture) = &mut self.capture {
            capture.complete(status);
        }
    }
    #[cfg(all(feature = "native-presentations", target_os = "macos"))]
    fn presentation_failure(&mut self, failure: Failure) {
        if let Some(capture) = &mut self.capture {
            capture.failure(failure);
        }
    }
    #[cfg(all(feature = "native-presentations", target_os = "macos"))]
    pub fn finish_capture(
        &mut self,
        steps: u64,
        generation: u64,
        code: i32,
        result: &crate::session::Result<serde_json::Value>,
    ) -> crate::session::Result<()> {
        let Some(mut capture) = self.capture.take() else {
            return Ok(());
        };
        let unbind = Capture::unbind(&self.surface);
        let written = capture.finish(
            steps,
            generation,
            code,
            result,
            unbind.as_ref().err().map(ToString::to_string),
        );
        // Sidecar is attempted even if unbinding failed; original window error
        // still has precedence in the caller. Callback captures remain safe.
        unbind?;
        written
    }
    /// The old composed host must already be destroyed. No provider or reserved
    /// frame may cross a session boundary; the OS surface alone is retained.
    pub fn replacement_targets(&mut self) -> Result<SurfaceTargets, Error> {
        if self.slot.borrow().is_some() || Rc::strong_count(&self.slot) != 1 {
            return Err(Error::Reservation);
        }
        Ok(SurfaceTargets {
            slot: Rc::clone(&self.slot),
        })
    }
    pub fn invalidate(&mut self) {
        self.slot.borrow_mut().take();
        self.configured = false;
    }
    pub fn cancel(&mut self) {
        self.slot.borrow_mut().take();
    }
    pub fn has_supported_size(&self) -> bool {
        let size = self.window.inner_size();
        supported_size(size.width, size.height)
    }
    #[cfg(all(feature = "native-presentations", target_os = "macos"))]
    pub fn presentation_new_events(&mut self, cause: tao::event::StartCause) {
        if let Some(capture) = &mut self.capture {
            capture.new_events(cause, std::time::Instant::now());
        }
    }
    #[cfg(all(feature = "native-presentations", target_os = "macos"))]
    pub fn presentation_pulse(&mut self, at: std::time::Instant) {
        if let Some(capture) = &mut self.capture {
            capture.pulse(at);
        }
    }
    pub fn reserve(&mut self, renderer: &Renderer) -> Result<bool, Error> {
        if self.slot.borrow().is_some() {
            #[cfg(all(feature = "native-presentations", target_os = "macos"))]
            self.presentation_failure(Failure::Reservation);
            return Err(Error::Reservation);
        }
        if let Err(error) = renderer.check_device() {
            #[cfg(all(feature = "native-presentations", target_os = "macos"))]
            self.presentation_failure(Failure::Device);
            return Err(Error::Render(error));
        }
        let size = self.window.inner_size();
        if !supported_size(size.width, size.height) || self.window.is_minimized() {
            #[cfg(all(feature = "native-presentations", target_os = "macos"))]
            {
                self.presentation_failure(if !supported_size(size.width, size.height) {
                    Failure::UnsupportedSize
                } else {
                    Failure::Minimized
                });
                self.presentation_boundary(Boundary::Inactive);
                self.presentation_complete(Status::Deferred);
            }
            self.skipped += 1;
            return Ok(false);
        }
        if (self.config.width, self.config.height) != (size.width, size.height) {
            self.config.width = size.width;
            self.config.height = size.height;
            self.configured = false;
        }
        let frame = bounded_acquire(|| {
            if !self.configured {
                #[cfg(all(feature = "native-presentations", target_os = "macos"))]
                self.presentation_boundary(Boundary::Configure);
                self.surface.configure(&renderer.device, &self.config);
                self.configured = true;
                self.reconfigurations += 1;
            }
            #[cfg(all(feature = "native-presentations", target_os = "macos"))]
            if let Some(capture) = &mut self.capture {
                capture.bind(&self.surface)?;
            }
            #[cfg(all(feature = "native-presentations", target_os = "macos"))]
            if let Some(capture) = &mut self.capture {
                capture.acquisition_begin();
            }
            let current = self.surface.get_current_texture();
            #[cfg(all(feature = "native-presentations", target_os = "macos"))]
            if let Some(capture) = &mut self.capture {
                use super::presentations::AcquisitionOutcome as A;
                capture.acquisition_end(match &current {
                    wgpu::CurrentSurfaceTexture::Success(_) => A::Success,
                    wgpu::CurrentSurfaceTexture::Suboptimal(_) => A::Suboptimal,
                    wgpu::CurrentSurfaceTexture::Timeout => A::Timeout,
                    wgpu::CurrentSurfaceTexture::Occluded => A::Occluded,
                    wgpu::CurrentSurfaceTexture::Outdated => A::Outdated,
                    wgpu::CurrentSurfaceTexture::Lost => A::Lost,
                    wgpu::CurrentSurfaceTexture::Validation => A::Validation,
                });
            }
            #[cfg(all(feature = "native-presentations", target_os = "macos"))]
            if self.capture.is_some() {
                Capture::unbind(&self.surface)?;
            }
            let texture = match current {
                wgpu::CurrentSurfaceTexture::Success(frame) => frame,
                wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                    self.configured = false;
                    frame
                }
                wgpu::CurrentSurfaceTexture::Timeout => {
                    #[cfg(all(feature = "native-presentations", target_os = "macos"))]
                    self.presentation_failure(Failure::Timeout);
                    return Ok(Attempt::Deferred);
                }
                wgpu::CurrentSurfaceTexture::Occluded => {
                    #[cfg(all(feature = "native-presentations", target_os = "macos"))]
                    self.presentation_failure(Failure::Occluded);
                    return Ok(Attempt::Deferred);
                }
                wgpu::CurrentSurfaceTexture::Outdated => {
                    #[cfg(all(feature = "native-presentations", target_os = "macos"))]
                    {
                        self.presentation_failure(Failure::Outdated);
                        self.presentation_boundary(Boundary::Retry);
                    }
                    self.configured = false;
                    return Ok(Attempt::Retry);
                }
                wgpu::CurrentSurfaceTexture::Lost => {
                    #[cfg(all(feature = "native-presentations", target_os = "macos"))]
                    {
                        self.presentation_failure(Failure::Lost);
                        self.presentation_boundary(Boundary::Retry);
                    }
                    self.recreate(renderer)?;
                    return Ok(Attempt::Retry);
                }
                wgpu::CurrentSurfaceTexture::Validation => {
                    #[cfg(all(feature = "native-presentations", target_os = "macos"))]
                    self.presentation_failure(Failure::Validation);
                    return Err(Error::SurfaceValidation);
                }
            };
            let view = texture.texture.create_view(&Default::default());
            Ok(Attempt::Ready(Frame {
                texture,
                view,
                format: self.config.format,
                width: self.config.width,
                height: self.config.height,
            }))
        })?;
        let available = frame.is_some();
        *self.slot.borrow_mut() = frame;
        #[cfg(all(feature = "native-presentations", target_os = "macos"))]
        if available && let Some(capture) = &mut self.capture {
            capture.reserved();
        }
        if !available {
            #[cfg(all(feature = "native-presentations", target_os = "macos"))]
            {
                self.presentation_boundary(Boundary::Retry);
                self.presentation_complete(Status::Deferred);
            }
            self.skipped += 1;
        }
        Ok(available)
    }
    /// Also used by the explicit numerical lifecycle probe: this recreates the
    /// actual native surface, but does not claim the OS generated a Lost event.
    pub fn recreate(&mut self, renderer: &Renderer) -> Result<(), Error> {
        self.cancel();
        #[cfg(all(feature = "native-presentations", target_os = "macos"))]
        if self.capture.is_some() {
            Capture::unbind(&self.surface)?;
        }
        self.surface = self.instance.create_surface(Arc::clone(&self.window))?;
        #[cfg(all(feature = "native-presentations", target_os = "macos"))]
        if let Some(capture) = &mut self.capture {
            capture.recreated();
        }
        let caps = self.surface.get_capabilities(&renderer.adapter);
        if !caps.formats.contains(&self.config.format)
            || !caps.alpha_modes.contains(&self.config.alpha_mode)
        {
            return Err(Error::SurfaceCapabilities);
        }
        self.recreations += 1;
        self.configured = false;
        Ok(())
    }
    pub fn consumed(&self) -> bool {
        self.slot.borrow().is_none()
    }
    /// Only a live reservation determines the dimensions that will be drawn.
    /// Never expose stale configuration after timeout, loss or cancellation.
    pub fn reserved_size(&self) -> Result<[u32; 2], Error> {
        self.slot
            .borrow()
            .as_ref()
            .map(|frame| [frame.width, frame.height])
            .ok_or(Error::Reservation)
    }
}

// Window constraints are requests: fullscreen, display-scale changes or an OS
// window manager can ignore them. Unsupported sizes defer simulation/presentation
// without allocating an oversized target or terminating the live session.
fn supported_size(width: u32, height: u32) -> bool {
    (16..=1920).contains(&width) && (16..=1080).contains(&height)
}
impl TargetProvider for SurfaceTargets {
    type Error = Error;
    type Frame = Frame;
    fn acquire(&mut self, _: &Renderer) -> Result<Frame, Error> {
        self.slot.borrow_mut().take().ok_or(Error::Reservation)
    }
}
impl RenderFrame for Frame {
    type Error = Error;
    fn target(&self) -> RetainedTarget<'_> {
        RetainedTarget {
            view: &self.view,
            format: self.format,
            width: self.width,
            height: self.height,
            viewport: None,
        }
    }
    fn present(self) -> Result<(), Error> {
        self.texture.present();
        Ok(())
    }
}

enum Attempt<T> {
    Ready(T),
    Retry,
    Deferred,
}
/// No more than two acquisitions per callback, even if the platform keeps
/// returning Lost/Outdated. Deferred frames do not enter the simulation scope.
fn bounded_acquire<T>(
    mut acquire: impl FnMut() -> Result<Attempt<T>, Error>,
) -> Result<Option<T>, Error> {
    for _ in 0..2 {
        match acquire()? {
            Attempt::Ready(frame) => return Ok(Some(frame)),
            Attempt::Retry => {}
            Attempt::Deferred => return Ok(None),
        }
    }
    Ok(None)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn os_size_changes_are_bounded_and_can_reenter_the_supported_range() {
        for size in [
            (0, 0),
            (15, 480),
            (640, 15),
            (1921, 1080),
            (1920, 1081),
            (3024, 1964),
            (u32::MAX, u32::MAX),
        ] {
            assert!(!supported_size(size.0, size.1));
        }
        for size in [(16, 16), (1920, 1080), (640, 480)] {
            assert!(supported_size(size.0, size.1));
        }
    }
    #[test]
    fn recovery_defers_repeated_loss_without_unbounded_acquisition() {
        let mut count = 0;
        let frame: Option<usize> = bounded_acquire(|| {
            count += 1;
            Ok(Attempt::Retry)
        })
        .unwrap();
        assert!(frame.is_none());
        assert_eq!(count, 2);
        let frame = bounded_acquire(|| {
            count += 1;
            Ok(Attempt::Ready(42))
        })
        .unwrap();
        assert_eq!(frame, Some(42));
        assert_eq!(count, 3);
    }
    #[test]
    fn transient_skip_and_fatal_validation_never_issue_extra_attempts() {
        let mut count = 0;
        let frame: Option<usize> = bounded_acquire(|| {
            count += 1;
            Ok(Attempt::Deferred)
        })
        .unwrap();
        assert!(frame.is_none());
        assert_eq!(count, 1);
        let error = bounded_acquire::<usize>(|| {
            count += 1;
            Err(Error::SurfaceValidation)
        });
        assert!(matches!(error, Err(Error::SurfaceValidation)));
        assert_eq!(count, 2);
    }
}
