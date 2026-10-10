use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::{
    Asset, AudioBus, AudioListener, AudioSource, AudioSpatial, Entity, Project, Scene, Transform,
    new_id,
};
use serde_json::json;
fn actor() -> Actor {
    Actor::import("audio-test")
}
fn fixture() -> (CommandBus, String, String, String, String) {
    let mut project = Project::empty("Audio");
    let scene = Scene::new("Main");
    let sid = scene.id.clone();
    project.scenes.insert(sid.clone(), scene);
    let mut bus = CommandBus::new(project).unwrap();
    let asset = Asset {
        id: new_id(),
        name: "Tone".into(),
        path: "tone.wav".into(),
        kind: "audio".into(),
        sha256: "0".repeat(64),
        import_settings: None,
    };
    let aid = asset.id.clone();
    let mut output = Entity::new("SFX");
    let bid = output.id.clone();
    output.components.insert(
        "AudioBus".into(),
        json!(AudioBus {
            parent: None,
            gain_db: 0.
        }),
    );
    let mut listener = Entity::new("Listener");
    listener
        .components
        .insert("Transform".into(), json!(Transform::default()));
    listener
        .components
        .insert("AudioListener".into(), json!(AudioListener {}));
    let mut sound = Entity::new("Source");
    let eid = sound.id.clone();
    sound
        .components
        .insert("Transform".into(), json!(Transform::default()));
    sound.components.insert(
        "AudioSource".into(),
        json!(AudioSource {
            clip: aid.clone(),
            bus: Some(bid.clone()),
            gain_db: 0.,
            pan: 0.,
            rate: 1.,
            start_seconds: 0.,
            playing: true,
            looping: true,
            streaming: true,
            spatial: Some(AudioSpatial {
                min_distance: 1.,
                max_distance: 20.
            })
        }),
    );
    bus.execute(
        vec![
            Command::UpsertAsset { asset },
            Command::CreateEntity {
                scene_id: sid.clone(),
                entity: output,
            },
            Command::CreateEntity {
                scene_id: sid.clone(),
                entity: listener,
            },
            Command::CreateEntity {
                scene_id: sid.clone(),
                entity: sound,
            },
        ],
        actor(),
        "Audio graph",
        Some(0),
    )
    .unwrap();
    (bus, sid, eid, bid, aid)
}
#[test]
fn audio_graph_is_one_reversible_transaction_and_rejects_broken_references_atomically() {
    let (mut bus, sid, eid, bid, aid) = fixture();
    let valid = bus.project().canonical_text().unwrap();
    for command in [
        Command::RemoveAsset { asset_id: aid },
        Command::DeleteEntity {
            scene_id: sid.clone(),
            entity_id: bid.clone(),
        },
    ] {
        assert!(
            bus.execute(vec![command], actor(), "Invalid reference", None)
                .is_err()
        );
        assert_eq!(bus.project().canonical_text().unwrap(), valid);
    }
    for (field, value) in [
        ("clip", json!(new_id())),
        ("bus", json!(new_id())),
        ("rate", json!(0)),
        ("pan", json!(0.5)),
        ("spatial", json!({"min_distance":5,"max_distance":1})),
    ] {
        let mut source =
            bus.project().scenes[&sid].entities[&eid].components["AudioSource"].clone();
        source[field] = value;
        assert!(
            bus.execute(
                vec![Command::SetComponent {
                    scene_id: sid.clone(),
                    entity_id: eid.clone(),
                    component: "AudioSource".into(),
                    value: source
                }],
                actor(),
                "Invalid audio",
                None
            )
            .is_err()
        );
        assert_eq!(bus.project().canonical_text().unwrap(), valid);
    }
    assert!(
        bus.execute(
            vec![Command::SetComponent {
                scene_id: sid,
                entity_id: bid.clone(),
                component: "AudioBus".into(),
                value: json!({"parent":bid,"gain_db":0})
            }],
            actor(),
            "Invalid bus cycle",
            None
        )
        .is_err()
    );
    bus.undo().unwrap();
    assert!(bus.project().assets.is_empty());
    assert!(bus.project().scenes.values().all(|s| s.entities.is_empty()));
    bus.redo().unwrap();
    assert_eq!(bus.project().canonical_text().unwrap(), valid);
}
#[test]
fn spatial_sources_require_one_listener_and_a_transform() {
    let (bus, sid, eid, _, _) = fixture();
    let original = bus.project();
    let mut project = original.clone();
    project
        .scenes
        .get_mut(&sid)
        .unwrap()
        .entities
        .get_mut(&eid)
        .unwrap()
        .components
        .remove("Transform");
    assert!(project.validate().is_err());
    project = original.clone();
    project
        .scenes
        .get_mut(&sid)
        .unwrap()
        .entities
        .retain(|_, e| !e.components.contains_key("AudioListener"));
    assert!(project.validate().is_err());
    project = original.clone();
    let mut other = Entity::new("Other listener");
    other
        .components
        .insert("Transform".into(), json!(Transform::default()));
    other.components.insert("AudioListener".into(), json!({}));
    project
        .scenes
        .get_mut(&sid)
        .unwrap()
        .entities
        .insert(other.id.clone(), other);
    assert!(project.validate().is_err());
    for kind in ["AudioBus", "AudioSource", "AudioListener"] {
        assert!(incant_doc::schema_registry().contains_key(kind));
    }
}
