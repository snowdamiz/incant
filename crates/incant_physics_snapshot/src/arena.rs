//! Inert arena layout. Payload rows belong to a future separately checked owner.
use crate::{Budget, Error, Identity, MAX_ITEMS, Result, vector};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Handle {
    pub slot: u32,
    pub generation: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArenaSlot {
    Free { next: Option<u32> },
    Occupied { generation: u32, row: u32 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArenaRecord {
    pub identity: Identity,
    pub generation: u32,
    pub free_head: Option<u32>,
    pub live: u32,
    pub slots: Vec<ArenaSlot>,
}

/// Immutable layout proof, not proof of any body/collider payload or world owner.
pub struct ValidatedArena {
    record: ArenaRecord,
}
impl ValidatedArena {
    pub fn record(&self) -> &ArenaRecord {
        &self.record
    }
    /// A stale, sentinel, out-of-range or free handle cannot address a payload.
    pub fn lookup(&self, handle: Handle) -> Option<u32> {
        match self.record.slots.get(handle.slot as usize)? {
            ArenaSlot::Occupied { generation, row } if *generation == handle.generation => {
                Some(*row)
            }
            _ => None,
        }
    }
}

pub(crate) fn arena_lengths(slots: usize, live: u32, generation: u32) -> Result<()> {
    // Native Arena::new/with_capacity always retains at least one slot. A zero
    // slot arena cannot grow through its current insert_slow_path (reserve(0)).
    if !(1..=MAX_ITEMS).contains(&slots) || live as usize > slots {
        return Err(Error::Length);
    }
    // Coarena reserves MAX as invalid; Arena::remove increments without a guard.
    // Exhausted-generation continuation needs a separate checked native policy.
    if generation == u32::MAX {
        return Err(Error::Generation);
    }
    Ok(())
}

pub fn validate_arena(record: ArenaRecord, budget: &mut Budget) -> Result<ValidatedArena> {
    let n = record.slots.len();
    arena_lengths(n, record.live, record.generation)?;
    budget.charge(n.checked_add(record.live as usize).ok_or(Error::Limit)?)?;
    let mut seen = vector::<u8>(n)?;
    seen.resize(n, 0);
    let mut rows = vector::<u8>(record.live as usize)?;
    rows.resize(record.live as usize, 0);
    let mut next = record.free_head;
    let mut free = 0usize;
    while let Some(index) = next {
        let index = index as usize;
        if index >= n || seen[index] != 0 {
            return Err(Error::Free);
        }
        let ArenaSlot::Free { next: successor } = record.slots[index] else {
            return Err(Error::Ownership);
        };
        seen[index] = 1;
        free += 1;
        next = successor;
    }
    let mut live = 0usize;
    for (index, slot) in record.slots.iter().enumerate() {
        match *slot {
            ArenaSlot::Free { .. } if seen[index] != 1 => return Err(Error::Free),
            ArenaSlot::Free { .. } => (),
            ArenaSlot::Occupied { generation, row } => {
                if generation > record.generation {
                    return Err(Error::Generation);
                }
                let row = row as usize;
                if seen[index] != 0 || row >= rows.len() || rows[row] != 0 {
                    return Err(Error::Ownership);
                }
                rows[row] = 1;
                live += 1;
            }
        }
    }
    if live != record.live as usize || live + free != n || rows.contains(&0) {
        return Err(Error::Ownership);
    }
    Ok(ValidatedArena { record })
}
