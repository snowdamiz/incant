//! Ephemeral host output. Capabilities are issued by the current device owner,
//! never serialized input, game state, or proof that a motor physically moved.
use crate::MAX_GAMEPADS;
use thiserror::Error;

pub const MAX_HAPTIC_REQUESTS: usize = 32;
pub const MAX_HAPTIC_DURATION_MS: u32 = 5_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HapticTarget {
    Gamepad { id: u32, generation: u32 },
    Device { generation: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HapticEffect {
    Rumble {
        strong: f32,
        weak: f32,
        duration_ms: u32,
    },
    Impact {
        intensity: f32,
    },
    Stop,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HapticRequest {
    pub target: HapticTarget,
    pub effect: HapticEffect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum HapticError {
    #[error("invalid haptic target or zero generation")]
    Target,
    #[error("haptic target is unavailable or its generation expired")]
    Unavailable,
    #[error("haptic effect is unsupported by this target")]
    Effect,
    #[error("haptic amplitude must be finite and between zero and one")]
    Amplitude,
    #[error("haptic duration must be between 1 and 5000 milliseconds")]
    Duration,
    #[error("haptic output exceeds 32 requests per tick")]
    Capacity,
    #[error("previous haptic output has not been consumed")]
    Pending,
}

/// Gamepad rumble and device impact are different capabilities of one output
/// API. A host must advertise only its actual current support. Generation zero
/// is reserved; reconnect and session replacement require fresh generations.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HapticCapabilities {
    gamepads: [Option<u32>; MAX_GAMEPADS],
    device: Option<u32>,
}
impl HapticCapabilities {
    pub fn with_gamepad(mut self, id: u32, generation: u32) -> Result<Self, HapticError> {
        if generation == 0 {
            return Err(HapticError::Target);
        }
        *self
            .gamepads
            .get_mut(id as usize)
            .ok_or(HapticError::Target)? = Some(generation);
        Ok(self)
    }
    pub fn with_device(mut self, generation: u32) -> Result<Self, HapticError> {
        if generation == 0 {
            return Err(HapticError::Target);
        }
        self.device = Some(generation);
        Ok(self)
    }
    pub fn targets(&self) -> impl Iterator<Item = HapticTarget> + '_ {
        self.gamepads
            .iter()
            .enumerate()
            .filter_map(|(id, generation)| {
                generation.map(|generation| HapticTarget::Gamepad {
                    id: id as u32,
                    generation,
                })
            })
            .chain(
                self.device
                    .map(|generation| HapticTarget::Device { generation }),
            )
    }
    pub fn is_empty(&self) -> bool {
        self.device.is_none() && self.gamepads.iter().all(Option::is_none)
    }
    pub fn validate(&self, request: HapticRequest) -> Result<(), HapticError> {
        let (generation, current) = match request.target {
            HapticTarget::Gamepad { id, generation } => (
                generation,
                *self.gamepads.get(id as usize).ok_or(HapticError::Target)?,
            ),
            HapticTarget::Device { generation } => (generation, self.device),
        };
        if generation == 0 {
            return Err(HapticError::Target);
        }
        if current != Some(generation) {
            return Err(HapticError::Unavailable);
        }
        let amplitude = |v: f32| {
            if v.is_finite() && (0. ..=1.).contains(&v) {
                Ok(())
            } else {
                Err(HapticError::Amplitude)
            }
        };
        match (request.target, request.effect) {
            (_, HapticEffect::Stop) => Ok(()),
            (
                HapticTarget::Gamepad { .. },
                HapticEffect::Rumble {
                    strong,
                    weak,
                    duration_ms,
                },
            ) => {
                amplitude(strong)?;
                amplitude(weak)?;
                if !(1..=MAX_HAPTIC_DURATION_MS).contains(&duration_ms) {
                    return Err(HapticError::Duration);
                }
                Ok(())
            }
            (HapticTarget::Device { .. }, HapticEffect::Impact { intensity }) => {
                amplitude(intensity)
            }
            _ => Err(HapticError::Effect),
        }
    }
}

/// Fixed storage prevents a forgotten host drain from growing a queue. A script
/// tick refuses before advancement while an earlier output remains pending.
#[derive(Debug, Clone, PartialEq)]
pub struct HapticRequests {
    requests: [Option<HapticRequest>; MAX_HAPTIC_REQUESTS],
    len: usize,
}
impl Default for HapticRequests {
    fn default() -> Self {
        Self {
            requests: [None; MAX_HAPTIC_REQUESTS],
            len: 0,
        }
    }
}
impl HapticRequests {
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn iter(&self) -> impl Iterator<Item = HapticRequest> + '_ {
        self.requests[..self.len].iter().flatten().copied()
    }
    pub fn push(
        &mut self,
        request: HapticRequest,
        capabilities: &HapticCapabilities,
    ) -> Result<(), HapticError> {
        capabilities.validate(request)?;
        let slot = self
            .requests
            .get_mut(self.len)
            .ok_or(HapticError::Capacity)?;
        *slot = Some(request);
        self.len += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn haptic_validation_is_bounded_and_generation_sensitive() {
        let capabilities = HapticCapabilities::default().with_gamepad(15, 2).unwrap();
        let mut request = HapticRequest {
            target: HapticTarget::Gamepad {
                id: 15,
                generation: 2,
            },
            effect: HapticEffect::Rumble {
                strong: 0.5,
                weak: 1.,
                duration_ms: 5_000,
            },
        };
        let mut queue = HapticRequests::default();
        for _ in 0..32 {
            queue.push(request, &capabilities).unwrap();
        }
        assert_eq!(
            queue.push(request, &capabilities),
            Err(HapticError::Capacity)
        );
        assert_eq!(queue.iter().count(), 32);
        request.target = HapticTarget::Gamepad {
            id: 15,
            generation: 1,
        };
        assert_eq!(
            capabilities.validate(request),
            Err(HapticError::Unavailable)
        );
        assert_eq!(
            HapticCapabilities::default().validate(request),
            Err(HapticError::Unavailable)
        );
        assert!(HapticCapabilities::default().with_gamepad(16, 1).is_err());
        assert!(HapticCapabilities::default().with_device(0).is_err());
    }
    #[test]
    fn haptic_unknown_effect_and_nonfinite_or_out_of_range_parameters_refuse() {
        let capabilities = HapticCapabilities::default()
            .with_gamepad(0, 1)
            .unwrap()
            .with_device(1)
            .unwrap();
        let target = HapticTarget::Gamepad {
            id: 0,
            generation: 1,
        };
        for value in [f32::NAN, f32::INFINITY, -0.1, 1.1] {
            assert_eq!(
                capabilities.validate(HapticRequest {
                    target,
                    effect: HapticEffect::Rumble {
                        strong: value,
                        weak: 0.,
                        duration_ms: 1
                    }
                }),
                Err(HapticError::Amplitude)
            );
        }
        for duration_ms in [0, 5_001, u32::MAX] {
            assert_eq!(
                capabilities.validate(HapticRequest {
                    target,
                    effect: HapticEffect::Rumble {
                        strong: 1.,
                        weak: 0.,
                        duration_ms
                    }
                }),
                Err(HapticError::Duration)
            );
        }
        assert_eq!(
            capabilities.validate(HapticRequest {
                target,
                effect: HapticEffect::Impact { intensity: 1. }
            }),
            Err(HapticError::Effect)
        );
        capabilities
            .validate(HapticRequest {
                target: HapticTarget::Device { generation: 1 },
                effect: HapticEffect::Stop,
            })
            .unwrap();
    }
}
