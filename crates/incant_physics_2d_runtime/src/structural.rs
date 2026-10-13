//! Cross-dimension callers keep command storage and detached plans until every
//! owner admits. Native execution spends no additional validation budget.
#[cfg(feature = "native-checkpoints")]
mod checked {
    use super::super::*;
    use rapier2d::pipeline::checked_restore::capture::{
        StructuralError, StructuralInsertions, StructuralPlan, StructuralRemoval,
    };
    pub struct StructuralChanges {
        updates: Vec<(StableId, Option<PreparedBody>)>,
        observed: Vec<Option<(Option<RigidBodyHandle>, ColliderHandle)>>,
        removals: Vec<StructuralRemoval>,
        removed_ids: BTreeMap<(u8, u32, u32), StableId>,
        rebuild: BTreeSet<StableId>,
        insertions: StructuralInsertions,
    }
    pub struct StructuralAdmission<'p> {
        changes: &'p StructuralChanges,
        plan: StructuralPlan<'p>,
    }
    fn same(old: Option<&Entry>, next: &PreparedBody) -> bool {
        old.is_some_and(|o| {
            o.config.body == next.body
                && o.config.collider == next.collider
                && o.config.geometry == next.geometry
        })
    }
    impl PhysicsWorld2D {
        pub fn prepare_structure(
            &self,
            updates: Vec<(StableId, Option<PreparedBody>)>,
        ) -> Result<StructuralChanges, PhysicsError> {
            self.validate_changes(&updates)?;
            let mut c = StructuralChanges {
                updates,
                observed: Vec::new(),
                removals: Vec::new(),
                removed_ids: BTreeMap::new(),
                rebuild: BTreeSet::new(),
                insertions: StructuralInsertions::default(),
            };
            for (id, next) in &c.updates {
                let old = self.entries.get(id);
                c.observed.push(old.map(|e| (e.body, e.collider)));
                let rebuild = next.as_ref().is_some_and(|n| !same(old, n));
                if (rebuild || next.is_none())
                    && let Some(old) = old
                {
                    let r = old.body.map_or(
                        StructuralRemoval::Collider(old.collider),
                        StructuralRemoval::Body,
                    );
                    c.removals.push(r);
                    c.removed_ids.insert(r.key(), *id);
                }
                if rebuild {
                    c.rebuild.insert(*id);
                    c.insertions.bodies += usize::from(next.as_ref().unwrap().body.is_some());
                    c.insertions.colliders += 1;
                }
            }
            c.removals.sort_unstable_by_key(|r| r.key());
            Ok(c)
        }
        pub fn admit_structure<'p>(
            &self,
            changes: &'p StructuralChanges,
            work: &mut CheckpointWorkBudget,
        ) -> Result<StructuralAdmission<'p>, PhysicsError> {
            self.validate_changes(&changes.updates)?;
            for ((id, next), observed) in changes.updates.iter().zip(&changes.observed) {
                let old = self.entries.get(id);
                if old.map(|e| (e.body, e.collider)) != *observed
                    || next.as_ref().is_some_and(|n| !same(old, n)) != changes.rebuild.contains(id)
                {
                    return Err(StructuralError::Plan.into());
                }
            }
            let numeric = changes
                .updates
                .iter()
                .filter(|(id, next)| next.is_some() && !changes.rebuild.contains(id))
                .count();
            let revisions = changes
                .removals
                .len()
                .checked_add(changes.insertions.bodies)
                .and_then(|n| n.checked_add(changes.insertions.colliders))
                .and_then(|n| {
                    numeric
                        .checked_mul(2)
                        .and_then(|edits| n.checked_add(edits))
                })
                .ok_or(PhysicsError::Limit)?;
            self.world
                .inner
                .preflight_revision_edits(revisions as u64)
                .map_err(StructuralError::Owner)?;
            let plan = self.world.inner.preflight_structure(
                &changes.removals,
                changes.insertions,
                work,
            )?;
            Ok(StructuralAdmission { changes, plan })
        }
        pub fn apply_structure(
            &mut self,
            admission: StructuralAdmission<'_>,
        ) -> Result<(), PhysicsError> {
            self.check_live()?;
            let c = admission.changes;
            let mut permit = self.world.inner.structural_permit(admission.plan)?;
            for r in &c.removals {
                match *r {
                    StructuralRemoval::Body(h) => {
                        permit.remove_body(h)?;
                    }
                    StructuralRemoval::Collider(h) => {
                        permit.remove_collider(h)?;
                    }
                }
                let old = self
                    .entries
                    .remove(&c.removed_ids[&r.key()])
                    .expect("admitted outer handle");
                self.ids.remove(&old.collider);
            }
            for (id, next) in &c.updates {
                if !c.rebuild.contains(id) {
                    continue;
                }
                let mut config = next.as_ref().unwrap().clone();
                let (body, collider) = native_parts(&config);
                config.state.sleeping = body.as_ref().is_some_and(RigidBody::is_sleeping);
                let body = body.map(|b| permit.insert_body(b)).transpose()?;
                let collider = permit.insert_collider(collider, body)?;
                self.ids.insert(collider, *id);
                self.entries.insert(
                    *id,
                    Entry {
                        body,
                        collider,
                        config,
                    },
                );
            }
            permit.finish()?;
            self.query_dirty |= !c.removals.is_empty() || !c.rebuild.is_empty();
            for (id, next) in &c.updates {
                if let Some(next) = next
                    && !c.rebuild.contains(id)
                {
                    self.update_motion(*id, &next.state);
                }
            }
            Ok(())
        }
    }
    fn native_parts(c: &PreparedBody) -> (Option<RigidBody>, Collider) {
        let shape = ColliderBuilder::new(c.shape())
            .density(c.collider.density as f32)
            .friction(c.collider.friction as f32)
            .restitution(c.collider.restitution as f32)
            .sensor(c.collider.sensor)
            .collision_groups(groups(c.collider.memberships, c.collider.filter))
            .active_collision_types(ActiveCollisionTypes::all());
        if let Some(p) = &c.body {
            let b = match p.motion {
                BodyMotion::Fixed => RigidBodyBuilder::fixed(),
                BodyMotion::Dynamic => RigidBodyBuilder::dynamic(),
                BodyMotion::Kinematic => RigidBodyBuilder::kinematic_velocity_based(),
            }
            .pose(pose(&c.state))
            .linvel(vector(c.state.velocity))
            .angvel(c.state.angular_velocity as f32)
            .gravity_scale(p.gravity_scale as f32)
            .linear_damping(p.linear_damping as f32)
            .angular_damping(p.angular_damping as f32)
            .can_sleep(p.can_sleep)
            .ccd_enabled(p.ccd)
            .build();
            (Some(b), shape.build())
        } else {
            (None, shape.position(pose(&c.state)).build())
        }
    }
}
#[cfg(feature = "native-checkpoints")]
pub use checked::{StructuralAdmission, StructuralChanges};
