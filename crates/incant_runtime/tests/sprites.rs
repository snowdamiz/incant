use incant_runtime::*;
use incant_types::Transform;
use sha2::{Digest, Sha256};
fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
fn sprite() -> NativeSpriteRenderer {
    NativeSpriteRenderer {
        texture: id(90),
        region: NativeSpriteRegion::Pixels([2, 3, 20, 30]),
        size: [4., 5.],
        pivot: [0.25, 0.75],
        tint: [0.5, 0.75, 1., 0.25],
        flip_x: true,
        flip_y: true,
        layer: -123,
        order: -123456,
        sampling: SpriteSampling::Linear,
        mode: SpriteMode::NineSlice {
            border: [2, 3, 4, 5],
            pixels_per_unit: 8.,
        },
    }
}
fn scene(s: NativeSpriteRenderer) -> Result<CookedScene, SceneError> {
    let mut node = CookedSpatial::empty(id(1));
    node.sprite = Some(Box::new(s));
    CookedScene::new(
        id(99),
        60,
        vec![CookedEntity {
            id: id(1),
            parent: None,
            transform: Transform::default(),
            velocity: None,
        }],
    )?
    .with_spatial(
        vec![VisualAssetRef {
            id: id(90),
            kind: VisualAssetKind::Texture,
            fingerprint: [7; 32],
        }],
        vec![node],
    )
}
fn rehash(bytes: &mut [u8]) {
    let n = (bytes.len() - 72) as u32;
    bytes[36..40].copy_from_slice(&n.to_le_bytes());
    let sum = Sha256::new()
        .chain_update(&bytes[..40])
        .chain_update(&bytes[72..])
        .finalize();
    bytes[40..72].copy_from_slice(&sum);
}
#[test]
fn binary_v5_preserves_sprite_numeric_values_and_binary_frame_ids() {
    for region in [
        NativeSpriteRegion::Pixels([2, 3, 20, 30]),
        NativeSpriteRegion::Atlas {
            frame: StableId([255; 16]),
        },
    ] {
        for mode in [SpriteMode::Simple, sprite().mode] {
            let mut s = sprite();
            s.region = region;
            s.mode = mode;
            let expected = scene(s.clone()).unwrap();
            let bytes = expected.to_bytes();
            assert_eq!(&bytes[8..12], &5u32.to_le_bytes());
            let loaded = CookedScene::from_bytes(&bytes).unwrap();
            assert_eq!(loaded.spatial()[0].sprite.as_deref(), Some(&s));
            assert_eq!(loaded.to_bytes(), bytes);
            let world = NativeWorld::new(loaded);
            assert_eq!(
                world
                    .spatial_view()
                    .unwrap()
                    .nodes()
                    .next()
                    .unwrap()
                    .declaration
                    .unwrap(),
                &expected.spatial()[0]
            );
        }
    }
}
#[test]
fn binary_sprite_tags_flags_ranges_and_truncations_are_checked() {
    let original = scene(sprite()).unwrap().to_bytes();
    let start = original.len() - 130;
    for (offset, byte) in [
        (start + 16, 2),
        (start + 97, 4),
        (start + 104, 2),
        (start + 105, 2),
    ] {
        let mut bytes = original.clone();
        bytes[offset] = byte;
        rehash(&mut bytes);
        assert!(CookedScene::from_bytes(&bytes).is_err(), "offset {offset}");
    }
    let mut bytes = original.clone();
    bytes[8..12].copy_from_slice(&4u32.to_le_bytes());
    rehash(&mut bytes);
    assert!(CookedScene::from_bytes(&bytes).is_err());
    let mut bytes = original.clone();
    bytes[start + 33..start + 41].copy_from_slice(&f64::NAN.to_le_bytes());
    rehash(&mut bytes);
    assert!(CookedScene::from_bytes(&bytes).is_err());
    for len in 72..original.len() {
        let mut bytes = original[..len].to_vec();
        rehash(&mut bytes);
        assert!(CookedScene::from_bytes(&bytes).is_err(), "truncation {len}");
    }
}
#[test]
fn invalid_structural_sprite_change_preserves_world_and_binding_generation() {
    let mut world = NativeWorld::new(scene(sprite()).unwrap());
    let before = world.spatial_view().unwrap().binding_generation();
    let original = world
        .spatial_view()
        .unwrap()
        .nodes()
        .next()
        .unwrap()
        .declaration
        .unwrap()
        .clone();
    for kind in 0..4 {
        let mut node = original.clone();
        let s = node.sprite.as_mut().unwrap();
        match kind {
            0 => s.texture = id(7),
            1 => s.size[0] = 0.,
            2 => s.tint[3] = f64::INFINITY,
            _ => {
                node.mesh = Some(NativeMeshRenderer {
                    mesh: id(90),
                    cast_shadows: false,
                })
            }
        }
        let mut commands = FrameCommands::default();
        commands
            .push(StructuralCommand::SetSpatial {
                entity: id(1),
                spatial: Some(node),
            })
            .unwrap();
        assert!(world.apply(&mut commands).is_err());
        assert_eq!(commands.len(), 1);
        let view = world.spatial_view().unwrap();
        assert_eq!(view.binding_generation(), before);
        assert_eq!(view.nodes().next().unwrap().declaration.unwrap(), &original);
    }
    let mut changed = original.clone();
    changed.sprite.as_mut().unwrap().order = 42;
    let mut commands = FrameCommands::default();
    commands
        .push(StructuralCommand::SetSpatial {
            entity: id(1),
            spatial: Some(changed.clone()),
        })
        .unwrap();
    world.apply(&mut commands).unwrap();
    let view = world.spatial_view().unwrap();
    assert_eq!(view.binding_generation(), before + 1);
    assert_eq!(view.nodes().next().unwrap().declaration.unwrap(), &changed);
}
