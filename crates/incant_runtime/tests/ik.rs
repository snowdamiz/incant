#[path = "support/scene_format.rs"]
mod scene_format;
use incant_runtime::*;
use incant_types::{Animation, Transform};
use sha2::{Digest, Sha256};
use std::sync::Arc;
#[path = "ik/alloc.rs"]
mod alloc;
#[global_allocator]
static ALLOC: alloc::CountAlloc = alloc::CountAlloc;

fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
fn sample(n: u128) -> AnimationIkSample {
    AnimationIkSample {
        id: id(n),
        chain: [0, 1, 2],
        target: [1., 1., 0.],
        pole: [0., 1., 0.],
        weight: 1.,
    }
}
fn node(entity: u128, count: usize) -> CookedSpatial {
    let mut n = CookedSpatial::empty(id(entity));
    n.mesh = Some(NativeMeshRenderer {
        mesh: id(9999),
        cast_shadows: true,
    });
    n.animation = Some(Arc::new(NativeAnimation {
        mesh_fingerprint: [7; 32],
        clip_durations: Arc::from([1.]),
        initial: Animation {
            clip: Some(0),
            seconds: 0.,
            speed: 1.,
            playing: true,
            looping: true,
        },
        graph: None,
    }));
    n.ik = Some(Arc::new(NativeIk {
        mesh_fingerprint: [7; 32],
        initial: (0..count).map(|i| sample(i as u128 + 100)).collect(),
    }));
    n
}
fn scene(owners: usize, count: usize) -> CookedScene {
    CookedScene::new(
        id(0),
        60,
        (1..=owners)
            .map(|i| CookedEntity {
                id: id(i as u128),
                parent: None,
                transform: Transform::default(),
                velocity: None,
            })
            .collect(),
    )
    .unwrap()
    .with_spatial(
        vec![VisualAssetRef {
            id: id(9999),
            fingerprint: [7; 32],
            kind: VisualAssetKind::Model,
        }],
        (1..=owners).map(|i| node(i as u128, count)).collect(),
    )
    .unwrap()
}
fn control(entity: u128, constraint: u128, target: f64) -> StructuralCommand {
    StructuralCommand::ControlIk {
        entity: id(entity),
        control: IkControl {
            constraint: id(constraint),
            target: [target, 1., 0.],
            pole: [0., 1., 0.],
            weight: 1.,
        },
    }
}
fn apply(
    world: &mut NativeWorld,
    values: impl IntoIterator<Item = StructuralCommand>,
) -> Result<(), RuntimeError> {
    let mut q = FrameCommands::default();
    for value in values {
        q.push(value)?;
    }
    world.apply(&mut q)
}
fn checksum(bytes: &mut [u8], prefix: usize, header: usize) {
    let mut h = Sha256::new();
    h.update(&bytes[..prefix]);
    h.update(&bytes[header..]);
    bytes[prefix..header].copy_from_slice(&h.finalize());
}

#[test]
fn v16_and_v9_roundtrip_exact_targets_and_older_non_ik_bytes() {
    let initial = scene(2, 2);
    let bytes = initial.to_bytes();
    assert_eq!(
        &bytes[8..12],
        &crate::scene_format::scene_format(16).to_le_bytes()
    );
    assert_eq!(CookedScene::from_bytes(&bytes).unwrap().to_bytes(), bytes);
    let mut world = NativeWorld::new(initial.clone());
    apply(&mut world, [control(1, 100, 1.5), control(2, 101, -3.)]).unwrap();
    for _ in 0..17 {
        world.step().unwrap();
    }
    let saved = world.save_bytes().unwrap();
    assert_eq!(&saved[8..12], &9u32.to_le_bytes());
    let mut restored = NativeWorld::from_save_bytes_for_scene(&saved, &initial).unwrap();
    assert_eq!(restored.ik_state(id(1)), world.ik_state(id(1)));
    assert_eq!(restored.save_bytes().unwrap(), saved);
    for _ in 0..31 {
        world.step().unwrap();
        restored.step().unwrap();
    }
    assert_eq!(restored.save_bytes().unwrap(), world.save_bytes().unwrap());
    let mut plain = node(1, 1);
    plain.ik = None;
    let no_ik = CookedScene::new(id(0), 60, vec![initial.entities()[0].clone()])
        .unwrap()
        .with_spatial(initial.visual_assets().to_vec(), vec![plain])
        .unwrap();
    assert_eq!(
        &no_ik.to_bytes()[8..12],
        &crate::scene_format::scene_format(11).to_le_bytes()
    );
    let plain = NativeWorld::new(no_ik).save_bytes().unwrap();
    assert_eq!(&plain[8..12], &4u32.to_le_bytes());
    assert_eq!(
        NativeWorld::from_save_bytes(&plain)
            .unwrap()
            .save_bytes()
            .unwrap(),
        plain
    );
}
#[test]
fn invalid_control_late_in_batch_retains_every_owner_and_arc() {
    let mut world = NativeWorld::new(scene(2, 2));
    let before = world.save_bytes().unwrap();
    let arc = world.ik_state(id(1)).unwrap().clone();
    assert!(apply(&mut world, [control(1, 100, 1.7), control(2, 999, 2.)]).is_err());
    assert_eq!(world.save_bytes().unwrap(), before);
    assert!(Arc::ptr_eq(&arc, world.ik_state(id(1)).unwrap()));
    assert!(apply(&mut world, [control(1, 100, f64::NAN)]).is_err());
    assert_eq!(world.save_bytes().unwrap(), before);
}
#[test]
fn final_binding_controls_replacement_removal_and_entity_reuse() {
    let mut world = NativeWorld::new(scene(1, 1));
    apply(&mut world, [control(1, 100, 4.)]).unwrap();
    let mut replacement = node(1, 1);
    Arc::make_mut(replacement.ik.as_mut().unwrap()).initial = Arc::from([sample(101)]);
    apply(
        &mut world,
        [
            control(1, 101, 2.),
            StructuralCommand::SetSpatial {
                entity: id(1),
                spatial: Some(replacement),
            },
        ],
    )
    .unwrap();
    assert_eq!(world.ik_state(id(1)).unwrap()[0].id, id(101));
    assert_eq!(world.ik_state(id(1)).unwrap()[0].target[0], 2.);
    apply(&mut world, [StructuralCommand::Despawn(id(1))]).unwrap();
    assert!(world.ik_state(id(1)).is_none());
    apply(
        &mut world,
        [
            StructuralCommand::Spawn(CookedEntity {
                id: id(1),
                parent: None,
                transform: Transform::default(),
                velocity: None,
            }),
            StructuralCommand::SetSpatial {
                entity: id(1),
                spatial: Some(node(1, 1)),
            },
        ],
    )
    .unwrap();
    assert_eq!(world.ik_state(id(1)).unwrap()[0], sample(100));
    apply(
        &mut world,
        [StructuralCommand::SetSpatial {
            entity: id(1),
            spatial: None,
        }],
    )
    .unwrap();
    assert!(world.ik_state(id(1)).is_none());
}
#[test]
fn malformed_rows_recomputed_checksums_and_foreign_binding_refuse_atomically() {
    let initial = scene(1, 1);
    let mut live = NativeWorld::new(initial.clone());
    let original = live.save_bytes().unwrap();
    for offset in [0usize, 16, 32, 38] {
        // owner, constraint, chain, first target
        let mut bad = original.clone();
        let start = bad.len() - 94;
        bad[start + offset] ^= 0x7f;
        if offset == 38 {
            bad[start + 38..start + 46].copy_from_slice(&f64::NAN.to_le_bytes());
        }
        checksum(&mut bad, 32, 64);
        assert!(live.restore_bytes(&bad).is_err());
        assert_eq!(live.save_bytes().unwrap(), original);
    }
    let mut foreign = scene(1, 1).spatial()[0].clone();
    Arc::make_mut(foreign.ik.as_mut().unwrap()).initial = Arc::from([sample(444)]);
    let foreign = CookedScene::new(id(0), 60, initial.entities().to_vec())
        .unwrap()
        .with_spatial(initial.visual_assets().to_vec(), vec![foreign])
        .unwrap();
    let mut forged = NativeWorld::new(foreign).save_bytes().unwrap();
    forged[64..96].copy_from_slice(&original[64..96]);
    checksum(&mut forged, 32, 64);
    assert!(matches!(
        NativeWorld::from_save_bytes_for_scene(&forged, &initial),
        Err(SaveError::Invalid("IK original constraint binding"))
    ));
}
#[test]
fn aggregate_caps_and_binary_counts_refuse_before_constraint_allocation() {
    let full = scene(128, 32);
    assert_eq!(full.spatial().len(), 128);
    let mut nodes = full.spatial().to_vec();
    nodes[0].ik = Some(Arc::new(NativeIk {
        mesh_fingerprint: [7; 32],
        initial: (0..33).map(|i| sample(i + 100)).collect(),
    }));
    assert!(
        full.clone()
            .with_spatial(full.visual_assets().to_vec(), nodes)
            .is_err()
    );
    let entities = (1..=129)
        .map(|i| CookedEntity {
            id: id(i),
            parent: None,
            transform: Transform::default(),
            velocity: None,
        })
        .collect();
    let over = CookedScene::new(id(0), 60, entities).unwrap();
    assert!(
        over.with_spatial(
            full.visual_assets().to_vec(),
            (1..=129).map(|i| node(i, 32)).collect()
        )
        .is_err()
    );
    let mut bytes = scene(1, 1).to_bytes();
    let count = bytes.len() - 78 - 4;
    bytes[count..count + 4].copy_from_slice(&u32::MAX.to_le_bytes());
    checksum(&mut bytes, 40, 72);
    assert!(CookedScene::from_bytes(&bytes).is_err());
}
#[test]
fn unchanged_and_control_updated_allocation_counts_are_separate() {
    let mut world = NativeWorld::new(scene(1, 2));
    let mut rows = Vec::with_capacity(2);
    world.snapshot_ik(&mut rows).unwrap();
    let (_, (unchanged, unchanged_bytes)) = alloc::measured(|| {
        for _ in 0..128 {
            world.snapshot_ik(&mut rows).unwrap();
            for n in world.spatial_view().unwrap().nodes() {
                std::hint::black_box(n.animation_ik);
            }
        }
    });
    assert_eq!((unchanged, unchanged_bytes), (0, 0));
    let mut q = FrameCommands::default();
    q.push(control(1, 100, 0.)).unwrap();
    q.clear();
    let (_, (moving, moving_bytes)) = alloc::measured(|| {
        for i in 0..128 {
            q.push(control(1, 100, i as f64)).unwrap();
            q.push(control(1, 101, i as f64)).unwrap();
            world.apply(&mut q).unwrap();
        }
    });
    println!(
        "IK allocations: unchanged128={unchanged}, moving128_two_controls={moving}, cumulative_requested_bytes={moving_bytes}"
    );
    assert!(moving > 0);
    assert!(moving <= 128 * 8, "bounded one-owner batches: {moving}");
}

#[test]
fn warmed_moving_target_allocation_scaling_and_batch_coalescing() {
    for (owners, constraints) in [(1, 1), (1, 32), (8, 1), (8, 32), (128, 32)] {
        let mut world = NativeWorld::new(scene(owners, constraints));
        let mut q = FrameCommands::default();
        // Warm the queue and all existing world caches before measurement.
        for _ in 0..2 {
            for owner in 1..=owners {
                q.push(control(owner as u128, 100, 0.)).unwrap();
                q.push(control(owner as u128, 100, 1.)).unwrap();
            }
            world.apply(&mut q).unwrap();
        }
        let mut previous = Vec::with_capacity(owners);
        let (_, (count, bytes)) = alloc::measured(|| {
            for tick in 0..128 {
                previous.clear();
                for owner in 1..=owners {
                    previous.push(world.ik_state(id(owner as u128)).unwrap().clone());
                }
                for owner in 1..=owners {
                    q.push(control(owner as u128, 100, tick as f64)).unwrap();
                    q.push(control(owner as u128, 100, tick as f64 + 0.5))
                        .unwrap();
                }
                world.apply(&mut q).unwrap();
                for owner in 1..=owners {
                    assert_eq!(
                        world.ik_state(id(owner as u128)).unwrap()[0].target[0],
                        tick as f64 + 0.5
                    );
                    assert_eq!(
                        previous[owner - 1][0].target[0],
                        if tick == 0 { 1. } else { tick as f64 - 0.5 }
                    );
                }
            }
        });
        println!(
            "IK moving: owners={owners}, constraints_per_owner={constraints}, batches=128, commands_per_owner=2, allocations={count}, cumulative_requested_bytes={bytes}, sample_size={}",
            std::mem::size_of::<AnimationIkSample>()
        );
        assert!(count >= 128 * owners);
        assert!(count <= 128 * (2 * owners + 1));
    }
}
