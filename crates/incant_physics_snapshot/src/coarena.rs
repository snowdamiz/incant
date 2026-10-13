//! Preserve invalidated and stale payloads: native get_unknown_gen observes them.
use crate::{
    ArenaSlot, Budget, Error, Handle, Identity, MAX_ITEMS, Result, ValidatedArena, vector,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CoarenaRow {
    /// MAX is the pinned invalidated/default marker; the payload still exists.
    pub generation: u32,
    pub row: u32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoarenaRecord {
    pub identity: Identity,
    pub rows: Vec<CoarenaRow>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowState {
    Current,
    Stale,
    Invalidated,
}

/// Borrows the exact supplied arena witness for its entire lifetime. This is a
/// structural association, not authorization of an enclosing world or payload.
pub struct ValidatedCoarena<'a> {
    record: CoarenaRecord,
    arena: &'a ValidatedArena,
}
impl ValidatedCoarena<'_> {
    pub fn record(&self) -> &CoarenaRecord {
        &self.record
    }
    pub fn state(&self, slot: u32) -> Option<RowState> {
        let item = self.record.rows.get(slot as usize)?;
        Some(if item.generation == u32::MAX {
            RowState::Invalidated
        } else if self
            .arena
            .lookup(Handle {
                slot,
                generation: item.generation,
            })
            .is_some()
        {
            RowState::Current
        } else {
            RowState::Stale
        })
    }
    /// Native Coarena::get matches its own generation even if the owner handle
    /// was removed. Callers requiring current ownership must check state/arena.
    /// Unlike that native getter, this checked boundary refuses MAX handles;
    /// use unknown_generation_row to preserve invalidated/default payloads.
    pub fn lookup(&self, handle: Handle) -> Option<u32> {
        let item = self.record.rows.get(handle.slot as usize)?;
        (item.generation == handle.generation && handle.generation != u32::MAX).then_some(item.row)
    }
    /// This includes invalidated/stale rows; no payload is silently erased.
    pub fn unknown_generation_row(&self, slot: u32) -> Option<u32> {
        self.record.rows.get(slot as usize).map(|item| item.row)
    }
}

pub(crate) fn coarena_length(count: usize, arena: &ValidatedArena) -> Result<()> {
    if count > MAX_ITEMS || count > arena.record().slots.len() {
        return Err(Error::Length);
    }
    Ok(())
}

pub fn validate_coarena<'a>(
    record: CoarenaRecord,
    arena: &'a ValidatedArena,
    budget: &mut Budget,
) -> Result<ValidatedCoarena<'a>> {
    if record.identity != arena.record().identity {
        return Err(Error::Identity);
    }
    coarena_length(record.rows.len(), arena)?;
    budget.charge(record.rows.len())?;
    let mut seen = vector::<u8>(record.rows.len())?;
    seen.resize(record.rows.len(), 0);
    for (index, item) in record.rows.iter().enumerate() {
        let row = item.row as usize;
        if row >= seen.len() || seen[row] != 0 {
            return Err(Error::Ownership);
        }
        seen[row] = 1;
        if item.generation != u32::MAX {
            if item.generation > arena.record().generation {
                return Err(Error::Generation);
            }
            if let ArenaSlot::Occupied { generation, .. } = arena.record().slots[index]
                && item.generation > generation
            {
                return Err(Error::Generation);
            }
        }
    }
    Ok(ValidatedCoarena { record, arena })
}
