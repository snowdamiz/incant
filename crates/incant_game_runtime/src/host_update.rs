//! A host readiness lease established outside the caller's publication lock.
use crate::{
    GameError, SceneHostUpdate,
    core::{Live, WithHost},
};
use kira::backend::Backend;

pub struct PreparedHostUpdate<'a, B: Backend, H: SceneHostUpdate> {
    slot: &'a mut Option<Live<B, WithHost<H>>>,
    live: Option<Live<B, WithHost<H>>>,
    preserve: bool,
}
impl<'a, B: Backend, H: SceneHostUpdate> PreparedHostUpdate<'a, B, H> {
    pub(crate) fn new(
        slot: &'a mut Option<Live<B, WithHost<H>>>,
        live: Live<B, WithHost<H>>,
    ) -> Self {
        Self {
            slot,
            live: Some(live),
            preserve: true,
        }
    }
    /// Typed replies carry their own refusals and retired owners. A failed world
    /// check returns the original request intact. Native panic poisons the lease;
    /// its outer Drop destroys the composite after the caller's lock unwinds.
    pub fn update(
        &mut self,
        request: H::Request,
    ) -> Result<H::Reply, (GameError<H::Error>, H::Request)> {
        let live = self.live.as_mut().expect("owned host lease");
        let world = match live.native.world() {
            Ok(world) => world,
            Err(error) => return Err((error.into(), request)),
        };
        self.preserve = false;
        let reply = live.host.0.update(world, request);
        self.preserve = true;
        Ok(reply)
    }
}
impl<B: Backend, H: SceneHostUpdate> Drop for PreparedHostUpdate<'_, B, H> {
    fn drop(&mut self) {
        if self.preserve {
            *self.slot = self.live.take();
        }
    }
}
