use super::*;
use incant_input::{InputEvent, KeyCode};
use incant_runtime::{CookedEntity, CookedScene};
mod animation;
fn id(n: u8) -> StableId {
    StableId([n; 16])
}
fn scene() -> CookedScene {
    CookedScene::new(
        id(99),
        60,
        vec![CookedEntity {
            id: id(1),
            parent: None,
            transform: Default::default(),
            velocity: None,
        }],
    )
    .unwrap()
}
fn code() -> CompiledScript {
    CompiledScript::compile(
        r#"
      let ephemeral=0;
      exports.default={persistence:{version:1},update(t){
        ephemeral++;
        let s=t.state.get() || {count:0,events:0};
        s={count:s.count+1,events:s.events+t.timers.events.length};
        if(t.clock.tick===1)t.timers.set('beat',3,3);
        t.state.set(s);t.chunks[0].transforms[0]=s.count*2+s.events;
        t.chunks[0].transforms[1]=ephemeral;
      }};
    "#,
    )
    .unwrap()
}
fn session(code: CompiledScript) -> NativeSession {
    NativeSession::new(scene(), code, InputActions::default()).unwrap()
}
fn x(session: &NativeSession) -> f64 {
    session
        .world()
        .unwrap()
        .inspect(id(1))
        .unwrap()
        .transform
        .translation[0]
}
#[test]
fn checkpoint_continues_data_timers_and_clock_but_reinitializes_ephemeral_globals() {
    let code = code();
    let mut baseline = session(code.clone());
    for _ in 0..5 {
        baseline.tick(&[]).unwrap();
    }
    let bytes = baseline.checkpoint_bytes().unwrap();
    let mut restored =
        NativeSession::from_checkpoint_bytes(&bytes, &scene(), code, InputActions::default())
            .unwrap();
    assert_eq!(
        restored.world().unwrap().save_bytes().unwrap(),
        baseline.world().unwrap().save_bytes().unwrap()
    );
    for _ in 0..20 {
        baseline.tick(&[]).unwrap();
        restored.tick(&[]).unwrap();
        assert_eq!(x(&baseline), x(&restored));
    }
    assert_eq!(restored.world().unwrap().tick(), 25);
    assert_eq!(
        restored
            .world()
            .unwrap()
            .inspect(id(1))
            .unwrap()
            .transform
            .translation[1],
        20.
    );
    assert_eq!(
        baseline
            .world()
            .unwrap()
            .inspect(id(1))
            .unwrap()
            .transform
            .translation[1],
        25.
    );
    assert_eq!(
        host(&baseline).unwrap().schedule.elapsed_seconds.to_bits(),
        host(&restored).unwrap().schedule.elapsed_seconds.to_bits()
    );
}
#[test]
fn corrupt_or_mismatched_restore_is_atomic_and_opt_in_is_required() {
    let code = code();
    let mut live = session(code.clone());
    live.tick(&[]).unwrap();
    let saved = live.checkpoint_bytes().unwrap();
    let identity = live.world().unwrap().incarnation();
    for index in [0, 8, 12, 16, 48, 80, 96, 128, saved.len() - 1] {
        let mut bad = saved.clone();
        bad[index] ^= 1;
        assert!(live.restore_checkpoint_bytes(&bad).is_err());
        assert_eq!(live.checkpoint_bytes().unwrap(), saved);
        assert_eq!(live.world().unwrap().incarnation(), identity);
    }
    let different =
        CompiledScript::compile("exports.default={persistence:{version:1},update(){}};").unwrap();
    assert!(matches!(
        NativeSession::from_checkpoint_bytes(&saved, &scene(), different, InputActions::default()),
        Err(CheckpointError::ProgramMismatch)
    ));
    let unopted = CompiledScript::compile("exports.default={update(){}};").unwrap();
    assert!(matches!(
        session(unopted).checkpoint_bytes(),
        Err(CheckpointError::Unsupported(_))
    ));
    live.tick(&[]).unwrap();
    assert!(x(&live) > 0.);
}
#[test]
fn ordinary_load_neutralizes_physical_controls_without_false_edges() {
    let mut live = session(code());
    live.tick(&[InputEvent::Key {
        code: KeyCode::Space,
        down: true,
    }])
    .unwrap();
    let bytes = live.checkpoint_bytes().unwrap();
    live.restore_checkpoint_bytes(&bytes).unwrap();
    let frame = live.input().unwrap();
    assert!(frame.keyboard.held.is_empty());
    assert!(frame.keyboard.pressed.is_empty());
    assert!(frame.keyboard.released.is_empty());
    live.tick(&[]).unwrap();
    assert!(live.input().unwrap().keyboard.released.is_empty());
    live.tick(&[InputEvent::Key {
        code: KeyCode::Space,
        down: true,
    }])
    .unwrap();
    assert!(
        live.input()
            .unwrap()
            .keyboard
            .pressed
            .contains(&KeyCode::Space)
    );
}

fn grouped(second_opted: bool) -> (CookedScene, CompiledScript) {
    use crate::{ScriptBindings, ScriptInstance};
    let mut entities = scene().entities().to_vec();
    let mut e = entities[0].clone();
    e.id = id(2);
    entities.push(e);
    let scene = CookedScene::new(id(99), 60, entities).unwrap();
    let bindings = ScriptBindings::new(
        Sha256::digest(scene.to_bytes()).into(),
        vec![id(101), id(102)],
        vec![
            ScriptInstance {
                entity: id(1),
                source: id(101),
                props: Default::default(),
            },
            ScriptInstance {
                entity: id(2),
                source: id(102),
                props: Default::default(),
            },
        ],
    )
    .unwrap();
    let text = format!(
        r#"
    let old;
    function behavior(index,opted){{return{{persistence:opted?{{version:1}}:undefined,update(t){{
      if(old){{let failed=false;try{{old.get()}}catch(_){{failed=true}}if(!failed)throw Error('prior source reactivated')}}
      if(opted){{
        const i=t.instances[0];const n=Number(t.state.get(i.entity)||0)+index;
        t.state.set(n,i.entity);t.state.set(Number(t.state.get()||0)+t.timers.events.length);
        for(const c of t.chunks)for(let r=0;r<c.ids.length;r++)if(c.ids[r]===i.entity)c.transforms[r*10]=n;
      }}
      if(t.clock.tick===1)t.timers.set('same',index,index);
      old=t.state;
    }}}}}}
    exports.default={{sources:[{{id:{:?},behavior:behavior(1,true)}},{{id:{:?},behavior:behavior(2,{second_opted})}}]}};
    "#,
        ulid::Ulid::from_bytes(id(101).0).to_string(),
        ulid::Ulid::from_bytes(id(102).0).to_string()
    );
    let compiled = CompiledScript::compile_project("checkpoint-groups", &text, bindings).unwrap();
    (scene, compiled)
}
#[test]
fn grouped_scopes_and_retired_instance_lifetimes_survive_restore() {
    use incant_runtime::{FrameCommands, StructuralCommand};
    let (scene, code) = grouped(true);
    let mut original = NativeSession::new(scene.clone(), code.clone(), Default::default()).unwrap();
    for _ in 0..4 {
        original.tick(&[]).unwrap();
    }
    let mut restored = NativeSession::from_checkpoint_bytes(
        &original.checkpoint_bytes().unwrap(),
        &scene,
        code,
        Default::default(),
    )
    .unwrap();
    for _ in 0..5 {
        original.tick(&[]).unwrap();
        restored.tick(&[]).unwrap();
        assert_eq!(
            original.checkpoint_bytes().unwrap(),
            restored.checkpoint_bytes().unwrap()
        );
    }
    // A host-side structural change is also a committed state. Reusing its ULID
    // must not make the original ScriptComponent attach to this new lifetime.
    let mut commands = FrameCommands::default();
    commands.push(StructuralCommand::Despawn(id(2))).unwrap();
    original.world.apply(&mut commands).unwrap();
    commands
        .push(StructuralCommand::Spawn(scene.entities()[1].clone()))
        .unwrap();
    original.world.apply(&mut commands).unwrap();
    let bytes = original.checkpoint_bytes().unwrap();
    restored.restore_checkpoint_bytes(&bytes).unwrap();
    for _ in 0..3 {
        original.tick(&[]).unwrap();
        restored.tick(&[]).unwrap();
    }
    assert_eq!(
        original.checkpoint_bytes().unwrap(),
        restored.checkpoint_bytes().unwrap()
    );
    assert_eq!(
        restored
            .world()
            .unwrap()
            .inspect(id(2))
            .unwrap()
            .transform
            .translation[0],
        0.
    );
    assert_eq!(
        host(&restored)
            .unwrap()
            .project
            .as_ref()
            .unwrap()
            .active_ids(&restored.world)
            .unwrap(),
        vec![id(1)]
    );
    assert!(
        !host(&original)
            .unwrap()
            .persistent
            .borrow()
            .entries
            .keys()
            .any(|k| k.entity == Some(id(2)))
    );
}
#[test]
fn non_opted_active_group_refuses_until_it_is_permanently_retired() {
    use incant_runtime::{FrameCommands, StructuralCommand};
    let (scene, code) = grouped(false);
    let mut session = NativeSession::new(scene, code, Default::default()).unwrap();
    session.tick(&[]).unwrap();
    assert!(matches!(
        session.checkpoint_bytes(),
        Err(CheckpointError::Unsupported(_))
    ));
    let mut commands = FrameCommands::default();
    commands.push(StructuralCommand::Despawn(id(2))).unwrap();
    session.world.apply(&mut commands).unwrap();
    let bytes = session.checkpoint_bytes().unwrap();
    session.restore_checkpoint_bytes(&bytes).unwrap();
    session.tick(&[]).unwrap();
    assert_eq!(session.world().unwrap().tick(), 2);
}
fn replace_section(bytes: &[u8], index: usize, payload: &[u8]) -> Vec<u8> {
    let lens: Vec<usize> = (0..4)
        .map(|i| u32::from_le_bytes(bytes[80 + i * 4..84 + i * 4].try_into().unwrap()) as usize)
        .collect();
    let start = 128 + lens[..index].iter().sum::<usize>();
    let end = start + lens[index];
    let mut out = bytes[..start].to_vec();
    out.extend(payload);
    out.extend(&bytes[end..]);
    out[80 + index * 4..84 + index * 4].copy_from_slice(&(payload.len() as u32).to_le_bytes());
    let mut h = Sha256::new();
    h.update(&out[..96]);
    h.update(&out[128..]);
    out[96..128].copy_from_slice(&h.finalize());
    out
}
#[test]
fn checksum_valid_malformed_sections_refuse_without_publishing_a_candidate() {
    let mut live = session(code());
    live.tick(&[]).unwrap();
    let saved = live.checkpoint_bytes().unwrap();
    let mut cases = Vec::new();
    cases.push((1, u32::MAX.to_le_bytes().to_vec()));
    let mut duplicate = 2u32.to_le_bytes().to_vec();
    duplicate.extend(id(1).0);
    duplicate.extend(id(1).0);
    cases.push((1, duplicate));
    cases.push((2, u32::MAX.to_le_bytes().to_vec()));
    let mut nan = vec![1, 0, 0, 0, 0, 0, 3];
    nan.extend(f64::NAN.to_bits().to_le_bytes());
    cases.push((2, nan));
    cases.push((2, vec![1, 0, 0, 0, 2, 0, 0])); // Unknown source tag.
    cases.push((2, vec![1, 0, 0, 0, 0, 0, 5, 255, 255, 255, 255]));
    let mut deep = vec![1, 0, 0, 0, 0, 0];
    for _ in 0..17 {
        deep.extend([5, 1, 0, 0, 0]);
    }
    deep.push(0);
    cases.push((2, deep));
    let mut unknown_instance = vec![1, 0, 0, 0, 0, 1];
    unknown_instance.extend(id(1).0);
    unknown_instance.push(0);
    cases.push((2, unknown_instance));
    let sections = codec::sections(&saved, identity(&host(&live).unwrap()._compiled)).unwrap();
    let mut clock = sections.schedule.to_vec();
    clock[..8].copy_from_slice(&2u64.to_le_bytes());
    cases.push((3, clock));
    let mut timer = sections.schedule.to_vec();
    let n = timer.len();
    timer[n - 12..n - 4].copy_from_slice(&1u64.to_le_bytes());
    cases.push((3, timer));
    for (section, data) in cases {
        let bad = replace_section(&saved, section, &data);
        assert!(
            live.restore_checkpoint_bytes(&bad).is_err(),
            "section {section}: {data:?}"
        );
        assert_eq!(saved, live.checkpoint_bytes().unwrap());
    }
    for end in [0, 1, 127, 128, saved.len() - 1] {
        assert!(live.restore_checkpoint_bytes(&saved[..end]).is_err());
    }
    live.tick(&[]).unwrap();
    assert_eq!(live.world().unwrap().tick(), 2);
}
#[test]
fn failed_sessions_and_physics_worlds_keep_their_refusals() {
    use incant_runtime::CookedPhysics;
    let bad = CompiledScript::compile(
        "exports.default={persistence:{version:1},update(){throw Error('failed')}}",
    )
    .unwrap();
    let mut failed = session(bad);
    assert!(failed.tick(&[]).is_err());
    assert!(matches!(
        failed.checkpoint_bytes(),
        Err(CheckpointError::FailedSession)
    ));
    let physical = scene()
        .with_physics(vec![CookedPhysics {
            entity: id(1),
            body: Some(Default::default()),
            collider: Default::default(),
            angular_velocity: None,
        }])
        .unwrap();
    let physical = NativeSession::new(physical, code(), Default::default()).unwrap();
    assert!(matches!(
        physical.checkpoint_bytes(),
        Err(CheckpointError::World(SaveError::PhysicsUnsupported))
    ));
}
#[test]
fn compatibility_includes_actual_dynamic_code_capability_and_original_world() {
    let code = code();
    let saved = session(code.clone()).checkpoint_bytes().unwrap();
    let mut no_eval = code.clone();
    no_eval.allow_source = false;
    assert!(matches!(
        NativeSession::from_checkpoint_bytes(&saved, &scene(), no_eval, Default::default()),
        Err(CheckpointError::ProgramMismatch)
    ));
    let different_scene = CookedScene::new(id(98), 60, scene().entities().to_vec()).unwrap();
    let mut live = NativeSession::new(different_scene, code, Default::default()).unwrap();
    let before = live.checkpoint_bytes().unwrap();
    assert!(matches!(
        live.restore_checkpoint_bytes(&saved),
        Err(CheckpointError::WorldMismatch)
    ));
    assert_eq!(live.checkpoint_bytes().unwrap(), before);
}
#[test]
fn maximum_scoped_timer_records_roundtrip_without_short_section_ceiling() {
    let (scene, code) = grouped(true);
    let mut live = NativeSession::new(scene, code, Default::default()).unwrap();
    live.tick(&[]).unwrap();
    let timers = match &mut live.host {
        SessionVm::Bytecode(host) => &mut host.schedule.timers,
        #[cfg(feature = "wasmtime-host")]
        SessionVm::DevelopmentWasm(_) => unreachable!(),
        #[cfg(feature = "static-module")]
        SessionVm::Static(_) => unreachable!(),
    };
    timers.clear();
    for n in 0..crate::schedule::MAX_TIMERS {
        timers.insert(
            crate::schedule::Key {
                scope: Scope::Source(id(101)),
                id: format!("{n:064}"),
            },
            crate::schedule::Timer {
                due: 2,
                interval: Some(u32::MAX),
            },
        );
    }
    let bytes = live.checkpoint_bytes().unwrap();
    live.restore_checkpoint_bytes(&bytes).unwrap();
    assert_eq!(bytes, live.checkpoint_bytes().unwrap());
}

fn rehash_world(bytes: &mut [u8]) {
    let mut h = Sha256::new();
    h.update(&bytes[..32]);
    h.update(&bytes[64..]);
    bytes[32..64].copy_from_slice(&h.finalize());
}
fn localized(text: &str) -> CookedScene {
    use incant_localization::StringTable;
    use incant_runtime::NativeLocalization;
    use std::collections::BTreeMap;
    let table = "00000000000000000000000002";
    scene()
        .with_localization(
            NativeLocalization::new(
                BTreeMap::from([(
                    table.into(),
                    StringTable {
                        id: table.into(),
                        name: "Game".into(),
                        source_locale: "en".into(),
                        messages: BTreeMap::from([(
                            "hello".into(),
                            BTreeMap::from([("en".into(), text.into())]),
                        )]),
                    },
                )]),
                Default::default(),
            )
            .unwrap(),
        )
        .unwrap()
}
#[test]
fn refreshed_checksums_cannot_replace_immutable_scene_rate_or_catalog() {
    let initial = localized("Hello");
    let code = code();
    let mut live = NativeSession::new(initial.clone(), code.clone(), Default::default()).unwrap();
    live.tick(&[]).unwrap();
    let saved = live.checkpoint_bytes().unwrap();
    let incarnation = live.world().unwrap().incarnation();
    let origin = live.world().unwrap().source_scene_fingerprint();
    let cases = [
        CookedScene::new(id(98), 60, initial.entities().to_vec())
            .unwrap()
            .with_localization(initial.localization().clone())
            .unwrap(),
        CookedScene::new(id(99), 30, initial.entities().to_vec())
            .unwrap()
            .with_localization(initial.localization().clone())
            .unwrap(),
        localized("Changed"),
    ];
    for foreign in cases {
        let mut world = NativeWorld::new(foreign).save_bytes().unwrap();
        // Keep plausible origin and committed tick while changing immutable
        // nested data. All nested and outer checksums are valid.
        world[64..96].copy_from_slice(&origin);
        world[16..24].copy_from_slice(&1u64.to_le_bytes());
        rehash_world(&mut world);
        let bad = replace_section(&saved, 0, &world);
        assert!(matches!(
            live.restore_checkpoint_bytes(&bad),
            Err(CheckpointError::World(SaveError::WorldMismatch))
        ));
        assert!(matches!(
            NativeSession::from_checkpoint_bytes(&bad, &initial, code.clone(), Default::default()),
            Err(CheckpointError::World(SaveError::WorldMismatch))
        ));
        assert_eq!(live.checkpoint_bytes().unwrap(), saved);
        assert_eq!(live.world().unwrap().incarnation(), incarnation);

        // Even a trusted program whose initialization would throw is not run
        // until immutable scene validation succeeds.
        let throws = CompiledScript::compile("throw Error('module initialization');").unwrap();
        let mut bad = bad;
        bad[16..48].copy_from_slice(&identity(&throws));
        let bad = replace_section(&bad, 0, &world);
        assert!(matches!(
            NativeSession::from_checkpoint_bytes(&bad, &initial, throws, Default::default()),
            Err(CheckpointError::World(SaveError::WorldMismatch))
        ));
    }
    let wrong_initial = localized("Another game catalog");
    assert!(matches!(
        NativeSession::from_checkpoint_bytes(&saved, &wrong_initial, code, Default::default()),
        Err(CheckpointError::World(SaveError::WorldMismatch))
    ));
    live.tick(&[]).unwrap();
    assert_eq!(live.world().unwrap().tick(), 2);
}
#[test]
fn current_locale_is_restored_while_trusted_initial_capabilities_still_refuse() {
    use incant_runtime::{CookedPhysics, FrameCommands, StructuralCommand};
    let initial = localized("Hello");
    let code = code();
    let mut live = NativeSession::new(initial.clone(), code.clone(), Default::default()).unwrap();
    let mut commands = FrameCommands::default();
    let mut locale = initial.localization().settings().clone();
    locale.pseudo = true;
    commands
        .push(StructuralCommand::SetLocale(locale.clone()))
        .unwrap();
    live.world.apply(&mut commands).unwrap();
    let bytes = live.checkpoint_bytes().unwrap();
    let loaded =
        NativeSession::from_checkpoint_bytes(&bytes, &initial, code.clone(), Default::default())
            .unwrap();
    assert_eq!(loaded.world().unwrap().localization().settings(), &locale);
    let physical = initial
        .with_physics(vec![CookedPhysics {
            entity: id(1),
            body: Some(Default::default()),
            collider: Default::default(),
            angular_velocity: None,
        }])
        .unwrap();
    assert!(matches!(
        NativeSession::from_checkpoint_bytes(&bytes, &physical, code, Default::default()),
        Err(CheckpointError::World(SaveError::PhysicsUnsupported))
    ));
}
