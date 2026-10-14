//! Each wgpu surface owns an acquired ANativeWindow; frames never cross callbacks.
use incant_render::{Renderer, RetainedTarget, wgpu};
use incant_render_gpu_host::{RenderFrame, TargetProvider};
use ndk::native_window::NativeWindow;
use std::{cell::RefCell, rc::Rc, sync::Arc};
use wgpu::rwh::{DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, WindowHandle};

struct Window(NativeWindow);
impl HasWindowHandle for Window {
    fn window_handle(&self) -> std::result::Result<WindowHandle<'_>, HandleError> {
        self.0.window_handle()
    }
}
impl HasDisplayHandle for Window {
    fn display_handle(&self) -> std::result::Result<DisplayHandle<'_>, HandleError> {
        Ok(DisplayHandle::android())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SurfaceError {
    #[error("native surface creation failed: {0}")]
    Create(#[from] wgpu::CreateSurfaceError),
    #[error("native renderer failed: {0}")]
    Render(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("native surface does not support the required format")]
    Capabilities,
    #[error("native surface frame reservation is invalid")]
    Reservation,
    #[error("native surface dimensions exceed the bounded output domain")]
    Dimensions,
    #[error("native surface validation failed")]
    Validation,
}
type Result<T> = std::result::Result<T, SurfaceError>;
pub(super) struct Surface {
    instance: wgpu::Instance,
    surface: Option<wgpu::Surface<'static>>,
    config: wgpu::SurfaceConfiguration,
    size: [u32; 2],
    configured: bool,
    slot: Rc<RefCell<Option<Frame>>>,
}
pub struct Targets(Rc<RefCell<Option<Frame>>>);
pub struct Frame {
    texture: wgpu::SurfaceTexture,
    view: wgpu::TextureView,
    format: wgpu::TextureFormat,
    size: [u32; 2],
}
pub(super) fn validate_size(size: [u32; 2]) -> Result<()> {
    if size.contains(&0) {
        return Ok(()); // invisible platform view, no target allocation
    }
    if size.iter().all(|n| (16..=4096).contains(n))
        && u64::from(size[0]) * u64::from(size[1]) <= 4 * 1024 * 1024
    {
        Ok(())
    } else {
        Err(SurfaceError::Dimensions)
    }
}
impl Surface {
    pub fn new(window: NativeWindow, size: [u32; 2]) -> Result<(Self, Arc<Renderer>, Targets)> {
        validate_size(size)?;
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let surface = instance.create_surface(Window(window))?;
        let renderer = Arc::new(
            pollster::block_on(Renderer::new(&instance, Some(&surface)))
                .map_err(SurfaceError::Render)?,
        );
        let caps = surface.get_capabilities(&renderer.adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .ok_or(SurfaceError::Capabilities)?;
        let alpha_mode = *caps.alpha_modes.first().ok_or(SurfaceError::Capabilities)?;
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
        let targets = Targets(Rc::clone(&slot));
        Ok((
            Self {
                instance,
                surface: Some(surface),
                config,
                size,
                configured: false,
                slot,
            },
            renderer,
            targets,
        ))
    }
    pub fn detach(&mut self) {
        self.cancel();
        self.surface.take();
        self.configured = false;
    }
    pub fn attach(
        &mut self,
        window: NativeWindow,
        size: [u32; 2],
        renderer: &Renderer,
    ) -> Result<()> {
        validate_size(size)?;
        let surface = self.instance.create_surface(Window(window))?;
        let caps = surface.get_capabilities(&renderer.adapter);
        if !caps.formats.contains(&self.config.format)
            || !caps.alpha_modes.contains(&self.config.alpha_mode)
            || !caps.present_modes.contains(&wgpu::PresentMode::Fifo)
        {
            return Err(SurfaceError::Capabilities);
        }
        self.detach();
        self.surface = Some(surface);
        self.size = size;
        Ok(())
    }
    pub fn cancel(&mut self) {
        self.slot.borrow_mut().take();
    }
    pub fn reserve(&mut self, renderer: &Renderer) -> Result<bool> {
        if self.slot.borrow().is_some() {
            return Err(SurfaceError::Reservation);
        }
        let Some(surface) = self.surface.as_ref() else {
            return Ok(false);
        };
        if self.size.contains(&0) {
            return Ok(false);
        }
        renderer.check_device().map_err(SurfaceError::Render)?;
        let limit = renderer.device.limits().max_texture_dimension_2d;
        if self.size.iter().any(|n| *n > limit) {
            return Err(SurfaceError::Dimensions);
        }
        for _ in 0..2 {
            if !self.configured {
                self.config.width = self.size[0];
                self.config.height = self.size[1];
                surface.configure(&renderer.device, &self.config);
                self.configured = true;
            }
            let texture = match surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(frame) => frame,
                wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                    self.configured = false;
                    frame
                }
                wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                    return Ok(false);
                }
                wgpu::CurrentSurfaceTexture::Outdated => {
                    self.configured = false;
                    continue;
                }
                // A lost Android surface requires a new SurfaceHolder callback.
                // No stale native-window handle is silently reused.
                wgpu::CurrentSurfaceTexture::Lost => return Err(SurfaceError::Validation),
                wgpu::CurrentSurfaceTexture::Validation => return Err(SurfaceError::Validation),
            };
            let view = texture.texture.create_view(&Default::default());
            *self.slot.borrow_mut() = Some(Frame {
                texture,
                view,
                format: self.config.format,
                size: self.size,
            });
            return Ok(true);
        }
        Ok(false)
    }
    pub fn consumed(&self) -> bool {
        self.slot.borrow().is_none()
    }
}
impl TargetProvider for Targets {
    type Error = SurfaceError;
    type Frame = Frame;
    fn acquire(&mut self, _: &Renderer) -> Result<Frame> {
        self.0.borrow_mut().take().ok_or(SurfaceError::Reservation)
    }
}
impl RenderFrame for Frame {
    type Error = SurfaceError;
    fn target(&self) -> RetainedTarget<'_> {
        RetainedTarget {
            view: &self.view,
            format: self.format,
            width: self.size[0],
            height: self.size[1],
            viewport: None,
        }
    }
    fn present(self) -> Result<()> {
        self.texture.present();
        Ok(())
    }
}
