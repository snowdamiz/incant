use super::*;
#[allow(dead_code)]
#[path = "../../incant_runtime/tests/support/lights.rs"]
mod fixture;
use fixture::*;
#[test]
fn light_controls_commit_with_numeric_writes_and_expired_capabilities_never_reactivate() {
    let source = r#"let old;exports.default={update(t){if(old){let caught=false;try{old('00000000000000000000000001',5)}catch(_){caught=true}if(!caught)throw Error('stale light capability');}old=t.commands.setLightIntensity;t.chunks[0].transforms[0]+=1;t.commands.setLightColor('00000000000000000000000001',new Float64Array([0.2,0.3,0.4]));t.commands.setLightIntensity('00000000000000000000000002',0);}};"#;
    let mut h = ScriptHost::new(CompiledScript::compile(source).unwrap()).unwrap();
    let mut w = incant_runtime::NativeWorld::new(scene());
    for n in 1..=2 {
        h.tick(&mut w, 1. / 60.).unwrap();
        assert_eq!(
            w.inspect(id(1)).unwrap().transform.translation[0],
            f64::from(n)
        );
    }
    let d = declarations(&w);
    assert_eq!(d[0].directional.as_ref().unwrap().color, [0.2, 0.3, 0.4]);
    assert_eq!(d[1].point.as_ref().unwrap().intensity, 0.);
    assert_eq!(w.spatial_view().unwrap().binding_generation(), 0);
}
#[test]
fn caught_light_errors_and_late_native_refusal_rollback_numeric_structural_and_queued_light_values()
{
    for bad in [
        "try{t.commands.setLightColor(id,[NaN,0,0])}catch(_){}",
        "try{t.commands.setLightIntensity(id,1000000.00000001)}catch(_){}",
        "try{t.commands.setLightColor(id,[.2,.3])}catch(_){}",
        "t.commands.setLightIntensity('00000000000000000000000004',1)",
        "t.commands.setLightIntensity('00000000000000000000000099',1)",
        "try{t.commands.setLightColor(id,{length:3,0:.2,1:.3,2:.4})}catch(_){}",
        "try{t.commands.setLightColor(id,Object.defineProperty([0,0,0],0,{get(){t.commands.setLightIntensity(id,500);return NaN}}))}catch(_){}",
    ] {
        let source = format!(
            "exports.default={{update(t){{const id='00000000000000000000000001';t.chunks[0].transforms[0]=42;t.commands.setLightIntensity(id,99);t.commands.setVelocity(id,[3,0,0]);{bad}}}}};"
        );
        let mut h = ScriptHost::new(CompiledScript::compile(&source).unwrap()).unwrap();
        let mut w = incant_runtime::NativeWorld::new(scene());
        let before = w.save_bytes().unwrap();
        assert!(h.tick(&mut w, 1. / 60.).is_err(), "{bad}");
        assert_eq!(w.save_bytes().unwrap(), before, "{bad}");
        assert!(h.tick(&mut w, 1. / 60.).is_err());
    }
}

#[test]
fn checkpoint_restores_light_values_and_opted_in_day_night_phase_without_rebinding() {
    let code = CompiledScript::compile(
        r#"
      exports.default={persistence:{version:1},update(t){
        let phase=(t.state.get()||{phase:0}).phase+1;
        t.state.set({phase});
        t.commands.setLightIntensity('00000000000000000000000001',phase*100);
        t.commands.setLightColor('00000000000000000000000002',[phase/100,0.5,0.75]);
      }};
    "#,
    )
    .unwrap();
    let scene = scene();
    let mut a = NativeSession::new(
        scene.clone(),
        code.clone(),
        incant_input::InputActions::default(),
    )
    .unwrap();
    for _ in 0..3 {
        a.tick(&[]).unwrap();
    }
    let save = a.checkpoint_bytes().unwrap();
    let mut b = NativeSession::from_checkpoint_bytes(
        &save,
        &scene,
        code,
        incant_input::InputActions::default(),
    )
    .unwrap();
    assert_eq!(
        declarations(a.world().unwrap()),
        declarations(b.world().unwrap())
    );
    for phase in 4..=8 {
        a.tick(&[]).unwrap();
        b.tick(&[]).unwrap();
        let current = declarations(b.world().unwrap());
        assert_eq!(declarations(a.world().unwrap()), current);
        assert_eq!(
            current[0].directional.as_ref().unwrap().intensity,
            f64::from(phase) * 100.
        );
        assert_eq!(
            current[1].point.as_ref().unwrap().color,
            [f64::from(phase) / 100., 0.5, 0.75]
        );
        assert_eq!(
            b.world()
                .unwrap()
                .spatial_view()
                .unwrap()
                .binding_generation(),
            0
        );
    }
}
