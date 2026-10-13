use incant_physics_runtime::{
    BodyState, PhysicsError, PhysicsRuntime, PhysicsSpec, PreparedBody, PreparedPhysics,
};
use incant_types::{BodyMotion, Collider, RigidBody};
use std::{
    cmp::Ordering,
    collections::BTreeMap,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering as AtomicOrdering},
    },
};

fn moving(x: f64, velocity: f64) -> PhysicsSpec {
    PhysicsSpec {
        body: Some(RigidBody {
            motion: BodyMotion::Kinematic,
            gravity_scale: 0.,
            ..Default::default()
        }),
        collider: Collider::default(),
        state: BodyState {
            translation: [x, 0., 0.],
            rotation: [0., 0., 0., 1.],
            velocity: [velocity, 0., 0.],
            angular_velocity: [0.; 3],
            sleeping: false,
        },
    }
}

fn sensor() -> PhysicsSpec {
    let mut spec = moving(0., 0.);
    spec.body = None;
    spec.collider.sensor = true;
    spec
}

#[test]
fn a_late_scene_failure_hides_partial_motion_and_previous_trigger_events() {
    let mut runtime = PhysicsRuntime::default();
    runtime.sync(
        PreparedPhysics::new(BTreeMap::from([
            (0_u8, BTreeMap::from([(1, sensor()), (2, moving(0., 1.))])),
            (9, BTreeMap::from([(3, moving(999_999., 2.))])),
        ]))
        .unwrap(),
    );
    runtime.step(1. / 60.).unwrap();
    assert_eq!(runtime.events().len(), 1);
    assert!(runtime.events()[0].entered);
    assert_eq!(runtime.state_iter().count(), 3);
    assert!(runtime.state(&9, &3).unwrap().translation[0] < 1_000_000.);

    // Scene0 advances first; the later scene crosses the admitted position
    // bound. No caller may observe a mixture of those partial and prior values.
    assert_eq!(runtime.step(1.), Err(PhysicsError::NumericRange));
    assert!(runtime.state(&0, &2).is_none());
    assert!(runtime.state(&9, &3).is_none());
    assert_eq!(runtime.state_iter().count(), 0);
    assert!(runtime.states().is_empty());
    assert!(runtime.events().is_empty());
    assert_eq!(runtime.step(1. / 60.), Err(PhysicsError::NumericRange));
    assert_eq!(
        runtime.sync_motion(&0, &2, &moving(0., 0.).state),
        Err(PhysicsError::NumericRange)
    );

    // Structural calls cannot turn this failed instance into a fresh session.
    runtime.remove(&9, &3);
    runtime.install(0, 4, PreparedBody::new(moving(0., 0.)).unwrap());
    runtime.sync(PreparedPhysics::new(BTreeMap::new()).unwrap());
    assert_eq!(runtime.step(1. / 60.), Err(PhysicsError::NumericRange));
    assert!(runtime.state(&0, &4).is_none());
    assert!(runtime.events().is_empty());
}

#[test]
fn invalid_step_leaves_a_healthy_runtime_and_its_observations_unchanged() {
    let mut runtime = PhysicsRuntime::default();
    runtime.sync(
        PreparedPhysics::new(BTreeMap::from([(
            0_u8,
            BTreeMap::from([(1, sensor()), (2, moving(0., 1.))]),
        )]))
        .unwrap(),
    );
    runtime.step(1. / 60.).unwrap();
    let states = runtime.states();
    let events = runtime.events().to_vec();
    for dt in [f64::NAN, f64::INFINITY, 0., 2.] {
        assert_eq!(runtime.step(dt), Err(PhysicsError::InvalidStep));
        assert_eq!(runtime.states(), states);
        assert_eq!(runtime.events(), events);
    }
    runtime.step(1. / 60.).unwrap();
    assert!(runtime.state(&0, &2).unwrap().translation[0] > states[&2].translation[0]);
}

// Ordered IDs are caller-provided. A panic while cloning an event ID is a real
// unwind after solver advancement, without adding a production failure hook.
#[derive(Debug)]
struct FallibleId {
    value: u8,
    panic_on_clone: Arc<AtomicBool>,
}
impl Clone for FallibleId {
    fn clone(&self) -> Self {
        assert!(
            !self.panic_on_clone.load(AtomicOrdering::Relaxed),
            "caller ID clone failed during event collection"
        );
        Self {
            value: self.value,
            panic_on_clone: Arc::clone(&self.panic_on_clone),
        }
    }
}
impl PartialEq for FallibleId {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}
impl Eq for FallibleId {}
impl PartialOrd for FallibleId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for FallibleId {
    fn cmp(&self, other: &Self) -> Ordering {
        self.value.cmp(&other.value)
    }
}

#[test]
fn a_caught_unwind_cannot_expose_or_resume_the_advanced_solver() {
    let panic_on_clone = Arc::new(AtomicBool::new(false));
    let id = |value| FallibleId {
        value,
        panic_on_clone: Arc::clone(&panic_on_clone),
    };
    let mut runtime = PhysicsRuntime::default();
    runtime.sync(
        PreparedPhysics::new(BTreeMap::from([(
            id(0),
            BTreeMap::from([(id(1), sensor()), (id(2), moving(0., 1.))]),
        )]))
        .unwrap(),
    );
    runtime.step(1. / 60.).unwrap();
    assert_eq!(runtime.events().len(), 1);
    panic_on_clone.store(true, AtomicOrdering::Relaxed);
    let result = catch_unwind(AssertUnwindSafe(|| runtime.step(1. / 60.)));
    assert!(result.is_err());

    // These observations and structural calls must not consult the failed
    // world's IDs or clone their state, even while the caller still panics.
    assert!(runtime.states().is_empty());
    assert_eq!(runtime.state_iter().count(), 0);
    assert!(runtime.state(&id(0), &id(2)).is_none());
    assert!(runtime.events().is_empty());
    runtime.install(id(0), id(3), PreparedBody::new(moving(0., 0.)).unwrap());
    runtime.sync(
        PreparedPhysics::new(BTreeMap::from([(
            id(0),
            BTreeMap::from([(id(4), moving(0., 0.))]),
        )]))
        .unwrap(),
    );
    panic_on_clone.store(false, AtomicOrdering::Relaxed);
    assert_eq!(runtime.step(1. / 60.), Err(PhysicsError::NumericRange));
}
