use super::{NAMES, Trace};
use crate::{CompiledScript, ScriptBindings, ScriptHost, ScriptInstance, ScriptValue};
use incant_runtime::{CookedEntity, CookedScene, NativeWorld, StableId};
use incant_types::Transform;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path};

const COUNT: usize = 9999;
const TICKS: u64 = 128;
const STRESS_SHA: &str = "7285e9009633052ba86ee0996c87f58097eb47fdaf0a73df10f42597b0119f1a";

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn id(n: usize) -> StableId {
    StableId(
        format!("01CSEN{:020}", 100000 + n)
            .parse::<ulid::Ulid>()
            .unwrap()
            .to_bytes(),
    )
}
fn source() -> StableId {
    StableId(
        "01CSSC00000000000000000002"
            .parse::<ulid::Ulid>()
            .unwrap()
            .to_bytes(),
    )
}
fn fixture(javascript: &str) -> (NativeWorld, ScriptHost) {
    let scene = CookedScene::new(
        StableId([99; 16]),
        60,
        (0..COUNT)
            .map(|n| CookedEntity {
                id: id(n),
                parent: None,
                transform: Transform::default(),
                velocity: None,
            })
            .collect(),
    )
    .unwrap();
    let bindings = ScriptBindings::new(
        Sha256::digest(scene.to_bytes()).into(),
        vec![source()],
        (0..COUNT)
            .map(|n| ScriptInstance {
                entity: id(n),
                source: source(),
                props: BTreeMap::from([("ordinal".into(), ScriptValue::Number(n as f64))]),
            })
            .collect(),
    )
    .unwrap();
    let code = format!(
        "const inner=(function(exports){{{javascript};return exports.default;}})({{}});\nexports.default={{sources:[{{id:'01CSSC00000000000000000002',behavior:inner}}]}};"
    );
    let script =
        CompiledScript::compile_project("core-script-phase-probe", &code, bindings).unwrap();
    let world = NativeWorld::new(scene);
    let host = ScriptHost::for_world(script, &world).unwrap();
    (world, host)
}
fn position(n: usize, tick: u64) -> [f64; 3] {
    let phase = ((tick % 64) as f64 - 32.) / 64.;
    [
        (n % 97) as f64 - 48. + phase,
        (n / 97 % 97) as f64 - 48.,
        (n % 13) as f64 - 6. - phase,
    ]
}
fn rows_match(world: &NativeWorld, tick: u64) -> bool {
    let mut sum_x = 0.;
    let mut sum_z = 0.;
    for n in 0..COUNT {
        let expected = position(n, tick);
        sum_x += expected[0];
        sum_z += expected[2];
        let state = world.inspect(id(n)).unwrap();
        if n != 0 && state.transform.translation != expected {
            return false;
        }
        if state.transform.rotation != [0., 0., 0., 1.] || state.transform.scale != [1.; 3] {
            return false;
        }
    }
    let actual = world.inspect(id(0)).unwrap().transform.translation;
    let expected = [COUNT as f64, sum_x / COUNT as f64, sum_z / COUNT as f64];
    actual
        .into_iter()
        .zip(expected)
        .all(|(a, b)| a.is_finite() && (a - b).abs() <= 1e-12)
}

#[test]
#[ignore = "Explicit CPU-only diagnostic;128 actual native ticks, not a shipping performance gate"]
fn actual_native_stress_regions_128() {
    let root = std::env::var("INCANT_CORE_PHASE_INPUT").expect("run retained prepare.mjs first");
    let root = Path::new(&root);
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["source_sha256"], STRESS_SHA);
    assert_eq!(
        digest(&fs::read(root.join("stress.ts")).unwrap()),
        STRESS_SHA
    );
    let javascript = fs::read_to_string(root.join("baseline.js")).unwrap();
    assert_eq!(
        digest(javascript.as_bytes()),
        manifest["files"]["baseline.js"].as_str().unwrap()
    );
    let (mut world, mut host) = fixture(&javascript);
    let capture = incant_profile::Capture::start().unwrap();
    let mut frames = Vec::with_capacity(TICKS as usize);
    for tick in 1..=TICKS {
        let trace = Trace::start();
        host.tick(&mut world, 1. / 60.).unwrap();
        let (regions, outer_ns) = trace.finish();
        assert!(
            rows_match(&world, tick),
            "actual native row mismatch at{tick}"
        );
        assert_eq!(
            world.tick(),
            0,
            "script-only probe must not secretly step the world"
        );
        frames.push(
            serde_json::json!({"tick":tick,"regions_ns":regions,"outer_ns":outer_ns,
            "tail_ns":outer_ns-regions.iter().sum::<u64>()}),
        );
    }
    let profile = capture.report();
    drop(capture);
    let profile = serde_json::to_value(profile).unwrap();
    assert_eq!(profile["quickjs_heap"]["samples"], TICKS);
    assert_eq!(profile["scripts"].as_array().unwrap().len(), 1);
    assert_eq!(profile["scripts"][0]["totals"]["calls"], TICKS);
    assert_eq!(profile["scripts"][0]["totals"]["unwound"], 0);
    for variant in ["omit-x", "omit-y", "omit-z"] {
        let bytes = fs::read(root.join(format!("{variant}.js"))).unwrap();
        assert_eq!(
            digest(&bytes),
            manifest["files"][format!("{variant}.js")].as_str().unwrap()
        );
        let (mut world, mut host) = fixture(std::str::from_utf8(&bytes).unwrap());
        host.tick(&mut world, 1. / 60.).unwrap();
        assert!(
            !rows_match(&world, 1),
            "omitted write escaped oracle:{variant}"
        );
    }
    println!(
        "CORE_SCRIPT_PHASE {}",
        serde_json::json!({"diagnostic_only":true,
        "ticks":TICKS,"rows":COUNT,"source_sha256":STRESS_SHA,"prepared":manifest,
        "region_names":NAMES,"frames":frames,"profile":profile,
        "negative_controls":["omit-x","omit-y","omit-z"],
        "scope":"CI native ScriptHost, one unchanged Core stress source and9999 transforms; no Core controller/assets/physics/UI/audio, no shipping or phase-gate claim"})
    );
}
