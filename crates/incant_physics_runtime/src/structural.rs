//! Structural transactions admit every affected native owner before executing any
//! edit. Detached plans borrow bounded command storage, never a live world.
use super::*;

impl<I: Ord + Clone> PhysicsRuntime<I> {
    /// Build a new trusted owner from checked declarations. This never edits or
    /// resets an existing/imported owner. Fresh arenas have no retained hostile
    /// counters, so the simulation's existing one-million declaration bound is
    /// preserved independently of the stricter snapshot format limit.
    pub fn from_prepared_bodies(
        scene: I,
        bodies: Vec<(I, PreparedBody)>,
    ) -> Result<Self, PhysicsError> {
        if bodies.len() > 1_000_000 {
            return Err(PhysicsError::Limit);
        }
        let mut seen = BTreeSet::new();
        for (id, _) in &bodies {
            if !seen.insert(id) {
                return Err(PhysicsError::Limit);
            }
        }
        let mut result = Self::default();
        if !bodies.is_empty() {
            let mut owner = SceneWorld::default();
            for (id, body) in bodies {
                owner.insert(id, body.0);
            }
            owner.query_dirty = true;
            result.scenes.insert(scene, owner);
        }
        Ok(result)
    }

    pub fn validate_changes(
        &self,
        changes: &[(I, I, Option<PreparedBody>)],
    ) -> Result<(), PhysicsError> {
        if self.failed {
            return Err(PhysicsError::NumericRange);
        }
        if changes.len() > 2_000_000 {
            return Err(PhysicsError::Limit);
        }
        let mut seen = BTreeSet::new();
        let mut count: usize = self.scenes.values().map(|s| s.entries.len()).sum();
        for (scene, id, next) in changes {
            if !seen.insert((scene, id)) {
                return Err(PhysicsError::Limit);
            }
            if self
                .scenes
                .get(scene)
                .is_some_and(|s| s.entries.contains_key(id))
            {
                count -= 1;
            }
            if next.is_some() {
                count = count.checked_add(1).ok_or(PhysicsError::Limit)?;
            }
        }
        if count > 1_000_000 {
            return Err(PhysicsError::Limit);
        }
        Ok(())
    }

    /// A bounded batch, including removals, is admitted before any map or solver
    /// changes. Rejected generation/capacity checks do not poison the runtime.
    pub fn apply_changes(
        &mut self,
        changes: Vec<(I, I, Option<PreparedBody>)>,
    ) -> Result<(), PhysicsError> {
        self.validate_changes(&changes)?;
        #[cfg(feature = "native-checkpoints")]
        {
            let changes = self.prepare_structure(changes)?;
            let mut work = CheckpointWorkBudget::new(64_000_000);
            let admitted = self.admit_structure(&changes, &mut work)?;
            self.apply_structure(admitted)?;
        }
        #[cfg(not(feature = "native-checkpoints"))]
        for (scene_id, id, next) in changes {
            if let Some(next) = next {
                let scene = self.scenes.entry(scene_id).or_default();
                if same(scene.entries.get(&id), &next.0) {
                    scene.update_motion(&id, &next.0.state)?;
                } else {
                    scene.remove(&id);
                    scene.insert(id, next.0);
                    scene.query_dirty = true;
                }
            } else if let Some(scene) = self.scenes.get_mut(&scene_id) {
                scene.remove(&id);
                scene.query_dirty = true;
            }
        }
        Ok(())
    }
}
fn same(old: Option<&Entry>, next: &PhysicsSpec) -> bool {
    old.is_some_and(|e| e.config.body == next.body && e.config.collider == next.collider)
}

#[cfg(feature = "native-checkpoints")]
mod checked {
    use super::*;
    use rapier3d::pipeline::checked_restore::capture::{
        StructuralError, StructuralInsertions, StructuralPermit, StructuralPlan, StructuralRemoval,
    };
    struct SceneChanges<I: Ord> {
        updates: BTreeMap<I, Option<PreparedBody>>,
        rebuild: BTreeSet<I>,
        observed: BTreeMap<I, Option<(Option<RigidBodyHandle>, ColliderHandle)>>,
        removals: Vec<StructuralRemoval>,
        removed_ids: BTreeMap<(u8, u32, u32), I>,
        insertions: StructuralInsertions,
    }
    /// Owned command storage. This is not an admission token and contains no
    /// mutable native references. Only structural declarations are copied.
    pub struct StructuralChanges<I: Ord> {
        scenes: BTreeMap<I, SceneChanges<I>>,
    }
    /// Exact existing-owner revisions plus detached, fully prepared new scenes.
    /// All planned input storage must remain alive until commit finishes.
    pub struct StructuralAdmission<'p, I: Ord> {
        changes: &'p StructuralChanges<I>,
        plans: BTreeMap<I, StructuralPlan<'p>>,
        fresh: BTreeMap<I, SceneWorld<I>>,
        before_count: usize,
    }
    impl<I: Ord + Clone> PhysicsRuntime<I> {
        pub fn prepare_structure(
            &self,
            changes: Vec<(I, I, Option<PreparedBody>)>,
        ) -> Result<StructuralChanges<I>, PhysicsError> {
            self.validate_changes(&changes)?;
            let mut scenes: BTreeMap<I, SceneChanges<I>> = BTreeMap::new();
            for (scene, id, next) in changes {
                let old = self.scenes.get(&scene).and_then(|s| s.entries.get(&id));
                let rebuild = next.as_ref().is_some_and(|n| !same(old, &n.0));
                let row = scenes.entry(scene).or_insert_with(|| SceneChanges {
                    updates: BTreeMap::new(),
                    rebuild: BTreeSet::new(),
                    observed: BTreeMap::new(),
                    removals: Vec::new(),
                    removed_ids: BTreeMap::new(),
                    insertions: StructuralInsertions::default(),
                });
                row.observed
                    .insert(id.clone(), old.map(|e| (e.body, e.collider)));
                if (rebuild || next.is_none())
                    && let Some(old) = old
                {
                    let r = old.body.map_or(
                        StructuralRemoval::Collider(old.collider),
                        StructuralRemoval::Body,
                    );
                    row.removals.push(r);
                    row.removed_ids.insert(r.key(), id.clone());
                }
                if rebuild {
                    row.rebuild.insert(id.clone());
                    row.insertions.bodies += usize::from(next.as_ref().unwrap().0.body.is_some());
                    row.insertions.colliders += 1;
                }
                row.updates.insert(id, next);
            }
            for row in scenes.values_mut() {
                row.removals.sort_unstable_by_key(|r| r.key());
            }
            Ok(StructuralChanges { scenes })
        }
        pub fn admit_structure<'p>(
            &self,
            changes: &'p StructuralChanges<I>,
            work: &mut CheckpointWorkBudget,
        ) -> Result<StructuralAdmission<'p, I>, PhysicsError> {
            if self.failed {
                return Err(PhysicsError::NumericRange);
            }
            let before_count = self.scenes.values().try_fold(0usize, |n, scene| {
                n.checked_add(scene.entries.len())
                    .ok_or(PhysicsError::Limit)
            })?;
            let mut after_count = before_count;
            for (id, row) in &changes.scenes {
                for (entity, next) in &row.updates {
                    if self
                        .scenes
                        .get(id)
                        .is_some_and(|s| s.entries.contains_key(entity))
                    {
                        after_count -= 1;
                    }
                    if next.is_some() {
                        after_count = after_count.checked_add(1).ok_or(PhysicsError::Limit)?;
                    }
                }
            }
            if after_count > 1_000_000 {
                return Err(PhysicsError::Limit);
            }
            let mut plans = BTreeMap::new();
            let mut fresh = BTreeMap::new();
            for (id, row) in &changes.scenes {
                for (entity, next) in &row.updates {
                    let old = self.scenes.get(id).and_then(|s| s.entries.get(entity));
                    if old.map(|e| (e.body, e.collider)) != row.observed[entity]
                        || next.as_ref().is_some_and(|n| !same(old, &n.0))
                            != row.rebuild.contains(entity)
                    {
                        return Err(StructuralError::Plan.into());
                    }
                }
                if let Some(scene) = self.scenes.get(id) {
                    plans.insert(
                        id.clone(),
                        scene.world.inner.preflight_structure(
                            &row.removals,
                            row.insertions,
                            work,
                        )?,
                    );
                } else {
                    // New scenes are candidates only, never inserted into the live
                    // map until all existing owners and both dimensions admit.
                    let mut scene = SceneWorld::default();
                    let plan = scene.world.inner.preflight_structure(
                        &row.removals,
                        row.insertions,
                        work,
                    )?;
                    let permit = scene.world.inner.structural_permit(plan)?;
                    execute(permit, row, &mut scene.entries, &mut scene.ids)?;
                    scene.query_dirty = !row.rebuild.is_empty();
                    fresh.insert(id.clone(), scene);
                }
            }
            Ok(StructuralAdmission {
                changes,
                plans,
                fresh,
                before_count,
            })
        }
        /// Acquire all existing-owner permits before any edit. The caller may
        /// admit other dimensions first; their edits cannot stale these owners.
        pub fn apply_structure(
            &mut self,
            mut admission: StructuralAdmission<'_, I>,
        ) -> Result<(), PhysicsError> {
            if self.failed {
                return Err(PhysicsError::NumericRange);
            }
            let current_count = self.scenes.values().try_fold(0usize, |n, scene| {
                n.checked_add(scene.entries.len())
                    .ok_or(PhysicsError::Limit)
            })?;
            if current_count != admission.before_count {
                return Err(StructuralError::Plan.into());
            }
            if admission
                .fresh
                .keys()
                .any(|id| self.scenes.contains_key(id))
                || admission
                    .plans
                    .keys()
                    .any(|id| !self.scenes.contains_key(id))
            {
                return Err(StructuralError::Plan.into());
            }
            let mut permits = Vec::with_capacity(admission.plans.len());
            for (id, scene) in &mut self.scenes {
                if let Some(plan) = admission.plans.remove(id) {
                    let row = &admission.changes.scenes[id];
                    let permit = scene.world.inner.structural_permit(plan)?;
                    permits.push((
                        permit,
                        row,
                        &mut scene.entries,
                        &mut scene.ids,
                        &mut scene.query_dirty,
                        &mut scene.character,
                    ));
                }
            }
            for (permit, row, entries, ids, dirty, character) in permits {
                execute(permit, row, entries, ids)?;
                if !row.removals.is_empty() || !row.rebuild.is_empty() {
                    *dirty = true;
                    *character = None;
                }
            }
            // No native access occurs between preflight and permit completion.
            // Numeric updates use the already checked declaration after all
            // structural plans are finished, so they cannot stale a later plan.
            for (id, row) in &admission.changes.scenes {
                if let Some(scene) = self.scenes.get_mut(id) {
                    for (entity, next) in &row.updates {
                        if let Some(next) = next
                            && !row.rebuild.contains(entity)
                        {
                            scene.update_motion(entity, &next.0.state)?;
                        }
                    }
                }
            }
            self.scenes.append(&mut admission.fresh);
            Ok(())
        }
    }
    fn execute<I: Ord + Clone>(
        mut permit: StructuralPermit<'_, '_>,
        row: &SceneChanges<I>,
        entries: &mut BTreeMap<I, Entry>,
        ids: &mut HashMap<ColliderHandle, I>,
    ) -> Result<(), PhysicsError> {
        for r in &row.removals {
            match *r {
                StructuralRemoval::Body(h) => {
                    permit.remove_body(h)?;
                }
                StructuralRemoval::Collider(h) => {
                    permit.remove_collider(h)?;
                }
            }
            let id = &row.removed_ids[&r.key()];
            let old = entries.remove(id).expect("admitted outer handle");
            ids.remove(&old.collider);
        }
        for id in &row.rebuild {
            let mut spec = row.updates[id].as_ref().unwrap().0.clone();
            let (body, collider) = native_parts(&spec);
            spec.state.sleeping = body.as_ref().is_some_and(RigidBody::is_sleeping);
            let body = body.map(|b| permit.insert_body(b)).transpose()?;
            let collider = permit.insert_collider(collider, body)?;
            ids.insert(collider, id.clone());
            entries.insert(
                id.clone(),
                Entry {
                    body,
                    collider,
                    config: spec,
                },
            );
        }
        permit.finish()?;
        Ok(())
    }
    fn native_parts(spec: &PhysicsSpec) -> (Option<RigidBody>, Collider) {
        let shape = ColliderBuilder::new(shapes::build(&spec.collider.shape))
            .density(spec.collider.density as f32)
            .friction(spec.collider.friction as f32)
            .restitution(spec.collider.restitution as f32)
            .sensor(spec.collider.sensor)
            .collision_groups(groups(spec.collider.memberships, spec.collider.filter))
            .active_collision_types(ActiveCollisionTypes::all());
        if let Some(c) = &spec.body {
            let b = match c.motion {
                BodyMotion::Fixed => RigidBodyBuilder::fixed(),
                BodyMotion::Dynamic => RigidBodyBuilder::dynamic(),
                BodyMotion::Kinematic => RigidBodyBuilder::kinematic_velocity_based(),
            }
            .pose(pose(&spec.state))
            .linvel(vector(spec.state.velocity))
            .angvel(vector(spec.state.angular_velocity))
            .gravity_scale(c.gravity_scale as f32)
            .linear_damping(c.linear_damping as f32)
            .angular_damping(c.angular_damping as f32)
            .can_sleep(c.can_sleep)
            .ccd_enabled(c.ccd)
            .build();
            (Some(b), shape.build())
        } else {
            (None, shape.position(pose(&spec.state)).build())
        }
    }
}
#[cfg(feature = "native-checkpoints")]
pub use checked::{StructuralAdmission, StructuralChanges};

#[cfg(test)]
mod tests {
    use super::*;
    fn body() -> PreparedBody {
        PreparedBody::new(PhysicsSpec {
            body: None,
            collider: DocCollider {
                shape: incant_types::ColliderShape::Sphere { radius: 0.25 },
                ..Default::default()
            },
            state: BodyState {
                translation: [0.; 3],
                rotation: [0., 0., 0., 1.],
                velocity: [0.; 3],
                angular_velocity: [0.; 3],
                sleeping: false,
            },
        })
        .unwrap()
    }
    #[test]
    fn fresh_declarations_reject_duplicate_identity_without_editing_an_owner() {
        assert!(matches!(
            PhysicsRuntime::from_prepared_bodies(1u32, vec![(2, body()), (2, body())]),
            Err(PhysicsError::Limit)
        ));
        let mut world = PhysicsRuntime::from_prepared_bodies(1u32, vec![(2, body())]).unwrap();
        assert_eq!(
            world
                .scenes
                .values()
                .map(|s| s.entries.len())
                .sum::<usize>(),
            1
        );
        assert!(
            world
                .apply_changes(vec![(1, 2, None), (1, 2, Some(body()))])
                .is_err()
        );
        assert_eq!(
            world
                .scenes
                .values()
                .map(|s| s.entries.len())
                .sum::<usize>(),
            1
        );
    }
    #[test]
    #[ignore = "bounded large-capacity regression; explicit CPU invocation only"]
    fn fresh_cooked_capacity_exceeds_snapshot_item_limit() {
        let count = 262_145u32;
        #[cfg(feature = "native-checkpoints")]
        assert_eq!(count as usize, incant_physics_snapshot::MAX_ITEMS + 1);
        let world =
            PhysicsRuntime::from_prepared_bodies(0u32, (0..count).map(|id| (id, body())).collect())
                .unwrap();
        assert_eq!(
            world
                .scenes
                .values()
                .map(|s| s.entries.len())
                .sum::<usize>(),
            count as usize
        );
        assert!(world.state(&0, &(count - 1)).is_some());
    }
}
