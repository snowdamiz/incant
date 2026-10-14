//! Immutable directed entries and adjacency owned by one published mesh.
use super::{Edge, State};
use crate::{MAX_POLYGONS, NavigationMesh};
use glam::Vec3;

const MAX_PORTALS: usize = 262_144;
const MAX_DIRECTED_LINKS: usize = 2 * 128;
// Exact boxed lengths, without allocator overhead. Construction additionally
// holds one polygon cursor per row; old snapshots may retain the previous graph.
const _: () = assert!(
    (MAX_PORTALS + MAX_DIRECTED_LINKS) * (size_of::<State>() + size_of::<usize>())
        + (MAX_POLYGONS + 1) * size_of::<usize>()
        <= 24 * 1024 * 1024
);

#[derive(Default)]
pub(crate) struct PreparedSearch {
    over_limit: bool,
    states: Box<[State]>,
    offsets: Box<[usize]>,
    outgoing: Box<[usize]>,
}

impl PreparedSearch {
    pub(crate) fn new(mesh: &NavigationMesh) -> Self {
        let portals: usize = mesh.portals.iter().map(Vec::len).sum();
        // This is a query-time refusal, after endpoint lookup and the
        // same-polygon shortcut. Do not turn it into a rebuild-time failure.
        if portals > MAX_PORTALS {
            return Self {
                over_limit: true,
                ..Self::default()
            };
        }
        let mut offsets = vec![0; mesh.polygons.len() + 1];
        for (from, edges) in mesh.portals.iter().enumerate() {
            offsets[from + 1] = edges.len();
        }
        for link in &mesh.links {
            offsets[link.first + 1] += 1;
            if link.bidirectional {
                offsets[link.last + 1] += 1;
            }
        }
        for i in 1..offsets.len() {
            offsets[i] += offsets[i - 1];
        }
        let count = *offsets.last().unwrap();
        let mut states = Vec::with_capacity(count);
        let mut outgoing = vec![0; count];
        let mut cursor = offsets[..mesh.polygons.len()].to_vec();
        let mut push = |from: usize, state: State| {
            // Zero is the per-query start. Keep original global state order,
            // including portals before links and forward before reverse.
            outgoing[cursor[from]] = states.len() + 1;
            cursor[from] += 1;
            states.push(state);
        };
        for (from, portals) in mesh.portals.iter().enumerate() {
            for (edge, p) in portals.iter().enumerate() {
                push(
                    from,
                    State {
                        poly: p.to,
                        at: (Vec3::from_array(p.a) + Vec3::from_array(p.b)) * 0.5,
                        edge: Some(Edge::Portal { from, edge }),
                        enter: (Vec3::from_array(p.a) + Vec3::from_array(p.b)) * 0.5,
                        traversal_cost: 0.,
                    },
                );
            }
        }
        for (index, link) in mesh.links.iter().enumerate() {
            for reversed in [false, true] {
                if reversed && !link.bidirectional {
                    continue;
                }
                let (from, to, enter, at) = if reversed {
                    (link.last, link.first, link.end, link.start)
                } else {
                    (link.first, link.last, link.start, link.end)
                };
                let enter = Vec3::from_array(enter);
                let at = Vec3::from_array(at);
                push(
                    from,
                    State {
                        poly: to,
                        at,
                        enter,
                        traversal_cost: enter.distance(at) + link.extra_cost,
                        edge: Some(Edge::Link { index, reversed }),
                    },
                );
            }
        }
        Self {
            over_limit: false,
            states: states.into_boxed_slice(),
            offsets: offsets.into_boxed_slice(),
            outgoing: outgoing.into_boxed_slice(),
        }
    }

    pub(super) fn over_limit(&self) -> bool {
        self.over_limit
    }

    pub(super) fn len(&self) -> usize {
        self.states.len()
    }

    pub(super) fn state(&self, index: usize) -> &State {
        &self.states[index - 1]
    }

    pub(super) fn outgoing(&self, polygon: usize) -> &[usize] {
        &self.outgoing[self.offsets[polygon]..self.offsets[polygon + 1]]
    }
}
