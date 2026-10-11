use crate::{EntityId, Error, FlipbookSnapshot, InputSnapshot, Result, Transform, Velocity, raw};
use core::marker::PhantomData;
use incant_wasm_abi as abi;
/// Valid only during one exported callback. Copies never borrow native storage.
/// An ignored method error still poisons the callback and prevents publication.
pub struct Tick<'tick> {
    epoch: i64,
    failed: bool,
    lifetime: PhantomData<fn(&'tick mut ()) -> &'tick mut ()>,
}
/// A table chunk from this callback, with aligned entity/component row ordering.
/// It cannot escape the exported callback, even through safe retained storage.
/// ```compile_fail
/// let mut retained = None;
/// incant_module::__private::run(1, 0.1, |tick, _| {
///     retained = Some(tick.chunk(0)?);
///     Ok(())
/// });
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Chunk<'tick> {
    index: i32,
    len: usize,
    velocity: bool,
    lifetime: PhantomData<fn(&'tick mut ()) -> &'tick mut ()>,
}
impl Chunk<'_> {
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn has_velocity(&self) -> bool {
        self.velocity
    }
}
impl<'tick> Tick<'tick> {
    pub(crate) fn new(epoch: i64) -> Self {
        Self {
            epoch,
            failed: false,
            lifetime: PhantomData,
        }
    }
    pub(crate) fn failed(&self) -> bool {
        self.failed
    }
    fn check<T>(&mut self, result: Result<T>) -> Result<T> {
        if self.failed {
            return Err(Error::Failed);
        }
        if result.is_err() {
            self.failed = true;
        }
        result
    }
    fn ready(&mut self) -> Result<()> {
        self.check(Ok(()))
    }
    pub fn chunk_count(&mut self) -> Result<usize> {
        self.ready()?;
        // SAFETY: scalar import only. Host checks active epoch/call quota.
        let count = unsafe { raw::incant_v1_chunk_count(self.epoch) };
        self.check(
            usize::try_from(count)
                .ok()
                .filter(|&n| n <= abi::CHUNKS)
                .ok_or(Error::Bounds),
        )
    }
    pub fn chunk(&mut self, index: usize) -> Result<Chunk<'tick>> {
        if index >= self.chunk_count()? {
            return self.check(Err(Error::Bounds));
        }
        // SAFETY: bounded index, scalar imports; no guest/native pointer involved.
        let (len, flags) = unsafe {
            (
                raw::incant_v1_chunk_len(self.epoch, index as i32),
                raw::incant_v1_chunk_flags(self.epoch, index as i32),
            )
        };
        let len = self.check(
            usize::try_from(len)
                .ok()
                .filter(|n| *n <= abi::COLUMNS / abi::ID_BYTES)
                .ok_or(Error::Bounds),
        )?;
        if flags & !1 != 0 {
            return self.check(Err(Error::Malformed));
        }
        Ok(Chunk {
            index: index as i32,
            len,
            velocity: flags & 1 != 0,
            lifetime: PhantomData,
        })
    }
    fn capacity<T>(&mut self, chunk: Chunk<'tick>, values: &[T]) -> Result<()> {
        self.check(if values.len() >= chunk.len {
            Ok(())
        } else {
            Err(Error::Capacity)
        })
    }
    pub fn read_ids<'a>(
        &mut self,
        chunk: Chunk<'tick>,
        values: &'a mut [EntityId],
    ) -> Result<&'a mut [EntityId]> {
        self.capacity(chunk, values)?;
        // SAFETY: EntityId is transparent initialized [u8;16]; whole prefix fits.
        unsafe {
            raw::incant_v1_read_ids(self.epoch, chunk.index, values.as_mut_ptr() as usize as i32);
        }
        Ok(&mut values[..chunk.len])
    }
    pub fn read_transforms<'a>(
        &mut self,
        chunk: Chunk<'tick>,
        values: &'a mut [Transform],
    ) -> Result<&'a mut [Transform]> {
        self.capacity(chunk, values)?;
        // SAFETY: initialized arrays of f64, exact numeric stride and whole prefix.
        unsafe {
            raw::incant_v1_read_transforms(
                self.epoch,
                chunk.index,
                values.as_mut_ptr() as usize as i32,
            );
        }
        Ok(&mut values[..chunk.len])
    }
    pub fn write_transforms(&mut self, chunk: Chunk<'tick>, values: &[Transform]) -> Result<()> {
        self.capacity(chunk, values)?;
        // SAFETY: initialized fixed-stride f64 prefix. Host validates numeric domain.
        unsafe {
            raw::incant_v1_write_transforms(
                self.epoch,
                chunk.index,
                values.as_ptr() as usize as i32,
            );
        }
        Ok(())
    }
    fn velocity(&mut self, chunk: Chunk<'tick>) -> Result<()> {
        self.check(if chunk.velocity {
            Ok(())
        } else {
            Err(Error::MissingVelocity)
        })
    }
    pub fn read_velocities<'a>(
        &mut self,
        chunk: Chunk<'tick>,
        values: &'a mut [Velocity],
    ) -> Result<&'a mut [Velocity]> {
        self.velocity(chunk)?;
        self.capacity(chunk, values)?;
        // SAFETY: initialized f64 arrays and validated present column/prefix.
        unsafe {
            raw::incant_v1_read_velocities(
                self.epoch,
                chunk.index,
                values.as_mut_ptr() as usize as i32,
            );
        }
        Ok(&mut values[..chunk.len])
    }
    pub fn write_velocities(&mut self, chunk: Chunk<'tick>, values: &[Velocity]) -> Result<()> {
        self.velocity(chunk)?;
        self.capacity(chunk, values)?;
        // SAFETY: initialized fixed-stride prefix; no pointer escapes this call.
        unsafe {
            raw::incant_v1_write_velocities(
                self.epoch,
                chunk.index,
                values.as_ptr() as usize as i32,
            );
        }
        Ok(())
    }
    pub fn spawn(
        &mut self,
        id: &EntityId,
        parent: Option<&EntityId>,
        transform: &Transform,
        velocity: Option<&Velocity>,
    ) -> Result<()> {
        self.ready()?;
        // SAFETY: borrowed fixed arrays live through the synchronous copy. -1 is
        // the registry's null sentinel, never a fabricated native pointer.
        unsafe {
            raw::incant_v1_spawn(
                self.epoch,
                ptr(id),
                optional(parent),
                ptr(transform),
                optional(velocity),
            );
        }
        Ok(())
    }
    pub fn despawn(&mut self, id: &EntityId) -> Result<()> {
        self.ready()?;
        // SAFETY: fixed-size identity is initialized and borrowed for this call.
        unsafe {
            raw::incant_v1_despawn(self.epoch, ptr(id));
        }
        Ok(())
    }
    pub fn set_parent(&mut self, id: &EntityId, parent: Option<&EntityId>) -> Result<()> {
        self.ready()?;
        // SAFETY: identities are borrowed initialized bytes; -1 is the null tag.
        unsafe {
            raw::incant_v1_set_parent(self.epoch, ptr(id), optional(parent));
        }
        Ok(())
    }
    pub fn set_velocity(&mut self, id: &EntityId, velocity: Option<&Velocity>) -> Result<()> {
        self.ready()?;
        // SAFETY: identity and optional f64 row stay live until the host copies them.
        unsafe {
            raw::incant_v1_set_velocity(self.epoch, ptr(id), optional(velocity));
        }
        Ok(())
    }
    fn control(&mut self, id: &EntityId, kind: abi::FlipbookControl, frame: u16) -> Result<()> {
        self.ready()?;
        // SAFETY: initialized identity, generated tag, and scalar frame.
        unsafe {
            raw::incant_flipbook_v1_control(self.epoch, ptr(id), kind as i32, i32::from(frame));
        }
        Ok(())
    }
    pub fn play(&mut self, id: &EntityId) -> Result<()> {
        self.control(id, abi::FlipbookControl::Play, 0)
    }
    pub fn pause(&mut self, id: &EntityId) -> Result<()> {
        self.control(id, abi::FlipbookControl::Pause, 0)
    }
    pub fn restart(&mut self, id: &EntityId) -> Result<()> {
        self.control(id, abi::FlipbookControl::Restart, 0)
    }
    pub fn seek(&mut self, id: &EntityId, frame: u16) -> Result<()> {
        self.control(id, abi::FlipbookControl::Seek, frame)
    }
    pub fn input<'a>(&mut self, scratch: &'a mut [u8]) -> Result<InputSnapshot<'a>> {
        self.ready()?;
        // SAFETY: length import has no pointers; the complete returned byte count
        // is validated against initialized scratch before exposing its pointer.
        let len = unsafe { raw::incant_input_v1_snapshot_len(self.epoch) };
        let len = self.snapshot_length(len, scratch.len(), abi::INPUT_MAX_BYTES)?;
        let actual = unsafe {
            raw::incant_input_v1_read_snapshot(
                self.epoch,
                scratch.as_mut_ptr() as usize as i32,
                len as i32,
            )
        };
        if actual as usize != len {
            return self.check(Err(Error::Malformed));
        }
        self.check(InputSnapshot::parse(&scratch[..len]))
    }
    pub fn flipbooks<'a>(&mut self, scratch: &'a mut [u8]) -> Result<FlipbookSnapshot<'a>> {
        self.ready()?;
        // SAFETY: scalar length query; the following copy uses checked scratch.
        let len = unsafe { raw::incant_flipbook_v1_snapshot_len(self.epoch) };
        let len = self.snapshot_length(len, scratch.len(), abi::FLIPBOOK_MAX_BYTES)?;
        let actual = unsafe {
            raw::incant_flipbook_v1_read_snapshot(
                self.epoch,
                scratch.as_mut_ptr() as usize as i32,
                len as i32,
            )
        };
        if actual as usize != len {
            return self.check(Err(Error::Malformed));
        }
        self.check(FlipbookSnapshot::parse(&scratch[..len]))
    }
    fn snapshot_length(&mut self, len: i32, capacity: usize, limit: usize) -> Result<usize> {
        let len = self.check(
            usize::try_from(len)
                .ok()
                .filter(|&n| n <= limit)
                .ok_or(Error::Bounds),
        )?;
        self.check(if len <= capacity {
            Ok(len)
        } else {
            Err(Error::Capacity)
        })
    }
}
fn ptr<T>(value: &T) -> i32 {
    value as *const T as usize as i32
}
fn optional<T>(value: Option<&T>) -> i32 {
    value.map_or(abi::OPTIONAL_NONE, ptr)
}
