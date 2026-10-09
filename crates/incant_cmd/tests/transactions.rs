use incant_cmd::*;
use incant_doc::*;
use serde_json::json;
fn setup() -> (CommandBus, String, String) {
    let mut p = Project::empty("test");
    let mut s = Scene::new("world");
    let e = Entity::new("player");
    let ids = (s.id.clone(), e.id.clone());
    s.entities.insert(e.id.clone(), e);
    p.scenes.insert(s.id.clone(), s);
    (CommandBus::new(p).unwrap(), ids.0, ids.1)
}
#[test]
fn transaction_failure_rolls_back_all_commands() {
    let (mut b, s, e) = setup();
    let before = b.project().clone();
    assert!(
        b.execute(
            vec![
                Command::RenameEntity {
                    scene_id: s.clone(),
                    entity_id: e.clone(),
                    name: "changed".into()
                },
                Command::SetComponent {
                    scene_id: s,
                    entity_id: e,
                    component: "Transform".into(),
                    value: json!({"bad":1})
                }
            ],
            Actor::user("test"),
            "bad",
            None
        )
        .is_err()
    );
    assert_eq!(b.project(), &before);
    assert!(b.history().is_empty());
    assert_eq!(b.revision(), 0);
}
#[test]
fn stale_edits_and_spoofed_actor_rejected() {
    let (mut b, _, _) = setup();
    assert!(matches!(
        b.execute(vec![], Actor::user("test"), "", Some(1)),
        Err(CommandError::Conflict { .. })
    ));
    assert!(
        b.execute(
            vec![Command::SetMemory {
                section: "a".into(),
                text: "b".into()
            }],
            Actor::agent("", ""),
            "",
            None
        )
        .is_err()
    );
}
#[test]
fn ten_thousand_random_undo_sequences() {
    let mut seed = 47u64;
    for i in 0..10_000 {
        let (mut bus, scene_id, entity_id) = setup();
        let initial = bus.project().clone();
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let length = (seed % 5 + 1) as usize;
        let mut states = vec![initial.clone()];
        for step in 0..length {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let command = match seed % 3 {
                0 => Command::RenameEntity {
                    scene_id: scene_id.clone(),
                    entity_id: entity_id.clone(),
                    name: format!("entity-{i}-{step}"),
                },
                1 => Command::SetComponent {
                    scene_id: scene_id.clone(),
                    entity_id: entity_id.clone(),
                    component: "Transform".into(),
                    value: json!(Transform {
                        translation: [step as f64, 0., 0.],
                        ..Default::default()
                    }),
                },
                _ => Command::SetMemory {
                    section: "test".into(),
                    text: format!("{i}-{step}"),
                },
            };
            bus.execute(
                vec![command],
                Actor::agent("test-model", "test-conversation"),
                "fuzz",
                None,
            )
            .unwrap();
            states.push(bus.project().clone());
        }
        for state in states[..length].iter().rev() {
            bus.undo().unwrap();
            assert_eq!(bus.project(), state);
        }
        for state in &states[1..] {
            bus.redo().unwrap();
            assert_eq!(bus.project(), state);
        }
        for _ in 0..length {
            bus.undo().unwrap();
        }
        assert_eq!(bus.project(), &initial);
    }
}
#[test]
fn new_edit_clears_redo() {
    let (mut b, _, _) = setup();
    for _ in 0..2 {
        b.execute(
            vec![Command::SetMemory {
                section: "x".into(),
                text: "y".into(),
            }],
            Actor::user("test"),
            "memory",
            None,
        )
        .unwrap();
    }
    b.undo().unwrap();
    assert!(b.can_redo());
    b.execute(
        vec![Command::SetMemory {
            section: "x".into(),
            text: "z".into(),
        }],
        Actor::user("test"),
        "memory",
        None,
    )
    .unwrap();
    assert!(!b.can_redo());
}
#[test]
fn journal_replays_undo_redo_and_recovers_partial_tail() {
    let p = Project::empty("journal");
    let path = std::env::temp_dir().join(format!("incant-{}.jsonl", new_id()));
    let expected;
    {
        let mut b = CommandBus::persistent(&path, p.clone()).unwrap();
        b.execute(
            vec![Command::SetMemory {
                section: "a".into(),
                text: "first".into(),
            }],
            Actor::user("test"),
            "one",
            None,
        )
        .unwrap();
        b.execute(
            vec![Command::SetMemory {
                section: "a".into(),
                text: "second".into(),
            }],
            Actor::user("test"),
            "two",
            None,
        )
        .unwrap();
        b.undo().unwrap();
        expected = b.project().clone();
    }
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap();
    f.write_all(b"{\"partial\"").unwrap();
    drop(f);
    {
        let mut b = CommandBus::persistent(&path, p.clone()).unwrap();
        assert_eq!(b.project(), &expected);
        assert!(b.can_redo());
        b.redo().unwrap();
        assert_eq!(b.project().memory["a"], "second");
    }
    {
        let b = CommandBus::persistent(&path, p).unwrap();
        assert_eq!(b.project().memory["a"], "second");
    }
    std::fs::remove_file(path).unwrap();
}
#[test]
fn journal_corruption_and_concurrent_writer_rejected() {
    let p = Project::empty("test");
    let path = std::env::temp_dir().join(format!("incant-{}.jsonl", new_id()));
    let b = CommandBus::persistent(&path, p.clone()).unwrap();
    assert!(CommandBus::persistent(&path, p.clone()).is_err());
    drop(b);
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, text.replace("test", "evil")).unwrap();
    assert!(CommandBus::persistent(&path, p).is_err());
    std::fs::remove_file(path).unwrap();
}

#[test]
fn saved_checkpoint_preserves_crdt_ancestry_after_restart() {
    let initial = Project::empty("causal recovery");
    let path = std::env::temp_dir().join(format!("incant-{}.jsonl", new_id()));
    let mut peer;
    let saved;
    {
        let mut bus = CommandBus::persistent(&path, initial.clone()).unwrap();
        peer = CollaborativeDocument::from_export(&bus.export_crdt().unwrap()).unwrap();
        bus.execute(
            vec![Command::SetMemory {
                section: "local".into(),
                text: "durable".into(),
            }],
            Actor::user("local"),
            "local edit",
            None,
        )
        .unwrap();
        saved = bus.project().clone();
    }
    let mut remote = peer.project().unwrap();
    remote.memory.insert("remote".into(), "concurrent".into());
    peer.replace(&remote).unwrap();
    {
        let mut bus = CommandBus::persistent(&path, saved).unwrap();
        bus.merge(&peer.export().unwrap(), Actor::user("remote"))
            .unwrap();
        peer.merge(&bus.export_crdt().unwrap()).unwrap();
        assert_eq!(bus.project(), &peer.project().unwrap());
        assert_eq!(bus.project().memory["local"], "durable");
        assert_eq!(bus.project().memory["remote"], "concurrent");
        bus.undo().unwrap();
        assert!(!bus.project().memory.contains_key("remote"));
        bus.redo().unwrap();
    }
    let bus = CommandBus::persistent(&path, initial).unwrap();
    assert_eq!(bus.project(), &peer.project().unwrap());
    drop(bus);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn simulation_commands_cannot_modify_or_merge_into_authoring_state() {
    let (authoring, scene_id, entity_id) = setup();
    let original = authoring.project().clone();
    let mut play = CommandBus::simulation(original.clone()).unwrap();
    play.execute(
        vec![Command::RenameEntity {
            scene_id,
            entity_id,
            name: "runtime".into(),
        }],
        Actor::user("play"),
        "runtime edit",
        None,
    )
    .unwrap();
    assert_ne!(play.project(), &original);
    assert_eq!(authoring.project(), &original);
    assert!(play.export_crdt().is_err());
    assert!(
        play.merge(&authoring.export_crdt().unwrap(), Actor::user("remote"))
            .is_err()
    );
}
