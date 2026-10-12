use incant_ui::*;
use std::sync::Arc;

fn id(n: u16) -> StableId {
    StableId(u128::from(n).to_be_bytes())
}
fn widget(n: u16, parent: Option<u16>, kind: UiKind<StableId>, style: UiStyle) -> Widget {
    Widget {
        incarnation: 1,
        id: id(n),
        parent: parent.map(id),
        node: Arc::new(UiNode {
            kind,
            style,
            order: 0,
            disabled: false,
        }),
    }
}
fn root(style: UiStyle) -> Widget {
    widget(1, None, UiKind::Root, style)
}
fn container(n: u16, style: UiStyle) -> Widget {
    widget(n, Some(1), UiKind::Container, style)
}
struct NoText;
impl TextMeasure for NoText {
    fn measure(
        &mut self,
        _: &Widget,
        _: &str,
        _: [Option<f32>; 2],
        _: [MeasureSpace; 2],
    ) -> Result<[f32; 2], UiError> {
        panic!("no text in this layout")
    }
}
fn fixed(w: f32, h: f32) -> UiStyle {
    UiStyle {
        size: [UiLength::Pixels(w), UiLength::Pixels(h)],
        shrink: 0.0,
        ..Default::default()
    }
}
fn node(frame: &LayoutFrame, n: u16) -> WidgetLayout {
    *frame.nodes.iter().find(|node| node.id == id(n)).unwrap()
}
#[test]
fn actual_flex_growth_padding_gap_and_viewport_resize() {
    let mut engine = LayoutEngine::new(vec![
        container(
            3,
            UiStyle {
                grow: 1.0,
                basis: UiLength::Pixels(0.0),
                ..fixed(0.0, 20.0)
            },
        ),
        root(UiStyle {
            direction: UiDirection::Row,
            padding: [10.0; 4],
            gap: [10.0; 2],
            ..Default::default()
        }),
        container(2, fixed(40.0, 20.0)),
    ])
    .unwrap();
    let candidate = engine.prepare([300.0, 100.0], 0, &mut NoText).unwrap();
    assert!(engine.frame().is_none());
    assert_eq!(
        node(candidate.frame(), 2).bounds,
        Rect {
            origin: [10.0, 10.0],
            size: [40.0, 20.0]
        }
    );
    assert_eq!(
        node(candidate.frame(), 3).bounds,
        Rect {
            origin: [60.0, 10.0],
            size: [230.0, 20.0]
        }
    );
    engine.commit(candidate).unwrap();
    let first = Arc::clone(engine.frame().unwrap());
    let unchanged = engine.prepare([300.0, 100.0], 0, &mut NoText).unwrap();
    assert!(Arc::ptr_eq(&first, unchanged.frame()));
    engine.commit(unchanged).unwrap();
    let resized = engine.prepare([200.0, 100.0], 0, &mut NoText).unwrap();
    assert_eq!(node(resized.frame(), 3).bounds.size, [130.0, 20.0]);
    assert_eq!(node(&first, 3).bounds.size, [230.0, 20.0]);
    engine.commit(resized).unwrap();
    assert_eq!(engine.frame().unwrap().generation, 2);
}
#[test]
fn wrap_order_and_clipping_are_independent_of_entity_insertion() {
    let mut a = container(2, fixed(70.0, 20.0));
    Arc::make_mut(&mut a.node).order = 2;
    let tree = LayoutEngine::new(vec![
        a,
        container(3, fixed(70.0, 20.0)),
        root(UiStyle {
            direction: UiDirection::Row,
            wrap: true,
            padding: [10.0; 4],
            gap: [10.0, 8.0],
            ..Default::default()
        }),
        container(4, fixed(70.0, 20.0)),
    ])
    .unwrap();
    let frame = tree.prepare([200.0, 50.0], 0, &mut NoText).unwrap();
    assert_eq!(
        frame.frame().nodes.iter().map(|n| n.id).collect::<Vec<_>>(),
        [id(1), id(3), id(4), id(2)]
    );
    assert_eq!(node(frame.frame(), 3).bounds.origin, [10.0, 10.0]);
    assert_eq!(node(frame.frame(), 4).bounds.origin, [90.0, 10.0]);
    assert_eq!(node(frame.frame(), 2).bounds.origin, [10.0, 38.0]);
    assert_eq!(node(frame.frame(), 2).clip.size, [70.0, 2.0]);
}
#[test]
fn failed_measure_or_bad_viewport_preserves_accepted_layout() {
    struct Measure(bool);
    impl TextMeasure for Measure {
        fn measure(
            &mut self,
            _: &Widget,
            _: &str,
            _: [Option<f32>; 2],
            _: [MeasureSpace; 2],
        ) -> Result<[f32; 2], UiError> {
            if self.0 {
                Err(UiError::Limit("fixture shaped output"))
            } else {
                Ok([24.0, 12.0])
            }
        }
    }
    let text = UiText {
        content: "abc".into(),
        fonts: vec![id(9)],
        font_size: 12.0,
        line_height: 12.0,
        wrap: incant_types::TextWrap::Word,
    };
    let mut tree = LayoutEngine::new(vec![
        root(Default::default()),
        widget(2, Some(1), UiKind::Label { text }, Default::default()),
    ])
    .unwrap();
    let first = tree
        .prepare([100.0, 100.0], 0, &mut Measure(false))
        .unwrap();
    tree.commit(first).unwrap();
    let first = Arc::clone(tree.frame().unwrap());
    assert!(tree.prepare([100.0, 100.0], 1, &mut Measure(true)).is_err());
    assert!(
        tree.prepare([f32::NAN, 100.0], 0, &mut Measure(false))
            .is_err()
    );
    assert!(Arc::ptr_eq(&first, tree.frame().unwrap()));
    assert_eq!(tree.frame().unwrap().generation, 1);
    let next = tree
        .prepare([100.0, 100.0], 1, &mut Measure(false))
        .unwrap();
    tree.commit(next).unwrap();
    assert_eq!(tree.frame().unwrap().generation, 2);
}
#[test]
fn stale_candidate_and_wrong_owner_never_replace_live_layout() {
    let mut a = LayoutEngine::new(vec![root(Default::default())]).unwrap();
    let mut b = LayoutEngine::new(vec![root(Default::default())]).unwrap();
    let stale = a.prepare([80.0, 50.0], 0, &mut NoText).unwrap();
    let accepted = a.prepare([100.0, 50.0], 0, &mut NoText).unwrap();
    a.commit(accepted).unwrap();
    assert!(matches!(a.commit(stale), Err(UiError::Stale)));
    let foreign = a.prepare([100.0, 50.0], 0, &mut NoText).unwrap();
    assert!(matches!(b.commit(foreign), Err(UiError::Stale)));
    assert!(b.frame().is_none());
    assert_eq!(a.frame().unwrap().viewport, [100.0, 50.0]);
}
#[test]
fn final_hierarchy_validation_refuses_removed_reparented_or_cyclic_links() {
    let child = container(2, Default::default());
    assert!(LayoutEngine::new(vec![child.clone()]).is_err());
    let mut wrong = child.clone();
    wrong.parent = Some(id(99));
    assert!(LayoutEngine::new(vec![root(Default::default()), wrong]).is_err());
    let mut cycle = child.clone();
    cycle.parent = Some(id(3));
    let mut other = container(3, Default::default());
    other.parent = Some(id(2));
    assert!(LayoutEngine::new(vec![root(Default::default()), cycle, other]).is_err());
    assert!(LayoutEngine::new(vec![root(Default::default()), child.clone(), child]).is_err());
    let mut too_deep = vec![root(Default::default())];
    for n in 2..=65 {
        let mut w = container(n, Default::default());
        w.parent = Some(id(n - 1));
        too_deep.push(w);
    }
    assert!(LayoutEngine::new(too_deep).is_err());
    let mut bad_root = root(Default::default());
    Arc::make_mut(&mut bad_root.node).style.size[0] = UiLength::Pixels(1.0);
    assert!(LayoutEngine::new(vec![bad_root]).is_err());
}
