#[path = "support/scene_format.rs"]
mod scene_format;
use incant_runtime::{
    CookedAudio, CookedEntity, CookedScene, FrameCommands, NativeAudioBus, NativeAudioSource,
    NativeWorld, StableId, StructuralCommand,
};
use incant_types::{AudioSpatial, Transform};

fn id(value: u128) -> StableId {
    StableId(value.to_be_bytes())
}
fn scene() -> CookedScene {
    CookedScene::new(
        id(99),
        60,
        (1..=4)
            .map(|n| CookedEntity {
                id: id(n),
                parent: (n == 2).then(|| id(1)),
                transform: Transform {
                    translation: [n as f64, 0., 0.],
                    ..Default::default()
                },
                velocity: None,
            })
            .collect(),
    )
    .unwrap()
}
fn nodes() -> Vec<CookedAudio> {
    vec![
        CookedAudio {
            entity: id(1),
            bus: Some(NativeAudioBus {
                parent: None,
                gain_db: -6.,
            }),
            source: None,
            listener: false,
        },
        CookedAudio {
            entity: id(2),
            bus: None,
            source: Some(NativeAudioSource {
                transport_epoch: 0,
                clip: id(90),
                bus: Some(id(1)),
                gain_db: -3.,
                pan: 0.,
                rate: 1.5,
                start_seconds: 0.25,
                playing: true,
                looping: true,
                streaming: true,
                spatial: Some(AudioSpatial {
                    min_distance: 1.,
                    max_distance: 100.,
                }),
            }),
            listener: false,
        },
        CookedAudio {
            entity: id(3),
            bus: None,
            source: None,
            listener: true,
        },
    ]
}
#[test]
fn canonical_audio_roundtrip_retains_typed_values_and_world_pose() {
    let plain = scene().to_bytes();
    assert_eq!(
        &plain[8..12],
        &crate::scene_format::scene_format(1).to_le_bytes()
    );
    assert_eq!(scene().with_audio(vec![]).unwrap().to_bytes(), plain);
    let expected = scene().with_audio(nodes()).unwrap().to_bytes();
    assert_eq!(
        &expected[8..12],
        &crate::scene_format::scene_format(3).to_le_bytes()
    );
    let mut reversed = nodes();
    reversed.reverse();
    assert_eq!(scene().with_audio(reversed).unwrap().to_bytes(), expected);
    let decoded = CookedScene::from_bytes(&expected).unwrap();
    assert_eq!(decoded.audio(), nodes());
    let world = NativeWorld::new(decoded);
    assert_eq!(world.audio_nodes().len(), 3);
    assert_eq!(
        world
            .audio_nodes()
            .find(|(n, _)| n.entity == id(2))
            .unwrap()
            .1[3][0],
        3.
    );
    let saved = world.save_bytes().unwrap();
    assert_eq!(&saved[8..12], &8u32.to_le_bytes());
    let restored = NativeWorld::from_save_bytes(&saved).unwrap();
    assert!(restored.requires_audio_checkpoint());
    assert_eq!(
        restored
            .audio_nodes()
            .map(|(n, _)| n.clone())
            .collect::<Vec<_>>(),
        world
            .audio_nodes()
            .map(|(n, _)| n.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(restored.save_bytes().unwrap(), saved);
}

#[test]
fn nonzero_transport_epochs_roundtrip_with_exact_u64_precision_in_scene_v19() {
    for epoch in [1, (1u64 << 53) + 1, u64::MAX] {
        let mut audio = nodes();
        audio[1].source.as_mut().unwrap().transport_epoch = epoch;
        let bytes = scene().with_audio(audio.clone()).unwrap().to_bytes();
        assert_eq!(
            &bytes[8..12],
            &crate::scene_format::scene_format(19).to_le_bytes()
        );
        let decoded = CookedScene::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.audio(), audio);
        assert_eq!(decoded.to_bytes(), bytes);
        assert!(CookedScene::from_bytes(&bytes[..bytes.len() - 1]).is_err());
    }
}
#[test]
fn invalid_audio_references_and_numbers_fail_before_world_creation() {
    let mut bad = nodes();
    bad[1].source.as_mut().unwrap().gain_db = f32::NAN;
    assert!(scene().with_audio(bad).is_err());
    let mut bad = nodes();
    bad[1].source.as_mut().unwrap().bus = Some(id(2));
    assert!(scene().with_audio(bad).is_err());
    let mut bad = nodes();
    bad[0].bus.as_mut().unwrap().parent = Some(id(1));
    assert!(scene().with_audio(bad).is_err());
    let mut bad = nodes();
    bad.pop();
    assert!(scene().with_audio(bad).is_err());
    let mut bad = nodes();
    bad[2].entity = id(100);
    assert!(scene().with_audio(bad).is_err());
    let mut bad = nodes();
    bad.push(bad[0].clone());
    assert!(scene().with_audio(bad).is_err());
}
#[test]
fn structural_audio_validates_final_graph_and_preserves_all_state_on_failure() {
    let mut world = NativeWorld::new(scene().with_audio(nodes()).unwrap());
    let before = world.snapshot();
    let mut queue = FrameCommands::default();
    queue
        .push(StructuralCommand::SetVelocity {
            entity: id(4),
            velocity: Some(incant_types::Velocity {
                linear: [1., 2., 3.],
            }),
        })
        .unwrap();
    queue.push(StructuralCommand::Despawn(id(3))).unwrap();
    assert!(world.apply(&mut queue).is_err());
    assert_eq!(queue.len(), 2);
    assert_eq!(world.snapshot(), before);
    assert_eq!(
        world
            .audio_nodes()
            .map(|(n, _)| n.clone())
            .collect::<Vec<_>>(),
        nodes()
    );
    queue.clear();
    let mut source = nodes().remove(1);
    source.source.as_mut().unwrap().spatial = None;
    queue
        .push(StructuralCommand::SetAudio {
            entity: id(2),
            audio: Some(source),
        })
        .unwrap();
    queue.push(StructuralCommand::Despawn(id(3))).unwrap();
    world.apply(&mut queue).unwrap();
    assert_eq!(world.audio_nodes().len(), 2);
    assert!(queue.is_empty());
    queue
        .push(StructuralCommand::SetAudio {
            entity: id(1),
            audio: None,
        })
        .unwrap();
    assert!(world.apply(&mut queue).is_err()); // source still references this bus
    queue.clear();
    queue
        .push(StructuralCommand::SetAudio {
            entity: id(1),
            audio: None,
        })
        .unwrap();
    queue
        .push(StructuralCommand::SetAudio {
            entity: id(2),
            audio: None,
        })
        .unwrap();
    world.apply(&mut queue).unwrap();
    assert_eq!(world.audio_nodes().len(), 0);
    let saved = world.save_bytes().unwrap();
    assert_eq!(&saved[8..12], &8u32.to_le_bytes());
    let restored = NativeWorld::from_save_bytes(&saved).unwrap();
    assert!(restored.requires_audio_checkpoint());
    assert_eq!(
        restored
            .audio_nodes()
            .map(|(n, _)| n.clone())
            .collect::<Vec<_>>(),
        world
            .audio_nodes()
            .map(|(n, _)| n.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(restored.save_bytes().unwrap(), saved);
}
#[test]
fn truncated_or_noncanonical_audio_cannot_load() {
    let bytes = scene().with_audio(nodes()).unwrap().to_bytes();
    for len in 0..bytes.len() {
        assert!(CookedScene::from_bytes(&bytes[..len]).is_err());
    }
    let mut damaged = bytes.clone();
    *damaged.last_mut().unwrap() ^= 1;
    assert!(CookedScene::from_bytes(&damaged).is_err());
    // Unknown flags with a valid integrity hash must still be refused.
    let mut flags = bytes;
    let first_audio_mask = 72 + 4 * 104 + 4 + 4 + 16;
    flags[first_audio_mask] = 128;
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    hash.update(&flags[..40]);
    hash.update(&flags[72..]);
    flags[40..72].copy_from_slice(&hash.finalize());
    assert!(CookedScene::from_bytes(&flags).is_err());
}

#[test]
fn listener_rotation_composes_local_rotations_independently_of_inherited_scale() {
    let mut entities = scene().entities().to_vec();
    let turn = glam::DQuat::from_rotation_z(std::f64::consts::FRAC_PI_2);
    entities
        .iter_mut()
        .find(|e| e.id == id(1))
        .unwrap()
        .transform = Transform {
        rotation: turn.to_array(),
        scale: [2., 3., 4.],
        ..Default::default()
    };
    entities
        .iter_mut()
        .find(|e| e.id == id(2))
        .unwrap()
        .transform
        .rotation = turn.to_array();
    let world = NativeWorld::new(CookedScene::new(id(99), 60, entities).unwrap());
    let actual = glam::DQuat::from_array(world.world_rotation(id(2)).unwrap());
    assert!(actual.dot(turn * turn).abs() > 1. - 1e-12);
    assert!(world.world_rotation(id(999)).is_none());
}
