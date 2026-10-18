use super::Tick;
use crate::{Error, Result, raw};
use incant_wasm_abi as abi;

impl Tick<'_> {
    /// Initial authored owners/props, captured before queued commands. No owner
    /// can reattach by reusing its stable ID. Returned views borrow only scratch.
    pub fn instances<'a>(&mut self, scratch: &'a mut [u8]) -> Result<abi::instances::Snapshot<'a>> {
        self.ready()?;
        // SAFETY: only scalar epoch crosses; no guest pointer is retained.
        let len = unsafe { raw::incant_instances_v1_snapshot_len(self.epoch) };
        let len = self.snapshot_length(len, scratch.len(), abi::INSTANCE_MAX_BYTES)?;
        // SAFETY: initialized exclusive scratch prefix with checked full capacity.
        let copied = unsafe {
            raw::incant_instances_v1_read_snapshot(
                self.epoch,
                scratch.as_mut_ptr() as usize as i32,
                len as i32,
            )
        };
        self.check(if copied == len as i32 {
            Ok(())
        } else {
            Err(Error::Malformed)
        })?;
        self.check(abi::instances::Snapshot::parse(&scratch[..len]).map_err(|_| Error::Malformed))
    }
}
