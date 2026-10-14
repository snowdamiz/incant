use super::*;
use crate::{CompiledScript, ScriptHost};
use incant_runtime::{CookedEntity, CookedScene, NativeWorld};

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
fn poisoned_guest_intrinsics_and_index_setters_cannot_change_private_rows() {
    let compiled = CompiledScript::compile(
        r#"
        exports.default = {persistence:{version:1},update(t) {
          const define=Object.defineProperty, proto=Object.getPrototypeOf;
          const frozen=Object.isFrozen, same=Object.is, object=Object, array=Array;
          const saved=[
            [object,'setPrototypeOf',object.setPrototypeOf],
            [object,'defineProperty',object.defineProperty],
            [object,'getOwnPropertyDescriptor',object.getOwnPropertyDescriptor],
            [object,'getPrototypeOf',object.getPrototypeOf],
            [object,'hasOwn',object.hasOwn], [object,'freeze',object.freeze],
            [Reflect,'ownKeys',Reflect.ownKeys], [array,'isArray',array.isArray],
            [globalThis,'String',String], [globalThis,'TypeError',TypeError]
          ];
          const deny=()=>{throw Error('poisoned intrinsic invoked')};
          let setterCalls=0;
          for(let i=0;i<2;i++) define(array.prototype,''+i,{
            __proto__:null,set(){setterCalls++;throw Error('inherited indexed setter')},configurable:true
          });
          // Property-descriptor objects must not consult inherited get/set.
          define(object.prototype,'get',{__proto__:null,value:deny,configurable:true});
          define(object.prototype,'set',{__proto__:null,value:deny,configurable:true});
          try {
            for(let i=0;i<saved.length;i++) saved[i][0][saved[i][1]]=deny;
            t.state.set({rows:[-0,true,null,'😀'],nested:{empty:[]}});
            const value=t.state.get();
            if(setterCalls!==0 || proto(value)!==null || proto(value.nested)!==null ||
               proto(value.rows)!==array.prototype || !same(value.rows[0],-0) ||
               value.rows[1]!==true || value.rows[2]!==null || value.rows[3]!=='😀' ||
               value.nested.empty.length!==0 || !frozen(value) || !frozen(value.rows) ||
               !frozen(value.nested) || !frozen(value.nested.empty))
              throw Error('public observation changed');
          } finally {
            delete object.prototype.get; delete object.prototype.set;
            delete array.prototype[0]; delete array.prototype[1];
            for(let i=0;i<saved.length;i++) saved[i][0][saved[i][1]]=saved[i][2];
          }
        }};
        "#,
    )
    .unwrap();
    let mut host = ScriptHost::new(compiled).unwrap();
    let mut world = world();
    for _ in 0..3 {
        host.tick(&mut world, 1. / 60.).unwrap();
    }
    let store = host.persistent.borrow();
    let entry = store.entries.values().next().unwrap();
    assert_eq!(entry.cost.bytes, 101);
    assert_eq!(entry.cost.nodes, 8);
    assert_eq!(store.cost.bytes, entry.cost.bytes);
    assert_eq!(store.cost.nodes, entry.cost.nodes);
    let ScriptValue::Object(value) = &entry.value else {
        panic!("object expected");
    };
    let ScriptValue::Array(rows) = &value["rows"] else {
        panic!("array expected");
    };
    let ScriptValue::Number(zero) = rows[0] else {
        panic!("number expected");
    };
    assert_eq!(zero.to_bits(), (-0.0_f64).to_bits());
}

#[test]
fn proxy_reentrancy_uses_captured_inspector_functions() {
    let compiled = CompiledScript::compile(
        r#"
        exports.default={persistence:{version:1},update(t){
          const keys=Reflect.ownKeys, setProto=Object.setPrototypeOf;
          const define=Object.defineProperty;
          const deny=()=>{throw Error('replaced intrinsic invoked')};
          const object=new Proxy({rows:[-0,{value:3}]},{ownKeys(o){
            t.state.set({inner:5});
            Reflect.ownKeys=deny; Object.setPrototypeOf=deny; Object.defineProperty=deny;
            return keys(o);
          }});
          try {t.state.set(object)} finally {
            Reflect.ownKeys=keys; Object.setPrototypeOf=setProto; Object.defineProperty=define;
          }
          const value=t.state.get();
          if(!Object.is(value.rows[0],-0)||value.rows[1].value!==3||value.inner!==undefined)
            throw Error('reentrant outer write lost');
        }};
        "#,
    )
    .unwrap();
    let mut host = ScriptHost::new(compiled).unwrap();
    let mut world = world();
    for _ in 0..3 {
        host.tick(&mut world, 1. / 60.).unwrap();
    }
}

#[test]
fn caught_proxy_nested_failure_rolls_back_outer_and_numeric_writes() {
    let compiled = CompiledScript::compile(
        r#"
        let first=true;
        exports.default={persistence:{version:1},update(t){
          if(first){first=false;t.state.set(7);return}
          const descriptor=Object.getOwnPropertyDescriptor;
          const object=new Proxy({value:9},{getOwnPropertyDescriptor(o,key){
            try {t.state.set(NaN)} catch(_) {}
            return descriptor(o,key);
          }});
          t.state.set(8); t.chunks[0].transforms[0]=99;
          try {t.state.set(object)} catch(_) {}
        }};
        "#,
    )
    .unwrap();
    let mut host = ScriptHost::new(compiled).unwrap();
    let mut world = world();
    host.tick(&mut world, 1. / 60.).unwrap();
    let before = world.save_bytes().unwrap();
    assert!(host.tick(&mut world, 1. / 60.).is_err());
    assert_eq!(before, world.save_bytes().unwrap());
    assert_eq!(
        host.persistent
            .borrow()
            .entries
            .values()
            .next()
            .unwrap()
            .value,
        ScriptValue::Number(7.)
    );
}

#[test]
fn rejected_inspector_shapes_keep_transaction_poisoning() {
    for invalid in [
        "Object.defineProperty([1],'0',{enumerable:false})",
        "Object.assign([1],{extra:2})",
        "Object.assign([1],{[Symbol()]:2})",
        "Object.defineProperty([1],'0',{get(){throw Error('getter invoked')}})",
        "Object.setPrototypeOf([1],null)",
        "new Proxy({value:1},{ownKeys(){return ['value','value']}})",
        "new Proxy({value:1},{getPrototypeOf(){throw Error('prototype trap')}})",
    ] {
        let compiled = CompiledScript::compile(&format!(
            "let first=true; exports.default={{persistence:{{version:1}},update(t){{\
             if(first){{first=false;t.state.set(7);return}}\
             t.state.set(8); t.chunks[0].transforms[0]=99;\
             try{{t.state.set({invalid})}}catch(_){{}}}}}};"
        ))
        .unwrap();
        let mut host = ScriptHost::new(compiled).unwrap();
        let mut world = world();
        host.tick(&mut world, 1. / 60.).unwrap();
        let before = world.save_bytes().unwrap();
        assert!(host.tick(&mut world, 1. / 60.).is_err(), "{invalid}");
        assert_eq!(before, world.save_bytes().unwrap(), "{invalid}");
        assert_eq!(
            host.persistent
                .borrow()
                .entries
                .values()
                .next()
                .unwrap()
                .value,
            ScriptValue::Number(7.),
            "{invalid}"
        );
    }
}
