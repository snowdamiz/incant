//! Private exact arena storage construction, after independent inert validation.
use super::*;
use crate::pipeline::checked_restore::allocation::*;
use incant_physics_snapshot::{ArenaSlot, ValidatedArena};
impl<T> Arena<T> {
    pub(crate) fn checkpoint_capture(
        &self,
        identity: incant_physics_snapshot::Identity,
        budget: &mut incant_physics_snapshot::Budget,
    ) -> incant_physics_snapshot::Result<incant_physics_snapshot::ArenaRecord> {
        use incant_physics_snapshot::{ArenaRecord, Error, MAX_ITEMS};
        if self.items.len() > MAX_ITEMS || self.len > MAX_ITEMS {
            return Err(Error::Limit);
        }
        let mut slots = budget.vector(self.items.len())?;
        let mut row = 0u32;
        for item in &self.items {
            slots.push(match item {
                Entry::Free { next_free } => ArenaSlot::Free { next: *next_free },
                Entry::Occupied { generation, .. } => {
                    let value = ArenaSlot::Occupied {
                        generation: *generation,
                        row,
                    };
                    row += 1;
                    value
                }
            });
        }
        Ok(ArenaRecord {
            identity,
            generation: self.generation,
            free_head: self.free_list_head,
            live: self.len as u32,
            slots,
        })
    }

    pub(crate) fn checkpoint_slot_count(&self) -> usize {
        self.items.len()
    }
    pub(crate) fn checkpoint_plan(value: &ValidatedArena) -> ConstructionResult<AllocationPlan> {
        // A later runtime owner must also fence every future generation-changing
        // operation. Do not construct the already-next-removal sentinel boundary.
        if value.record().generation >= u32::MAX - 1 {
            return Err(ConstructionError::Generation);
        }
        AllocationPlan::array::<Entry<T>>(value.record().slots.len())
    }
    pub(crate) fn from_checkpoint(
        value: &ValidatedArena,
        permit: &mut AllocationPermit,
        mut payload: impl FnMut(u32, &mut AllocationPermit) -> ConstructionResult<T>,
    ) -> ConstructionResult<Self> {
        Self::checkpoint_plan(value)?;
        let r = value.record();
        let mut items = permit.vector(r.slots.len())?;
        for slot in &r.slots {
            items.push(match *slot {
                ArenaSlot::Free { next } => Entry::Free { next_free: next },
                ArenaSlot::Occupied { generation, row } => Entry::Occupied {
                    generation,
                    value: payload(row, permit)?,
                },
            });
        }
        Ok(Self {
            items,
            generation: r.generation,
            free_list_head: r.free_head,
            len: r.live as usize,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use incant_physics_snapshot::{
        ArenaRecord, Budget, Dimension, Identity, decode_arena, encode_arena, validate_arena,
    };
    fn identity() -> Identity {
        Identity {
            dimension: if crate::math::DIM == 2 {
                Dimension::Planar
            } else {
                Dimension::Spatial
            },
            build: [7; 32],
        }
    }
    fn captured(a: &Arena<u32>) -> (incant_physics_snapshot::ValidatedArena, Vec<u32>) {
        let mut payload = Vec::new();
        let slots = a
            .items
            .iter()
            .map(|s| match s {
                Entry::Free { next_free } => ArenaSlot::Free { next: *next_free },
                Entry::Occupied { generation, value } => {
                    let row = payload.len() as u32;
                    payload.push(*value);
                    ArenaSlot::Occupied {
                        generation: *generation,
                        row,
                    }
                }
            })
            .collect();
        let r = ArenaRecord {
            identity: identity(),
            generation: a.generation,
            free_head: a.free_list_head,
            live: a.len as u32,
            slots,
        };
        let mut b = Budget::new(65536).unwrap();
        let v = validate_arena(r, &mut b).unwrap();
        let bytes = encode_arena(&v, &mut b).unwrap();
        (decode_arena(&bytes, identity(), &mut b).unwrap(), payload)
    }
    fn restore(a: &Arena<u32>) -> Arena<u32> {
        let (v, p) = captured(a);
        let plan = Arena::<u32>::checkpoint_plan(&v).unwrap();
        let mut permit = AllocationBudget::new(plan.bytes(), plan.allocations())
            .admit(plan)
            .unwrap();
        let b = Arena::from_checkpoint(&v, &mut permit, |row, _| {
            p.get(row as usize)
                .copied()
                .ok_or(ConstructionError::Payload)
        })
        .unwrap();
        permit.finish().unwrap();
        b
    }
    fn equal(a: &Arena<u32>, b: &Arena<u32>) {
        assert_eq!(captured(a).0.record(), captured(b).0.record());
        assert_eq!(captured(a).1, captured(b).1);
    }
    #[test]
    fn checked_storage_arena_decoded_generations_free_order_and_growth_continue() {
        for full in [false, true] {
            let mut a = Arena::with_capacity(4);
            let ids: Vec<_> = (0..4).map(|i| a.insert(i)).collect();
            if !full {
                a.remove(ids[1]);
                a.remove(ids[3]);
            }
            let mut b = restore(&a);
            equal(&a, &b);
            for i in 10..26 {
                assert_eq!(a.insert(i), b.insert(i));
                equal(&a, &b);
            }
            for id in ids {
                assert_eq!(a.remove(id), b.remove(id));
                equal(&a, &b);
            }
            a.clear();
            b.clear();
            equal(&a, &b);
            for i in 30..38 {
                assert_eq!(a.insert(i), b.insert(i));
                equal(&a, &b);
            }
        }
    }
    #[test]
    fn checked_storage_arena_refuses_exhaustion_and_payload_failure() {
        let mut a = Arena::with_capacity(2);
        a.insert(4);
        a.insert(9);
        let (v, _) = captured(&a);
        let plan = Arena::<u32>::checkpoint_plan(&v).unwrap();
        let mut permit = AllocationBudget::new(plan.bytes(), 1).admit(plan).unwrap();
        let mut calls = 0;
        assert!(matches!(
            Arena::<u32>::from_checkpoint(&v, &mut permit, |_, _| {
                calls += 1;
                Err(ConstructionError::Payload)
            }),
            Err(ConstructionError::Payload)
        ));
        assert_eq!(calls, 1);
        let mut r = v.record().clone();
        r.generation = u32::MAX - 1;
        let v = validate_arena(r, &mut Budget::new(4096).unwrap()).unwrap();
        assert!(matches!(
            Arena::<u32>::checkpoint_plan(&v),
            Err(ConstructionError::Generation)
        ));
        let mut corrupt = v.record().clone();
        corrupt.slots[1] = corrupt.slots[0];
        assert!(validate_arena(corrupt, &mut Budget::new(4096).unwrap()).is_err());
    }
    #[test]
    fn checked_storage_late_failure_drops_candidates_without_refunding_budget() {
        use core::cell::Cell;
        use std::rc::Rc;
        struct Payload(Rc<Cell<usize>>);
        impl Drop for Payload {
            fn drop(&mut self) {
                self.0.set(self.0.get() + 1);
            }
        }
        let mut a = Arena::with_capacity(2);
        a.insert(7);
        a.insert(11);
        let (v, _) = captured(&a);
        let plan = Arena::<Payload>::checkpoint_plan(&v).unwrap();
        let mut budget = AllocationBudget::new(plan.bytes(), plan.allocations());
        let mut permit = budget.admit(plan).unwrap();
        let drops = Rc::new(Cell::new(0));
        let result = Arena::<Payload>::from_checkpoint(&v, &mut permit, |row, _| {
            if row == 1 {
                Err(ConstructionError::Payload)
            } else {
                Ok(Payload(Rc::clone(&drops)))
            }
        });
        assert!(matches!(result, Err(ConstructionError::Payload)));
        assert_eq!(drops.get(), 1);
        assert!(budget.admit(plan).is_err());
        assert_eq!(a.len(), 2);
        assert_eq!(a.iter().map(|(_, v)| *v).collect::<Vec<_>>(), [7, 11]);
    }
}

impl<T> Arena<T> {
    // Exact native new() storage, not merely len()==0. Used only for the
    // explicitly unsupported joint/soft families, which must have no history.
    pub(crate) fn checkpoint_pristine_empty(&self) -> bool {
        self.len == 0
            && self.generation == 0
            && self.free_list_head == Some(0)
            && self.items.len() == DEFAULT_CAPACITY
            && self.items.iter().enumerate().all(|(i, e)| {
                matches!(e,
                Entry::Free { next_free } if *next_free ==
                    (i + 1 < DEFAULT_CAPACITY).then_some((i + 1) as u32))
            })
    }
    pub(crate) fn checkpoint_empty_plan() -> ConstructionResult<AllocationPlan> {
        AllocationPlan::array::<Entry<T>>(DEFAULT_CAPACITY)
    }
    pub(crate) fn checkpoint_empty(permit: &mut AllocationPermit) -> ConstructionResult<Self> {
        let mut items = permit.vector(DEFAULT_CAPACITY)?;
        for i in 0..DEFAULT_CAPACITY {
            items.push(Entry::Free {
                next_free: (i + 1 < DEFAULT_CAPACITY).then_some((i + 1) as u32),
            });
        }
        Ok(Self {
            items,
            generation: 0,
            free_list_head: Some(0),
            len: 0,
        })
    }
}
