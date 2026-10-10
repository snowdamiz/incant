use incant_runtime::{
    CookedEntity, CookedScene, FrameCommands, NativeWorld, RuntimeError, SaveError, SceneError,
    StableId, StructuralCommand as Command,
};
use incant_types::{Transform, Velocity};
use sha2::{Digest, Sha256};

fn id(value: u128) -> StableId {
    StableId(value.to_be_bytes())
}
fn entity(value: u128, parent: Option<u128>, moving: bool) -> CookedEntity {
    CookedEntity {
        id: id(value),
        parent: parent.map(id),
        transform: Transform {
            translation: [value as f64, -0., 0.],
            ..Default::default()
        },
        velocity: moving.then_some(Velocity {
            linear: [3., -2., 1.],
        }),
    }
}
fn world(rate: u32) -> NativeWorld {
    NativeWorld::new(
        CookedScene::new(
            id(99),
            rate,
            vec![entity(1, None, true), entity(2, Some(1), false)],
        )
        .unwrap(),
    )
}
fn rehash(bytes: &mut [u8], prefix: usize, header: usize) {
    let mut hash = Sha256::new();
    hash.update(&bytes[..prefix]);
    hash.update(&bytes[header..]);
    bytes[prefix..header].copy_from_slice(&hash.finalize());
}
fn advance(world: &mut NativeWorld, count: u64) {
    for _ in 0..count {
        world.step().unwrap();
    }
}

#[test]
fn save_round_trip_resumes_exactly_at_the_saved_clock_and_tick_rate() {
    for rate in [1, 60, 120, 240] {
        let mut uninterrupted = world(rate);
        advance(&mut uninterrupted, 25);
        let bytes = uninterrupted.save_bytes().unwrap();
        let before = uninterrupted.snapshot();
        let mut restored = NativeWorld::from_save_bytes(&bytes).unwrap();
        assert_eq!(restored.tick(), 25);
        assert_eq!(restored.tick_rate(), rate);
        assert_eq!(restored.scene_id(), id(99));
        assert_eq!(restored.snapshot(), before);
        assert_eq!(restored.save_bytes().unwrap(), bytes);
        assert_eq!(
            restored.inspect(id(2)).unwrap().transform.translation[1].to_bits(),
            (-0_f64).to_bits()
        );
        for _ in 0..35 {
            uninterrupted.step().unwrap();
            restored.step().unwrap();
            assert_eq!(restored.snapshot(), uninterrupted.snapshot());
        }
        assert_eq!(restored.tick(), 60);
        assert_eq!(
            restored.save_bytes().unwrap(),
            uninterrupted.save_bytes().unwrap()
        );
    }
}

#[test]
fn saves_include_committed_structural_and_direct_column_changes() {
    let mut original = world(60);
    advance(&mut original, 3);
    let mut commands = FrameCommands::default();
    for command in [
        Command::Spawn(entity(3, None, true)),
        Command::SetParent {
            entity: id(2),
            parent: Some(id(3)),
        },
        Command::Despawn(id(1)),
        Command::SetVelocity {
            entity: id(2),
            velocity: Some(Velocity {
                linear: [0.5, 0., 0.],
            }),
        },
    ] {
        commands.push(command).unwrap();
    }
    original.apply(&mut commands).unwrap();
    original.with_numeric_columns(|columns| {
        columns.for_each_chunk(|chunk| {
            for transform in chunk.transforms.as_chunks_mut::<10>().0 {
                transform[0] = 64.;
            }
        });
    });
    let bytes = original.save_bytes().unwrap();
    let mut restored = NativeWorld::from_save_bytes(&bytes).unwrap();
    assert_eq!(restored.snapshot(), original.snapshot());
    assert!(restored.inspect(id(1)).is_none());
    assert_eq!(restored.inspect(id(2)).unwrap().parent, Some(id(3)));
    assert_eq!(restored.inspect(id(2)).unwrap().world_transform[3][0], 128.);
    advance(&mut original, 57);
    advance(&mut restored, 57);
    assert_eq!(
        restored.save_bytes().unwrap(),
        original.save_bytes().unwrap()
    );
}

#[test]
fn save_order_does_not_depend_on_archetype_insertion_history() {
    let initial = world(120);
    let mut reordered = NativeWorld::new(CookedScene::new(id(99), 120, vec![]).unwrap());
    let mut commands = FrameCommands::default();
    commands
        .push(Command::Spawn(entity(2, Some(1), false)))
        .unwrap();
    commands
        .push(Command::Spawn(entity(1, None, true)))
        .unwrap();
    reordered.apply(&mut commands).unwrap();
    assert_eq!(
        initial.save_bytes().unwrap(),
        reordered.save_bytes().unwrap()
    );
}

#[test]
fn every_truncation_and_single_byte_mutation_leaves_the_destination_intact() {
    let mut target = world(60);
    advance(&mut target, 7);
    let before = target.save_bytes().unwrap();
    let source = world(60).save_bytes().unwrap();
    for end in 0..source.len() {
        assert!(
            target.restore_bytes(&source[..end]).is_err(),
            "prefix {end}"
        );
    }
    for index in 0..source.len() {
        let mut damaged = source.clone();
        damaged[index] ^= 1;
        assert!(target.restore_bytes(&damaged).is_err(), "byte {index}");
    }
    let mut extra = source;
    extra.push(0);
    assert!(target.restore_bytes(&extra).is_err());
    assert_eq!(target.save_bytes().unwrap(), before);
    target.step().unwrap();
    assert_eq!(target.tick(), 8);
}

#[test]
fn unknown_version_flags_and_rehashed_malformed_payloads_fail_closed() {
    let source = world(60).save_bytes().unwrap();
    for (offset, value) in [(8, 2_u32), (12, 1), (24, u32::MAX)] {
        let mut bad = source.clone();
        bad[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        rehash(&mut bad, 32, 64);
        assert!(NativeWorld::from_save_bytes(&bad).is_err());
    }
    // Recompute both hashes, so semantic checks, not merely checksums, reject
    // an unsupported component, bad parent, duplicate ID and non-finite value.
    for case in 0..5 {
        let mut bad = source.clone();
        let scene = &mut bad[64..];
        match case {
            0 => scene[92..96].copy_from_slice(&7_u32.to_le_bytes()),
            1 => scene[88..92].copy_from_slice(&100_u32.to_le_bytes()),
            2 => scene[72..88].copy_from_slice(&id(99).0),
            3 => scene[96..104].copy_from_slice(&f64::NAN.to_le_bytes()),
            _ => scene[8..12].copy_from_slice(&2_u32.to_le_bytes()),
        }
        rehash(scene, 40, 72);
        rehash(&mut bad, 32, 64);
        assert!(
            matches!(NativeWorld::from_save_bytes(&bad), Err(SaveError::Scene(_))),
            "case {case}"
        );
    }
}

#[test]
fn different_world_and_tick_rate_do_not_replace_an_existing_session() {
    let mut target = world(60);
    advance(&mut target, 10);
    let before = target.save_bytes().unwrap();
    let other = NativeWorld::new(CookedScene::new(id(100), 60, vec![]).unwrap());
    for bytes in [
        other.save_bytes().unwrap(),
        world(120).save_bytes().unwrap(),
    ] {
        assert_eq!(target.restore_bytes(&bytes), Err(SaveError::WorldMismatch));
        assert_eq!(target.save_bytes().unwrap(), before);
    }
    target
        .restore_bytes(&world(60).save_bytes().unwrap())
        .unwrap();
    assert_eq!(target.tick(), 0);
    target.step().unwrap();
    assert_eq!(target.tick(), 1);
}

#[test]
fn invalid_live_values_refuse_to_produce_a_save_without_mutation() {
    let mut target = world(60);
    target.for_each_moving_mut(|_, t, _| t.scale[0] = 0.);
    let before = target.snapshot();
    assert!(matches!(
        target.save_bytes(),
        Err(SaveError::Scene(SceneError::Component(_)))
    ));
    assert_eq!(target.snapshot(), before);
    assert_eq!(target.tick(), 0);
}

#[test]
fn saved_clock_overflow_cannot_partially_step_the_restored_world() {
    let mut bytes = world(60).save_bytes().unwrap();
    bytes[16..24].copy_from_slice(&u64::MAX.to_le_bytes());
    rehash(&mut bytes, 32, 64);
    let mut restored = NativeWorld::from_save_bytes(&bytes).unwrap();
    let before = restored.snapshot();
    assert_eq!(restored.step(), Err(RuntimeError::TickOverflow));
    assert_eq!(restored.snapshot(), before);
    assert_eq!(restored.save_bytes().unwrap(), bytes);
}

#[test]
fn scene_and_legacy_json_are_not_silently_accepted_as_native_saves() {
    let scene = CookedScene::new(id(99), 60, vec![]).unwrap().to_bytes();
    assert!(NativeWorld::from_save_bytes(&scene).is_err());
    assert!(NativeWorld::from_save_bytes(br#"{"version":1,"tick":0,"entities":[]}"#).is_err());
    let mut empty = NativeWorld::new(CookedScene::new(id(100), 60, vec![]).unwrap());
    empty.step().unwrap();
    let mut restored = NativeWorld::from_save_bytes(&empty.save_bytes().unwrap()).unwrap();
    assert_eq!(restored.entity_count(), 0);
    restored.step().unwrap();
    assert_eq!(restored.tick(), 2);
}
