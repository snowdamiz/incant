//! Recorded gamepad IDs are virtual input, never authority over physical motors.
use super::{Error, gamepad::Gamepads, input::PendingInput};
use incant_input::haptics::{HapticCapabilities, HapticRequests};

pub struct Devices(Option<Gamepads>);
impl Devices {
    pub fn new(live: bool) -> Result<Self, Error> {
        Ok(Self(if live { Some(Gamepads::new()?) } else { None }))
    }
    pub fn poll(&mut self, input: &mut PendingInput, accepting: bool) -> Result<(), Error> {
        self.0
            .as_mut()
            .map_or(Ok(()), |pads| pads.poll(input, accepting))
    }
    pub fn synchronize(&mut self, input: &mut PendingInput) -> Result<(), Error> {
        self.0
            .as_mut()
            .map_or(Ok(()), |pads| pads.synchronize(input))
    }
    pub fn haptic_capabilities(&mut self) -> HapticCapabilities {
        self.0
            .as_mut()
            .map_or_else(HapticCapabilities::default, Gamepads::haptic_capabilities)
    }
    pub fn haptics(&mut self, output: &HapticRequests) -> Result<(), Error> {
        match &mut self.0 {
            Some(pads) => pads.haptics(output),
            None if output.is_empty() => Ok(()),
            None => Err(Error::HapticBackend),
        }
    }
    pub fn pause_haptics(&mut self) -> Result<(), Error> {
        self.0.as_mut().map_or(Ok(()), Gamepads::pause_haptics)
    }
    pub fn discard_pending(&mut self) {
        if let Some(pads) = &mut self.0 {
            pads.discard_pending();
        }
    }
    pub fn commit(&mut self) {
        if let Some(pads) = &mut self.0 {
            pads.commit();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recording_needs_no_hardware_and_advertises_no_motor_targets() {
        let mut devices = Devices::new(false).unwrap();
        let mut pending = PendingInput::new();
        devices.poll(&mut pending, true).unwrap();
        devices.synchronize(&mut pending).unwrap();
        assert!(pending.events().is_empty());
        assert!(devices.haptic_capabilities().is_empty());
        devices.haptics(&HapticRequests::default()).unwrap();
        let capabilities = HapticCapabilities::default().with_gamepad(0, 1).unwrap();
        let mut request = HapticRequests::default();
        request
            .push(
                incant_input::haptics::HapticRequest {
                    target: incant_input::haptics::HapticTarget::Gamepad {
                        id: 0,
                        generation: 1,
                    },
                    effect: incant_input::haptics::HapticEffect::Stop,
                },
                &capabilities,
            )
            .unwrap();
        assert!(devices.haptics(&request).is_err());
        devices.pause_haptics().unwrap();
        devices.discard_pending();
        devices.commit();
    }
}
