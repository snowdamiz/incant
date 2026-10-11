use incant_2d::tilemap::*;
fn map(size: [u32; 2], cells: Vec<Option<u32>>) -> Tilemap<u32> {
    Tilemap {
        tileset: 10,
        size,
        cell_size: [0.5, 2.],
        layers: vec![TilemapLayer {
            id: 1,
            cells,
            layer: 0,
            order: 0,
            solid: true,
        }],
    }
}
#[test]
fn collision_rectangles_exactly_cover_every_small_grid_and_union_layers() {
    for bits in 0_u32..512 {
        let cells: Vec<_> = (0..9)
            .map(|i| (bits & (1 << i) != 0).then_some(7))
            .collect();
        let mut value = map([3, 3], cells.clone());
        value.layers.push(TilemapLayer {
            id: 2,
            cells: cells.into_iter().rev().collect(),
            layer: -1,
            order: 0,
            solid: true,
        });
        let result = value.prepare().unwrap();
        let mut hits = [0_u8; 9];
        for rect in result.collision() {
            assert!(rect.size[0] > 0 && rect.size[1] > 0);
            assert!(rect.origin[0] + rect.size[0] <= 3 && rect.origin[1] + rect.size[1] <= 3);
            for y in rect.origin[1]..rect.origin[1] + rect.size[1] {
                for x in rect.origin[0]..rect.origin[0] + rect.size[0] {
                    hits[(y * 3 + x) as usize] += 1;
                }
            }
        }
        for (i, hits) in hits.into_iter().enumerate() {
            let expected = value.layers.iter().any(|layer| layer.cells[i].is_some());
            assert_eq!(hits, u8::from(expected), "pattern {bits}, cell {i}");
        }
    }
}
#[test]
fn local_geometry_order_and_collision_ignore_empty_and_nonsolid_cells() {
    let mut value = map([3, 2], vec![Some(8), Some(9), None, Some(8), Some(9), None]);
    value.layers.push(TilemapLayer {
        id: 2,
        cells: vec![Some(3); 6],
        layer: -2,
        order: 0,
        solid: false,
    });
    let result = value.prepare().unwrap();
    assert_eq!(result.placements().len(), 10);
    assert_eq!(result.placements()[0].layer, 2);
    assert_eq!(result.placements()[5].cell, [2, 1]);
    assert_eq!(result.placements()[6].frame, 8);
    assert_eq!(result.cell_bounds([2, 1]), [[1., 2.], [1.5, 4.]]);
    assert_eq!(
        result.collision(),
        &[TileCollisionRect {
            origin: [0, 0],
            size: [2, 2]
        }]
    );
    let pointer = result.placements().as_ptr();
    for _ in 0..100 {
        assert_eq!(pointer, result.placements().as_ptr());
    }
}
#[test]
fn malformed_shapes_identities_and_extents_fail_before_preparation() {
    let valid = map([1, 1], vec![Some(1)]);
    for size in [[0, 1], [1, 0], [4097, 1], [4096, 4096]] {
        let mut bad = valid.clone();
        bad.size = size;
        assert!(matches!(bad.prepare(), Err(TilemapError::Bounds)));
    }
    let mut bad = valid.clone();
    bad.layers[0].cells.push(None);
    assert_eq!(bad.validate(), Err(TilemapError::Bounds));
    let mut bad = valid.clone();
    bad.layers.push(bad.layers[0].clone());
    assert_eq!(bad.validate(), Err(TilemapError::DuplicateLayer));
    for scalar in [0., f64::NAN, f64::INFINITY, -1., 1_000_001.] {
        let mut bad = valid.clone();
        bad.cell_size[0] = scalar;
        assert_eq!(bad.validate(), Err(TilemapError::CellSize));
    }
    let mut bad = map([2, 1], vec![None; 2]);
    bad.cell_size[0] = 1_000_000.;
    assert_eq!(bad.validate(), Err(TilemapError::CellSize));
    let mut document = Tilemap {
        tileset: "00000000000000000000000001".to_owned(),
        size: [1, 1],
        cell_size: [1.; 2],
        layers: vec![TilemapLayer {
            id: "00000000000000000000000002".into(),
            cells: vec![Some("00000000000000000000000003".into())],
            layer: 0,
            order: 0,
            solid: false,
        }],
    };
    document.validate_document().unwrap();
    document.layers[0].cells[0] = Some("not-an-id".into());
    assert_eq!(document.validate_document(), Err(TilemapError::Identity));
}
#[test]
fn maximum_cell_storage_and_checkerboard_rectangles_stay_within_caps() {
    let cells = (0..MAX_CELL_SLOTS)
        .map(|i| ((i % 512 + i / 512) % 2 == 0).then_some(1))
        .collect();
    let mut value = map([512, 512], cells);
    assert_eq!(value.cell_slots(), MAX_CELL_SLOTS);
    let prepared = value.prepare().unwrap();
    assert_eq!(prepared.placements().len(), MAX_CELL_SLOTS / 2);
    assert_eq!(prepared.collision().len(), MAX_CELL_SLOTS / 2);
    value.layers.push(TilemapLayer {
        id: 2,
        cells: vec![None; MAX_CELL_SLOTS],
        layer: 0,
        order: 0,
        solid: false,
    });
    assert_eq!(value.validate(), Err(TilemapError::Bounds));
}
