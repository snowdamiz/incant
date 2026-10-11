//! Single-threaded surface ownership. Reserve before simulation, consume inside
//! mandatory sync, present before returning to the event loop. No frame survives
//! a resize callback, and only this module can construct a reservation/provider.
use super::Error;
use incant_render::{Renderer, RetainedTarget, wgpu};
use incant_render_gpu_host::{RenderFrame, TargetProvider};
use std::{cell::RefCell, rc::Rc, sync::Arc};
use tao::window::Window;

pub struct SurfaceState {
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
    pub fn new(window: Arc<Window>) -> Result<(Self, Arc<Renderer>, SurfaceTargets), Error> {
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(Arc::clone(&window))?;
        let renderer = Arc::new(
            pollster::block_on(Renderer::new(&instance, Some(&surface))).map_err(Error::Render)?,
        );
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
    pub fn invalidate(&mut self) {
        self.slot.borrow_mut().take();
        self.configured = false;
    }
    pub fn cancel(&mut self) {
        self.slot.borrow_mut().take();
    }
    pub fn reserve(&mut self, renderer: &Renderer) -> Result<bool, Error> {
        if self.slot.borrow().is_some() {
            return Err(Error::Reservation);
        }
        renderer.check_device().map_err(Error::Render)?;
        let size = self.window.inner_size();
        if size.width == 0 || size.height == 0 || self.window.is_minimized() {
            self.skipped += 1;
            return Ok(false);
        }
        if size.width < 16 || size.height < 16 || size.width > 1920 || size.height > 1080 {
            return Err(Error::SurfaceSize);
        }
        if (self.config.width, self.config.height) != (size.width, size.height) {
            self.config.width = size.width;
            self.config.height = size.height;
            self.configured = false;
        }
        let frame = bounded_acquire(|| {
            if !self.configured {
                self.surface.configure(&renderer.device, &self.config);
                self.configured = true;
                self.reconfigurations += 1;
            }
            let texture = match self.surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(frame) => frame,
                wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                    self.configured = false;
                    frame
                }
                wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                    return Ok(Attempt::Deferred);
                }
                wgpu::CurrentSurfaceTexture::Outdated => {
                    self.configured = false;
                    return Ok(Attempt::Retry);
                }
                wgpu::CurrentSurfaceTexture::Lost => {
                    self.recreate(renderer)?;
                    return Ok(Attempt::Retry);
                }
                wgpu::CurrentSurfaceTexture::Validation => return Err(Error::SurfaceValidation),
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
        if !available {
            self.skipped += 1;
        }
        Ok(available)
    }
    /// Also used by the explicit numerical lifecycle probe: this recreates the
    /// actual native surface, but does not claim the OS generated a Lost event.
    pub fn recreate(&mut self, renderer: &Renderer) -> Result<(), Error> {
        self.cancel();
        self.surface = self.instance.create_surface(Arc::clone(&self.window))?;
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
