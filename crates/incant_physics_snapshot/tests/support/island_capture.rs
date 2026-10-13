//! One-way observation of actual pinned IslandManager serde fields.
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
fn optional(v: &Value) -> Option<u32> {
    (!v.is_null()).then(|| word(v))
}
fn push_bodies(v: &Value, out: &mut Vec<Handle>) -> RecordSpan {
    let start = out.len() as u32;
    out.extend(v["bodies"].as_array().unwrap().iter().map(handle));
    RecordSpan {
        start,
        count: out.len() as u32 - start,
    }
}
pub fn capture(v: &Value, identity: Identity) -> IslandTable {
    let p = &v["persistent"];
    let mut r = IslandTable {
        identity,
        active_epoch: word(&v["active_set_epoch"]),
        awake: optional(&v["awake_island"]),
        sleep_scan_stamp: word(&p["sleep_scan_stamp"]),
        pending_split: optional(&p["split_island"]),
        bootstrapped: p["bootstrapped"].as_bool().unwrap(),
        pending_removals: p["removal_journal"].as_array().unwrap().len() as u32,
        joint_links: 0,
        joint_locations: p["joint_link_locs"].as_array().unwrap().len() as u32,
        active_live: word(&v["islands"]["n"]),
        persistent_live: word(&p["islands"]["n"]),
        active: vec![],
        active_free: v["free_islands"]
            .as_array()
            .unwrap()
            .iter()
            .map(word)
            .collect(),
        active_bodies: vec![],
        persistent: vec![],
        persistent_free: p["free_islands"]
            .as_array()
            .unwrap()
            .iter()
            .map(word)
            .collect(),
        persistent_bodies: vec![],
        contacts: vec![],
        locations: vec![],
    };
    for a in v["islands"]["v"].as_array().unwrap() {
        r.active
            .push((!a.is_null()).then(|| push_bodies(a, &mut r.active_bodies)));
    }
    for a in p["islands"]["v"].as_array().unwrap() {
        r.persistent.push((!a.is_null()).then(|| {
            let bodies = push_bodies(a, &mut r.persistent_bodies);
            let start = r.contacts.len() as u32;
            for c in a["contact_links"].as_array().unwrap() {
                r.contacts.push(IslandContact {
                    edge: word(&c["edge_id"]),
                    bodies: [handle(&c["body1"]), handle(&c["body2"])],
                });
            }
            r.joint_links += a["joint_links"].as_array().unwrap().len() as u32;
            PersistentIslandRow {
                bodies,
                contacts: RecordSpan {
                    start,
                    count: r.contacts.len() as u32 - start,
                },
                constraint_remove_count: word(&a["constraint_remove_count"]),
                split_denied_until: word(&a["split_denied_until"]),
                sleeping: a["sleeping"].as_bool().unwrap(),
            }
        }));
    }
    for loc in p["contact_link_locs"].as_array().unwrap() {
        let id = word(&loc[0]);
        let index = word(&loc[1]);
        if id == u32::MAX {
            assert_eq!(index, u32::MAX);
            r.locations.push(None);
        } else {
            r.locations.push(Some(IslandLocation { island: id, index }));
        }
    }
    r
}
