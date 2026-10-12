#![cfg(feature = "static-conformance")]
use incant_runtime::*;
use incant_types::Transform;
use incant_wasm_dev::DevModuleHost;
use incant_wasm_runtime::{StaticModule, embedded};
const DT: f64 = 1. / 60.;
fn scene() -> CookedScene {
    CookedScene::new(
        StableId([0; 16]),
        64,
        vec![CookedEntity {
            id: StableId([1; 16]),
            parent: None,
            transform: Transform::default(),
            velocity: None,
        }],
    )
    .unwrap()
}
fn module(name: &str) -> Vec<u8> {
    if name.starts_with("rust-") {
        std::fs::read(
            std::path::Path::new(env!("INCANT_RUST_SDK_GUESTS")).join(format!("{name}.wasm")),
        )
        .unwrap()
    } else {
        wat::parse_file(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../incant_wasm_runtime/fixtures")
                .join(format!("{name}.wat")),
        )
        .unwrap()
    }
}
fn expected() -> [f64; 3] {
    let agents = vec![
        SteeringAgent {
            id: StableId(9u128.to_be_bytes()),
            position: [-1., 0., 0.],
            velocity: [1., 0.],
            preferred_velocity: [1., 0.],
            radius: 0.3,
            height: 1.8,
            max_speed: 1.,
            responsibility: 1.,
        },
        SteeringAgent {
            id: StableId(2u128.to_be_bytes()),
            position: [1., 0., 0.],
            velocity: [-1., 0.],
            preferred_velocity: [-1., 0.],
            radius: 0.3,
            height: 1.8,
            max_speed: 1.,
            responsibility: 1.,
        },
    ];
    let obstacles = vec![SteeringObstacle {
        id: StableId(7u128.to_be_bytes()),
        vertices: vec![[0., -2.], [3., -2.], [3., -1.], [0., -1.]],
        closed: true,
        min_y: 0.,
        max_y: 2.,
    }];
    let result = NativeWorld::new(scene())
        .with_numeric_physics(|_, query| {
            query.steer_agents(
                &SteeringQuery {
                    agents,
                    obstacles,
                    time_horizon: 2.,
                    obstacle_time_horizon: 2.,
                    margin: 0.05,
                },
                DT,
            )
        })
        .unwrap()
        .unwrap();
    [
        result[0].velocity[0] as f64,
        result[0].velocity[1] as f64,
        result[1].velocity[0] as f64,
    ]
}
#[test]
fn actual_rust_guest_copied_steering_matches_native_bulk_oracle_on_both_hosts() {
    let name = "rust-steering";
    let mut native = StaticModule::new(embedded::linked(name).unwrap()).unwrap();
    let mut dev = DevModuleHost::new(&module(name)).unwrap();
    let (mut a, mut b) = (NativeWorld::new(scene()), NativeWorld::new(scene()));
    let expected = expected();
    for tick in 1..=3 {
        native.tick(&mut a, DT).unwrap();
        dev.tick(&mut b, DT).unwrap();
        assert_eq!(a.save_bytes().unwrap(), b.save_bytes().unwrap());
        assert_eq!(
            a.inspect(StableId([1; 16])).unwrap().transform.translation,
            expected
        );
        assert_eq!(
            native.logs().unwrap().entries(),
            dev.logs().unwrap().entries()
        );
        assert_eq!(native.logs().unwrap().entries().len(), tick);
        assert_eq!(
            native.logs().unwrap().entries()[tick - 1].message,
            "steering accepted"
        );
        if tick == 2 {
            dev.reload(&module(name)).unwrap();
        }
    }
}
#[test]
fn actual_ignored_sdk_errors_or_callback_failure_reject_numeric_structural_and_log_prefixes() {
    for mode in [
        "capacity",
        "number",
        "domain",
        "rounding",
        "duplicate",
        "count",
        "quota",
        "callback",
    ] {
        let name = format!("rust-steering-{mode}");
        let mut native = StaticModule::new(embedded::linked(&name).unwrap()).unwrap();
        let mut dev = DevModuleHost::new(&module(&name)).unwrap();
        let (mut a, mut b) = (NativeWorld::new(scene()), NativeWorld::new(scene()));
        let before = a.save_bytes().unwrap();
        assert!(native.tick(&mut a, DT).is_err(), "static {mode}");
        assert!(dev.tick(&mut b, DT).is_err(), "dev {mode}");
        assert_eq!(a.save_bytes().unwrap(), before);
        assert_eq!(b.save_bytes().unwrap(), before);
        assert!(native.is_failed() && dev.is_failed());
        assert!(native.logs().is_err() && dev.logs().is_err());
    }
    assert!(StaticModule::new(embedded::linked("steering-start").unwrap()).is_err());
    assert!(DevModuleHost::new(&module("steering-start")).is_err());
}

#[test]
fn raw_import_pointer_length_epoch_and_frame_failures_preserve_both_host_worlds() {
    for mode in 2..=10 {
        let mut native = StaticModule::new(embedded::linked("steering").unwrap()).unwrap();
        let mut dev = DevModuleHost::new(&module("steering")).unwrap();
        let (mut a, mut b) = (NativeWorld::new(scene()), NativeWorld::new(scene()));
        let before = a.save_bytes().unwrap();
        assert!(
            native.tick(&mut a, mode as f64 * DT).is_err(),
            "static {mode}"
        );
        assert!(dev.tick(&mut b, mode as f64 * DT).is_err(), "dev {mode}");
        assert_eq!(a.save_bytes().unwrap(), before);
        assert_eq!(b.save_bytes().unwrap(), before);
        assert!(native.is_failed() && dev.is_failed());
    }
    // A valid empty query and an unaligned copied destination use the same ABI.
    let mut native = StaticModule::new(embedded::linked("steering").unwrap()).unwrap();
    let mut dev = DevModuleHost::new(&module("steering")).unwrap();
    let (mut a, mut b) = (NativeWorld::new(scene()), NativeWorld::new(scene()));
    native.tick(&mut a, DT).unwrap();
    dev.tick(&mut b, DT).unwrap();
    assert_eq!(a.save_bytes().unwrap(), b.save_bytes().unwrap());
    assert_eq!(
        a.inspect(StableId([1; 16])).unwrap().transform.translation[0],
        99.
    );
}
