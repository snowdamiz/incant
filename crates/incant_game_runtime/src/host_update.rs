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
    /// Request/retirement storage remains with the caller on every refusal and
    /// unwind. Native panic poisons the lease; its outer Drop destroys the
    /// composite after the caller's publication lock has unwound.
    pub fn update(&mut self, request: &mut H::Request) -> Result<H::Reply, GameError<H::Error>> {
        let live = self.live.as_mut().expect("owned host lease");
        let world = live.native.world()?;
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
