use incant_runtime::{
    CookedEntity, CookedScene, FrameCommands, NativeWorld, StableId, StructuralCommand,
};
use incant_types::{CoroutineValue as V, Transform};
use incant_wasm_abi as abi;
use incant_wasm_runtime::{
    Call, Capability,
    instances::Catalog,
    persistent::{Declaration, PersistentSnapshot},
};
use std::{collections::BTreeMap, sync::Arc};
fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
fn scene() -> CookedScene {
    CookedScene::new(
        id(9),
        60,
        (1..=3)
            .map(|n| CookedEntity {
                id: id(n),
                parent: None,
                transform: Default::default(),
                velocity: None,
            })
            .collect(),
    )
    .unwrap()
}
fn catalog(w: &NativeWorld, speed: f64) -> Arc<Catalog> {
    Arc::new(
        Catalog::new(
            w.source_scene_fingerprint(),
            BTreeMap::from([
                (id(1), BTreeMap::from([("speed".into(), V::Number(speed))])),
                (id(2), BTreeMap::from([("speed".into(), V::Number(20.))])),
            ]),
        )
        .unwrap(),
    )
}
fn configured(w: &NativeWorld, c: Arc<Catalog>) -> Capability {
    let mut cap = Capability::with_persistence(Some(Declaration::new(8, [7; 32]).unwrap()));
    cap.configure_instances(c).unwrap();
    cap.bind_instances(w).unwrap();
    cap
}
fn read(cap: &mut Capability, e: i64) -> Vec<u8> {
    let len = cap.invoke(&mut [], e, Call::InstancesLen).unwrap() as usize;
    let mut bytes = vec![0xa5; len + 2];
    cap.invoke(
        &mut bytes,
        e,
        Call::ReadInstances {
            ptr: 1,
            capacity: len as i32,
        },
    )
    .unwrap();
    assert_eq!(bytes[0], 0xa5);
    assert_eq!(bytes[len + 1], 0xa5);
    bytes[1..len + 1].to_vec()
}

#[test]
fn copied_properties_retire_owner_identity_and_survive_fresh_checkpoint_restore() {
    let initial = scene();
    let mut w = NativeWorld::new(initial.clone());
    let c = catalog(&w, 10.);
    let mut cap = configured(&w, c.clone());
    w.step().unwrap();
    let e = cap.begin(&mut w, 1. / 60.).unwrap();
    let first = read(&mut cap, e);
    assert_eq!(abi::instances::Snapshot::parse(&first).unwrap().len(), 2);
    cap.finish(&mut w).unwrap();
    let mut remove = FrameCommands::default();
    remove.push(StructuralCommand::Despawn(id(1))).unwrap();
    w.apply(&mut remove).unwrap();
    let mut spawn = FrameCommands::default();
    spawn
        .push(StructuralCommand::Spawn(CookedEntity {
            id: id(1),
            parent: None,
            transform: Default::default(),
            velocity: None,
        }))
        .unwrap();
    w.apply(&mut spawn).unwrap();
    w.step().unwrap();
    let e = cap.begin(&mut w, 1. / 60.).unwrap();
    let second = read(&mut cap, e);
    let rows: Vec<_> = abi::instances::Snapshot::parse(&second)
        .unwrap()
        .iter()
        .map(|r| r.entity)
        .collect();
    assert_eq!(rows, vec![id(2).0]);
    cap.finish(&mut w).unwrap();
    assert_eq!(
        abi::instances::Snapshot::parse(&first).unwrap().len(),
        2,
        "retained historical copy is immutable data"
    );
    let snapshot = cap.persistent_checkpoint(&w).unwrap();
    let encoded = snapshot.encode();
    assert!(encoded.starts_with(b"INCWPS04"));
    let decoded =
        PersistentSnapshot::decode(&encoded, Declaration::new(8, [7; 32]).unwrap()).unwrap();
    assert_eq!(decoded, snapshot);
    let mut restored =
        NativeWorld::from_save_bytes_for_scene(&w.save_bytes().unwrap(), &initial).unwrap();
    let mut candidate = Capability::with_persistence(Some(Declaration::new(8, [7; 32]).unwrap()));
    candidate.configure_instances(c).unwrap();
    candidate
        .restore_persistent(&restored, &decoded, 0, false)
        .unwrap();
    restored.step().unwrap();
    let e = candidate.begin(&mut restored, 1. / 60.).unwrap();
    let bytes = read(&mut candidate, e);
    assert_eq!(
        abi::instances::Snapshot::parse(&bytes)
            .unwrap()
            .iter()
            .map(|r| r.entity)
            .collect::<Vec<_>>(),
        vec![id(2).0]
    );
    candidate.finish(&mut restored).unwrap();
    let mut wrong = Capability::with_persistence(Some(Declaration::new(8, [7; 32]).unwrap()));
    wrong.configure_instances(catalog(&w, 11.)).unwrap();
    assert!(wrong.restore_persistent(&w, &decoded, 0, false).is_err());
}

#[test]
fn caught_short_copy_refuses_numeric_writes_but_stale_epoch_cannot_poison_new_tick() {
    let mut w = NativeWorld::new(scene());
    let mut cap = configured(&w, catalog(&w, 10.));
    w.step().unwrap();
    let old = cap.begin(&mut w, 1. / 60.).unwrap();
    cap.finish(&mut w).unwrap();
    w.step().unwrap();
    let e = cap.begin(&mut w, 1. / 60.).unwrap();
    assert_ne!(e, old);
    assert!(cap.invoke(&mut [], old, Call::InstancesLen).is_err());
    cap.finish(&mut w).unwrap();
    let mut w = NativeWorld::new(scene());
    let mut cap = configured(&w, catalog(&w, 10.));
    w.step().unwrap();
    let before = w.snapshot();
    let e = cap.begin(&mut w, 1. / 60.).unwrap();
    let mut changed = Vec::new();
    for _ in 0..3 {
        let mut t = Transform::default();
        t.translation[0] = 99.;
        for v in t.translation.into_iter().chain(t.rotation).chain(t.scale) {
            changed.extend(v.to_le_bytes());
        }
    }
    cap.invoke(
        &mut changed,
        e,
        Call::Column {
            chunk: 0,
            ptr: 0,
            velocity: false,
            write: true,
        },
    )
    .unwrap();
    let mut short = [0xa5; 1];
    assert!(
        cap.invoke(
            &mut short,
            e,
            Call::ReadInstances {
                ptr: 0,
                capacity: 1
            }
        )
        .is_err()
    );
    assert_eq!(short, [0xa5]);
    assert!(cap.finish(&mut w).is_err());
    assert_eq!(w.snapshot(), before);
}

#[test]
fn catalog_admission_checks_shared_value_bounds_scene_owner_and_wire_identity() {
    let w = NativeWorld::new(scene());
    for value in [
        V::Number(f64::INFINITY),
        V::String("x".repeat(1024 * 1024)),
        (0..17).fold(V::Null, |v, _| V::Array(vec![v])),
    ] {
        assert!(
            Catalog::new(
                w.source_scene_fingerprint(),
                BTreeMap::from([(id(1), BTreeMap::from([("bad".into(), value)]))])
            )
            .is_err()
        );
    }
    let c = catalog(&w, 10.);
    let frame = c.frame();
    assert_eq!(Catalog::from_frame(c.scene(), &frame).unwrap(), *c);
    for offset in [8, 20, 44] {
        let mut bad = frame.clone();
        bad[offset] = 255;
        assert!(Catalog::from_frame(c.scene(), &bad).is_err());
    }
    let mut nan = frame.clone();
    let n = nan.len();
    nan[n - 8..].copy_from_slice(&f64::NAN.to_le_bytes());
    assert!(Catalog::from_frame(c.scene(), &nan).is_err());
    let absent = Catalog::new(
        w.source_scene_fingerprint(),
        BTreeMap::from([(id(99), BTreeMap::new())]),
    )
    .unwrap();
    assert!(absent.validate_initial(&w).is_err());
    assert!(
        Catalog::from_frame([0; 32], &frame)
            .unwrap()
            .validate_world(&w)
            .is_err()
    );
}
#[test]
fn empty_legacy_host_and_catalog_quotas_remain_explicit() {
    let mut w = NativeWorld::new(scene());
    let mut plain = Capability::default();
    w.step().unwrap();
    let e = plain.begin(&mut w, 1. / 60.).unwrap();
    let bytes = read(&mut plain, e);
    assert!(abi::instances::Snapshot::parse(&bytes).unwrap().is_empty());
    plain.finish(&mut w).unwrap();
    let rows = (0..=abi::INSTANCE_MAX_ROWS)
        .map(|n| (id(n as u128), BTreeMap::new()))
        .collect();
    assert!(Catalog::new(w.source_scene_fingerprint(), rows).is_err());
    let mut w = NativeWorld::new(scene());
    let mut cap = configured(&w, catalog(&w, 10.));
    w.step().unwrap();
    let e = cap.begin(&mut w, 1. / 60.).unwrap();
    let mut refused = false;
    for _ in 0..=abi::CALLS {
        if cap.invoke(&mut [], e, Call::InstancesLen).is_err() {
            refused = true;
            break;
        }
    }
    assert!(refused);
    assert!(cap.finish(&mut w).is_err());
}

#[test]
fn module_only_persistent_bytes_keep_the_original_v2_framing() {
    let declaration = Declaration::new(8, [7; 32]).unwrap();
    let mut cap = Capability::with_persistence(Some(declaration));
    let mut world = NativeWorld::new(scene());
    world.step().unwrap();
    let epoch = cap.begin(&mut world, 0.25).unwrap();
    cap.finish(&mut world).unwrap();
    let mut expected = b"INCWPS02".to_vec();
    expected.extend(declaration.encode());
    expected.extend(1u64.to_le_bytes());
    expected.extend(epoch.to_le_bytes());
    expected.extend([0; 8]);
    expected.extend(
        abi::TimerHeader {
            magic: *abi::TIMER_STATE_MAGIC,
            tick: 1,
            elapsed_seconds: 0.25,
            count: 0,
            stride: abi::TimerStateRow::SIZE as u32,
        }
        .encode(),
    );
    let actual = cap.persistent_checkpoint(&world).unwrap().encode();
    assert_eq!(actual, expected);
    assert_eq!(
        PersistentSnapshot::decode(&expected, declaration)
            .unwrap()
            .encode(),
        expected
    );
}
