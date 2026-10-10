//! Funnel each walkable leg independently; never smooth across an authored link.
use crate::search::{Edge, Route};
use crate::{NavigationError, NavigationMesh, NavigationPath, OffMeshTraversal, limit};
use glam::Vec3;
type Point = [f32; 3];
impl NavigationMesh {
    pub(crate) fn surface_route(
        &self,
        route: Route,
        start: Point,
        end: Point,
    ) -> Result<NavigationPath, NavigationError> {
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
        let mut points = vec![];
        let mut traversals = vec![];
        let mut leg = vec![route.corridor[0]];
        let mut gates = vec![(start, start)];
        let mut work = 0;
        for edge in route.edges {
            match edge {
                Edge::Portal { from, edge } => {
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
                    leg.push(p.to);
                }
                Edge::Link { index, reversed } => {
                    let link = &self.links[index];
                    let (a, b, destination) = if reversed {
                        (link.end, link.start, link.first)
                    } else {
                        (link.start, link.end, link.last)
                    };
                    gates.push((a, a));
                    self.append_leg(&mut points, &leg, &gates, &mut work)?;
                    let from_index = (points.len() - 1) as u32;
                    points.push(b);
                    traversals.push(OffMeshTraversal {
                        link_id: link.id.clone(),
                        from_index,
                        to_index: from_index + 1,
                        reversed,
                    });
                    leg = vec![destination];
                    gates = vec![(b, b)];
                }
            }
        }
        gates.push((end, end));
        self.append_leg(&mut points, &leg, &gates, &mut work)?;
        Ok(NavigationPath {
            points,
            traversals,
            corridor: route.corridor.into_iter().map(|i| i as u32).collect(),
            visited: route.visited,
            generation: self.generation(),
        })
    }
    fn append_leg(
        &self,
        points: &mut Vec<Point>,
        leg: &[usize],
        gates: &[(Point, Point)],
        work: &mut usize,
    ) -> Result<(), NavigationError> {
        let flat = crate::path::funnel(gates);
        let detailed = crate::surface::follow_with_budget(&self.polygons, leg, gates, &flat, work)?;
        for point in detailed {
            if points.last().is_none_or(|last| {
                Vec3::from_array(*last).distance_squared(Vec3::from_array(point)) >= 1e-10
            }) {
                points.push(point);
            }
        }
        if points.len() > 4096 {
            return Err(limit("linked path exceeds 4096 points"));
        }
        Ok(())
    }
}
