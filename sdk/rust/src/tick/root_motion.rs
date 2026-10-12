use super::Tick;
use crate::{EntityId, Error, Result, RootMotionControl, RootMotionSnapshot, raw};
use incant_wasm_abi as abi;
impl Tick<'_> {
    /// Complete copied pose, root-strip instructions, ordered IK and motion
    /// receipts captured before callback commands. Legacy pose/IK methods refuse
    /// any committed root stripping, including inactive and extract-only roots.
    pub fn root_motion<'a>(&mut self, scratch: &'a mut [u8]) -> Result<RootMotionSnapshot<'a>> {
        self.ready()?;
        // SAFETY: synchronous scalar import under this callback's epoch.
        let len = unsafe { raw::incant_root_motion_v1_snapshot_len(self.epoch) };
        let len = self.check(usize::try_from(len).map_err(|_| Error::Failed))?;
        self.check(
            if len <= abi::ROOT_MOTION_MAX_BYTES && len <= scratch.len() {
                Ok(())
            } else {
                Err(Error::Capacity)
            },
        )?;
        // SAFETY: initialized exclusive output scratch remains live during copy.
        let copied = unsafe {
            raw::incant_root_motion_v1_read_snapshot(
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
        self.check(RootMotionSnapshot::parse(&scratch[..len]))
    }
    /// Atomic existing-owner control. Application toggles never seek/replay
    /// historical motion; reset clears deltas/accumulation without moving clocks.
    pub fn control_root_motion(
        &mut self,
        entity: EntityId,
        control: RootMotionControl,
    ) -> Result<()> {
        self.ready()?;
        let (kind, value) = match control {
            RootMotionControl::SetApplying(value) => (0, u32::from(value)),
            RootMotionControl::ResetAccumulated => (1, 0),
        };
        let bytes = abi::RootMotionCommand {
            entity: entity.0,
            kind,
            value,
        }
        .encode();
        // SAFETY: synchronous copy of the complete bounded initialized command.
        let result = unsafe {
            raw::incant_root_motion_v1_control(
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
