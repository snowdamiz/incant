//! Construct adjacency only from its immutable complete checked topology.
use super::*;
use crate::pipeline::checked_restore::allocation::*;
use incant_physics_snapshot::ValidatedGraph;
impl<N, E> Graph<N, E> {
    pub(crate) fn checkpoint_plan(value: &ValidatedGraph) -> ConstructionResult<AllocationPlan> {
        let mut p = AllocationPlan::array::<Node<N>>(value.record().nodes.len())?;
        p.append(AllocationPlan::array::<Edge<E>>(
            value.record().edges.len(),
        )?)?;
        Ok(p)
    }
    pub(crate) fn from_checkpoint(
        value: &ValidatedGraph,
        permit: &mut AllocationPermit,
        mut node: impl FnMut(u32, &mut AllocationPermit) -> ConstructionResult<N>,
        mut edge: impl FnMut(u32, &mut AllocationPermit) -> ConstructionResult<E>,
    ) -> ConstructionResult<Self> {
        let mut nodes = permit.vector(value.record().nodes.len())?;
        let mut edges = permit.vector(value.record().edges.len())?;
        let ix = |v: Option<u32>| v.map_or_else(EdgeIndex::end, EdgeIndex::new);
        for n in &value.record().nodes {
            nodes.push(Node {
                weight: node(n.row, permit)?,
                next: n.head.map(ix),
            });
        }
        for e in &value.record().edges {
            edges.push(Edge {
                weight: edge(e.row, permit)?,
                next: e.next.map(ix),
                prev: e.prev.map(ix),
                node: e.node.map(NodeIndex::new),
            });
        }
        Ok(Self { nodes, edges })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use incant_physics_snapshot::*;
    fn identity() -> Identity {
        Identity {
            dimension: if crate::math::DIM == 2 {
                Dimension::Planar
            } else {
                Dimension::Spatial
            },
            build: [7; 32],
        }
    }
    fn capture(a: &Graph<u32, u32>) -> ValidatedGraph {
        let ix = |e: EdgeIndex| (e != EdgeIndex::end()).then_some(e.0);
        let record = GraphRecord {
            identity: identity(),
            nodes: a
                .nodes
                .iter()
                .enumerate()
                .map(|(i, n)| GraphNode {
                    head: n.next.map(ix),
                    row: i as u32,
                })
                .collect(),
            edges: a
                .edges
                .iter()
                .enumerate()
                .map(|(i, e)| GraphEdge {
                    node: e.node.map(|n| n.0),
                    next: e.next.map(ix),
                    prev: e.prev.map(ix),
                    row: i as u32,
                })
                .collect(),
        };
        let mut b = Budget::new(65536).unwrap();
        let v = validate_graph(record, &mut b).unwrap();
        let bytes = encode_graph(&v, &mut b).unwrap();
        decode_graph(&bytes, identity(), &mut b).unwrap()
    }
    fn equal(a: &Graph<u32, u32>, b: &Graph<u32, u32>) {
        assert_eq!(capture(a).record(), capture(b).record());
        assert_eq!(
            a.nodes.iter().map(|n| n.weight).collect::<Vec<_>>(),
            b.nodes.iter().map(|n| n.weight).collect::<Vec<_>>()
        );
        assert_eq!(
            a.edges.iter().map(|e| e.weight).collect::<Vec<_>>(),
            b.edges.iter().map(|e| e.weight).collect::<Vec<_>>()
        );
    }
    #[test]
    fn checked_storage_graph_decode_construct_swap_removal_parallel_edges_and_selfloops() {
        let mut a = Graph::with_capacity(4, 6);
        let nodes: Vec<_> = (0..4).map(|i| a.add_node(i + 10)).collect();
        for (i, (s, t)) in [(0, 0), (0, 1), (0, 1), (1, 2), (2, 0), (3, 2)]
            .into_iter()
            .enumerate()
        {
            a.add_edge(nodes[s], nodes[t], i as u32 + 20);
        }
        a.remove_edge(EdgeIndex::new(2));
        let v = capture(&a);
        let plan = Graph::<u32, u32>::checkpoint_plan(&v).unwrap();
        let mut p = AllocationBudget::new(plan.bytes(), plan.allocations())
            .admit(plan)
            .unwrap();
        let mut b = Graph::from_checkpoint(
            &v,
            &mut p,
            |i, _| Ok(a.nodes[i as usize].weight),
            |i, _| Ok(a.edges[i as usize].weight),
        )
        .unwrap();
        p.finish().unwrap();
        equal(&a, &b);
        assert_eq!(
            a.remove_edge(EdgeIndex::new(0)),
            b.remove_edge(EdgeIndex::new(0))
        );
        equal(&a, &b);
        assert_eq!(a.remove_node(nodes[1]), b.remove_node(nodes[1]));
        equal(&a, &b);
        assert_eq!(a.add_node(99), b.add_node(99));
        assert_eq!(
            a.add_edge(NodeIndex::new(0), NodeIndex::new(3), 101),
            b.add_edge(NodeIndex::new(0), NodeIndex::new(3), 101)
        );
        equal(&a, &b);
        let mut bad = v.record().clone();
        bad.edges[0].next[0] = Some(0);
        assert!(validate_graph(bad, &mut Budget::new(65536).unwrap()).is_err());
    }
}
