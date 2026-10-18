//! Canonical schedule bytes, with routine schemas but no native entity tokens.
use super::*;
pub(crate) const MAX_BYTES: usize =
    16 + abi::COROUTINE_STORE_BYTES + abi::COROUTINE_MAX_TASKS * (abi::CoroutineHeader::SIZE + 40);

pub(super) fn decode(bytes: &[u8], tick: u64) -> Result<BTreeMap<String, Arc<Task>>, Error> {
    if bytes.len() < 16
        || bytes.len() > MAX_BYTES
        || &bytes[..8] != abi::COROUTINE_STATE_MAGIC
        || bytes[12..16] != [0; 4]
        || tick > timers::MAX_TICK
    {
        return Err(invalid());
    }
    let count = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    if count > abi::COROUTINE_MAX_TASKS || (tick == 0 && count != 0) {
        return Err(invalid());
    }
    let mut source = &bytes[16..];
    let mut tasks = BTreeMap::new();
    let mut stored = 0usize;
    let mut previous = String::new();
    for _ in 0..count {
        let metadata = source.get(..40).ok_or_else(invalid)?;
        let due = u64::from_le_bytes(metadata[..8].try_into().unwrap());
        if due <= tick || due > timers::MAX_TICK {
            return Err(invalid());
        }
        source = &source[40..];
        let header = abi::CoroutineHeader::decode(source).ok_or_else(invalid)?;
        let len = abi::CoroutineHeader::SIZE
            .checked_add(header.state_len as usize)
            .ok_or_else(invalid)?;
        let (h, data) = frame(source.get(..len).ok_or_else(invalid)?).map_err(|_| invalid())?;
        if h.kind != 4 {
            return Err(invalid());
        }
        let id = timers::id(&h.id, h.id_len).map_err(|_| invalid())?;
        if previous.as_str() >= id {
            return Err(invalid());
        }
        previous = id.to_owned();
        let routine = Routine::new(
            timers::id(&h.routine, h.routine_len).map_err(|_| invalid())?,
            h.state_len,
            metadata[8..40].try_into().unwrap(),
        )
        .ok_or_else(invalid)?;
        stored += data.len();
        if stored > abi::COROUTINE_STORE_BYTES {
            return Err(Error::Limit("coroutine saved state quota"));
        }
        tasks.insert(
            id.to_owned(),
            Arc::new(Task {
                routine,
                due,
                owner: (h.flags == 1).then_some(StableId(h.entity)),
                identity: None,
                bytes: data.into(),
            }),
        );
        source = &source[len..];
    }
    if !source.is_empty() {
        return Err(invalid());
    }
    Ok(tasks)
}
pub(crate) fn validate(bytes: &[u8], tick: u64) -> Result<(), Error> {
    decode(bytes, tick).map(|_| ())
}

impl State {
    pub fn checkpoint(
        &self,
        owner: impl Fn(StableId) -> Option<EntityIdentity>,
    ) -> Option<Vec<u8>> {
        if self.catalog.is_empty() {
            return None;
        }
        let mut bytes = Vec::from(*abi::COROUTINE_STATE_MAGIC);
        bytes.extend([0; 8]);
        let mut count = 0u32;
        for (id, task) in &self.committed {
            if task
                .owner
                .is_some_and(|id| owner(id).is_none_or(|identity| Some(identity) != task.identity))
            {
                continue;
            }
            count += 1;
            bytes.extend(task.due.to_le_bytes());
            bytes.extend(task.routine.schema());
            bytes.extend(header(id, task, 4, task.bytes.len()).encode());
            bytes.extend(&task.bytes);
        }
        bytes[8..12].copy_from_slice(&count.to_le_bytes());
        Some(bytes)
    }
    pub fn restored(
        &self,
        bytes: Option<&[u8]>,
        tick: u64,
        owner: impl Fn(StableId) -> Option<EntityIdentity>,
    ) -> Result<Self, Error> {
        let mut state = Self::new(self.catalog.clone())?;
        if let Some(bytes) = bytes {
            if self.catalog.is_empty() {
                return Err(invalid());
            }
            state.committed = decode(bytes, tick)?;
            for task in state.committed.values_mut() {
                if !self.catalog.contains(&task.routine) {
                    return Err(Error::Abi("coroutine saved routine schema"));
                }
                if let Some(id) = task.owner {
                    Arc::make_mut(task).identity = Some(owner(id).ok_or_else(invalid)?);
                }
            }
        } else if !self.catalog.is_empty() {
            return Err(Error::Abi("missing coroutine saved schedule"));
        }
        Ok(state)
    }
}
