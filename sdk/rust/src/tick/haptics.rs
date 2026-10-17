use super::Tick;
use crate::{Error, HapticEffect, HapticTarget, HapticTargets, Result, raw};
use incant_wasm_abi as abi;

impl Tick<'_> {
    /// Snapshot only the trusted host's actual current support. Empty is valid;
    /// this never infers rumble availability from ordinary input events.
    pub fn haptic_capabilities<'a>(&mut self, scratch: &'a mut [u8]) -> Result<HapticTargets<'a>> {
        self.ready()?;
        // SAFETY: synchronous scalar import under this callback's epoch.
        let len = unsafe { raw::incant_haptics_v1_snapshot_len(self.epoch) };
        let len = self.snapshot_length(len, scratch.len(), abi::HAPTIC_MAX_BYTES)?;
        // SAFETY: checked initialized scratch stays exclusively borrowed.
        let copied = unsafe {
            raw::incant_haptics_v1_read_snapshot(
                self.epoch,
                scratch.as_mut_ptr() as usize as i32,
                len as i32,
            )
        };
        self.check(if copied == len as i32 {
            Ok(())
        } else {
            Err(Error::Failed)
        })?;
        self.check(HapticTargets::parse(&scratch[..len]))
    }
    /// Queue ephemeral output for delivery only after the whole composed tick
    /// accepts. Success is not evidence that a physical actuator moved.
    pub fn play_haptic(&mut self, target: HapticTarget, effect: HapticEffect) -> Result<()> {
        let (kind, strong, weak, duration_ms) = match effect {
            HapticEffect::Rumble {
                strong,
                weak,
                duration_ms,
            } => (0, strong, weak, duration_ms),
            HapticEffect::Impact { intensity } => (1, intensity, 0., 0),
        };
        self.haptic_command(target, kind, strong, weak, duration_ms)
    }
    pub fn stop_haptic(&mut self, target: HapticTarget) -> Result<()> {
        self.haptic_command(target, 2, 0., 0., 0)
    }
    fn haptic_command(
        &mut self,
        target: HapticTarget,
        effect_kind: u32,
        strong: f64,
        weak: f64,
        duration_ms: u32,
    ) -> Result<()> {
        self.ready()?;
        let target = target.wire();
        let row = abi::HapticCommand {
            target_kind: target.kind,
            id: target.id,
            generation: target.generation,
            effect_kind,
            strong,
            weak,
            duration_ms,
            reserved: 0,
        };
        self.check(abi::haptics::validate_command(&row).map_err(|_| Error::Bounds))?;
        let mut bytes = [0; abi::HapticCommandHeader::SIZE + abi::HapticCommand::SIZE];
        bytes[..abi::HapticCommandHeader::SIZE].copy_from_slice(
            &abi::HapticCommandHeader {
                magic: *abi::HAPTIC_COMMAND_MAGIC,
                count: 1,
                stride: abi::HapticCommand::SIZE as u32,
            }
            .encode(),
        );
        bytes[abi::HapticCommandHeader::SIZE..].copy_from_slice(&row.encode());
        // SAFETY: one synchronous copy of initialized fixed-size stack bytes.
        let result = unsafe {
            raw::incant_haptics_v1_command(
                self.epoch,
                bytes.as_ptr() as usize as i32,
                bytes.len() as i32,
            )
        };
        self.check(if result == 0 {
            Ok(())
        } else {
            Err(Error::Failed)
        })
    }
}
