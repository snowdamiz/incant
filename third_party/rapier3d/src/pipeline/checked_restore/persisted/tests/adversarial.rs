use super::*;
use alloc::vec::Vec;
use incant_physics_snapshot::*;
// Editable inert sections only. No hostile record is ever serde-deserialized
// into live native storage; every native candidate uses the checked importer.
pub(super) fn section(bytes: &[u8], tag: usize, replacement: &[u8]) -> Vec<u8> {
    let header = 16 + 224;
    assert_eq!(&bytes[..8], b"ICNATIV1");
    let mut out = bytes[..header].to_vec();
    let mut cursor = header;
    for i in 0..22 {
        let offset = 16 + 48 + i * 8 + 4;
        let len = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        if i == tag {
            out[offset..offset + 4].copy_from_slice(&(replacement.len() as u32).to_le_bytes());
            out.extend_from_slice(replacement);
        } else {
            out.extend_from_slice(&bytes[cursor..cursor + len]);
        }
        cursor += len;
    }
    let len = out.len() - 16;
    out[8..12].copy_from_slice(&(len as u32).to_le_bytes());
    out
}
fn edit_manifolds(frame: &NativeFrame, change: impl FnOnce(&mut ManifoldTable)) -> Vec<u8> {
    let replacement = with_decoded_world_boundary(
        &frame.encoded()[16..],
        frame.identity(),
        &mut payload(),
        |f| {
            let cs = f.broad().context().narrow.context().rigid.collider_state();
            let mut record = f.manifolds().record().clone();
            change(&mut record);
            let checked = validate_manifolds(
                record,
                ManifoldContext {
                    graph: f.contact_graph(),
                    attachments: f.attachments(),
                    geometries: cs.context().geometries,
                    journals: PendingJournals::default(),
                },
                &mut payload(),
            )?;
            encode_manifolds(&checked, &mut payload())
        },
    )
    .unwrap();
    section(frame.encoded(), 8, &replacement)
}
fn refusal(bytes: &[u8]) -> PersistedError {
    CheckpointOwner::from_persisted(bytes, &mut payload(), &mut work(), &mut allocation())
        .err()
        .expect("hostile byte admission")
}
#[test]
fn persisted_native_consumed_feature_material_dominance_and_warmstart_refusals() {
    let mut owner = boxes();
    let frame = capture(&mut owner);
    let valid = frame.persisted_bytes(&mut payload(), &mut work()).unwrap();
    let bytes = edit_manifolds(&frame, |m| {
        assert!(!m.points.is_empty());
        m.points[0].features[0] = 0xc0010101;
    });
    assert_eq!(
        refusal(&bytes),
        PersistedError::Domain(DomainError::Feature)
    );
    let bytes = edit_manifolds(&frame, |m| {
        let row = m
            .manifolds
            .iter_mut()
            .find(|m| m.solver.count != 0)
            .unwrap();
        row.friction = 0.1234f32.to_bits();
    });
    assert_eq!(
        refusal(&bytes),
        PersistedError::Domain(DomainError::Material)
    );
    let bytes = edit_manifolds(&frame, |m| {
        let row = m
            .manifolds
            .iter_mut()
            .find(|m| m.solver.count != 0)
            .unwrap();
        row.relative_dominance = -row.relative_dominance;
    });
    // Existing graph joins can reject this before the additional dominance check.
    let e = refusal(&bytes);
    assert!(
        matches!(
            e,
            PersistedError::Record(_) | PersistedError::Domain(DomainError::Dominance)
        ),
        "{e:?}"
    );
    let bytes = edit_manifolds(&frame, |m| {
        m.points[0].data.warmstart_impulse = f32::MAX.to_bits();
    });
    assert_eq!(
        refusal(&bytes),
        PersistedError::Domain(DomainError::Arithmetic)
    );
    let bytes = edit_manifolds(&frame, |m| {
        m.points[0].data.warmstart_impulse = (-1f32).to_bits();
    });
    assert_eq!(
        refusal(&bytes),
        PersistedError::Domain(DomainError::Arithmetic)
    );
    assert_eq!(
        capture(&mut owner)
            .persisted_bytes(&mut payload(), &mut work())
            .unwrap(),
        valid
    );
}
#[test]
fn persisted_native_recycle_and_query_bound_refusals_do_not_rebuild_saved_state() {
    let mut owner = boxes();
    let frame = capture(&mut owner);
    let (pair_bytes, tree_bytes) = with_decoded_world_boundary(
        &frame.encoded()[16..],
        frame.identity(),
        &mut payload(),
        |f| {
            let rigid = f.broad().context().narrow.context().rigid;
            let mut pair = rigid.pairs().record().clone();
            let cache = pair
                .pairs
                .iter_mut()
                .find_map(|p| p.recycle.as_mut())
                .unwrap();
            cache.max_extent = (f32::from_bits(cache.max_extent) + 1.).to_bits();
            let pair = validate_pair_state(pair, f.manifolds(), &mut payload())?;
            let pair = encode_pair_state(&pair, &mut payload())?;
            let t = f.broad().context().tree;
            let mut r = t.record().clone();
            let leaf = r
                .nodes
                .iter_mut()
                .flat_map(|w| [&mut w.left, &mut w.right])
                .find(|n| n.leaf_count() == 1)
                .unwrap();
            leaf.mins = leaf.maxs;
            let tree = validate_pending_bvh(r, t.context(), &mut payload())?;
            Ok((pair, encode_pending_bvh(&tree, &mut payload())?))
        },
    )
    .unwrap();
    assert_eq!(
        refusal(&section(frame.encoded(), 9, &pair_bytes)),
        PersistedError::Domain(DomainError::Recycle)
    );
    assert_eq!(
        refusal(&section(frame.encoded(), 16, &tree_bytes)),
        PersistedError::Domain(DomainError::BroadBounds)
    );
    let mut bytes = frame.encoded().to_vec();
    bytes[16 + 12] ^= 1;
    assert!(matches!(
        refusal(&bytes),
        PersistedError::Record(Error::Identity)
    ));
}
#[test]
fn persisted_native_island_counter_exhaustion_refuses_before_construction_or_step() {
    let mut owner = boxes();
    let frame = capture(&mut owner);
    let replacement = with_decoded_world_boundary(
        &frame.encoded()[16..],
        frame.identity(),
        &mut payload(),
        |f| {
            let cs = f.broad().context().narrow.context().rigid.collider_state();
            let mut r = f
                .broad()
                .context()
                .narrow
                .context()
                .rigid
                .islands()
                .record()
                .clone();
            r.persistent
                .iter_mut()
                .flatten()
                .next()
                .unwrap()
                .constraint_remove_count = u32::MAX;
            let v = validate_islands(
                r,
                IslandContext {
                    bodies: cs.context().bodies,
                    manifolds: f.manifolds(),
                    domain: IslandDomain::default(),
                },
                &mut payload(),
            )?;
            encode_islands(&v, &mut payload())
        },
    )
    .unwrap();
    assert_eq!(
        refusal(&section(frame.encoded(), 10, &replacement)),
        PersistedError::Domain(DomainError::Counter)
    );
    let mut imported = CheckpointOwner::from_persisted(
        frame.encoded(),
        &mut payload(),
        &mut work(),
        &mut allocation(),
    )
    .unwrap();
    let w = imported.access_mut().unwrap();
    w.islands
        .persistent
        .islands
        .iter_mut()
        .next()
        .unwrap()
        .1
        .constraint_remove_count = u32::MAX;
    let before = serde_json::to_vec(w).unwrap();
    assert_eq!(
        imported.step(),
        Err(CaptureError::ImportedDomain(DomainError::Counter))
    );
    assert_eq!(serde_json::to_vec(imported.get().unwrap()).unwrap(), before);
}
#[test]
fn persisted_native_pending_unlinks_and_both_modified_endpoints_have_headroom() {
    let mut source = boxes();
    let f = capture(&mut source);
    let mut imported = CheckpointOwner::from_persisted(
        f.encoded(),
        &mut payload(),
        &mut work(),
        &mut allocation(),
    )
    .unwrap();
    let w = imported.access_mut().unwrap();
    let c = w.colliders.colliders.checkpoint_slot_count() as u64;
    let b = w.bodies.bodies.checkpoint_slot_count() as u64;
    let pairs = c * (c - 1) / 2;
    for (_, i) in w.islands.persistent.islands.iter_mut() {
        i.constraint_remove_count = 0;
    }
    w.islands
        .persistent
        .islands
        .iter_mut()
        .next()
        .unwrap()
        .1
        .constraint_remove_count = (u64::from(u32::MAX) - b - 2 * pairs) as u32;
    assert_eq!(super::super::pre_step(w), Err(DomainError::Counter));
    // At the new exact counter bound, a real queued contact unlink must still
    // be charged. Neither check executes a speculative native simulation step.
    w.islands
        .persistent
        .islands
        .iter_mut()
        .next()
        .unwrap()
        .1
        .constraint_remove_count = (u64::from(u32::MAX) - b - 3 * pairs) as u32;
    super::super::pre_step(w).unwrap();
    let edge = w
        .islands
        .persistent
        .islands
        .iter()
        .flat_map(|(_, i)| &i.contact_links)
        .next()
        .unwrap()
        .edge_id;
    w.islands.persistent.unlink_contact(edge);
    assert_eq!(w.islands.checkpoint_pending_removal_count(), 1);
    assert_eq!(super::super::pre_step(w), Err(DomainError::Counter));
}
