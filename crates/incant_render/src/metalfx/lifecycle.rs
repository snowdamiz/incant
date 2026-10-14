//! No native pointer enters a callback. A completion lease keeps only safe wgpu
//! textures alive; the raw command buffer separately retains its native scaler.
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
const CAPACITY: usize = 3;
pub(super) struct Pool([Arc<AtomicBool>; CAPACITY]);
impl Default for Pool {
    fn default() -> Self {
        Self(std::array::from_fn(|_| Arc::new(AtomicBool::new(false))))
    }
}
pub(super) struct Lease(Arc<AtomicBool>);
impl Pool {
    pub fn reserve(&self) -> Option<Lease> {
        self.0.iter().find_map(|slot| {
            slot.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .ok()
                .map(|_| Lease(Arc::clone(slot)))
        })
    }
}
impl Lease {
    /// The exact callback used by the graph: only safe Send resources are held.
    /// Dropping an unsubmitted callback releases the same lease as completion.
    pub fn completion<T: wgpu::WasmNotSend + 'static>(
        self,
        resources: T,
    ) -> impl FnOnce() + wgpu::WasmNotSend + 'static {
        move || drop((self, resources))
    }
}
impl Drop for Lease {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}
/// Lifetime accounting includes accepted, candidate and callback-retired targets.
#[derive(Default)]
pub(super) struct Budget(Arc<AtomicU64>);
pub(super) struct Allocation {
    used: Arc<AtomicU64>,
    bytes: u64,
}
impl Budget {
    #[cfg(test)]
    pub fn check(&self, bytes: u64) -> Result<(), super::MetalFxError> {
        self.0
            .load(Ordering::Acquire)
            .checked_add(bytes)
            .filter(|n| *n <= super::MAX_METALFX_TRANSITION_BYTES)
            .map(|_| ())
            .ok_or(super::MetalFxError::Capacity)
    }
    pub fn reserve(&self, bytes: u64) -> Result<Allocation, super::MetalFxError> {
        self.0
            .try_update(Ordering::AcqRel, Ordering::Acquire, |used| {
                used.checked_add(bytes)
                    .filter(|n| *n <= super::MAX_METALFX_TRANSITION_BYTES)
            })
            .map_err(|_| super::MetalFxError::Capacity)?;
        Ok(Allocation {
            used: Arc::clone(&self.0),
            bytes,
        })
    }
}
impl Drop for Allocation {
    fn drop(&mut self) {
        self.used.fetch_sub(self.bytes, Ordering::AcqRel);
    }
}
#[derive(Default)]
pub(super) struct History {
    dirty: bool,
    accepted: Option<u64>,
    dimensions: Option<[u32; 4]>,
}
impl History {
    pub fn reset(&self, sequence: u64, has_previous: bool, dimensions: [u32; 4]) -> bool {
        self.dirty
            || !has_previous
            || self.dimensions != Some(dimensions)
            || self.accepted.and_then(|n| n.checked_add(1)) != Some(sequence)
    }
    /// Mark before entering native code. Any unwinding/failed post-encoding leaves
    /// a required reset; only the single successful batch submission clears it.
    pub fn encoding(&mut self) {
        self.dirty = true;
    }
    pub fn accept(&mut self, sequence: u64, dimensions: [u32; 4]) {
        self.accepted = Some(sequence);
        self.dimensions = Some(dimensions);
        self.dirty = false;
    }
    pub fn invalidate(&mut self) {
        self.dirty = true;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dropped_pending_and_late_completion_cannot_reuse_busy_lease() {
        let pool = Pool::default();
        let a = pool.reserve().unwrap();
        let b = pool.reserve().unwrap();
        let c = pool.reserve().unwrap();
        assert!(pool.reserve().is_none());
        drop(b);
        let replacement = pool.reserve().unwrap();
        drop(a);
        drop(c);
        let x = pool.reserve().unwrap();
        let y = pool.reserve().unwrap();
        assert!(pool.reserve().is_none());
        drop(replacement);
        assert!(pool.reserve().is_some());
        drop((x, y));
    }
    #[test]
    fn abandoned_native_encode_and_backend_change_require_reset() {
        let mut h = History::default();
        assert!(h.reset(1, false, [48, 32, 96, 64]));
        h.encoding();
        h.accept(1, [48, 32, 96, 64]);
        assert!(!h.reset(2, true, [48, 32, 96, 64]));
        h.encoding();
        assert!(h.reset(2, true, [48, 32, 96, 64]));
        h.accept(2, [48, 32, 96, 64]);
        assert!(!h.reset(3, true, [48, 32, 96, 64]));
        h.invalidate();
        assert!(h.reset(3, true, [48, 32, 96, 64]));
        h.accept(3, [48, 32, 96, 64]);
        assert!(h.reset(4, true, [96, 64, 96, 64]));
        assert!(h.reset(4, true, [48, 32, 192, 128]));
        assert!(h.reset(5, true, [48, 32, 96, 64]));
        assert!(h.reset(4, false, [48, 32, 96, 64]));
    }
    #[test]
    fn preference_never_silently_accepts_a_required_native_path() {
        use crate::metalfx::{Fallback, MetalFxError, Preference, Selection};
        assert_eq!(
            Preference::PreferMetalFx.select(Some(Fallback::Aspect)),
            Ok(Selection::Spatial(Some(Fallback::Aspect)))
        );
        assert_eq!(
            Preference::RequireMetalFx.select(Some(Fallback::Aspect)),
            Err(MetalFxError::Unavailable(Fallback::Aspect))
        );
        assert_eq!(
            Preference::Spatial.select(None),
            Ok(Selection::Spatial(None))
        );
        assert_eq!(
            Preference::RequireMetalFx.select(Some(Fallback::PixelCamera)),
            Err(MetalFxError::Unavailable(Fallback::PixelCamera))
        );
    }
    #[test]
    fn actual_completion_closure_retains_resources_after_owner_drop_and_abandon() {
        let pool = Pool::default();
        let resource = Arc::new(17u32);
        let weak = Arc::downgrade(&resource);
        let completion = pool.reserve().unwrap().completion(Arc::clone(&resource));
        drop(resource);
        drop(pool);
        assert!(weak.upgrade().is_some());
        std::thread::spawn(completion).join().unwrap();
        assert!(weak.upgrade().is_none());
        let pool = Pool::default();
        let resource = Arc::new(29u32);
        let weak = Arc::downgrade(&resource);
        let abandoned = pool.reserve().unwrap().completion(resource);
        let a = pool.reserve().unwrap();
        let b = pool.reserve().unwrap();
        assert!(pool.reserve().is_none());
        drop(abandoned);
        assert!(weak.upgrade().is_none());
        assert!(pool.reserve().is_some());
        drop((a, b));
    }
    #[test]
    fn in_flight_retired_allocations_remain_charged_until_actual_completion() {
        let budget = Budget::default();
        let max = super::super::MAX_METALFX_TRANSITION_BYTES;
        let old = budget.reserve(max / 2).unwrap();
        let candidate = budget.reserve(max / 2).unwrap();
        assert!(budget.reserve(1).is_err());
        assert!(budget.check(u64::MAX).is_err());
        let pool = Pool::default();
        let completion = pool.reserve().unwrap().completion(old);
        drop(candidate);
        let replacement = budget.reserve(max / 2).unwrap();
        assert!(budget.reserve(1).is_err());
        std::thread::spawn(completion).join().unwrap();
        budget.check(max / 2).unwrap();
        drop(replacement);
        budget.check(max).unwrap();
        let abandoned = budget.reserve(max).unwrap();
        assert!(budget.check(1).is_err());
        drop(abandoned);
        budget.check(max).unwrap();
    }
}
