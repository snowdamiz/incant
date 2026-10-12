//! Native session IDs are leases for one opaque accepted text owner. They are
//! never reassigned at event delivery, including after blur or widget replacement.
use super::Error;
use incant_ui::{FocusToken, UiRuntime};
#[derive(Default)]
pub(super) struct Ime {
    serial: u64,
    owner: Option<FocusToken>,
    stale: u64,
}
impl Ime {
    pub fn synchronize(
        &mut self,
        ui: Option<&UiRuntime>,
        active: bool,
    ) -> Result<Option<(u64, bool)>, Error> {
        let owner = ui.filter(|_| active).and_then(UiRuntime::text_focus);
        if self.serial != 0 && self.owner == owner {
            return Ok(None);
        }
        let serial = self.next_serial()?;
        self.serial = serial;
        self.owner = owner;
        Ok(Some((serial, self.owner.is_some())))
    }
    fn next_serial(&self) -> Result<u64, Error> {
        self.serial.checked_add(1).ok_or(Error::ImeSession)
    }
    pub fn target(&mut self, ui: Option<&UiRuntime>, serial: u64) -> Option<FocusToken> {
        if serial == self.serial
            && self.owner.is_some()
            && self.owner == ui.and_then(UiRuntime::text_focus)
        {
            self.owner.clone()
        } else {
            self.stale = self.stale.saturating_add(1);
            None
        }
    }
    pub fn managed(&self) -> bool {
        self.serial != 0
    }
    pub fn stale(&self) -> u64 {
        self.stale
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_generation_exhaustion_refuses_instead_of_reusing_a_lease() {
        let mut ime = Ime {
            serial: u64::MAX,
            owner: None,
            stale: 0,
        };
        // Identical disabled state remains idempotent; a new owner would require
        // a checked increment. Exercise the exact arithmetic admission path.
        assert_eq!(ime.synchronize(None, false).unwrap(), None);
        assert!(matches!(ime.next_serial(), Err(Error::ImeSession)));
        assert_eq!(ime.serial, u64::MAX);
        ime.serial = 0;
        assert_eq!(ime.synchronize(None, false).unwrap(), Some((1, false)));
        assert_eq!(ime.synchronize(None, false).unwrap(), None);
    }
}
