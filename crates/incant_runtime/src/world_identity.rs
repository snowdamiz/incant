//! Process-local identity for retained consumers; never serialized or authored.
use std::{fmt, sync::Arc};

/// Identifies one native world across moves and successful in-place restores.
/// Distinct constructions never compare equal, even with identical scene bytes
/// and generation counters. Clone once when attaching a retained consumer, then
/// compare borrowed identities on subsequent frames without allocation/refcount
/// changes. Keeping a clone alive prevents pointer reuse from creating an ABA
/// match. This is not a stable entity/scene ID or authentication capability.
#[derive(Clone)]
pub struct WorldIdentity(Arc<()>);
impl WorldIdentity {
    pub(crate) fn new() -> Self {
        Self(Arc::new(()))
    }
}
impl PartialEq for WorldIdentity {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for WorldIdentity {}
impl fmt::Debug for WorldIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WorldIdentity").finish_non_exhaustive()
    }
}
