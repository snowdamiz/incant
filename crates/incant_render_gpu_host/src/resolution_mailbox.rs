//! Single pending callback/result slot. Late callbacks keep only their old slot.
use incant_render::resolution::ResolutionError;
use std::sync::{Arc, Mutex};
enum State<T> {
    Idle,
    Pending,
    Ready(T),
}
pub(crate) struct Mailbox<T>(Arc<Mutex<State<T>>>);
pub(crate) struct Ticket<T> {
    slot: Arc<Mutex<State<T>>>,
    armed: bool,
}
impl<T> Default for Mailbox<T> {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(State::Idle)))
    }
}
impl<T> Mailbox<T> {
    pub fn reserve(&self) -> Result<Option<Ticket<T>>, ResolutionError> {
        let mut slot = self.0.lock().map_err(|_| ResolutionError::Mailbox)?;
        if !matches!(*slot, State::Idle) {
            return Ok(None);
        }
        *slot = State::Pending;
        Ok(Some(Ticket {
            slot: Arc::clone(&self.0),
            armed: true,
        }))
    }
    pub fn take(&self) -> Result<Option<T>, ResolutionError> {
        let mut slot = self.0.lock().map_err(|_| ResolutionError::Mailbox)?;
        if !matches!(*slot, State::Ready(_)) {
            return Ok(None);
        }
        let State::Ready(value) = std::mem::replace(&mut *slot, State::Idle) else {
            unreachable!()
        };
        Ok(Some(value))
    }
}
impl<T> Ticket<T> {
    pub fn complete(mut self, value: T) {
        self.publish(value);
    }
    fn publish(&mut self, value: T) {
        // Disarm before releasing Ready: the owner may consume it and reserve
        // another callback before this completed ticket's destructor runs.
        self.armed = false;
        if let Ok(mut slot) = self.slot.lock() {
            *slot = State::Ready(value);
        }
    }
}
impl<T> Drop for Ticket<T> {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        if let Ok(mut slot) = self.slot.lock()
            && matches!(*slot, State::Pending)
        {
            *slot = State::Idle;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cross_thread_completion_is_single_flight_and_consumed_once() {
        let slot = Mailbox::default();
        let ticket = slot.reserve().unwrap().unwrap();
        assert!(slot.reserve().unwrap().is_none());
        assert!(slot.take().unwrap().is_none());
        std::thread::spawn(move || ticket.complete(17u64))
            .join()
            .unwrap();
        assert!(slot.reserve().unwrap().is_none());
        assert_eq!(slot.take().unwrap(), Some(17));
        assert_eq!(slot.take().unwrap(), None);
        assert!(slot.reserve().unwrap().is_some());
    }
    #[test]
    fn completed_ticket_destruction_cannot_cancel_a_new_callback() {
        let slot = Mailbox::default();
        let mut old = slot.reserve().unwrap().unwrap();
        old.publish(1u64);
        assert_eq!(slot.take().unwrap(), Some(1));
        let new = slot.reserve().unwrap().unwrap();
        drop(old);
        assert!(slot.reserve().unwrap().is_none());
        new.complete(2);
        assert_eq!(slot.take().unwrap(), Some(2));
    }
    #[test]
    fn dropped_callback_and_retired_owner_do_not_fill_replacement_slot() {
        let old = Mailbox::<u64>::default();
        drop(old.reserve().unwrap().unwrap());
        let late = old.reserve().unwrap().unwrap();
        let replacement = Mailbox::<u64>::default();
        drop(old);
        late.complete(19);
        assert_eq!(replacement.take().unwrap(), None);
        let current = replacement.reserve().unwrap().unwrap();
        current.complete(20);
        assert_eq!(replacement.take().unwrap(), Some(20));
    }
}
