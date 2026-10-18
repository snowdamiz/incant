//! Copied current device generations and queued output, never actuator handles.
use crate::{Error, Result};
use incant_wasm_abi as abi;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HapticTarget {
    Gamepad { id: u32, generation: u32 },
    Device { generation: u32 },
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HapticEffect {
    Rumble {
        strong: f64,
        weak: f64,
        duration_ms: u32,
    },
    Impact {
        intensity: f64,
    },
}
impl HapticTarget {
    pub(crate) fn wire(self) -> abi::HapticTarget {
        match self {
            Self::Gamepad { id, generation } => abi::HapticTarget {
                kind: 0,
                id,
                generation,
            },
            Self::Device { generation } => abi::HapticTarget {
                kind: 1,
                id: 0,
                generation,
            },
        }
    }
}
/// Validated ordered targets borrowed from caller-owned scratch. Capabilities
/// expire when the trusted device owner changes generation or clears authority.
#[derive(Clone, Copy)]
pub struct HapticTargets<'a>(abi::haptics::Targets<'a>);
impl<'a> HapticTargets<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self> {
        abi::haptics::Targets::parse(bytes)
            .map(Self)
            .map_err(|_| Error::Malformed)
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn iter(&self) -> impl ExactSizeIterator<Item = HapticTarget> + '_ {
        self.0.iter().map(|r| {
            if r.kind == 0 {
                HapticTarget::Gamepad {
                    id: r.id,
                    generation: r.generation,
                }
            } else {
                HapticTarget::Device {
                    generation: r.generation,
                }
            }
        })
    }
}
