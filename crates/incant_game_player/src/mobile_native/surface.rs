//! Platform-owned CAMetalLayer; an acquired drawable never crosses a callback.
use incant_render::{Renderer, RetainedTarget, wgpu};
use incant_render_gpu_host::{RenderFrame, TargetProvider};
use std::{cell::RefCell, ffi::c_void, ptr::NonNull, rc::Rc, sync::Arc};

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
    surface: wgpu::Surface<'static>,
    layer: NonNull<c_void>,
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
    /// The platform retains its valid CAMetalLayer through this owner's drop
    /// and serializes all mutations/callbacks on the view's owning thread.
    pub unsafe fn new(
        layer: NonNull<c_void>,
        size: [u32; 2],
    ) -> Result<(Self, Arc<Renderer>, Targets)> {
        validate_size(size)?;
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::METAL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        // SAFETY: the retained layer and owning-thread contract is the caller's.
        let surface = unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::CoreAnimationLayer(
                layer.as_ptr(),
            ))?
        };
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
                surface,
                layer,
                config,
                size,
                configured: false,
                slot,
            },
            renderer,
            targets,
        ))
    }
    pub fn resize(&mut self, size: [u32; 2]) -> Result<()> {
        validate_size(size)?;
        self.cancel();
        self.size = size;
        self.configured = false;
        Ok(())
    }
    pub fn cancel(&mut self) {
        self.slot.borrow_mut().take();
    }
    pub fn reserve(&mut self, renderer: &Renderer) -> Result<bool> {
        if self.slot.borrow().is_some() {
            return Err(SurfaceError::Reservation);
        }
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
                self.surface.configure(&renderer.device, &self.config);
                self.configured = true;
            }
            let texture = match self.surface.get_current_texture() {
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
                wgpu::CurrentSurfaceTexture::Lost => {
                    // SAFETY: the original platform layer still obeys new's lifetime contract.
                    self.surface = unsafe {
                        self.instance.create_surface_unsafe(
                            wgpu::SurfaceTargetUnsafe::CoreAnimationLayer(self.layer.as_ptr()),
                        )?
                    };
                    let caps = self.surface.get_capabilities(&renderer.adapter);
                    if !caps.formats.contains(&self.config.format)
                        || !caps.alpha_modes.contains(&self.config.alpha_mode)
                    {
                        return Err(SurfaceError::Capabilities);
                    }
                    self.configured = false;
                    continue;
                }
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
