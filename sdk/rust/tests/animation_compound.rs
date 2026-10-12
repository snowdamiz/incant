use incant_module::{
    AnimationGraphSnapshotV2, AnimationIkSnapshotV2, AnimationPoseKindV5 as Kind,
    AnimationPoseSnapshotV5 as Snapshot, PoseInterpolation as Inner,
};
use incant_wasm_abi as abi;
fn sample(clip: u32, seconds: f64, weight: f64) -> abi::AnimationPoseSample {
    abi::AnimationPoseSample {
        clip,
        seconds,
        weight,
        reserved: 0,
    }
}
fn row() -> abi::AnimationRowV4 {
    abi::AnimationRowV4 {
        id: [1; 16],
        clip: 2,
        flags: 3,
        seconds: 0.5,
        speed: -2.,
        source_clip: 0,
        transition_flags: 0,
        source_seconds: 0.,
        source_speed: 0.,
        elapsed: 0.,
        duration: 0.,
        pose_kind: 4,
        sample_offset: 0,
        sample_count: 4,
        reserved: 0,
    }
}
fn descriptor() -> abi::AnimationPoseGroups {
    abi::AnimationPoseGroups {
        entity: [1; 16],
        split: 2,
        source_kind: 1,
        destination_kind: 2,
        reserved: 0,
        weight: 0.,
    }
}
fn frame(
    r: abi::AnimationRowV4,
    g: Option<abi::AnimationPoseGroups>,
    samples: &[abi::AnimationPoseSample],
) -> Vec<u8> {
    let mut bytes = abi::AnimationHeaderV5 {
        magic: *abi::ANIMATION_V5_MAGIC,
        scene: [9; 16],
        count: 1,
        stride: 96,
        sample_count: samples.len() as u32,
        sample_stride: 24,
        compound_count: u32::from(g.is_some()),
        compound_stride: 40,
    }
    .encode()
    .to_vec();
    bytes.extend(r.encode());
    for s in samples {
        bytes.extend(s.encode());
    }
    if let Some(g) = g {
        bytes.extend(g.encode());
    }
    bytes
}
fn samples() -> [abi::AnimationPoseSample; 4] {
    [
        sample(u32::MAX, 0., 1.),
        sample(1, 0.25, 0.),
        sample(2, 0.5, 0.25),
        sample(3, 0.75, 0.75),
    ]
}
#[test]
fn compound_retains_inactive_slerp_endpoints_and_independent_mean_at_every_outer_weight() {
    for weight in [0., 0.125, 1.] {
        let mut g = descriptor();
        g.weight = weight;
        let mut unaligned = vec![0xa5];
        unaligned.extend(frame(row(), Some(g), &samples()));
        let s = Snapshot::parse(&unaligned[1..]).unwrap();
        let r = s.iter().next().unwrap();
        assert_eq!(r.kind, Kind::Compound);
        assert_eq!(r.outer_weight, Some(weight));
        assert_eq!(r.group(0).unwrap().interpolation, Inner::SphericalPair);
        assert_eq!(r.group(1).unwrap().interpolation, Inner::Weighted);
        let a: Vec<_> = r.group(0).unwrap().samples().collect();
        assert_eq!(a[0].clip, None);
        assert_eq!(a[1].weight, 0.);
        assert_eq!(
            r.group(1)
                .unwrap()
                .samples()
                .map(|s| s.weight)
                .collect::<Vec<_>>(),
            [0.25, 0.75]
        );
        assert!(r.group(2).is_none());
    }
}
#[test]
fn standalone_clip_pair_transition_and_sixteen_term_mean_remain_complete() {
    for kind in [
        Kind::Single,
        Kind::SlerpParameter,
        Kind::SlerpTransition,
        Kind::WeightedQuaternionMean,
    ] {
        let mut r = row();
        r.pose_kind = kind as u32;
        let values = match kind {
            Kind::Single => vec![sample(2, 0.5, 1.)],
            Kind::WeightedQuaternionMean => (0..16).map(|n| sample(n, 0.5, 1. / 16.)).collect(),
            _ => vec![sample(u32::MAX, 0., 1.), sample(2, 0.5, 0.)],
        };
        if kind == Kind::WeightedQuaternionMean {
            r.clip = 0;
        }
        if kind == Kind::SlerpTransition {
            r.source_clip = u32::MAX;
            r.transition_flags = 1;
            r.source_speed = 1.;
            r.duration = 1.;
        }
        r.sample_count = values.len() as u32;
        let bytes = frame(r, None, &values);
        let view = Snapshot::parse(&bytes).unwrap();
        let r = view.iter().next().unwrap();
        assert_eq!(r.kind, kind);
        assert_eq!(r.group(0).unwrap().samples().len(), values.len());
        assert!(r.outer_weight.is_none());
    }
}
#[test]
fn malformed_late_group_and_inactive_inputs_refuse_before_any_view() {
    let valid = frame(row(), Some(descriptor()), &samples());
    for n in 0..valid.len() {
        assert!(Snapshot::parse(&valid[..n]).is_err());
    }
    for mode in 0..24 {
        let mut r = row();
        let mut g = descriptor();
        let mut s = samples();
        match mode {
            0 => g.entity = [2; 16],
            1 => g.split = 0,
            2 => g.split = 4,
            3 => g.split = 9,
            4 => g.source_kind = 256,
            5 => g.destination_kind = 3,
            6 => g.reserved = 1,
            7 => g.weight = f64::NAN,
            8 => g.weight = 1.01,
            9 => g.weight = -0.1,
            10 => s[1].seconds = f64::NAN,
            11 => s[0].seconds = 1.,
            12 => s[3].weight = 0.,
            13 => s[3] = s[2],
            14 => s[3].weight = 0.7,
            15 => s[2].reserved = 1,
            16 => r.clip = 3,
            17 => r.sample_offset = 1,
            18 => r.reserved = 1,
            19 => r.pose_kind = 3,
            20 => r.transition_flags = 1,
            21 => g.destination_kind = 0,
            22 => s[0].weight = 0.,
            _ => s[3].clip = 256,
        }
        assert!(
            Snapshot::parse(&frame(r, Some(g), &s)).is_err(),
            "mode{mode}"
        );
    }
    for (offset, value) in [
        (24, 100001u32),
        (28, 95),
        (32, 214337),
        (36, 23),
        (40, 1025),
        (44, 39),
    ] {
        let mut b = valid.clone();
        b[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        assert!(Snapshot::parse(&b).is_err());
    }
    let mut b = valid.clone();
    b.push(0);
    assert!(Snapshot::parse(&b).is_err());
}
#[test]
fn ik_v2_validates_both_sections_and_membership_before_exposing_compound() {
    let pose = frame(row(), Some(descriptor()), &samples());
    let header = abi::AnimationIkHeader {
        magic: *abi::ANIMATION_IK_V2_MAGIC,
        pose_bytes: pose.len() as u32,
        count: 1,
        stride: 104,
        reserved: 0,
    };
    let constraint = abi::AnimationIkRow {
        entity: [1; 16],
        constraint: [9; 16],
        chain: [0, 1, 2],
        reserved: 0,
        target: [1., 2., 3.],
        pole: [0., 1., 0.],
        weight: 0.5,
    };
    let bytes = [header.encode().as_slice(), &pose, &constraint.encode()].concat();
    let parsed = AnimationIkSnapshotV2::parse(&bytes).unwrap();
    assert_eq!(parsed.poses().iter().next().unwrap().kind, Kind::Compound);
    assert_eq!(parsed.constraints().next().unwrap().target, [1., 2., 3.]);
    for mode in 0..6 {
        let mut r = constraint;
        match mode {
            0 => r.entity = [2; 16],
            1 => r.chain = [0, 0, 2],
            2 => r.chain = [0, 1, 4096],
            3 => r.weight = f64::NAN,
            4 => r.reserved = 1,
            _ => r.target[0] = 1e10,
        };
        let b = [header.encode().as_slice(), &pose, &r.encode()].concat();
        assert!(AnimationIkSnapshotV2::parse(&b).is_err());
    }
    let mut b = bytes.clone();
    b[abi::AnimationIkHeader::SIZE
        + abi::AnimationHeaderV5::SIZE
        + abi::AnimationRowV4::SIZE
        + 24
        + 8..][..8]
        .copy_from_slice(&f64::NAN.to_le_bytes());
    assert!(AnimationIkSnapshotV2::parse(&b).is_err());
}
#[test]
fn graph_v2_requires_sorted_associated_fade_clock_and_complete_old_frame() {
    let graph = abi::AnimationGraphRow {
        id: [1; 16],
        state: [20; 16],
        seconds: 0.25,
        elapsed: 0.125,
        flags: 1,
        parameter_offset: 0,
        parameter_count: 0,
        reserved: 0,
    };
    let base = [
        abi::AnimationGraphHeader {
            magic: *abi::ANIMATION_GRAPH_MAGIC,
            count: 1,
            stride: 64,
            parameter_count: 0,
            parameter_stride: 24,
        }
        .encode()
        .as_slice(),
        &graph.encode(),
    ]
    .concat();
    let header = abi::AnimationGraphHeaderV2 {
        magic: *abi::ANIMATION_GRAPH_V2_MAGIC,
        graph_bytes: base.len() as u32,
        count: 1,
        stride: 72,
        reserved: 0,
    };
    let fade = abi::AnimationGraphFade {
        entity: [1; 16],
        state: [19; 16],
        seconds: 0.5,
        speed: -2.,
        elapsed: 0.125,
        duration: 0.25,
        flags: 1,
        reserved: 0,
    };
    let bytes = [header.encode().as_slice(), &base, &fade.encode()].concat();
    let view = AnimationGraphSnapshotV2::parse(&bytes).unwrap();
    assert!(view.graphs().iter().next().unwrap().paused);
    assert_eq!(view.fades().next().unwrap().speed, -2.);
    for n in 0..bytes.len() {
        assert!(AnimationGraphSnapshotV2::parse(&bytes[..n]).is_err());
    }
    for mode in 0..12 {
        let mut f = fade;
        match mode {
            0 => f.entity = [2; 16],
            1 => f.state = [20; 16],
            2 => f.elapsed = 0.2,
            3 => f.duration = 0.125,
            4 => f.duration = 61.,
            5 => f.seconds = -1.,
            6 => f.speed = 17.,
            7 => f.flags = 2,
            8 => f.reserved = 1,
            9 => f.seconds = f64::NAN,
            10 => f.duration = 0.,
            _ => f.speed = f64::INFINITY,
        };
        let b = [header.encode().as_slice(), &base, &f.encode()].concat();
        assert!(AnimationGraphSnapshotV2::parse(&b).is_err());
    }
}

fn many_pose_frame(rows: &[abi::AnimationRowV4], groups: &[abi::AnimationPoseGroups]) -> Vec<u8> {
    let mut b = abi::AnimationHeaderV5 {
        magic: *abi::ANIMATION_V5_MAGIC,
        scene: [9; 16],
        count: rows.len() as u32,
        stride: 96,
        sample_count: (rows.len() * 4) as u32,
        sample_stride: 24,
        compound_count: groups.len() as u32,
        compound_stride: 40,
    }
    .encode()
    .to_vec();
    for r in rows {
        b.extend(r.encode());
    }
    for _ in rows {
        for s in samples() {
            b.extend(s.encode());
        }
    }
    for g in groups {
        b.extend(g.encode());
    }
    b
}
#[test]
fn sparse_descriptors_and_pose_owners_are_exact_sorted_unique_associations() {
    let rows: Vec<_> = (1..=3)
        .map(|id| abi::AnimationRowV4 {
            id: [id; 16],
            sample_offset: (id as u32 - 1) * 4,
            ..row()
        })
        .collect();
    let groups: Vec<_> = (1..=3)
        .map(|id| abi::AnimationPoseGroups {
            entity: [id; 16],
            ..descriptor()
        })
        .collect();
    assert_eq!(
        Snapshot::parse(&many_pose_frame(&rows, &groups))
            .unwrap()
            .len(),
        3
    );
    for mode in 0..8 {
        let mut r = rows.clone();
        let mut g = groups.clone();
        match mode {
            0 => g[2].entity = g[1].entity,
            1 => g.swap(0, 2),
            2 => {
                g.remove(1);
            }
            3 => g.push(g[2]),
            4 => r[2].id = r[1].id,
            5 => r.swap(0, 2),
            6 => g[2].entity = [4; 16],
            _ => r[2].sample_offset = 4,
        }
        assert!(
            Snapshot::parse(&many_pose_frame(&r, &g)).is_err(),
            "mode{mode}"
        );
    }
    let mut r = row();
    r.sample_count = 16;
    r.clip = 8;
    r.seconds = 0.5;
    let s: Vec<_> = (0..16).map(|clip| sample(clip, 0.5, 0.125)).collect();
    let g = abi::AnimationPoseGroups {
        split: 8,
        source_kind: 2,
        destination_kind: 2,
        ..descriptor()
    };
    let b = frame(r, Some(g), &s);
    let parsed = Snapshot::parse(&b).unwrap();
    let p = parsed.iter().next().unwrap();
    assert_eq!(p.group(0).unwrap().samples().len(), 8);
    assert_eq!(p.group(1).unwrap().samples().len(), 8);
}
#[test]
fn ik_v2_rejects_duplicate_ids_owner_reentry_and_missing_late_pose_owner() {
    let rows: Vec<_> = (1..=3)
        .map(|id| abi::AnimationRowV4 {
            id: [id; 16],
            sample_offset: (id as u32 - 1) * 4,
            ..row()
        })
        .collect();
    let groups: Vec<_> = (1..=3)
        .map(|id| abi::AnimationPoseGroups {
            entity: [id; 16],
            ..descriptor()
        })
        .collect();
    let pose = many_pose_frame(&rows, &groups);
    let baseline = abi::AnimationIkRow {
        entity: [1; 16],
        constraint: [9; 16],
        chain: [0, 1, 2],
        reserved: 0,
        target: [1., 2., 3.],
        pole: [0., 1., 0.],
        weight: 0.5,
    };
    let valid = [
        baseline,
        abi::AnimationIkRow {
            entity: [2; 16],
            ..baseline
        },
    ];
    let encode = |constraints: &[abi::AnimationIkRow]| {
        let mut b = abi::AnimationIkHeader {
            magic: *abi::ANIMATION_IK_V2_MAGIC,
            pose_bytes: pose.len() as u32,
            count: constraints.len() as u32,
            stride: 104,
            reserved: 0,
        }
        .encode()
        .to_vec();
        b.extend(&pose);
        for r in constraints {
            b.extend(r.encode());
        }
        b
    };
    assert_eq!(
        AnimationIkSnapshotV2::parse(&encode(&valid))
            .unwrap()
            .constraints()
            .len(),
        2
    );
    for rows in [
        vec![baseline, baseline],
        vec![valid[1], valid[0]],
        vec![valid[0], valid[1], valid[0]],
        vec![
            baseline,
            abi::AnimationIkRow {
                entity: [4; 16],
                ..baseline
            },
        ],
        (0..33)
            .map(|n| abi::AnimationIkRow {
                constraint: [n; 16],
                ..baseline
            })
            .collect(),
    ] {
        assert!(AnimationIkSnapshotV2::parse(&encode(&rows)).is_err());
    }
}
#[test]
fn graph_v2_rejects_sparse_duplicate_out_of_order_or_unassociated_fades() {
    let graph = abi::AnimationGraphRow {
        id: [1; 16],
        state: [20; 16],
        seconds: 0.25,
        elapsed: 0.125,
        flags: 0,
        parameter_offset: 0,
        parameter_count: 0,
        reserved: 0,
    };
    let mut base = abi::AnimationGraphHeader {
        magic: *abi::ANIMATION_GRAPH_MAGIC,
        count: 3,
        stride: 64,
        parameter_count: 0,
        parameter_stride: 24,
    }
    .encode()
    .to_vec();
    for id in 1..=3 {
        base.extend(
            abi::AnimationGraphRow {
                id: [id; 16],
                ..graph
            }
            .encode(),
        );
    }
    let fade = abi::AnimationGraphFade {
        entity: [1; 16],
        state: [19; 16],
        seconds: 0.5,
        speed: -2.,
        elapsed: 0.125,
        duration: 0.25,
        flags: 1,
        reserved: 0,
    };
    let encode = |rows: &[abi::AnimationGraphFade]| {
        let mut b = abi::AnimationGraphHeaderV2 {
            magic: *abi::ANIMATION_GRAPH_V2_MAGIC,
            graph_bytes: base.len() as u32,
            count: rows.len() as u32,
            stride: 72,
            reserved: 0,
        }
        .encode()
        .to_vec();
        b.extend(&base);
        for r in rows {
            b.extend(r.encode());
        }
        b
    };
    let third = abi::AnimationGraphFade {
        entity: [3; 16],
        ..fade
    };
    assert_eq!(
        AnimationGraphSnapshotV2::parse(&encode(&[fade, third]))
            .unwrap()
            .fades()
            .len(),
        2
    );
    for rows in [
        vec![fade, fade],
        vec![third, fade],
        vec![
            fade,
            abi::AnimationGraphFade {
                entity: [4; 16],
                ..fade
            },
        ],
        vec![
            fade,
            abi::AnimationGraphFade {
                entity: [0; 16],
                ..fade
            },
        ],
    ] {
        assert!(AnimationGraphSnapshotV2::parse(&encode(&rows)).is_err());
    }
}
