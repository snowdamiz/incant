use super::{
    Config, Fallback, MetalFxError, Preference, Selection,
    lifecycle::{Budget, History, Lease, Pool},
    plan::Plan,
    targets::Targets,
};
#[cfg(target_os = "macos")]
use std::cell::RefCell;
use std::{marker::PhantomData, rc::Rc, sync::Arc};
#[cfg(target_os = "macos")]
type Scaler = Rc<RefCell<super::native::Scaler>>;
struct Prepared {
    targets: Arc<Targets>,
    #[cfg(target_os = "macos")]
    scaler: Scaler,
}

pub(crate) struct Owner {
    preference: Preference,
    range: Result<[f32; 2], Fallback>,
    config: Config,
    targets: Option<Arc<Targets>>,
    #[cfg(target_os = "macos")]
    scaler: Option<Scaler>,
    prepared: Option<Prepared>,
    preparation_failure: Option<([u32; 2], Fallback)>,
    history: History,
    pool: Pool,
    budget: Budget,
    accepted: Option<Selection>,
    _thread: PhantomData<Rc<()>>,
}
pub(crate) struct Pending {
    pub selection: Selection,
    pub(super) plan: Option<Plan>,
    pub(super) targets: Option<Arc<Targets>>,
    #[cfg(target_os = "macos")]
    scaler: Option<Scaler>,
    lease: Option<Lease>,
    sequence: u64,
    reset: bool,
    pub(super) jitter: [f32; 2],
    pub(super) previous_jitter: [f32; 2],
    pub(super) config: Config,
    pub(super) has_previous: bool,
}
impl Owner {
    pub fn new(
        preference: Preference,
        config: Config,
        range: Result<[f32; 2], Fallback>,
    ) -> Result<Self, MetalFxError> {
        config.validate()?;
        preference.select(range.err())?;
        Ok(Self {
            preference,
            range,
            config,
            targets: None,
            #[cfg(target_os = "macos")]
            scaler: None,
            prepared: None,
            preparation_failure: None,
            history: History::default(),
            pool: Pool::default(),
            budget: Budget::default(),
            accepted: None,
            _thread: PhantomData,
        })
    }
    pub fn selection(&self) -> Option<Selection> {
        self.accepted
    }
    /// Explicit host/startup work. Ordinary submit never constructs a scaler or
    /// allocates these vendor boundary targets. Failed preparation preserves the
    /// accepted owner and any older prepared extent.
    pub fn prepare_extent(
        &mut self,
        renderer: &crate::Renderer,
        size: [u32; 2],
    ) -> Result<(), MetalFxError> {
        let extent = [0., 0., size[0] as f32, size[1] as f32];
        let p = Plan::new(extent, extent, [1., 4.]).map_err(MetalFxError::Unavailable)?;
        p.preflight(
            self.targets.as_ref().map_or(0, |t| t.bytes),
            renderer.device.limits().max_texture_dimension_2d,
        )?;
        if self.preference.select(self.range.err())? != Selection::MetalFx {
            return Ok(());
        }
        if self.targets.as_ref().is_some_and(|t| t.size == size) {
            self.prepared = None;
            self.preparation_failure = None;
            return Ok(());
        }
        if self
            .prepared
            .as_ref()
            .is_some_and(|p| p.targets.size == size)
        {
            return Ok(());
        }
        let allocation = self.budget.reserve(p.bytes()?)?;
        #[cfg(target_os = "macos")]
        {
            let scaler = match super::native::Scaler::new(&renderer.device, size) {
                Ok(s) => s,
                Err(reason) => {
                    self.preference.select(Some(reason))?;
                    self.preparation_failure = Some((size, reason));
                    return Ok(());
                }
            };
            let targets = Arc::new(Targets::new(&renderer.device, p, allocation)?);
            self.prepared = Some(Prepared {
                targets,
                scaler: Rc::new(RefCell::new(scaler)),
            });
            self.preparation_failure = None;
            Ok(())
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = allocation;
            Err(MetalFxError::Unavailable(Fallback::Backend))
        }
    }
    /// Admission and leases only; all resources already exist from explicit
    /// preparation. No initialization work is hidden in retained submission.
    pub fn prepare(
        &self,
        renderer: &crate::Renderer,
        scene: &crate::RenderScene,
        overlays: bool,
        frame: &crate::resolution::FramePlan,
        output: [f32; 4],
    ) -> Result<Pending, MetalFxError> {
        let reason = if scene.camera.pixel_resolution.is_some() {
            Some(Fallback::PixelCamera)
        } else if overlays {
            Some(Fallback::Overlays)
        } else if renderer
            .bloom
            .as_ref()
            .is_some_and(|b| b.config.intensity > 0.)
        {
            Some(Fallback::Bloom)
        } else {
            self.range.err()
        };
        let checked =
            reason.map_or_else(|| Plan::new(frame.rect, output, self.range.unwrap()), Err);
        let plan = checked.ok();
        let mut selection = self.preference.select(checked.err())?;
        let mut targets = None;
        let mut lease = None;
        #[cfg(target_os = "macos")]
        let mut scaler = None;
        if selection == Selection::MetalFx {
            let p = plan.ok_or(MetalFxError::Interop)?;
            p.preflight(0, renderer.device.limits().max_texture_dimension_2d)?;
            if let Some(t) = self.targets.as_ref().filter(|t| t.size == p.size()) {
                targets = Some(Arc::clone(t));
                #[cfg(target_os = "macos")]
                {
                    scaler = self.scaler.clone();
                }
            } else if let Some(prepared) = self
                .prepared
                .as_ref()
                .filter(|v| v.targets.size == p.size())
            {
                targets = Some(Arc::clone(&prepared.targets));
                #[cfg(target_os = "macos")]
                {
                    scaler = Some(Rc::clone(&prepared.scaler));
                }
            } else {
                let reason = self
                    .preparation_failure
                    .filter(|(size, _)| *size == p.size())
                    .map_or(Fallback::UnpreparedExtent, |(_, reason)| reason);
                selection = self.preference.select(Some(reason))?;
            }
        }
        if selection == Selection::MetalFx {
            lease = self.pool.reserve();
            if lease.is_none() {
                selection = self.preference.select(Some(Fallback::Busy))?;
            }
        }
        Ok(Pending {
            selection,
            plan,
            targets,
            #[cfg(target_os = "macos")]
            scaler,
            lease,
            sequence: 0,
            reset: true,
            jitter: [0.; 2],
            previous_jitter: [0.; 2],
            config: self.config,
            has_previous: false,
        })
    }
    pub fn bind_motion(
        &self,
        p: &mut Pending,
        frame: &crate::motion::MotionFrame,
    ) -> Result<(), MetalFxError> {
        if !p.native() {
            return Ok(());
        }
        let phase = frame.jitter_phase().ok_or(MetalFxError::Interop)?;
        p.sequence = frame.sequence();
        p.has_previous = frame.has_previous();
        p.reset = self.accepted != Some(Selection::MetalFx)
            || self.history.reset(
                p.sequence,
                p.has_previous,
                p.plan.ok_or(MetalFxError::Interop)?.dimensions(),
            );
        p.jitter = crate::temporal::jitter::SAMPLES[phase as usize];
        p.previous_jitter = if p.has_previous {
            crate::temporal::jitter::SAMPLES[(phase as usize + 7) % 8]
        } else {
            p.jitter
        };
        Ok(())
    }
    pub fn encode(
        &mut self,
        device: &wgpu::Device,
        p: &mut Pending,
    ) -> Result<wgpu::CommandBuffer, MetalFxError> {
        self.history.encoding();
        #[cfg(target_os = "macos")]
        {
            let mut scaler = p
                .scaler
                .as_ref()
                .ok_or(MetalFxError::Interop)?
                .try_borrow_mut()
                .map_err(|_| MetalFxError::Interop)?;
            let plan = p.plan.ok_or(MetalFxError::Interop)?;
            scaler.encode(
                device,
                &p.targets.as_ref().ok_or(MetalFxError::Interop)?.textures,
                [plan.source[2], plan.source[3]],
                p.jitter,
                p.reset,
            )
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (device, p);
            Err(MetalFxError::Unavailable(Fallback::Backend))
        }
    }
    pub fn accept(&mut self, p: Pending) {
        if p.native() {
            self.history
                .accept(p.sequence, p.plan.expect("native plan").dimensions());
            #[cfg(target_os = "macos")]
            {
                self.scaler = p.scaler;
            }
            if self.prepared.as_ref().is_some_and(|v| {
                p.targets
                    .as_ref()
                    .is_some_and(|t| Arc::ptr_eq(t, &v.targets))
            }) {
                self.prepared = None;
            }
            self.targets = p.targets;
        } else {
            self.history.invalidate();
        }
        self.accepted = Some(p.selection);
    }
}
impl Pending {
    pub fn native(&self) -> bool {
        self.selection == Selection::MetalFx
    }
    pub fn completion(&mut self, encoder: &mut wgpu::CommandEncoder) {
        if let Some(lease) = self.lease.take() {
            encoder.on_submitted_work_done(lease.completion(self.targets.clone()));
        }
    }
}

#[cfg(all(test, target_os = "macos"))]
#[path = "prewarm_tests.rs"]
mod prewarm_tests;
