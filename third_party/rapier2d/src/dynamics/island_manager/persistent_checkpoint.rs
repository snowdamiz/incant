//! Exact private island construction from the joined immutable rigid witness.
//! No public native owner, phase authentication or save installer is exposed.
use super::{ContactLink, INVALID_LOC, PersistentIsland, PersistentIslands};
use crate::alloc_prelude::*;
use crate::dynamics::island_manager::Island;
use crate::dynamics::{IslandManager, RigidBodyHandle};
use crate::pipeline::checked_restore::allocation::*;
use incant_physics_snapshot::{Handle, Identity, IslandTable, RecordSpan, ValidatedRigidOwner};
use parry::utils::VecMap;

fn check(owner: &ValidatedRigidOwner<'_>, expected: Identity) -> ConstructionResult<()> {
    let r = owner.islands().record();
    let s = owner.record();
    if r.identity != expected
        || s.identity != expected
        || expected.dimension as usize != crate::math::DIM
    {
        return Err(ConstructionError::Payload);
    }
    // Already enforced by the immutable witnesses; retain the local boundary.
    if !r.bootstrapped
        || r.pending_removals != 0
        || r.joint_links != 0
        || r.joint_locations != 0
        || s.sleep_scan_touched != 0
        || r.sleep_scan_stamp != s.sleep_scan_stamp
    {
        return Err(ConstructionError::Payload);
    }
    Ok(())
}
fn handle(h: Handle) -> RigidBodyHandle {
    RigidBodyHandle::from_raw_parts(h.slot, h.generation)
}
fn span<T>(rows: &[T], s: RecordSpan) -> ConstructionResult<&[T]> {
    let end = s
        .start
        .checked_add(s.count)
        .ok_or(ConstructionError::Limit)?;
    rows.get(s.start as usize..end as usize)
        .ok_or(ConstructionError::Payload)
}
fn handles(
    rows: &[Handle],
    permit: &mut AllocationPermit,
) -> ConstructionResult<Vec<RigidBodyHandle>> {
    let mut out = permit.vector(rows.len())?;
    out.extend(rows.iter().copied().map(handle));
    Ok(out)
}
/// VecMap has only infallible allocation APIs. Charge its exact Option<T> Vec
/// request before allocation; ordinary allocator OOM can still abort, as for the
/// already admitted Box/Arc payloads. No temporary heap storage is introduced.
fn slots<T: Default>(count: usize, permit: &mut AllocationPermit) -> ConstructionResult<VecMap<T>> {
    permit.charge(AllocationPlan::array::<Option<T>>(count)?)?;
    let mut out = VecMap::with_capacity(count);
    if out.capacity() != count {
        return Err(ConstructionError::Allocation);
    }
    if count != 0 {
        out.insert(count - 1, T::default());
        out.remove(count - 1);
    }
    Ok(out)
}
impl IslandManager {
    pub(crate) fn checkpoint_plan(
        owner: &ValidatedRigidOwner<'_>,
        expected: Identity,
    ) -> ConstructionResult<AllocationPlan> {
        check(owner, expected)?;
        let r = owner.islands().record();
        let mut p = AllocationPlan::array::<Option<Island>>(r.active.len())?;
        p.append(AllocationPlan::array::<usize>(r.active_free.len())?)?;
        for s in r.active.iter().flatten() {
            p.append(AllocationPlan::array::<RigidBodyHandle>(s.count as usize)?)?;
        }
        p.append(AllocationPlan::array::<Option<PersistentIsland>>(
            r.persistent.len(),
        )?)?;
        p.append(AllocationPlan::array::<u32>(r.persistent_free.len())?)?;
        for s in r.persistent.iter().flatten() {
            p.append(AllocationPlan::array::<RigidBodyHandle>(
                s.bodies.count as usize,
            )?)?;
            p.append(AllocationPlan::array::<ContactLink>(
                s.contacts.count as usize,
            )?)?;
        }
        p.append(AllocationPlan::array::<(u32, u32)>(r.locations.len())?)?;
        p.append(AllocationPlan::array::<(u32, bool)>(
            owner.record().sleep_scan.len(),
        )?)?;
        Ok(p)
    }
    pub(crate) fn from_checkpoint(
        owner: &ValidatedRigidOwner<'_>,
        expected: Identity,
        permit: &mut AllocationPermit,
    ) -> ConstructionResult<Self> {
        check(owner, expected)?;
        let r = owner.islands().record();
        let mut islands = slots::<Island>(r.active.len(), permit)?;
        for (i, s) in r.active.iter().enumerate() {
            if let Some(s) = s {
                islands.insert(
                    i,
                    Island {
                        bodies: handles(span(&r.active_bodies, *s)?, permit)?,
                    },
                );
            }
        }
        let mut free_islands = permit.vector(r.active_free.len())?;
        free_islands.extend(r.active_free.iter().map(|i| *i as usize));
        Ok(Self {
            active_set_epoch: r.active_epoch,
            islands,
            awake_island: r.awake.map(|i| i as usize),
            free_islands,
            // update_substep_groups clears groups, resets union-find and clears
            // per-key scatter buffers before reading. See native source evidence.
            solve_groups: Vec::new(),
            substep_groups_workspace: Default::default(),
            persistent: persistent(r, owner, permit)?,
        })
    }
}
fn persistent(
    r: &IslandTable,
    owner: &ValidatedRigidOwner<'_>,
    permit: &mut AllocationPermit,
) -> ConstructionResult<PersistentIslands> {
    let mut islands = slots::<PersistentIsland>(r.persistent.len(), permit)?;
    for (i, row) in r.persistent.iter().enumerate() {
        if let Some(row) = row {
            let mut contact_links = permit.vector(row.contacts.count as usize)?;
            contact_links.extend(
                span(&r.contacts, row.contacts)?
                    .iter()
                    .map(|c| ContactLink {
                        edge_id: c.edge,
                        body1: handle(c.bodies[0]),
                        body2: handle(c.bodies[1]),
                    }),
            );
            islands.insert(
                i,
                PersistentIsland {
                    bodies: handles(span(&r.persistent_bodies, row.bodies)?, permit)?,
                    contact_links,
                    joint_links: Vec::new(),
                    constraint_remove_count: row.constraint_remove_count,
                    split_denied_until: row.split_denied_until,
                    sleeping: row.sleeping,
                },
            );
        }
    }
    let mut free_islands = permit.vector(r.persistent_free.len())?;
    free_islands.extend_from_slice(&r.persistent_free);
    let mut contact_link_locs = permit.vector(r.locations.len())?;
    contact_link_locs.extend(
        r.locations
            .iter()
            .map(|v| v.map_or(INVALID_LOC, |v| (v.island, v.index))),
    );
    let mut sleep_scan = permit.vector(owner.record().sleep_scan.len())?;
    sleep_scan.extend(
        owner
            .record()
            .sleep_scan
            .iter()
            .map(|v| (v.stamp, v.eligible)),
    );
    Ok(PersistentIslands {
        islands,
        free_islands,
        contact_link_locs,
        joint_link_locs: Default::default(),
        removal_journal: Vec::new(),
        // Local search increments its stamp and clears both frontier/reached
        // sets; wrap clears the visited map before any matching read.
        local_split: Default::default(),
        split_island: r.pending_split,
        // Global split resets union-find/counts, increments the node-map stamp
        // (clearing on wrap), then writes each current member before lookup.
        split_workspace: Default::default(),
        sleep_scan,
        sleep_scan_touched: Vec::new(),
        sleep_scan_stamp: r.sleep_scan_stamp,
        bootstrapped: r.bootstrapped,
    })
}

#[cfg(all(test, feature = "serde-serialize"))]
mod tests {
    use super::*;
    #[test]
    fn checked_islands_vecmap_temporary_occupancy_preserves_empty_tail() {
        for count in [0, 1, 7] {
            let plan = AllocationPlan::array::<Option<Island>>(count).unwrap();
            let mut permit = AllocationBudget::new(plan.bytes(), plan.allocations())
                .admit(plan)
                .unwrap();
            let mut map = slots::<Island>(count, &mut permit).unwrap();
            permit.finish().unwrap();
            let v = serde_json::to_value(&map).unwrap();
            assert_eq!(v["n"], 0);
            assert_eq!(v["v"].as_array().unwrap().len(), count);
            assert!(
                v["v"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(serde_json::Value::is_null)
            );
            if count > 0 {
                map.insert(0, Island::default());
                map.remove(0).unwrap();
                assert_eq!(serde_json::to_value(&map).unwrap(), v);
                assert_eq!(map.capacity(), count);
            }
        }
    }
}

#[path = "persistent_capture.rs"]
mod capture;
