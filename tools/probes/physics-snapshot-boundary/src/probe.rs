use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

type Pair = (u128, u128);
type HandleRecord = (u128, Option<(u32, u32)>, (u32, u32));
#[derive(Clone)]
struct Entry {
    body: Option<RigidBodyHandle>,
    collider: ColliderHandle,
    shape: SharedShape,
    kind: Option<RigidBodyType>,
    sensor: bool,
}
struct Owner {
    world: PhysicsWorld,
    entries: BTreeMap<u128, Entry>,
    overlaps: BTreeSet<Pair>,
    events: Vec<(Pair, bool)>,
}
impl Owner {
    fn empty() -> Self {
        Self {
            world: PhysicsWorld::default(),
            entries: BTreeMap::new(),
            overlaps: BTreeSet::new(),
            events: vec![],
        }
    }
    fn insert(
        &mut self,
        id: u128,
        kind: Option<RigidBodyType>,
        shape: SharedShape,
        x: f32,
        y: f32,
        sensor: bool,
    ) {
        let collider = ColliderBuilder::new(shape.clone())
            .sensor(sensor)
            .friction(0.7)
            .active_collision_types(ActiveCollisionTypes::all())
            .user_data(id);
        let (body, collider) = if let Some(kind) = kind {
            let (b, c) = self.world.insert(
                RigidBodyBuilder::new(kind)
                    .translation(vector(x, y))
                    .ccd_enabled(true)
                    .user_data(id),
                collider,
            );
            (Some(b), c)
        } else {
            (
                None,
                self.world
                    .insert_collider(collider.translation(vector(x, y)), None),
            )
        };
        assert!(
            self.entries
                .insert(
                    id,
                    Entry {
                        body,
                        collider,
                        shape,
                        kind,
                        sensor
                    }
                )
                .is_none()
        );
    }
    fn remove(&mut self, id: u128) {
        let e = self.entries.remove(&id).unwrap();
        if let Some(b) = e.body {
            self.world.remove_body(b);
        } else {
            self.world.remove_collider(e.collider);
        }
        // As in the current native adapters, past overlaps are retained until
        // the next step emits the corresponding exit, including deleted IDs.
    }
    fn step(&mut self) {
        self.world.integration_parameters.dt = 1. / 60.;
        self.world.step();
        assert!(self.world.quarantine().bodies().is_empty());
        assert!(self.world.quarantine().colliders().is_empty());
        let mut next = BTreeSet::new();
        for (a, _, b, _, yes) in self.world.intersection_pairs() {
            if yes {
                let a = self.world.colliders[a].user_data;
                let b = self.world.colliders[b].user_data;
                next.insert((a.min(b), a.max(b)));
            }
        }
        self.events = next
            .difference(&self.overlaps)
            .map(|&p| (p, true))
            .chain(self.overlaps.difference(&next).map(|&p| (p, false)))
            .collect();
        self.events.sort();
        self.overlaps = next;
    }
    fn states(&self) -> Vec<(u128, Vec<u8>)> {
        self.entries
            .iter()
            .map(|(&id, e)| {
                let c = &self.world.colliders[e.collider];
                let state = if let Some(h) = e.body {
                    let b = &self.world.bodies[h];
                    json!([b.position(), b.linvel(), b.angvel(), b.is_sleeping()])
                } else {
                    json!([c.position()])
                };
                (id, serde_json::to_vec(&state).unwrap())
            })
            .collect()
    }
    fn bytes(&self) -> Vec<u8> {
        serde_json::to_vec(&self.world).unwrap()
    }
    fn handles(&self) -> Vec<HandleRecord> {
        self.entries
            .iter()
            .map(|(&id, e)| {
                (
                    id,
                    e.body.map(|h| h.into_raw_parts()),
                    e.collider.into_raw_parts(),
                )
            })
            .collect()
    }
    fn trusted_clone(&self) -> Self {
        // Every field below is from a trusted live owner. No saved numeric
        // index is accepted here, nor does this establish a binary loader.
        assert!(self.entries.len() <= 64);
        let w = &self.world;
        Self {
            world: PhysicsWorld {
                gravity: w.gravity,
                integration_parameters: w.integration_parameters,
                physics_pipeline: PhysicsPipeline::new(),
                collision_pipeline: CollisionPipeline::new(),
                islands: w.islands.clone(),
                broad_phase: w.broad_phase.clone(),
                narrow_phase: w.narrow_phase.clone(),
                bodies: w.bodies.clone(),
                colliders: w.colliders.clone(),
                impulse_joints: w.impulse_joints.clone(),
                multibody_joints: w.multibody_joints.clone(),
                soft_bodies: w.soft_bodies.clone(),
                ccd_solver: CCDSolver::new(),
            },
            entries: self.entries.clone(),
            overlaps: self.overlaps.clone(),
            events: self.events.clone(),
        }
    }
    fn pose_reconstruction(&self) -> Self {
        // Deliberately named lossy comparison, not a replacement save policy.
        let mut out = Self::empty();
        out.world.gravity = self.world.gravity;
        out.world.integration_parameters = self.world.integration_parameters;
        for (&id, e) in &self.entries {
            out.insert(id, e.kind, e.shape.clone(), 0., 0., e.sensor);
            let n = &out.entries[&id];
            if let (Some(b), Some(nb)) = (e.body, n.body) {
                let b = &self.world.bodies[b];
                let new = &mut out.world.bodies[nb];
                new.set_position(*b.position(), false);
                new.set_linvel(b.linvel(), false);
                new.set_angvel(b.angvel(), false);
                if b.is_sleeping() {
                    new.sleep();
                }
            } else {
                out.world.colliders[n.collider]
                    .set_position(*self.world.colliders[e.collider].position());
            }
        }
        out.overlaps = self.overlaps.clone();
        out.events = self.events.clone();
        out
    }
}
fn fixture() -> Owner {
    let mut o = Owner::empty();
    // Compound ground also represents the checked rectangular tile-box shape
    // route. This does not exercise Incant's tilemap cooker.
    let floor = SharedShape::compound(
        (0..5)
            .map(|i| {
                (
                    Pose::from_translation(vector(i as f32 * 2. - 4., 0.)),
                    cuboid(1., 0.5),
                )
            })
            .collect(),
    );
    o.insert(1, None, floor, 0., -0.5, false);
    for i in 0..8 {
        o.insert(
            10 + i,
            Some(RigidBodyType::Dynamic),
            cuboid(0.45, 0.45),
            if i % 2 == 0 { -0.47 } else { 0.47 },
            0.48 + (i / 2) as f32 * 0.94,
            false,
        );
    }
    o.insert(
        30,
        Some(RigidBodyType::Dynamic),
        SharedShape::ball(0.35),
        3.,
        3.,
        false,
    );
    o.insert(
        31,
        Some(RigidBodyType::Dynamic),
        SharedShape::capsule_y(0.3, 0.2),
        -3.,
        3.,
        false,
    );
    o.insert(
        40,
        Some(RigidBodyType::KinematicVelocityBased),
        SharedShape::ball(0.25),
        -1.5,
        1.,
        false,
    );
    o.insert(50, None, cuboid(0.7, 0.7), -1.5, 1., true);
    // Recycled slot/generation is intentionally unlike canonical-ID insertion.
    o.insert(
        60,
        Some(RigidBodyType::Dynamic),
        SharedShape::ball(0.1),
        9.,
        9.,
        false,
    );
    o.remove(60);
    o.insert(
        61,
        Some(RigidBodyType::Dynamic),
        SharedShape::ball(0.1),
        4.,
        2.,
        false,
    );
    o
}
fn edits(o: &mut Owner, n: usize) {
    if n == 10 || n == 30 {
        let h = o.entries[&40].body.unwrap();
        o.world.bodies[h].set_translation(vector(if n == 10 { 4. } else { -1.5 }, 1.), true);
    }
    if n == 60 {
        o.remove(40);
    }
    if n == 80 {
        o.insert(
            40,
            Some(RigidBodyType::KinematicVelocityBased),
            SharedShape::ball(0.25),
            -1.5,
            1.,
            false,
        );
    }
}
pub fn run() -> Vec<Value> {
    let mut results = vec![];
    for before in [0, 30, 150, 480] {
        let mut original = fixture();
        for _ in 0..before {
            original.step();
        }
        let mut clone = original.trusted_clone();
        assert_eq!(original.bytes(), clone.bytes());
        assert_eq!(original.handles(), clone.handles());
        let mut reset = original.pose_reconstruction();
        let initial_reset_bytes_equal = original.bytes() == reset.bytes();
        let initial_reset_states_equal = original.states() == reset.states();
        assert!(initial_reset_states_equal);
        let mut first_state_difference = None;
        let mut first_event_difference = None;
        let mut first_event_records = None;
        let mut max_translation_difference = 0_f32;
        let mut max_velocity_difference = 0_f32;
        let initial_sleeping_bodies = original
            .world
            .bodies
            .iter()
            .filter(|(_, b)| b.is_sleeping())
            .count();
        let mut entered = 0;
        let mut exited = 0;
        let mut warmstarts = 0;
        for pair in original.world.narrow_phase.contact_pairs() {
            for manifold in pair.solver_manifolds() {
                for contact in &manifold.points {
                    if contact.data.warmstart_impulse != 0. {
                        warmstarts += 1;
                    }
                }
            }
        }
        for n in 1..=120 {
            for owner in [&mut original, &mut clone, &mut reset] {
                edits(owner, n);
                owner.step();
            }
            assert_eq!(
                original.bytes(),
                clone.bytes(),
                "dimension {DIMENSION}, boundary {before}, step {n}"
            );
            assert_eq!(original.handles(), clone.handles());
            assert_eq!(original.states(), clone.states());
            assert_eq!(original.overlaps, clone.overlaps);
            assert_eq!(original.events, clone.events);
            entered += original.events.iter().filter(|(_, e)| *e).count();
            exited += original.events.iter().filter(|(_, e)| !*e).count();
            if first_state_difference.is_none() && original.states() != reset.states() {
                first_state_difference = Some(n);
            }
            if first_event_difference.is_none() && original.events != reset.events {
                first_event_difference = Some(n);
                first_event_records = Some(
                    json!({"uninterrupted":original.events,"pose_reconstructed":reset.events}),
                );
            }
            for (id, entry) in &original.entries {
                if let (Some(a), Some(b)) = (entry.body, reset.entries[id].body) {
                    let a = &original.world.bodies[a];
                    let b = &reset.world.bodies[b];
                    max_translation_difference = max_translation_difference
                        .max((a.translation() - b.translation()).length());
                    max_velocity_difference =
                        max_velocity_difference.max((a.linvel() - b.linvel()).length());
                }
            }
        }
        assert!(entered > 0 && exited > 0);
        if before != 0 {
            assert!(warmstarts > 0);
            assert!(first_state_difference.is_some());
        }
        results.push(json!({"dimension":DIMENSION,"checkpoint_tick":before,"continued_ticks":120,"trusted_clone_exact":true,"full_serialized_state_and_handles_compared_each_tick":true,"initial_warmstart_contacts":warmstarts,"initial_sleeping_bodies":initial_sleeping_bodies,"entered":entered,"exited":exited,"lossy_comparison":{"initial_states_equal":initial_reset_states_equal,"initial_complete_bytes_equal":initial_reset_bytes_equal,"first_subsequent_state_difference":first_state_difference,"first_subsequent_event_difference":first_event_difference,"first_different_event_records":first_event_records,"max_translation_difference":max_translation_difference,"max_linear_velocity_difference":max_velocity_difference},"untrusted_bytes_decoded":false}));
    }
    results
}
