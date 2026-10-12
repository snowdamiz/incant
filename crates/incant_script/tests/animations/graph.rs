use super::*;
const PARAMETER: &str = "0000000000000000000000000A";
const BLEND: &str = "0000000000000000000000000N";
fn graph_scene() -> CookedScene {
    let base = scene(2);
    let mut nodes = base.spatial().to_vec();
    let graph = NativeAnimationGraph::new(
        0,
        vec![NativeAnimationParameter {
            id: id(10),
            initial: 0.0,
            min: 0.0,
            max: 1.0,
        }],
        vec![
            NativeAnimationGraphState {
                id: id(20),
                motion: NativeAnimationMotion::Clip(Some(0)),
                looping: false,
            },
            NativeAnimationGraphState {
                id: id(21),
                motion: NativeAnimationMotion::Blend1d {
                    parameter: 0,
                    cycle_seconds: 2.0,
                    points: Arc::from([
                        NativeAnimationBlendPoint {
                            threshold: 0.0,
                            clip: 0,
                        },
                        NativeAnimationBlendPoint {
                            threshold: 1.0,
                            clip: 1,
                        },
                    ]),
                },
                looping: true,
            },
        ],
        vec![],
        &[2.0, 4.0],
    )
    .unwrap();
    for node in &mut nodes {
        Arc::make_mut(node.animation.as_mut().unwrap()).graph = Some(Arc::new(graph.clone()));
    }
    base.with_spatial(
        vec![VisualAssetRef {
            id: id(u128::MAX),
            kind: VisualAssetKind::Model,
            fingerprint: [4; 32],
        }],
        nodes,
    )
    .unwrap()
}
#[test]
fn graph_sdk_captures_owned_aligned_columns_and_commits_strict_controls() {
    let mut host = host(&format!(
        r#"let n=0,old;exports.default={{update({{animations,commands}}){{
        const g=animations.graphs,i=g.ids.indexOf('{ONE}'),o=g.parameterOffsets[i];
        if(g.ids.length!==2||g.states.length!==2||g.parameterOffsets.length!==3||g.parameterOffsets[2]!==2||g.parameterIds[o]!=='{PARAMETER}')throw Error('alignment');
        if(n===0){{old=g;commands.setAnimationParameter('{ONE}','{PARAMETER}',0.25);commands.selectAnimationState('{ONE}','{BLEND}');if(g.parameterValues[o]!==0)throw Error('queued');}}
        else{{if(old.parameterValues[o]!==0||old.seconds[i]!==0)throw Error('retained');old.parameterValues[o]=99;try{{old.states[i]='wrong'}}catch(_){{}}if(g.parameterValues[o]!==0.25||g.states[i]!=='{BLEND}'||g.weights[i]!==0.25||g.sourceClips[i]!==0||g.sourceSeconds[i]!==0.125||animations.seconds[animations.ids.indexOf('{ONE}')]!==0.25)throw Error('blend');}}
        n++;
    }}}};"#
    ));
    let mut world = NativeWorld::new(graph_scene());
    host.tick(&mut world, 0.125).unwrap();
    world.step().unwrap();
    host.tick(&mut world, 0.125).unwrap();
    assert_eq!(
        world.animation_graph_state(id(1)).unwrap().parameters[0],
        0.25
    );
    assert_eq!(
        world.animation_graph_state(id(2)).unwrap().parameters[0],
        0.0
    );
}
#[test]
fn graph_ingress_and_late_refusal_roll_back_numeric_controls_and_logs() {
    for tail in [
        format!("commands.setAnimationParameter('{ONE}','{PARAMETER}',2);"),
        format!("try{{commands.setAnimationParameter('{ONE}','{PARAMETER}','0.5')}}catch(_){{}}"),
        format!(
            "try{{commands.selectAnimationState('{ONE}',{{toString(){{throw Error('coerce')}}}})}}catch(_){{}}"
        ),
        format!("commands.selectAnimationState('{ONE}','0000000000000000000000000Z');"),
        format!("commands.controlAnimation('{ONE}','seek',0.5);"),
        "throw Error('late');".into(),
    ] {
        let mut host = host(&format!(
            r#"exports.default={{update({{chunks,commands,log}}){{chunks[0].transforms[0]=99;commands.setAnimationParameter('{ONE}','{PARAMETER}',0.5);commands.selectAnimationState('{ONE}','{BLEND}');log('prefix');{tail}}}}};"#
        ));
        let mut world = NativeWorld::new(graph_scene());
        let before = world.snapshot();
        let graph = world.animation_graph_state(id(1));
        assert!(host.tick(&mut world, 0.125).is_err(), "{tail}");
        assert_eq!(world.snapshot(), before);
        assert_eq!(world.animation_graph_state(id(1)), graph);
        assert!(matches!(host.logs(), Err(ScriptError::FailedSession)));
    }
}
#[test]
fn actual_guest_graph_controls_resume_active_paused_and_blended_sessions_exactly() {
    use incant_script::NativeSession;
    let base = graph_scene();
    let mut nodes = base.spatial().to_vec();
    for node in &mut nodes {
        let definition = Arc::make_mut(node.animation.as_mut().unwrap());
        let graph = definition.graph.as_ref().unwrap();
        let mut states = graph.states().to_vec();
        states.push(NativeAnimationGraphState {
            id: id(22),
            motion: NativeAnimationMotion::Clip(Some(1)),
            looping: true,
        });
        definition.graph = Some(Arc::new(
            NativeAnimationGraph::new(
                0,
                graph.parameters().to_vec(),
                states,
                vec![NativeAnimationGraphTransition {
                    id: id(30),
                    from: 0,
                    to: 2,
                    priority: 0,
                    fade_seconds: 1.0,
                    after_seconds: None,
                    conditions: Arc::from([NativeAnimationCondition {
                        parameter: 0,
                        comparison: incant_types::AnimationComparison::GreaterEqual,
                        value: 0.5,
                    }]),
                }],
                &definition.clip_durations,
            )
            .unwrap(),
        ));
    }
    let initial = base
        .with_spatial(
            vec![VisualAssetRef {
                id: id(u128::MAX),
                kind: VisualAssetKind::Model,
                fingerprint: [4; 32],
            }],
            nodes,
        )
        .unwrap();
    let code=CompiledScript::compile(&format!(r#"exports.default={{persistence:{{version:1}},update(t){{
        const k=t.clock.tick,c=t.commands,g=t.animations.graphs;let count=Number(t.state.get()||0)+1;if(count!==k)throw Error('persistent clock');t.state.set(count);
        if(k===1)c.setAnimationParameter('{ONE}','{PARAMETER}',0.5);if(k===3)c.controlAnimation('{ONE}','playing',false);if(k===6)c.controlAnimation('{ONE}','playing',true);
        if(k===15){{c.selectAnimationState('{ONE}','{BLEND}');c.setAnimationParameter('{ONE}','{PARAMETER}',0.25);}}
        const i=g.ids.indexOf('{ONE}');t.chunks[0].transforms[0]=count+g.seconds[i];
    }}}};"#)).unwrap();
    for split in [2, 4, 10, 18] {
        let mut baseline =
            NativeSession::new(initial.clone(), code.clone(), Default::default()).unwrap();
        for _ in 0..split {
            baseline.tick(&[]).unwrap();
        }
        let saved = baseline.checkpoint_bytes().unwrap();
        let mut fresh = NativeSession::from_checkpoint_bytes(
            &saved,
            &initial,
            code.clone(),
            Default::default(),
        )
        .unwrap();
        let mut restored =
            NativeSession::new(initial.clone(), code.clone(), Default::default()).unwrap();
        restored.tick(&[]).unwrap();
        restored.restore_checkpoint_bytes(&saved).unwrap();
        assert_eq!(fresh.checkpoint_bytes().unwrap(), saved);
        assert_eq!(restored.checkpoint_bytes().unwrap(), saved);
        for _ in 0..30 {
            baseline.tick(&[]).unwrap();
            fresh.tick(&[]).unwrap();
            restored.tick(&[]).unwrap();
            let expected = baseline.checkpoint_bytes().unwrap();
            assert_eq!(fresh.checkpoint_bytes().unwrap(), expected);
            assert_eq!(restored.checkpoint_bytes().unwrap(), expected);
        }
    }
}
