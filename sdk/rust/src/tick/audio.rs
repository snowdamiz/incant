use super::Tick;
use crate::{AudioCommand, AudioDeclarations, Error, Result, raw};
use incant_wasm_abi as abi;

impl Tick<'_> {
    /// Copy all desired committed audio declarations before this callback's
    /// commands. This is not a query of mixer voice state or playback cursors.
    pub fn audio<'a>(&mut self, scratch: &'a mut [u8]) -> Result<AudioDeclarations<'a>> {
        self.ready()?;
        // SAFETY: synchronous scalar import under this callback's epoch.
        let len = unsafe { raw::incant_audio_v1_snapshot_len(self.epoch) };
        let len = self.snapshot_length(len, scratch.len(), abi::AUDIO_MAX_BYTES)?;
        // SAFETY: exclusive initialized scratch remains live through the copy.
        let copied = unsafe {
            raw::incant_audio_v1_read_snapshot(
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
        self.check(AudioDeclarations::parse(&scratch[..len]))
    }
    /// Queue an ordered bounded batch in the shared transaction. Invalid input
    /// or insufficient scratch cancels the entire callback, including earlier
    /// writes even when the caller catches the returned error. Owner roles and
    /// transport epoch overflow are validated against final native declarations.
    pub fn audio_commands(&mut self, commands: &[AudioCommand], scratch: &mut [u8]) -> Result<()> {
        self.ready()?;
        self.check(if commands.len() <= abi::AUDIO_MAX_COMMANDS {
            Ok(())
        } else {
            Err(Error::Capacity)
        })?;
        let len = abi::AudioCommandHeader::SIZE + commands.len() * abi::AudioCommand::SIZE;
        self.check(if len <= scratch.len() {
            Ok(())
        } else {
            Err(Error::Capacity)
        })?;
        for command in commands {
            self.check(command.wire())?;
        }
        // All validation precedes scratch writes and the sole import call.
        scratch[..abi::AudioCommandHeader::SIZE].copy_from_slice(
            &abi::AudioCommandHeader {
                magic: *abi::AUDIO_COMMAND_MAGIC,
                count: commands.len() as u32,
                stride: abi::AudioCommand::SIZE as u32,
            }
            .encode(),
        );
        for (command, target) in commands.iter().zip(
            scratch[abi::AudioCommandHeader::SIZE..len]
                .as_chunks_mut::<{ abi::AudioCommand::SIZE }>()
                .0,
        ) {
            target.copy_from_slice(&command.wire().unwrap().encode());
        }
        // SAFETY: one synchronous copy of the complete bounded initialized batch.
        let result = unsafe {
            raw::incant_audio_v1_command(self.epoch, scratch.as_ptr() as usize as i32, len as i32)
        };
        self.check(if result == 0 {
            Ok(())
        } else {
            Err(Error::Failed)
        })
    }
}
