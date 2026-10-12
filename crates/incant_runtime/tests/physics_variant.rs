use incant_runtime::{
    CookedEntity, CookedScene, NativeWorld, PHYSICS_VARIANT, PhysicsMode, PhysicsVariant,
    SceneError, StableId, require_physics_mode,
};
use incant_types::{Transform, Velocity};
use sha2::{Digest, Sha256};

fn scene() -> CookedScene {
    CookedScene::new(
        StableId([1; 16]),
        60,
        vec![CookedEntity {
            id: StableId([2; 16]),
            parent: None,
            transform: Transform::default(),
            velocity: Some(Velocity {
                linear: [3., 0., 0.],
            }),
        }],
    )
    .unwrap()
}

fn image(version: u32, payload: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::from(*b"INCANTSC");
    bytes.extend(version.to_le_bytes());
    bytes.extend(60_u32.to_le_bytes());
    bytes.extend([1; 16]);
    bytes.extend(0_u32.to_le_bytes());
    bytes.extend((payload.len() as u32).to_le_bytes());
    bytes.resize(72, 0);
    bytes.extend(payload);
    let mut hash = Sha256::new();
    hash.update(&bytes[..40]);
    hash.update(payload);
    bytes[40..72].copy_from_slice(&hash.finalize());
    bytes
}

#[test]
fn legacy_scenes_keep_deterministic_semantics() {
    let bytes = image(1, &[]);
    let result = CookedScene::from_bytes(&bytes);
    if PHYSICS_VARIANT == PhysicsVariant::Deterministic {
        assert_eq!(result.unwrap().to_bytes(), bytes);
    } else {
        assert!(matches!(result, Err(SceneError::PhysicsVariant {
            requested: PhysicsVariant::Deterministic, compiled
        }) if compiled == PHYSICS_VARIANT));
    }
}

#[test]
fn variant_admission_precedes_scene_records_and_world_creation() {
    for (tag, variant) in [
        (0, PhysicsVariant::Deterministic),
        (1, PhysicsVariant::Fast),
        (2, PhysicsVariant::FastSimd8),
    ] {
        if variant == PHYSICS_VARIANT {
            continue;
        }
        // Deliberately omit all remaining layout-2 sections: exact variant refusal
        // must happen first, before allocating or constructing solver state.
        assert!(
            matches!(CookedScene::from_bytes(&image(2 | (tag << 16), &[])),
            Err(SceneError::PhysicsVariant { requested, compiled })
            if requested == variant && compiled == PHYSICS_VARIANT)
        );
        assert!(NativeWorld::from_bytes(&image(2 | (tag << 16), &[])).is_err());
    }
    assert!(matches!(
        CookedScene::from_bytes(&image(1 | (255 << 16), &[])),
        Err(SceneError::Version(_))
    ));
}

#[test]
fn cooked_and_saved_worlds_preserve_the_selected_build() {
    let cooked = scene();
    assert_eq!(cooked.physics_variant(), PHYSICS_VARIANT);
    let bytes = cooked.to_bytes();
    assert_eq!(CookedScene::from_bytes(&bytes).unwrap().to_bytes(), bytes);
    assert_eq!(
        u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
        match PHYSICS_VARIANT {
            PhysicsVariant::Deterministic => 1,
            PhysicsVariant::Fast => 1 | (1 << 16),
            PhysicsVariant::FastSimd8 => 1 | (2 << 16),
        }
    );
    let mut world = NativeWorld::from_bytes(&bytes).unwrap();
    world.step().unwrap();
    let saved = world.save_bytes().unwrap();
    let restored = NativeWorld::from_save_bytes(&saved).unwrap();
    assert_eq!(restored.tick(), 1);
    assert_eq!(restored.save_bytes().unwrap(), saved);
    assert_eq!(
        restored
            .inspect(StableId([2; 16]))
            .unwrap()
            .transform
            .translation,
        [0.05, 0., 0.]
    );
}

#[test]
fn project_mode_requires_a_matching_kernel_family() {
    require_physics_mode(PHYSICS_VARIANT.mode()).unwrap();
    let other = if PHYSICS_VARIANT.mode() == PhysicsMode::Fast {
        PhysicsMode::Deterministic
    } else {
        PhysicsMode::Fast
    };
    assert!(
        matches!(require_physics_mode(other), Err(SceneError::PhysicsMode {
        requested, compiled
    }) if requested == other && compiled == PHYSICS_VARIANT)
    );
}

#[test]
fn rehashed_foreign_variant_save_is_rejected_without_changing_the_live_world() {
    let mut world = NativeWorld::new(scene());
    world.step().unwrap();
    let before = world.save_bytes().unwrap();
    // This plain world uses the bound v3 save framing: 64-byte header and
    // 40-byte binding extension, followed by the complete cooked scene.
    assert_eq!(u32::from_le_bytes(before[8..12].try_into().unwrap()), 3);
    let scene_start = 104;
    assert_eq!(&before[scene_start..scene_start + 8], b"INCANTSC");
    let scene_len = u32::from_le_bytes(before[96..100].try_into().unwrap()) as usize;
    for (tag, variant) in [
        (0, PhysicsVariant::Deterministic),
        (1, PhysicsVariant::Fast),
        (2, PhysicsVariant::FastSimd8),
    ] {
        if variant == PHYSICS_VARIANT {
            continue;
        }
        let mut foreign = before.clone();
        let nested = &mut foreign[scene_start..scene_start + scene_len];
        let layout = u32::from_le_bytes(nested[8..12].try_into().unwrap()) & 0xffff;
        nested[8..12].copy_from_slice(&(layout | (tag << 16)).to_le_bytes());
        let mut inner_hash = Sha256::new();
        inner_hash.update(&nested[..40]);
        inner_hash.update(&nested[72..]);
        nested[40..72].copy_from_slice(&inner_hash.finalize());
        let mut outer_hash = Sha256::new();
        outer_hash.update(&foreign[..32]);
        outer_hash.update(&foreign[64..]);
        foreign[32..64].copy_from_slice(&outer_hash.finalize());
        // Both integrity layers are valid. The exact solver identity, not a
        // checksum failure or the later original-scene binding, must refuse it.
        assert!(matches!(
            NativeWorld::from_save_bytes(&foreign),
            Err(incant_runtime::SaveError::Scene(SceneError::PhysicsVariant {
                requested, compiled
            })) if requested == variant && compiled == PHYSICS_VARIANT
        ));
        assert!(matches!(
            world.restore_bytes(&foreign),
            Err(incant_runtime::SaveError::Scene(SceneError::PhysicsVariant {
                requested, compiled
            })) if requested == variant && compiled == PHYSICS_VARIANT
        ));
        assert_eq!(world.save_bytes().unwrap(), before);
        assert_eq!(world.tick(), 1);
    }
    world.step().unwrap();
    assert_eq!(world.tick(), 2);
}
