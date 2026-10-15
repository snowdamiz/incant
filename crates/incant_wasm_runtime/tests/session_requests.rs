use incant_runtime::{CookedEntity, CookedScene, NativeWorld, StableId};
use incant_types::{Transform, Velocity};
use incant_wasm_runtime::{Call, Capability};

fn world() -> NativeWorld {
    NativeWorld::new(
        CookedScene::new(
            StableId([0; 16]),
            64,
            vec![CookedEntity {
                id: StableId([1; 16]),
                parent: None,
                transform: Transform::default(),
                velocity: Some(Velocity { linear: [0.; 3] }),
            }],
        )
        .unwrap(),
    )
}
fn write_x(cap: &mut Capability, epoch: i64, x: f64) {
    let mut memory = vec![0; 80];
    cap.invoke(
        &mut memory,
        epoch,
        Call::Column {
            chunk: 0,
            ptr: 0,
            velocity: false,
            write: false,
        },
    )
    .unwrap();
    memory[..8].copy_from_slice(&x.to_le_bytes());
    cap.invoke(
        &mut memory,
        epoch,
        Call::Column {
            chunk: 0,
            ptr: 0,
            velocity: false,
            write: true,
        },
    )
    .unwrap();
}

#[test]
fn unsupported_restart_refuses_even_if_ignored_and_discards_numeric_prefix() {
    let mut world = world();
    let before = world.save_bytes().unwrap();
    let mut cap = Capability::default();
    let epoch = cap.begin(&mut world, 0.25).unwrap();
    write_x(&mut cap, epoch, 7.);
    assert!(cap.invoke(&mut [], epoch, Call::RequestRestart).is_err());
    assert!(cap.finish(&mut world).is_err());
    assert_eq!(world.save_bytes().unwrap(), before);
    assert!(cap.take_restart_request().is_err());
    assert!(cap.enable_restart_requests().is_err());
}

#[test]
fn repeated_requests_commit_once_and_callback_authority_expires_with_the_tick() {
    let mut world = world();
    let mut cap = Capability::default();
    cap.enable_restart_requests().unwrap();
    let epoch = cap.begin(&mut world, 0.25).unwrap();
    assert!(
        cap.invoke(&mut [], epoch - 1, Call::RequestRestart)
            .is_err()
    );
    write_x(&mut cap, epoch, 7.);
    for _ in 0..3 {
        cap.invoke(&mut [], epoch, Call::RequestRestart).unwrap();
    }
    assert!(cap.take_restart_request().is_err());
    cap.finish(&mut world).unwrap();
    assert_eq!(world.snapshot()[0].transform.translation[0], 7.);
    assert!(cap.take_restart_request().unwrap());
    assert!(!cap.take_restart_request().unwrap());
    assert!(cap.invoke(&mut [], epoch, Call::RequestRestart).is_err());
    cap.begin(&mut world, 0.25).unwrap();
    cap.finish(&mut world).unwrap();
    assert!(!cap.take_restart_request().unwrap());
}

#[test]
fn unconsumed_request_survives_later_accepted_ticks_until_taken() {
    let mut world = world();
    let mut cap = Capability::default();
    cap.enable_restart_requests().unwrap();
    let epoch = cap.begin(&mut world, 0.25).unwrap();
    cap.invoke(&mut [], epoch, Call::RequestRestart).unwrap();
    cap.finish(&mut world).unwrap();
    for _ in 0..2 {
        cap.begin(&mut world, 0.25).unwrap();
        cap.finish(&mut world).unwrap();
    }
    assert!(cap.take_restart_request().unwrap());
    assert!(!cap.take_restart_request().unwrap());
}

#[test]
fn request_quota_is_shared_with_structural_commands_and_failure_rolls_back() {
    for overflow in [false, true] {
        let mut world = world();
        let before = world.save_bytes().unwrap();
        let mut cap = Capability::default();
        cap.enable_restart_requests().unwrap();
        let epoch = cap.begin(&mut world, 0.25).unwrap();
        let mut memory = vec![1; 40];
        memory[16..24].copy_from_slice(&9f64.to_le_bytes());
        memory[24..40].fill(0);
        cap.invoke(&mut memory, epoch, Call::SetVelocity(0, 16))
            .unwrap();
        for _ in 1..incant_wasm_runtime::limits::COMMANDS {
            cap.invoke(&mut [], epoch, Call::RequestRestart).unwrap();
        }
        if overflow {
            assert!(cap.invoke(&mut [], epoch, Call::RequestRestart).is_err());
            assert!(cap.finish(&mut world).is_err());
            assert_eq!(world.save_bytes().unwrap(), before);
            assert!(cap.take_restart_request().is_err());
        } else {
            cap.finish(&mut world).unwrap();
            assert_eq!(world.snapshot()[0].velocity.as_ref().unwrap().linear[0], 9.);
            assert!(cap.take_restart_request().unwrap());
        }
    }
}

#[test]
fn late_world_refusal_cannot_publish_a_restart_or_numeric_writes() {
    let mut world = world();
    let before = world.save_bytes().unwrap();
    let mut cap = Capability::default();
    cap.enable_restart_requests().unwrap();
    let epoch = cap.begin(&mut world, 0.25).unwrap();
    write_x(&mut cap, epoch, 7.);
    cap.invoke(&mut [], epoch, Call::RequestRestart).unwrap();
    cap.invoke(&mut [2; 16], epoch, Call::Despawn(0)).unwrap();
    assert!(cap.finish(&mut world).is_err());
    assert_eq!(world.save_bytes().unwrap(), before);
    assert!(cap.take_restart_request().is_err());
}

#[test]
fn replacement_preserves_owner_opt_in_but_not_pending_or_committed_requests() {
    for accepted in [false, true] {
        let mut world = world();
        let mut cap = Capability::default();
        cap.enable_restart_requests().unwrap();
        let epoch = cap.begin(&mut world, 0.25).unwrap();
        cap.invoke(&mut [], epoch, Call::RequestRestart).unwrap();
        if accepted {
            cap.finish(&mut world).unwrap();
        } else {
            cap.abort();
        }
        let mut cap = cap.replacement();
        assert!(!cap.take_restart_request().unwrap());
        let epoch = cap.begin(&mut world, 0.25).unwrap();
        cap.invoke(&mut [], epoch, Call::RequestRestart).unwrap();
        cap.finish(&mut world).unwrap();
        assert!(cap.take_restart_request().unwrap());
    }
}
