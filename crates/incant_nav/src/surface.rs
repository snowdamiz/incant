//! Restore height detail after XZ funnel smoothing, following the ordered corridor.
use crate::{NavigationError, NavigationPolygon, invalid, limit};
use glam::{DVec3, Vec3};
type Point = [f32; 3];
pub(crate) fn follow_with_budget(
    polys: &[NavigationPolygon],
    corridor: &[usize],
    gates: &[(Point, Point)],
    flat: &[Point],
    work: &mut usize,
) -> Result<Vec<Point>, NavigationError> {
    if flat.len() == 1 {
        return Ok(flat.to_vec());
    }
    let mut output = vec![];
    let mut segment = 0;
    let mut begin = 0.;
    let mut cursor = flat[0];
    for (i, &poly) in corridor.iter().enumerate() {
        let gate = gates[i + 1];
        loop {
            let a = flat[segment];
            let b = flat[segment + 1];
            if let Some(t) = intersection(a, b, gate.0, gate.1, begin) {
                let p = lerp(a, b, t);
                append_surface(&mut output, &polys[poly], cursor, p, work)?;
                cursor = p;
                begin = t;
                break;
            }
            append_surface(&mut output, &polys[poly], cursor, b, work)?;
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
fn cross(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}
fn delta(a: Point, b: Point) -> [f64; 2] {
    [
        f64::from(a[0]) - f64::from(b[0]),
        f64::from(a[2]) - f64::from(b[2]),
    ]
}
fn precision(points: &[Point]) -> f64 {
    // Stored positions are f32. Projection/interpolation can round a boundary
    // point by an ULP; allow two ULPs in world space, not a fixed percentage of
    // a short segment or a skinny triangle.
    points
        .iter()
        .flat_map(|p| [p[0], p[2]])
        .map(|x| f64::from(x.abs()))
        .fold(1., f64::max)
        * f64::from(f32::EPSILON)
        * 2.
}
fn lerp(a: Point, b: Point, t: f64) -> Point {
    DVec3::from_array(a.map(f64::from))
        .lerp(DVec3::from_array(b.map(f64::from)), t)
        .as_vec3()
        .to_array()
}
pub(crate) fn intersection(a: Point, b: Point, c: Point, d: Point, begin: f64) -> Option<f64> {
    let r = delta(b, a);
    let s = delta(d, c);
    let offset = delta(c, a);
    let length = r[0].hypot(r[1]);
    let gate_length = s[0].hypot(s[1]);
    let eps = precision(&[a, b, c, d]);
    let on_gate = |point: [f64; 2]| {
        let u = if gate_length == 0. {
            0.
        } else {
            ((point[0] - f64::from(c[0])) * s[0] + (point[1] - f64::from(c[2])) * s[1])
                / (gate_length * gate_length)
        }
        .clamp(0., 1.);
        (point[0] - f64::from(c[0]) - s[0] * u).hypot(point[1] - f64::from(c[2]) - s[1] * u) <= eps
    };
    // Two independently rounded projections may run almost along the portal
    // without their infinite lines meeting within the short segment. Test the
    // actual boundary distance before dividing by that tiny determinant.
    if on_gate([
        f64::from(a[0]) + r[0] * begin,
        f64::from(a[2]) + r[1] * begin,
    ]) {
        return Some(begin);
    }
    if length == 0. {
        return None;
    }
    let teps = eps / length;
    let denominator = cross(r, s);
    let found = if denominator.abs() > f64::EPSILON * length * gate_length * 16. {
        let t = cross(offset, s) / denominator;
        let u = cross(offset, r) / denominator;
        let ueps = eps / gate_length;
        (t >= begin - teps && t <= 1. + teps && u >= -ueps && u <= 1. + ueps)
            .then(|| t.clamp(begin, 1.))
    } else {
        if cross(offset, r).abs() > eps * length {
            return None;
        }
        let t = (offset[0] * r[0] + offset[1] * r[1]) / (length * length);
        let end = delta(d, a);
        let u = (end[0] * r[0] + end[1] * r[1]) / (length * length);
        let low = t.min(u).max(begin);
        let high = t.max(u).min(1.);
        (low <= high + teps).then(|| low.clamp(begin, 1.))
    };
    found.or_else(|| on_gate([f64::from(b[0]), f64::from(b[2])]).then_some(1.))
}
fn barycentric(p: Point, triangle: &[Point; 3]) -> Option<[f64; 3]> {
    let [a, b, c] = *triangle;
    let denominator = cross(delta(b, a), delta(c, a));
    if denominator == 0. {
        return None;
    }
    let v = cross(delta(p, a), delta(c, a)) / denominator;
    let w = cross(delta(b, a), delta(p, a)) / denominator;
    Some([1. - v - w, v, w])
}
fn weight_tolerance(triangle: &[Point; 3]) -> [f64; 3] {
    let [a, b, c] = *triangle;
    let area = cross(delta(b, a), delta(c, a)).abs();
    let eps = precision(triangle);
    [(b, c), (c, a), (a, b)].map(|(u, v)| {
        let d = delta(u, v);
        eps * d[0].hypot(d[1]) / area
    })
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
        let tolerance = weight_tolerance(triangle);
        let (mut low, mut high) = (0_f64, 1_f64);
        for j in 0..3 {
            let d = end[j] - start[j];
            if d.abs() < 1e-14 {
                if start[j] < -tolerance[j] {
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
    breaks.sort_by(f64::total_cmp);
    breaks.dedup_by(|a, b| (*a - *b).abs() < 1e-5);
    for t in breaks {
        let mut p = lerp(a, b, t);
        let height = poly
            .triangles
            .iter()
            .find_map(|tri| {
                let weights = barycentric(p, tri)?;
                let tolerance = weight_tolerance(tri);
                if (0..3).any(|i| weights[i] < -tolerance[i]) {
                    return None;
                }
                let weights = weights.map(|w| w.max(0.));
                let sum: f64 = weights.iter().sum();
                Some(
                    ((weights[0] * f64::from(tri[0][1])
                        + weights[1] * f64::from(tri[1][1])
                        + weights[2] * f64::from(tri[2][1]))
                        / sum) as f32,
                )
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
