use super::Tick;
use crate::{EntityId, Error, HudControl, HudSnapshot, Result, raw};
use incant_wasm_abi as abi;
impl Tick<'_> {
    /// Declarations are captured before commands, while UI notifications are
    /// already routed. Repeated reads return the same bytes during this callback.
    /// Input-v1 separately exposes gameplay controls AFTER widget consumption.
    pub fn hud<'a>(&mut self, scratch: &'a mut [u8]) -> Result<HudSnapshot<'a>> {
        self.ready()?;
        // SAFETY: synchronous scalar import with this callback's epoch.
        let len = unsafe { raw::incant_hud_v1_snapshot_len(self.epoch) };
        let len = self.snapshot_length(len, scratch.len(), abi::HUD_MAX_BYTES)?;
        // SAFETY: exclusive initialized scratch remains live during the copy.
        let actual = unsafe {
            raw::incant_hud_v1_read_snapshot(
                self.epoch,
                scratch.as_mut_ptr() as usize as i32,
                len as i32,
            )
        };
        self.check(if actual == len as i32 {
            Ok(())
        } else {
            Err(Error::Failed)
        })?;
        self.check(HudSnapshot::parse(&scratch[..len]))
    }
    /// Stage an existing owner's literal text/disabled control. Scratch requires
    /// HudCommand::SIZE plus the string bytes (at most HUD_MAX_TEXT_BYTES).
    /// All HUD commands share 256-call/1MiB text limits with native structural
    /// commands. Fonts, hierarchy, role and layout are unchanged. Final role and
    /// lifetime validation is ordered with the rest of this callback's commands.
    pub fn control_hud(
        &mut self,
        entity: EntityId,
        control: HudControl<'_>,
        scratch: &mut [u8],
    ) -> Result<()> {
        self.ready()?;
        let len = self.check(
            abi::hud::encode_command(entity.0, control, scratch).map_err(|_| Error::Capacity),
        )?;
        // SAFETY: complete initialized bounded command copied synchronously.
        let result = unsafe {
            raw::incant_hud_v1_control(self.epoch, scratch.as_ptr() as usize as i32, len as i32)
        };
        self.check(if result == 0 {
            Ok(())
        } else {
            Err(Error::Failed)
        })
    }
}
