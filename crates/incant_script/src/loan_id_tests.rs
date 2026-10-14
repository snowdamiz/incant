use crate::{
    CompiledScript, ScriptHost,
    tests::{check, id, world},
};
use incant_runtime::{CookedEntity, FrameCommands, StructuralCommand};
use incant_types::{Transform, Velocity};

#[test]
fn immutable_row_labels_reuse_metadata_without_reusing_live_views() {
    let code = CompiledScript::compile(
        r#"
        exports.default = defineBehavior({ update({chunks}) {
            if (globalThis.previous) {
                for (let i=0;i<chunks.length;i++) {
                    if (previous[i].ids !== chunks[i].ids) throw Error('rebuilt unchanged labels');
                    if (previous[i].view.buffer.byteLength !== 0) throw Error('retained live loan');
                    if (previous[i].view === chunks[i].transforms) throw Error('reused view');
                }
            }
            for (const c of chunks) {
                if (!Object.isFrozen(c.ids)) throw Error('mutable labels');
                if (Reflect.set(c.ids, '0', 'forged')) throw Error('row replacement');
                if (Reflect.set(c.ids, 'length', 0)) throw Error('row deletion');
                c.transforms[0] += 1;
            }
            globalThis.previous=chunks.map(c=>({ids:c.ids,view:c.transforms}));
        }});
    "#,
    )
    .unwrap();
    let mut host = ScriptHost::new(code).unwrap();
    let mut world = world();
    for _ in 0..3 {
        host.tick(&mut world, 1. / 60.).unwrap();
        host.context.with(|ctx| ctx.run_gc());
    }
    assert_eq!(world.inspect(id(1)).unwrap().transform.translation[0], 3.);
    check(
        &host,
        "for(const c of previous) if(c.view.buffer.byteLength!==0) throw Error('live after tick');",
    );
}

#[test]
fn structural_row_changes_preserve_old_labels_and_current_write_mapping() {
    let code = CompiledScript::compile(r#"
        exports.default = defineBehavior({ update({chunks}) {
            for (const old of globalThis.oldLabels || []) {
                if (old.ids.join('/') !== old.text) throw Error('old snapshot changed');
            }
            globalThis.oldLabels=(globalThis.oldLabels || []).concat(chunks.map(c=>({ids:c.ids,text:c.ids.join('/')})));
            for (const c of chunks) for(let row=0;row<c.ids.length;row++) {
                // Fixture IDs 1..3 have these exact final ULID digits.
                c.transforms[row*c.transformStride]=Number(c.ids[row].slice(-1))*10;
            }
        }});
    "#).unwrap();
    let mut host = ScriptHost::new(code).unwrap();
    let mut world = world();
    let entity = |n, velocity| CookedEntity {
        id: id(n),
        parent: None,
        transform: Transform::default(),
        velocity,
    };
    let mut changes = FrameCommands::default();
    changes
        .push(StructuralCommand::Spawn(entity(3, None)))
        .unwrap();
    world.apply(&mut changes).unwrap();
    host.tick(&mut world, 1. / 60.).unwrap();
    changes.push(StructuralCommand::Despawn(id(2))).unwrap();
    world.apply(&mut changes).unwrap();
    // Removing the last row in one table shrinks the metadata cache.
    host.tick(&mut world, 1. / 60.).unwrap();
    changes
        .push(StructuralCommand::SetVelocity {
            entity: id(3),
            velocity: Some(Velocity { linear: [0.; 3] }),
        })
        .unwrap();
    changes
        .push(StructuralCommand::Spawn(entity(
            2,
            Some(Velocity { linear: [0.; 3] }),
        )))
        .unwrap();
    world.apply(&mut changes).unwrap();
    host.tick(&mut world, 1. / 60.).unwrap();
    for n in 1..=3 {
        assert_eq!(
            world.inspect(id(n)).unwrap().transform.translation[0],
            n as f64 * 10.
        );
    }
    host.context.with(|ctx| ctx.run_gc());
    check(
        &host,
        "for(const old of oldLabels) if(old.ids.join('/')!==old.text) throw Error('retained labels changed');",
    );
}
