//! Test-only f64 reference for the R4 declared support and evidence measure.
//! Uses explicit sample sets and associated weighted sums, never GPU output as
//! an expected value. The finite fixtures use one known native depth per frame.
use super::super::oracle;
use super::Frame;
use std::collections::BTreeSet;

#[path = "attachment_reference.rs"]
pub(super) mod attachment;

const PRESENT: u32 = 1 << 31;
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Result {
    pub color: [f64; 4],
    pub depth: f64,
    pub record: [u32; 2],
    pub evidence: [f64; 2],
}
pub(super) struct Input<'a> {
    pub frame: &'a Frame,
    pub source_size: usize,
    pub output_size: usize,
    pub current: &'a [[f32; 4]],
    pub motion: &'a [[f32; 4]],
    pub native_depth: f32,
    pub old: &'a [[f32; 4]],
    pub old_depth: &'a [f32],
    pub old_evidence: &'a [[f32; 4]],
    pub owner: Option<Owners<'a>>,
}
#[derive(Clone, Copy)]
pub(super) struct Owners<'a> {
    pub ids: &'a [u32],
    pub records: &'a [[u32; 2]],
    pub changes: &'a [u32; 3132],
}
#[derive(Clone, Copy)]
struct Tap {
    at: usize,
    weight: f64,
}
fn valid(v: [f32; 4]) -> bool {
    v.into_iter().all(|x| x.is_finite() && x >= 0.) && v[3] <= 1.
}
fn clean(v: [f32; 4]) -> [f64; 4] {
    oracle::color(v.map(f64::from))
}
fn inside(p: [f64; 2], r: [f64; 4]) -> bool {
    (0..2).all(|i| p[i] >= r[i] && p[i] < r[i] + r[i + 2])
}
fn index(p: [i32; 2], size: usize) -> usize {
    assert!(p.iter().all(|&x| x >= 0 && (x as usize) < size));
    p[1] as usize * size + p[0] as usize
}
fn class(n: u32) -> u32 {
    1 << (n % 31)
}
fn id(n: u32, limit: u32) -> bool {
    (1..=limit.min(100_000)).contains(&n)
}
fn changed(n: u32, o: Owners<'_>) -> bool {
    o.changes[4 + (n >> 5) as usize] & (1 << (n & 31)) != 0
}
#[derive(Clone, Debug)]
struct Record {
    primary: u32,
    ids: BTreeSet<u32>,
    overflow: Option<u32>,
}
impl Record {
    fn fresh(primary: u32, ids: BTreeSet<u32>) -> Self {
        Self {
            primary,
            ids,
            overflow: None,
        }
    }
    fn read(v: [u32; 2], limit: u32) -> Option<Self> {
        let a = v[0] & 0x1ffff;
        if v[0] & PRESENT == 0 || v[0] & !(PRESENT | 0x1ffff) != 0 || !id(a, limit) {
            return None;
        }
        let mut ids = BTreeSet::from([a]);
        let overflow = if v[1] & PRESENT != 0 {
            let mask = v[1] & !PRESENT;
            if mask & class(a) == 0 {
                return None;
            }
            Some(mask)
        } else {
            if v[1] != 0 {
                if v[1] == a || !id(v[1], limit) {
                    return None;
                }
                ids.insert(v[1]);
            }
            None
        };
        Some(Self {
            primary: a,
            ids,
            overflow,
        })
    }
    fn join(&mut self, other: &Self) {
        self.ids.extend(&other.ids);
        if let Some(mask) = other.overflow {
            self.overflow = Some(self.overflow.unwrap_or(0) | mask);
        }
    }
    fn encode(&self) -> [u32; 2] {
        if self.primary == 0 {
            return [0; 2];
        }
        let mask = self
            .ids
            .iter()
            .fold(self.overflow.unwrap_or(0), |m, &n| m | class(n));
        [
            PRESENT | self.primary,
            if self.overflow.is_some() || self.ids.len() > 2 {
                PRESENT | mask
            } else {
                self.ids
                    .iter()
                    .copied()
                    .find(|&n| n != self.primary)
                    .unwrap_or(0)
            },
        ]
    }
    fn unaffected(
        &self,
        o: Owners<'_>,
        support: u32,
        changed_classes: u32,
        allow_primary: bool,
    ) -> bool {
        if let Some(mask) = self.overflow {
            (mask | support) & changed_classes == 0
        } else {
            self.ids
                .iter()
                .all(|&n| (allow_primary && n == self.primary) || !changed(n, o))
        }
    }
}
impl Input<'_> {
    fn z(&self, d: f32) -> f64 {
        if !(0. ..1.).contains(&d) {
            return 0.;
        }
        let [near, far, ortho, _] = self.frame.depth.map(f64::from);
        let d = f64::from(d);
        if ortho > 0.5 {
            near * (1. - d) + far * d
        } else {
            1. / ((1. - d) / near + d / far)
        }
    }
    fn compatible(&self, a: f64, b: f64) -> bool {
        a <= f64::from(self.frame.depth[1])
            && b <= f64::from(self.frame.depth[1])
            && oracle::matches(
                a,
                b,
                self.frame.options[1].into(),
                self.frame.options[2].into(),
            )
    }
    fn support(&self, p: [f64; 2]) -> Vec<Tap> {
        let source = self.frame.source.map(f64::from);
        let output = self.frame.output.map(f64::from);
        let jitter = [self.frame.jitter[0].into(), self.frame.jitter[1].into()];
        let q = oracle::source(p, output, source, jitter).map(|v| v.floor() as i32);
        let ratio = output[2] / source[2];
        let sigma = 0.35 + 0.325 * (ratio - 2.).max(0.);
        let mut taps = Vec::new();
        for y in q[1] - 1..=q[1] + 1 {
            for x in q[0] - 1..=q[0] + 1 {
                if !inside([f64::from(x) + 0.5, f64::from(y) + 0.5], source) {
                    continue;
                }
                let center: [f64; 2] = std::array::from_fn(|i| {
                    output[i]
                        + ((f64::from([x, y][i]) + 0.5 - source[i]) / source[i + 2] - jitter[i])
                            * output[i + 2]
                });
                let distance: f64 = (0..2).map(|i| (center[i] - p[i]).powi(2)).sum();
                let weight = ratio.powi(2) / (2. * std::f64::consts::PI * sigma.powi(2))
                    * (-distance / (2. * sigma.powi(2))).exp();
                if weight > 0. {
                    taps.push(Tap {
                        at: index([x, y], self.source_size),
                        weight,
                    });
                }
            }
        }
        taps
    }
    fn evidence(&self, at: usize) -> Option<[f64; 2]> {
        let e = self.old_evidence[at];
        (self.frame.reactive[3] >= 0.5
            && (0. ..=65504.).contains(&e[0])
            && (0. ..8.).contains(&e[1])
            && e[1] == e[1].floor())
        .then(|| [e[0].into(), e[1].into()])
    }
    pub fn resolve(&self, pixel: [u32; 2]) -> Result {
        let p = pixel.map(|x| f64::from(x) + 0.5);
        let output = self.frame.output.map(f64::from);
        let mut result = Result::default();
        if !inside(p, output) {
            return result;
        }
        let taps = self.support(p);
        let admitted: Vec<_> = taps.iter().filter(|t| valid(self.current[t.at])).collect();
        let mass: f64 = admitted.iter().map(|t| t.weight).sum();
        let sum: [f64; 4] = std::array::from_fn(|i| {
            admitted
                .iter()
                .map(|t| clean(self.current[t.at])[i] * t.weight)
                .sum()
        });
        if mass > 0. {
            result.color = oracle::color(sum.map(|v| v / mass));
        }
        if admitted.len() != taps.len() {
            return result;
        }
        let z = self.z(self.native_depth);
        let surface: Vec<_> = taps
            .iter()
            .filter(|t| self.current[t.at][3] > 0. && z > 0.)
            .collect();
        let single = z > 0. && taps.iter().all(|t| self.current[t.at][3] == 1.);
        let anchor = surface.first().copied();
        let ratio = output[2] / f64::from(self.frame.source[2]);
        let h = f64::from(self.frame.options[0]).clamp(0., 0.99);
        let cap = h / (1. - h) * ratio.powi(2).min(4.);
        let mut record = None;
        let mut support_classes = 0;
        let mut changed_secondary = false;
        let mut any_changed = false;
        let mut changed_classes = self.owner.map_or(0, |o| o.changes[0]);
        if let Some(o) = self.owner {
            let ids: BTreeSet<_> = surface.iter().map(|t| o.ids[t.at] & !PRESENT).collect();
            if ids.iter().any(|&n| !id(n, o.changes[2])) {
                return result;
            }
            let primary = anchor.map_or(0, |t| o.ids[t.at] & !PRESENT);
            support_classes = ids.iter().fold(0, |m, &n| m | class(n));
            for t in &surface {
                let n = o.ids[t.at] & !PRESENT;
                let c = o.ids[t.at] & PRESENT != 0 || changed(n, o);
                any_changed |= c;
                changed_secondary |= c && n != primary;
            }
            if any_changed {
                changed_classes |= support_classes;
            }
            record = Some(Record::fresh(primary, ids));
            let static_motion = anchor.is_none_or(|anchor| {
                let motion = self.motion[anchor.at];
                motion.iter().all(|x| x.is_finite())
                    && motion[3] >= 0.5
                    && (0..2).all(|i| {
                        (f64::from(self.frame.jitter[i]) + f64::from(motion[i])
                            - f64::from(self.frame.jitter[i + 2]))
                        .abs()
                            * output[i + 2]
                            < 1. / 256.
                    })
            });
            if self.frame.depth[3] >= 0.5
                && o.changes[1] & 1 != 0
                && o.changes[1] & 2 == 0
                && !any_changed
                && static_motion
            {
                let at = index(pixel.map(|x| x as i32), self.output_size);
                let prior = Record::read(o.records[at], o.changes[2]);
                if let (Some(mut prior), Some(e)) = (prior, self.evidence(at)) {
                    // The wire set stores only the first two exact IDs before overflow.
                    // An overflow class is never proof that its primary was seen.
                    let second = surface
                        .iter()
                        .map(|t| o.ids[t.at] & !PRESENT)
                        .find(|&n| n != primary);
                    let seen =
                        primary != 0 && (prior.primary == primary || second == Some(prior.primary));
                    let age = if seen { 0. } else { e[1] + 1. };
                    let old_z = f64::from(self.old_depth[at]);
                    if valid(self.old[at])
                        && self.old[at][3] > 0.
                        && age < 8.
                        && (anchor.is_some() || self.old[at][3] < 1.)
                        && prior.unaffected(o, support_classes, changed_classes, false)
                        && old_z.is_finite()
                        && (0. ..=f64::from(self.frame.depth[1])).contains(&old_z)
                    {
                        let w = e[0].min(cap);
                        let total = w + mass;
                        result.color = oracle::color(std::array::from_fn(|i| {
                            (clean(self.old[at])[i] * w + sum[i]) / total
                        }));
                        result.depth = if record.as_ref().unwrap().primary == prior.primary {
                            z
                        } else {
                            old_z
                        };
                        prior.join(record.as_ref().unwrap());
                        result.record = prior.encode();
                        result.evidence = [total.min(cap), age];
                        return result;
                    }
                }
                if anchor.is_some() {
                    result.depth = z;
                    result.record = record.unwrap().encode();
                    result.evidence = [mass.min(cap), 0.];
                }
                return result;
            }
        }
        if !single {
            return result;
        }
        result.depth = z;
        result.evidence = [mass.min(cap), 0.];
        if let Some(r) = &record {
            result.record = r.encode();
        }
        if self.frame.depth[3] < 0.5 || changed_secondary {
            return result;
        }
        let motion = self.motion[anchor.unwrap().at];
        let expected = self.z(motion[2]);
        if taps.iter().any(|t| {
            let m = self.motion[t.at];
            !m.iter().all(|x| x.is_finite())
                || m[3] < 0.5
                || !self.compatible(self.z(m[2]), expected)
                || (0..2).any(|i| {
                    (f64::from(m[i]) - f64::from(motion[i])).abs() * output[i + 2] >= 1. / 256.
                })
        }) {
            return result;
        }
        let Some(old) = oracle::previous(
            p,
            output,
            [self.frame.jitter[0].into(), self.frame.jitter[1].into()],
            [self.frame.jitter[2].into(), self.frame.jitter[3].into()],
            [motion[0].into(), motion[1].into()],
        ) else {
            return result;
        };
        let mut weights = Vec::new();
        for (xy, b) in oracle::taps(old, output).unwrap() {
            if b <= 0. {
                continue;
            }
            let at = index(xy, self.output_size);
            let Some(e) = self.evidence(at) else {
                continue;
            };
            if !valid(self.old[at])
                || self.old[at][3] < 1.
                || !self.compatible(self.old_depth[at].into(), expected)
            {
                continue;
            }
            if let Some(o) = self.owner {
                let Some(r) = Record::read(o.records[at], o.changes[2]) else {
                    continue;
                };
                if r.primary != record.as_ref().unwrap().primary
                    || !r.unaffected(o, support_classes, changed_classes, true)
                {
                    continue;
                }
                record.as_mut().unwrap().join(&r);
            }
            weights.push((at, b * e[0]));
        }
        let c: f64 = weights.iter().map(|&(_, w)| w).sum();
        if c <= 0. {
            return result;
        }
        let history: [f64; 4] = std::array::from_fn(|i| {
            weights
                .iter()
                .map(|&(at, w)| clean(self.old[at])[i] * w)
                .sum::<f64>()
                / c
        });
        let reaction = history[..3]
            .iter()
            .zip(&result.color)
            .map(|(a, b)| (a - b).abs())
            .fold(0., f64::max)
            / f64::from(self.frame.options[3]);
        let w = (c * f64::from(self.frame.reactive[2]).max(0.)).min(cap)
            * (1. - f64::from(self.frame.reactive[0]) * reaction.clamp(0., 1.));
        result.color = oracle::color(std::array::from_fn(|i| {
            let minimum = admitted
                .iter()
                .map(|t| clean(self.current[t.at])[i])
                .fold(result.color[i], f64::min);
            let maximum = admitted
                .iter()
                .map(|t| clean(self.current[t.at])[i])
                .fold(result.color[i], f64::max);
            (history[i].clamp(minimum, maximum) * w + sum[i]) / (w + mass)
        }));
        result.evidence = [(w + mass).min(cap), 0.];
        if let Some(r) = record {
            result.record = r.encode();
        }
        result
    }
}

/// One half-float rounding step plus a 2^-18 arithmetic allowance. This is a
/// numeric conversion bound, not an appearance threshold. No relative tolerance
/// masks nonzero output when the independently expected value is exactly zero.
pub(super) fn half_bound(expected: f64) -> f64 {
    if expected == 0. {
        return 0.;
    }
    let exponent = expected.abs().log2().floor().max(-14.);
    2_f64.powf(exponent - 10.) + expected.abs() * 2_f64.powi(-18)
}
#[cfg(test)]
#[path = "evidence_tests.rs"]
mod tests;
