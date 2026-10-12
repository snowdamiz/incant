use crate::*;
use incant_runtime::{CookedEntity, CookedScene, NativeWorld, StableId};
fn world() -> NativeWorld {
    NativeWorld::new(
        CookedScene::new(
            StableId([1; 16]),
            60,
            vec![CookedEntity {
                id: StableId([2; 16]),
                parent: None,
                transform: Default::default(),
                velocity: None,
            }],
        )
        .unwrap(),
    )
}
#[test]
fn typed_store_commits_with_world_and_reads_owned_frozen_data() {
    let compiled = CompiledScript::compile(
        r#"exports.default = { persistence: {version:1}, update(t) {
      const n = t.state.get();
      if (n !== null && !Object.isFrozen(n)) throw Error('mutable persistent observation');
      const count = n === null ? 1 : n.count + 1;
      t.state.set({count, rows: [null, true, '😀', -0]});
      if (t.state.get().count !== count) throw Error('overlay not visible');
      t.chunks[0].transforms[0] = count;
    }};"#,
    )
    .unwrap();
    let mut host = ScriptHost::new(compiled).unwrap();
    let mut world = world();
    for _ in 0..3 {
        host.tick(&mut world, 1. / 60.).unwrap();
    }
    let store = host.persistent.borrow();
    let entry = store
        .entries
        .get(&super::Key {
            scope: crate::schedule::Scope::Global,
            entity: None,
        })
        .unwrap();
    assert!(
        matches!(&entry.value,ScriptValue::Object(o) if o.get("count")==Some(&ScriptValue::Number(3.)))
    );
}
#[test]
fn invalid_caught_data_refuses_numeric_and_persistent_writes() {
    for invalid in [
        "NaN",
        "new Date()",
        "new Proxy({a:1}, {getOwnPropertyDescriptor(){throw Error('trap')}})",
        "{get secret(){return 1}}",
        "[,,,]",
        "{[Symbol()]:1}",
        "(()=>{const a={};a.a=a;return a})()",
    ] {
        let compiled = CompiledScript::compile(&format!(
            r#"let first=true; exports.default={{persistence:{{version:1}},update(t) {{
          if(first){{first=false;t.state.set(7);return;}}
          globalThis.kept=t.state; t.state.set(8);t.chunks[0].transforms[0]=99;
          try{{t.state.set({invalid})}}catch(_){{}}
        }}}};"#
        ))
        .unwrap();
        let mut host = ScriptHost::new(compiled).unwrap();
        let mut world = world();
        host.tick(&mut world, 1. / 60.).unwrap();
        let before = world.save_bytes().unwrap();
        assert!(host.tick(&mut world, 1. / 60.).is_err(), "{invalid}");
        assert_eq!(before, world.save_bytes().unwrap());
        let store = host.persistent.borrow();
        assert_eq!(
            store.entries.values().next().unwrap().value,
            ScriptValue::Number(7.)
        );
    }
}
#[test]
fn behavior_without_opt_in_cannot_use_store() {
    let mut host = ScriptHost::new(
        CompiledScript::compile("exports.default={update(t){try{t.state.set(1)}catch(_){}}};")
            .unwrap(),
    )
    .unwrap();
    assert!(host.tick(&mut world(), 1. / 60.).is_err());
}
#[test]
fn stale_functions_and_proxy_reentrancy_never_keep_a_rust_borrow() {
    let compiled=CompiledScript::compile(r#"
      let old;
      exports.default={persistence:{version:1},update(t){
        if(old){try{old.set(100);throw Error('stale accepted')}catch(e){if(e.message==='stale accepted')throw e}}
        const object=new Proxy({value:1},{ownKeys(o){t.state.set(5);return Reflect.ownKeys(o)}});
        t.state.set(object);
        if(t.state.get().value!==1)throw Error('reentrant overlay lost');
        old=t.state;
      }};
    "#).unwrap();
    let mut host = ScriptHost::new(compiled).unwrap();
    let mut world = world();
    for _ in 0..3 {
        host.tick(&mut world, 1. / 60.).unwrap();
    }
}
#[test]
fn limits_are_aggregate_and_caught_errors_roll_back() {
    for body in [
        "for(let i=0;i<257;i++){try{t.state.get()}catch(_){}}",
        "try{t.state.set('x'.repeat(1048576))}catch(_){}",
        "try{t.state.set(Array(65536).fill(null))}catch(_){}",
        "try{t.state.set('\\ud800')}catch(_){}",
    ] {
        let compiled=CompiledScript::compile(&format!("exports.default={{persistence:{{version:1}},update(t){{t.chunks[0].transforms[0]=42;{body}}}}};")).unwrap();
        let mut host = ScriptHost::new(compiled).unwrap();
        let mut world = world();
        let before = world.save_bytes().unwrap();
        assert!(host.tick(&mut world, 1. / 60.).is_err());
        assert_eq!(before, world.save_bytes().unwrap());
        assert!(host.persistent.borrow().entries.is_empty());
    }
}
