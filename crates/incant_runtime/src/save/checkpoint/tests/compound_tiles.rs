use super::*;
use crate::{CookedSpatial, NativeTilemap, NativeTilemapLayer, VisualAssetKind, VisualAssetRef};
use incant_types::{ColliderPart, PrimitiveColliderShape};

fn warm_compound_tiles() -> NativeWorld {
    let mut tile = CookedSpatial::empty(id(11));
    tile.tilemap = Some(Box::new(NativeTilemap {
        tileset: id(90),
        size: [5, 1],
        cell_size: [1., 1.],
        layers: vec![NativeTilemapLayer {
            id: id(80),
            cells: vec![Some(id(81)), Some(id(81)), None, Some(id(81)), Some(id(81))],
            layer: 0,
            order: 0,
            solid: true,
        }],
    }));
    let mut tile_entity = entity(11, -1.);
    tile_entity.transform.translation[0] = -1.;
    let scene = CookedScene::new(
        id(99),
        64,
        vec![entity(1, -0.5), entity(2, 1.), tile_entity, entity(12, 1.)],
    )
    .unwrap()
    .with_spatial(
        vec![VisualAssetRef {
            id: id(90),
            kind: VisualAssetKind::Texture,
            fingerprint: [3; 32],
        }],
        vec![tile],
    )
    .unwrap()
    .with_physics(vec![
        CookedPhysics {
            entity: id(1),
            body: None,
            collider: Collider {
                shape: ColliderShape::Box {
                    half_extents: [4., 0.5, 4.],
                },
                ..Default::default()
            },
            angular_velocity: None,
        },
        CookedPhysics {
            entity: id(2),
            body: Some(RigidBody::default()),
            collider: Collider {
                shape: ColliderShape::Compound {
                    parts: [1, 2]
                        .map(|n| ColliderPart {
                            id: format!("{n:026}"),
                            translation: [if n == 1 { -0.3 } else { 0.3 }, 0., 0.],
                            rotation: [0., 0., 0., 1.],
                            shape: PrimitiveColliderShape::Box {
                                half_extents: [0.35; 3],
                            },
                        })
                        .into(),
                },
                ..Default::default()
            },
            angular_velocity: None,
        },
    ])
    .unwrap()
    .with_physics_2d(vec![CookedPhysics2D {
        entity: id(12),
        body: Some(RigidBody::default()),
        collider: Collider2D::default(),
        angular_velocity: None,
    }])
    .unwrap();
    let mut world = NativeWorld::new(scene);
    for _ in 0..128 {
        world.step().unwrap();
    }
    world
}
#[test]
fn opaque_checkpoint_joins_compound_and_automatic_tiles_without_opening_save_bytes() {
    let mut source = warm_compound_tiles();
    assert_eq!(source.save_bytes(), Err(SaveError::PhysicsUnsupported));
    assert!(source.physics_2d.checkpoint_configurations().is_err());
    let rows: Vec<_> = source
        .physics_2d
        .checkpoint_geometry_configurations()
        .unwrap()
        .collect();
    assert_eq!(rows.len(), 2);
    assert!(
        matches!(rows[0].geometry, incant_physics_2d_runtime::CheckpointGeometry::Tiles(p) if p.len() == 2)
    );
    // Pending ECS motion must remain pending, rather than overwrite the captured
    // solver poses. Tile geometry stays tied to its immutable prepared map.
    source.with_numeric_columns(|columns| {
        columns.for_each_chunk(|chunk| {
            for (i, entity) in chunk.ids.iter().enumerate() {
                if [id(2), id(12)].contains(entity) {
                    chunk.transforms[i * crate::TRANSFORM_STRIDE + 1] += 0.75;
                }
            }
        })
    });
    let checkpoint = capture(&mut source);
    let mut restored = reconstruct(&checkpoint);
    observations(&source, &restored);
    for _ in 0..64 {
        source.step().unwrap();
        restored.step().unwrap();
        observations(&source, &restored);
        assert_eq!(
            capture(&mut source).components,
            capture(&mut restored).components
        );
    }
    assert_eq!(restored.save_bytes(), Err(SaveError::PhysicsUnsupported));
    let mut missing = capture(&mut source);
    missing.planar = None;
    assert!(matches!(
        missing.reconstruct(
            &mut payload(),
            &mut planar_work(),
            &mut spatial_work(),
            &mut allocations()
        ),
        Err(WorldCheckpointError::Owner("missing native owner"))
    ));
}
#[test]
fn tile_checkpoint_binding_refuses_map_and_membership_drift_before_publication() {
    let mut source = warm_compound_tiles();
    source.tilemap_collision_ids.remove(&id(11));
    assert!(matches!(
        source.checkpoint(&mut payload(), &mut planar_work(), &mut spatial_work()),
        Err(WorldCheckpointError::Owner("tile binding configuration"))
    ));
    let mut source = warm_compound_tiles();
    let mut changed = source
        .world
        .get::<CookedSpatial>(source.entities[&id(11)])
        .unwrap()
        .tilemap
        .as_ref()
        .unwrap()
        .as_ref()
        .clone();
    changed.cell_size[0] = 0.75;
    source
        .world
        .entity_mut(source.entities[&id(11)])
        .insert(crate::tilemap::Prepared(changed.prepare().unwrap()));
    assert!(matches!(
        source.checkpoint(&mut payload(), &mut planar_work(), &mut spatial_work()),
        Err(WorldCheckpointError::Owner("tile rectangle declaration"))
    ));
    let mut source = warm_compound_tiles();
    source
        .world
        .get_mut::<Binding2D>(source.entities[&id(11)])
        .unwrap()
        .collider = Some(Collider2D::default());
    assert!(matches!(
        source.checkpoint(&mut payload(), &mut planar_work(), &mut spatial_work()),
        Err(WorldCheckpointError::Owner("tile binding configuration"))
    ));
}
