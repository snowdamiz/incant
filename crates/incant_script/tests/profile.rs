#![cfg(all(feature = "profile", feature = "compiler"))]
use incant_profile::{Capture, HeapMode};
use incant_runtime::{CookedEntity, CookedScene, NativeWorld, StableId};
use incant_script::{CompiledScript, ScriptBindings, ScriptHost, ScriptInstance};
use incant_types::Transform;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
#[test]
fn actual_source_groups_have_numeric_identity_with_heap_and_rollback_observations() {
    for mode in [HeapMode::Maintained, HeapMode::Detailed] {
        check_mode(mode);
    }
}
fn check_mode(mode: HeapMode) {
    let id = |n: u128| StableId(n.to_be_bytes());
    let initial = CookedScene::new(
        id(99),
        60,
        vec![
            CookedEntity {
                id: id(1),
                parent: None,
                transform: Transform::default(),
                velocity: None,
            },
            CookedEntity {
                id: id(2),
                parent: None,
                transform: Transform::default(),
                velocity: None,
            },
        ],
    )
    .unwrap();
    let bindings = ScriptBindings::new(
        Sha256::digest(initial.to_bytes()).into(),
        vec![id(10), id(20)],
        vec![
            ScriptInstance {
                entity: id(1),
                source: id(10),
                props: BTreeMap::new(),
            },
            ScriptInstance {
                entity: id(2),
                source: id(20),
                props: BTreeMap::new(),
            },
        ],
    )
    .unwrap();
    let code = format!(
        "exports.default = {{sources:[{{id:{:?},behavior:{{update(ctx){{for(const c of ctx.chunks)c.transforms[0]+=1;}}}}}},{{id:{:?},behavior:{{update(ctx){{if(ctx.clock.tick===2)throw Error('private gameplay payload');}}}}}}]}};",
        ulid::Ulid::from(10_u128).to_string(),
        ulid::Ulid::from(20_u128).to_string()
    );
    let code =
        CompiledScript::compile_project("private-user-filename.cjs", &code, bindings).unwrap();
    let mut world = NativeWorld::new(initial);
    let mut host = ScriptHost::for_world(code, &world).unwrap();
    let capture = Capture::start_with_heap_mode(mode).unwrap();
    incant_profile::frame(|| host.tick(&mut world, 1. / 60.)).unwrap();
    let successful = capture.report();
    let tick = &successful.frame_wall.phases[incant_profile::Phase::ScriptTick as usize];
    assert_eq!(tick.samples, 1);
    assert_eq!(tick.missing_frames, 0);
    assert_eq!(tick.invalid_frames, 0);
    assert!(tick.p95_upper_ns.is_some());
    assert_eq!(
        successful
            .scripts
            .iter()
            .map(|s| s.totals.calls)
            .sum::<u64>(),
        2
    );
    let before = world.inspect(id(1)).unwrap().transform;
    assert!(incant_profile::frame(|| host.tick(&mut world, 1. / 60.)).is_err());
    assert_eq!(world.inspect(id(1)).unwrap().transform, before);
    let report = capture.report();
    assert_eq!(report.frame_wall.failed_frames, 1);
    assert!(
        report
            .frame_wall
            .phases
            .iter()
            .all(|row| row.p95_upper_ns.is_none())
    );
    assert_eq!(report.scripts.len(), 2);
    assert_eq!(report.scripts[0].source, Some(id(10).0));
    assert_eq!(report.scripts[1].source, Some(id(20).0));
    assert!(report.scripts.iter().all(|s| s.totals.calls == 2));
    assert_eq!(report.quickjs_heap.samples, 2);
    assert_eq!(report.quickjs_heap.invalid_samples, 0);
    assert_eq!(report.quickjs_heap.mode, mode);
    let latest = report.quickjs_heap.latest.unwrap();
    assert!(latest.allocated_bytes > 0 && latest.allocated_bytes < 32 * 1024 * 1024);
    assert_eq!(latest.limit_bytes, 32 * 1024 * 1024);
    assert_eq!(
        latest.estimated_used_bytes.is_some(),
        mode == HeapMode::Detailed
    );
    if let Some(used) = latest.estimated_used_bytes {
        assert!(used > 0);
    }
    let serialized = serde_json::to_string(&report).unwrap();
    assert!(!serialized.contains("private gameplay"));
    assert!(!serialized.contains("private-user-filename"));
}
