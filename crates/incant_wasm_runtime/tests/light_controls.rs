#[allow(dead_code)]
#[path = "../../incant_runtime/tests/support/lights.rs"]
mod fixture;
use fixture::*;
use incant_runtime::*;
use incant_wasm_abi as abi;
use incant_wasm_runtime::{Call, Capability};
fn batch(rows: &[abi::LightCommand]) -> Vec<u8> {
    let mut b = abi::LightCommandHeader {
        magic: *abi::LIGHT_COMMAND_MAGIC,
        count: rows.len() as u32,
        stride: abi::LightCommand::SIZE as u32,
    }
    .encode()
    .to_vec();
    for r in rows {
        b.extend(r.encode());
    }
    b
}
fn row(n: u128, kind: u32, values: [f64; 3]) -> abi::LightCommand {
    abi::LightCommand {
        entity: id(n).0,
        kind,
        reserved: 0,
        values,
    }
}
fn submit(
    c: &mut Capability,
    m: &mut [u8],
    e: i64,
    b: &[u8],
) -> Result<i32, incant_wasm_runtime::BoundaryError> {
    m[32..32 + b.len()].copy_from_slice(b);
    c.invoke(
        m,
        e,
        Call::CommandLight {
            ptr: 32,
            len: b.len() as i32,
        },
    )
}
#[test]
fn light_wire_commits_exact_native_values_and_stale_epoch_refuses_without_poisoning_current_tick() {
    let mut w = NativeWorld::new(scene());
    let mut native = NativeWorld::new(scene());
    let mut c = Capability::default();
    let mut m = vec![0; 65536];
    let e = c.begin(&mut w, 1. / 60.).unwrap();
    let b = batch(&[
        row(1, 0, [0.2, 0.3, 0.4]),
        row(2, 1, [0., 0., 0.]),
        row(3, 1, [100., 0., 0.]),
    ]);
    assert!(submit(&mut c, &mut m, e - 1, &b).is_err());
    submit(&mut c, &mut m, e, &b).unwrap();
    assert_eq!(declarations(&w), declarations(&native));
    c.finish(&mut w).unwrap();
    native
        .apply(&mut queue([
            control(1, LightControl::Color([0.2, 0.3, 0.4])),
            control(2, LightControl::Intensity(0.)),
            control(3, LightControl::Intensity(100.)),
        ]))
        .unwrap();
    assert_eq!(w.save_bytes().unwrap(), native.save_bytes().unwrap());
    assert!(submit(&mut c, &mut m, e, &b).is_err());
}
#[test]
fn caught_malformed_batches_late_owner_refusal_and_shared_quotas_publish_no_prefix() {
    let valid = row(1, 1, [55., 0., 0.]);
    let mut invalids = vec![];
    for bad in [
        row(2, 1, [f64::NAN, 0., 0.]),
        row(2, 1, [1_000_000.00000001, 0., 0.]),
        row(2, 1, [1., -0., 0.]),
        row(2, 0, [1. + f64::EPSILON, 0., 0.]),
        row(2, 256, [0.; 3]),
    ] {
        invalids.push(batch(&[valid, bad]));
    }
    let mut reserved = valid;
    reserved.reserved = 1;
    invalids.push(batch(&[valid, reserved]));
    let mut truncated = batch(&[valid]);
    truncated.pop();
    invalids.push(truncated);
    for b in invalids {
        let mut w = NativeWorld::new(scene());
        let before = w.save_bytes().unwrap();
        let mut c = Capability::default();
        let mut m = vec![0; 65536];
        let e = c.begin(&mut w, 1. / 60.).unwrap();
        submit(&mut c, &mut m, e, &batch(&[valid])).unwrap();
        assert!(submit(&mut c, &mut m, e, &b).is_err());
        assert!(c.finish(&mut w).is_err());
        assert_eq!(w.save_bytes().unwrap(), before);
    }
    for n in [4, 999] {
        let mut w = NativeWorld::new(scene());
        let before = w.save_bytes().unwrap();
        let mut c = Capability::default();
        let mut m = vec![0; 65536];
        let e = c.begin(&mut w, 1. / 60.).unwrap();
        submit(&mut c, &mut m, e, &batch(&[valid, row(n, 1, [3., 0., 0.])])).unwrap();
        assert!(c.finish(&mut w).is_err());
        assert_eq!(w.save_bytes().unwrap(), before);
    }
    let mut w = NativeWorld::new(scene());
    let before = w.save_bytes().unwrap();
    let mut c = Capability::default();
    let mut m = vec![0; 65536];
    let e = c.begin(&mut w, 1. / 60.).unwrap();
    submit(
        &mut c,
        &mut m,
        e,
        &batch(&vec![valid; abi::LIGHT_MAX_COMMANDS]),
    )
    .unwrap();
    assert!(submit(&mut c, &mut m, e, &batch(&[valid])).is_err());
    assert!(c.finish(&mut w).is_err());
    assert_eq!(w.save_bytes().unwrap(), before);
}
