//! Separate serial/parallel/browser builds share this exact input and recorder.
//! This is solver conformance, not a timing benchmark or an Incant play loop.
use rapier3d::prelude::*;

#[cfg(all(target_family = "wasm", feature = "parallel"))]
compile_error!("the parity browser build must remain serial");

pub const STEPS: u32 = 640;
const HEADER_WORDS: usize = 16;
const RECORD_WORDS: u32 = 15;

pub struct Run {
    pub bytes: Vec<u8>,
    pub bodies: usize,
    pub workers: usize,
    pub max_contacts: usize,
    pub max_sleeping: usize,
    pub wake_transitions: usize,
    pub joints: usize,
}

fn floor(world: &mut PhysicsWorld, handles: &mut Vec<RigidBodyHandle>) {
    handles.push(
        world
            .insert(
                RigidBodyBuilder::fixed().translation(Vector::new(0., -0.5, 0.)),
                ColliderBuilder::cuboid(64., 0.5, 64.).friction(0.5),
            )
            .0,
    );
}

fn pile(world: &mut PhysicsWorld, handles: &mut Vec<RigidBodyHandle>) {
    for y in 0..4 {
        for z in 0..8 {
            for x in 0..8 {
                let jitter = ((x * 7 + z * 11 + y * 3) % 5) as f32 / 64.;
                handles.push(
                    world
                        .insert(
                            RigidBodyBuilder::dynamic().translation(Vector::new(
                                x as f32 * 1.0625 - 4. + jitter,
                                y as f32 * 1.125 + 0.625,
                                z as f32 * 1.0625 - 4. - jitter,
                            )),
                            ColliderBuilder::cuboid(0.5, 0.5, 0.5).friction(0.5),
                        )
                        .0,
                );
            }
        }
    }
    // Isolated resting bodies guarantee meaningful natural sleep/wake coverage.
    for i in 0..16 {
        handles.push(
            world
                .insert(
                    RigidBodyBuilder::dynamic().translation(Vector::new(
                        i as f32 * 2. - 16.,
                        0.5,
                        16.,
                    )),
                    ColliderBuilder::cuboid(0.5, 0.5, 0.5),
                )
                .0,
        );
    }
}

fn chains(world: &mut PhysicsWorld, handles: &mut Vec<RigidBodyHandle>) {
    for chain in 0..24 {
        let x = (chain % 6) as f32 * 2. - 5.;
        let z = (chain / 6) as f32 * 2. - 3.;
        let anchor =
            world.insert_body(RigidBodyBuilder::fixed().translation(Vector::new(x, 8., z)));
        handles.push(anchor);
        let mut parent = anchor;
        for link in 0..8 {
            let handle = world
                .insert(
                    RigidBodyBuilder::dynamic().translation(Vector::new(x, 7. - link as f32, z)),
                    ColliderBuilder::cuboid(0.375, 0.5, 0.375).friction(0.5),
                )
                .0;
            world.insert_impulse_joint(
                parent,
                handle,
                SphericalJointBuilder::new()
                    .local_anchor1(Vector::new(0., -0.5, 0.))
                    .local_anchor2(Vector::new(0., 0.5, 0.)),
            );
            handles.push(handle);
            parent = handle;
        }
    }
}

fn record(world: &PhysicsWorld, handles: &[RigidBodyHandle], words: &mut Vec<u32>) {
    for (id, handle) in handles.iter().enumerate() {
        let body = &world.bodies[*handle];
        words.push(id as u32);
        for value in body
            .translation()
            .to_array()
            .into_iter()
            .chain(body.rotation().to_array())
            .chain(body.linvel().to_array())
            .chain(body.angvel().to_array())
        {
            assert!(
                value.is_finite() && value.abs() < 1024.,
                "non-finite/unbounded body state"
            );
            words.push(value.to_bits());
        }
        words.push(u32::from(body.is_sleeping()));
    }
}

/// workers=0 is the separately compiled serial solver; parallel builds require 1/4.
pub fn run(scenario: u32, workers: usize) -> Result<Run, String> {
    if scenario > 1 {
        return Err("scenario must be 0 (contacts) or 1 (joints)".into());
    }
    let mut world = PhysicsWorld::new();
    #[cfg(feature = "parallel")]
    {
        if workers != 1 && workers != 4 {
            return Err("parallel build requires 1 or 4 workers".into());
        }
        world
            .configure_thread_pool(workers)
            .map_err(|error| error.to_string())?;
        if world.num_threads() != Some(workers) {
            return Err("requested pool was not configured".into());
        }
    }
    #[cfg(not(feature = "parallel"))]
    if workers != 0 {
        return Err("serial build requires zero workers".into());
    }
    world.gravity = Vector::new(0., -8., 0.);
    world.integration_parameters.dt = 1. / 64.;
    let mut handles = Vec::new();
    floor(&mut world, &mut handles);
    if scenario == 0 {
        pile(&mut world, &mut handles);
    } else {
        chains(&mut world, &mut handles);
    }
    let joints = world.impulse_joints.len();
    let mut words = vec![0; HEADER_WORDS];
    record(&world, &handles, &mut words);
    let mut previous_sleep = vec![false; handles.len()];
    let mut max_contacts = 0;
    let mut max_sleeping = 0;
    let mut wake_transitions = 0;
    for tick in 0..STEPS {
        // Every authored input is dyadic; no PRNG, trig setup, wall clock or host math.
        if tick == 64 || tick == 384 || tick == 512 {
            let count = handles.len();
            for (index, handle) in handles.iter().enumerate() {
                if index % 13 == 1 || (scenario == 0 && index + 16 >= count) {
                    let body = &mut world.bodies[*handle];
                    body.apply_impulse(Vector::new(0.125, 0.25, -0.0625), true);
                    body.apply_torque_impulse(Vector::new(0.03125, -0.0625, 0.015625), true);
                }
            }
        }
        world.step();
        max_contacts = max_contacts.max(
            world
                .narrow_phase
                .contact_pairs()
                .filter(|pair| pair.has_any_active_contact())
                .count(),
        );
        let mut sleeping = 0;
        for (index, handle) in handles.iter().enumerate() {
            let body = &world.bodies[*handle];
            let asleep = body.is_dynamic() && body.is_sleeping();
            sleeping += usize::from(asleep);
            wake_transitions += usize::from(previous_sleep[index] && !asleep);
            previous_sleep[index] = asleep;
        }
        max_sleeping = max_sleeping.max(sleeping);
        record(&world, &handles, &mut words);
    }
    if max_contacts < 20 {
        return Err("scenario did not exercise enough active contacts".into());
    }
    if scenario == 0 && (max_sleeping < 16 || wake_transitions < 16) {
        return Err(format!(
            "sleep/wake coverage missing: sleeping={max_sleeping}, wakes={wake_transitions}"
        ));
    }
    if scenario == 1 && joints != 192 {
        return Err("joint coverage missing".into());
    }
    words[..12].copy_from_slice(&[
        0x50415231,
        1,
        scenario,
        handles.len() as u32,
        STEPS + 1,
        RECORD_WORDS,
        (1f32 / 64.).to_bits(),
        (-8f32).to_bits(),
        max_contacts as u32,
        max_sleeping as u32,
        wake_transitions as u32,
        joints as u32,
    ]);
    Ok(Run {
        bytes: words.into_iter().flat_map(u32::to_le_bytes).collect(),
        bodies: handles.len(),
        workers,
        max_contacts,
        max_sleeping,
        wake_transitions,
        joints,
    })
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn state_stream(scenario: u32) -> Result<Vec<u8>, wasm_bindgen::JsValue> {
    run(scenario, 0)
        .map(|result| result.bytes)
        .map_err(|error| wasm_bindgen::JsValue::from_str(&error))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_unknown_scenarios_and_unbuilt_execution_modes() {
        assert!(run(2, 0).is_err());
        assert!(run(0, 2).is_err());
    }
}
