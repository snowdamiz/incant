//! Fixed cold diagnostic selection. No clock, queue, allocation or frame pacing loop.
use core::sync::atomic::{AtomicU8, Ordering};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Policy {
    #[default]
    Ordinary,
    MinimumDuration60,
}
impl Policy {
    pub fn seconds(self) -> Option<f64> {
        match self {
            Self::Ordinary => None,
            Self::MinimumDuration60 => Some(1.0 / 60.0),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Unsupported,
    NotCold,
    NotConfigured,
    Transaction,
    NotFifo,
}
impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::Unsupported => "Metal minimum duration requires macOS 10.15.4 or newer",
            Self::NotCold => {
                "Metal minimum duration selection requires a fresh unconfigured surface"
            }
            Self::NotConfigured => "Metal minimum duration surface is not configured",
            Self::Transaction => "Metal minimum duration refuses transaction presentation",
            Self::NotFifo => "Metal minimum duration requires FIFO presentation",
        })
    }
}
impl core::error::Error for Error {}

/// 0/1 = cold ordinary/selected; 2/3 = sealed ordinary/selected. Selection cannot
/// change after configure, including unconfigure/reconfigure. A fresh surface
/// needs its own explicit admission. Acquired frames retain an independent copy.
#[derive(Debug, Default)]
pub(super) struct State(AtomicU8);
impl State {
    pub(super) fn select(&self, supported: bool, transaction: bool) -> Result<(), Error> {
        if !supported {
            return Err(Error::Unsupported);
        }
        if transaction {
            return Err(Error::Transaction);
        }
        self.0
            .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
            .map(|_| ())
            .map_err(|_| Error::NotCold)
    }
    pub(super) fn configure(&self, fifo: bool, transaction: bool) -> Result<(), Error> {
        loop {
            let value = self.0.load(Ordering::Acquire);
            if value & 1 != 0 {
                if transaction {
                    return Err(Error::Transaction);
                }
                if !fifo {
                    return Err(Error::NotFifo);
                }
            }
            if value >= 2
                || self
                    .0
                    .compare_exchange(value, value | 2, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok()
            {
                return Ok(());
            }
            // A concurrent cold selection requires revalidating that policy
            // against this configuration before it can be sealed.
        }
    }
    pub(super) fn snapshot(&self, transaction: bool) -> Result<Policy, Error> {
        match self.0.load(Ordering::Acquire) {
            2 => Ok(Policy::Ordinary),
            3 if !transaction => Ok(Policy::MinimumDuration60),
            3 => Err(Error::Transaction),
            _ => Err(Error::NotConfigured),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ordinary_snapshot_never_selects_a_duration() {
        let state = State::default();
        assert_eq!(state.snapshot(false), Err(Error::NotConfigured));
        state.configure(true, true).unwrap();
        assert_eq!(state.snapshot(true).unwrap().seconds(), None);
        assert_eq!(state.select(true, false), Err(Error::NotCold));
    }
    #[test]
    fn selected_frame_snapshot_survives_reconfigure_without_live_mutation() {
        let state = State::default();
        state.select(true, false).unwrap();
        state.configure(true, false).unwrap();
        let held = state.snapshot(false).unwrap();
        state.configure(true, false).unwrap();
        assert_eq!(held, Policy::MinimumDuration60);
        assert_eq!(held.seconds(), Some(1.0 / 60.0));
        assert_eq!(state.select(true, false), Err(Error::NotCold));
        assert_eq!(state.snapshot(false).unwrap(), held);
    }
    #[test]
    fn unsupported_or_transaction_refusal_does_not_select_or_seal() {
        let state = State::default();
        assert_eq!(state.select(false, false), Err(Error::Unsupported));
        assert_eq!(state.select(true, true), Err(Error::Transaction));
        state.select(true, false).unwrap();
        assert_eq!(state.configure(false, false), Err(Error::NotFifo));
        assert_eq!(state.configure(true, true), Err(Error::Transaction));
        assert_eq!(state.snapshot(false), Err(Error::NotConfigured));
        state.configure(true, false).unwrap();
        assert_eq!(state.snapshot(true), Err(Error::Transaction));
    }
    #[test]
    fn recreated_surface_requires_new_explicit_cold_selection() {
        let old = State::default();
        old.select(true, false).unwrap();
        old.configure(true, false).unwrap();
        let fresh = State::default();
        fresh.configure(true, false).unwrap();
        assert_eq!(old.snapshot(false).unwrap(), Policy::MinimumDuration60);
        assert_eq!(fresh.snapshot(false).unwrap(), Policy::Ordinary);
    }
}
