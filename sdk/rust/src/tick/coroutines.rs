use super::Tick;
use crate::{
    EntityId, Error, Result,
    coroutines::{Next, Task},
    raw,
};
use incant_wasm_abi::{self as abi, CoroutineHeader as Header};

impl<'tick> Tick<'tick> {
    pub fn start_coroutine(
        &mut self,
        id: &str,
        routine: &str,
        state: &[u8],
        entity: Option<EntityId>,
        scratch: &mut [u8],
    ) -> Result<()> {
        self.ready()?;
        let (id, id_len) = self.check(abi::timers::encode_id(id).map_err(|_| Error::Bounds))?;
        let (routine, routine_len) =
            self.check(abi::timers::encode_id(routine).map_err(|_| Error::Bounds))?;
        self.check(
            if state.is_empty()
                || state.len() > abi::COROUTINE_FRAME_BYTES
                || scratch.len() < Header::SIZE + state.len()
            {
                Err(Error::Capacity)
            } else {
                Ok(())
            },
        )?;
        let header = Header {
            magic: *abi::COROUTINE_MAGIC,
            kind: 0,
            delay: 0,
            id_len,
            routine_len,
            flags: u32::from(entity.is_some()),
            state_len: state.len() as u32,
            id,
            routine,
            entity: entity.map_or([0; 16], |id| id.0),
        };
        scratch[..Header::SIZE].copy_from_slice(&header.encode());
        scratch[Header::SIZE..Header::SIZE + state.len()].copy_from_slice(state);
        self.coroutine_command(&scratch[..Header::SIZE + state.len()])
    }
    pub fn cancel_coroutine(&mut self, id: &str) -> Result<()> {
        self.ready()?;
        let (id, id_len) = self.check(abi::timers::encode_id(id).map_err(|_| Error::Bounds))?;
        self.coroutine_command(
            &Header {
                magic: *abi::COROUTINE_MAGIC,
                kind: 1,
                delay: 0,
                id_len,
                routine_len: 0,
                flags: 0,
                state_len: 0,
                id,
                routine: [0; 64],
                entity: [0; 16],
            }
            .encode(),
        )
    }
    fn coroutine_command(&mut self, bytes: &[u8]) -> Result<()> {
        self.ready()?;
        self.check(
            abi::coroutines::frame(bytes)
                .map(|_| ())
                .map_err(|_| Error::Malformed),
        )?;
        // SAFETY: bounded initialized bytes are copied during this active call.
        let result = unsafe {
            raw::incant_coroutines_v1_command(
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
    #[doc(hidden)]
    pub fn resume_coroutines(
        &mut self,
        mut callback: impl FnMut(&mut Self, &str, Task<'_>) -> Result<Next>,
    ) -> Result<()> {
        let mut scratch = [0u8; abi::COROUTINE_MAX_BYTES];
        loop {
            self.ready()?;
            // SAFETY: exclusive initialized fixed scratch and active epoch.
            let len = unsafe {
                raw::incant_coroutines_v1_next(
                    self.epoch,
                    scratch.as_mut_ptr() as usize as i32,
                    scratch.len() as i32,
                )
            };
            if len == 0 {
                return Ok(());
            }
            let len = self.snapshot_length(len, scratch.len(), abi::COROUTINE_MAX_BYTES)?;
            let (mut header, _) =
                self.check(abi::coroutines::frame(&scratch[..len]).map_err(|_| Error::Malformed))?;
            self.check(if header.kind == 4 {
                Ok(())
            } else {
                Err(Error::Malformed)
            })?;
            let id = abi::timers::id(&header.id, header.id_len).map_err(|_| Error::Malformed)?;
            let routine = abi::timers::id(&header.routine, header.routine_len)
                .map_err(|_| Error::Malformed)?;
            let result = callback(
                self,
                routine,
                Task {
                    id,
                    entity: (header.flags == 1).then_some(EntityId(header.entity)),
                    state: &mut scratch[Header::SIZE..len],
                },
            );
            let next = self.check(result)?;
            let len = match next {
                Next::Done => {
                    header.kind = 2;
                    header.state_len = 0;
                    Header::SIZE
                }
                Next::Wait(delay) => {
                    header.kind = 3;
                    header.delay = delay;
                    len
                }
            };
            scratch[..Header::SIZE].copy_from_slice(&header.encode());
            self.coroutine_command(&scratch[..len])?;
        }
    }
}
