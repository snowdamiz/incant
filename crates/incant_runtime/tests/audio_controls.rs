#[path = "support/audio_controls.rs"]
mod fixture;
use fixture::*;
use incant_runtime::*;
use incant_types::{Transform, Velocity};
fn control(n: u128, control: AudioControl) -> StructuralCommand {
    StructuralCommand::ControlAudio {
        entity: id(n),
        control,
    }
}
fn queue(items: Vec<StructuralCommand>) -> FrameCommands {
    let mut q = FrameCommands::default();
    for item in items {
        q.push(item).unwrap();
    }
    q
}
#[test]
fn typed_controls_preserve_declarations_and_compose_in_order_with_numeric_commands() {
    let mut world = NativeWorld::new(scene(false, true, true));
    let before = audio(&world);
    let mut q = queue(vec![
        control(2, AudioControl::SetPlaying(false)),
        control(2, AudioControl::SourceGain(-6.)),
        control(2, AudioControl::SourceGain(-12.)),
        control(1, AudioControl::BusGain(-3.)),
    ]);
    world.apply(&mut q).unwrap();
    assert!(q.is_empty());
    let mut expected = before;
    expected[0].bus.as_mut().unwrap().gain_db = -3.;
    expected[1].source.as_mut().unwrap().playing = false;
    expected[1].source.as_mut().unwrap().gain_db = -12.;
    assert_eq!(audio(&world), expected);
    let mut q = queue(vec![
        control(2, AudioControl::SetPlaying(true)),
        StructuralCommand::SetVelocity {
            entity: id(3),
            velocity: Some(Velocity {
                linear: [2., 0., 0.],
            }),
        },
    ]);
    world.apply(&mut q).unwrap();
    expected[1].source.as_mut().unwrap().playing = true;
    assert_eq!(audio(&world), expected);
    assert_eq!(
        world.inspect(id(3)).unwrap().velocity.unwrap().linear,
        [2., 0., 0.]
    );
}
#[test]
fn invalid_controls_refuse_every_change_and_retain_the_submitted_queue() {
    for bad in [
        control(2, AudioControl::Seek(f64::NAN)),
        control(2, AudioControl::Seek(f64::INFINITY)),
        control(2, AudioControl::Seek(-0.001)),
        control(2, AudioControl::Seek(3600.000001)),
        control(1, AudioControl::Stop),
        control(3, AudioControl::Restart),
        control(2, AudioControl::SourceGain(f32::NAN)),
        control(1, AudioControl::BusGain(f32::INFINITY)),
        control(2, AudioControl::SourceGain(25.)),
        control(1, AudioControl::SetPlaying(false)),
        control(2, AudioControl::BusGain(-3.)),
        control(3, AudioControl::SetPlaying(true)),
        control(88, AudioControl::SourceGain(0.)),
    ] {
        let mut world = NativeWorld::new(scene(false, true, true));
        let before = audio(&world);
        let snapshot = world.snapshot();
        let mut q = queue(vec![
            control(2, AudioControl::SetPlaying(false)),
            control(1, AudioControl::BusGain(-20.)),
            StructuralCommand::SetVelocity {
                entity: id(3),
                velocity: Some(Velocity {
                    linear: [9., 0., 0.],
                }),
            },
            bad,
        ]);
        assert!(world.apply(&mut q).is_err());
        assert_eq!(q.len(), 4);
        assert_eq!(audio(&world), before);
        assert_eq!(world.snapshot(), snapshot);
    }
}

#[test]
fn transport_requests_are_ordered_repeatable_and_survive_world_save() {
    let mut world = NativeWorld::new(scene(false, true, true));
    for (controls, epoch, seconds, playing) in [
        (
            vec![
                AudioControl::Stop,
                AudioControl::Restart,
                AudioControl::Seek(0.5),
            ],
            3,
            0.5,
            true,
        ),
        (
            vec![AudioControl::SetPlaying(false), AudioControl::Seek(0.5)],
            4,
            0.5,
            false,
        ),
        (
            vec![AudioControl::Restart, AudioControl::Stop],
            6,
            0.,
            false,
        ),
        (
            vec![AudioControl::Restart, AudioControl::Restart],
            8,
            0.,
            true,
        ),
    ] {
        world
            .apply(&mut queue(
                controls.into_iter().map(|c| control(2, c)).collect(),
            ))
            .unwrap();
        let nodes = audio(&world);
        let source = nodes[1].source.as_ref().unwrap();
        assert_eq!(
            (source.transport_epoch, source.start_seconds, source.playing),
            (epoch, seconds, playing)
        );
        let bytes = world.save_bytes().unwrap();
        let restored = NativeWorld::from_save_bytes(&bytes).unwrap();
        assert_eq!(audio(&restored), nodes);
        assert_eq!(restored.save_bytes().unwrap(), bytes);
    }
}

#[test]
fn exhausted_transport_epoch_refuses_without_committing_numeric_or_audio_prefix() {
    let mut world = NativeWorld::new(scene(false, true, true));
    let mut node = source(false, true, true);
    node.source.as_mut().unwrap().transport_epoch = u64::MAX;
    world
        .apply(&mut queue(vec![StructuralCommand::SetAudio {
            entity: id(2),
            audio: Some(node),
        }]))
        .unwrap();
    let before = world.save_bytes().unwrap();
    let mut commands = queue(vec![
        control(1, AudioControl::BusGain(-9.)),
        control(2, AudioControl::Restart),
    ]);
    assert!(matches!(
        world.apply(&mut commands),
        Err(RuntimeError::GenerationOverflow)
    ));
    assert_eq!(commands.len(), 2);
    assert_eq!(world.save_bytes().unwrap(), before);
}
#[test]
fn controls_use_final_declarations_and_invalid_final_source_or_bus_is_atomic() {
    let mut world = NativeWorld::new(scene(false, true, true));
    let mut added = source(false, true, false);
    added.entity = id(4);
    let mut q = queue(vec![
        control(4, AudioControl::SourceGain(-9.)),
        StructuralCommand::Spawn(CookedEntity {
            id: id(4),
            parent: None,
            transform: Transform::default(),
            velocity: None,
        }),
        StructuralCommand::SetAudio {
            entity: id(4),
            audio: Some(added),
        },
    ]);
    world.apply(&mut q).unwrap();
    assert_eq!(audio(&world)[2].source.as_ref().unwrap().gain_db, -9.);
    for replacement in [
        None,
        Some(CookedAudio {
            entity: id(2),
            bus: Some(NativeAudioBus {
                parent: Some(id(88)),
                gain_db: 0.,
            }),
            source: None,
            listener: false,
        }),
    ] {
        let before = audio(&world);
        let snapshot = world.snapshot();
        let mut q = queue(vec![
            control(1, AudioControl::BusGain(-24.)),
            control(2, AudioControl::SetPlaying(false)),
            StructuralCommand::SetAudio {
                entity: id(2),
                audio: replacement,
            },
            StructuralCommand::SetParent {
                entity: id(3),
                parent: Some(id(1)),
            },
        ]);
        assert!(world.apply(&mut q).is_err());
        assert_eq!(audio(&world), before);
        assert_eq!(world.snapshot(), snapshot);
    }
    let mut invalid = source(false, true, true);
    invalid.source.as_mut().unwrap().bus = Some(id(88));
    let mut q = queue(vec![
        control(1, AudioControl::BusGain(-18.)),
        control(2, AudioControl::SourceGain(-12.)),
        StructuralCommand::SetAudio {
            entity: id(2),
            audio: Some(invalid),
        },
    ]);
    let before = audio(&world);
    assert!(world.apply(&mut q).is_err());
    assert_eq!(audio(&world), before);
}
