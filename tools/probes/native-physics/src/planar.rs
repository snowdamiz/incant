//! The exact same cooked 2D scenario and full public/ECS recorder is used by
//! native workers, serial browser WASM and the isolated shipping executable.
use incant_runtime::{
    CookedEntity, CookedPhysics2D, CookedScene, CookedSpatial, FrameCommands, NativeTilemap,
    NativeTilemapLayer, NativeWorld, RayQuery2D, StableId, StructuralCommand, VisualAssetKind,
    VisualAssetRef,
};
use incant_types::{
    AngularVelocity2D, BodyMotion, Collider2D, ColliderShape2D, RigidBody, Transform, Velocity,
};

const STEPS: u32 = 640;
const BODIES: u32 = 35;
fn id(n: u32) -> StableId {
    StableId(u128::from(n).to_be_bytes())
}

#[allow(dead_code)] // Fixture producer is excluded from the shipping example entry point.
pub fn scene() -> Result<CookedScene, String> {
    let mut entities = Vec::new();
    let mut physics = Vec::new();
    for n in 1..=BODIES {
        let index = n.saturating_sub(2);
        let position = match n {
            1 => [-16., -1., 7.],
            34 => [10., 2., 4.],
            35 => [-9., 1.5, 5.],
            _ => [
                -12. + f64::from(index % 4) * 2.,
                0.75 + f64::from(index / 4) * 1.125,
                2.,
            ],
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
        if n == 1 {
            continue;
        }
        let shape = if n == 35 {
            ColliderShape2D::Box {
                half_extents: [4., 2.],
            }
        } else {
            match n % 3 {
                0 => ColliderShape2D::Circle { radius: 0.45 },
                1 => ColliderShape2D::Capsule {
                    radius: 0.25,
                    half_height: 0.2,
                },
                _ => ColliderShape2D::Box {
                    half_extents: [0.45, 0.45],
                },
            }
        };
        physics.push(CookedPhysics2D {
            entity: id(n),
            body: (n != 35).then_some(RigidBody {
                motion: if n == 34 {
                    BodyMotion::Kinematic
                } else {
                    BodyMotion::Dynamic
                },
                ..Default::default()
            }),
            collider: Collider2D {
                shape,
                sensor: n == 35,
                ..Default::default()
            },
            angular_velocity: (n == 34).then_some(AngularVelocity2D { angular: 0.25 }),
        });
    }
    let mut map = CookedSpatial::empty(id(1));
    map.tilemap = Some(Box::new(NativeTilemap {
        tileset: id(90),
        size: [40, 1],
        cell_size: [1., 1.],
        layers: vec![NativeTilemapLayer {
            id: id(80),
            cells: (0..40)
                .map(|n| (!matches!(n, 20 | 21)).then_some(id(81)))
                .collect(),
            layer: 0,
            order: 0,
            solid: true,
        }],
    }));
    CookedScene::new(id(999), 64, entities)
        .map_err(|e| e.to_string())?
        .with_spatial(
            vec![VisualAssetRef {
                id: id(90),
                kind: VisualAssetKind::Texture,
                fingerprint: [3; 32],
            }],
            vec![map],
        )
        .map_err(|e| e.to_string())?
        .with_physics_2d(physics)
        .map_err(|e| e.to_string())
}

pub fn run(mut world: NativeWorld) -> Result<Vec<u8>, String> {
    if world.tick() != 0 || world.tick_rate() != 64 || world.entity_count() != BODIES as usize {
        return Err("unexpected fixture clock/entity count".into());
    }
    let mut ray = RayQuery2D {
        origin: [4.5, 1.],
        direction: [0., -1.],
        max_distance: 2.,
        include_sensors: false,
        exclude_entity: None,
        memberships: u32::MAX,
        filter: u32::MAX,
    };
    if world.raycast_2d(&ray).map_err(|e| e.to_string())?.is_some() {
        return Err("solid tile hole lost".into());
    }
    ray.origin[0] = 3.5;
    if world
        .raycast_2d(&ray)
        .map_err(|e| e.to_string())?
        .map(|h| h.entity)
        != Some(id(1))
    {
        return Err("solid tile collider missing".into());
    }
    let mut bytes = Vec::with_capacity(32 + BODIES as usize * (STEPS as usize + 1) * 169);
    bytes.resize(32, 0);
    let (mut max_sleeping, mut wake_transitions, mut events) = (0u32, 0u32, 0u32);
    let mut last_sleeping = [false; BODIES as usize];
    for frame in 0..=STEPS {
        if frame == 480 {
            world
                .with_numeric_columns(|columns| {
                    columns.for_each_chunk(|mut chunk| {
                        for (row, entity) in chunk.ids.iter().enumerate() {
                            if (id(2)..=id(9)).contains(entity) {
                                let velocity =
                                    chunk.velocities.as_deref_mut().expect("body motion");
                                velocity[row * 3] = 0.5;
                                velocity[row * 3 + 1] = 2.;
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
                .push(StructuralCommand::SetAngularVelocity2D {
                    entity: id(34),
                    velocity: Some(AngularVelocity2D { angular: -0.5 }),
                })
                .map_err(|e| e.to_string())?;
            let mut body = world
                .physics_2d_configuration(id(33))
                .ok_or("missing replacement body")?;
            body.collider.shape = ColliderShape2D::Box {
                half_extents: [0.375, 0.375],
            };
            commands
                .push(StructuralCommand::SetPhysics2D {
                    entity: id(33),
                    physics: Some(body),
                })
                .map_err(|e| e.to_string())?;
            world.apply(&mut commands).map_err(|e| e.to_string())?;
        }
        if frame > 0 {
            world.step().map_err(|e| e.to_string())?;
        }
        if world.tick() != u64::from(frame) {
            return Err("clock did not advance exactly once".into());
        }
        let mut sleeping = 0;
        for n in 1..=BODIES {
            let state = world.physics_2d_state(id(n)).ok_or("missing 2D body")?;
            let entity = world.inspect(id(n)).ok_or("missing ECS entity")?;
            bytes.extend(id(n).0);
            for value in state
                .translation
                .into_iter()
                .chain([state.rotation])
                .chain(state.velocity)
                .chain([state.angular_velocity])
                .chain(entity.transform.translation)
                .chain(entity.transform.rotation)
                .chain(entity.transform.scale)
                .chain(entity.velocity.map_or([0.; 3], |v| v.linear))
            {
                if !value.is_finite() {
                    return Err("nonfinite published 2D state".into());
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
        if !world.physics_2d_events().windows(2).all(|p| p[0] < p[1]) {
            return Err("2D trigger ordering/uniqueness lost".into());
        }
        events += world.physics_2d_events().len() as u32;
    }
    if max_sleeping < 16 || wake_transitions < 8 || events == 0 {
        return Err(format!(
            "missing 2D coverage: sleep={max_sleeping} wake={wake_transitions} events={events}"
        ));
    }
    for (chunk, value) in bytes[..32].as_chunks_mut::<4>().0.iter_mut().zip([
        0x4e503244,
        1,
        BODIES,
        STEPS + 1,
        169,
        max_sleeping,
        wake_transitions,
        events,
    ]) {
        chunk.copy_from_slice(&value.to_le_bytes());
    }
    Ok(bytes)
}
