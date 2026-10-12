use incant_runtime::{CookedScene, NativeWorld, StableId};
use incant_wasm_runtime::{Call, Capability};

#[test]
fn copied_log_is_owned_and_ignored_ingress_failure_cannot_commit_or_refund_history() {
    let mut world = NativeWorld::new(CookedScene::new(StableId([0; 16]), 64, vec![]).unwrap());
    let mut cap = Capability::default();
    let mut bytes = b"accepted".to_vec();
    let epoch = cap.begin(&mut world, 0.25).unwrap();
    cap.invoke(
        &mut bytes,
        epoch,
        Call::Log {
            level: 1,
            ptr: 0,
            len: 8,
        },
    )
    .unwrap();
    bytes.fill(b'x');
    cap.finish(&mut world).unwrap();
    assert_eq!(cap.logs().unwrap().entries()[0].message, "accepted");
    let epoch = cap.begin(&mut world, 0.25).unwrap();
    cap.invoke(
        &mut bytes,
        epoch,
        Call::Log {
            level: 1,
            ptr: 0,
            len: 8,
        },
    )
    .unwrap();
    assert!(
        cap.invoke(
            &mut bytes,
            epoch,
            Call::Log {
                level: 4,
                ptr: 0,
                len: 8
            }
        )
        .is_err()
    );
    assert!(cap.finish(&mut world).is_err());
    assert!(cap.logs().is_err());
    let mut cap = cap.replacement();
    assert_eq!(cap.logs().unwrap().entries().len(), 1);
    let epoch = cap.begin(&mut world, 0.25).unwrap();
    cap.invoke(
        &mut bytes,
        epoch,
        Call::Log {
            level: 1,
            ptr: 0,
            len: 0,
        },
    )
    .unwrap();
    cap.finish(&mut world).unwrap();
    assert_eq!(cap.logs().unwrap().entries()[1].tick, 2);
}
