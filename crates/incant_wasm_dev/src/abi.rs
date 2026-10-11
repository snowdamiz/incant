//! incant_v1: exact byte counts, little-endian f64 columns, opaque tick epochs.
use crate::{bounds, transaction::Chunk};
use incant_runtime::{CookedEntity, FrameCommands, StableId, StructuralCommand};
use incant_types::{Transform, Velocity};
use wasmtime::{Caller, Linker, Memory, Result, StoreLimits, bail};

pub struct State {
    pub limits: StoreLimits,
    pub active: bool,
    pub epoch: i64,
    pub chunks: Vec<Chunk>,
    pub commands: FrameCommands,
    pub transferred: usize,
    pub calls: usize,
}
impl State {
    fn call(&mut self, epoch: i64) -> Result<()> {
        if !self.active || self.epoch != epoch {
            bail!("inactive or expired ECS epoch");
        }
        self.calls += 1;
        if self.calls > bounds::CALLS {
            bail!("host call quota");
        }
        Ok(())
    }
    fn transfer(&mut self, count: usize) -> Result<()> {
        self.transferred = self
            .transferred
            .checked_add(count)
            .filter(|n| *n <= bounds::TRANSFERS)
            .ok_or_else(|| wasmtime::format_err!("transfer quota"))?;
        Ok(())
    }
    fn command(&mut self, command: StructuralCommand) -> Result<()> {
        if self.commands.len() == bounds::COMMANDS {
            bail!("structural command quota");
        }
        self.commands.push(command).map_err(wasmtime::Error::new)
    }
}
fn memory(caller: &mut Caller<'_, State>) -> Result<Memory> {
    caller
        .get_export("memory")
        .and_then(|e| e.into_memory())
        .ok_or_else(|| wasmtime::format_err!("missing memory"))
}
fn range(ptr: i32, len: usize, total: usize) -> Result<std::ops::Range<usize>> {
    let begin =
        usize::try_from(ptr).map_err(|_| wasmtime::format_err!("negative memory offset"))?;
    let end = begin
        .checked_add(len)
        .filter(|end| *end <= total)
        .ok_or_else(|| wasmtime::format_err!("memory range out of bounds"))?;
    Ok(begin..end)
}
fn read<const N: usize>(caller: &mut Caller<'_, State>, ptr: i32) -> Result<[u8; N]> {
    caller.data_mut().transfer(N)?;
    let memory = memory(caller)?;
    let bytes = memory.data(&*caller);
    Ok(bytes[range(ptr, N, bytes.len())?].try_into().unwrap())
}
fn stable(caller: &mut Caller<'_, State>, ptr: i32) -> Result<StableId> {
    Ok(StableId(read(caller, ptr)?))
}
fn optional_id(caller: &mut Caller<'_, State>, ptr: i32) -> Result<Option<StableId>> {
    if ptr == -1 {
        Ok(None)
    } else {
        stable(caller, ptr).map(Some)
    }
}
fn values<const N: usize>(bytes: &[u8]) -> [f64; N] {
    std::array::from_fn(|i| f64::from_le_bytes(bytes[i * 8..i * 8 + 8].try_into().unwrap()))
}
fn velocity(caller: &mut Caller<'_, State>, ptr: i32) -> Result<Option<Velocity>> {
    if ptr == -1 {
        return Ok(None);
    }
    Ok(Some(Velocity {
        linear: values(&read::<24>(caller, ptr)?),
    }))
}
fn index(index: i32, state: &State) -> Result<usize> {
    usize::try_from(index)
        .ok()
        .filter(|i| *i < state.chunks.len())
        .ok_or_else(|| wasmtime::format_err!("invalid numeric chunk"))
}
fn column(
    mut caller: Caller<'_, State>,
    epoch: i64,
    chunk: i32,
    ptr: i32,
    velocity: bool,
    write: bool,
) -> Result<()> {
    caller.data_mut().call(epoch)?;
    let chunk = index(chunk, caller.data())?;
    let len = if velocity {
        caller.data().chunks[chunk]
            .velocities
            .as_ref()
            .ok_or_else(|| wasmtime::format_err!("chunk has no Velocity"))?
            .len()
    } else {
        caller.data().chunks[chunk].transforms.len()
    };
    caller.data_mut().transfer(len * 8)?;
    let memory = memory(&mut caller)?;
    let (bytes, state) = memory.data_and_store_mut(&mut caller);
    let region = range(ptr, len * 8, bytes.len())?;
    let chunk = &mut state.chunks[chunk];
    let column = if velocity {
        chunk.velocities.as_mut().unwrap()
    } else {
        &mut chunk.transforms
    };
    if write {
        for (number, raw) in column.iter_mut().zip(bytes[region].as_chunks::<8>().0) {
            *number = f64::from_le_bytes(*raw);
        }
        if velocity {
            chunk.dirty_velocity = true;
        } else {
            chunk.dirty_transform = true;
        }
    } else {
        for (number, raw) in column.iter().zip(bytes[region].as_chunks_mut::<8>().0) {
            raw.copy_from_slice(&number.to_le_bytes());
        }
    }
    Ok(())
}
pub fn install(linker: &mut Linker<State>) -> Result<()> {
    linker.func_wrap(
        "incant_v1",
        "chunk_count",
        |mut c: Caller<'_, State>, e: i64| -> Result<i32> {
            c.data_mut().call(e)?;
            Ok(c.data().chunks.len() as i32)
        },
    )?;
    linker.func_wrap(
        "incant_v1",
        "chunk_len",
        |mut c: Caller<'_, State>, e: i64, i: i32| -> Result<i32> {
            c.data_mut().call(e)?;
            Ok(c.data().chunks[index(i, c.data())?].ids.len() as i32)
        },
    )?;
    linker.func_wrap(
        "incant_v1",
        "chunk_flags",
        |mut c: Caller<'_, State>, e: i64, i: i32| -> Result<i32> {
            c.data_mut().call(e)?;
            Ok(i32::from(
                c.data().chunks[index(i, c.data())?].velocities.is_some(),
            ))
        },
    )?;
    linker.func_wrap(
        "incant_v1",
        "read_ids",
        |mut c: Caller<'_, State>, e: i64, i: i32, p: i32| -> Result<()> {
            c.data_mut().call(e)?;
            let i = index(i, c.data())?;
            let len = c.data().chunks[i].ids.len() * 16;
            c.data_mut().transfer(len)?;
            let m = memory(&mut c)?;
            let (bytes, state) = m.data_and_store_mut(&mut c);
            let region = range(p, len, bytes.len())?;
            for (id, raw) in state.chunks[i]
                .ids
                .iter()
                .zip(bytes[region].as_chunks_mut::<16>().0)
            {
                raw.copy_from_slice(&id.0);
            }
            Ok(())
        },
    )?;
    for (name, velocity, write) in [
        ("read_transforms", false, false),
        ("write_transforms", false, true),
        ("read_velocities", true, false),
        ("write_velocities", true, true),
    ] {
        linker.func_wrap(
            "incant_v1",
            name,
            move |c: Caller<'_, State>, e: i64, i: i32, p: i32| column(c, e, i, p, velocity, write),
        )?;
    }
    linker.func_wrap(
        "incant_v1",
        "spawn",
        |mut c: Caller<'_, State>,
         e: i64,
         id: i32,
         parent: i32,
         transform: i32,
         motion: i32|
         -> Result<()> {
            c.data_mut().call(e)?;
            let id = stable(&mut c, id)?;
            let parent = optional_id(&mut c, parent)?;
            let numbers: [f64; 10] = values(&read::<80>(&mut c, transform)?);
            let velocity = velocity(&mut c, motion)?;
            c.data_mut().command(StructuralCommand::Spawn(CookedEntity {
                id,
                parent,
                velocity,
                transform: Transform {
                    translation: numbers[..3].try_into().unwrap(),
                    rotation: numbers[3..7].try_into().unwrap(),
                    scale: numbers[7..].try_into().unwrap(),
                },
            }))
        },
    )?;
    linker.func_wrap(
        "incant_v1",
        "despawn",
        |mut c: Caller<'_, State>, e: i64, id: i32| -> Result<()> {
            c.data_mut().call(e)?;
            let id = stable(&mut c, id)?;
            c.data_mut().command(StructuralCommand::Despawn(id))
        },
    )?;
    linker.func_wrap(
        "incant_v1",
        "set_velocity",
        |mut c: Caller<'_, State>, e: i64, id: i32, v: i32| -> Result<()> {
            c.data_mut().call(e)?;
            let entity = stable(&mut c, id)?;
            let velocity = velocity(&mut c, v)?;
            c.data_mut()
                .command(StructuralCommand::SetVelocity { entity, velocity })
        },
    )?;
    linker.func_wrap(
        "incant_v1",
        "set_parent",
        |mut c: Caller<'_, State>, e: i64, id: i32, parent: i32| -> Result<()> {
            c.data_mut().call(e)?;
            let entity = stable(&mut c, id)?;
            let parent = optional_id(&mut c, parent)?;
            c.data_mut()
                .command(StructuralCommand::SetParent { entity, parent })
        },
    )?;
    Ok(())
}
