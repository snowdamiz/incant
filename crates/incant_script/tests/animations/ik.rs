use super::*;
fn ik_scene() -> CookedScene {
    let initial = scene(2);
    let mut nodes = initial.spatial().to_vec();
    for n in &mut nodes {
        n.ik = Some(Arc::new(NativeIk {
            mesh_fingerprint: [4; 32],
            initial: Arc::from([AnimationIkSample {
                id: id(9),
                chain: [0, 1, 2],
                target: [1., 1., 0.],
                pole: [0., 1., 0.],
                weight: 1.,
            }]),
        }));
    }
    initial
        .clone()
        .with_spatial(initial.visual_assets().to_vec(), nodes)
        .unwrap()
}
#[test]
fn owned_ik_columns_and_accepted_controls_use_model_coordinates() {
    let mut world = NativeWorld::new(ik_scene());
    let mut host = host(
        r#"let old;exports.default={update({animations,commands}){
      const s=animations.ik;if(s.ids.length!==2||s.chains.length!==6||s.targets.length!==6||s.weights.length!==2)throw Error('shape');
      if(old&&old.targets[0]!==999)throw Error('owned');
      for(let i=0;i<s.ids.length;i++)commands.setAnimationIk(s.ids[i],s.constraintIds[i],s.targets[3*i]+1,1,0,0,1,0,.5);
      old=s;s.targets[0]=999;
    }}"#,
    );
    host.tick(&mut world, 0.125).unwrap();
    assert_eq!(world.ik_state(id(1)).unwrap()[0].target, [2., 1., 0.]);
    host.tick(&mut world, 0.125).unwrap();
    assert_eq!(world.ik_state(id(1)).unwrap()[0].target, [3., 1., 0.]);
    assert_eq!(world.ik_state(id(2)).unwrap()[0].weight, 0.5);
}
#[test]
fn caught_invalid_ik_and_script_traps_reject_numeric_and_all_owner_changes() {
    for tail in [
        "throw Error('after IK')",
        "try{commands.setAnimationIk(s.ids[1],s.constraintIds[1],NaN,1,0,0,1,0,1)}catch{}",
        "commands.setAnimationIk(s.ids[1],'00000000000000000000000099',2,1,0,0,1,0,1)",
    ] {
        let mut world = NativeWorld::new(ik_scene());
        let saved = world.save_bytes().unwrap();
        let mut host = host(&format!(
            "exports.default={{update({{animations,commands,chunks}}){{const s=animations.ik;chunks[0].transforms[0]=27;commands.setAnimationIk(s.ids[0],s.constraintIds[0],2,1,0,0,1,0,.5);{tail};}}}}"
        ));
        assert!(host.tick(&mut world, 0.125).is_err());
        assert_eq!(world.save_bytes().unwrap(), saved);
    }
}
#[test]
fn guest_prototype_setters_cannot_intercept_ik_snapshots_or_reactivate_commands() {
    let mut world = NativeWorld::new(ik_scene());
    let mut host = host(
        r#"for(const k of ['ik','constraintIds','targets','poles','weights','chains'])Object.defineProperty(Object.prototype,k,{set(){throw Error('prototype')}});let old;
    exports.default={update({animations,commands}){if(old){try{old('00000000000000000000000001','00000000000000000000000009',9,1,0,0,1,0,1);throw Error('revoked')}catch(e){if(e.message==='revoked')throw e}}old=commands.setAnimationIk;const s=animations.ik;if(s.targets[0]!==1)throw Error('retained state');}}"#,
    );
    host.tick(&mut world, 0.125).unwrap();
    host.tick(&mut world, 0.125).unwrap();
}
