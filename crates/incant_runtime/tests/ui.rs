use incant_runtime::*;
use incant_types::{Transform, UiKind, UiNode, UiStyle, UiText, Velocity};
use sha2::{Digest, Sha256};
use std::sync::Arc;
fn id(n: u8) -> StableId {
    StableId([n; 16])
}
fn entity(n: u8, parent: Option<u8>) -> CookedEntity {
    CookedEntity {
        id: id(n),
        parent: parent.map(id),
        transform: Default::default(),
        velocity: None,
    }
}
fn node(kind: UiKind<StableId>) -> Arc<NativeUi> {
    Arc::new(UiNode {
        kind,
        style: UiStyle::default(),
        order: 0,
        disabled: false,
    })
}
fn text() -> UiText<StableId> {
    UiText {
        content: "Name שלום 中文".into(),
        fonts: vec![id(9)],
        font_size: 16.0,
        line_height: 20.0,
        wrap: TextWrap::WordOrGlyph,
    }
}
fn declarations() -> Vec<CookedUi> {
    vec![
        CookedUi {
            entity: id(1),
            node: node(UiKind::Root),
        },
        CookedUi {
            entity: id(2),
            node: node(UiKind::Container),
        },
        CookedUi {
            entity: id(3),
            node: node(UiKind::TextInput { text: text() }),
        },
    ]
}
fn scene() -> CookedScene {
    CookedScene::new(
        id(0),
        60,
        vec![
            entity(1, None),
            entity(2, Some(1)),
            entity(3, Some(1)),
            entity(4, None),
        ],
    )
    .unwrap()
    .with_text(
        vec![FontAssetRef {
            id: id(9),
            fingerprint: [7; 32],
        }],
        vec![],
    )
    .unwrap()
    .with_ui(declarations())
    .unwrap()
}
fn apply(w: &mut NativeWorld, c: Vec<StructuralCommand>) -> Result<(), RuntimeError> {
    let mut cmds = FrameCommands::default();
    for c in c {
        cmds.push(c).unwrap();
    }
    w.apply(&mut cmds)
}
fn checksum(mut bytes: Vec<u8>) -> Vec<u8> {
    let n = bytes.len() - 72;
    bytes[36..40].copy_from_slice(&(n as u32).to_le_bytes());
    let mut h = Sha256::new();
    h.update(&bytes[..40]);
    h.update(&bytes[72..]);
    bytes[40..72].copy_from_slice(&h.finalize());
    bytes
}
#[test]
fn actual_v13_roundtrip_and_legacy_emission_are_exact() {
    let s = scene();
    let bytes = s.to_bytes();
    assert_eq!(&bytes[8..12], &13_u32.to_le_bytes());
    let loaded = CookedScene::from_bytes(&bytes).unwrap();
    assert_eq!(loaded.ui(), s.ui());
    assert_eq!(loaded.to_bytes(), bytes);
    let old = CookedScene::new(id(0), 60, vec![entity(1, None)]).unwrap();
    let bytes = old.to_bytes();
    assert_eq!(old.with_ui(vec![]).unwrap().to_bytes(), bytes);
    let only = CookedScene::new(id(0), 60, vec![entity(1, None)])
        .unwrap()
        .with_ui(vec![CookedUi {
            entity: id(1),
            node: node(UiKind::Root),
        }])
        .unwrap()
        .to_bytes();
    assert_eq!(CookedScene::from_bytes(&only).unwrap().to_bytes(), only);
}
#[test]
fn malformed_ui_suffix_and_recomputed_checksum_refuse() {
    let s = scene();
    let bytes = s.to_bytes();
    let suffix = bytes.len() - (4 + 3 * 91 + 14 + text().content.len() + 16);
    assert_eq!(&bytes[suffix..suffix + 4], &3u32.to_le_bytes());
    for (offset, value) in [
        (suffix, 255),
        (suffix + 4 + 16, 8),
        (suffix + 4 + 17, 2),
        (suffix + 4 + 22, 9),
        (suffix + 4 + 16 + 1 + 1 + 4 + 10 + 8, 2),
    ] {
        let mut b = bytes.clone();
        b[offset] = value;
        assert!(
            CookedScene::from_bytes(&checksum(b)).is_err(),
            "offset {offset}"
        );
    }
    let mut duplicate = bytes.clone();
    duplicate[suffix + 4 + 91..suffix + 4 + 91 + 16].copy_from_slice(&id(1).0);
    assert!(CookedScene::from_bytes(&checksum(duplicate)).is_err());
    for n in 1..50 {
        assert!(CookedScene::from_bytes(&checksum(bytes[..bytes.len() - n].to_vec())).is_err());
    }
    let mut nan = bytes.clone();
    nan[suffix + 4 + 22 + 1..suffix + 4 + 22 + 5].copy_from_slice(&f32::NAN.to_le_bytes());
    assert!(CookedScene::from_bytes(&checksum(nan)).is_err());
}
#[test]
fn structural_reparent_despawn_and_final_configuration_are_atomic() {
    let mut w = NativeWorld::new(scene());
    let original = Arc::clone(
        w.ui_view()
            .unwrap()
            .nodes()
            .find(|n| n.id == id(3))
            .unwrap()
            .node,
    );
    let before = w.snapshot();
    assert!(
        apply(
            &mut w,
            vec![
                StructuralCommand::SetVelocity {
                    entity: id(4),
                    velocity: Some(Velocity {
                        linear: [8.0, 0.0, 0.0]
                    })
                },
                StructuralCommand::Despawn(id(1))
            ]
        )
        .is_err()
    );
    assert_eq!(w.snapshot(), before);
    assert_eq!(w.ui_view().unwrap().generation(), 0);
    assert!(
        apply(
            &mut w,
            vec![StructuralCommand::SetParent {
                entity: id(3),
                parent: Some(id(4))
            }]
        )
        .is_err()
    );
    assert!(
        apply(
            &mut w,
            vec![StructuralCommand::SetParent {
                entity: id(4),
                parent: Some(id(1))
            }]
        )
        .is_err()
    );
    apply(
        &mut w,
        vec![StructuralCommand::SetParent {
            entity: id(3),
            parent: Some(id(2)),
        }],
    )
    .unwrap();
    assert_eq!(w.ui_view().unwrap().generation(), 1);
    assert_eq!(
        w.ui_view()
            .unwrap()
            .nodes()
            .find(|n| n.id == id(3))
            .unwrap()
            .parent,
        Some(id(2))
    );
    assert!(Arc::ptr_eq(
        &original,
        w.ui_view()
            .unwrap()
            .nodes()
            .find(|n| n.id == id(3))
            .unwrap()
            .node
    ));
    assert!(apply(&mut w, vec![StructuralCommand::Despawn(id(2))]).is_err());
    apply(
        &mut w,
        vec![
            StructuralCommand::Despawn(id(2)),
            StructuralCommand::SetParent {
                entity: id(3),
                parent: Some(id(1)),
            },
        ],
    )
    .unwrap();
    assert_eq!(w.ui_view().unwrap().generation(), 2);
    apply(
        &mut w,
        vec![StructuralCommand::SetUi {
            entity: id(3),
            node: Some(node(UiKind::TextInput { text: text() })),
        }],
    )
    .unwrap();
    assert_eq!(w.ui_view().unwrap().generation(), 2);
}
#[test]
fn native_lifetime_reuse_and_empty_ui_never_silently_enable_saving() {
    let mut w = NativeWorld::new(scene());
    let old = w
        .ui_view()
        .unwrap()
        .nodes()
        .find(|n| n.id == id(3))
        .unwrap()
        .incarnation;
    apply(&mut w, vec![StructuralCommand::Despawn(id(3))]).unwrap();
    apply(
        &mut w,
        vec![
            StructuralCommand::Spawn(entity(3, Some(1))),
            StructuralCommand::SetUi {
                entity: id(3),
                node: Some(node(UiKind::TextInput { text: text() })),
            },
        ],
    )
    .unwrap();
    assert_ne!(
        old,
        w.ui_view()
            .unwrap()
            .nodes()
            .find(|n| n.id == id(3))
            .unwrap()
            .incarnation
    );
    assert_eq!(w.save_bytes().unwrap_err(), SaveError::UiUnsupported);
    apply(
        &mut w,
        vec![
            StructuralCommand::Despawn(id(1)),
            StructuralCommand::Despawn(id(2)),
            StructuralCommand::Despawn(id(3)),
        ],
    )
    .unwrap();
    assert_eq!(w.ui_view().unwrap().nodes().len(), 0);
    assert_eq!(w.save_bytes().unwrap_err(), SaveError::UiUnsupported);
}
#[test]
fn world_components_and_numeric_ui_motion_refuse_without_partial_publication() {
    let s = scene();
    let mut w = NativeWorld::new(s);
    let before = w.snapshot();
    assert!(
        apply(
            &mut w,
            vec![StructuralCommand::SetVelocity {
                entity: id(3),
                velocity: Some(Velocity { linear: [0.0; 3] })
            }]
        )
        .is_err()
    );
    assert_eq!(w.snapshot(), before);
    let bad = CookedScene::new(
        id(0),
        60,
        vec![
            entity(1, None),
            CookedEntity {
                transform: Transform {
                    translation: [1.0, 0.0, 0.0],
                    ..Default::default()
                },
                ..entity(2, Some(1))
            },
        ],
    )
    .unwrap();
    assert!(
        bad.with_ui(vec![
            CookedUi {
                entity: id(1),
                node: node(UiKind::Root)
            },
            CookedUi {
                entity: id(2),
                node: node(UiKind::Container)
            }
        ])
        .is_err()
    );
    // A component can be removed and a replacement UI declaration installed at
    // one sync point, but the final native graph is still the authority.
    apply(
        &mut w,
        vec![StructuralCommand::SetVelocity {
            entity: id(4),
            velocity: Some(Velocity {
                linear: [1.0, 0.0, 0.0],
            }),
        }],
    )
    .unwrap();
    apply(
        &mut w,
        vec![
            StructuralCommand::SetVelocity {
                entity: id(4),
                velocity: None,
            },
            StructuralCommand::SetParent {
                entity: id(4),
                parent: Some(id(1)),
            },
            StructuralCommand::SetUi {
                entity: id(4),
                node: Some(node(UiKind::Container)),
            },
        ],
    )
    .unwrap();
    assert_eq!(w.ui_view().unwrap().nodes().len(), 4);
    w.step().unwrap();
}
