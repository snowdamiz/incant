//! Trusted native observer; never a decoder for project or save data.
use incant_physics_snapshot::*;
use serde_json::Value;
fn word(v: &Value) -> u32 {
    v.as_u64().unwrap().try_into().unwrap()
}
fn words(v: &Value) -> Vec<u32> {
    v.as_array().unwrap().iter().map(word).collect()
}
pub fn capture(
    v: &Value,
    identity: Identity,
    stamp: u32,
    slots: &[(u32, bool)],
) -> RigidOwnerRecord {
    let mut references = vec![];
    let mut buckets = vec![];
    for b in v["solver_contact_graph"]["buckets"].as_array().unwrap() {
        let start = references.len() as u32;
        for r in b.as_array().unwrap() {
            references.push(SolverReference {
                edge: word(&r["edge"]),
                manifold: word(&r["manifold"]),
            });
        }
        buckets.push(RecordSpan {
            start,
            count: references.len() as u32 - start,
        });
    }
    RigidOwnerRecord {
        identity,
        sleep_scan_stamp: stamp,
        sleep_scan: slots
            .iter()
            .map(|(stamp, eligible)| SleepScanSlot {
                stamp: *stamp,
                eligible: *eligible,
            })
            .collect(),
        sleep_scan_touched: 0,
        body_color_masks: v["body_solver_color_masks"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().parse().unwrap())
            .collect(),
        pair_hints: words(&v["pair_solver_hints"]),
        buckets,
        references,
        graph_valid: v["solver_graph_valid"].as_bool().unwrap(),
        graph_epoch: word(&v["solver_graph_epoch"]),
        graph_multibody_epoch: word(&v["solver_graph_mb_epoch"]),
        force_valid: v["force_list_valid"].as_bool().unwrap(),
        force_pairs: words(&v["force_event_pairs"]),
        force_positions: words(&v["force_event_pos"]),
        force_flagged: words(&v["force_event_flagged"]),
    }
}
