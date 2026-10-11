use crate::{
    BoundaryError as Error, limits,
    transaction::{self, Chunk},
};
use incant_runtime::{
    CookedEntity, FlipbookControl, FlipbookSnapshot, FrameCommands, NativeWorld, StableId,
    StructuralCommand, WorldIdentity,
};
use incant_types::{Transform, Velocity};

// The native component representation and generated wire record must agree on
// every numeric field. These compile on each target, not only the test host.
const _: () = {
    use incant_wasm_abi as abi;
    assert!(incant_runtime::TRANSFORM_STRIDE == abi::TRANSFORM_STRIDE);
    assert!(incant_runtime::VELOCITY_STRIDE == abi::VELOCITY_STRIDE);
    assert!(std::mem::size_of::<Transform>() == abi::TransformRow::SIZE);
    assert!(std::mem::offset_of!(Transform, translation) == abi::TransformRow::TRANSLATION_OFFSET);
    assert!(std::mem::offset_of!(Transform, rotation) == abi::TransformRow::ROTATION_OFFSET);
    assert!(std::mem::offset_of!(Transform, scale) == abi::TransformRow::SCALE_OFFSET);
    assert!(std::mem::size_of::<Velocity>() == abi::VelocityRow::SIZE);
    assert!(std::mem::offset_of!(Velocity, linear) == abi::VelocityRow::LINEAR_OFFSET);
};

/// Trusted host adapter operation; all integer arguments are guest offsets/indices.
#[derive(Clone, Copy)]
pub enum Call {
    FlipbookLen,
    ReadFlipbooks {
        ptr: i32,
        capacity: i32,
    },
    ControlFlipbook {
        id: i32,
        kind: i32,
        frame: i32,
    },
    InputLen,
    ReadInput {
        ptr: i32,
        capacity: i32,
    },
    ChunkCount,
    ChunkLen(i32),
    ChunkFlags(i32),
    ReadIds(i32, i32),
    Column {
        chunk: i32,
        ptr: i32,
        velocity: bool,
        write: bool,
    },
    Spawn {
        id: i32,
        parent: i32,
        transform: i32,
        velocity: i32,
    },
    Despawn(i32),
    SetVelocity(i32, i32),
    SetParent(i32, i32),
}
/// Adapter state for one module. Only copied values enter guest linear memory.
/// Hosts must keep exclusive world ownership from begin through finish/abort.
/// Failed acceptance is terminal, including unwinds; reload explicitly replaces
/// this state. No guest access or native pointer is retained by this type.
#[derive(Default)]
pub struct Capability {
    identity: Option<WorldIdentity>,
    failed: bool,
    active: bool,
    epoch: i64,
    chunks: Vec<Chunk>,
    commands: FrameCommands,
    input: Vec<u8>,
    input_available: bool,
    flipbook_rows: Vec<FlipbookSnapshot>,
    flipbooks: Vec<u8>,
    transferred: usize,
    calls: usize,
}
impl Capability {
    pub fn is_failed(&self) -> bool {
        self.failed
    }
    pub fn replacement(&self) -> Self {
        Self {
            identity: self.identity.clone(),
            ..Self::default()
        }
    }
    pub fn begin(&mut self, world: &mut NativeWorld, dt: f64) -> Result<i64, Error> {
        if self.failed {
            return Err(Error::Failed);
        }
        if !dt.is_finite() || dt <= 0. || dt > 1. {
            return Err(Error::Abi("invalid dt"));
        }
        let identity = world.spatial_view()?.identity().clone();
        if self.identity.as_ref().is_some_and(|old| old != &identity) {
            return Err(Error::Abi("module is bound to a different world"));
        }
        self.identity = Some(identity);
        self.failed = true;
        self.input_available = false;
        self.input.clear();
        self.epoch = self.epoch.checked_add(1).ok_or(Error::Limit("epoch"))?;
        self.chunks = world.with_numeric_columns(transaction::capture)?;
        if let Err(error) =
            crate::flipbooks::encode(world, &mut self.flipbook_rows, &mut self.flipbooks)
        {
            self.abort();
            return Err(error);
        }
        self.commands.clear();
        self.transferred = 0;
        self.calls = 0;
        self.active = true;
        Ok(self.epoch)
    }
    /// Optional v1 input capability. Only an engine-validated runtime is accepted.
    pub fn begin_with_input(
        &mut self,
        world: &mut NativeWorld,
        dt: f64,
        input: &incant_input::InputRuntime,
    ) -> Result<i64, Error> {
        let epoch = self.begin(world, dt)?;
        if let Err(error) = crate::input::encode(input, world.tick(), &mut self.input) {
            self.abort();
            return Err(error);
        }
        self.input_available = true;
        Ok(epoch)
    }
    pub fn abort(&mut self) {
        self.active = false;
        self.input_available = false;
        self.input.clear();
        self.flipbook_rows.clear();
        self.flipbooks.clear();
        self.chunks.clear();
        self.commands.clear();
    }
    pub fn finish(&mut self, world: &mut NativeWorld) -> Result<(), Error> {
        if !self.active {
            return Err(Error::Abi("inactive ECS transaction"));
        }
        self.active = false;
        self.input_available = false;
        self.input.clear();
        self.flipbook_rows.clear();
        self.flipbooks.clear();
        if Some(world.spatial_view()?.identity()) != self.identity.as_ref() {
            self.abort();
            return Err(Error::Abi("different commit world"));
        }
        let result =
            world.with_numeric_columns(|columns| transaction::commit(columns, &self.chunks));
        self.chunks.clear();
        let mut commands = std::mem::take(&mut self.commands);
        let result = result.and_then(|undo| {
            if let Err(error) = world.apply(&mut commands) {
                world.with_numeric_columns(|columns| undo.restore(columns));
                return Err(error.into());
            }
            Ok(())
        });
        if result.is_ok() {
            self.failed = false;
        }
        result
    }
    fn enter(&mut self, epoch: i64) -> Result<(), Error> {
        if !self.active || self.epoch != epoch {
            return Err(Error::Abi("inactive or expired ECS epoch"));
        }
        self.calls += 1;
        if self.calls > limits::CALLS {
            return Err(Error::Limit("host call quota"));
        }
        Ok(())
    }
    fn transfer(&mut self, count: usize) -> Result<(), Error> {
        self.transferred = self
            .transferred
            .checked_add(count)
            .filter(|n| *n <= limits::TRANSFERS)
            .ok_or(Error::Limit("transfer quota"))?;
        Ok(())
    }
    fn command(&mut self, command: StructuralCommand) -> Result<(), Error> {
        if self.commands.len() == limits::COMMANDS {
            return Err(Error::Limit("structural command quota"));
        }
        self.commands.push(command)?;
        Ok(())
    }
    fn index(&self, index: i32) -> Result<usize, Error> {
        usize::try_from(index)
            .ok()
            .filter(|i| *i < self.chunks.len())
            .ok_or(Error::Abi("invalid numeric chunk"))
    }
    fn read<const N: usize>(&mut self, memory: &[u8], ptr: i32) -> Result<[u8; N], Error> {
        self.transfer(N)?;
        Ok(memory[range(ptr, N, memory.len())?].try_into().unwrap())
    }
    fn stable(&mut self, memory: &[u8], ptr: i32) -> Result<StableId, Error> {
        Ok(StableId(self.read(memory, ptr)?))
    }
    fn optional_id(&mut self, memory: &[u8], ptr: i32) -> Result<Option<StableId>, Error> {
        if ptr == -1 {
            Ok(None)
        } else {
            self.stable(memory, ptr).map(Some)
        }
    }
    fn velocity(&mut self, memory: &[u8], ptr: i32) -> Result<Option<Velocity>, Error> {
        if ptr == -1 {
            return Ok(None);
        }
        Ok(Some(Velocity {
            linear: values(&self.read::<24>(memory, ptr)?),
        }))
    }
    /// Synchronous import; memory contains initialized guest bytes only and is
    /// borrowed for this call. No world observation/publication occurs here.
    pub fn invoke(&mut self, memory: &mut [u8], epoch: i64, call: Call) -> Result<i32, Error> {
        self.enter(epoch)?;
        if memory.len() > limits::MEMORY {
            return Err(Error::Limit("linear memory"));
        }
        match call {
            Call::FlipbookLen => return Ok(self.flipbooks.len() as i32),
            Call::ReadFlipbooks { ptr, capacity } => {
                let len = self.flipbooks.len();
                if usize::try_from(capacity).ok().is_none_or(|n| n < len) {
                    return Err(Error::Abi("flipbook destination capacity"));
                }
                range(ptr, capacity as usize, memory.len())?;
                let region = range(ptr, len, memory.len())?;
                self.transfer(len)?;
                memory[region].copy_from_slice(&self.flipbooks);
                return Ok(len as i32);
            }
            Call::ControlFlipbook { id, kind, frame } => {
                let control = match (kind, frame) {
                    (0, 0) => FlipbookControl::Play,
                    (1, 0) => FlipbookControl::Pause,
                    (2, 0) => FlipbookControl::Restart,
                    (3, frame) => FlipbookControl::Seek(
                        u16::try_from(frame).map_err(|_| Error::Abi("flipbook seek frame"))?,
                    ),
                    _ => return Err(Error::Abi("flipbook control tag/reserved frame")),
                };
                let entity = self.stable(memory, id)?;
                self.command(StructuralCommand::ControlFlipbook { entity, control })?;
            }
            Call::InputLen => {
                if !self.input_available {
                    return Err(Error::Abi("input capability unavailable"));
                }
                return Ok(self.input.len() as i32);
            }
            Call::ReadInput { ptr, capacity } => {
                if !self.input_available {
                    return Err(Error::Abi("input capability unavailable"));
                }
                let len = self.input.len();
                if usize::try_from(capacity).ok().is_none_or(|n| n < len) {
                    return Err(Error::Abi("input destination capacity"));
                }
                range(ptr, capacity as usize, memory.len())?;
                let region = range(ptr, len, memory.len())?;
                self.transfer(len)?;
                memory[region].copy_from_slice(&self.input);
                return Ok(len as i32);
            }
            Call::ChunkCount => return Ok(self.chunks.len() as i32),
            Call::ChunkLen(i) => return Ok(self.chunks[self.index(i)?].ids.len() as i32),
            Call::ChunkFlags(i) => {
                return Ok(i32::from(self.chunks[self.index(i)?].velocities.is_some()));
            }
            Call::ReadIds(i, ptr) => {
                let i = self.index(i)?;
                let len = self.chunks[i].ids.len() * 16;
                self.transfer(len)?;
                let region = range(ptr, len, memory.len())?;
                for (id, raw) in self.chunks[i]
                    .ids
                    .iter()
                    .zip(memory[region].as_chunks_mut::<16>().0)
                {
                    raw.copy_from_slice(&id.0);
                }
            }
            Call::Column {
                chunk,
                ptr,
                velocity,
                write,
            } => {
                let i = self.index(chunk)?;
                let len = if velocity {
                    self.chunks[i]
                        .velocities
                        .as_ref()
                        .ok_or(Error::Abi("chunk has no Velocity"))?
                        .len()
                } else {
                    self.chunks[i].transforms.len()
                };
                self.transfer(len * 8)?;
                let region = range(ptr, len * 8, memory.len())?;
                let chunk = &mut self.chunks[i];
                let column = if velocity {
                    chunk.velocities.as_mut().unwrap()
                } else {
                    &mut chunk.transforms
                };
                if write {
                    for (number, raw) in column.iter_mut().zip(memory[region].as_chunks::<8>().0) {
                        *number = f64::from_le_bytes(*raw);
                    }
                    if velocity {
                        chunk.dirty_velocity = true;
                    } else {
                        chunk.dirty_transform = true;
                    }
                } else {
                    for (number, raw) in column.iter().zip(memory[region].as_chunks_mut::<8>().0) {
                        raw.copy_from_slice(&number.to_le_bytes());
                    }
                }
            }
            Call::Spawn {
                id,
                parent,
                transform,
                velocity,
            } => {
                let id = self.stable(memory, id)?;
                let parent = self.optional_id(memory, parent)?;
                let numbers: [f64; 10] = values(&self.read::<80>(memory, transform)?);
                let velocity = self.velocity(memory, velocity)?;
                self.command(StructuralCommand::Spawn(CookedEntity {
                    id,
                    parent,
                    velocity,
                    transform: Transform {
                        translation: numbers[..3].try_into().unwrap(),
                        rotation: numbers[3..7].try_into().unwrap(),
                        scale: numbers[7..].try_into().unwrap(),
                    },
                }))?;
            }
            Call::Despawn(ptr) => {
                let id = self.stable(memory, ptr)?;
                self.command(StructuralCommand::Despawn(id))?;
            }
            Call::SetVelocity(id, ptr) => {
                let entity = self.stable(memory, id)?;
                let velocity = self.velocity(memory, ptr)?;
                self.command(StructuralCommand::SetVelocity { entity, velocity })?;
            }
            Call::SetParent(id, ptr) => {
                let entity = self.stable(memory, id)?;
                let parent = self.optional_id(memory, ptr)?;
                self.command(StructuralCommand::SetParent { entity, parent })?;
            }
        }
        Ok(0)
    }
}
fn range(ptr: i32, len: usize, total: usize) -> Result<std::ops::Range<usize>, Error> {
    let begin = usize::try_from(ptr).map_err(|_| Error::Abi("negative memory offset"))?;
    let end = begin
        .checked_add(len)
        .filter(|end| *end <= total)
        .ok_or(Error::Abi("memory range out of bounds"))?;
    Ok(begin..end)
}
fn values<const N: usize>(bytes: &[u8]) -> [f64; N] {
    std::array::from_fn(|i| f64::from_le_bytes(bytes[i * 8..i * 8 + 8].try_into().unwrap()))
}
