#[path = "support/flipbooks.rs"]
mod fixture;
use fixture::*;
use incant_runtime::*;
use incant_types::{Transform, Velocity};
#[test]
fn retained_query_tracks_new_archetypes_growth_removal_and_owned_snapshots() {
    let initial = scene(2);
    let mut world = NativeWorld::new(initial.clone());
    let mut rows = Vec::new();
    world.snapshot_flipbooks(&mut rows).unwrap();
    let old = rows.clone();
    assert_eq!(rows.len(), 2);
    for _ in 0..12 {
        world.step().unwrap();
    }
    world.snapshot_flipbooks(&mut rows).unwrap();
    rows.sort_by_key(|row| row.id);
    assert_eq!(
        rows[0],
        FlipbookSnapshot {
            id: id(1),
            frame: 0,
            playing: true,
            completed: false
        }
    );
    assert_eq!(
        rows[1],
        FlipbookSnapshot {
            id: id(2),
            frame: 2,
            playing: false,
            completed: true
        }
    );
    assert!(
        old.iter()
            .all(|row| row.frame == 0 && row.playing && !row.completed)
    );
    let mut commands = FrameCommands::default();
    commands.push(StructuralCommand::Despawn(id(2))).unwrap();
    commands
        .push(StructuralCommand::SetVelocity {
            entity: id(1),
            velocity: Some(Velocity {
                linear: [1., 0., 0.],
            }),
        })
        .unwrap();
    for n in 3..=1026 {
        commands
            .push(StructuralCommand::Spawn(CookedEntity {
                id: id(n),
                parent: None,
                transform: Transform::default(),
                velocity: None,
            }))
            .unwrap();
        let mut node = initial.spatial()[0].clone();
        node.entity = id(n);
        commands
            .push(StructuralCommand::SetSpatial {
                entity: id(n),
                spatial: Some(node),
            })
            .unwrap();
    }
    world.apply(&mut commands).unwrap();
    world.snapshot_flipbooks(&mut rows).unwrap();
    assert_eq!(rows.len(), 1025);
    assert!(!rows.iter().any(|r| r.id == id(2)));
    assert!(rows.iter().any(|r| r.id == id(1026)));
    let capacity = rows.capacity();
    let pointer = rows.as_ptr();
    for _ in 0..8 {
        world.step().unwrap();
        world.snapshot_flipbooks(&mut rows).unwrap();
    }
    assert_eq!(rows.capacity(), capacity);
    assert_eq!(rows.as_ptr(), pointer);
    assert!(rows.iter().all(|r| r.frame == 2));
    commands
        .push(StructuralCommand::SetSpatial {
            entity: id(1),
            spatial: None,
        })
        .unwrap();
    world.apply(&mut commands).unwrap();
    world.snapshot_flipbooks(&mut rows).unwrap();
    assert!(!rows.iter().any(|r| r.id == id(1)));
}
#[test]
fn empty_world_clears_reused_scratch() {
    let mut rows = vec![FlipbookSnapshot {
        id: id(99),
        frame: 99,
        playing: true,
        completed: true,
    }];
    let mut world = NativeWorld::new(scene(0));
    world.snapshot_flipbooks(&mut rows).unwrap();
    assert!(rows.is_empty());
}

#[test]
fn mixed_motion_and_playback_commands_validate_every_row_before_publication() {
    let mut world = NativeWorld::new(scene(2));
    let mut commands = FrameCommands::default();
    let pause = StructuralCommand::ControlFlipbook {
        entity: id(1),
        control: FlipbookControl::Pause,
    };
    let velocity = StructuralCommand::SetVelocity {
        entity: id(1),
        velocity: Some(Velocity {
            linear: [4., 0., 0.],
        }),
    };
    for invalid in [
        StructuralCommand::ControlFlipbook {
            entity: id(2),
            control: FlipbookControl::Seek(3),
        },
        StructuralCommand::SetAngularVelocity {
            entity: id(2),
            velocity: None,
        },
    ] {
        commands.push(pause.clone()).unwrap();
        commands.push(velocity.clone()).unwrap();
        commands.push(invalid).unwrap();
        let before = world.snapshot();
        assert!(world.apply(&mut commands).is_err());
        assert_eq!(world.snapshot(), before);
        assert!(world.flipbook_state(id(1)).unwrap().playing());
        assert_eq!(commands.len(), 3);
        commands.clear();
    }
    commands.push(pause).unwrap();
    commands.push(velocity).unwrap();
    commands
        .push(StructuralCommand::ControlFlipbook {
            entity: id(1),
            control: FlipbookControl::Seek(2),
        })
        .unwrap();
    world.apply(&mut commands).unwrap();
    assert!(commands.is_empty());
    for _ in 0..4 {
        world.step().unwrap();
    }
    let state = world.flipbook_state(id(1)).unwrap();
    assert_eq!(state.frame(), 2);
    assert!(!state.playing());
    assert_eq!(world.snapshot()[0].transform.translation, [0.25, 0., 0.]);
}
