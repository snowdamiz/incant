use super::{
    Fallback, MetalFxError, Preference, Selection,
    lifecycle::{Budget, History, Lease, Pool},
    plan::Plan,
    targets::Targets,
};
use std::{marker::PhantomData, rc::Rc, sync::Arc};

pub(crate) struct Owner {
    preference: Preference,
    range: Result<[f32; 2], Fallback>,
    targets: Option<Arc<Targets>>,
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    scaler: Option<super::native::Scaler>,
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
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    candidate: Option<super::native::Scaler>,
    lease: Option<Lease>,
    sequence: u64,
    reset: bool,
    jitter: [f32; 2],
}
impl Owner {
    pub fn new(
        d: &wgpu::Device,
        preference: Preference,
        support: Result<(), Fallback>,
    ) -> Result<Self, MetalFxError> {
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        let range = support.and_then(|_| super::native::range(d));
        #[cfg(not(any(target_os = "macos", target_os = "ios")))]
        let range = {
            let _ = (d, support);
            Err(Fallback::Backend)
        };
        preference.select(range.err())?;
        Ok(Self {
            preference,
            range,
            targets: None,
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            scaler: None,
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
    #[cfg(test)]
    pub(crate) fn cached_texture(&self) -> Option<wgpu::Texture> {
        self.targets.as_ref().map(|t| t.textures[0].clone())
    }
    /// Called before either motion or upscaling targets may allocate.
    pub fn preflight(
        &self,
        d: &wgpu::Device,
        plan: Option<crate::upscaling::Plan>,
    ) -> Result<Option<Plan>, MetalFxError> {
        if self.preference == Preference::Portable {
            return Ok(None);
        }
        let checked = self.range.and_then(|range| {
            plan.ok_or(Fallback::PixelCamera)
                .and_then(|p| Plan::new(p.source_rect, p.rect, range))
        });
        match checked {
            Ok(p) => {
                p.preflight(
                    self.targets.as_ref().map_or(0, |t| t.bytes),
                    d.limits().max_texture_dimension_2d,
                )?;
                if self.targets.as_ref().is_none_or(|t| t.size != p.size()) {
                    self.budget.check(p.bytes()?)?;
                }
                Ok(Some(p))
            }
            Err(reason) => {
                self.preference.select(Some(reason))?;
                Ok(None)
            }
        }
    }
    pub fn prepare(
        &self,
        d: &wgpu::Device,
        p: &mut crate::upscaling::Pending,
    ) -> Result<Pending, MetalFxError> {
        let data = p.native_input();
        let plan = self.preflight(d, data.map(|(_, p, _, _)| p))?;
        let input = data.map(|(i, _, _, _)| i);
        let sequence = input.map_or(0, |i| i.frame.sequence());
        let mut reason = self.range.err().or_else(|| {
            if data.is_none() {
                Some(Fallback::PixelCamera)
            } else {
                None
            }
        });
        if reason.is_none() && plan.is_none() && self.preference != Preference::Portable {
            let (_, q, _, _) = data.unwrap();
            reason = Plan::new(q.source_rect, q.rect, self.range.unwrap()).err();
        }
        let lease = plan.and_then(|_| self.pool.reserve());
        if plan.is_some() && lease.is_none() {
            reason = Some(Fallback::Busy);
        }
        let mut selection = self.preference.select(reason)?;
        if self.preference == Preference::Portable {
            selection = Selection::Portable(None);
        }
        let mut targets = None;
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        let mut candidate = None;
        if selection == Selection::MetalFx {
            let plan = plan.ok_or(MetalFxError::Interop)?;
            let allocation = if self.targets.as_ref().is_none_or(|t| t.size != plan.size()) {
                Some(self.budget.reserve(plan.bytes()?)?)
            } else {
                None
            };
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            if self.scaler.as_ref().is_none_or(|s| s.size != plan.size()) {
                match super::native::Scaler::new(d, plan.size()) {
                    Ok(s) => candidate = Some(s),
                    Err(e) => selection = self.preference.select(Some(e))?,
                }
            }
            if selection == Selection::MetalFx {
                targets = Some(match &self.targets {
                    Some(t) if t.size == plan.size() => Arc::clone(t),
                    _ => Arc::new(Targets::new(
                        d,
                        plan,
                        allocation.ok_or(MetalFxError::Interop)?,
                    )?),
                });
            }
        }
        // Conservatively reset native history on content-size changes. The portable
        // path preserves output history; no cross-size native motion convention
        // is assumed for Apple's private temporal state.
        let reset = self.accepted != Some(Selection::MetalFx)
            || plan.is_none_or(|p| {
                self.history.reset(
                    sequence,
                    input.is_some_and(|i| i.frame.has_previous()),
                    p.dimensions(),
                )
            });
        let jitter = input.zip(plan).map_or([0., 0.], |(i, p)| {
            [
                i.jitter[0] * p.source[2] as f32,
                i.jitter[1] * p.source[3] as f32,
            ]
        });
        if self.accepted != Some(selection) {
            p.discard_color_history();
        }
        Ok(Pending {
            selection,
            plan,
            targets,
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            candidate,
            lease,
            sequence,
            reset,
            jitter,
        })
    }
    pub fn encode(
        &mut self,
        d: &wgpu::Device,
        p: &mut Pending,
    ) -> Result<wgpu::CommandBuffer, MetalFxError> {
        self.history.encoding();
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        {
            let scaler = p
                .candidate
                .as_mut()
                .or(self.scaler.as_mut())
                .ok_or(MetalFxError::Interop)?;
            let plan = p.plan.ok_or(MetalFxError::Interop)?;
            scaler.encode(
                d,
                &p.targets.as_ref().ok_or(MetalFxError::Interop)?.textures,
                [plan.source[2], plan.source[3]],
                p.jitter,
                p.reset,
            )
        }
        #[cfg(not(any(target_os = "macos", target_os = "ios")))]
        {
            let _ = (d, p.reset, p.jitter);
            Err(MetalFxError::Unavailable(Fallback::Backend))
        }
    }
    pub fn accept(&mut self, p: Pending) {
        if p.selection == Selection::MetalFx {
            self.history.accept(
                p.sequence,
                p.plan.expect("native pending plan").dimensions(),
            );
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            if let Some(s) = p.candidate {
                self.scaler = Some(s);
            }
            self.targets = p.targets;
        } else {
            self.history.invalidate();
            // Keep the single bounded cache through temporary Busy/fractional
            // fallback. Returning at the same extent must not recreate native
            // pipelines. Owner disable/replacement drops it; history is reset.
        }
        self.accepted = Some(p.selection);
    }
}
impl Pending {
    pub fn native(&self) -> bool {
        self.selection == Selection::MetalFx
    }
    pub fn completion(&mut self) -> impl FnOnce(&mut wgpu::CommandEncoder) + use<> {
        let lease = self.lease.take();
        let targets = self.targets.clone();
        move |encoder| {
            if let Some(lease) = lease {
                encoder.on_submitted_work_done(lease.completion(targets));
            }
        }
    }
}
