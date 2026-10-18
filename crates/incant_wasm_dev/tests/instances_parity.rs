#![cfg(feature = "persistent-conformance")]
use incant_runtime::{CookedEntity, CookedScene, NativeWorld, StableId};
use incant_script::{
    CompiledScript, NativeSession, ScriptBindings, ScriptInstance, ScriptValue, SessionProgram,
    WasmInstanceCatalog,
};
use incant_wasm_dev::DevModuleHost;
use incant_wasm_runtime::{StaticModule, embedded};
use std::{collections::BTreeMap, sync::Arc};

fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
fn scene() -> CookedScene {
    CookedScene::new(
        id(9),
        60,
        (1..=3)
            .map(|n| CookedEntity {
                id: id(n),
                parent: None,
                transform: Default::default(),
                velocity: None,
            })
            .collect(),
    )
    .unwrap()
}
fn props(speed: f64) -> BTreeMap<String, ScriptValue> {
    BTreeMap::from([("speed".into(), ScriptValue::Number(speed))])
}
fn catalog(speed: f64) -> WasmInstanceCatalog {
    WasmInstanceCatalog::new(
        NativeWorld::new(scene()).source_scene_fingerprint(),
        BTreeMap::from([(id(1), props(speed)), (id(2), props(20.))]),
    )
    .unwrap()
}
fn program(dev: bool, name: &str, speed: f64) -> SessionProgram {
    let program = if dev {
        SessionProgram::DevelopmentWasm(
            std::fs::read(
                std::path::Path::new(env!("INCANT_RUST_SDK_GUESTS")).join(format!("{name}.wasm")),
            )
            .unwrap(),
        )
    } else {
        embedded::linked(name).unwrap().into()
    };
    program.with_instances(catalog(speed)).unwrap()
}
fn typed(retire: bool) -> SessionProgram {
    let scene = scene();
    let origin = NativeWorld::new(scene).source_scene_fingerprint();
    let bindings = ScriptBindings::new(
        origin,
        vec![id(4)],
        vec![
            ScriptInstance {
                entity: id(1),
                source: id(4),
                props: props(10.),
            },
            ScriptInstance {
                entity: id(2),
                source: id(4),
                props: props(20.),
            },
        ],
    )
    .unwrap();
    CompiledScript::compile_project("instances-parity",&format!(r#"
    exports.default={{sources:[{{id:'00000000000000000000000004',behavior:{{persistence:{{version:1}},update({{instances,chunks,clock,commands}}){{
        for(const c of chunks)for(let row=0;row<c.ids.length;row++){{const i=instances.find(i=>i.entity===c.ids[row]);if(i)c.transforms[row*10]+=i.props.speed;}}
        if({retire}){{if(clock.tick===1)commands.despawn('00000000000000000000000001');if(clock.tick===2)commands.spawn('00000000000000000000000001',[0,0,0,0,0,0,1,1,1,1],null,null);}}
    }}}}}}]}};
    "#),bindings).unwrap().into()
}

#[test]
fn actual_rust_guests_match_typed_authored_owners_and_do_not_reattach_reused_ids() {
    for retire in [false, true] {
        let name = if retire {
            "rust-instances-retire"
        } else {
            "rust-instances"
        };
        let mut ts = NativeSession::new(scene(), typed(retire), Default::default()).unwrap();
        let mut native =
            NativeSession::new(scene(), program(false, name, 10.), Default::default()).unwrap();
        let mut dev =
            NativeSession::new(scene(), program(true, name, 10.), Default::default()).unwrap();
        for _ in 0..4 {
            for session in [&mut ts, &mut native, &mut dev] {
                session.tick(&[]).unwrap();
            }
            let expected = ts.world().unwrap().snapshot();
            assert_eq!(native.world().unwrap().snapshot(), expected);
            assert_eq!(dev.world().unwrap().snapshot(), expected);
        }
        let rows = ts.world().unwrap().snapshot();
        assert_eq!(
            rows.iter()
                .find(|r| r.id == id(1))
                .unwrap()
                .transform
                .translation[0],
            if retire { 0. } else { 40. }
        );
        assert_eq!(
            rows.iter()
                .find(|r| r.id == id(2))
                .unwrap()
                .transform
                .translation[0],
            80.
        );
        assert_eq!(
            rows.iter()
                .find(|r| r.id == id(3))
                .unwrap()
                .transform
                .translation[0],
            0.
        );
    }
}
#[test]
fn actual_both_hosts_preserve_membership_on_restore_and_refuse_changed_catalog() {
    for dev in [false, true] {
        let code = program(dev, "rust-instances-retire", 10.);
        let initial = scene();
        let mut live =
            NativeSession::new(initial.clone(), code.clone(), Default::default()).unwrap();
        live.tick(&[]).unwrap();
        live.tick(&[]).unwrap();
        let saved = live.checkpoint_bytes().unwrap();
        let mut restored =
            NativeSession::from_checkpoint_bytes(&saved, &initial, code, Default::default())
                .unwrap();
        for _ in 0..3 {
            live.tick(&[]).unwrap();
            restored.tick(&[]).unwrap();
            assert_eq!(
                live.world().unwrap().snapshot(),
                restored.world().unwrap().snapshot()
            );
            assert_eq!(
                live.checkpoint_bytes().unwrap(),
                restored.checkpoint_bytes().unwrap()
            );
        }
        assert!(
            NativeSession::from_checkpoint_bytes(
                &saved,
                &initial,
                program(dev, "rust-instances-retire", 11.),
                Default::default()
            )
            .is_err()
        );
    }
}
#[test]
fn actual_both_hosts_refuse_ignored_bounds_stale_tokens_and_callback_failure() {
    for dev in [false, true] {
        for suffix in ["scratch", "callback", "trap", "stale"] {
            let mut host = NativeSession::new(
                scene(),
                program(dev, &format!("rust-instances-{suffix}"), 10.),
                Default::default(),
            )
            .unwrap();
            if suffix == "stale" {
                host.tick(&[]).unwrap();
            }
            assert!(host.tick(&[]).is_err(), "{dev} {suffix}");
            assert!(host.world().is_err());
            assert!(host.checkpoint_bytes().is_err());
        }
    }
}

#[test]
fn actual_both_hosts_reload_retains_removal_before_same_id_respawn() {
    let name = "rust-instances-retire";
    let code = embedded::linked(name).unwrap();
    let bytes = std::fs::read(
        std::path::Path::new(env!("INCANT_RUST_SDK_GUESTS")).join(format!("{name}.wasm")),
    )
    .unwrap();
    let mut native = StaticModule::new(code).unwrap();
    let mut dev = DevModuleHost::new(&bytes).unwrap();
    let mut a = NativeWorld::new(scene());
    let mut b = NativeWorld::new(scene());
    let params = Arc::new(catalog(10.));
    native.configure_instances(params.clone(), &a).unwrap();
    dev.configure_instances(params, &b).unwrap();
    a.step().unwrap();
    b.step().unwrap();
    native.tick(&mut a, 1. / 60.).unwrap();
    dev.tick(&mut b, 1. / 60.).unwrap();
    assert!(a.inspect(id(1)).is_none());
    assert!(b.inspect(id(1)).is_none());
    let removed = native.persistent_checkpoint(&a).unwrap();
    assert_eq!(dev.persistent_checkpoint(&b).unwrap(), removed);
    native.reload(code).unwrap();
    dev.reload(&bytes).unwrap();
    assert_eq!(native.persistent_checkpoint(&a).unwrap(), removed);
    assert_eq!(dev.persistent_checkpoint(&b).unwrap(), removed);
    // Tick two respawns the same stable ID; tick three must not retune it.
    for _ in 0..2 {
        a.step().unwrap();
        b.step().unwrap();
        native.tick(&mut a, 1. / 60.).unwrap();
        dev.tick(&mut b, 1. / 60.).unwrap();
        assert_eq!(a.snapshot(), b.snapshot());
    }
    assert_eq!(a.inspect(id(1)).unwrap().transform.translation[0], 0.);
    assert_eq!(a.inspect(id(2)).unwrap().transform.translation[0], 60.);
    assert_eq!(a.inspect(id(3)).unwrap().transform.translation[0], 0.);
    assert_eq!(
        native.persistent_checkpoint(&a).unwrap(),
        dev.persistent_checkpoint(&b).unwrap()
    );
}
