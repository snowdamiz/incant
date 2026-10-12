#![cfg(feature = "static-conformance")]
#[path = "../../incant_wasm_runtime/tests/support/animation_graph.rs"]
mod fixture;
use incant_runtime::{NativeWorld, StableId};
use incant_wasm_dev::DevModuleHost;
use incant_wasm_runtime::{StaticModule, embedded};

#[test]
fn real_v1_v2_guest_imports_refuse_only_unrepresentable_blends_on_both_hosts() {
    let guest = wat::parse_file(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../incant_wasm_runtime/fixtures/animation-graph-legacy.wat"),
    )
    .unwrap();
    for parameter in [0., 0.25, 1.] {
        for mode in 1..=5 {
            let mut a = NativeWorld::new(fixture::scene(parameter));
            let mut b = NativeWorld::new(fixture::scene(parameter));
            a.step().unwrap();
            b.step().unwrap();
            let before = a.save_bytes().unwrap();
            let mut native =
                StaticModule::new(embedded::linked("animation-graph-legacy").unwrap()).unwrap();
            let mut dev = DevModuleHost::new(&guest).unwrap();
            let dt = f64::from(mode) / 60.;
            let nr = native.tick(&mut a, dt);
            let dr = dev.tick(&mut b, dt);
            if parameter == 0.25 && mode != 5 {
                let error = nr.unwrap_err().to_string();
                assert!(
                    error.contains("animation graph blend requires graph snapshot ABI"),
                    "{error}"
                );
                assert!(dr.is_err());
                assert!(native.is_failed() && dev.is_failed());
                assert!(native.logs().is_err() && dev.logs().is_err());
                assert_eq!(a.save_bytes().unwrap(), before);
                assert_eq!(b.save_bytes().unwrap(), before);
                assert!(native.tick(&mut a, dt).is_err());
                assert!(dev.tick(&mut b, dt).is_err());
            } else {
                nr.unwrap();
                dr.unwrap();
                assert_eq!(
                    a.inspect(StableId([1; 16])).unwrap().transform.translation[0],
                    77.
                );
                assert_eq!(a.save_bytes().unwrap(), b.save_bytes().unwrap());
            }
        }
    }
}
