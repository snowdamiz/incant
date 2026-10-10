//! Restore height detail after XZ funnel smoothing, following the ordered corridor.
use crate::{NavigationError, NavigationPolygon, invalid, limit};
use glam::Vec3;
type Point = [f32; 3];
pub(crate) fn follow(
    polys: &[NavigationPolygon],
    corridor: &[usize],
    gates: &[(Point, Point)],
    flat: &[Point],
) -> Result<Vec<Point>, NavigationError> {
    if flat.len() == 1 {
        return Ok(flat.to_vec());
    }
    let mut output = vec![];
    let mut work = 0usize;
    let mut segment = 0;
    let mut begin = 0.;
    let mut cursor = flat[0];
    for (i, &poly) in corridor.iter().enumerate() {
        let gate = gates[i + 1];
        loop {
            let a = flat[segment];
            let b = flat[segment + 1];
            if let Some(t) = intersection(a, b, gate.0, gate.1, begin) {
                let p = Vec3::from_array(a).lerp(Vec3::from_array(b), t).to_array();
                append_surface(&mut output, &polys[poly], cursor, p, &mut work)?;
                cursor = p;
                begin = t;
                break;
            }
            append_surface(&mut output, &polys[poly], cursor, b, &mut work)?;
            segment += 1;
            if segment + 1 >= flat.len() {
                return Err(invalid("funnel did not cross its navigation corridor"));
            }
            begin = 0.;
            cursor = b;
        }
    }
    Ok(output)
}
fn cross(a: [f32; 2], b: [f32; 2]) -> f32 {
    a[0] * b[1] - a[1] * b[0]
}
fn intersection(a: Point, b: Point, c: Point, d: Point, begin: f32) -> Option<f32> {
    let r = [b[0] - a[0], b[2] - a[2]];
    let s = [d[0] - c[0], d[2] - c[2]];
    let delta = [c[0] - a[0], c[2] - a[2]];
    let denominator = cross(r, s);
    let eps = 1e-5;
    if denominator.abs() > 1e-8 {
        let t = cross(delta, s) / denominator;
        let u = cross(delta, r) / denominator;
        if t >= begin - eps && t <= 1. + eps && u >= -eps && u <= 1. + eps {
            Some(t.clamp(begin, 1.))
        } else {
            None
        }
    } else {
        let length = r[0] * r[0] + r[1] * r[1];
        if length < 1e-12 {
            return if (a[0] - c[0]).abs() + (a[2] - c[2]).abs() < eps {
                Some(begin)
            } else {
                None
            };
        }
        if cross(delta, r).abs() > eps * length.sqrt() {
            return None;
        }
        let t = (delta[0] * r[0] + delta[1] * r[1]) / length;
        let u = ((d[0] - a[0]) * r[0] + (d[2] - a[2]) * r[1]) / length;
        let low = t.min(u).max(begin);
        let high = t.max(u).min(1.);
        if low <= high + eps {
            Some(low.clamp(begin, 1.))
        } else {
            None
        }
    }
}
fn barycentric(p: Point, triangle: &[Point; 3]) -> Option<[f32; 3]> {
    let [a, b, c] = *triangle;
    let denominator = cross([b[0] - a[0], b[2] - a[2]], [c[0] - a[0], c[2] - a[2]]);
    if denominator.abs() < 1e-10 {
        return None;
    }
    let v = cross([p[0] - a[0], p[2] - a[2]], [c[0] - a[0], c[2] - a[2]]) / denominator;
    let w = cross([b[0] - a[0], b[2] - a[2]], [p[0] - a[0], p[2] - a[2]]) / denominator;
    Some([1. - v - w, v, w])
}
fn append_surface(
    out: &mut Vec<Point>,
    poly: &NavigationPolygon,
    a: Point,
    b: Point,
    work: &mut usize,
) -> Result<(), NavigationError> {
    let triangles = poly.triangles.len();
    *work = work
        .saturating_add(triangles.saturating_mul(triangles.saturating_mul(2).saturating_add(3)));
    if *work > 2_000_000 {
        return Err(limit("path height-detail work budget"));
    }
    let mut breaks = vec![0., 1.];
    for triangle in &poly.triangles {
        let Some(start) = barycentric(a, triangle) else {
            continue;
        };
        let end = barycentric(b, triangle).unwrap();
        let (mut low, mut high) = (0_f32, 1_f32);
        for j in 0..3 {
            let d = end[j] - start[j];
            if d.abs() < 1e-8 {
                if start[j] < -1e-4 {
                    high = -1.;
                }
            } else if d > 0. {
                low = low.max(-start[j] / d);
            } else {
                high = high.min(-start[j] / d);
            }
        }
        if low <= high + 1e-5 && low <= 1. && high >= 0. {
            breaks.push(low.clamp(0., 1.));
            breaks.push(high.clamp(0., 1.));
        }
    }
    breaks.sort_by(f32::total_cmp);
    breaks.dedup_by(|a, b| (*a - *b).abs() < 1e-5);
    for t in breaks {
        let mut p = Vec3::from_array(a).lerp(Vec3::from_array(b), t).to_array();
        let height = poly
            .triangles
            .iter()
            .find_map(|tri| {
                let weights = barycentric(p, tri)?;
                if weights.iter().any(|w| *w < -1e-3) {
                    return None;
                }
                Some(weights[0] * tri[0][1] + weights[1] * tri[1][1] + weights[2] * tri[2][1])
            })
            .ok_or_else(|| invalid("smoothed navigation segment left its polygon"))?;
        p[1] = height;
        append(out, p);
        if out.len() > 4096 {
            return Err(limit("detailed path exceeds 4096 points"));
        }
    }
    Ok(())
}
fn append(points: &mut Vec<Point>, p: Point) {
    if points
        .last()
        .is_some_and(|a| Vec3::from_array(*a).distance_squared(Vec3::from_array(p)) < 1e-10)
    {
        return;
    }
    if points.len() >= 2 {
        let a = Vec3::from_array(points[points.len() - 2]);
        let b = Vec3::from_array(points[points.len() - 1]);
        let c = Vec3::from_array(p);
        let d = c - a;
        let len = d.length_squared();
        if len > 1e-10 {
            let t = (b - a).dot(d) / len;
            if (0. ..=1.).contains(&t) && (a + d * t).distance_squared(b) < 1e-8 {
                points.pop();
            }
        }
    }
    points.push(p);
}
