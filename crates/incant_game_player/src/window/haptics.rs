//! Bounded desired-state adapter. Gilrs accepts asynchronous motor requests;
//! neither a successful call nor a capability asserts physical delivery.
use super::Error;
use incant_input::{
    MAX_GAMEPADS,
    haptics::{HapticCapabilities, HapticEffect, HapticRequests},
};

pub(super) trait Device {
    fn rumble(&self, strong: f32, weak: f32, duration_ms: u32) -> Result<(), Error>;
    fn stop(&self) -> Result<(), Error>;
}
struct Slot<D> {
    generation: u32,
    connected: bool,
    supported: bool,
    attempted: bool,
    device: Option<D>,
}
#[derive(Clone, Copy)]
enum PauseFailure {
    Backend,
    Generation,
}
impl PauseFailure {
    fn error(self) -> Error {
        match self {
            Self::Backend => Error::HapticBackend,
            Self::Generation => Error::HapticGeneration,
        }
    }
}
pub(super) struct Haptics<D: Device> {
    slots: [Slot<D>; MAX_GAMEPADS],
    paused: bool,
    pause_failure: Option<PauseFailure>,
}
impl<D: Device> Default for Haptics<D> {
    fn default() -> Self {
        Self {
            slots: std::array::from_fn(|_| Slot {
                generation: 0,
                connected: false,
                supported: false,
                attempted: false,
                device: None,
            }),
            paused: true,
            pause_failure: None,
        }
    }
}
impl<D: Device> Haptics<D> {
    pub fn connect(
        &mut self,
        id: u32,
        supported: bool,
        create: impl FnOnce() -> Option<D>,
    ) -> Result<(), Error> {
        let slot = self
            .slots
            .get_mut(id as usize)
            .ok_or(Error::HapticBackend)?;
        if slot.connected {
            return Ok(());
        }
        slot.generation = slot
            .generation
            .checked_add(1)
            .ok_or(Error::HapticGeneration)?;
        slot.connected = true;
        if supported && !slot.attempted {
            slot.attempted = true;
            slot.device = create(); // unavailable FF must not disable ordinary input
        }
        slot.supported = supported && slot.device.is_some();
        Ok(())
    }
    pub fn disconnect(&mut self, id: u32) -> Result<(), Error> {
        let slot = self
            .slots
            .get_mut(id as usize)
            .ok_or(Error::HapticBackend)?;
        if !slot.connected {
            return Ok(());
        }
        slot.connected = false;
        slot.supported = false;
        if let Some(device) = &slot.device {
            device.stop()?;
        }
        Ok(())
    }
    pub fn resume(&mut self) -> HapticCapabilities {
        if self.pause_failure.is_some() {
            return HapticCapabilities::default();
        }
        self.paused = false;
        self.capabilities()
    }
    fn capabilities(&self) -> HapticCapabilities {
        let mut result = HapticCapabilities::default();
        if !self.paused {
            for (id, slot) in self.slots.iter().enumerate() {
                if slot.connected && slot.supported {
                    result = result
                        .with_gamepad(id as u32, slot.generation)
                        .expect("bounded ID and nonzero checked generation");
                }
            }
        }
        result
    }
    pub fn pause(&mut self) -> Result<(), Error> {
        if self.paused {
            return self
                .pause_failure
                .map(PauseFailure::error)
                .map_or(Ok(()), Err);
        }
        self.paused = true;
        if self.stop_all().is_err() {
            self.pause_failure = Some(PauseFailure::Backend);
        }
        for slot in &mut self.slots {
            if slot.connected && slot.supported {
                if let Some(next) = slot.generation.checked_add(1) {
                    slot.generation = next;
                } else {
                    slot.supported = false;
                    self.pause_failure.get_or_insert(PauseFailure::Generation);
                }
            }
        }
        self.pause_failure
            .map(PauseFailure::error)
            .map_or(Ok(()), Err)
    }
    fn stop_all(&self) -> Result<(), Error> {
        let mut result = Ok(());
        for slot in &self.slots {
            if let Some(device) = &slot.device
                && let Err(error) = device.stop()
            {
                result = Err(error);
            }
        }
        result
    }
    pub fn dispatch(&mut self, requests: &HapticRequests) -> Result<(), Error> {
        if let Some(failure) = self.pause_failure {
            return Err(failure.error());
        }
        let capabilities = self.capabilities();
        let mut desired = [None; MAX_GAMEPADS];
        for request in requests.iter() {
            if let Err(error) = capabilities.validate(request) {
                let _ = self.pause();
                return Err(Error::HapticRequest(error));
            }
            let incant_input::haptics::HapticTarget::Gamepad { id, .. } = request.target else {
                return Err(Error::HapticBackend);
            };
            desired[id as usize] = Some(request.effect);
        }
        // Validate the whole batch first, then coalesce to the last desired
        // state per pad. This bounds driver calls even for repeated commands.
        let result = (|| {
            for (slot, effect) in self.slots.iter().zip(desired) {
                if let Some(effect) = effect {
                    let device = slot.device.as_ref().ok_or(Error::HapticBackend)?;
                    match effect {
                        HapticEffect::Rumble {
                            strong,
                            weak,
                            duration_ms,
                        } => device.rumble(strong, weak, duration_ms)?,
                        HapticEffect::Stop => device.stop()?,
                        HapticEffect::Impact { .. } => return Err(Error::HapticBackend),
                    }
                }
            }
            Ok(())
        })();
        if result.is_err() {
            let _ = self.pause();
        }
        result
    }
}
impl<D: Device> Drop for Haptics<D> {
    fn drop(&mut self) {
        let _ = self.stop_all();
    }
}

pub(super) struct Motors {
    strong: gilrs::ff::Effect,
    weak: gilrs::ff::Effect,
}
impl Motors {
    pub fn new(id: gilrs::GamepadId, backend: &mut gilrs::Gilrs) -> Option<Self> {
        use gilrs::ff::{BaseEffect, BaseEffectType, EffectBuilder, Repeat, Replay, Ticks};
        let mut create = |kind| {
            EffectBuilder::new()
                .add_effect(BaseEffect {
                    kind,
                    scheduling: Replay {
                        play_for: Ticks::from_ms(5_000),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .gamepads(&[id])
                .repeat(Repeat::For(Ticks::from_ms(5_000)))
                .finish(backend)
                .ok()
        };
        Some(Self {
            strong: create(BaseEffectType::Strong {
                magnitude: u16::MAX,
            })?,
            weak: create(BaseEffectType::Weak {
                magnitude: u16::MAX,
            })?,
        })
    }
}
impl Device for Motors {
    fn rumble(&self, strong: f32, weak: f32, duration_ms: u32) -> Result<(), Error> {
        use gilrs::ff::{Repeat, Ticks};
        self.stop()?;
        for (effect, gain) in [(&self.strong, strong), (&self.weak, weak)] {
            effect.set_gain(gain).map_err(|_| Error::HapticBackend)?;
            effect
                .set_repeat(Repeat::For(Ticks::from_ms(duration_ms)))
                .map_err(|_| Error::HapticBackend)?;
            effect.play().map_err(|_| Error::HapticBackend)?;
        }
        Ok(())
    }
    fn stop(&self) -> Result<(), Error> {
        let strong = self.strong.stop();
        let weak = self.weak.stop();
        strong.and(weak).map_err(|_| Error::HapticBackend)
    }
}
#[cfg(test)]
mod tests;
