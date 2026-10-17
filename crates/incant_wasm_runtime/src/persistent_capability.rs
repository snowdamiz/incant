//! Explicit boundary operations; no guest callback or capture-time state mutation.
use super::*;
use crate::persistent::{Declaration, PersistentSnapshot, PersistentState};

impl Capability {
    pub fn with_persistence(declaration: Option<Declaration>) -> Self {
        Self {
            persistent: declaration.map(PersistentState::new),
            ..Self::default()
        }
    }
    /// Non-consuming admission check. A save cannot acknowledge pending restart.
    pub fn checkpoint_ready(&self, world: &NativeWorld) -> Result<(), Error> {
        if self.failed || self.active {
            return Err(Error::Failed);
        }
        self.haptics.ready()?;
        if self.restart_committed {
            return Err(Error::Abi("pending session restart request"));
        }
        if self.physics.as_ref().is_some_and(|state| state.bound()) {
            return Err(Error::Abi("bound physics loan"));
        }
        let identity = world.spatial_view()?.identity().clone();
        if self.identity.as_ref().is_some_and(|old| *old != identity) {
            return Err(Error::Abi("different checkpoint world"));
        }
        if self.tick != world.tick() {
            return Err(Error::Abi("world/module tick pairing"));
        }
        Ok(())
    }
    pub fn persistent_checkpoint(&self, world: &NativeWorld) -> Result<PersistentSnapshot, Error> {
        self.checkpoint_ready(world)?;
        self.persistent
            .as_ref()
            .ok_or(Error::Abi("module has no persistent state declaration"))?
            .snapshot(self.tick, self.epoch)
    }
    /// The receiver must be a fresh, inactive VM. Startup callbacks cannot have
    /// obtained tick authority. Only trusted receiving-owner restart opt-in survives.
    pub fn restore_persistent(
        &mut self,
        world: &NativeWorld,
        snapshot: &PersistentSnapshot,
        receiver_watermark: i64,
        restart_enabled: bool,
    ) -> Result<(), Error> {
        if self.failed
            || self.active
            || self.tick != 0
            || self.identity.is_some()
            || self.restart_committed
        {
            return Err(Error::Abi("persistent restore requires a fresh host"));
        }
        self.haptics.ready()?;
        if snapshot.tick() != world.tick() || !(0..i64::MAX).contains(&receiver_watermark) {
            return Err(Error::Abi("persistent restore clock/watermark"));
        }
        let identity = world.spatial_view()?.identity().clone();
        self.persistent
            .as_mut()
            .ok_or(Error::Abi("module has no persistent state declaration"))?
            .restore(snapshot)?;
        self.haptics.clear();
        self.epoch = snapshot.watermark().max(receiver_watermark);
        self.tick = snapshot.tick();
        self.identity = Some(identity);
        self.restart_enabled = restart_enabled;
        Ok(())
    }
    pub fn checkpoint_authority(&self, world: &NativeWorld) -> Result<(i64, bool), Error> {
        self.checkpoint_ready(world)?;
        Ok((self.epoch, self.restart_enabled))
    }
    /// Validate compatibility before moving committed data out of the old host.
    /// Same-layout reload preserves persistent data, revocation and accepted clock.
    pub fn prepare_reload(&self, candidate: &Self) -> Result<(), Error> {
        if self.active
            || candidate.active
            || candidate.failed
            || candidate.tick != 0
            || candidate.identity.is_some()
            || candidate.restart_committed
        {
            return Err(Error::Abi("reload boundary"));
        }
        self.haptics.ready()?;
        candidate.haptics.ready()?;
        if self.persistent.as_ref().map(PersistentState::declaration)
            != candidate
                .persistent
                .as_ref()
                .map(PersistentState::declaration)
        {
            return Err(Error::Abi("persistent reload declaration mismatch"));
        }
        // A failed persistent transaction cannot become a healthy restored state.
        if self.persistent.is_some() && (self.failed || self.restart_committed) {
            return Err(Error::Failed);
        }
        Ok(())
    }
}
