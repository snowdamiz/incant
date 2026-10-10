use incant_runtime::{CookedEntity, CookedScene, NativeWorld, SceneError, StableId};
use incant_types::{Transform, Velocity};
use sha2::{Digest, Sha256};

fn id(value: u128) -> StableId {
    StableId(value.to_be_bytes())
}
fn entity(value: u128, moving: bool) -> CookedEntity {
    CookedEntity {
        id: id(value),
        parent: None,
        transform: Transform::default(),
        velocity: moving.then_some(Velocity {
            linear: [2., 0., 0.],
        }),
    }
}
fn checksum(bytes: &mut [u8]) {
    let mut hash = Sha256::new();
    hash.update(&bytes[..40]);
    hash.update(&bytes[72..]);
    bytes[40..72].copy_from_slice(&hash.finalize());
}

#[test]
fn binary_round_trip_is_canonical_across_input_order_and_preserves_bits() {
    let mut child = entity(2, false);
    child.parent = Some(id(1));
    child.transform.translation[0] = -0.;
    let root = entity(1, true);
    let a = CookedScene::new(id(8), 120, vec![root.clone(), child.clone()]).unwrap();
    let b = CookedScene::new(id(8), 120, vec![child, root]).unwrap();
    assert_eq!(a.to_bytes(), b.to_bytes());
    let loaded = CookedScene::from_bytes(&a.to_bytes()).unwrap();
    assert_eq!(loaded.id(), id(8));
    assert_eq!(loaded.tick_rate(), 120);
    assert_eq!(loaded.entities(), a.entities());
    assert_eq!(loaded.to_bytes(), a.to_bytes());
    assert_eq!(
        loaded.entities()[0].transform.translation[0].to_bits(),
        (-0_f64).to_bits()
    );
}

#[test]
fn every_truncation_and_unhashed_change_is_rejected() {
    let bytes = CookedScene::new(id(1), 60, vec![entity(3, true), entity(4, false)])
        .unwrap()
        .to_bytes();
    for length in 0..bytes.len() {
        assert!(
            CookedScene::from_bytes(&bytes[..length]).is_err(),
            "length {length}"
        );
    }
    for index in 0..bytes.len() {
        let mut damaged = bytes.clone();
        damaged[index] ^= 1;
        assert!(CookedScene::from_bytes(&damaged).is_err(), "byte {index}");
    }
    let mut extra = bytes.clone();
    extra.push(0);
    assert!(CookedScene::from_bytes(&extra).is_err());
}

#[test]
fn rehashed_malformed_records_still_fail_semantic_checks() {
    let bytes = CookedScene::new(id(1), 60, vec![entity(3, true)])
        .unwrap()
        .to_bytes();
    for (offset, value) in [
        (12, 0_u32),
        (32, u32::MAX),
        (36, 0),
        (88, 7),
        (88, 0),
        (92, 7),
    ] {
        let mut bad = bytes.clone();
        bad[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        checksum(&mut bad);
        assert!(
            CookedScene::from_bytes(&bad).is_err(),
            "offset {offset} value {value}"
        );
    }
    for offset in [96, 120, 152, 176] {
        let mut bad = bytes.clone();
        bad[offset..offset + 8].copy_from_slice(&f64::NAN.to_le_bytes());
        checksum(&mut bad);
        assert!(CookedScene::from_bytes(&bad).is_err(), "NaN at {offset}");
    }
    let mut unknown = bytes;
    unknown[8..12].copy_from_slice(&2_u32.to_le_bytes());
    assert_eq!(
        CookedScene::from_bytes(&unknown).unwrap_err(),
        SceneError::Version(2)
    );
}

#[test]
fn duplicate_ids_noncanonical_order_and_cycles_are_rejected() {
    assert!(matches!(
        CookedScene::new(id(9), 60, vec![entity(1, false), entity(1, true)]),
        Err(SceneError::Duplicate(_))
    ));
    let mut a = entity(1, false);
    let mut b = entity(2, false);
    a.parent = Some(b.id);
    b.parent = Some(a.id);
    assert!(CookedScene::new(id(9), 60, vec![a, b]).is_err());
    let mut bytes = CookedScene::new(id(9), 60, vec![entity(1, false), entity(2, false)])
        .unwrap()
        .to_bytes();
    bytes[72..88].copy_from_slice(&id(3).0);
    checksum(&mut bytes);
    assert!(CookedScene::from_bytes(&bytes).is_err());
}

#[test]
fn invalid_numeric_ranges_and_missing_parents_fail_before_world_creation() {
    let mut root = entity(1, false);
    root.transform.scale = [1e308; 3];
    let mut child = entity(2, false);
    child.parent = Some(root.id);
    child.transform.scale = [1e308; 3];
    assert!(CookedScene::new(id(9), 60, vec![root, child]).is_err());
    for bad in [0., f64::INFINITY, f64::NAN] {
        let mut e = entity(1, false);
        e.transform.rotation[3] = bad;
        assert!(CookedScene::new(id(9), 60, vec![e]).is_err());
    }
    let mut orphan = entity(1, false);
    orphan.parent = Some(id(2));
    assert_eq!(
        CookedScene::new(id(9), 60, vec![orphan]).unwrap_err(),
        SceneError::Parent(id(1))
    );
}

#[test]
fn native_step_and_bulk_write_update_parent_local_and_global_values() {
    let mut root = entity(1, true);
    root.transform.translation[0] = 5.;
    root.transform.scale = [2., 2., 2.];
    let mut child = entity(2, false);
    child.parent = Some(root.id);
    child.transform.translation[0] = 1.;
    let scene = CookedScene::new(id(9), 2, vec![child, root]).unwrap();
    let mut world = NativeWorld::from_bytes(&scene.to_bytes()).unwrap();
    assert_eq!(world.inspect(id(2)).unwrap().world_transform[3][0], 7.);
    world.step().unwrap();
    assert_eq!(world.tick(), 1);
    assert_eq!(world.scene_id(), id(9));
    assert_eq!(world.entity_count(), 2);
    assert_eq!(world.inspect(id(2)).unwrap().world_transform[3][0], 8.);
    let mut visits = 0;
    world.for_each_moving_mut(|_, t, v| {
        visits += 1;
        t.translation[0] = 10.;
        v.linear[0] = 4.;
    });
    assert_eq!(visits, 1);
    assert_eq!(world.inspect(id(2)).unwrap().world_transform[3][0], 12.);
    world.step().unwrap();
    assert_eq!(world.inspect(id(2)).unwrap().world_transform[3][0], 14.);
    assert!(world.inspect(id(404)).is_none());
    assert_eq!(world.snapshot().len(), 2);
    let reset = NativeWorld::new(scene);
    assert_eq!(reset.inspect(id(2)).unwrap().world_transform[3][0], 7.);
}

#[test]
fn ten_thousand_native_entities_repeat_and_discard_without_authoring_state() {
    let scene =
        CookedScene::new(id(90), 60, (1..=10_000).map(|i| entity(i, true)).collect()).unwrap();
    let bytes = scene.to_bytes();
    let mut a = NativeWorld::from_bytes(&bytes).unwrap();
    let mut b = NativeWorld::from_bytes(&bytes).unwrap();
    for _ in 0..60 {
        a.step().unwrap();
        b.step().unwrap();
    }
    assert_eq!(a.snapshot(), b.snapshot());
    assert!((a.inspect(id(777)).unwrap().transform.translation[0] - 2.).abs() < 1e-12);
    assert_eq!(scene.to_bytes(), bytes);
    let reset = NativeWorld::from_bytes(&bytes).unwrap();
    assert_eq!(reset.tick(), 0);
    assert_eq!(reset.inspect(id(777)).unwrap().transform.translation[0], 0.);
}

#[test]
fn deeply_nested_hierarchy_loads_iteratively_and_empty_scene_steps() {
    let entities = (1..=10_000)
        .map(|i| {
            let mut e = entity(i, false);
            e.parent = (i > 1).then(|| id(i - 1));
            e.transform.translation[0] = 1.;
            e
        })
        .collect();
    let scene = CookedScene::new(id(90), 60, entities).unwrap();
    let world = NativeWorld::from_bytes(&scene.to_bytes()).unwrap();
    assert_eq!(
        world.inspect(id(10_000)).unwrap().world_transform[3][0],
        10_000.
    );
    let mut empty = NativeWorld::new(CookedScene::new(id(91), 60, vec![]).unwrap());
    empty.step().unwrap();
    assert!(empty.snapshot().is_empty());
}
