use crate::{NavigationError, NavigationMesh, invalid};
use glam::Vec3;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PathRequest {
    pub start: [f32; 3],
    pub end: [f32; 3],
    pub snap_distance: f32,
    pub max_visited: u32,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct NavigationPath {
    /// World-space points on the quantized navigation surface. Voxel/detail
    /// sampling approximates height and rounds steps; use physics for grounding
    /// or exact surface placement. The chosen corridor may include tile bends.
    pub points: Vec<[f32; 3]>,
    pub corridor: Vec<u32>,
    pub visited: u32,
    pub generation: u64,
}
impl NavigationMesh {
    pub fn find_path(&self, q: &PathRequest) -> Result<Option<NavigationPath>, NavigationError> {
        let mut result = self.corridor_path(q)?;
        if let Some(path) = &mut result {
            self.shortcut(path, q.max_visited);
        }
        Ok(result)
    }
    fn corridor_path(&self, q: &PathRequest) -> Result<Option<NavigationPath>, NavigationError> {
        if self.settings.is_none() {
            return Err(invalid("navigation mesh has not been built"));
        }
        if !q
            .start
            .iter()
            .chain(&q.end)
            .all(|x| x.is_finite() && x.abs() <= 100_000.)
            || !q.snap_distance.is_finite()
            || !(0.001..=10.).contains(&q.snap_distance)
            || !(1..=32_768).contains(&q.max_visited)
        {
            return Err(invalid(
                "invalid path position, snap distance or search budget",
            ));
        }
        let Some((first, start)) = self.nearest(q.start, q.snap_distance) else {
            return Ok(None);
        };
        let Some((last, end)) = self.nearest(q.end, q.snap_distance) else {
            return Ok(None);
        };
        let centers: Vec<_> = self
            .polygons
            .iter()
            .map(|p| {
                p.vertices
                    .iter()
                    .map(|p| Vec3::from_array(*p))
                    .sum::<Vec3>()
                    / p.vertices.len() as f32
            })
            .collect();
        let Some(route) = self.search_portals(first, last, start, end, q.max_visited)? else {
            return Ok(None);
        };
        let corridor = route.corridor;
        let mut gates = vec![(start, start)];
        for (from, edge) in route.edges {
            let p = &self.portals[from][edge];
            let a = Vec3::from_array(p.a);
            let b = Vec3::from_array(p.b);
            let forward = centers[p.to] - centers[from];
            let from_mid = a - (a + b) * 0.5;
            gates.push(if forward.x * from_mid.z - forward.z * from_mid.x > 0. {
                (p.a, p.b)
            } else {
                (p.b, p.a)
            });
        }
        gates.push((end, end));
        Ok(Some(NavigationPath {
            points: crate::surface::follow(&self.polygons, &corridor, &gates, &funnel(&gates))?,
            corridor: corridor.into_iter().map(|i| i as u32).collect(),
            visited: route.visited,
            generation: self.generation(),
        }))
    }
    pub(crate) fn nearest(&self, p: [f32; 3], max: f32) -> Option<(usize, [f32; 3])> {
        let p = Vec3::from_array(p);
        let mut best = max * max;
        let mut found = None;
        for (i, poly) in self.polygons.iter().enumerate() {
            for triangle in &poly.triangles {
                let [a, b, c] = triangle.map(Vec3::from_array);
                let point = closest_triangle(p, a, b, c);
                let d = p.distance_squared(point);
                if d <= best && (found.is_none() || d < best) {
                    best = d;
                    found = Some((i, point.to_array()));
                }
            }
        }
        found
    }
}
fn closest_triangle(p: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    // Project onto the triangle plane, then clamp outside points to its three edges.
    let n = (b - a).cross(c - a);
    let n2 = n.length_squared();
    if n2 > 1e-16 {
        let q = p - n * ((p - a).dot(n) / n2);
        if [(a, b), (b, c), (c, a)]
            .into_iter()
            .all(|(u, v)| (v - u).cross(q - u).dot(n) >= -1e-7 * n2)
        {
            return q;
        }
    }
    [(a, b), (b, c), (c, a)]
        .into_iter()
        .map(|(u, v)| {
            let d = v - u;
            let length = d.length_squared();
            if length <= 1e-16 {
                u
            } else {
                u + d * ((p - u).dot(d) / length).clamp(0., 1.)
            }
        })
        .min_by(|a, b| p.distance_squared(*a).total_cmp(&p.distance_squared(*b)))
        .unwrap()
}
fn cross(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> f32 {
    (b[0] - a[0]) * (c[2] - a[2]) - (b[2] - a[2]) * (c[0] - a[0])
}
fn same(a: [f32; 3], b: [f32; 3]) -> bool {
    (a[0] - b[0]).abs() < 1e-5 && (a[2] - b[2]).abs() < 1e-5
}
fn funnel(gates: &[([f32; 3], [f32; 3])]) -> Vec<[f32; 3]> {
    let mut points = vec![gates[0].0];
    let mut apex = gates[0].0;
    let mut left = apex;
    let mut right = apex;
    let (mut li, mut ri) = (0, 0);
    let mut i = 1;
    while i < gates.len() {
        let (nl, nr) = gates[i];
        if cross(apex, right, nr) >= 0. {
            if same(apex, right) || cross(apex, left, nr) < 0. {
                right = nr;
                ri = i;
            } else {
                points.push(left);
                apex = left;
                right = apex;
                left = apex;
                ri = li;
                i = li + 1;
                continue;
            }
        }
        if cross(apex, left, nl) <= 0. {
            if same(apex, left) || cross(apex, right, nl) > 0. {
                left = nl;
                li = i;
            } else {
                points.push(right);
                apex = right;
                left = apex;
                right = apex;
                li = ri;
                i = ri + 1;
                continue;
            }
        }
        i += 1;
    }
    let end = gates.last().unwrap().0;
    if points.last() != Some(&end) {
        points.push(end);
    }
    points
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn astar_corridor_cost_matches_independent_dijkstra_search() {
        // Two pillars leave several competing corridors. The independent oracle
        // uses a linear minimum selection and f64 costs, not A*'s heap/heuristic.
        let settings = crate::NavigationSettings {
            min: [-8., -2., -6.],
            max: [8., 5., 6.],
            cell_size: 0.25,
            tile_cells: 16,
            ..Default::default()
        };
        let mut sources = std::collections::BTreeMap::from([(
            "floor".into(),
            crate::NavigationGeometry {
                vertices: vec![[-8., 0., -6.], [8., 0., -6.], [8., 0., 6.], [-8., 0., 6.]],
                triangles: vec![[0, 2, 1], [0, 3, 2]],
            },
        )]);
        for (id, x, z) in [("north", -2., 1.), ("south", 2., -1.)] {
            sources.insert(
                id.into(),
                crate::NavigationGeometry {
                    vertices: vec![
                        [x - 0.5, 0., z - 2.],
                        [x + 0.5, 0., z - 2.],
                        [x + 0.5, 0., z + 2.],
                        [x - 0.5, 0., z + 2.],
                        [x - 0.5, 3., z - 2.],
                        [x + 0.5, 3., z - 2.],
                        [x + 0.5, 3., z + 2.],
                        [x - 0.5, 3., z + 2.],
                    ],
                    triangles: vec![
                        [4, 6, 5],
                        [4, 7, 6],
                        [0, 1, 2],
                        [0, 2, 3],
                        [0, 4, 5],
                        [0, 5, 1],
                        [1, 5, 6],
                        [1, 6, 2],
                        [2, 6, 7],
                        [2, 7, 3],
                        [3, 7, 4],
                        [3, 4, 0],
                    ],
                },
            );
        }
        let mut nav = NavigationMesh::default();
        nav.rebuild(&settings, &sources).unwrap();
        let mut shortened = false;
        for z in [-3., 0., 3.] {
            let path = nav
                .corridor_path(&PathRequest {
                    start: [-6., 0., z],
                    end: [6., 0., -z],
                    snap_distance: 1.,
                    max_visited: 1000,
                })
                .unwrap()
                .unwrap();
            // Independent Dijkstra oracle over the directed-entry graph, using
            // an unsorted scan rather than the production A* queue/heuristic.
            let first = path.corridor[0] as usize;
            let last = *path.corridor.last().unwrap() as usize;
            let start = Vec3::from_array(path.points[0]);
            let end = Vec3::from_array(*path.points.last().unwrap());
            let mut nodes = vec![(first, first, start)];
            for (from, portals) in nav.portals.iter().enumerate() {
                for p in portals {
                    nodes.push((
                        from,
                        p.to,
                        (Vec3::from_array(p.a) + Vec3::from_array(p.b)) * 0.5,
                    ));
                }
            }
            let mut costs = vec![f64::INFINITY; nodes.len()];
            let mut settled = vec![false; nodes.len()];
            costs[0] = 0.;
            let mut goal = f64::INFINITY;
            while let Some(i) = (0..nodes.len())
                .filter(|&i| !settled[i])
                .min_by(|&a, &b| costs[a].total_cmp(&costs[b]))
            {
                if costs[i] >= goal {
                    break;
                }
                settled[i] = true;
                if nodes[i].1 == last {
                    goal = goal.min(costs[i] + f64::from(nodes[i].2.distance(end)));
                }
                for j in 1..nodes.len() {
                    if nodes[j].0 == nodes[i].1 {
                        costs[j] =
                            costs[j].min(costs[i] + f64::from(nodes[i].2.distance(nodes[j].2)));
                    }
                }
            }
            let mut points = vec![start];
            for pair in path.corridor.windows(2) {
                let gates: Vec<_> = nav.portals[pair[0] as usize]
                    .iter()
                    .filter(|p| p.to == pair[1] as usize)
                    .collect();
                assert_eq!(
                    gates.len(),
                    1,
                    "this oracle fixture has unique polygon-to-polygon gates"
                );
                points.push((Vec3::from_array(gates[0].a) + Vec3::from_array(gates[0].b)) * 0.5);
            }
            points.push(end);
            let actual = points
                .windows(2)
                .map(|p| f64::from(p[0].distance(p[1])))
                .sum::<f64>();
            assert!(
                (actual - goal).abs() < 1e-4,
                "portal A* cost {actual} != Dijkstra {goal}"
            );
            let request = PathRequest {
                start: [-6., 0., z],
                end: [6., 0., -z],
                snap_distance: 1.,
                max_visited: 1000,
            };
            let repaired = nav.find_path(&request).unwrap().unwrap();
            let length = |p: &NavigationPath| {
                p.points
                    .windows(2)
                    .map(|p| (p[1][0] - p[0][0]).hypot(p[1][2] - p[0][2]))
                    .sum::<f32>()
            };
            assert!(length(&repaired) <= length(&path) + 1e-4);
            shortened |= length(&repaired) < length(&path) - 0.001;
            assert!(repaired.visited <= request.max_visited);
            for pair in repaired.corridor.windows(2) {
                assert!(
                    nav.portals[pair[0] as usize]
                        .iter()
                        .any(|p| p.to == pair[1] as usize)
                );
            }
            let limited = nav
                .find_path(&PathRequest {
                    max_visited: path.visited,
                    ..request
                })
                .unwrap()
                .unwrap();
            assert_eq!(
                limited, path,
                "optional smoothing must preserve the original when no visit budget remains"
            );
        }
        assert!(
            shortened,
            "line-of-sight repair should remove a remaining portal-graph detour"
        );
    }
}
