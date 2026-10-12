#![cfg(feature = "static-conformance")]
#[path = "../../incant_runtime/tests/support/navigation_queries.rs"]
mod fixture;
use fixture::*;
use incant_runtime::*;
use incant_wasm_dev::DevModuleHost;
use incant_wasm_runtime::{StaticModule, embedded};
const DT: f64 = 1. / 60.;
fn guest(name: &str) -> Vec<u8> {
    std::fs::read(std::path::Path::new(env!("INCANT_RUST_SDK_GUESTS")).join(format!("{name}.wasm")))
        .unwrap()
}
fn score(p: Option<NavigationPath>) -> (f64, f64) {
    let Some(p) = p else { return (0., 0.) };
    let mut sum = p.visited as f64;
    for (i, p) in p.points.iter().enumerate() {
        for (axis, &n) in p.iter().enumerate() {
            sum += f64::from(n) * (i * 3 + axis + 1) as f64;
        }
    }
    for n in p.corridor {
        sum += n as f64;
    }
    for t in p.traversals {
        assert_eq!(t.link_id, "00000000000000000000000008");
        sum += (t.from_index + t.to_index * 17 + u32::from(t.reversed) * 31 + 8) as f64;
    }
    (sum, p.generation as f64)
}
fn oracle(w: &mut NativeWorld) {
    let (mut checksum, generation) = score(w.navigation_path(id(1), &path()).unwrap());
    let q = path();
    checksum += score(
        w.navigation_path(
            id(1),
            &PathRequest {
                start: q.end,
                end: q.start,
                ..q
            },
        )
        .unwrap(),
    )
    .0;
    let g = w.grid_path(id(1), &grid()).unwrap().unwrap();
    checksum += g.cost as f64;
    for (i, c) in g.cells.iter().enumerate() {
        checksum += (i as f64 + 1.) * (c[0] as f64 + c[1] as f64 * 7.);
    }
    w.with_numeric_columns(|cols| {
        cols.for_each_chunk(|c| {
            for (entity, t) in c
                .ids
                .iter()
                .zip(c.transforms.as_chunks_mut::<10>().0.iter_mut())
            {
                if *entity == id(4) {
                    t[0] = checksum;
                    t[1] = generation + g.generation as f64;
                    t[2] += 1.;
                }
                if *entity == id(3) {
                    t[2] = 4.;
                }
            }
        })
    });
    w.apply(&mut FrameCommands::default()).unwrap();
}
#[test]
fn actual_rust_routes_match_native_oracle_and_rebuild_only_after_committed_source_motion() {
    for scene in [scene(), linked_scene(true), linked_scene(false)] {
        let mut a = NativeWorld::new(scene.clone());
        let mut b = NativeWorld::new(scene.clone());
        let mut expected = NativeWorld::new(scene);
        let mut native =
            StaticModule::new(embedded::linked("rust-navigation-paths").unwrap()).unwrap();
        let mut dev = DevModuleHost::new(&guest("rust-navigation-paths")).unwrap();
        for turn in 0..3 {
            oracle(&mut expected);
            native.tick(&mut a, DT).unwrap();
            dev.tick(&mut b, DT).unwrap();
            assert_eq!(a.snapshot(), expected.snapshot());
            assert_eq!(b.snapshot(), expected.snapshot());
            assert_eq!(
                a.navigation_path(id(1), &path()).unwrap(),
                expected.navigation_path(id(1), &path()).unwrap()
            );
            assert_eq!(
                b.navigation_path(id(1), &path()).unwrap(),
                expected.navigation_path(id(1), &path()).unwrap()
            );
            assert_eq!(
                native.logs().unwrap().entries(),
                dev.logs().unwrap().entries()
            );
            assert_eq!(native.logs().unwrap().entries().len(), turn + 1);
            if turn == 1 {
                dev.reload(&guest("rust-navigation-paths")).unwrap();
            }
        }
    }
}
#[test]
fn actual_sdk_and_native_refusals_roll_back_numeric_structural_cache_and_log_prefixes() {
    for mode in [
        "capacity",
        "grid-capacity",
        "number",
        "rounding",
        "coordinate",
        "owner",
        "grid-owner",
        "endpoint",
        "exhaustion",
        "work",
        "quota",
        "late",
        "callback",
        "trap",
    ] {
        let name = format!("rust-navigation-paths-{mode}");
        let mut native = StaticModule::new(embedded::linked(&name).unwrap()).unwrap();
        let mut dev = DevModuleHost::new(&guest(&name)).unwrap();
        let (mut a, mut b) = (NativeWorld::new(scene()), NativeWorld::new(scene()));
        let before = a.snapshot();
        let route = a.navigation_path(id(1), &path()).unwrap();
        assert!(native.tick(&mut a, DT).is_err(), "static {mode}");
        assert!(dev.tick(&mut b, DT).is_err(), "dev {mode}");
        assert_eq!(a.snapshot(), before);
        assert_eq!(b.snapshot(), before);
        assert_eq!(a.navigation_path(id(1), &path()).unwrap(), route);
        assert_eq!(b.navigation_path(id(1), &path()).unwrap(), route);
        assert!(native.is_failed() && dev.is_failed());
        assert!(native.logs().is_err() && dev.logs().is_err());
    }
}
#[test]
fn actual_raw_query_ingress_and_reply_failures_roll_back_both_hosts() {
    let wasm = wat::parse_file(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../incant_wasm_runtime/fixtures/navigation-paths.wat"),
    )
    .unwrap();
    for mode in 2..=13 {
        let mut native = StaticModule::new(embedded::linked("navigation-paths").unwrap()).unwrap();
        let mut dev = DevModuleHost::new(&wasm).unwrap();
        let (mut a, mut b) = (NativeWorld::new(scene()), NativeWorld::new(scene()));
        let before = a.snapshot();
        assert!(
            native.tick(&mut a, mode as f64 * DT).is_err(),
            "static {mode}"
        );
        assert!(dev.tick(&mut b, mode as f64 * DT).is_err(), "dev {mode}");
        assert_eq!(a.snapshot(), before);
        assert_eq!(b.snapshot(), before);
        assert!(native.logs().is_err() && dev.logs().is_err());
    }
    for mode in [1, 14, 15] {
        let mut native = StaticModule::new(embedded::linked("navigation-paths").unwrap()).unwrap();
        let mut dev = DevModuleHost::new(&wasm).unwrap();
        let (mut a, mut b) = (NativeWorld::new(scene()), NativeWorld::new(scene()));
        native.tick(&mut a, mode as f64 * DT).unwrap();
        dev.tick(&mut b, mode as f64 * DT).unwrap();
        assert_eq!(a.snapshot(), b.snapshot());
        assert_eq!(a.inspect(id(1)).unwrap().transform.translation[0], 99.);
    }
}
