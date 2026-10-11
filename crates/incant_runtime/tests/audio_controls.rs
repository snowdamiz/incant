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
