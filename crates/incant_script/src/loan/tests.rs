use super::*;
use crate::{CompiledScript, NativeSession, ScriptHost, program::SessionVm, tests::*};
use incant_runtime::{CookedEntity, CookedScene, FrameCommands, StructuralCommand};
use incant_types::{Transform, Velocity};
use std::panic::{AssertUnwindSafe, catch_unwind};

fn script(body: &str) -> CompiledScript {
    CompiledScript::compile(&format!(
        "exports.default = defineBehavior({{ update({{chunks, commands}}) {{ {body} }} }});"
    ))
    .unwrap()
}

fn session_cache(session: &NativeSession) -> &Scratch {
    match &session.host {
        SessionVm::Bytecode(host) => &host.loan_scratch,
        #[cfg(feature = "static-module")]
        SessionVm::Static(_) => panic!("bytecode host"),
        #[cfg(feature = "wasmtime-host")]
        SessionVm::DevelopmentWasm(_) => panic!("bytecode host"),
    }
}

#[test]
fn reused_copies_capture_current_values_and_restore_every_late_failure() {
    for refusal in [
        "throw Error('late');",
        "chunks.at(-1).transforms[3] = NaN;",
        "commands.despawn('00000000000000000000000999');",
    ] {
        let mut host = ScriptHost::new(script(&format!(
            r#"
            globalThis.turn = (globalThis.turn || 0) + 1;
            if (!globalThis.views) globalThis.views = [];
            for (const old of views) if (old.buffer.byteLength !== 0) throw Error('stale view');
            for (const c of chunks) {{
                views.push(c.transforms, c.transforms.subarray(1), new DataView(c.transforms.buffer));
                c.transforms[0] += 7;
                if (c.velocities) c.velocities[0] += 1;
            }}
            if (turn === 3) {{ {refusal} }}
            "#
        )))
        .unwrap();
        let mut world = world();
        host.tick(&mut world, 0.1).unwrap();
        let allocations = host.loan_scratch.allocations();
        host.tick(&mut world, 0.1).unwrap();
        assert_eq!(host.loan_scratch.allocations(), allocations);
        assert_eq!(world.inspect(id(1)).unwrap().transform.translation[0], 14.);
        assert_eq!(
            world.inspect(id(2)).unwrap().velocity.unwrap().linear[0],
            3.
        );
        let before = world.snapshot();
        assert!(host.tick(&mut world, 0.1).is_err(), "{refusal}");
        assert_eq!(world.snapshot(), before, "must restore the latest capture");
        assert_eq!(host.loan_scratch.allocations(), allocations);
        check(
            &host,
            "for (const v of views) if (v.buffer.byteLength !== 0) throw Error('attached after refusal');",
        );
        assert!(matches!(
            host.tick(&mut world, 0.1),
            Err(ScriptError::FailedSession)
        ));
    }
}

#[test]
fn same_sized_replacement_rows_capture_new_identities_and_values() {
    let mut host = ScriptHost::new(script(
        "globalThis.turn=(globalThis.turn||0)+1; globalThis.labels=chunks.flatMap(c=>c.ids); globalThis.views=chunks.map(c=>c.transforms); for(const c of chunks)c.transforms[0]+=99; if(turn===2)throw Error('late replacement');",
    )).unwrap();
    let mut first = world();
    host.tick(&mut first, 0.1).unwrap();
    let allocations = host.loan_scratch.allocations();
    let mut replacement = NativeWorld::new(
        CookedScene::new(
            id(999),
            60,
            vec![
                CookedEntity {
                    id: id(15),
                    parent: None,
                    transform: Transform {
                        translation: [41., 0., 0.],
                        ..Default::default()
                    },
                    velocity: None,
                },
                CookedEntity {
                    id: id(16),
                    parent: None,
                    transform: Transform {
                        translation: [72., 0., 0.],
                        ..Default::default()
                    },
                    velocity: Some(Velocity {
                        linear: [9., 0., 0.],
                    }),
                },
            ],
        )
        .unwrap(),
    );
    let before = replacement.snapshot();
    assert!(matches!(
        host.tick(&mut replacement, 0.1),
        Err(ScriptError::Javascript { .. })
    ));
    assert_eq!(replacement.snapshot(), before);
    assert_eq!(host.loan_scratch.allocations(), allocations);
    let mut saved: Vec<_> = host
        .loan_scratch
        .0
        .iter()
        .flat_map(|c| c.ids.iter().copied())
        .collect();
    saved.sort_unstable();
    assert_eq!(saved, vec![id(15), id(16)]);
    check(
        &host,
        "if(!labels.includes('0000000000000000000000000F')||!labels.includes('0000000000000000000000000G'))throw Error('stale IDs'); for(const v of views)if(v.byteLength!==0)throw Error('attached');",
    );
}

#[test]
fn topology_growth_shrink_and_empty_world_retire_cache_tails_and_views() {
    let mut host = ScriptHost::new(script(
        r#"
        if (globalThis.old) for (const v of old) if(v.byteLength !== 0) throw Error('old loan');
        globalThis.old = chunks.flatMap(c => [c.transforms, ...(c.velocities ? [c.velocities] : [])]);
        for(const c of chunks) for(let row=0; row<c.ids.length; row++) c.transforms[row*10] += 1;
        "#,
    ))
    .unwrap();
    let mut world = world();
    host.tick(&mut world, 0.1).unwrap();
    let mut commands = FrameCommands::default();
    for n in 3..35 {
        commands
            .push(StructuralCommand::Spawn(CookedEntity {
                id: id(n),
                parent: None,
                transform: Transform::default(),
                velocity: (n % 2 == 0).then_some(Velocity { linear: [0.; 3] }),
            }))
            .unwrap();
    }
    world.apply(&mut commands).unwrap();
    host.tick(&mut world, 0.1).unwrap();
    world.with_numeric_columns(|columns| {
        let mut saved = host.loan_scratch.0.iter();
        columns.for_each_chunk(|chunk| {
            let copy = saved.next().unwrap();
            assert_eq!(&*copy.ids, chunk.ids);
            assert_eq!(copy.transforms.len(), chunk.transforms.len());
            assert_eq!(
                copy.velocities.as_deref().map(<[f64]>::len),
                chunk.velocities.map(|v| v.len())
            );
        });
        assert!(saved.next().is_none());
    });
    for n in (1..35).rev() {
        commands.push(StructuralCommand::Despawn(id(n))).unwrap();
    }
    world.apply(&mut commands).unwrap();
    host.tick(&mut world, 0.1).unwrap();
    assert!(host.loan_scratch.allocations().is_empty());
    assert_eq!(host.loan_scratch.0.capacity(), 0);
    check(&host, "if(old.length !== 0) throw Error('nonempty world');");
    commands
        .push(StructuralCommand::Spawn(CookedEntity {
            id: id(1),
            parent: None,
            transform: Transform::default(),
            velocity: None,
        }))
        .unwrap();
    world.apply(&mut commands).unwrap();
    host.tick(&mut world, 0.1).unwrap();
    assert_eq!(world.inspect(id(1)).unwrap().transform.translation[0], 1.);
    assert_eq!(host.loan_scratch.0.len(), 1);
}

#[test]
fn constructor_refusal_returns_only_complete_copies_without_rolling_back_a_prefix() {
    let mut world = world();
    let mut scratch = Scratch::default();
    world
        .with_numeric_columns(|columns| Loan::new(columns, &mut scratch).unwrap().finish())
        .apply(&mut world, &mut FrameCommands::default())
        .unwrap();
    let before = world.snapshot();
    world.with_numeric_columns(|columns| {
        // Either table fits on its own, but the second exceeds this reduced
        // test budget. The production entry point always uses 32 MiB.
        assert!(matches!(
            Loan::capture(columns, &mut scratch, 104),
            Err(ScriptError::InputLimit)
        ));
    });
    assert_eq!(world.snapshot(), before);
    assert_eq!(scratch.0.len(), 1, "uncaptured cached tail must be evicted");
    world
        .with_numeric_columns(|columns| Loan::new(columns, &mut scratch).unwrap().finish())
        .apply(&mut world, &mut FrameCommands::default())
        .unwrap();
    assert_eq!(world.snapshot(), before);
    assert_eq!(scratch.0.len(), 2);
}

#[test]
fn rejected_apply_validates_the_last_identity_before_restoring_any_prefix() {
    let mut world = world();
    let mut scratch = Scratch::default();
    let mut undo = world.with_numeric_columns(|columns| {
        let loan = Loan::new(columns, &mut scratch).unwrap();
        // Trusted native fault injection, before any JavaScript exposure.
        loan._columns
            .for_each_chunk(|chunk| chunk.transforms[0] = 88.);
        loan.finish()
    });
    let before = world.snapshot();
    undo.0.chunks.last_mut().unwrap().ids[0] = id(999);
    let mut commands = FrameCommands::default();
    commands.push(StructuralCommand::Despawn(id(999))).unwrap();
    let result = catch_unwind(AssertUnwindSafe(|| undo.apply(&mut world, &mut commands)));
    assert!(
        result.is_err(),
        "corrupt final layout must refuse restoration"
    );
    assert_eq!(world.snapshot(), before, "no restored component prefix");
    // The cache holds only plain copies; a later capture replaces every ID.
    world.with_numeric_columns(|columns| drop(Loan::new(columns, &mut scratch).unwrap()));
    assert_eq!(world.snapshot(), before);
}

#[test]
fn unwind_detaches_views_restores_current_values_then_returns_reusable_storage() {
    let mut host = ScriptHost::new(script("for(const c of chunks)c.transforms[0]+=4;")).unwrap();
    let mut world = world();
    host.tick(&mut world, 0.1).unwrap();
    let allocations = host.loan_scratch.allocations();
    let before = world.snapshot();
    let result = catch_unwind(AssertUnwindSafe(|| {
        host.context.with(|ctx| {
            let freeze = host.freeze.clone().restore(&ctx).unwrap();
            world.with_numeric_columns(|columns| {
                let mut loan = Loan::new(columns, &mut host.loan_scratch).unwrap();
                let chunks = loan.expose(&ctx, &freeze, &mut host.chunk_ids).unwrap();
                ctx.globals().set("loan", chunks).unwrap();
                ctx.eval::<(), _>("globalThis.saved = loan.flatMap(c=>[c.transforms,c.transforms.subarray(1),new DataView(c.transforms.buffer)]); for(const c of loan)c.transforms[0]=99;").unwrap();
                panic!("native unwind after exposure");
            });
        });
    }));
    assert!(result.is_err());
    assert_eq!(world.snapshot(), before);
    assert_eq!(host.loan_scratch.allocations(), allocations);
    check(
        &host,
        "for(const v of saved)if(v.buffer.byteLength!==0)throw Error('unwind retained view');",
    );
    host.tick(&mut world, 0.1).unwrap();
    assert_eq!(world.inspect(id(1)).unwrap().transform.translation[0], 8.);
    assert_eq!(host.loan_scratch.allocations(), allocations);
}

#[test]
fn checkpoint_replacement_and_host_destruction_do_not_transfer_cached_copies() {
    let scene = CookedScene::new(
        id(999),
        60,
        vec![CookedEntity {
            id: id(1),
            parent: None,
            transform: Transform::default(),
            velocity: None,
        }],
    )
    .unwrap();
    let code = CompiledScript::compile(
        "exports.default={persistence:{version:1},update(t){t.chunks[0].transforms[0]+=1;}};",
    )
    .unwrap();
    let mut live = NativeSession::new(scene.clone(), code.clone(), Default::default()).unwrap();
    live.tick(&[]).unwrap();
    assert!(!session_cache(&live).allocations().is_empty());
    let bytes = live.checkpoint_bytes().unwrap();
    live.restore_checkpoint_bytes(&bytes).unwrap();
    assert!(session_cache(&live).allocations().is_empty());
    live.tick(&[]).unwrap();
    assert_eq!(
        live.world()
            .unwrap()
            .inspect(id(1))
            .unwrap()
            .transform
            .translation[0],
        2.
    );
    // Dropping the old world/VM after a populated cache must not dereference its
    // retired table storage; a new host owns an empty cache and fresh views.
    drop(live);
    let mut fresh = NativeSession::new(scene, code, Default::default()).unwrap();
    assert!(session_cache(&fresh).allocations().is_empty());
    fresh.tick(&[]).unwrap();
    assert_eq!(
        fresh
            .world()
            .unwrap()
            .inspect(id(1))
            .unwrap()
            .transform
            .translation[0],
        1.
    );
}
