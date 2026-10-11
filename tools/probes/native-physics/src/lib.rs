//! Same cooked/native ECS input, commands and full public body recorder on both
//! native worker counts and actual serial browser WASM. This is not a timer.
use incant_runtime::{
    CookedEntity, CookedPhysics, CookedScene, FrameCommands, NativeWorld, StableId,
    StructuralCommand,
};
use incant_types::{
    AngularVelocity, BodyMotion, Collider, ColliderPart, ColliderShape, PrimitiveColliderShape,
    RigidBody, Transform, Velocity,
};

pub mod planar;

const STEPS: u32 = 640;
const BODIES: u32 = 35;
fn id(n: u32) -> StableId {
    StableId(u128::from(n).to_be_bytes())
}

fn workers(workers: usize) -> Result<(), String> {
    #[cfg(not(target_family = "wasm"))]
    {
        if workers != 1 && workers != 4 {
            return Err("native requires one or four workers".into());
        }
        rayon::ThreadPoolBuilder::new()
            .num_threads(workers)
            .build_global()
            .map_err(|e| e.to_string())?;
        if rayon::current_num_threads() != workers {
            return Err("worker pool mismatch".into());
        }
    }
    #[cfg(target_family = "wasm")]
    if workers != 0 {
        return Err("WASM must be serial".into());
    }
    Ok(())
}

pub fn run(workers_count: usize) -> Result<Vec<u8>, String> {
    workers(workers_count)?;
    let mut entities = Vec::new();
    let mut physics = Vec::new();
    for n in 1..=BODIES {
        let position = match n {
            1 => [0., -0.5, 0.],
            34 => [8., 2., 0.],
            35 => [0., 2., 0.],
            _ => {
                let index = n - 2;
                [
                    f64::from(index % 4) * 1.125 - 2.,
                    f64::from(index / 16) * 1.125 + 0.625,
                    f64::from((index / 4) % 4) * 1.125 - 2.,
                ]
            }
        };
        entities.push(CookedEntity {
            id: id(n),
            parent: None,
            transform: Transform {
                translation: position,
                ..Default::default()
            },
            velocity: (n == 34).then_some(Velocity {
                linear: [0.25, 0., 0.],
            }),
        });
        let shape = if n == 1 {
            ColliderShape::Box {
                half_extents: [64., 0.5, 64.],
            }
        } else if n == 34 {
            ColliderShape::Compound {
                parts: vec![
                    ColliderPart {
                        id: "00000000000000000000000001".into(),
                        translation: [0.5, 0., 0.],
                        rotation: [0., 0., 0., 1.],
                        shape: PrimitiveColliderShape::Sphere { radius: 0.25 },
                    },
                    ColliderPart {
                        id: "00000000000000000000000002".into(),
                        translation: [-0.5, 0., 0.],
                        rotation: [0., 0., 0., 1.],
                        shape: PrimitiveColliderShape::Capsule {
                            radius: 0.25,
                            half_height: 0.25,
                        },
                    },
                ],
            }
        } else {
            Collider::default().shape
        };
        physics.push(CookedPhysics {
            entity: id(n),
            body: if n == 1 || n == 35 {
                None
            } else {
                Some(RigidBody {
                    motion: if n == 34 {
                        BodyMotion::Kinematic
                    } else {
                        BodyMotion::Dynamic
                    },
                    ..Default::default()
                })
            },
            collider: Collider {
                shape,
                sensor: n == 35,
                ..Default::default()
            },
            angular_velocity: (n == 34).then_some(AngularVelocity {
                angular: [0., 0.25, 0.],
            }),
        });
    }
    let scene = CookedScene::new(id(999), 64, entities)
        .map_err(|e| e.to_string())?
        .with_physics(physics)
        .map_err(|e| e.to_string())?;
    // Exercise real canonical binary loading rather than constructing a different
    // world in the browser adapter. Source documents never enter this harness.
    let mut world = NativeWorld::from_bytes(&scene.to_bytes()).map_err(|e| e.to_string())?;
    let mut bytes = vec![0; 32];
    let mut max_sleeping = 0;
    let mut wake_transitions = 0;
    let mut last_sleeping = [false; BODIES as usize];
    let mut events = 0u32;
    for frame in 0..=STEPS {
        if frame == 480 {
            world
                .with_numeric_columns(|columns| {
                    columns.for_each_chunk(|mut chunk| {
                        for (row, entity) in chunk.ids.iter().enumerate() {
                            if (id(2)..=id(9)).contains(entity) {
                                let velocity =
                                    chunk.velocities.as_deref_mut().expect("body motion");
                                velocity[row * 3 + 1] = 2.;
                                velocity[row * 3] = 0.5;
                            }
                        }
                    });
                    columns.validate()
                })
                .map_err(|e| e.to_string())?;
        }
        if frame == 560 {
            let mut commands = FrameCommands::default();
            commands
                .push(StructuralCommand::SetAngularVelocity {
                    entity: id(34),
                    velocity: Some(AngularVelocity {
                        angular: [0., -0.5, 0.],
                    }),
                })
                .map_err(|e| e.to_string())?;
            world.apply(&mut commands).map_err(|e| e.to_string())?;
        }
        if frame > 0 {
            world.step().map_err(|e| e.to_string())?;
        }
        let mut sleeping = 0;
        for n in 1..=BODIES {
            let state = world.physics_state(id(n)).ok_or("missing body")?;
            bytes.extend(id(n).0);
            for value in state
                .translation
                .into_iter()
                .chain(state.rotation)
                .chain(state.velocity)
                .chain(state.angular_velocity)
            {
                if !value.is_finite() {
                    return Err("nonfinite published state".into());
                }
                bytes.extend(value.to_bits().to_le_bytes());
            }
            bytes.push(u8::from(state.sleeping));
            sleeping += u32::from(state.sleeping);
            let old = &mut last_sleeping[n as usize - 1];
            wake_transitions += u32::from(*old && !state.sleeping);
            *old = state.sleeping;
        }
        max_sleeping = max_sleeping.max(sleeping);
        events += world.physics_events().len() as u32;
    }
    if max_sleeping < 16 || wake_transitions < 8 || events == 0 {
        return Err(format!(
            "missing coverage: sleep={max_sleeping} wake={wake_transitions} events={events}"
        ));
    }
    for (chunk, value) in bytes[..32].as_chunks_mut::<4>().0.iter_mut().zip([
        0x4e504831,
        1,
        BODIES,
        STEPS + 1,
        121,
        max_sleeping,
        wake_transitions,
        events,
    ]) {
        chunk.copy_from_slice(&value.to_le_bytes());
    }
    Ok(bytes)
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn state_stream() -> Result<Vec<u8>, wasm_bindgen::JsValue> {
    run(0).map_err(|e| wasm_bindgen::JsValue::from_str(&e))
}

/// Separate planar stream; no changes to the existing 3D recorder.
pub fn run_2d(workers_count: usize) -> Result<Vec<u8>, String> {
    workers(workers_count)?;
    let world = NativeWorld::from_bytes(&planar::scene()?.to_bytes()).map_err(|e| e.to_string())?;
    planar::run(world)
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn state_stream_2d() -> Result<Vec<u8>, wasm_bindgen::JsValue> {
    run_2d(0).map_err(|e| wasm_bindgen::JsValue::from_str(&e))
}
