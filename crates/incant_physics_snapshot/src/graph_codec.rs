use crate::{
    Budget, Error, GraphEdge, GraphNode, GraphRecord, Identity, MAX_BYTES, Result, ValidatedGraph,
    arena_codec::{read_identity, write_identity},
    codec::Reader,
    graph::graph_lengths,
    validate_graph, vector,
};
const MAGIC: &[u8; 8] = b"ICGRF001";
const HEADER: usize = 56;
const NONE: u32 = u32::MAX;
fn length(nodes: usize, edges: usize) -> Result<usize> {
    nodes
        .checked_mul(12)
        .and_then(|v| edges.checked_mul(28).and_then(|e| v.checked_add(e)))
        .and_then(|v| v.checked_add(HEADER))
        .filter(|v| *v <= MAX_BYTES)
        .ok_or(Error::Limit)
}
fn reference(value: u32) -> Option<u32> {
    (value != NONE).then_some(value)
}

pub fn decode_graph(
    bytes: &[u8],
    expected: Identity,
    budget: &mut Budget,
) -> Result<ValidatedGraph> {
    if !(HEADER..=MAX_BYTES).contains(&bytes.len()) {
        return Err(Error::Length);
    }
    let mut read = Reader { bytes, position: 0 };
    read_identity(&mut read, MAGIC, expected)?;
    let n = read.word()? as usize;
    let e = read.word()? as usize;
    if read.word()? != 0 {
        return Err(Error::Header);
    }
    graph_lengths(n, e)?;
    if bytes.len() != length(n, e)? {
        return Err(Error::Length);
    }
    budget.charge(
        n.checked_mul(size_of::<GraphNode>())
            .and_then(|v| {
                e.checked_mul(size_of::<GraphEdge>())
                    .and_then(|e| v.checked_add(e))
            })
            .ok_or(Error::Limit)?,
    )?;
    let mut nodes = vector(n)?;
    let mut edges = vector(e)?;
    for _ in 0..n {
        nodes.push(GraphNode {
            head: [reference(read.word()?), reference(read.word()?)],
            row: read.word()?,
        });
    }
    for _ in 0..e {
        edges.push(GraphEdge {
            node: [read.word()?, read.word()?],
            next: [reference(read.word()?), reference(read.word()?)],
            prev: [reference(read.word()?), reference(read.word()?)],
            row: read.word()?,
        });
    }
    validate_graph(
        GraphRecord {
            identity: expected,
            nodes,
            edges,
        },
        budget,
    )
}
pub fn encode_graph(value: &ValidatedGraph, budget: &mut Budget) -> Result<Vec<u8>> {
    let record = value.record();
    let len = length(record.nodes.len(), record.edges.len())?;
    budget.charge(len)?;
    let mut bytes = vector(len)?;
    write_identity(&mut bytes, MAGIC, record.identity);
    for word in [record.nodes.len() as u32, record.edges.len() as u32, 0] {
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    for node in &record.nodes {
        for word in [
            node.head[0].unwrap_or(NONE),
            node.head[1].unwrap_or(NONE),
            node.row,
        ] {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
    }
    for edge in &record.edges {
        for word in edge
            .node
            .into_iter()
            .chain(edge.next.map(|v| v.unwrap_or(NONE)))
            .chain(edge.prev.map(|v| v.unwrap_or(NONE)))
            .chain([edge.row])
        {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
    }
    Ok(bytes)
}
