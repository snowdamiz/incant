use super::*;
use incant_runtime::{CookedEntity, CookedScene, FrameCommands, StableId, StructuralCommand};
use incant_types::{Transform, Velocity};

pub(super) fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
pub(super) fn world() -> NativeWorld {
    NativeWorld::new(
        CookedScene::new(
            id(999),
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
                    parent: Some(id(1)),
                    transform: Transform::default(),
                    velocity: Some(Velocity {
                        linear: [1., 0., 0.],
                    }),
                },
            ],
        )
        .unwrap(),
    )
}
fn script(body: &str) -> CompiledScript {
    CompiledScript::compile(&format!(
        "exports.default = defineBehavior({{ update({{chunks, dt}}) {{ {body} }} }});"
    ))
    .unwrap()
}
pub(super) fn check(host: &ScriptHost, source: &str) {
    host.context.with(|ctx| ctx.eval::<(), _>(source)).unwrap();
}

#[test]
fn bytecode_runs_direct_columns_across_tables_and_survives_source_token_drop() {
    let code = script(
        "for (const c of chunks) { for (let i = 0; i < c.transforms.length; i += c.transformStride) c.transforms[i] += dt; if (c.velocities) c.velocities[1] = 3; }",
    );
    assert!(code.byte_len() > 0);
    let mut host = ScriptHost::new(code.clone()).unwrap();
    drop(code);
    let mut world = world();
    for _ in 0..120 {
        host.tick(&mut world, 1. / 60.).unwrap();
    }
    assert!((world.inspect(id(1)).unwrap().transform.translation[0] - 2.).abs() < 1e-12);
    assert!((world.inspect(id(2)).unwrap().world_transform[3][0] - 4.).abs() < 1e-12);
    assert_eq!(
        world.inspect(id(2)).unwrap().velocity.unwrap().linear,
        [1., 3., 0.]
    );
    assert_eq!(world.tick(), 0, "host owns only the script phase");
}

#[test]
fn retained_views_are_detached_before_later_archetype_growth() {
    let mut host = ScriptHost::new(script("globalThis.retained = chunks.flatMap(c => [c.transforms, c.transforms.subarray(1), new DataView(c.transforms.buffer)]); for (const c of chunks) c.transforms[0] += dt;")).unwrap();
    let mut world = world();
    host.tick(&mut world, 0.25).unwrap();
    check(
        &host,
        "for (const v of retained) { if (v.buffer.byteLength !== 0) throw Error('attached'); if (v instanceof Float64Array && v.length !== 0) throw Error('live typed view'); if (v instanceof DataView) { let rejected=false; try {v.getFloat64(0);} catch (_) {rejected=true;} if(!rejected) throw Error('live DataView'); } }",
    );
    let mut commands = FrameCommands::default();
    for n in 1000..6000 {
        commands
            .push(StructuralCommand::Spawn(CookedEntity {
                id: id(n),
                parent: None,
                transform: Transform::default(),
                velocity: None,
            }))
            .unwrap();
    }
    commands
        .push(StructuralCommand::SetVelocity {
            entity: id(1),
            velocity: Some(Velocity {
                linear: [1., 0., 0.],
            }),
        })
        .unwrap();
    world.apply(&mut commands).unwrap();
    host.context.with(|ctx| ctx.run_gc());
    check(
        &host,
        "for (const v of retained) if (v.buffer.byteLength !== 0) throw Error('reattached');",
    );
    host.tick(&mut world, 0.25).unwrap();
}

#[test]
fn failure_restores_numeric_columns_and_fails_the_host() {
    let mut host = ScriptHost::new(script("globalThis.failed = chunks[0].transforms; for (const c of chunks) { c.transforms[0] = 100; if(c.velocities)c.velocities[0] = 400; } throw Error('deliberate failure');")).unwrap();
    let mut world = world();
    let before = world.snapshot();
    assert!(matches!(
        host.tick(&mut world, 0.1),
        Err(ScriptError::Javascript { .. })
    ));
    assert_eq!(world.snapshot(), before);
    check(
        &host,
        "if (failed.byteLength !== 0) throw Error('still attached');",
    );
    assert!(matches!(
        host.tick(&mut world, 0.1),
        Err(ScriptError::FailedSession)
    ));
}

#[test]
fn allocation_failure_and_cpu_budget_leave_no_component_prefix() {
    for body in [
        "chunks[0].transforms[0] = 77; new ArrayBuffer(64 * 1024 * 1024);",
        "chunks[0].transforms[0] = 88; for (;;) {}",
    ] {
        let mut host = ScriptHost::with_budget(script(body), Duration::from_millis(50)).unwrap();
        let mut world = world();
        let before = world.snapshot();
        assert!(host.tick(&mut world, 0.1).is_err());
        assert_eq!(world.snapshot(), before);
    }
}

#[test]
fn transferring_or_resizing_component_buffers_is_rejected() {
    let mut host = ScriptHost::new(script(r#"
        const buffer=chunks[0].transforms.buffer;
        for (const name of ['transfer', 'transferToFixedLength', 'transferToImmutable']) {
            for (const n of [undefined, buffer.byteLength, 0, buffer.byteLength+8]) {
                let rejected=false; try { ArrayBuffer.prototype[name].call(buffer,n); } catch(_) {rejected=true;}
                if(!rejected) throw Error('transfer allowed');
            }
            const d=Object.getOwnPropertyDescriptor(ArrayBuffer.prototype,name);
            if(d.writable || d.configurable) throw Error('mutable transfer');
        }
        let rejected=false; try {buffer.resize(0);} catch(_) {rejected=true;}
        if(!rejected) throw Error('resize allowed');
        globalThis.copy=buffer.slice(0);
    "#)).unwrap();
    host.tick(&mut world(), 0.1).unwrap();
    check(
        &host,
        "if (copy.byteLength === 0) throw Error('owned copy detached');",
    );
}

#[test]
fn unsupported_behavior_and_async_are_explicit() {
    let code = CompiledScript::compile("exports.default={initialState:{x:0},update(){}};").unwrap();
    assert!(matches!(
        ScriptHost::new(code),
        Err(ScriptError::Javascript { .. })
    ));
    let code = CompiledScript::compile(
        "exports.default={async update({chunks}){chunks[0].transforms[0]=42;}};",
    )
    .unwrap();
    let mut host = ScriptHost::new(code).unwrap();
    let mut world = world();
    let before = world.snapshot();
    assert!(matches!(
        host.tick(&mut world, 0.1),
        Err(ScriptError::UnsupportedAsync)
    ));
    assert_eq!(world.snapshot(), before);
}

#[test]
fn compilation_does_not_execute_and_constructor_has_no_os_capabilities() {
    let code = CompiledScript::compile("throw Error('only during initialization');").unwrap();
    assert!(matches!(
        ScriptHost::new(code),
        Err(ScriptError::Javascript { .. })
    ));
    let mut host = ScriptHost::new(script("for(const name of ['require','process','fetch','Worker','ShadowRealm','structuredClone']) if(typeof globalThis[name] !== 'undefined') throw Error('unexpected capability');")).unwrap();
    host.tick(&mut world(), 0.1).unwrap();
}

#[test]
fn partial_exposure_revokes_views_before_hostile_exception_getters() {
    let code = CompiledScript::compile(
        r#"
        Object.defineProperty(Object.prototype, 'transforms', {set(view) {
            globalThis.exposed = view;
            view[0] = 91;
            const error = new Error();
            Object.defineProperty(error, 'message', {get() {
                globalThis.detachedDuringDiagnostic = exposed.buffer.byteLength === 0;
                return 'metadata setter';
            }});
            throw error;
        }});
        exports.default = {update() {throw Error('must not execute');}};
    "#,
    )
    .unwrap();
    let mut host = ScriptHost::new(code).unwrap();
    let mut world = world();
    let before = world.snapshot();
    assert!(matches!(
        host.tick(&mut world, 0.1),
        Err(ScriptError::Javascript {
            phase: "component views",
            ..
        })
    ));
    assert_eq!(world.snapshot(), before);
    check(
        &host,
        "if (!detachedDuringDiagnostic) throw Error('diagnostic observed live loan');",
    );
}

#[test]
fn ignored_deferred_work_is_rejected_during_initialization_and_ticks() {
    for deferred in [
        "void new Promise(() => {});",
        "void (async () => {})();",
        "Promise.resolve().then(() => { throw Error('never drain'); });",
        "class Deferred extends Promise {}; void new Deferred(() => {});",
        "void new (async function(){}).constructor('return 1')();",
        "void import('unavailable-module');",
    ] {
        let code =
            CompiledScript::compile(&format!("{deferred} exports.default = {{update() {{}}}};"))
                .unwrap();
        assert!(
            matches!(ScriptHost::new(code), Err(ScriptError::UnsupportedAsync)),
            "init: {deferred}"
        );

        let mut host = ScriptHost::new(script(&format!(
            "globalThis.deferredView = chunks[0].transforms; deferredView[0] = 53; {deferred}"
        )))
        .unwrap();
        let mut world = world();
        let before = world.snapshot();
        assert!(
            matches!(
                host.tick(&mut world, 0.1),
                Err(ScriptError::UnsupportedAsync)
            ),
            "tick: {deferred}"
        );
        assert_eq!(world.snapshot(), before);
        check(
            &host,
            "if (deferredView.byteLength !== 0) throw Error('live deferred view');",
        );
        assert!(matches!(
            host.tick(&mut world, 0.1),
            Err(ScriptError::FailedSession)
        ));
    }
}

#[test]
fn deferred_work_during_metadata_exposure_also_rolls_back() {
    let code = CompiledScript::compile(
        r#"
        Object.defineProperty(Object.prototype, 'transforms', {set(view) {
            globalThis.metadataView = view;
            view[0] = 39;
            void new Promise(() => {});
        }});
        exports.default = {update() {}};
    "#,
    )
    .unwrap();
    let mut host = ScriptHost::new(code).unwrap();
    let mut world = world();
    let before = world.snapshot();
    assert!(matches!(
        host.tick(&mut world, 0.1),
        Err(ScriptError::UnsupportedAsync)
    ));
    assert_eq!(world.snapshot(), before);
    check(
        &host,
        "if (metadataView.byteLength !== 0) throw Error('live metadata view');",
    );
}
