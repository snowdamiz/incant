//! Trusted one-way native observation; no untrusted native deserialization.
use incant_physics_snapshot::*;
use serde_json::Value;
fn word(v: &Value) -> u32 {
    v.as_u64().unwrap().try_into().unwrap()
}
fn handle(v: &Value) -> Handle {
    Handle {
        slot: word(&v["index"]),
        generation: word(&v["generation"]),
    }
}
fn node(v: &Value, d: Dimension) -> Node {
    let mut mins = [0; 3];
    let mut maxs = [0; 3];
    for i in 0..d as usize {
        mins[i] = (v["mins"][i].as_f64().unwrap() as f32).to_bits();
        maxs[i] = (v["maxs"][i].as_f64().unwrap() as f32).to_bits();
    }
    Node {
        mins,
        maxs,
        children: word(&v["children"]),
        data: word(&v["data"]),
    }
}
pub fn capture(v: &Value, id: Identity) -> (BvhRecord, CoarenaRecord, BroadOwnerRecord) {
    let t = &v["tree"];
    let tree = BvhRecord {
        identity: id,
        nodes: t["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| Wide {
                left: node(&n["left"], id.dimension),
                right: node(&n["right"], id.dimension),
            })
            .collect(),
        parents: t["parents"].as_array().unwrap().iter().map(word).collect(),
        leaves: LeafMap {
            occupied: word(&t["leaf_node_indices"]["n"]),
            slots: t["leaf_node_indices"]["v"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| (!v.is_null()).then(|| word(v)))
                .collect(),
        },
        free: t["free_wide_nodes"]
            .as_array()
            .unwrap()
            .iter()
            .map(word)
            .collect(),
        rebuild_frame: word(&t["optimization"]["rebuild_frame_index"]),
        rebuild_start: word(&t["optimization"]["rebuild_start_index"]),
    };
    let co = v["pair_adjacency"]["data"].as_array().unwrap();
    let layout = CoarenaRecord {
        identity: id,
        rows: co
            .iter()
            .enumerate()
            .map(|(i, v)| CoarenaRow {
                generation: word(&v[0]),
                row: i as u32,
            })
            .collect(),
    };
    let mut neighbors = vec![];
    let adjacency = co
        .iter()
        .map(|v| {
            let start = neighbors.len() as u32;
            neighbors.extend(v[1].as_array().unwrap().iter().map(handle));
            RecordSpan {
                start,
                count: neighbors.len() as u32 - start,
            }
        })
        .collect();
    let record = BroadOwnerRecord {
        identity: id,
        pairs: v["pairs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| BroadPair {
                colliders: [handle(&p[0][0]), handle(&p[0][1])],
                inserted_frame: word(&p[1]),
            })
            .collect(),
        adjacency,
        neighbors,
        previous_leaves: v["prev_updated_leaves"]
            .as_array()
            .unwrap()
            .iter()
            .map(word)
            .collect(),
        pending_aabbs: v["pending_set_aabb"]
            .as_array()
            .unwrap()
            .iter()
            .map(handle)
            .collect(),
        frame: word(&v["frame_index"]),
        changes_since_optimize: word(&v["changes_since_optimize"]),
        reinsert_leaf_updates: v["reinsert_leaf_updates"].as_bool().unwrap(),
        adaptive_margin: v["adaptive_change_detection_margin"].as_bool().unwrap(),
        optimization: match v["optimization_strategy"].as_str().unwrap() {
            "SubtreeOptimizer" => BroadOptimization::Subtree,
            "None" => BroadOptimization::None,
            _ => panic!("unknown native strategy"),
        },
    };
    (tree, layout, record)
}
