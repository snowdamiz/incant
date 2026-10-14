use incant_runtime::{
    CookedEntity, CookedScene, FrameCommands, NativeWorld, StableId, StructuralCommand,
};
use incant_types::Velocity;

fn id(n: u8) -> StableId {
    StableId([n; 16])
}

fn entity(n: u8) -> CookedEntity {
    CookedEntity {
        id: id(n),
        parent: None,
        transform: Default::default(),
        velocity: None,
    }
}

fn world() -> NativeWorld {
    NativeWorld::new(CookedScene::new(id(0), 60, vec![entity(1), entity(2)]).unwrap())
}

#[test]
fn tokens_require_the_exact_world_generation_and_stable_id() {
    let first = world();
    let other = world();
    let token = first.entity_identity(id(1)).unwrap();
    assert!(first.matches_entity_identity(id(1), token));
    assert!(!first.matches_entity_identity(id(2), token));
    assert!(!first.matches_entity_identity(id(99), token));
    assert!(!other.matches_entity_identity(id(1), token));
}

#[test]
fn topology_and_archetype_moves_keep_a_live_token() {
    let mut native = world();
    let token = native.entity_identity(id(1)).unwrap();
    let mut commands = FrameCommands::default();
    commands
        .push(StructuralCommand::SetParent {
            entity: id(1),
            parent: Some(id(2)),
        })
        .unwrap();
    commands
        .push(StructuralCommand::SetVelocity {
            entity: id(1),
            velocity: Some(Velocity {
                linear: [1., 0., 0.],
            }),
        })
        .unwrap();
    native.apply(&mut commands).unwrap();
    native.step().unwrap();
    assert!(native.matches_entity_identity(id(1), token));
    assert_eq!(native.entity_identity(id(1)), Some(token));
    commands
        .push(StructuralCommand::SetVelocity {
            entity: id(1),
            velocity: None,
        })
        .unwrap();
    native.apply(&mut commands).unwrap();
    assert!(native.matches_entity_identity(id(1), token));
}

#[test]
fn despawn_and_stable_id_reuse_never_revive_the_old_token() {
    let mut native = world();
    let original = native.entity_identity(id(1)).unwrap();
    let mut commands = FrameCommands::default();
    commands.push(StructuralCommand::Despawn(id(1))).unwrap();
    native.apply(&mut commands).unwrap();
    assert!(!native.matches_entity_identity(id(1), original));
    commands.push(StructuralCommand::Spawn(entity(1))).unwrap();
    native.apply(&mut commands).unwrap();
    assert!(!native.matches_entity_identity(id(1), original));
    let replacement = native.entity_identity(id(1)).unwrap();
    assert_ne!(replacement, original);
    assert!(native.matches_entity_identity(id(1), replacement));
}

#[test]
fn refused_restore_keeps_tokens_but_successful_restore_invalidates_them() {
    let mut native = world();
    let bytes = native.save_bytes().unwrap();
    let original = native.entity_identity(id(1)).unwrap();
    assert!(native.restore_bytes(&bytes[..bytes.len() - 1]).is_err());
    assert!(native.matches_entity_identity(id(1), original));
    native.restore_bytes(&bytes).unwrap();
    assert!(!native.matches_entity_identity(id(1), original));
    let restored = native.entity_identity(id(1)).unwrap();
    assert!(native.matches_entity_identity(id(1), restored));
}
