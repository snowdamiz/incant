#[path = "support/scene_format.rs"]
mod scene_format;
use incant_runtime::*;
use incant_types::{Transform, Velocity};
use sha2::{Digest, Sha256};
use std::sync::Arc;
fn id(n: u8) -> StableId {
    StableId([n; 16])
}
fn entity(n: u8) -> CookedEntity {
    CookedEntity {
        id: id(n),
        parent: None,
        transform: Transform::default(),
        velocity: None,
    }
}
fn text(s: &str) -> Arc<NativeText> {
    Arc::new(NativeText {
        content: Arc::from(s),
        fonts: Arc::from([id(9)]),
        layout: TextLayout {
            font_size: 18.,
            line_height: 24.,
            width: 200.,
            wrap: TextWrap::WordOrGlyph,
        },
    })
}
fn scene() -> CookedScene {
    CookedScene::new(id(0), 60, vec![entity(1), entity(2)])
        .unwrap()
        .with_text(
            vec![FontAssetRef {
                id: id(9),
                fingerprint: [7; 32],
            }],
            vec![CookedText {
                entity: id(1),
                text: text("Hello שלום 世界"),
            }],
        )
        .unwrap()
}
fn apply(w: &mut NativeWorld, commands: Vec<StructuralCommand>) -> Result<(), RuntimeError> {
    let mut c = FrameCommands::default();
    for x in commands {
        c.push(x).unwrap();
    }
    w.apply(&mut c)
}
fn content(w: &NativeWorld) -> String {
    w.text_view()
        .unwrap()
        .nodes()
        .next()
        .unwrap()
        .1
        .content
        .to_string()
}
fn refresh(mut bytes: Vec<u8>) -> Vec<u8> {
    let len = bytes.len() - 72;
    bytes[36..40].copy_from_slice(&(len as u32).to_le_bytes());
    let mut hash = Sha256::new();
    hash.update(&bytes[..40]);
    hash.update(&bytes[72..]);
    bytes[40..72].copy_from_slice(&hash.finalize());
    bytes
}
#[test]
fn scene_v12_roundtrip_preserves_chain_utf8_and_old_static_bytes() {
    let plain = CookedScene::new(id(0), 60, vec![entity(1), entity(2)]).unwrap();
    let old = plain.to_bytes();
    assert_eq!(
        u32::from_le_bytes(old[8..12].try_into().unwrap()),
        crate::scene_format::scene_format(1)
    );
    assert_eq!(plain.with_text(vec![], vec![]).unwrap().to_bytes(), old);
    let s = scene();
    let bytes = s.to_bytes();
    assert_eq!(
        u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
        crate::scene_format::scene_format(12)
    );
    let loaded = CookedScene::from_bytes(&bytes).unwrap();
    assert_eq!(loaded.to_bytes(), bytes);
    assert_eq!(loaded.texts(), s.texts());
    assert_eq!(loaded.font_assets(), s.font_assets());
}
#[test]
fn text_retains_arcs_through_motion_relocation_noop_and_reparent() {
    let s = scene();
    let original = s.texts()[0].text.clone();
    let mut w = NativeWorld::new(s);
    apply(
        &mut w,
        vec![StructuralCommand::SetVelocity {
            entity: id(1),
            velocity: Some(Velocity {
                linear: [1., 0., 0.],
            }),
        }],
    )
    .unwrap();
    for _ in 0..10 {
        w.step().unwrap();
    }
    let ptr = w.text_view().unwrap().nodes().next().unwrap().1 as *const NativeText;
    assert_eq!(ptr, Arc::as_ptr(&original));
    apply(
        &mut w,
        vec![StructuralCommand::SetText {
            entity: id(1),
            text: Some(text("Hello שלום 世界")),
        }],
    )
    .unwrap();
    assert_eq!(w.text_view().unwrap().generation(), 0);
    assert_eq!(
        w.text_view().unwrap().nodes().next().unwrap().1 as *const _,
        ptr
    );
    apply(
        &mut w,
        vec![StructuralCommand::SetParent {
            entity: id(1),
            parent: Some(id(2)),
        }],
    )
    .unwrap();
    assert_eq!(
        w.text_view().unwrap().nodes().next().unwrap().1 as *const _,
        ptr
    );
    assert_eq!(w.text_view().unwrap().generation(), 0);
}
#[test]
fn text_lifetime_changes_are_final_state_atomic_and_do_not_reuse_despawned_text() {
    let mut w = NativeWorld::new(scene());
    let before = w.save_bytes().unwrap();
    let mut invalid = (*text("bad")).clone();
    invalid.fonts = Arc::from([id(8)]);
    assert!(
        apply(
            &mut w,
            vec![
                StructuralCommand::SetVelocity {
                    entity: id(1),
                    velocity: Some(Velocity {
                        linear: [9., 0., 0.]
                    })
                },
                StructuralCommand::SetText {
                    entity: id(1),
                    text: Some(Arc::new(invalid))
                }
            ]
        )
        .is_err()
    );
    assert_eq!(w.save_bytes().unwrap(), before);
    apply(
        &mut w,
        vec![StructuralCommand::SetText {
            entity: id(1),
            text: Some(text("new")),
        }],
    )
    .unwrap();
    assert_eq!(content(&w), "new");
    assert_eq!(w.text_view().unwrap().generation(), 1);
    apply(&mut w, vec![StructuralCommand::Despawn(id(1))]).unwrap();
    assert_eq!(w.text_view().unwrap().nodes().len(), 0);
    assert_eq!(w.text_view().unwrap().generation(), 2);
    apply(&mut w, vec![StructuralCommand::Spawn(entity(1))]).unwrap();
    assert_eq!(w.text_view().unwrap().nodes().len(), 0);
    apply(
        &mut w,
        vec![StructuralCommand::SetText {
            entity: id(1),
            text: Some(text("replacement")),
        }],
    )
    .unwrap();
    assert_eq!(content(&w), "replacement");
    assert!(apply(&mut w, vec![StructuralCommand::Spawn(entity(9))]).is_err());
}
#[test]
fn text_save_restore_keeps_origin_font_catalog_and_invalidates_retained_consumers() {
    let initial = scene();
    let mut w = NativeWorld::new(initial.clone());
    let identity = w.text_view().unwrap().identity().clone();
    let old = w.save_bytes().unwrap();
    apply(
        &mut w,
        vec![StructuralCommand::SetText {
            entity: id(1),
            text: Some(text("changed")),
        }],
    )
    .unwrap();
    w.step().unwrap();
    let current = w.save_bytes().unwrap();
    let fresh = NativeWorld::from_save_bytes_for_scene(&current, &initial).unwrap();
    assert_eq!(content(&fresh), "changed");
    assert_eq!(fresh.tick(), 1);
    w.restore_bytes(&old).unwrap();
    assert_eq!(content(&w), "Hello שלום 世界");
    assert_eq!(w.text_view().unwrap().generation(), 2);
    assert_eq!(w.text_view().unwrap().identity(), &identity);
    let other = initial
        .with_text(
            vec![FontAssetRef {
                id: id(9),
                fingerprint: [8; 32],
            }],
            vec![CookedText {
                entity: id(1),
                text: text("Hello שלום 世界"),
            }],
        )
        .unwrap();
    assert!(NativeWorld::from_save_bytes_for_scene(&current, &other).is_err());
    let mut foreign = NativeWorld::new(other);
    let before = foreign.save_bytes().unwrap();
    assert!(foreign.restore_bytes(&current).is_err());
    assert_eq!(foreign.save_bytes().unwrap(), before);
}
#[test]
fn text_binary_refuses_checksum_valid_bad_counts_utf8_metrics_tags_and_references() {
    let bytes = scene().to_bytes();
    let suffix = bytes.windows(32).rposition(|w| w == [7; 32]).unwrap() - 20;
    // Font count, font ID, fingerprint, text count, entity, content length.
    assert_eq!(
        u32::from_le_bytes(bytes[suffix..suffix + 4].try_into().unwrap()),
        1
    );
    let text_start = suffix + 4 + 48 + 4;
    let len = usize::try_from(u32::from_le_bytes(
        bytes[text_start + 16..text_start + 20].try_into().unwrap(),
    ))
    .unwrap();
    let chain = text_start + 20 + len;
    let metrics = chain + 1 + 16;
    for (offset, data) in [
        (suffix, u32::MAX.to_le_bytes().to_vec()),
        (text_start + 16, u32::MAX.to_le_bytes().to_vec()),
        (text_start + 20, vec![0xff]),
        (chain, vec![33]),
        (chain + 1, id(8).0.to_vec()),
        (metrics, f32::NAN.to_le_bytes().to_vec()),
        (metrics + 12, vec![3]),
    ] {
        let mut corrupt = bytes.clone();
        corrupt[offset..offset + data.len()].copy_from_slice(&data);
        assert!(
            CookedScene::from_bytes(&refresh(corrupt)).is_err(),
            "offset {offset}"
        );
    }
    for n in 1..=20 {
        assert!(CookedScene::from_bytes(&refresh(bytes[..bytes.len() - n].to_vec())).is_err());
    }
}
#[test]
fn scene_limits_and_duplicate_bindings_are_refused() {
    let plain = || CookedScene::new(id(0), 60, vec![entity(1)]).unwrap();
    let font = FontAssetRef {
        id: id(9),
        fingerprint: [7; 32],
    };
    assert!(plain().with_text(vec![font, font], vec![]).is_err());
    assert!(
        plain()
            .with_text(vec![FontAssetRef { id: id(1), ..font }], vec![])
            .is_err()
    );
    assert!(
        plain()
            .with_text(
                vec![font],
                vec![CookedText {
                    entity: id(2),
                    text: text("x")
                }]
            )
            .is_err()
    );
    assert!(
        plain()
            .with_text(
                vec![font],
                vec![CookedText {
                    entity: id(1),
                    text: text(&"x".repeat(MAX_TEXT_BYTES + 1))
                }]
            )
            .is_err()
    );
    let mut bad = (*text("x")).clone();
    bad.fonts = Arc::from([id(9), id(9)]);
    assert!(bad.validate().is_err());
}

#[test]
fn checksum_valid_save_catalog_substitution_does_not_publish() {
    let initial = scene();
    let mut w = NativeWorld::new(initial.clone());
    let before = w.save_bytes().unwrap();
    let mut changed = before.clone();
    let scene_start = 104;
    let mut bytes = changed[scene_start..].to_vec();
    let fingerprint = bytes.windows(32).rposition(|w| w == [7; 32]).unwrap();
    bytes[fingerprint] = 8;
    let bytes = refresh(bytes);
    changed[scene_start..].copy_from_slice(&bytes);
    let mut hash = Sha256::new();
    hash.update(&changed[..32]);
    hash.update(&changed[64..]);
    changed[32..64].copy_from_slice(&hash.finalize());
    assert!(NativeWorld::from_save_bytes_for_scene(&changed, &initial).is_err());
    assert!(w.restore_bytes(&changed).is_err());
    assert_eq!(w.save_bytes().unwrap(), before);
}
#[test]
fn aggregate_text_bytes_are_admitted_at_scene_and_atomic_command_boundaries() {
    let entities: Vec<_> = (0u128..65)
        .map(|n| CookedEntity {
            id: StableId(n.to_be_bytes()),
            parent: None,
            transform: Default::default(),
            velocity: None,
        })
        .collect();
    let font = FontAssetRef {
        id: id(9),
        fingerprint: [7; 32],
    };
    let plain = CookedScene::new(id(255), 60, entities.clone()).unwrap();
    let full = text(&"x".repeat(MAX_TEXT_BYTES));
    let nodes = |count| {
        entities
            .iter()
            .take(count)
            .map(|e| CookedText {
                entity: e.id,
                text: full.clone(),
            })
            .collect()
    };
    let accepted = plain.clone().with_text(vec![font], nodes(64)).unwrap();
    assert!(plain.with_text(vec![font], nodes(65)).is_err());
    let mut w = NativeWorld::new(accepted);
    let before = w.save_bytes().unwrap();
    assert!(
        apply(
            &mut w,
            vec![StructuralCommand::SetText {
                entity: entities[64].id,
                text: Some(full)
            }]
        )
        .is_err()
    );
    assert_eq!(w.save_bytes().unwrap(), before);
}
