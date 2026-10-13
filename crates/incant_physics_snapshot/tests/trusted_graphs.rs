//! Public native methods on a cloned trusted raw_graph, observed one-way only.
use incant_physics_snapshot::*;
use serde_json::Value;
fn budget() -> Budget {
    Budget::new(MAX_ALLOCATION).unwrap()
}
fn word(value: &Value) -> u32 {
    value.as_u64().unwrap().try_into().unwrap()
}
fn link(value: &Value) -> Option<u32> {
    let value = word(value);
    (value != u32::MAX).then_some(value)
}
fn observe(value: Value, dimension: Dimension, expected: Vec<Vec<(u32, u32)>>) {
    let mut payload = Vec::new();
    let nodes = value["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(index, node)| GraphNode {
            head: [link(&node["next"][0]), link(&node["next"][1])],
            row: index as u32,
        })
        .collect();
    let edges = value["edges"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(index, edge)| {
            payload.push(word(&edge["weight"]));
            GraphEdge {
                node: [word(&edge["node"][0]), word(&edge["node"][1])],
                next: [link(&edge["next"][0]), link(&edge["next"][1])],
                prev: [link(&edge["prev"][0]), link(&edge["prev"][1])],
                row: index as u32,
            }
        })
        .collect();
    let record = GraphRecord {
        identity: Identity {
            dimension,
            build: [41; 32],
        },
        nodes,
        edges,
    };
    let checked = validate_graph(record.clone(), &mut budget()).unwrap();
    let bytes = encode_graph(&checked, &mut budget()).unwrap();
    let loaded = decode_graph(&bytes, record.identity, &mut budget()).unwrap();
    assert_eq!(loaded.record(), &record);
    assert_eq!(encode_graph(&loaded, &mut budget()).unwrap(), bytes);
    for (index, expected) in expected.into_iter().enumerate() {
        let actual: Vec<_> = loaded
            .incident(index as u32)
            .unwrap()
            .map(|edge| {
                (
                    edge,
                    payload[loaded.record().edges[edge as usize].row as usize],
                )
            })
            .collect();
        assert_eq!(actual, expected);
    }
}
macro_rules! check {
    ($graph:ident,$dimension:ident) => {{
        let expected = (0..$graph.raw_nodes().len())
            .map(|i| {
                $graph
                    .edges(ColliderGraphIndex::new(i as u32))
                    .map(|e| (e.id().index() as u32, *e.weight()))
                    .collect()
            })
            .collect();
        observe(
            serde_json::to_value(&$graph).unwrap(),
            Dimension::$dimension,
            expected,
        );
    }};
}
macro_rules! cases {
    ($module:ident,$rapier:ident,$dimension:ident) => {
        mod $module {
            use super::*;
            use $rapier::geometry::{
                ColliderGraphIndex, InteractionGraph, TemporaryInteractionIndex,
            };
            #[test]
            fn native_adjacency_order_survives_edge_and_node_swap_removal() {
                // Type inferred from the public read view; no private path or
                // production mutation adapter is introduced.
                let mut graph = InteractionGraph::<u32, u32>::new().raw_graph().clone();
                check!(graph, $dimension);
                let nodes: Vec<_> = (0..12).map(|i| graph.add_node(i + 100)).collect();
                check!(graph, $dimension);
                let mut value = 0;
                for i in 0..12 {
                    graph.add_edge(nodes[i], nodes[(i + 1) % 12], value);
                    value += 1;
                    if i % 3 == 0 {
                        graph.add_edge(nodes[i], nodes[i], value);
                        value += 1;
                    }
                    if i % 2 == 0 {
                        graph.add_edge(nodes[i], nodes[(i + 1) % 12], value);
                        value += 1;
                    }
                    graph.add_edge(nodes[(i + 1) % 12], nodes[i], value);
                    value += 1;
                }
                check!(graph, $dimension);
                // Nonterminal removals force actual edge-array swap-removal and
                // repair of both endpoint adjacency lists.
                for index in [0, 7, 3] {
                    assert!(
                        graph
                            .remove_edge(TemporaryInteractionIndex::new(index))
                            .is_some()
                    );
                    check!(graph, $dimension);
                }
                let last = graph.raw_edges().len() as u32 - 1;
                assert!(
                    graph
                        .remove_edge(TemporaryInteractionIndex::new(last))
                        .is_some()
                );
                check!(graph, $dimension);
                for node in [2, 0, 5] {
                    assert!(graph.remove_node(ColliderGraphIndex::new(node)).is_some());
                    check!(graph, $dimension);
                }
                let a = graph.add_node(700);
                let b = graph.add_node(701);
                graph.add_edge(a, b, 800);
                graph.add_edge(a, a, 801);
                graph.add_edge(a, b, 802);
                check!(graph, $dimension);
                while !graph.raw_nodes().is_empty() {
                    assert!(graph.remove_node(ColliderGraphIndex::new(0)).is_some());
                    check!(graph, $dimension);
                }
                assert!(graph.raw_edges().is_empty());
            }
        }
    };
}
cases!(planar, rapier2d, Planar);
cases!(spatial, rapier3d, Spatial);
