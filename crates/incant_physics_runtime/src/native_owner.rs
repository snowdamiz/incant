//! Native ownership access is explicit so read-only queries preserve a completed
//! checkpoint and every mutable borrow revokes it before native state can change.
use super::PhysicsError;
use rapier3d::pipeline::PhysicsWorld;
#[cfg(feature = "native-checkpoints")]
use rapier3d::pipeline::checked_restore::capture::CheckpointOwner;

pub(super) struct NativeOwner {
    #[cfg(feature = "native-checkpoints")]
    pub(super) inner: CheckpointOwner,
    #[cfg(not(feature = "native-checkpoints"))]
    inner: PhysicsWorld,
}
impl Default for NativeOwner {
    fn default() -> Self {
        Self {
            #[cfg(feature = "native-checkpoints")]
            inner: CheckpointOwner::for_simulation(),
            #[cfg(not(feature = "native-checkpoints"))]
            inner: PhysicsWorld::default(),
        }
    }
}
impl NativeOwner {
    pub(super) fn read(&self) -> &PhysicsWorld {
        #[cfg(feature = "native-checkpoints")]
        {
            self.inner
                .get()
                .expect("failed outer owner must hide native observations")
        }
        #[cfg(not(feature = "native-checkpoints"))]
        {
            &self.inner
        }
    }
    pub(super) fn edit(&mut self) -> &mut PhysicsWorld {
        #[cfg(feature = "native-checkpoints")]
        {
            self.inner
                .access_mut()
                .expect("failed outer owner must refuse native mutations")
        }
        #[cfg(not(feature = "native-checkpoints"))]
        {
            &mut self.inner
        }
    }
    pub(super) fn step(&mut self) -> Result<(), PhysicsError> {
        #[cfg(feature = "native-checkpoints")]
        {
            self.inner.step().map_err(|_| PhysicsError::NumericRange)
        }
        #[cfg(not(feature = "native-checkpoints"))]
        {
            self.inner.step();
            Ok(())
        }
    }
}
