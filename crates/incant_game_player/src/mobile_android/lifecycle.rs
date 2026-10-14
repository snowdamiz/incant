//! Bounded platform identities, independent of GPU/audio availability.
use incant_input::MAX_TOUCHES;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum LifecycleError {
    #[error("Android host is busy or belongs to another callback thread")]
    Busy,
    #[error("Android host identity exhausted")]
    Exhausted,
    #[error("Android callback is stale")]
    Stale,
    #[error("Android touch identity or capacity is invalid")]
    Touch,
}
pub struct Gate {
    claimed: AtomicBool,
    next: AtomicU64,
}
pub struct Claim<'a> {
    gate: &'a Gate,
    id: i64,
    published: bool,
}
impl Claim<'_> {
    pub fn id(&self) -> i64 {
        self.id
    }
    pub fn publish(mut self) {
        self.published = true;
    }
}
impl Drop for Claim<'_> {
    fn drop(&mut self) {
        if !self.published {
            self.gate.release();
        }
    }
}
impl Gate {
    pub const fn new() -> Self {
        Self {
            claimed: AtomicBool::new(false),
            next: AtomicU64::new(1),
        }
    }
    pub fn reserve(&self) -> Result<Claim<'_>, LifecycleError> {
        Ok(Claim {
            gate: self,
            id: self.claim()?,
            published: false,
        })
    }
    fn claim(&self) -> Result<i64, LifecycleError> {
        self.claimed
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| LifecycleError::Busy)?;
        match self
            .next
            .try_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < i64::MAX as u64).then(|| n + 1)
            }) {
            Ok(id) => Ok(id as i64),
            Err(_) => {
                self.release();
                Err(LifecycleError::Exhausted)
            }
        }
    }
    pub fn release(&self) {
        self.claimed.store(false, Ordering::Release);
    }
}
pub struct Slot<T> {
    owner: Option<(i64, T)>,
    closed: i64,
}
impl<T> Default for Slot<T> {
    fn default() -> Self {
        Self {
            owner: None,
            closed: 0,
        }
    }
}
impl<T> Slot<T> {
    pub fn install(&mut self, id: i64, owner: T) -> Result<(), LifecycleError> {
        if id <= 0 || self.owner.is_some() {
            return Err(LifecycleError::Busy);
        }
        self.owner = Some((id, owner));
        Ok(())
    }
    pub fn get(&mut self, id: i64) -> Result<&mut T, LifecycleError> {
        self.owner
            .as_mut()
            .filter(|(current, _)| *current == id)
            .map(|(_, owner)| owner)
            .ok_or(LifecycleError::Stale)
    }
    /// Only the last retired identity is idempotently closeable. A stale close
    /// must never destroy a new owner.
    pub fn close(&mut self, id: i64) -> Result<bool, LifecycleError> {
        if id > 0 && self.owner.is_none() && id == self.closed {
            return Ok(false);
        }
        self.get(id)?;
        self.owner.take();
        self.closed = id;
        Ok(true)
    }
    pub fn retire(&mut self) -> bool {
        if let Some((id, _)) = self.owner.take() {
            self.closed = id;
            true
        } else {
            false
        }
    }
}
#[derive(Default)]
pub struct TouchSlots([Option<(i32, u64)>; MAX_TOUCHES]);
impl TouchSlots {
    pub fn reset(&mut self) {
        self.0.fill(None);
    }
    pub fn start(&mut self, id: i32, epoch: u64) -> Result<u32, LifecycleError> {
        if !(0..32).contains(&id) || epoch == 0 || self.0.iter().flatten().any(|(v, _)| *v == id) {
            return Err(LifecycleError::Touch);
        }
        let slot = self
            .0
            .iter()
            .position(Option::is_none)
            .ok_or(LifecycleError::Touch)?;
        self.0[slot] = Some((id, epoch));
        Ok(slot as u32)
    }
    pub fn get(&self, id: i32, epoch: u64) -> Result<u32, LifecycleError> {
        self.0
            .iter()
            .position(|s| *s == Some((id, epoch)))
            .map(|v| v as u32)
            .ok_or(LifecycleError::Stale)
    }
    pub fn end(&mut self, slot: u32) {
        self.0[slot as usize] = None;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn generation_busy_stale_close_and_idempotent_retirement() {
        let gate = Gate::new();
        let mut slot = Slot::default();
        let a = gate.claim().unwrap();
        slot.install(a, 10).unwrap();
        assert_eq!(gate.claim(), Err(LifecycleError::Busy));
        assert_eq!(*slot.get(a).unwrap(), 10);
        assert!(slot.close(a).unwrap());
        gate.release();
        assert!(!slot.close(a).unwrap());
        let b = gate.claim().unwrap();
        slot.install(b, 20).unwrap();
        assert_ne!(a, b);
        assert_eq!(slot.close(a), Err(LifecycleError::Stale));
        assert_eq!(*slot.get(b).unwrap(), 20);
        assert!(slot.retire());
        assert!(!slot.retire());
        gate.release();
    }
    #[test]
    fn failed_construction_and_unwind_release_claim_before_publication() {
        let gate = Gate::new();
        let first = {
            let claim = gate.reserve().unwrap();
            claim.id()
        };
        let _ = std::panic::catch_unwind(|| {
            let _claim = gate.reserve().unwrap();
            panic!("fixture");
        });
        let claim = gate.reserve().unwrap();
        assert!(claim.id() > first);
        claim.publish();
        assert!(matches!(gate.reserve(), Err(LifecycleError::Busy)));
        gate.release();
    }
    #[test]
    fn reentrant_access_refuses_without_dropping_current_owner() {
        use std::{
            cell::{Cell, RefCell},
            rc::Rc,
        };
        struct Tracked(Rc<Cell<u32>>);
        impl Drop for Tracked {
            fn drop(&mut self) {
                self.0.set(self.0.get() + 1);
            }
        }
        let drops = Rc::new(Cell::new(0));
        let slot = RefCell::new(Slot::default());
        slot.borrow_mut()
            .install(1, Tracked(Rc::clone(&drops)))
            .unwrap();
        let mut held = slot.borrow_mut();
        assert!(slot.try_borrow_mut().is_err());
        assert!(held.get(2).is_err());
        assert_eq!(drops.get(), 0);
        held.close(1).unwrap();
        assert_eq!(drops.get(), 1);
        assert!(!held.close(1).unwrap());
        assert_eq!(drops.get(), 1);
    }
    #[test]
    fn identity_exhaustion_never_wraps_or_keeps_claim() {
        let gate = Gate::new();
        gate.next.store(i64::MAX as u64, Ordering::Relaxed);
        assert_eq!(gate.claim(), Err(LifecycleError::Exhausted));
        assert_eq!(gate.claim(), Err(LifecycleError::Exhausted));
        assert!(!gate.claimed.load(Ordering::Acquire));
    }
    #[test]
    fn touch_slots_are_bounded_and_old_up_never_retargets_reused_slot() {
        let mut slots = TouchSlots::default();
        let old = slots.start(31, 2).unwrap();
        assert_eq!(slots.start(31, 2), Err(LifecycleError::Touch));
        slots.reset();
        let new = slots.start(31, 3).unwrap();
        assert_eq!(old, new);
        assert_eq!(slots.get(31, 2), Err(LifecycleError::Stale));
        assert_eq!(slots.get(31, 3).unwrap(), new);
        slots.end(new);
        for i in 0..MAX_TOUCHES {
            slots.start(i as i32, 4).unwrap();
        }
        assert_eq!(slots.start(30, 4), Err(LifecycleError::Touch));
        assert_eq!(slots.start(-1, 4), Err(LifecycleError::Touch));
        assert_eq!(slots.start(32, 4), Err(LifecycleError::Touch));
        assert_eq!(slots.start(30, 0), Err(LifecycleError::Touch));
    }
}
