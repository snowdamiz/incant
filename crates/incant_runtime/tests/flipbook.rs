use incant_runtime::*;
use incant_types::{Transform, Velocity};
use std::sync::Arc;
fn id(n: u8) -> StableId {
    StableId([n; 16])
}
fn scene() -> CookedScene {
    let sprite = NativeSpriteRenderer {
        texture: id(3),
        region: NativeSpriteRegion::Pixels([0, 0, 2, 2]),
        size: [1.; 2],
        pivot: [0.5; 2],
        tint: [1.; 4],
        flip_x: false,
        flip_y: false,
        layer: 0,
        order: 0,
        sampling: SpriteSampling::Nearest,
        mode: SpriteMode::Simple,
    };
    let mut node = CookedSpatial::empty(id(1));
    node.sprite = Some(Box::new(sprite));
    node.flipbook = Some(Arc::new(NativeFlipbook {
        frames: vec![
            NativeSpriteRegion::Pixels([0, 0, 2, 2]),
            NativeSpriteRegion::Pixels([2, 0, 2, 2]),
            NativeSpriteRegion::Pixels([4, 0, 2, 2]),
        ]
        .into(),
        rate: FrameRate {
            numerator: 12,
            denominator: 1,
        },
        looping: true,
        playing: true,
    }));
    CookedScene::new(
        id(0),
        60,
        vec![CookedEntity {
            id: id(1),
            parent: None,
            transform: Transform::default(),
            velocity: Some(Velocity {
                linear: [1., 0., 0.],
            }),
        }],
    )
    .unwrap()
    .with_spatial(
        vec![VisualAssetRef {
            id: id(3),
            kind: VisualAssetKind::Texture,
            fingerprint: [4; 32],
        }],
        vec![node],
    )
    .unwrap()
}
fn control(world: &mut NativeWorld, control: FlipbookControl) {
    let mut commands = FrameCommands::default();
    commands
        .push(StructuralCommand::ControlFlipbook {
            entity: id(1),
            control,
        })
        .unwrap();
    world.apply(&mut commands).unwrap();
}
#[test]
fn v8_round_trip_advances_only_on_successful_fixed_ticks_with_retained_binding() {
    let cooked = scene();
    let bytes = cooked.to_bytes();
    assert_eq!(&bytes[8..12], &8u32.to_le_bytes());
    assert_eq!(CookedScene::from_bytes(&bytes).unwrap().to_bytes(), bytes);
    let mut world = NativeWorld::from_bytes(&bytes).unwrap();
    let ptr = Arc::as_ptr(
        world
            .spatial_view()
            .unwrap()
            .nodes()
            .next()
            .unwrap()
            .declaration
            .unwrap()
            .flipbook
            .as_ref()
            .unwrap(),
    );
    for tick in 1..=100 {
        world.step().unwrap();
        assert_eq!(
            world.flipbook_state(id(1)).unwrap().frame(),
            ((tick / 5) % 3) as u16
        );
        let view = world.spatial_view().unwrap();
        assert_eq!(view.binding_generation(), 0);
        assert_eq!(view.topology_generation(), 0);
        let node = view.nodes().next().unwrap();
        assert_eq!(node.flipbook.unwrap(), world.flipbook_state(id(1)).unwrap());
        assert_eq!(
            Arc::as_ptr(node.declaration.unwrap().flipbook.as_ref().unwrap()),
            ptr
        );
    }
    assert!(world.save_bytes().is_ok());
}
#[test]
fn paused_phase_seek_restart_and_noop_reinstall_are_observable() {
    let mut world = NativeWorld::new(scene());
    for _ in 0..4 {
        world.step().unwrap();
    }
    control(&mut world, FlipbookControl::Pause);
    let paused = world.flipbook_state(id(1));
    for _ in 0..5 {
        world.step().unwrap();
    }
    assert_eq!(world.flipbook_state(id(1)), paused);
    let same = world
        .spatial_view()
        .unwrap()
        .nodes()
        .next()
        .unwrap()
        .declaration
        .unwrap()
        .clone();
    let mut commands = FrameCommands::default();
    commands
        .push(StructuralCommand::SetSpatial {
            entity: id(1),
            spatial: Some(same),
        })
        .unwrap();
    world.apply(&mut commands).unwrap();
    control(&mut world, FlipbookControl::Play);
    world.step().unwrap();
    assert_eq!(world.flipbook_state(id(1)).unwrap().frame(), 1);
    control(&mut world, FlipbookControl::Seek(2));
    world.step().unwrap();
    assert_eq!(world.flipbook_state(id(1)).unwrap().frame(), 2);
    control(&mut world, FlipbookControl::Restart);
    assert_eq!(world.flipbook_state(id(1)).unwrap().frame(), 0);
}
#[test]
fn invalid_control_and_mixed_final_configuration_preserve_whole_batch_and_cursor() {
    let mut world = NativeWorld::new(scene());
    world.step().unwrap();
    let before = world.snapshot();
    let state = world.flipbook_state(id(1));
    let mut commands = FrameCommands::default();
    commands
        .push(StructuralCommand::SetVelocity {
            entity: id(1),
            velocity: Some(Velocity {
                linear: [7., 0., 0.],
            }),
        })
        .unwrap();
    commands
        .push(StructuralCommand::ControlFlipbook {
            entity: id(1),
            control: FlipbookControl::Seek(3),
        })
        .unwrap();
    assert!(world.apply(&mut commands).is_err());
    assert_eq!(commands.len(), 2);
    assert_eq!(world.snapshot(), before);
    assert_eq!(world.flipbook_state(id(1)), state);
    commands.clear();
    commands
        .push(StructuralCommand::ControlFlipbook {
            entity: id(1),
            control: FlipbookControl::Pause,
        })
        .unwrap();
    commands
        .push(StructuralCommand::SetSpatial {
            entity: id(1),
            spatial: None,
        })
        .unwrap();
    assert!(world.apply(&mut commands).is_err());
    assert_eq!(world.flipbook_state(id(1)), state);
}
#[test]
fn decoder_checks_new_record_branches_after_integrity_and_static_v5_stays_static() {
    use sha2::{Digest, Sha256};
    let bytes = scene().to_bytes();
    let record = bytes.len() - (29 + 3 * 17);
    let corrupt = |offset: usize, value: u8| {
        let mut b = bytes.clone();
        b[offset] = value;
        let mut hash = Sha256::new();
        hash.update(&b[..40]);
        hash.update(&b[72..]);
        b[40..72].copy_from_slice(&hash.finalize());
        assert!(CookedScene::from_bytes(&b).is_err());
    };
    corrupt(record - 4, 0);
    corrupt(record + 16, 0);
    corrupt(record + 20, 0);
    corrupt(record + 24, 4);
    corrupt(record + 25, 0);
    corrupt(record + 29, 2);
    corrupt(record, 9);
    // Refresh both envelope size and checksum so every truncation reaches the
    // version-8 record reader, rather than just the outer integrity guard.
    for removed in 1..46 {
        let mut truncated = bytes[..bytes.len() - removed].to_vec();
        let length = (truncated.len() - 72) as u32;
        truncated[36..40].copy_from_slice(&length.to_le_bytes());
        let mut hash = Sha256::new();
        hash.update(&truncated[..40]);
        hash.update(&truncated[72..]);
        truncated[40..72].copy_from_slice(&hash.finalize());
        assert!(CookedScene::from_bytes(&truncated).is_err());
    }
    let mut nodes = scene().spatial().to_vec();
    nodes[0].flipbook = None;
    let static_scene = scene()
        .with_spatial(scene().visual_assets().to_vec(), nodes)
        .unwrap();
    assert_eq!(&static_scene.to_bytes()[8..12], &5u32.to_le_bytes());
    let mut world = NativeWorld::new(static_scene);
    world.step().unwrap();
    assert_eq!(world.flipbook_state(id(1)), None);
}

#[test]
fn rejected_step_does_not_consume_phase_and_replacement_controls_use_final_configuration() {
    let base = scene();
    let mut entities = base.entities().to_vec();
    entities[0].transform.translation[0] = 1.79e308;
    entities[0].velocity = Some(Velocity {
        linear: [1.79e308, 0., 0.],
    });
    let cooked = CookedScene::new(id(0), 60, entities)
        .unwrap()
        .with_spatial(base.visual_assets().to_vec(), base.spatial().to_vec())
        .unwrap();
    let mut world = NativeWorld::new(cooked);
    let before = world.snapshot();
    let state = world.flipbook_state(id(1));
    assert!(world.step().is_err());
    assert_eq!(world.snapshot(), before);
    assert_eq!(world.flipbook_state(id(1)), state);
    let mut replacement = base.spatial()[0].clone();
    let animation = Arc::make_mut(replacement.flipbook.as_mut().unwrap());
    animation.frames = animation.frames[..2].into();
    animation.looping = false;
    animation.playing = false;
    let mut commands = FrameCommands::default();
    // Controls target the final declaration, regardless of their placement in the batch.
    for command in [
        StructuralCommand::ControlFlipbook {
            entity: id(1),
            control: FlipbookControl::Seek(1),
        },
        StructuralCommand::SetSpatial {
            entity: id(1),
            spatial: Some(replacement),
        },
        StructuralCommand::SetVelocity {
            entity: id(1),
            velocity: None,
        },
        StructuralCommand::ControlFlipbook {
            entity: id(1),
            control: FlipbookControl::Play,
        },
    ] {
        commands.push(command).unwrap();
    }
    world.apply(&mut commands).unwrap();
    assert_eq!(world.flipbook_state(id(1)).unwrap().frame(), 1);
    for _ in 0..4 {
        world.step().unwrap();
        assert!(!world.flipbook_state(id(1)).unwrap().completed());
    }
    world.step().unwrap();
    assert!(world.flipbook_state(id(1)).unwrap().completed());
    assert!(!world.flipbook_state(id(1)).unwrap().playing());
    commands.push(StructuralCommand::Despawn(id(1))).unwrap();
    world.apply(&mut commands).unwrap();
    assert!(world.flipbook_state(id(1)).is_none());
    commands
        .push(StructuralCommand::Spawn(base.entities()[0].clone()))
        .unwrap();
    commands
        .push(StructuralCommand::SetSpatial {
            entity: id(1),
            spatial: Some(base.spatial()[0].clone()),
        })
        .unwrap();
    commands
        .push(StructuralCommand::ControlFlipbook {
            entity: id(1),
            control: FlipbookControl::Seek(2),
        })
        .unwrap();
    world.apply(&mut commands).unwrap();
    assert_eq!(world.flipbook_state(id(1)).unwrap().frame(), 2);
}

#[test]
fn scene_total_frame_budget_is_enforced_before_installation() {
    let base = scene();
    let mut animation = base.spatial()[0]
        .flipbook
        .as_ref()
        .unwrap()
        .as_ref()
        .clone();
    animation.frames = vec![NativeSpriteRegion::Pixels([0, 0, 1, 1]); MAX_FLIPBOOK_FRAMES].into();
    let animation = Arc::new(animation);
    let count = MAX_SCENE_FLIPBOOK_FRAMES / MAX_FLIPBOOK_FRAMES + 1;
    let mut entities = Vec::new();
    let mut nodes = Vec::new();
    for i in 0..count {
        let id = StableId((i as u128 + 1).to_be_bytes());
        let mut entity = base.entities()[0].clone();
        entity.id = id;
        let mut node = base.spatial()[0].clone();
        node.entity = id;
        node.flipbook = Some(Arc::clone(&animation));
        entities.push(entity);
        nodes.push(node);
    }
    let scene = CookedScene::new(id(0), 60, entities).unwrap();
    assert!(
        scene
            .with_spatial(base.visual_assets().to_vec(), nodes)
            .is_err()
    );
}
