//! Analytic counterexamples for the declared R4 evidence estimator. These are
//! mathematical contracts, not quality thresholds or a rasterization substitute.
use super::*;

const N: usize = 16;
const P: [u32; 2] = [8, 8];
fn at(x: usize, y: usize) -> usize {
    y * N + x
}
fn rgba(x: f32) -> [f32; 4] {
    [x, x, x, 1.]
}
struct Fixture {
    frame: Frame,
    current: Vec<[f32; 4]>,
    motion: Vec<[f32; 4]>,
    old: Vec<[f32; 4]>,
    depth: Vec<f32>,
    evidence: Vec<[f32; 4]>,
    ids: Vec<u32>,
    records: Vec<[u32; 2]>,
    changes: [u32; 3132],
}
impl Fixture {
    fn new() -> Self {
        let mut s = Self {
            frame: Frame {
                source: [0., 0., 16., 16.],
                output: [0., 0., 16., 16.],
                depth: [0.1, 100., 0., 1.],
                options: [0.9, 0.001, 0.001, 1.],
                reactive: [0., 9., 1., 1.],
                jitter: [0.; 4],
            },
            current: vec![rgba(0.5); N * N],
            motion: vec![[0., 0., 0.5, 1.]; N * N],
            old: vec![rgba(0.75); N * N],
            depth: vec![1. / (0.5 / 0.1 + 0.5 / 100.); N * N],
            evidence: vec![[9., 0., 0., 0.]; N * N],
            ids: vec![5; N * N],
            records: vec![[PRESENT | 5, 0]; N * N],
            changes: [0; 3132],
        };
        s.changes[1] = 1;
        s.changes[2] = 100_000;
        s
    }
    fn resolve(&self, owners: bool) -> Result {
        Input {
            frame: &self.frame,
            source_size: N,
            output_size: N,
            current: &self.current,
            motion: &self.motion,
            native_depth: 0.5,
            old: &self.old,
            old_depth: &self.depth,
            old_evidence: &self.evidence,
            owner: owners.then_some(Owners {
                ids: &self.ids,
                records: &self.records,
                changes: &self.changes,
            }),
        }
        .resolve(P)
    }
}
fn mass() -> f64 {
    let sigma = 0.35_f64;
    (1. + 2. * (-1. / (2. * sigma * sigma)).exp()).powi(2)
        / (2. * std::f64::consts::PI * sigma * sigma)
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-10, "{a} != {b}");
}
#[test]
fn clear_has_full_measure_but_no_owner_or_reusable_history() {
    let mut f = Fixture::new();
    f.frame.depth[3] = 0.;
    f.current.fill([0.; 4]);
    f.current[at(8, 8)] = rgba(1.);
    let result = f.resolve(false);
    let e = (-1. / (2. * 0.35_f64.powi(2))).exp();
    close(result.color[0], 1. / (1. + 2. * e).powi(2));
    close(result.color[3], result.color[0]);
    assert_eq!(result.evidence, [0.; 2]);
    // Residual HDR at alpha zero is still clear at full measure.
    f.current[at(7, 8)] = [60000., 1., 2., 0.];
    assert_eq!(f.resolve(false).color, result.color);
    // A malformed negative sample contributes no mass and forbids history.
    f.current[at(7, 8)] = [-1., 0., 0., 0.];
    let malformed = f.resolve(false);
    assert!(malformed.color[0] > result.color[0]);
    assert_eq!(malformed.evidence, [0.; 2]);
}
#[test]
fn density_and_cap_follow_paired_sum_not_each_tap() {
    let mut f = Fixture::new();
    f.frame.options[0] = 0.99;
    f.frame.reactive[2] = 1. / 16.;
    f.current[at(7, 8)] = rgba(0.);
    f.current[at(9, 8)] = rgba(1.);
    f.motion.fill([0.5 / 16., 0., 0.5, 1.]);
    f.old[at(8, 8)] = rgba(1.);
    f.old[at(9, 8)] = rgba(0.);
    f.evidence[at(8, 8)][0] = 396.;
    f.evidence[at(9, 8)][0] = 20.;
    let expected = (396. / 416. * 13. + 0.5 * mass()) / (13. + mass());
    let result = f.resolve(false);
    close(result.color[0], expected);
    close(result.evidence[0], 13. + mass());
    let cap_each = (99. / 119. * (119. / 32.) + 0.5 * mass()) / (119. / 32. + mass());
    assert!((result.color[0] - cap_each).abs() > 0.05);
    // Different output/source heights: scale prior area, not width squared.
    // The frame carries the host's area ratio; set 1/8 rather than 1/16.
    f.frame.reactive[2] = 1. / 8.;
    close(
        f.resolve(false).color[0],
        (396. / 416. * 26. + 0.5 * mass()) / (26. + mass()),
    );
    f.frame.reactive[3] = 0.;
    close(f.resolve(false).color[0], 0.5);
}
#[test]
fn paired_history_rejects_bad_depth_or_evidence_in_both_projections() {
    for ortho in [false, true] {
        let mut f = Fixture::new();
        f.frame.depth[2] = f32::from(u8::from(ortho));
        if ortho {
            f.depth.fill(50.05);
        }
        f.current[at(7, 8)] = rgba(0.);
        f.current[at(9, 8)] = rgba(1.);
        f.motion.fill([0.5 / 16., 0., 0.5, 1.]);
        f.old[at(8, 8)] = rgba(0.25);
        f.old[at(9, 8)] = rgba(1.);
        f.depth[at(9, 8)] = 0.;
        let expected = (0.25 * 4.5 + 0.5 * mass()) / (4.5 + mass());
        close(f.resolve(false).color[0], expected);
        f.depth[at(9, 8)] = f.depth[at(8, 8)];
        f.evidence[at(9, 8)][1] = 8.;
        close(f.resolve(false).color[0], expected);
        for invalid in [f32::NAN, -1., 65505.] {
            f.evidence[at(9, 8)] = [invalid, 0., 0., 0.];
            close(f.resolve(false).color[0], expected);
        }
        f.evidence[at(9, 8)] = [9., 1.5, 0., 0.];
        close(f.resolve(false).color[0], expected);
    }
}
#[test]
fn every_positive_current_motion_contributor_must_agree() {
    let mut f = Fixture::new();
    f.current[at(7, 8)] = rgba(0.);
    f.current[at(9, 8)] = rgba(1.);
    let reused = f.resolve(false);
    assert!(reused.color[0] > 0.5);
    // The outer 3x3 corner lies outside the former bilinear support.
    f.motion[at(7, 7)][0] = 1. / 16.;
    close(f.resolve(false).color[0], 0.5);
    f.motion[at(7, 7)] = [0., 0., 0.9, 1.];
    close(f.resolve(false).color[0], 0.5);
    f.motion[at(7, 7)] = [f32::NAN, 0., 0.5, 1.];
    close(f.resolve(false).color[0], 0.5);
}
#[test]
fn carried_primary_and_finite_retirement_hold_without_reaction() {
    let mut f = Fixture::new();
    f.records.fill([PRESENT | 7, 5]);
    f.evidence[at(8, 8)][1] = 6.;
    let carried = f.resolve(true);
    assert_eq!(carried.record, [PRESENT | 7, 5]);
    assert_eq!(carried.evidence[1], 7.);
    assert!(carried.color[0] > 0.5);
    f.evidence[at(8, 8)][1] = 7.;
    let retired = f.resolve(true);
    close(retired.color[0], 0.5);
    assert_eq!(retired.record, [PRESENT | 5, 0]);
    assert_eq!(retired.evidence[1], 0.);
    // Current primary is absent in clear support: a partial history can age,
    // opaque history cannot; residual RGB cannot reset the absence clock.
    f.current.fill([10., 10., 10., 0.]);
    f.old.fill([0.2, 0.2, 0.2, 0.5]);
    f.evidence[at(8, 8)][1] = 6.;
    let partial = f.resolve(true);
    assert_eq!(partial.evidence[1], 7.);
    assert!(partial.color[3] > 0.);
    f.evidence[at(8, 8)][1] = 7.;
    assert_eq!(f.resolve(true).color, [0.; 4]);
    f.evidence[at(8, 8)][1] = 0.;
    f.old.fill(rgba(1.));
    assert_eq!(f.resolve(true).record, [0; 2]);
}
#[test]
fn changed_contributors_camera_light_and_origin_refuse_static_persistence() {
    let mut f = Fixture::new();
    let stable = f.resolve(true);
    assert!(stable.color[0] > 0.5);
    for flags in [0, 3] {
        f.changes[1] = flags;
        close(f.resolve(true).color[0], 0.5);
    }
    f.changes[1] = 1;
    f.frame.reactive[3] = 0.;
    close(f.resolve(true).color[0], 0.5);
    f.frame.reactive[3] = 1.;
    f.records.fill([PRESENT | 5, 7]);
    f.changes[0] = class(7);
    f.changes[4] = 1 << 7;
    close(f.resolve(true).color[0], 0.5);
    // Conservative class collision also refuses overflow history.
    f.records
        .fill([PRESENT | 5, PRESENT | class(5) | class(7) | class(9)]);
    close(f.resolve(true).color[0], 0.5);
    f.changes[0] = 0;
    f.changes[4] = 0;
    f.ids[at(7, 7)] = 7 | PRESENT;
    close(f.resolve(true).color[0], 0.5);
}

#[test]
fn split_lattice_preserves_actual_odd_offset_coordinate_mapping() {
    use super::super::super::kernel_oracle::Kernel;
    for scale in [4, 7, 12, 16] {
        let mut strata = [
            std::collections::BTreeSet::new(),
            std::collections::BTreeSet::new(),
        ];
        for phase in 0..8 {
            let k = Kernel::current([885, 501], [3.25, 5.75, 875.5, 489.5], scale, phase).unwrap();
            for (axis, set) in strata.iter_mut().enumerate() {
                set.insert(((k.jitter[axis] * k.source[axis + 2] + 0.5) * 8.).floor() as i32);
            }
            let p = [450.5, 252.5];
            let taps = k.taps(p).unwrap();
            for tap in taps.into_iter().flatten() {
                let source = oracle::source(tap.position, k.output, k.source, k.jitter);
                for (v, t) in source.into_iter().zip(tap.texel) {
                    close(v, f64::from(t) + 0.5);
                }
                assert!(tap.weight.is_finite() && tap.weight >= 0.);
            }
            let constant = k.evidence(p, |_| [0.25, 0.125, 0., 0.5]).unwrap();
            for (v, want) in constant.sum.into_iter().zip([0.25, 0.125, 0., 0.5]) {
                close(v / constant.mass, want);
            }
        }
        for set in strata {
            assert_eq!(set, (0..8).collect());
        }
    }
    // Eight axis strata are not a guarantee for arbitrary thin 2D geometry.
    // No contrast, shimmer or historical appearance gate is asserted here.
}
