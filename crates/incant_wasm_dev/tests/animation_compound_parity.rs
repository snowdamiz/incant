#![cfg(feature = "static-conformance")]
#[path = "../../incant_wasm_runtime/tests/support/animation_compound.rs"]
mod fixture;
use incant_runtime::*;
use incant_wasm_dev::DevModuleHost;
use incant_wasm_runtime::{StaticModule, embedded};
fn guest(name: &str) -> Vec<u8> {
    std::fs::read(std::path::Path::new(env!("INCANT_RUST_SDK_GUESTS")).join(format!("{name}.wasm")))
        .unwrap()
}
#[test]
fn complete_compound_groups_outgoing_clocks_and_ik_match_native_oracle_in_both_hosts() {
    for ik in [false, true] {
        let name = if ik {
            "rust-animation-compound"
        } else {
            "rust-animation-compound-standalone"
        };
        let mut native = StaticModule::new(embedded::linked(name).unwrap()).unwrap();
        let mut dev = DevModuleHost::new(&guest(name)).unwrap();
        let mut a = NativeWorld::new(scene(ik));
        let mut b = NativeWorld::new(scene(ik));
        let mut oracle = NativeWorld::new(scene(ik));
        let mut sixteen = false;
        let mut compound = false;
        for turn in 0..96 {
            a.step().unwrap();
            b.step().unwrap();
            oracle.step().unwrap();
            let mut rows = Vec::new();
            oracle.snapshot_animations(&mut rows).unwrap();
            sixteen |= rows
                .iter()
                .any(|r| r.pose.is_some_and(|p| p.samples().len() == 16));
            compound |= rows
                .iter()
                .any(|r| r.pose.is_some_and(|p| p.outer_weight().is_some()));
            fixture::oracle(&mut oracle, turn);
            native
                .tick(&mut a, 1. / 60.)
                .unwrap_or_else(|e| panic!("static IK={ik} turn={turn}: {e}"));
            dev.tick(&mut b, 1. / 60.)
                .unwrap_or_else(|e| panic!("dev IK={ik} turn={turn}: {e}"));
            assert_eq!(
                a.save_bytes().unwrap(),
                oracle.save_bytes().unwrap(),
                "static{turn}"
            );
            assert_eq!(
                b.save_bytes().unwrap(),
                oracle.save_bytes().unwrap(),
                "dev{turn}"
            );
            assert_eq!(
                native.logs().unwrap().entries(),
                dev.logs().unwrap().entries()
            );
            if turn == 35 {
                dev.reload(&guest(name)).unwrap();
            }
        }
        assert!(sixteen && compound);
    }
}

fn scene(ik: bool) -> CookedScene {
    let scene = fixture::scene();
    if ik {
        scene
    } else {
        let mut nodes = scene.spatial().to_vec();
        for n in &mut nodes {
            n.ik = None;
        }
        scene
            .clone()
            .with_spatial(scene.visual_assets().to_vec(), nodes)
            .unwrap()
    }
}
fn compound_world(ik: bool) -> NativeWorld {
    compound_world_sized(ik, 0)
}
fn compound_world_sized(ik: bool, extra: usize) -> NativeWorld {
    let scene = scene(ik);
    let scene = if extra == 0 {
        scene
    } else {
        let mut nodes = scene.spatial().to_vec();
        let mut entities = scene.entities().to_vec();
        for n in 0..extra {
            let mut node = nodes[0].clone();
            node.entity = fixture::id(1000 + n as u128);
            entities.push(CookedEntity {
                id: node.entity,
                parent: None,
                transform: incant_types::Transform::default(),
                velocity: None,
            });
            nodes.push(node);
        }
        CookedScene::new(fixture::id(99), 60, entities)
            .unwrap()
            .with_spatial(scene.visual_assets().to_vec(), nodes)
            .unwrap()
    };
    let mut world = NativeWorld::new(scene);
    for _ in 0..8 {
        world.step().unwrap();
    }
    let mut rows = Vec::new();
    world.snapshot_animations(&mut rows).unwrap();
    assert!(
        rows.iter()
            .any(|r| r.pose.is_some_and(|p| p.outer_weight().is_some()))
    );
    world
}
#[test]
fn caught_old_imports_and_late_failures_rollback_full_prefix_with_compound_and_ik() {
    for mode in [
        "capacity",
        "graph_capacity",
        "legacy",
        "v2",
        "v3",
        "v4",
        "v5",
        "ik_v1",
        "graph_v1",
        "number",
        "coordinate",
        "weight",
        "constraint",
        "owner",
        "quota",
        "late",
        "callback",
        "trap",
    ] {
        let name = format!("rust-animation-compound-{}", mode.replace('_', "-"));
        let mut native = StaticModule::new(embedded::linked(&name).unwrap()).unwrap();
        let mut dev = DevModuleHost::new(&guest(&name)).unwrap();
        let mut a = compound_world(true);
        let mut b = compound_world(true);
        let before = a.save_bytes().unwrap();
        assert!(native.tick(&mut a, 1. / 60.).is_err(), "static {mode}");
        assert!(dev.tick(&mut b, 1. / 60.).is_err(), "dev {mode}");
        assert_eq!(a.save_bytes().unwrap(), before, "static {mode}");
        assert_eq!(b.save_bytes().unwrap(), before, "dev {mode}");
        assert!(native.is_failed() && dev.is_failed());
        assert!(native.logs().is_err() && dev.logs().is_err());
        assert!(native.tick(&mut a, 1. / 60.).is_err() && dev.tick(&mut b, 1. / 60.).is_err());
    }
}
#[test]
fn identical_prefix_without_injected_failure_commits_all_effects() {
    let name = "rust-animation-compound-prefix";
    let mut native = StaticModule::new(embedded::linked(name).unwrap()).unwrap();
    let mut dev = DevModuleHost::new(&guest(name)).unwrap();
    let mut a = compound_world(true);
    let mut b = compound_world(true);
    let before = a.save_bytes().unwrap();
    native.tick(&mut a, 1. / 60.).unwrap();
    dev.tick(&mut b, 1. / 60.).unwrap();
    assert_ne!(a.save_bytes().unwrap(), before);
    assert_eq!(a.save_bytes().unwrap(), b.save_bytes().unwrap());
    assert_eq!(a.ik_state(fixture::id(1)).unwrap()[0].target, [9., 1., 0.]);
    assert!(a.entity_identity(fixture::id(77)).is_some());
    assert_eq!(native.logs().unwrap().entries().len(), 1);
    assert_eq!(
        native.logs().unwrap().entries(),
        dev.logs().unwrap().entries()
    );
}
#[test]
fn new_protocol_versions_and_signatures_remain_closed() {
    for import in [
        r#"(import "incant_animation_v6" "snapshot_len" (func(param i64)(result i32)))"#,
        r#"(import "incant_animation_ik_v3" "snapshot_len" (func(param i64)(result i32)))"#,
        r#"(import "incant_animation_graph_v3" "snapshot_len" (func(param i64)(result i32)))"#,
        r#"(import "incant_animation_v5" "read_snapshot" (func(param i64 i32)(result i32)))"#,
        r#"(import "incant_animation_ik_v2" "read_snapshot" (func(param i64 i32)(result i32)))"#,
        r#"(import "incant_animation_graph_v2" "read_snapshot" (func(param i64 i32)(result i32)))"#,
        r#"(import "incant_animation_graph_v2" "control" (func(param i64 i32 i32)))"#,
    ] {
        let bytes=wat::parse_str(format!("(module {import} (memory(export \"memory\")1) (func(export \"incant_abi\")(result i32)i32.const 1)(func(export \"update\")(param i64 f64)))")).unwrap();
        assert!(DevModuleHost::new(&bytes).is_err());
        assert!(incant_wasm_cook::prepare(&bytes).is_err());
    }
}
#[test]
fn raw_new_imports_enforce_every_capacity_epoch_and_shared_quota_on_both_hosts() {
    let bytes = wat::parse_str(include_str!(
        "../../incant_wasm_runtime/fixtures/animation-compound.wat"
    ))
    .unwrap();
    for mode in 0..39 {
        let mut native =
            StaticModule::new(embedded::linked("animation-compound").unwrap()).unwrap();
        let mut dev = DevModuleHost::new(&bytes).unwrap();
        let extra = if (22..=24).contains(&mode) { 12 } else { 0 };
        let mut a = compound_world_sized(mode >= 25, extra);
        let mut b = compound_world_sized(mode >= 25, extra);
        let before = a.save_bytes().unwrap();
        let n = native.tick(&mut a, f64::from(mode + 1) / 64.);
        let d = dev.tick(&mut b, f64::from(mode + 1) / 64.);
        if mode == 0 {
            n.unwrap();
            d.unwrap();
            assert_ne!(a.save_bytes().unwrap(), before);
            assert_eq!(a.save_bytes().unwrap(), b.save_bytes().unwrap());
        } else {
            let n = n.unwrap_err().to_string();
            let d = d.unwrap_err().to_string();
            if (19..=21).contains(&mode) {
                assert!(n.contains("host call quota"), "{n}");
                assert!(d.contains("host call quota"), "{d}");
            }
            if (22..=24).contains(&mode) {
                assert!(n.contains("transfer quota"), "{n}");
                assert!(d.contains("transfer quota"), "{d}");
            }
            assert_eq!(a.save_bytes().unwrap(), before, "static mode{mode}");
            assert_eq!(b.save_bytes().unwrap(), before, "dev mode{mode}");
        }
    }
}
