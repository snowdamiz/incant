//! Exact native slot/order observation. Validation remains a separate step.
use super::*;
use crate::pipeline::checked_restore::capture::util::{copy, count, handle, map, total};
use incant_physics_snapshot::{self as s, Budget, Error};
impl IslandManager {
    /// Exact count of already queued unlinks; observing never drains the journal.
    pub(crate) fn checkpoint_pending_removal_count(&self) -> usize {
        self.persistent.removal_journal.len()
    }

    pub(crate) fn checkpoint_capture(
        &self,
        identity: Identity,
        budget: &mut Budget,
    ) -> s::Result<(IslandTable, Vec<s::SleepScanSlot>, u32)> {
        let p = &self.persistent;
        let (active, active_live) = self.islands.checkpoint_slots();
        let (persistent, persistent_live) = p.islands.checkpoint_slots();
        for n in [active.len(), persistent.len(), active_live, persistent_live] {
            count(n)?;
        }
        let active_count = total(active.iter().flatten().map(|x| x.bodies.len()))?;
        let persistent_count = total(persistent.iter().flatten().map(|x| x.bodies.len()))?;
        let contacts_count = total(persistent.iter().flatten().map(|x| x.contact_links.len()))?;
        let joint_links = count(total(
            persistent.iter().flatten().map(|x| x.joint_links.len()),
        )?)?;
        let mut active_rows = budget.vector(active.len())?;
        let mut active_bodies = budget.vector(active_count)?;
        for slot in active {
            active_rows.push(if let Some(row) = slot {
                let span = RecordSpan {
                    start: active_bodies.len() as u32,
                    count: row.bodies.len() as u32,
                };
                active_bodies.extend(row.bodies.iter().map(|h| handle(h.into_raw_parts())));
                Some(span)
            } else {
                None
            });
        }
        let mut persistent_rows = budget.vector(persistent.len())?;
        let mut persistent_bodies = budget.vector(persistent_count)?;
        let mut contacts = budget.vector(contacts_count)?;
        for slot in persistent {
            persistent_rows.push(if let Some(row) = slot {
                let bodies = RecordSpan {
                    start: persistent_bodies.len() as u32,
                    count: row.bodies.len() as u32,
                };
                persistent_bodies.extend(row.bodies.iter().map(|h| handle(h.into_raw_parts())));
                let span = RecordSpan {
                    start: contacts.len() as u32,
                    count: row.contact_links.len() as u32,
                };
                contacts.extend(row.contact_links.iter().map(|v| s::IslandContact {
                    edge: v.edge_id,
                    bodies: [
                        handle(v.body1.into_raw_parts()),
                        handle(v.body2.into_raw_parts()),
                    ],
                }));
                Some(s::PersistentIslandRow {
                    bodies,
                    contacts: span,
                    constraint_remove_count: row.constraint_remove_count,
                    split_denied_until: row.split_denied_until,
                    sleeping: row.sleeping,
                })
            } else {
                None
            });
        }
        count(self.free_islands.len())?;
        let mut active_free = budget.vector(self.free_islands.len())?;
        for n in &self.free_islands {
            active_free.push(u32::try_from(*n).map_err(|_| Error::Limit)?);
        }
        let locations = map(&p.contact_link_locs, budget, |v| {
            if *v == INVALID_LOC {
                None
            } else {
                Some(s::IslandLocation {
                    island: v.0,
                    index: v.1,
                })
            }
        })?;
        let scan = map(&p.sleep_scan, budget, |v| s::SleepScanSlot {
            stamp: v.0,
            eligible: v.1,
        })?;
        Ok((
            IslandTable {
                identity,
                active_epoch: self.active_set_epoch,
                awake: self
                    .awake_island
                    .map(|i| u32::try_from(i).map_err(|_| Error::Limit))
                    .transpose()?,
                sleep_scan_stamp: p.sleep_scan_stamp,
                pending_split: p.split_island,
                bootstrapped: p.bootstrapped,
                pending_removals: count(p.removal_journal.len())?,
                joint_links,
                joint_locations: count(p.joint_link_locs.len())?,
                active_live: active_live as u32,
                persistent_live: persistent_live as u32,
                active: active_rows,
                active_free,
                active_bodies,
                persistent: persistent_rows,
                persistent_free: copy(&p.free_islands, budget)?,
                persistent_bodies,
                contacts,
                locations,
            },
            scan,
            count(p.sleep_scan_touched.len())?,
        ))
    }
}
