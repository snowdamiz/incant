//! One-way native observation only; never native serde deserialization.
use incant_physics_snapshot::*;
use serde_json::Value;
fn word(value: &Value) -> u32 {
    value.as_u64().unwrap().try_into().unwrap()
}
fn node(value: &Value, dim: Dimension) -> Node {
    let mut mins = [0; 3];
    let mut maxs = [0; 3];
    for i in 0..dim as usize {
        mins[i] = (value["mins"][i].as_f64().unwrap() as f32).to_bits();
        maxs[i] = (value["maxs"][i].as_f64().unwrap() as f32).to_bits();
    }
    Node {
        mins,
        children: word(&value["children"]),
        maxs,
        data: word(&value["data"]),
    }
}
pub fn capture(value: Value, identity: Identity) -> BvhRecord {
    let dimension = identity.dimension;
    let nodes = value["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| Wide {
            left: node(&v["left"], dimension),
            right: node(&v["right"], dimension),
        })
        .collect();
    let parents = value["parents"]
        .as_array()
        .unwrap()
        .iter()
        .map(word)
        .collect();
    let slots = value["leaf_node_indices"]["v"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| if v.is_null() { None } else { Some(word(v)) })
        .collect();
    let free = value["free_wide_nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(word)
        .collect();
    BvhRecord {
        identity,
        nodes,
        parents,
        leaves: LeafMap {
            occupied: word(&value["leaf_node_indices"]["n"]),
            slots,
        },
        free,
        rebuild_frame: word(&value["optimization"]["rebuild_frame_index"]),
        rebuild_start: word(&value["optimization"]["rebuild_start_index"]),
    }
}
