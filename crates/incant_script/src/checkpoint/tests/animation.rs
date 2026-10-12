use super::*;
use incant_localization::LocaleSettings;
use incant_runtime::{
    CookedSpatial, NativeAnimation, NativeMeshRenderer, VisualAssetKind, VisualAssetRef,
};
use incant_types::Animation;
use std::sync::Arc;

fn initial() -> CookedScene {
    let mut node = CookedSpatial::empty(id(1));
    node.mesh = Some(NativeMeshRenderer {
        mesh: id(3),
        cast_shadows: false,
    });
    node.animation = Some(Arc::new(NativeAnimation {
        graph: None,
        mesh_fingerprint: [4; 32],
        clip_durations: Arc::from([2., 0.]),
        initial: Animation {
            clip: Some(0),
            seconds: 0.,
            speed: 1.,
            playing: true,
            looping: false,
        },
    }));
    localized("Hello")
        .with_spatial(
            vec![VisualAssetRef {
                id: id(3),
                kind: VisualAssetKind::Model,
                fingerprint: [4; 32],
            }],
            vec![node],
        )
        .unwrap()
}
fn behavior() -> CompiledScript {
    CompiledScript::compile(&format!(r#"
    exports.default={{persistence:{{version:1}},update(t){{
        const i=t.animations.ids.indexOf({:?});
        if(i<0)throw Error('animation missing');
        let s=t.state.get()||{{count:0,events:0}};
        s={{count:s.count+1,events:s.events+t.timers.events.length,
            seconds:t.animations.seconds[i],flags:t.animations.flags[i]}};
        t.state.set(s);t.chunks[0].transforms[0]=s.count+s.events+s.seconds;
        if(t.clock.tick===1){{
            t.timers.set('beat',3,3);
            t.commands.setLocale({{locale:'fr-CA',fallbacks:['en'],pseudo:true}});
        }}else{{
            const locale=t.localization.locale();
            if(locale.locale!=='fr-CA'||locale.fallbacks[0]!=='en'||!locale.pseudo)throw Error('locale lost');
            if(!t.localization.localize({{table_id:'00000000000000000000000002',key:'hello'}}).text)throw Error('catalog lost');
        }}
        const entity=t.animations.ids[i];
        if(t.clock.tick===2){{
            t.commands.controlAnimation(entity,'seek',1);
            t.commands.controlAnimation(entity,'speed',-0.5);
            t.commands.controlAnimation(entity,'looping',true);
        }}
        if(t.clock.tick===4)t.commands.controlAnimation(entity,'playing',false);
        if(t.clock.tick===5)t.commands.controlAnimation(entity,'playing',true);
        if(t.clock.tick===7)t.commands.controlAnimation(entity,'seek',0.5);
    }}}};"#, ulid::Ulid::from_bytes(id(1).0).to_string())).unwrap()
}

#[test]
fn v4_animation_locale_state_and_timers_resume_through_fresh_and_in_place_load() {
    let initial = initial();
    let code = behavior();
    let mut baseline =
        NativeSession::new(initial.clone(), code.clone(), Default::default()).unwrap();
    for _ in 0..5 {
        baseline.tick(&[]).unwrap();
    }
    let saved = baseline.checkpoint_bytes().unwrap();
    let sections = codec::sections(&saved, identity(&code)).unwrap();
    assert_eq!(&sections.world[8..12], &4u32.to_le_bytes());
    let mut fresh =
        NativeSession::from_checkpoint_bytes(&saved, &initial, code.clone(), Default::default())
            .unwrap();
    let mut restored = NativeSession::new(initial, code, Default::default()).unwrap();
    restored.tick(&[]).unwrap();
    restored.restore_checkpoint_bytes(&saved).unwrap();
    let locale = LocaleSettings {
        locale: "fr-CA".into(),
        fallbacks: vec!["en".into()],
        pseudo: true,
    };
    for session in [&fresh, &restored] {
        assert_eq!(session.checkpoint_bytes().unwrap(), saved);
        assert_eq!(session.world().unwrap().localization().settings(), &locale);
        assert_eq!(
            session.world().unwrap().animation_state(id(1)),
            baseline.world().unwrap().animation_state(id(1))
        );
    }
    for _ in 0..50 {
        baseline.tick(&[]).unwrap();
        fresh.tick(&[]).unwrap();
        restored.tick(&[]).unwrap();
        let expected = baseline.checkpoint_bytes().unwrap();
        assert_eq!(fresh.checkpoint_bytes().unwrap(), expected);
        assert_eq!(restored.checkpoint_bytes().unwrap(), expected);
    }
    assert_eq!(restored.world().unwrap().tick(), 55);
}

#[test]
fn active_crossfade_v5_resumes_through_ordinary_session_checkpoint() {
    use incant_runtime::{AnimationControl, AnimationSample, FrameCommands, StructuralCommand};
    let initial = initial();
    let code = CompiledScript::compile("exports.default={persistence:{version:1},update(t){let s=t.state.get()||{ticks:0};t.state.set({ticks:s.ticks+1});}};").unwrap();
    let mut baseline =
        NativeSession::new(initial.clone(), code.clone(), Default::default()).unwrap();
    let mut commands = FrameCommands::default();
    commands
        .push(StructuralCommand::ControlAnimation {
            entity: id(1),
            control: AnimationControl::Crossfade {
                target: AnimationSample {
                    clip: Some(0),
                    seconds: 1.0,
                },
                duration: 0.5,
            },
        })
        .unwrap();
    baseline.world.apply(&mut commands).unwrap();
    for _ in 0..7 {
        baseline.tick(&[]).unwrap();
    }
    let saved = baseline.checkpoint_bytes().unwrap();
    assert_eq!(
        &codec::sections(&saved, identity(&code)).unwrap().world[8..12],
        &5u32.to_le_bytes()
    );
    let mut fresh =
        NativeSession::from_checkpoint_bytes(&saved, &initial, code.clone(), Default::default())
            .unwrap();
    let mut restored = NativeSession::new(initial, code.clone(), Default::default()).unwrap();
    restored.restore_checkpoint_bytes(&saved).unwrap();
    for _ in 0..50 {
        baseline.tick(&[]).unwrap();
        fresh.tick(&[]).unwrap();
        restored.tick(&[]).unwrap();
        assert_eq!(
            fresh.checkpoint_bytes().unwrap(),
            baseline.checkpoint_bytes().unwrap()
        );
        assert_eq!(
            restored.checkpoint_bytes().unwrap(),
            baseline.checkpoint_bytes().unwrap()
        );
    }
    let completed = baseline.checkpoint_bytes().unwrap();
    assert_eq!(
        &codec::sections(&completed, identity(&code)).unwrap().world[8..12],
        &4u32.to_le_bytes()
    );
}

#[test]
fn v4_load_refuses_legacy_and_wrong_bindings_without_replacing_live_session() {
    let initial = initial();
    let code = behavior();
    let mut live = NativeSession::new(initial.clone(), code.clone(), Default::default()).unwrap();
    live.tick(&[]).unwrap();
    let saved = live.checkpoint_bytes().unwrap();
    let incarnation = live.world().unwrap().incarnation();
    let mut plain = NativeWorld::new(scene()).save_bytes().unwrap();
    plain[16..24].copy_from_slice(&1u64.to_le_bytes());
    let mut v1 = plain[..64].to_vec();
    v1[8..12].copy_from_slice(&1u32.to_le_bytes());
    v1.extend(scene().to_bytes());
    let size = (v1.len() - 64) as u64;
    v1[24..32].copy_from_slice(&size.to_le_bytes());
    plain[8..12].copy_from_slice(&2u32.to_le_bytes());
    for mut world in [v1, plain] {
        rehash_world(&mut world);
        assert!(NativeWorld::from_save_bytes(&world).is_ok());
        let bad = replace_section(&saved, 0, &world);
        assert!(matches!(
            live.restore_checkpoint_bytes(&bad),
            Err(CheckpointError::Invalid(
                "session requires world save v3-v11"
            ))
        ));
        assert!(matches!(
            NativeSession::from_checkpoint_bytes(&bad, &initial, code.clone(), Default::default()),
            Err(CheckpointError::Invalid(
                "session requires world save v3-v11"
            ))
        ));
        assert_eq!(live.checkpoint_bytes().unwrap(), saved);
        assert_eq!(live.world().unwrap().incarnation(), incarnation);
    }
    for version in [0, 12, u32::MAX] {
        let sections = codec::sections(&saved, identity(&code)).unwrap();
        let mut world = sections.world.to_vec();
        world[8..12].copy_from_slice(&version.to_le_bytes());
        rehash_world(&mut world);
        let bad = replace_section(&saved, 0, &world);
        assert!(matches!(
            live.restore_checkpoint_bytes(&bad),
            Err(CheckpointError::Invalid(
                "session requires world save v3-v11"
            ))
        ));
        assert_eq!(live.checkpoint_bytes().unwrap(), saved);
        assert_eq!(live.world().unwrap().incarnation(), incarnation);
    }
    for short in [&[][..], &b"INCANTSV\x04\0\0"[..]] {
        let bad = replace_section(&saved, 0, short);
        assert!(matches!(
            live.restore_checkpoint_bytes(&bad),
            Err(CheckpointError::Invalid(
                "session requires world save v3-v11"
            ))
        ));
        assert_eq!(live.checkpoint_bytes().unwrap(), saved);
    }
    let mut spatial = initial.spatial().to_vec();
    let mut changed = (**spatial[0].animation.as_ref().unwrap()).clone();
    changed.clip_durations = Arc::from([3., 0.]);
    spatial[0].animation = Some(Arc::new(changed));
    let different_animation = initial
        .clone()
        .with_spatial(initial.visual_assets().to_vec(), spatial)
        .unwrap();
    let different_catalog = initial
        .clone()
        .with_localization(localized("Different catalog").localization().clone())
        .unwrap();
    for (foreign, forged_origin) in [(different_animation, false), (different_catalog, true)] {
        let mut world = NativeWorld::new(foreign.clone()).save_bytes().unwrap();
        // Origin is a compatibility label, not authentication. A legitimately
        // different initial animation has a different binding; immutable tables
        // must also refuse even with a forged matching origin and valid hashes.
        if forged_origin {
            world[64..96].copy_from_slice(&live.world().unwrap().source_scene_fingerprint());
        }
        world[16..24].copy_from_slice(&1u64.to_le_bytes());
        rehash_world(&mut world);
        let bad = replace_section(&saved, 0, &world);
        assert!(matches!(
            live.restore_checkpoint_bytes(&bad),
            Err(CheckpointError::World(SaveError::WorldMismatch))
        ));
        assert!(matches!(
            NativeSession::from_checkpoint_bytes(
                &saved,
                &foreign,
                code.clone(),
                Default::default()
            ),
            Err(CheckpointError::World(SaveError::WorldMismatch))
        ));
        assert_eq!(live.checkpoint_bytes().unwrap(), saved);
        assert_eq!(live.world().unwrap().incarnation(), incarnation);
    }
    let different =
        CompiledScript::compile("exports.default={persistence:{version:1},update(){}};").unwrap();
    assert!(matches!(
        NativeSession::from_checkpoint_bytes(&saved, &initial, different, Default::default()),
        Err(CheckpointError::ProgramMismatch)
    ));
    live.tick(&[]).unwrap();
    assert_eq!(live.world().unwrap().tick(), 2);
}
