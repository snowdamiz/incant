use incant_runtime::*;
use incant_types::{Transform, Velocity};
use sha2::{Digest, Sha256};
fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
fn map(solid: bool) -> NativeTilemap {
    NativeTilemap {
        tileset: id(90),
        size: [3, 2],
        cell_size: [0.5, 2.],
        layers: vec![NativeTilemapLayer {
            id: id(2),
            cells: vec![
                Some(id(8)),
                Some(id(8)),
                None,
                Some(id(8)),
                Some(id(8)),
                None,
            ],
            layer: -1,
            order: 4,
            solid,
        }],
    }
}
fn scene(map: NativeTilemap) -> Result<CookedScene, SceneError> {
    let solid = map.layers.iter().any(|layer| layer.solid);
    let mut node = CookedSpatial::empty(id(1));
    node.tilemap = Some(Box::new(map));
    CookedScene::new(
        id(99),
        60,
        vec![CookedEntity {
            id: id(1),
            parent: None,
            transform: Transform::default(),
            velocity: Some(Velocity {
                linear: [if solid { 0. } else { 1. }, 0., 0.],
            }),
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
    let len = (bytes.len() - 72) as u32;
    bytes[36..40].copy_from_slice(&len.to_le_bytes());
    let hash = Sha256::new()
        .chain_update(&bytes[..40])
        .chain_update(&bytes[72..])
        .finalize();
    bytes[40..72].copy_from_slice(&hash);
}
#[test]
fn v6_round_trip_preserves_binary_frames_and_retained_local_preparation() {
    let input = scene(map(false)).unwrap();
    let bytes = input.to_bytes();
    assert_eq!(&bytes[8..12], &6u32.to_le_bytes());
    let loaded = CookedScene::from_bytes(&bytes).unwrap();
    assert_eq!(loaded.spatial(), input.spatial());
    assert_eq!(loaded.to_bytes(), bytes);
    let mut world = NativeWorld::new(loaded);
    let pointer = {
        let view = world.spatial_view().unwrap();
        let node = view.nodes().next().unwrap();
        let map = node.tilemap.unwrap();
        assert_eq!(map.placements().len(), 4);
        assert!(map.collision().is_empty());
        map.placements().as_ptr()
    };
    for _ in 0..20 {
        world.step().unwrap();
        let view = world.spatial_view().unwrap();
        let node = view.nodes().next().unwrap();
        assert_eq!(node.tilemap.unwrap().placements().as_ptr(), pointer);
        assert!((node.global.w_axis.x - world.tick() as f64 / 60.).abs() < 1e-12);
    }
    assert!(world.save_bytes().is_ok());
}
#[test]
fn solid_maps_step_in_retained_solver_and_structural_removal_clears_binding() {
    let mut world = NativeWorld::new(scene(map(true)).unwrap());
    let before = world.snapshot();
    let view = world.spatial_view().unwrap();
    let node = view.nodes().next().unwrap();
    let rects = node.tilemap.unwrap().collision();
    assert_eq!(rects.len(), 1);
    assert_eq!(rects[0].origin, [0, 0]);
    assert_eq!(rects[0].size, [2, 2]);
    world.step().unwrap();
    assert_eq!(world.snapshot(), before);
    assert_eq!(world.tick(), 1);
    let mut commands = FrameCommands::default();
    commands
        .push(StructuralCommand::SetSpatial {
            entity: id(1),
            spatial: None,
        })
        .unwrap();
    world.apply(&mut commands).unwrap();
    world.step().unwrap();
    assert!(
        world
            .spatial_view()
            .unwrap()
            .nodes()
            .next()
            .unwrap()
            .tilemap
            .is_none()
    );
    let mut world = NativeWorld::new(scene(map(true)).unwrap());
    commands.push(StructuralCommand::Despawn(id(1))).unwrap();
    world.apply(&mut commands).unwrap();
    world.step().unwrap();
    assert_eq!(world.entity_count(), 0);
}
#[test]
fn rejected_map_batch_keeps_motion_topology_generations_and_prepared_storage() {
    let mut world = NativeWorld::new(scene(map(false)).unwrap());
    let before = world.snapshot();
    let (pointer, generation) = {
        let view = world.spatial_view().unwrap();
        (
            view.nodes()
                .next()
                .unwrap()
                .tilemap
                .unwrap()
                .placements()
                .as_ptr(),
            view.binding_generation(),
        )
    };
    let mut node = CookedSpatial::empty(id(1));
    let mut bad = map(true);
    bad.tileset = id(91);
    node.tilemap = Some(Box::new(bad));
    let mut commands = FrameCommands::default();
    commands
        .push(StructuralCommand::SetVelocity {
            entity: id(1),
            velocity: Some(Velocity { linear: [99.; 3] }),
        })
        .unwrap();
    commands
        .push(StructuralCommand::SetSpatial {
            entity: id(1),
            spatial: Some(node),
        })
        .unwrap();
    assert!(world.apply(&mut commands).is_err());
    assert_eq!(commands.len(), 2);
    assert_eq!(world.snapshot(), before);
    let view = world.spatial_view().unwrap();
    assert_eq!(view.binding_generation(), generation);
    assert_eq!(
        view.nodes()
            .next()
            .unwrap()
            .tilemap
            .unwrap()
            .placements()
            .as_ptr(),
        pointer
    );
    world.step().unwrap();
}
#[test]
fn malformed_lengths_flags_versions_and_frames_are_bounded_before_publication() {
    let original = scene(map(true)).unwrap().to_bytes();
    // This fixture has one entity, asset and spatial record. Tile header follows
    // the fixed entity+section prefixes, asset descriptor and spatial ID/mask.
    let start = 72 + 128 + 4 + 4 + 4 + 49 + 4 + 17;
    for (offset, bytes) in [
        (start + 16, 0u32.to_le_bytes().to_vec()),
        (start + 40, u32::MAX.to_le_bytes().to_vec()),
        (start + 44 + 22, vec![2]),
        (start + 44 + 23, u32::MAX.to_le_bytes().to_vec()),
        (start + 44 + 27, vec![2]),
    ] {
        let mut bad = original.clone();
        bad[offset..offset + bytes.len()].copy_from_slice(&bytes);
        rehash(&mut bad);
        assert!(CookedScene::from_bytes(&bad).is_err(), "offset {offset}");
    }
    for len in 72..original.len() {
        let mut bad = original[..len].to_vec();
        rehash(&mut bad);
        assert!(CookedScene::from_bytes(&bad).is_err(), "length {len}");
    }
    let mut bad = original.clone();
    bad[8..12].copy_from_slice(&5u32.to_le_bytes());
    rehash(&mut bad);
    assert!(CookedScene::from_bytes(&bad).is_err());
    let mut bad = map(true);
    bad.tileset = id(91);
    assert!(scene(bad).is_err());
    let mut bad = map(true);
    bad.layers.push(bad.layers[0].clone());
    assert!(scene(bad).is_err());
}
#[test]
fn aggregate_cell_cap_is_checked_at_scene_and_structural_boundaries() {
    let template = NativeTilemap {
        tileset: id(90),
        size: [512, 512],
        cell_size: [1.; 2],
        layers: vec![NativeTilemapLayer {
            id: id(7),
            cells: vec![None; 262144],
            layer: 0,
            order: 0,
            solid: false,
        }],
    };
    let entities = (1..=5)
        .map(|n| CookedEntity {
            id: id(n),
            parent: None,
            transform: Transform::default(),
            velocity: None,
        })
        .collect();
    let base = CookedScene::new(id(99), 60, entities).unwrap();
    let assets = vec![VisualAssetRef {
        id: id(90),
        kind: VisualAssetKind::Texture,
        fingerprint: [0; 32],
    }];
    let nodes: Vec<_> = (1..=5)
        .map(|n| {
            let mut node = CookedSpatial::empty(id(n));
            node.tilemap = Some(Box::new(template.clone()));
            node
        })
        .collect();
    assert!(
        base.clone()
            .with_spatial(assets.clone(), nodes.clone())
            .is_err()
    );
    let accepted = base.with_spatial(assets, nodes[..4].to_vec()).unwrap();
    let bytes = accepted.to_bytes();
    CookedScene::from_bytes(&bytes).unwrap();
    let mut world = NativeWorld::new(accepted);
    let generation = world.spatial_view().unwrap().binding_generation();
    let mut commands = FrameCommands::default();
    commands
        .push(StructuralCommand::SetSpatial {
            entity: id(5),
            spatial: Some(nodes[4].clone()),
        })
        .unwrap();
    assert!(world.apply(&mut commands).is_err());
    assert_eq!(
        world.spatial_view().unwrap().binding_generation(),
        generation
    );
}

#[test]
fn frozen_source_free_v1_through_v5_scenes_reencode_byte_for_byte_and_still_step() {
    for bytes in [
        include_bytes!("fixtures/tilemap-compat/v1.scene").as_slice(),
        include_bytes!("fixtures/tilemap-compat/v2.scene"),
        include_bytes!("fixtures/tilemap-compat/v3.scene"),
        include_bytes!("fixtures/tilemap-compat/v4.scene"),
        include_bytes!("fixtures/tilemap-compat/v5.scene"),
    ] {
        let scene = CookedScene::from_bytes(bytes).unwrap();
        assert_eq!(scene.to_bytes(), bytes);
        let mut first = NativeWorld::new(scene.clone());
        let mut second = NativeWorld::new(scene);
        for _ in 0..3 {
            first.step().unwrap();
            second.step().unwrap();
            assert_eq!(first.snapshot(), second.snapshot());
        }
    }
}

#[test]
fn noops_reparenting_and_archetype_relocation_preserve_local_prepared_buffers() {
    let scene = scene(map(false)).unwrap();
    let declaration = scene.spatial()[0].clone();
    let mut world = NativeWorld::new(scene);
    let (pointer, generation) = {
        let view = world.spatial_view().unwrap();
        (
            view.nodes()
                .next()
                .unwrap()
                .tilemap
                .unwrap()
                .placements()
                .as_ptr(),
            view.binding_generation(),
        )
    };
    let mut commands = FrameCommands::default();
    commands
        .push(StructuralCommand::SetSpatial {
            entity: id(1),
            spatial: Some(declaration.clone()),
        })
        .unwrap();
    world.apply(&mut commands).unwrap();
    assert_eq!(
        world.spatial_view().unwrap().binding_generation(),
        generation
    );
    assert_eq!(
        world
            .spatial_view()
            .unwrap()
            .nodes()
            .next()
            .unwrap()
            .tilemap
            .unwrap()
            .placements()
            .as_ptr(),
        pointer
    );
    commands
        .push(StructuralCommand::Spawn(CookedEntity {
            id: id(50),
            parent: None,
            transform: Transform {
                translation: [3., 4., 0.],
                ..Default::default()
            },
            velocity: None,
        }))
        .unwrap();
    commands
        .push(StructuralCommand::SetParent {
            entity: id(1),
            parent: Some(id(50)),
        })
        .unwrap();
    commands
        .push(StructuralCommand::SetVelocity {
            entity: id(1),
            velocity: None,
        })
        .unwrap();
    world.apply(&mut commands).unwrap();
    let view = world.spatial_view().unwrap();
    let node = view.nodes().find(|n| n.id == id(1)).unwrap();
    assert_eq!(node.tilemap.unwrap().placements().as_ptr(), pointer);
    assert_eq!(node.global.w_axis.truncate().to_array(), [3., 4., 0.]);
    let mut changed = declaration;
    changed.tilemap.as_mut().unwrap().layers[0].solid = true;
    commands
        .push(StructuralCommand::SetSpatial {
            entity: id(1),
            spatial: Some(changed),
        })
        .unwrap();
    commands
        .push(StructuralCommand::SetParent {
            entity: id(50),
            parent: Some(id(1)),
        })
        .unwrap();
    assert!(world.apply(&mut commands).is_err());
    let view = world.spatial_view().unwrap();
    assert_eq!(view.binding_generation(), generation);
    let map = view
        .nodes()
        .find(|n| n.id == id(1))
        .unwrap()
        .tilemap
        .unwrap();
    assert_eq!(map.placements().as_ptr(), pointer);
    assert!(map.collision().is_empty());
    world.step().unwrap(); // Rejected solid configuration did not install collision.
}
