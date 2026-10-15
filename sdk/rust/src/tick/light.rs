use super::Tick;
use crate::{Error, LightCommand, Result, raw};
use incant_wasm_abi as abi;
impl Tick<'_> {
    /// Ordered controls on existing final punctual lights. Preserves bindings;
    /// invalid/ignored errors poison this tick, including earlier numeric writes.
    /// Environment, ranges, cones, kind and shadow configuration are not controls.
    pub fn light_commands(&mut self, commands: &[LightCommand], scratch: &mut [u8]) -> Result<()> {
        self.ready()?;
        self.check(if commands.len() <= abi::LIGHT_MAX_COMMANDS {
            Ok(())
        } else {
            Err(Error::Capacity)
        })?;
        let len = abi::LightCommandHeader::SIZE + commands.len() * abi::LightCommand::SIZE;
        self.check(if len <= scratch.len() {
            Ok(())
        } else {
            Err(Error::Capacity)
        })?;
        for command in commands {
            self.check(command.wire())?;
        }
        scratch[..abi::LightCommandHeader::SIZE].copy_from_slice(
            &abi::LightCommandHeader {
                magic: *abi::LIGHT_COMMAND_MAGIC,
                count: commands.len() as u32,
                stride: abi::LightCommand::SIZE as u32,
            }
            .encode(),
        );
        for (command, row) in commands.iter().zip(
            scratch[abi::LightCommandHeader::SIZE..len]
                .as_chunks_mut::<{ abi::LightCommand::SIZE }>()
                .0,
        ) {
            row.copy_from_slice(&command.wire().unwrap().encode());
        }
        // SAFETY: synchronous copy of the initialized bounded caller-owned batch.
        let result = unsafe {
            raw::incant_light_v1_command(self.epoch, scratch.as_ptr() as usize as i32, len as i32)
        };
        self.check(if result == 0 {
            Ok(())
        } else {
            Err(Error::Failed)
        })
    }
}
