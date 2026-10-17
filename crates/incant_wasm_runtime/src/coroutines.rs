//! Staged cooperative tasks. Neither continuations nor guest pointers survive a tick.
pub(crate) mod checkpoint;
use crate::BoundaryError as Error;
use incant_runtime::{EntityIdentity, StableId};
use incant_wasm_abi::{
    self as abi,
    coroutines::{Routine, frame},
    timers,
};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone)]
struct Task {
    routine: Routine,
    owner: Option<StableId>,
    identity: Option<EntityIdentity>,
    due: u64,
    bytes: Box<[u8]>,
}
#[derive(Default)]
pub(crate) struct State {
    catalog: Vec<Routine>,
    committed: BTreeMap<String, Arc<Task>>,
    staged: Option<BTreeMap<String, Arc<Task>>>,
    current: Option<(String, Arc<Task>)>,
    tick: u64,
    actions: usize,
    resumes: usize,
    transfers: usize,
    pub rejected: bool,
}
fn invalid() -> Error {
    Error::Abi("coroutine contract")
}
fn deadline(tick: u64, delay: u32) -> Result<u64, Error> {
    tick.checked_add(u64::from(delay))
        .filter(|next| delay > 0 && *next <= timers::MAX_TICK)
        .ok_or_else(invalid)
}
impl State {
    pub fn new(mut catalog: Vec<Routine>) -> Result<Self, Error> {
        if catalog.len() > abi::COROUTINE_MAX_ROUTINES {
            return Err(invalid());
        }
        catalog.sort_by(|a, b| a.name().cmp(b.name()));
        if catalog.windows(2).any(|w| w[0].name() == w[1].name()) {
            return Err(invalid());
        }
        Ok(Self {
            catalog,
            ..Self::default()
        })
    }
    pub fn enabled(&self) -> bool {
        !self.catalog.is_empty()
    }
    pub fn same_catalog(&self, other: &Self) -> bool {
        self.catalog == other.catalog
    }
    pub fn replacement(&mut self) -> Self {
        Self {
            catalog: self.catalog.clone(),
            committed: std::mem::take(&mut self.committed),
            ..Self::default()
        }
    }
    pub fn begin(
        &mut self,
        tick: u64,
        owner: impl Fn(StableId) -> Option<EntityIdentity>,
    ) -> Result<(), Error> {
        self.abort();
        self.rejected = false;
        self.actions = 0;
        self.resumes = 0;
        self.transfers = 0;
        self.tick = tick;
        if tick == 0 || tick > timers::MAX_TICK {
            return Err(invalid());
        }
        let mut next = self.committed.clone();
        next.retain(|_, task| {
            task.owner
                .is_none_or(|id| owner(id).is_some_and(|value| Some(value) == task.identity))
        });
        if next.values().any(|task| task.due < tick) {
            return Err(invalid());
        }
        self.staged = Some(next);
        Ok(())
    }
    pub fn abort(&mut self) {
        self.staged = None;
        self.current = None;
    }
    pub fn ready(&self) -> bool {
        !self.rejected
            && self.current.is_none()
            && self
                .staged
                .as_ref()
                .is_some_and(|s| s.values().all(|task| task.due > self.tick))
    }
    pub fn bind_new_owners(
        &mut self,
        owner: impl Fn(StableId) -> Option<EntityIdentity>,
    ) -> Result<(), Error> {
        for task in self.staged.as_mut().ok_or_else(invalid)?.values_mut() {
            if let Some(id) = task.owner
                && task.identity.is_none()
            {
                Arc::make_mut(task).identity = Some(owner(id).ok_or_else(invalid)?);
            }
        }
        Ok(())
    }
    pub fn commit(&mut self, owner: impl Fn(StableId) -> Option<EntityIdentity>) {
        let mut tasks = self.staged.take().expect("active coroutine transaction");
        tasks.retain(|_, task| {
            task.owner
                .is_none_or(|id| owner(id).is_some_and(|value| Some(value) == task.identity))
        });
        self.committed = tasks;
    }
    fn transfer(&mut self, count: usize) -> Result<(), Error> {
        self.transfers = self
            .transfers
            .checked_add(count)
            .filter(|n| *n <= abi::COROUTINE_TRANSFER_BYTES)
            .ok_or(Error::Limit("coroutine transfers"))?;
        Ok(())
    }
    fn insert(&mut self, id: String, task: Arc<Task>) -> Result<(), Error> {
        let state = self.staged.as_mut().ok_or_else(invalid)?;
        if !state.contains_key(&id) && state.len() == abi::COROUTINE_MAX_TASKS {
            return Err(Error::Limit("coroutine task quota"));
        }
        let size = state.values().map(|t| t.bytes.len()).sum::<usize>()
            - state.get(&id).map_or(0, |t| t.bytes.len())
            + task.bytes.len();
        if size > abi::COROUTINE_STORE_BYTES {
            return Err(Error::Limit("coroutine state quota"));
        }
        state.insert(id, task);
        Ok(())
    }
    pub fn command(&mut self, bytes: &[u8], owner: impl Fn(StableId) -> bool) -> Result<(), Error> {
        let (h, data) = frame(bytes).map_err(|_| invalid())?;
        let id = timers::id(&h.id, h.id_len).map_err(|_| invalid())?;
        if self.catalog.is_empty() || h.kind == 4 {
            return Err(invalid());
        }
        self.transfer(data.len())?;
        if h.kind < 2 {
            self.actions += 1;
            if self.actions > abi::COROUTINE_MAX_ACTIONS {
                return Err(Error::Limit("coroutine actions"));
            }
        }
        match h.kind {
            0 => {
                let name = timers::id(&h.routine, h.routine_len).map_err(|_| invalid())?;
                let routine = *self
                    .catalog
                    .iter()
                    .find(|r| r.name() == name)
                    .ok_or_else(invalid)?;
                if data.len() != routine.bytes()
                    || self.staged.as_ref().ok_or_else(invalid)?.contains_key(id)
                {
                    return Err(invalid());
                }
                let owner = if h.flags == 1 {
                    let id = StableId(h.entity);
                    if !owner(id) {
                        return Err(invalid());
                    }
                    Some(id)
                } else {
                    None
                };
                self.insert(
                    id.to_owned(),
                    Arc::new(Task {
                        routine,
                        owner,
                        identity: None,
                        due: deadline(self.tick, 1)?,
                        bytes: data.into(),
                    }),
                )?;
            }
            1 => {
                self.staged.as_mut().ok_or_else(invalid)?.remove(id);
            }
            2 | 3 => {
                let (key, old) = self.current.as_ref().ok_or_else(invalid)?;
                if key != id
                    || timers::id(&h.routine, h.routine_len).map_err(|_| invalid())?
                        != old.routine.name()
                    || h.flags != u32::from(old.owner.is_some())
                    || h.entity != old.owner.map_or([0; 16], |id| id.0)
                    || (h.kind == 3 && data.len() != old.routine.bytes())
                {
                    return Err(invalid());
                }
                let due = if h.kind == 3 {
                    Some(deadline(self.tick, h.delay)?)
                } else {
                    None
                };
                let (key, old) = self.current.take().unwrap();
                // A cancel or cancel/start during the callback supersedes its result.
                if self
                    .staged
                    .as_ref()
                    .unwrap()
                    .get(&key)
                    .is_some_and(|now| Arc::ptr_eq(now, &old))
                {
                    if let Some(due) = due {
                        self.insert(
                            key,
                            Arc::new(Task {
                                due,
                                bytes: data.into(),
                                routine: old.routine,
                                owner: old.owner,
                                identity: old.identity,
                            }),
                        )?;
                    } else {
                        self.staged.as_mut().unwrap().remove(&key);
                    }
                }
            }
            _ => return Err(invalid()),
        }
        Ok(())
    }
    pub fn next(&mut self, output: &mut [u8]) -> Result<usize, Error> {
        if self.catalog.is_empty() || self.current.is_some() {
            return Err(invalid());
        }
        let Some((id, task)) = self
            .staged
            .as_ref()
            .ok_or_else(invalid)?
            .iter()
            .find(|(_, t)| t.due == self.tick)
        else {
            return Ok(0);
        };
        let (id, task) = (id.clone(), Arc::clone(task));
        let len = abi::CoroutineHeader::SIZE + task.bytes.len();
        if output.len() < len {
            return Err(Error::Abi("coroutine scratch capacity"));
        }
        if self.resumes == abi::COROUTINE_MAX_TASKS {
            return Err(Error::Limit("coroutine resume quota"));
        }
        self.resumes += 1;
        self.transfer(task.bytes.len())?;
        let header = header(&id, &task, 4, task.bytes.len());
        output[..abi::CoroutineHeader::SIZE].copy_from_slice(&header.encode());
        output[abi::CoroutineHeader::SIZE..len].copy_from_slice(&task.bytes);
        self.current = Some((id, task));
        Ok(len)
    }
}
fn header(id: &str, task: &Task, kind: u32, size: usize) -> abi::CoroutineHeader {
    let (id, id_len) = timers::encode_id(id).expect("validated task id");
    let (routine, routine_len) = timers::encode_id(task.routine.name()).expect("validated routine");
    abi::CoroutineHeader {
        magic: *abi::COROUTINE_MAGIC,
        kind,
        delay: 0,
        id_len,
        routine_len,
        flags: u32::from(task.owner.is_some()),
        state_len: size as u32,
        id,
        routine,
        entity: task.owner.map_or([0; 16], |id| id.0),
    }
}
