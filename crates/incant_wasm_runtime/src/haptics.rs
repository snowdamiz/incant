//! Fixed transient queues shared by both Wasm hosts. No actuation or device IO.
use crate::BoundaryError as Error;
use incant_input::haptics::{
    HapticCapabilities, HapticEffect, HapticError, HapticRequest, HapticRequests, HapticTarget,
};
use incant_wasm_abi as abi;

const _: () = {
    assert!(abi::HAPTIC_MAX_TARGETS == incant_input::MAX_GAMEPADS + 1);
    assert!(abi::HAPTIC_MAX_REQUESTS == incant_input::haptics::MAX_HAPTIC_REQUESTS);
    assert!(abi::HAPTIC_MAX_DURATION_MS == incant_input::haptics::MAX_HAPTIC_DURATION_MS as usize);
};
#[derive(Default)]
pub(crate) struct State {
    capabilities: HapticCapabilities,
    staged: HapticRequests,
    committed: HapticRequests,
    pub rejected: bool,
}
impl State {
    pub fn ready(&self) -> Result<(), Error> {
        if self.committed.is_empty() {
            Ok(())
        } else {
            Err(HapticError::Pending.into())
        }
    }
    pub fn set(&mut self, value: HapticCapabilities) -> Result<(), Error> {
        self.ready()?;
        self.capabilities = value;
        Ok(())
    }
    pub fn take(&mut self) -> HapticRequests {
        std::mem::take(&mut self.committed)
    }
    pub fn clear(&mut self) {
        *self = Self::default();
    }
    pub fn begin(&mut self) {
        self.staged = Default::default();
        self.rejected = false;
    }
    pub fn abort(&mut self) {
        self.staged = Default::default();
    }
    pub fn commit(&mut self) {
        self.committed = std::mem::take(&mut self.staged);
    }
    /// Even a direct trusted replacement cannot silently consume old output.
    /// Public hosts refuse reload while it is pending, then replace neutral.
    pub fn replacement(&mut self) -> Self {
        Self {
            committed: self.take(),
            ..Self::default()
        }
    }
    pub fn len(&self) -> usize {
        self.staged.len()
    }
    pub fn snapshot_len(&self) -> usize {
        abi::HapticHeader::SIZE + self.capabilities.targets().count() * abi::HapticTarget::SIZE
    }
    pub fn snapshot(&self, output: &mut [u8]) {
        output[..abi::HapticHeader::SIZE].copy_from_slice(
            &abi::HapticHeader {
                magic: *abi::HAPTIC_MAGIC,
                count: self.capabilities.targets().count() as u32,
                stride: abi::HapticTarget::SIZE as u32,
            }
            .encode(),
        );
        for (target, to) in self.capabilities.targets().zip(
            output[abi::HapticHeader::SIZE..]
                .as_chunks_mut::<{ abi::HapticTarget::SIZE }>()
                .0,
        ) {
            let row = match target {
                HapticTarget::Gamepad { id, generation } => abi::HapticTarget {
                    kind: 0,
                    id,
                    generation,
                },
                HapticTarget::Device { generation } => abi::HapticTarget {
                    kind: 1,
                    id: 0,
                    generation,
                },
            };
            to.copy_from_slice(&row.encode());
        }
    }
    pub fn queue(
        &mut self,
        commands: abi::haptics::Commands<'_>,
        available: usize,
    ) -> Result<(), Error> {
        if commands.iter().len() > available {
            return Err(Error::Limit("structural command quota"));
        }
        if commands.iter().len() > abi::HAPTIC_MAX_REQUESTS - self.staged.len() {
            return Err(HapticError::Capacity.into());
        }
        // Whole-frame wire validation has already completed. Validate every
        // generation against current authority before changing even staging.
        for row in commands.iter() {
            self.capabilities.validate(request(row))?;
        }
        for row in commands.iter() {
            self.staged
                .push(request(row), &self.capabilities)
                .expect("prevalidated bounded haptic batch");
        }
        Ok(())
    }
}
fn request(row: abi::HapticCommand) -> HapticRequest {
    let target = if row.target_kind == 0 {
        HapticTarget::Gamepad {
            id: row.id,
            generation: row.generation,
        }
    } else {
        HapticTarget::Device {
            generation: row.generation,
        }
    };
    let effect = match row.effect_kind {
        0 => HapticEffect::Rumble {
            strong: row.strong as f32,
            weak: row.weak as f32,
            duration_ms: row.duration_ms,
        },
        1 => HapticEffect::Impact {
            intensity: row.strong as f32,
        },
        2 => HapticEffect::Stop,
        _ => unreachable!("validated haptic command"),
    };
    HapticRequest { target, effect }
}
