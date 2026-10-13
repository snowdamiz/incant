use super::*;
use crate::{CookedEntity, CookedPhysics, CookedPhysics2D, FrameCommands, StructuralCommand};
use incant_physics_snapshot::MAX_ALLOCATION;
use incant_types::{
    Collider, Collider2D, ColliderShape, ColliderShape2D, RigidBody, Transform, Velocity,
};
fn id(n: u8) -> StableId {
    StableId([n; 16])
}
fn payload() -> Budget {
    Budget::new(MAX_ALLOCATION).unwrap()
}
fn pw() -> PlanarWork {
    PlanarWork::new(100_000_000)
}
fn sw() -> SpatialWork {
    SpatialWork::new(100_000_000)
}
fn alloc() -> AllocationBudget {
    AllocationBudget::new(MAX_ALLOCATION, 100_000)
}
fn scene() -> CookedScene {
    let entities = [
        (1, -0.5),
        (2, 1.),
        (3, 0.6),
        (11, -0.5),
        (12, 1.),
        (13, 0.6),
    ]
    .into_iter()
    .map(|(n, y)| CookedEntity {
        id: id(n),
        parent: None,
        transform: Transform {
            translation: [0., y, 0.],
            ..Default::default()
        },
        velocity: None,
    })
    .collect();
    let spatial = (1..=3)
        .map(|n| CookedPhysics {
            entity: id(n),
            body: (n == 2).then_some(RigidBody {
                ccd: false,
                ..Default::default()
            }),
            collider: Collider {
                shape: if n == 2 {
                    ColliderShape::Sphere { radius: 0.4 }
                } else {
                    ColliderShape::Box {
                        half_extents: [4., 0.5, 4.],
                    }
                },
                sensor: n == 3,
                ..Default::default()
            },
            angular_velocity: None,
        })
        .collect();
    let planar = (11..=13)
        .map(|n| CookedPhysics2D {
            entity: id(n),
            body: (n == 12).then_some(RigidBody {
                ccd: false,
                ..Default::default()
            }),
            collider: Collider2D {
                shape: if n == 12 {
                    ColliderShape2D::Circle { radius: 0.4 }
                } else {
                    ColliderShape2D::Box {
                        half_extents: [4., 0.5],
                    }
                },
                sensor: n == 13,
                ..Default::default()
            },
            angular_velocity: None,
        })
        .collect();
    CookedScene::new(id(99), 64, entities)
        .unwrap()
        .with_physics(spatial)
        .unwrap()
        .with_physics_2d(planar)
        .unwrap()
}
fn warm() -> NativeWorld {
    let mut w = NativeWorld::new(scene());
    for _ in 0..64 {
        w.step().unwrap();
    }
    w
}
fn save(w: &mut NativeWorld) -> Vec<u8> {
    let mut p = payload();
    let mut a = pw();
    let mut b = sw();
    w.checkpoint(&mut p, &mut a, &mut b)
        .unwrap()
        .persisted_bytes(&mut p, &mut a, &mut b)
        .unwrap()
}
fn load(w: &NativeWorld, b: &[u8]) -> Result<NativeWorld, WorldCheckpointError> {
    NativeWorld::from_persisted_bytes_for(w, b, &mut payload(), &mut pw(), &mut sw(), &mut alloc())
}
fn seal(b: &mut [u8]) {
    let d = checksum(b);
    b[32..64].copy_from_slice(&d);
}
#[test]
fn persisted_world_keeps_pending_motion_events_and_exact_continuation() {
    let mut source = warm();
    let mut commands = FrameCommands::default();
    for n in [2, 12] {
        commands
            .push(StructuralCommand::SetVelocity {
                entity: id(n),
                velocity: Some(Velocity {
                    linear: [0.25, 0., 0.],
                }),
            })
            .unwrap();
    }
    source.apply(&mut commands).unwrap();
    assert!(matches!(
        source.save_bytes(),
        Err(SaveError::PhysicsUnsupported)
    ));
    let bytes = save(&mut source);
    assert_eq!(&bytes[8..12], &12u32.to_le_bytes());
    let mut restored = load(&source, &bytes).unwrap();
    assert_eq!(save(&mut restored), bytes);
    for i in 0..72 {
        eprintln!(
            "PERSISTED_WORLD continuation={i} source_tick={} import_tick={}",
            source.tick(),
            restored.tick()
        );
        source.step().unwrap();
        restored.step().unwrap();
        assert_eq!(source.snapshot(), restored.snapshot());
        assert_eq!(source.physics_events(), restored.physics_events());
        assert_eq!(source.physics_2d_events(), restored.physics_2d_events());
        eprintln!("PERSISTED_WORLD save source tick={}", source.tick());
        let source_bytes = save(&mut source);
        eprintln!("PERSISTED_WORLD save imported tick={}", restored.tick());
        assert_eq!(source_bytes, save(&mut restored));
    }
    assert!(NativeWorld::from_save_bytes(&bytes).is_err());
}
#[test]
fn persisted_world_rejects_corruption_framing_missing_owner_and_bad_cache_atomically() {
    let mut source = warm();
    let bytes = save(&mut source);
    let original = source.snapshot();
    let mut cases = Vec::new();
    for at in [0, 8, 24, 32] {
        let mut b = bytes.clone();
        b[at] ^= 1;
        cases.push(b);
    }
    for at in [12, 92] {
        let mut b = bytes.clone();
        b[at] = 255;
        seal(&mut b);
        cases.push(b);
    }
    for at in [64, 72, 80] {
        let mut b = bytes.clone();
        b[at..at + 8].copy_from_slice(&u64::MAX.to_le_bytes());
        seal(&mut b);
        cases.push(b);
    }
    let mut no_owner = bytes.clone();
    no_owner[12..16].copy_from_slice(&0u32.to_le_bytes());
    seal(&mut no_owner);
    cases.push(no_owner);
    let n = u32::from_le_bytes(bytes[88..92].try_into().unwrap()) as usize;
    assert!(n > 0);
    let mut bad_cache = bytes.clone();
    let at = bad_cache.len() - n * CACHE_ROW + 16;
    bad_cache[at..at + 8].copy_from_slice(&f64::NAN.to_le_bytes());
    seal(&mut bad_cache);
    cases.push(bad_cache);
    let mut extra = bytes.clone();
    extra.push(0);
    cases.push(extra);
    for cut in [0, 8, 95, bytes.len() - 1] {
        cases.push(bytes[..cut].to_vec());
    }
    for b in cases {
        assert!(load(&source, &b).is_err());
        assert_eq!(source.snapshot(), original);
    }
    assert!(
        NativeWorld::from_persisted_bytes_for(
            &source,
            &bytes,
            &mut Budget::new(0).unwrap(),
            &mut pw(),
            &mut sw(),
            &mut alloc()
        )
        .is_err()
    );
    assert_eq!(save(&mut source), bytes);
}
#[test]
fn persisted_world_binds_receiving_scene_and_never_steps_initial_physics() {
    let mut source = warm();
    let bytes = save(&mut source);
    let other = NativeWorld::new(CookedScene::new(id(100), 64, Vec::new()).unwrap());
    assert!(load(&other, &bytes).is_err());
    let mut initial = NativeWorld::new(scene());
    let before = initial.snapshot();
    let result = initial.checkpoint(&mut payload(), &mut pw(), &mut sw());
    assert!(result.is_err());
    assert_eq!(initial.snapshot(), before);
    assert_eq!(initial.tick(), 0);
}
#[test]
fn persisted_world_physics_free_frame_retains_legacy_bytes_and_rejects_size_overflow() {
    let mut world = NativeWorld::new(CookedScene::new(id(88), 60, Vec::new()).unwrap());
    let legacy = world.save_bytes().unwrap();
    let bytes = save(&mut world);
    let restored = load(&world, &bytes).unwrap();
    assert_eq!(restored.save_bytes().unwrap(), legacy);
    assert!(checked_sum(&[usize::MAX, 1]).is_err());
    let mut huge = bytes.clone();
    huge[72..80].copy_from_slice(&(PHYSICS_BYTES as u64).to_le_bytes());
    seal(&mut huge);
    assert!(sections(&huge).is_err());
    assert_eq!(
        MAX_PERSISTED_WORLD_BYTES,
        codec::MAX_SAVE_BYTES + PHYSICS_BYTES
    );
}

#[test]
fn persisted_world_late_planar_generation_refusal_preserves_both_solvers_and_queue() {
    let mut original = warm();
    let mut bytes = save(&mut original);
    let component_len = u64_at(&bytes, 64) as usize;
    let spatial_len = u64_at(&bytes, 72) as usize;
    let planar_outer = HEADER + component_len + spatial_len;
    assert_eq!(&bytes[planar_outer..planar_outer + 8], b"INPHY2D1");
    // Typed native body-arena scalar: one surviving removal has insufficient
    // generation headroom. No raw object install or runtime test hook is used.
    let native = planar_outer + 12;
    assert_eq!(&bytes[native..native + 8], b"ICNATIV1");
    let frame = native + 16;
    assert_eq!(&bytes[frame..frame + 8], b"ICWORLD1");
    let sections_count =
        u32::from_le_bytes(bytes[frame + 44..frame + 48].try_into().unwrap()) as usize;
    assert_eq!(sections_count, 22);
    let metadata_len =
        u32::from_le_bytes(bytes[frame + 52..frame + 56].try_into().unwrap()) as usize;
    let body_arena = frame + 48 + sections_count * 8 + metadata_len;
    assert_eq!(&bytes[body_arena..body_arena + 8], b"ICARN001");
    bytes[body_arena + 44..body_arena + 48].copy_from_slice(&(u32::MAX - 2).to_le_bytes());
    seal(&mut bytes);
    let mut candidate = load(&original, &bytes).unwrap();
    let before = save(&mut candidate);
    let snapshot = candidate.snapshot();
    let handles = candidate.entities.clone();
    let mut commands = FrameCommands::default();
    for entity in [id(2), id(12)] {
        commands.push(StructuralCommand::Despawn(entity)).unwrap();
    }
    let queued: Vec<_> = commands.iter().cloned().collect();
    let result = candidate.apply(&mut commands);
    assert!(matches!(
        result,
        Err(crate::RuntimeError::Physics2D(
            incant_physics_2d_runtime::PhysicsError::Structural(_)
        ))
    ));
    assert_eq!(candidate.snapshot(), snapshot);
    assert_eq!(candidate.entities, handles);
    assert_eq!(commands.iter().cloned().collect::<Vec<_>>(), queued);
    assert_eq!(save(&mut candidate), before);
}
