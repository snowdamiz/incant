//! A* over directed portal entries. Polygon centroids are not traversal points:
//! charging for them can steer a route toward arbitrary tile-row boundaries.
use crate::{NavigationError, NavigationMesh, invalid, limit};
use glam::Vec3;
mod prepared;
pub(crate) use prepared::PreparedSearch;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod uncached;
use std::{cmp::Ordering, collections::BinaryHeap};

#[derive(Clone, Copy)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub(crate) enum Edge {
    Portal { from: usize, edge: usize },
    Link { index: usize, reversed: bool },
}
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub(crate) struct Route {
    pub corridor: Vec<usize>,
    pub edges: Vec<Edge>,
    pub visited: u32,
}
struct State {
    poly: usize,
    at: Vec3,
    edge: Option<Edge>,
    enter: Vec3,
    traversal_cost: f32,
}
#[derive(Clone, Copy)]
struct Entry {
    f: f32,
    g: f32,
    index: usize,
}
impl PartialEq for Entry {
    fn eq(&self, b: &Self) -> bool {
        self.cmp(b) == Ordering::Equal
    }
}
impl Eq for Entry {}
impl PartialOrd for Entry {
    fn partial_cmp(&self, b: &Self) -> Option<Ordering> {
        Some(self.cmp(b))
    }
}
impl Ord for Entry {
    fn cmp(&self, b: &Self) -> Ordering {
        b.f.total_cmp(&self.f)
            .then_with(|| b.g.total_cmp(&self.g))
            .then_with(|| b.index.cmp(&self.index))
    }
}

impl NavigationMesh {
    pub(crate) fn search_portals(
        &self,
        first: usize,
        last: usize,
        start: [f32; 3],
        end: [f32; 3],
        max_visited: u32,
    ) -> Result<Option<Route>, NavigationError> {
        if first == last {
            return Ok(Some(Route {
                corridor: vec![first],
                edges: vec![],
                visited: 1,
            }));
        }
        if self.search.over_limit() {
            return Err(limit("navigation exceeds 262144 directed portal states"));
        }
        let end = Vec3::from_array(end);
        let start = State {
            poly: first,
            at: Vec3::from_array(start),
            edge: None,
            enter: Vec3::from_array(start),
            traversal_cost: 0.,
        };
        // Index zero remains the query's virtual start; all cached indices retain
        // the old portal/link enumeration and therefore identical heap ties.
        let state = |index| {
            if index == 0 {
                &start
            } else {
                self.search.state(index)
            }
        };
        let state_count = self.search.len() + 1;
        let mut costs = vec![f32::INFINITY; state_count];
        let mut previous = vec![None; state_count];
        let mut queue = BinaryHeap::new();
        costs[0] = 0.;
        queue.push(Entry {
            f: state(0).at.distance(end),
            g: 0.,
            index: 0,
        });
        let mut visited = 0;
        while let Some(Entry { g, index, .. }) = queue.pop() {
            if g > costs[index] {
                continue;
            }
            if visited == max_visited {
                return Err(limit("path search visit budget"));
            }
            visited += 1;
            let current = state(index);
            if current.poly == last {
                // The heuristic is the exact final in-polygon segment here, so
                // the first settled entry into the goal polygon is optimal.
                let mut corridor = vec![];
                let mut edges = vec![];
                let mut cursor = index;
                while cursor != 0 {
                    corridor.push(state(cursor).poly);
                    edges.push(
                        state(cursor)
                            .edge
                            .ok_or_else(|| invalid("missing portal entry"))?,
                    );
                    cursor = previous[cursor].ok_or_else(|| invalid("missing search parent"))?;
                    if corridor.len() >= 4096 {
                        return Err(limit("path corridor exceeds 4096 polygons"));
                    }
                }
                corridor.push(first);
                corridor.reverse();
                edges.reverse();
                return Ok(Some(Route {
                    corridor,
                    edges,
                    visited,
                }));
            }
            for &next in self.search.outgoing(current.poly) {
                let cost = g + current.at.distance(state(next).enter) + state(next).traversal_cost;
                if cost < costs[next] {
                    costs[next] = cost;
                    previous[next] = Some(index);
                    queue.push(Entry {
                        f: cost + state(next).at.distance(end),
                        g: cost,
                        index: next,
                    });
                }
            }
        }
        Ok(None)
    }
}
