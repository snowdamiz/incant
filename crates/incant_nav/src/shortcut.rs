//! Bounded line-of-sight corridor repair. Every shortcut walks real portals,
//! including zero-length transitions at tile corners; disconnected floors cannot
//! be crossed merely because their XZ projections overlap.
use crate::{NavigationMesh, NavigationPath, NavigationPolygon};
use std::{cmp::Ordering, collections::BinaryHeap};
type Point = [f32; 3];
#[derive(Clone, Copy)]
struct Visit {
    at: f64,
    poly: usize,
}
impl PartialEq for Visit {
    fn eq(&self, b: &Self) -> bool {
        self.cmp(b) == Ordering::Equal
    }
}
impl Eq for Visit {}
impl PartialOrd for Visit {
    fn partial_cmp(&self, b: &Self) -> Option<Ordering> {
        Some(self.cmp(b))
    }
}
impl Ord for Visit {
    fn cmp(&self, b: &Self) -> Ordering {
        b.at.total_cmp(&self.at)
            .then_with(|| b.poly.cmp(&self.poly))
    }
}
fn cross(a: Point, b: Point, c: Point) -> f64 {
    (f64::from(b[0]) - f64::from(a[0])) * (f64::from(c[2]) - f64::from(a[2]))
        - (f64::from(b[2]) - f64::from(a[2])) * (f64::from(c[0]) - f64::from(a[0]))
}
fn interval(poly: &NavigationPolygon, a: Point, b: Point) -> Option<(f64, f64)> {
    let points = &poly.vertices;
    let area = (1..points.len() - 1)
        .map(|i| cross(points[0], points[i], points[i + 1]))
        .sum::<f64>();
    if area.abs() < 1e-10 {
        return None;
    }
    let sign = area.signum();
    let (mut low, mut high) = (0_f64, 1_f64);
    for i in 0..points.len() {
        let p = points[i];
        let q = points[(i + 1) % points.len()];
        let start = cross(p, q, a) * sign;
        let end = cross(p, q, b) * sign;
        let delta = end - start;
        if delta.abs() < 1e-10 {
            if start < -1e-5 {
                return None;
            }
        } else if delta > 0. {
            low = low.max(-start / delta);
        } else {
            high = high.min(-start / delta);
        }
    }
    (low <= high + 1e-5 && low <= 1. + 1e-5 && high >= -1e-5).then_some((low.max(0.), high.min(1.)))
}
struct Walk {
    corridor: Vec<usize>,
    gates: Vec<(Point, Point)>,
}
impl NavigationMesh {
    fn line_walk(
        &self,
        start: usize,
        end: usize,
        a: Point,
        b: Point,
        budget: &mut u32,
    ) -> Option<Walk> {
        let n = self.polygons.len();
        let mut best = vec![f64::INFINITY; n];
        let mut parent = vec![None; n];
        let mut queue = BinaryHeap::new();
        best[start] = 0.;
        queue.push(Visit {
            at: 0.,
            poly: start,
        });
        while let Some(Visit { at, poly }) = queue.pop() {
            if at > best[poly] {
                continue;
            }
            *budget = budget.checked_sub(1)?;
            let Some((low, high)) = interval(&self.polygons[poly], a, b) else {
                continue;
            };
            if at < low - 1e-5 || at > high + 1e-5 {
                continue;
            }
            if poly == end && high >= 1. - 1e-5 {
                let mut corridor = vec![end];
                let mut edges = vec![];
                let mut cursor = end;
                while cursor != start {
                    let (from, edge): (usize, usize) = parent[cursor]?;
                    let p = &self.portals[from][edge];
                    edges.push((p.a, p.b));
                    corridor.push(from);
                    cursor = from;
                    if corridor.len() > 4096 {
                        return None;
                    }
                }
                corridor.reverse();
                edges.reverse();
                let mut gates = vec![(a, a)];
                gates.extend(edges);
                gates.push((b, b));
                return Some(Walk { corridor, gates });
            }
            for (edge, p) in self.portals[poly].iter().enumerate() {
                let Some(t) = crate::surface::intersection(a, b, p.a, p.b, at) else {
                    continue;
                };
                if t > high + 1e-5 || t >= best[p.to] {
                    continue;
                }
                best[p.to] = t;
                parent[p.to] = Some((poly, edge));
                queue.push(Visit { at: t, poly: p.to });
            }
        }
        None
    }
    pub(crate) fn shortcut(&self, path: &mut NavigationPath, max_visited: u32) {
        if path.points.len() < 3 {
            return;
        }
        // Shortcuts use only the visit allowance left by A*. Low budgets preserve
        // the valid original path rather than making optional smoothing fail.
        let Some(mut budget) = max_visited.checked_sub(path.visited) else {
            return;
        };
        let available = budget;
        if let Some((points, corridor)) = self.improved_path(path, &mut budget) {
            path.points = points;
            path.corridor = corridor.into_iter().map(|i| i as u32).collect();
        }
        path.visited += available - budget;
    }
    fn improved_path(
        &self,
        path: &NavigationPath,
        budget: &mut u32,
    ) -> Option<(Vec<Point>, Vec<usize>)> {
        let mut work = 0usize;
        let lookup_work = self
            .polygons
            .iter()
            .map(|p| p.triangles.len())
            .sum::<usize>();
        let mut attempts = 0;
        let mut points = vec![];
        let mut corridor = vec![];
        let mut begin = 0;
        while begin + 1 < path.points.len() {
            let a = path.points[begin];
            work += lookup_work;
            if work > 2_000_000 {
                return None;
            }
            let (start, _) = self.nearest(a, 0.001)?;
            let mut found = None;
            for last in (begin + 1..path.points.len()).rev() {
                attempts += 1;
                if attempts > 64 || *budget == 0 {
                    return None;
                }
                let b = path.points[last];
                work += lookup_work + self.polygons.len();
                if work > 2_000_000 {
                    return None;
                }
                let (end, _) = self.nearest(b, 0.001)?;
                if let Some(walk) = self.line_walk(start, end, a, b, budget) {
                    let Ok(segment) = crate::surface::follow_with_budget(
                        &self.polygons,
                        &walk.corridor,
                        &walk.gates,
                        &[a, b],
                        &mut work,
                    ) else {
                        continue;
                    };
                    found = Some((last, walk.corridor, segment));
                    break;
                }
            }
            let (last, mut piece, mut segment) = found?;
            if !points.is_empty() {
                segment.remove(0);
            }
            if corridor.last() == piece.first() {
                piece.remove(0);
            }
            points.extend(segment);
            corridor.extend(piece);
            begin = last;
            if points.len() > 4096 || corridor.len() > 4096 {
                return None;
            }
        }
        let length = |p: &[Point]| {
            p.windows(2)
                .map(|p| (p[1][0] - p[0][0]).hypot(p[1][2] - p[0][2]))
                .sum::<f32>()
        };
        if length(&points) > length(&path.points) + 1e-4 {
            return None;
        }
        Some((points, corridor))
    }
}
