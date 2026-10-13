use super::*;
use s::construction::AllocationBudget;

fn identity() -> s::Identity {
    s::Identity {
        dimension: if crate::math::DIM == 2 {
            s::Dimension::Planar
        } else {
            s::Dimension::Spatial
        },
        build: [41; 32],
    }
}
fn budget() -> s::Budget {
    s::Budget::new(s::MAX_ALLOCATION).unwrap()
}
fn source() -> Compound {
    let mut shapes = Vec::new();
    for i in [4., 0., 2., 6.] {
        shapes.push((
            Pose::from_parts(Vector::splat(i), Default::default()),
            SharedShape::new(crate::shape::Cuboid::new(Vector::splat(0.4))),
        ));
    }
    #[cfg(feature = "dim3")]
    {
        shapes.push((
            Pose::from_parts(
                Vector::new(-0., 2., 1.),
                crate::math::Rotation::from_rotation_y(0.3),
            ),
            SharedShape::capsule_y(0.3, 0.2),
        ));
        shapes.push((
            Pose::from_parts(Vector::new(1., 2., 3.), Default::default()),
            SharedShape::ball(0.2),
        ));
    }
    Compound::new(shapes)
}
fn witness(c: &Compound) -> s::ValidatedGeometry {
    let (g, r) = c.checkpoint_capture(identity(), &mut budget()).unwrap();
    s::attach_compound(s::validate_geometry(g).unwrap(), r, &mut budget()).unwrap()
}
#[test]
fn checked_compound_preserves_children_acceleration_and_queries_without_shared_native_shapes() {
    let c = source();
    let g = witness(&c);
    let w = g.compound().unwrap();
    let plan = Compound::checkpoint_plan(&w, identity()).unwrap();
    let mut permit = AllocationBudget::new(plan.bytes(), plan.allocations())
        .admit(plan)
        .unwrap();
    let (restored, bytes, calls) = crate::checkpoint_allocation_tests::measure(|| {
        Compound::from_checkpoint(&w, identity(), &mut permit).unwrap()
    });
    permit.finish().unwrap();
    assert!(bytes <= plan.bytes());
    assert_eq!(calls, plan.allocations());
    std::eprintln!(
        "CHECKED_COMPOUND dimension={} deterministic={} actual_bytes={} charged_bytes={} calls={}",
        crate::math::DIM,
        cfg!(feature = "enhanced-determinism"),
        bytes,
        plan.bytes(),
        calls
    );
    let (ga, ra) = c.checkpoint_capture(identity(), &mut budget()).unwrap();
    let (gb, rb) = restored
        .checkpoint_capture(identity(), &mut budget())
        .unwrap();
    assert_eq!(ga, gb);
    assert_eq!(ra, rb);
    for ((_, a), (_, b)) in c.shapes().iter().zip(restored.shapes()) {
        assert!(!alloc::sync::Arc::ptr_eq(&a.0, &b.0));
    }
    for i in 0..24 {
        let q = Aabb::from_half_extents(Vector::splat(i as f32 * 0.3), Vector::splat(0.75));
        assert_eq!(
            c.bvh().intersect_aabb(&q).collect::<Vec<_>>(),
            restored.bvh().intersect_aabb(&q).collect::<Vec<_>>()
        );
    }
    assert_eq!(c.flags(), restored.flags());
    assert!(restored.pseudo_normals.is_none());
}
#[test]
fn checked_compound_refuses_unknown_flags_cache_drift_identity_and_short_permit() {
    let c = source();
    let g = witness(&c);
    let w = g.compound().unwrap();
    let plan = Compound::checkpoint_plan(&w, identity()).unwrap();
    assert!(AllocationBudget::new(plan.bytes() - 1, plan.allocations())
        .admit(plan)
        .is_err());
    let empty = AllocationPlan::default();
    let mut p = AllocationBudget::new(0, 0).admit(empty).unwrap();
    assert!(Compound::from_checkpoint(&w, identity(), &mut p).is_err());
    let mut other = identity();
    other.build[0] ^= 1;
    assert!(Compound::checkpoint_plan(&w, other).is_err());
    assert!(c
        .checkpoint_capture(identity(), &mut s::Budget::new(0).unwrap())
        .is_err());
    for kind in 0..2 {
        let mut c = source();
        if kind == 0 {
            c.aabbs[0].mins[0] -= 0.1;
        } else {
            c.aabb.maxs[0] += 0.1;
        }
        let (g, r) = c.checkpoint_capture(identity(), &mut budget()).unwrap();
        if let Ok(g) = s::attach_compound(s::validate_geometry(g).unwrap(), r, &mut budget()) {
            assert!(Compound::checkpoint_plan(&g.compound().unwrap(), identity()).is_err());
        }
    }
    let mut c = source();
    c.flags = CompoundFlags::from_bits_retain(128);
    assert!(c.checkpoint_capture(identity(), &mut budget()).is_err());
    c.flags = CompoundFlags::empty();
    c.pseudo_normals = Some(Vec::new());
    assert!(c.checkpoint_capture(identity(), &mut budget()).is_err());
}
#[test]
fn checked_compound_refuses_nested_custom_domain_and_noncanonical_children() {
    // Mutating private fields here models native states that public constructors
    // reject; capture must refuse them, without invoking a panicking constructor.
    let mut c = source();
    c.shapes[0].1 = SharedShape::new(source());
    assert!(c.checkpoint_capture(identity(), &mut budget()).is_err());
    let mut c = source();
    c.shapes[0].1 = SharedShape::capsule_x(0.3, 0.2);
    assert!(c.checkpoint_capture(identity(), &mut budget()).is_err());
    #[cfg(feature = "dim2")]
    {
        let mut c = source();
        c.shapes[0].0.rotation = crate::math::Rotation::from_cos_sin_unchecked(1., -0.);
        assert!(c.checkpoint_capture(identity(), &mut budget()).is_err());
        let mut c = source();
        c.shapes[0].1 = SharedShape::ball(0.3);
        assert!(c.checkpoint_capture(identity(), &mut budget()).is_err());
    }
    #[cfg(feature = "dim3")]
    {
        let mut c = source();
        let mut capsule = crate::shape::Capsule::new_y(0.3, 0.2);
        capsule.segment.a.x = f32::from_bits(capsule.segment.a.x.to_bits() ^ (1 << 31));
        c.shapes[0].1 = SharedShape::new(capsule);
        assert!(c.checkpoint_capture(identity(), &mut budget()).is_err());
    }
}

#[test]
fn checked_compound_mass_replays_native_sum_without_temporary_allocation() {
    fn bits(m: crate::mass_properties::MassProperties) -> Vec<u32> {
        let mut out = Vec::new();
        for i in 0..crate::math::DIM {
            out.push(m.local_com[i].to_bits());
        }
        out.push(m.inv_mass.to_bits());
        #[cfg(feature = "dim2")]
        out.push(m.inv_principal_inertia.to_bits());
        #[cfg(feature = "dim3")]
        {
            out.extend(m.inv_principal_inertia.to_array().map(f32::to_bits));
            out.extend(m.principal_inertia_local_frame.to_array().map(f32::to_bits));
        }
        out
    }
    for c in [
        source(),
        Compound::new(vec![(
            Pose::default(),
            SharedShape::new(crate::shape::Cuboid::new(Vector::splat(0.5))),
        )]),
    ] {
        let g = witness(&c);
        let w = g.compound().unwrap();
        let (_, bytes, calls) = crate::checkpoint_allocation_tests::measure(|| {
            Compound::checkpoint_plan(&w, identity()).unwrap()
        });
        assert_eq!((bytes, calls), (0, 0));
        for density in [0., -0., 0.125, 1., 7., 1000.] {
            let expected = c.mass_properties(density);
            let (actual, bytes, calls) = crate::checkpoint_allocation_tests::measure(|| {
                Compound::checkpoint_mass_properties(&w, density)
            });
            assert_eq!((bytes, calls), (0, 0));
            assert_eq!(bits(actual), bits(expected));
        }
    }
}
