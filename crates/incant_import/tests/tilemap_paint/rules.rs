use super::support::{Fixture, frame, rules};
use incant_cmd::Actor;
use incant_import::{TilemapBrush as B, TilemapTerrainAction as A};

#[test]
fn every_cardinal_mask_uses_local_north_east_south_west_without_diagonals() {
    let mut f = Fixture::new([3, 3]);
    let original = f.map();
    for mask in 0..16 {
        let mut cells = vec![None; 9];
        cells[4] = Some(frame(0));
        for (bit, index) in [7, 5, 1, 3].into_iter().enumerate() {
            if mask & (1 << bit) != 0 {
                cells[index] = Some(frame(0));
            }
        }
        // Diagonals never contribute; a foreign frame also never joins.
        for i in [0, 2, 6, 8] {
            cells[i] = Some(frame(16));
        }
        f.set(cells);
        let p = f
            .prepare(&f.request(
                vec![[1, 1]],
                B::Autotile {
                    action: A::Paint,
                    frames: rules(),
                },
            ))
            .unwrap();
        p.commit(&mut f.bus, Actor::user("test")).unwrap();
        let map = f.map();
        assert_eq!(map.layers[0].cells[4], Some(frame(mask)));
        assert_eq!(map.layers[1], original.layers[1]);
        assert_eq!(map.cell_size, original.cell_size);
        assert_eq!(map.size, original.size);
        assert_eq!(map.layers[0].solid, original.layers[0].solid);
        assert_eq!(map.layers[0].layer, original.layers[0].layer);
        assert_eq!(map.layers[0].order, original.layers[0].order);
    }
}

#[test]
fn simultaneous_stroke_order_is_irrelevant_and_erase_retiles_neighbors() {
    let f = Fixture::new([3, 1]);
    let a = f.request(
        vec![[0, 0], [1, 0], [2, 0]],
        B::Autotile {
            action: A::Paint,
            frames: rules(),
        },
    );
    let mut b = a.clone();
    b.cells.reverse();
    let first = f.prepare(&a).unwrap();
    let second = f.prepare(&b).unwrap();
    assert_eq!(first.commands(), second.commands());
    assert_eq!(first.preview().changes, second.preview().changes);
    let mut f = f;
    first.commit(&mut f.bus, Actor::user("test")).unwrap();
    assert_eq!(
        f.map().layers[0].cells,
        vec![Some(frame(2)), Some(frame(10)), Some(frame(8))]
    );
    let erase = f
        .prepare(&f.request(
            vec![[1, 0]],
            B::Autotile {
                action: A::Erase,
                frames: rules(),
            },
        ))
        .unwrap();
    assert_eq!(erase.preview().changes.len(), 3);
    erase.commit(&mut f.bus, Actor::user("test")).unwrap();
    assert_eq!(
        f.map().layers[0].cells,
        vec![Some(frame(0)), None, Some(frame(0))]
    );
}

#[test]
fn unchanged_selection_repairs_neighbors_and_rules_do_not_link_later_plain_edits() {
    let mut f = Fixture::new([3, 1]);
    f.set(vec![Some(frame(2)), Some(frame(0)), Some(frame(16))]);
    let p = f
        .prepare(&f.request(
            vec![[0, 0]],
            B::Autotile {
                action: A::Paint,
                frames: rules(),
            },
        ))
        .unwrap();
    assert_eq!(p.preview().changes.len(), 1);
    assert_eq!(p.preview().changes[0].cell, [1, 0]);
    p.commit(&mut f.bus, Actor::user("test")).unwrap();
    assert_eq!(
        f.map().layers[0].cells,
        vec![Some(frame(2)), Some(frame(8)), Some(frame(16))]
    );
    let p = f.prepare(&f.request(vec![[0, 0]], B::Erase)).unwrap();
    p.commit(&mut f.bus, Actor::user("test")).unwrap();
    assert_eq!(
        f.map().layers[0].cells[1],
        Some(frame(8)),
        "plain erase does not retain/apply prior rules"
    );
}

#[test]
fn boundaries_do_not_wrap_and_duplicate_rule_outputs_define_a_single_family() {
    let mut f = Fixture::new([2, 2]);
    let p = f
        .prepare(&f.request(
            vec![[1, 0], [0, 1]],
            B::Autotile {
                action: A::Paint,
                frames: rules(),
            },
        ))
        .unwrap();
    p.commit(&mut f.bus, Actor::user("test")).unwrap();
    assert_eq!(
        f.map().layers[0].cells,
        vec![None, Some(frame(0)), Some(frame(0)), None]
    );
    let p = f
        .prepare(&f.request(
            vec![[0, 0], [1, 0], [0, 1], [1, 1]],
            B::Autotile {
                action: A::Paint,
                frames: Box::new(std::array::from_fn(|_| frame(3))),
            },
        ))
        .unwrap();
    p.commit(&mut f.bus, Actor::user("test")).unwrap();
    assert!(f.map().layers[0].cells.iter().all(|c| *c == Some(frame(3))));
}

#[test]
fn erasing_empty_cell_repairs_stale_cardinal_neighbors_and_preserves_foreign_cells() {
    let mut f = Fixture::new([3, 1]);
    f.set(vec![Some(frame(2)), None, Some(frame(16))]);
    let prepared = f
        .prepare(&f.request(
            vec![[1, 0]],
            B::Autotile {
                action: A::Erase,
                frames: rules(),
            },
        ))
        .unwrap();
    assert_eq!(prepared.preview().changes.len(), 1);
    assert_eq!(prepared.preview().changes[0].cell, [0, 0]);
    prepared.commit(&mut f.bus, Actor::user("test")).unwrap();
    assert_eq!(
        f.map().layers[0].cells,
        vec![Some(frame(0)), None, Some(frame(16))]
    );
}
