use super::Tick;
use crate::{Error, Result, raw};
use incant_wasm_abi::persistent::MAX_BYTES;

impl Tick<'_> {
    /// Copy committed state (including earlier writes in this tick) into scratch.
    /// No persistent declaration means no state authority, including empty reads.
    pub fn read_persistent(&mut self, offset: u32, output: &mut [u8]) -> Result<()> {
        self.ready()?;
        self.check(if output.len() <= MAX_BYTES && offset <= MAX_BYTES as u32 {
            Ok(())
        } else {
            Err(Error::Bounds)
        })?;
        // SAFETY: live caller-owned output remains exclusively borrowed for the copy.
        let status = unsafe {
            raw::incant_state_v1_read(
                self.epoch,
                output.as_mut_ptr() as usize as i32,
                offset as i32,
                output.len() as i32,
            )
        };
        self.check(if status == 0 {
            Ok(())
        } else {
            Err(Error::Failed)
        })
    }
    /// Stage copied bytes. An ignored invalid current-epoch call poisons the
    /// transaction; no bytes publish until numeric/structural acceptance.
    pub fn write_persistent(&mut self, offset: u32, bytes: &[u8]) -> Result<()> {
        self.ready()?;
        self.check(if bytes.len() <= MAX_BYTES && offset <= MAX_BYTES as u32 {
            Ok(())
        } else {
            Err(Error::Bounds)
        })?;
        // SAFETY: the initialized source stays live throughout the synchronous copy.
        let status = unsafe {
            raw::incant_state_v1_write(
                self.epoch,
                bytes.as_ptr() as usize as i32,
                offset as i32,
                bytes.len() as i32,
            )
        };
        self.check(if status == 0 {
            Ok(())
        } else {
            Err(Error::Failed)
        })
    }
}
