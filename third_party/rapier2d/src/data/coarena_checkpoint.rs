//! Preserve stale and invalidated coarena payloads; never default-fill saved rows.
use super::*;
use crate::pipeline::checked_restore::allocation::*;
use incant_physics_snapshot::ValidatedCoarena;
impl<T> Coarena<T> {
    pub(crate) fn checkpoint_rows(&self) -> &[(u32, T)] {
        &self.data
    }
    pub(crate) fn checkpoint_capture(
        &self,
        identity: incant_physics_snapshot::Identity,
        budget: &mut incant_physics_snapshot::Budget,
    ) -> incant_physics_snapshot::Result<incant_physics_snapshot::CoarenaRecord> {
        use incant_physics_snapshot::{CoarenaRecord, CoarenaRow, Error, MAX_ITEMS};
        if self.data.len() > MAX_ITEMS {
            return Err(Error::Limit);
        }
        let mut rows = budget.vector(self.data.len())?;
        rows.extend(
            self.data
                .iter()
                .enumerate()
                .map(|(i, (generation, _))| CoarenaRow {
                    generation: *generation,
                    row: i as u32,
                }),
        );
        Ok(CoarenaRecord { identity, rows })
    }

    pub(crate) fn checkpoint_plan(
        value: &ValidatedCoarena<'_>,
    ) -> ConstructionResult<AllocationPlan> {
        AllocationPlan::array::<(u32, T)>(value.record().rows.len())
    }
    pub(crate) fn from_checkpoint(
        value: &ValidatedCoarena<'_>,
        permit: &mut AllocationPermit,
        mut payload: impl FnMut(u32, &mut AllocationPermit) -> ConstructionResult<T>,
    ) -> ConstructionResult<Self> {
        let mut data = permit.vector(value.record().rows.len())?;
        for r in &value.record().rows {
            data.push((r.generation, payload(r.row, permit)?));
        }
        Ok(Self { data })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Arena;
    use incant_physics_snapshot::*;
    #[test]
    fn checked_storage_coarena_decoded_stale_invalidated_payloads_continue() {
        let mut arena = Arena::with_capacity(4);
        let handles: Vec<_> = (0..4).map(|i| arena.insert(i)).collect();
        let mut a = Coarena::new();
        for (i, h) in handles.iter().enumerate() {
            a.insert(*h, 10 + i as u32);
        }
        a.remove(handles[1], 91);
        arena.remove(handles[1]);
        arena.remove(handles[2]);
        let identity = Identity {
            dimension: if crate::math::DIM == 2 {
                Dimension::Planar
            } else {
                Dimension::Spatial
            },
            build: [7; 32],
        };
        // Actual public arena handles establish exact generations and free order.
        let r = ArenaRecord {
            identity,
            generation: 2,
            free_head: Some(2),
            live: 2,
            slots: vec![
                ArenaSlot::Occupied {
                    generation: 0,
                    row: 0,
                },
                ArenaSlot::Free { next: None },
                ArenaSlot::Free { next: Some(1) },
                ArenaSlot::Occupied {
                    generation: 0,
                    row: 1,
                },
            ],
        };
        let mut budget = Budget::new(65536).unwrap();
        let av = validate_arena(r, &mut budget).unwrap();
        let r = CoarenaRecord {
            identity,
            rows: a
                .data
                .iter()
                .enumerate()
                .map(|(i, (generation, _))| CoarenaRow {
                    generation: *generation,
                    row: i as u32,
                })
                .collect(),
        };
        let v = validate_coarena(r, &av, &mut budget).unwrap();
        let bytes = encode_coarena(&v, &mut budget).unwrap();
        let v = decode_coarena(&bytes, &av, &mut budget).unwrap();
        assert_eq!(v.state(1), Some(RowState::Invalidated));
        assert_eq!(v.state(2), Some(RowState::Stale));
        let plan = Coarena::<u32>::checkpoint_plan(&v).unwrap();
        let mut p = AllocationBudget::new(plan.bytes(), plan.allocations())
            .admit(plan)
            .unwrap();
        let mut b =
            Coarena::from_checkpoint(&v, &mut p, |row, _| Ok(a.data[row as usize].1)).unwrap();
        p.finish().unwrap();
        assert_eq!(a.data, b.data);
        assert_eq!(b.get(Index::from_raw_parts(1, u32::MAX)), Some(&91));
        assert_eq!(b.get(handles[2]), Some(&12));
        let reused = arena.insert(77);
        assert_eq!(reused.into_raw_parts(), (2, 2));
        a.insert(reused, 88);
        b.insert(reused, 88);
        assert_eq!(a.data, b.data);
        assert_eq!(a.remove(handles[2], 0), b.remove(handles[2], 0));
        assert_eq!(a.remove(reused, 99), b.remove(reused, 99));
        assert_eq!(a.data, b.data);
        *a.ensure_element_exist(Index::from_raw_parts(3, 2), 55) += 1;
        *b.ensure_element_exist(Index::from_raw_parts(3, 2), 55) += 1;
        assert_eq!(a.data, b.data);
    }
}

impl<T> Coarena<T> {
    pub(crate) fn checkpoint_no_rows(&self) -> bool {
        self.data.is_empty()
    }
}
