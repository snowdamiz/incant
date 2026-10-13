//! Inert adjacency only; contact payloads and solver conflict ownership remain unchecked.
use crate::{Budget, Error, Identity, MAX_ITEMS, Result, vector};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum Side {
    Outgoing = 0,
    Incoming = 1,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GraphNode {
    pub head: [Option<u32>; 2],
    pub row: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GraphEdge {
    pub node: [u32; 2],
    pub next: [Option<u32>; 2],
    pub prev: [Option<u32>; 2],
    pub row: u32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphRecord {
    pub identity: Identity,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}
pub struct ValidatedGraph {
    record: GraphRecord,
}
impl ValidatedGraph {
    pub fn record(&self) -> &GraphRecord {
        &self.record
    }
    pub fn adjacency(&self, node: u32, side: Side) -> Option<Adjacency<'_>> {
        let head = self.record.nodes.get(node as usize)?.head[side as usize];
        Some(Adjacency {
            edges: &self.record.edges,
            next: head,
            side: side as usize,
        })
    }
    /// Native undirected order: outgoing list, then incoming, self-loops once.
    pub fn incident(&self, node: u32) -> Option<Incident<'_>> {
        Some(Incident {
            outgoing: self.adjacency(node, Side::Outgoing)?,
            incoming: self.adjacency(node, Side::Incoming)?,
            node,
        })
    }
}
pub struct Adjacency<'a> {
    edges: &'a [GraphEdge],
    next: Option<u32>,
    side: usize,
}
impl Iterator for Adjacency<'_> {
    type Item = u32;
    fn next(&mut self) -> Option<Self::Item> {
        let index = self.next?;
        self.next = self.edges[index as usize].next[self.side];
        Some(index)
    }
}
impl std::iter::FusedIterator for Adjacency<'_> {}
pub struct Incident<'a> {
    outgoing: Adjacency<'a>,
    incoming: Adjacency<'a>,
    node: u32,
}
impl Iterator for Incident<'_> {
    type Item = u32;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(index) = self.outgoing.next() {
            return Some(index);
        }
        let edges = self.incoming.edges;
        let node = self.node;
        self.incoming
            .by_ref()
            .find(|&index| edges[index as usize].node[0] != node)
    }
}
impl std::iter::FusedIterator for Incident<'_> {}

pub(crate) fn graph_lengths(nodes: usize, edges: usize) -> Result<()> {
    if nodes > MAX_ITEMS || edges > MAX_ITEMS || (nodes == 0 && edges != 0) {
        return Err(Error::Length);
    }
    Ok(())
}
fn edge_reference(value: Option<u32>, edges: usize) -> bool {
    value.is_none_or(|i| (i as usize) < edges)
}

pub fn validate_graph(record: GraphRecord, budget: &mut Budget) -> Result<ValidatedGraph> {
    let n = record.nodes.len();
    let e = record.edges.len();
    graph_lengths(n, e)?;
    // All scalar references are bounded before scratch or traversal.
    if record
        .nodes
        .iter()
        .any(|v| v.head.iter().any(|i| !edge_reference(*i, e)))
        || record.edges.iter().any(|v| {
            v.node.iter().any(|i| *i as usize >= n)
                || v.next.iter().chain(&v.prev).any(|i| !edge_reference(*i, e))
        })
    {
        return Err(Error::Adjacency);
    }
    budget.charge(
        e.checked_mul(3)
            .and_then(|v| v.checked_add(n))
            .ok_or(Error::Limit)?,
    )?;
    let mut node_rows = vector::<u8>(n)?;
    node_rows.resize(n, 0);
    let mut edge_rows = vector::<u8>(e)?;
    edge_rows.resize(e, 0);
    let mut seen = vector::<[u8; 2]>(e)?;
    seen.resize(e, [0; 2]);
    for node in &record.nodes {
        let row = node.row as usize;
        if row >= n || node_rows[row] != 0 {
            return Err(Error::Ownership);
        }
        node_rows[row] = 1;
    }
    for edge in &record.edges {
        let row = edge.row as usize;
        if row >= e || edge_rows[row] != 0 {
            return Err(Error::Ownership);
        }
        edge_rows[row] = 1;
    }
    for (node_index, node) in record.nodes.iter().enumerate() {
        for (side, head) in node.head.into_iter().enumerate() {
            let mut previous = None;
            let mut next = head;
            while let Some(index) = next {
                let index = index as usize;
                let edge = record.edges[index];
                if seen[index][side] != 0
                    || edge.node[side] as usize != node_index
                    || edge.prev[side] != previous
                {
                    return Err(Error::Adjacency);
                }
                seen[index][side] = 1;
                previous = Some(index as u32);
                next = edge.next[side];
            }
        }
    }
    if seen.iter().any(|marks| *marks != [1, 1]) {
        return Err(Error::Adjacency);
    }
    Ok(ValidatedGraph { record })
}
