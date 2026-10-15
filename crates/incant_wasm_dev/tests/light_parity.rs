#![cfg(feature = "static-conformance")]
#[allow(dead_code)]
#[path = "../../incant_runtime/tests/support/lights.rs"]
mod fixture;
use fixture::*;
use incant_runtime::*;
use incant_wasm_dev::DevModuleHost;
use incant_wasm_runtime::{StaticModule, embedded};
fn guest(name: &str) -> Vec<u8> {
    std::fs::read(std::path::Path::new(env!("INCANT_RUST_SDK_GUESTS")).join(format!("{name}.wasm")))
        .unwrap()
}
#[test]
fn real_rust_light_controls_match_native_on_wasm2c_and_wasmtime_without_rebinding() {
    let mut a = NativeWorld::new(scene());
    let mut b = NativeWorld::new(scene());
    let mut native = NativeWorld::new(scene());
    let name = "rust-light-controls";
    let mut compiled = StaticModule::new(embedded::linked(name).unwrap()).unwrap();
    let mut dev = DevModuleHost::new(&guest(name)).unwrap();
    for phase in 1..=8 {
        compiled.tick(&mut a, 1. / 60.).unwrap();
        dev.tick(&mut b, 1. / 60.).unwrap();
        native.with_numeric_columns(|cols| {
            cols.for_each_chunk(|c| {
                for (row, owner) in c.ids.iter().enumerate() {
                    if *owner == id(1) {
                        c.transforms[row * TRANSFORM_STRIDE] = f64::from(phase);
                    }
                }
            })
        });
        native
            .apply(&mut queue([
                control(1, LightControl::Intensity(f64::from(phase) * 100.)),
                control(2, LightControl::Color([0.25, 0.5, 0.75])),
                control(
                    3,
                    LightControl::Intensity(if phase % 3 == 0 { 0. } else { 10. }),
                ),
            ]))
            .unwrap();
        assert_eq!(a.save_bytes().unwrap(), native.save_bytes().unwrap());
        assert_eq!(b.save_bytes().unwrap(), native.save_bytes().unwrap());
        assert_eq!(a.spatial_view().unwrap().binding_generation(), 0);
    }
}
#[test]
fn caught_rust_sdk_errors_and_late_light_owner_refusal_rollback_on_both_real_hosts() {
    for suffix in ["scratch", "range", "owner"] {
        let name = format!("rust-light-{suffix}");
        let mut a = NativeWorld::new(scene());
        let mut b = NativeWorld::new(scene());
        let before = a.save_bytes().unwrap();
        let mut compiled = StaticModule::new(embedded::linked(&name).unwrap()).unwrap();
        let mut dev = DevModuleHost::new(&guest(&name)).unwrap();
        assert!(compiled.tick(&mut a, 1. / 60.).is_err());
        assert!(dev.tick(&mut b, 1. / 60.).is_err());
        assert_eq!(a.save_bytes().unwrap(), before);
        assert_eq!(b.save_bytes().unwrap(), before);
        assert!(compiled.is_failed() && dev.is_failed());
    }
}
