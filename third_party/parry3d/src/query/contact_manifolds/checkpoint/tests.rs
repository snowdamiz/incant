use super::*;
use crate::math::{Pose, Vector};
use crate::query::{
    ContactManifold, DefaultQueryDispatcher, PersistentQueryDispatcher, TrackedContact,
};
use crate::shape::{Compound, Cuboid, Shape, SharedShape};
use alloc::vec::Vec;
use s::construction::AllocationBudget;

fn budget() -> s::Budget {
    s::Budget::new(s::MAX_ALLOCATION).unwrap()
}
fn source(n: usize) -> Compound {
    Compound::new(
        (0..n)
            .map(|i| {
                let mut v = Vector::ZERO;
                v.x = i as f32 * 0.8;
                (
                    Pose::from_parts(v, Default::default()),
                    SharedShape::new(Cuboid::new(Vector::splat(0.35))),
                )
            })
            .collect(),
    )
}
type Manifold = ContactManifold<u32, u32>;
// Test-only scalar reconstruction of the accompanying contact payload. The
// workspace constructor is tested without cloning any dynamic workspace.
fn copy_manifolds(m: &[Manifold]) -> Vec<Manifold> {
    m.iter()
        .map(|m| {
            let mut n = Manifold::new();
            n.local_n1 = m.local_n1;
            n.local_n2 = m.local_n2;
            n.subshape1 = m.subshape1;
            n.subshape2 = m.subshape2;
            n.data = m.data;
            n.set_subshape_pos1(m.subshape_pos1().copied());
            n.set_subshape_pos2(m.subshape_pos2().copied());
            for c in &m.points {
                n.points.push(TrackedContact {
                    local_p1: c.local_p1,
                    local_p2: c.local_p2,
                    dist: c.dist,
                    fid1: c.fid1,
                    fid2: c.fid2,
                    data: c.data,
                });
            }
            n
        })
        .collect()
}
fn bits(m: &[Manifold]) -> Vec<u32> {
    let mut out = Vec::new();
    for m in m {
        out.extend([m.subshape1, m.subshape2, m.data, m.points.len() as u32]);
        for v in [m.local_n1, m.local_n2] {
            for i in 0..crate::math::DIM {
                out.push(v[i].to_bits());
            }
        }
        for p in [m.subshape_pos1(), m.subshape_pos2()] {
            out.push(u32::from(p.is_some()));
            if let Some(p) = p {
                for i in 0..crate::math::DIM {
                    out.push(p.translation[i].to_bits());
                }
                #[cfg(feature = "dim2")]
                out.extend([p.rotation.re.to_bits(), p.rotation.im.to_bits()]);
                #[cfg(feature = "dim3")]
                out.extend(p.rotation.to_array().map(f32::to_bits));
            }
        }
        for c in &m.points {
            for v in [c.local_p1, c.local_p2] {
                for i in 0..crate::math::DIM {
                    out.push(v[i].to_bits());
                }
            }
            out.extend([c.dist.to_bits(), c.fid1.0, c.fid2.0, c.data]);
        }
    }
    out
}
fn restore(r: &s::WorkspaceRecord) -> ContactManifoldsWorkspace {
    let plan = match r.kind {
        s::WorkspaceKind::CompositeShape => {
            CompositeShapeShapeContactManifoldsWorkspace::checkpoint_plan(r.detectors.len())
                .unwrap()
        }
        s::WorkspaceKind::CompositeComposite => {
            CompositeShapeCompositeShapeContactManifoldsWorkspace::checkpoint_plan(
                r.detectors.len(),
            )
            .unwrap()
        }
        _ => panic!("test needs actual composite workspace"),
    };
    assert!(AllocationBudget::new(plan.bytes() - 1, plan.allocations())
        .admit(plan)
        .is_err());
    let mut p = AllocationBudget::new(plan.bytes(), plan.allocations())
        .admit(plan)
        .unwrap();
    let (out, bytes, calls) = crate::checkpoint_allocation_tests::measure(|| match r.kind {
        s::WorkspaceKind::CompositeShape => {
            let w = CompositeShapeShapeContactManifoldsWorkspace::checkpoint_construct(
                r.timestamp,
                &r.detectors,
                &mut p,
            )
            .unwrap();
            ContactManifoldsWorkspace(p.boxed(w).unwrap())
        }
        s::WorkspaceKind::CompositeComposite => {
            let w = CompositeShapeCompositeShapeContactManifoldsWorkspace::checkpoint_construct(
                r.timestamp,
                &r.detectors,
                &mut p,
            )
            .unwrap();
            ContactManifoldsWorkspace(p.boxed(w).unwrap())
        }
        _ => unreachable!(),
    });
    p.finish().unwrap();
    assert!(bytes <= plan.bytes());
    assert_eq!(calls, plan.allocations());
    std::eprintln!("CHECKED_WORKSPACE dimension={} deterministic={} kind={:?} entries={} actual_bytes={} charged_bytes={} calls={}",crate::math::DIM,cfg!(feature="enhanced-determinism"),r.kind,r.detectors.len(),bytes,plan.bytes(),calls);
    out
}
fn compare_record(a: &s::WorkspaceRecord, b: &s::WorkspaceRecord) {
    assert_eq!(a.kind, b.kind);
    assert_eq!(a.timestamp, b.timestamp);
    assert_eq!(a.detectors, b.detectors);
}
fn exercise(double: bool, flipped: bool) {
    let compound = source(48);
    let second = source(3);
    let primitive = Cuboid::new(Vector::splat(0.65));
    let (a, b): (&dyn Shape, &dyn Shape) = if double {
        (&compound, &second)
    } else if flipped {
        (&primitive, &compound)
    } else {
        (&compound, &primitive)
    };
    let mut original = None;
    let mut manifolds = Vec::<Manifold>::new();
    let mut poses = Vec::new();
    // Populate, empty, revisit and move the working set through disjoint leaves.
    for x in [0., 7., 18., 30., 100., 0., 0.1, 16., 8., 0.] {
        let mut v = Vector::ZERO;
        v.x = if flipped { -x } else { x };
        poses.push(Pose::from_parts(v, Default::default()));
    }
    for pose in &poses[..6] {
        DefaultQueryDispatcher
            .contact_manifolds(pose, a, b, 0.02, &mut manifolds, &mut original)
            .unwrap();
    }
    assert!(!manifolds.is_empty());
    for (i, m) in manifolds.iter_mut().enumerate() {
        m.data = 100 + i as u32;
        for (j, p) in m.points.iter_mut().enumerate() {
            p.data = 400 + i as u32 * 8 + j as u32;
        }
    }
    let saved = original
        .as_ref()
        .unwrap()
        .checkpoint_capture(&mut budget())
        .unwrap();
    assert!(original
        .as_ref()
        .unwrap()
        .checkpoint_capture(&mut s::Budget::new(0).unwrap())
        .is_err());
    let mut candidate = Some(restore(&saved));
    let mut candidate_manifolds = copy_manifolds(&manifolds);
    compare_record(
        &saved,
        &candidate
            .as_ref()
            .unwrap()
            .checkpoint_capture(&mut budget())
            .unwrap(),
    );
    let mut reused = false;
    let mut empty = false;
    for pose in poses.iter().cycle().take(60) {
        DefaultQueryDispatcher
            .contact_manifolds(pose, a, b, 0.02, &mut manifolds, &mut original)
            .unwrap();
        DefaultQueryDispatcher
            .contact_manifolds(pose, a, b, 0.02, &mut candidate_manifolds, &mut candidate)
            .unwrap();
        assert_eq!(bits(&manifolds), bits(&candidate_manifolds));
        reused |= manifolds
            .iter()
            .any(|m| m.data >= 100 || m.points.iter().any(|p| p.data >= 400));
        empty |= manifolds.is_empty();
        compare_record(
            &original
                .as_ref()
                .unwrap()
                .checkpoint_capture(&mut budget())
                .unwrap(),
            &candidate
                .as_ref()
                .unwrap()
                .checkpoint_capture(&mut budget())
                .unwrap(),
        );
    }
    assert!(reused && empty);
}
#[test]
fn checked_workspace_compound_primitive_preserves_churn_and_reused_contacts() {
    exercise(false, false);
    exercise(false, true);
}
#[test]
fn checked_workspace_compound_compound_preserves_churn_and_reused_contacts() {
    exercise(true, false);
}
#[test]
fn checked_workspace_map_canonicalization_is_mode_specific_and_precharged() {
    use crate::utils::hashmap::HashMap;
    let mut source = HashMap::<u32, u32>::default();
    for k in 0..512 {
        assert_eq!(source.insert(k, k + 17), None);
    }
    source.retain(|k, _| k % 47 == 0);
    assert_eq!(source.insert(700, 900), None);
    assert_eq!(source.insert(3, 20), None);
    let record = map::capture(
        &source,
        false,
        s::WorkspaceKind::CompositeShape,
        |v| (*v as usize, false),
        &mut budget(),
    )
    .unwrap();
    #[cfg(feature = "enhanced-determinism")]
    assert_eq!(
        record
            .detectors
            .iter()
            .map(|d| d.key[0])
            .collect::<Vec<_>>(),
        source.keys().copied().collect::<Vec<_>>()
    );
    #[cfg(not(feature = "enhanced-determinism"))]
    assert!(record.detectors.windows(2).all(|w| w[0].key < w[1].key));
    let plan = map::plan::<u32, u32>(record.detectors.len()).unwrap();
    let mut p = AllocationBudget::new(plan.bytes(), plan.allocations())
        .admit(plan)
        .unwrap();
    let restored = map::construct::<u32, u32>(&record.detectors, |d| d.manifold, &mut p).unwrap();
    p.finish().unwrap();
    assert_eq!(source.len(), restored.len());
    for (k, v) in source {
        assert_eq!(restored[&k], v);
    }
    let next = map::capture(
        &restored,
        false,
        s::WorkspaceKind::CompositeShape,
        |v| (*v as usize, false),
        &mut budget(),
    )
    .unwrap();
    compare_record(&record, &next);
    assert!(map::plan::<u32, u32>(usize::MAX).is_err());
}

#[test]
fn checked_workspace_plans_bound_actual_tables_entries_and_box_allocations() {
    for kind in [
        s::WorkspaceKind::CompositeShape,
        s::WorkspaceKind::CompositeComposite,
    ] {
        for n in [0, 1, 3, 4, 7, 8, 14, 15, 28, 29, 127, 128, 4096] {
            let record = s::WorkspaceRecord {
                kind,
                timestamp: true,
                detectors: (0..n)
                    .map(|i| s::SubDetector {
                        key: [
                            i,
                            if kind == s::WorkspaceKind::CompositeShape {
                                0
                            } else {
                                i + 7
                            },
                        ],
                        manifold: i,
                        timestamp: true,
                    })
                    .collect(),
            };
            let workspace = restore(&record);
            compare_record(
                &record,
                &workspace.checkpoint_capture(&mut budget()).unwrap(),
            );
        }
    }
}
#[test]
fn checked_workspace_low_level_duplicate_keys_and_invalid_single_keys_refuse() {
    let rows = [s::SubDetector {
        key: [1, 0],
        manifold: 0,
        timestamp: true,
    }; 2];
    let plan = map::plan::<u32, u32>(rows.len()).unwrap();
    let mut p = AllocationBudget::new(plan.bytes(), plan.allocations())
        .admit(plan)
        .unwrap();
    assert!(map::construct::<u32, u32>(&rows, |d| d.manifold, &mut p).is_err());
    p.finish().unwrap();
    let rows = [s::SubDetector {
        key: [1, 7],
        manifold: 0,
        timestamp: true,
    }];
    let plan = map::plan::<u32, u32>(rows.len()).unwrap();
    let mut p = AllocationBudget::new(plan.bytes(), plan.allocations())
        .admit(plan)
        .unwrap();
    assert!(map::construct::<u32, u32>(&rows, |d| d.manifold, &mut p).is_err());
    p.finish().unwrap();
}

#[test]
fn checked_workspace_refuses_custom_owner_even_with_a_known_typed_view() {
    struct Masquerade(CompositeShapeShapeContactManifoldsWorkspace);
    impl WorkspaceData for Masquerade {
        fn as_typed_workspace_data(&self) -> TypedWorkspaceData<'_> {
            TypedWorkspaceData::CompositeShapeShapeContactManifoldsWorkspace(&self.0)
        }
        fn clone_dyn(&self) -> alloc::boxed::Box<dyn WorkspaceData> {
            alloc::boxed::Box::new(Self(Default::default()))
        }
    }
    let custom = ContactManifoldsWorkspace::from(Masquerade(Default::default()));
    assert_eq!(
        custom.checkpoint_capture(&mut budget()).unwrap_err(),
        s::Error::Workspace
    );
}
