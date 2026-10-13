use super::*;
fn tile_config() -> PreparedBody {
    let map = incant_2d::Tilemap {
        tileset: id(20),
        size: [5, 1],
        cell_size: [1., 1.],
        layers: vec![incant_2d::TilemapLayer {
            id: id(21),
            cells: vec![Some(id(22)), Some(id(22)), None, Some(id(22)), Some(id(22))],
            layer: 0,
            order: 0,
            solid: true,
        }],
    }
    .prepare()
    .unwrap();
    PreparedBody::tilemap(
        &map,
        BodyState {
            translation: [-1., -1.],
            ..Default::default()
        },
    )
    .unwrap()
}
#[test]
fn tile_configuration_copy_and_borrow_preserve_geometry_with_exact_charge() {
    let config = tile_config();
    let bytes = 2 * size_of::<([f64; 2], [f64; 2])>();
    assert!(copy_config(&config, &mut Budget::new(bytes - 1).unwrap()).is_err());
    let mut exact = Budget::new(bytes).unwrap();
    let copied = copy_config(&config, &mut exact).unwrap();
    assert_eq!(exact.remaining(), 0);
    assert_eq!(config.geometry, copied.geometry);
    let mut world = PhysicsWorld2D::default();
    world.apply(vec![(id(1), Some(config))]).unwrap();
    let row = world
        .checkpoint_geometry_configurations()
        .unwrap()
        .next()
        .unwrap();
    assert_eq!(row.id, id(1));
    let CheckpointGeometry::Tiles(parts) = row.geometry else {
        panic!()
    };
    let config::Geometry::Tiles(owned) = &world.entries[&id(1)].config.geometry else {
        panic!()
    };
    assert_eq!(parts.len(), 2);
    assert!(std::ptr::eq(parts.as_ptr(), owned.as_ptr()));
    assert!(world.checkpoint_configurations().is_err()); // Legacy API remains explicit.
}
#[test]
fn tile_native_checkpoint_preserves_warm_continuation_and_refuses_late_drift() {
    let mut source = PhysicsWorld2D::default();
    source
        .apply(vec![
            (id(1), Some(tile_config())),
            (id(2), Some(declaration(true, false, 1.))),
            (id(3), Some(declaration(false, true, 0.6))),
        ])
        .unwrap();
    for _ in 0..512 {
        source.step(1. / 64.).unwrap();
    }
    assert!(source.state(id(2)).unwrap().sleeping);
    assert!(source.raycast(&ray()).unwrap().is_some());
    let checkpoint = capture(&mut source);
    let mut restored = checkpoint
        .reconstruct(&mut budget(), &mut work(), &mut allocations())
        .unwrap();
    for tick in 0..96 {
        for world in [&mut source, &mut restored] {
            if tick == 8 {
                world.apply(vec![(id(3), None)]).unwrap();
            }
            if tick == 16 {
                world
                    .apply(vec![(id(4), Some(declaration(false, true, 0.6)))])
                    .unwrap();
            }
            if tick == 40 {
                let mut motion = world.state(id(2)).unwrap().clone();
                motion.translation[1] += 1.;
                motion.velocity[0] = 0.2;
                world.sync_motion(&[(id(2), motion)]).unwrap();
            }
            world.step(1. / 64.).unwrap();
        }
        assert_eq!(states(&source), states(&restored), "tick {tick}");
        assert_eq!(source.events(), restored.events(), "tick {tick}");
        assert_eq!(source.raycast(&ray()), restored.raycast(&ray()));
        assert_eq!(
            capture(&mut source).native.encoded(),
            capture(&mut restored).native.encoded(),
            "tick {tick}"
        );
    }
    let before = capture(&mut restored).native.encoded().to_vec();
    let mut corrupt = capture(&mut source);
    let config::Geometry::Tiles(parts) = &mut corrupt.entries[0].config.geometry else {
        panic!()
    };
    parts.swap(0, 1);
    assert!(matches!(
        restored.restore_checkpoint(&corrupt, &mut budget(), &mut work(), &mut allocations()),
        Err(CheckpointError::Owner("collider geometry"))
    ));
    assert_eq!(capture(&mut restored).native.encoded(), before);
}
