//! Trusted one-way native observer. Never deserializes an untrusted solver object.
use super::observe as body_capture;
use incant_physics_snapshot::*;
use serde_json::Value;
fn word(v: &Value) -> u32 {
    v.as_u64().unwrap().try_into().unwrap()
}
fn float(v: &Value) -> u32 {
    (v.as_f64().unwrap() as f32).to_bits()
}
fn rule(v: &Value) -> CombineRule {
    match v.as_str().unwrap() {
        "Average" => CombineRule::Average,
        "Min" => CombineRule::Min,
        "Multiply" => CombineRule::Multiply,
        "Max" => CombineRule::Max,
        other => panic!("unknown rule {other}"),
    }
}
fn groups(v: &Value) -> ColliderGroups {
    ColliderGroups {
        memberships: word(&v["memberships"]),
        filter: word(&v["filter"]),
        test: match v["test_mode"].as_str().unwrap() {
            "And" => GroupTest::And,
            "Or" => GroupTest::Or,
            other => panic!("unknown group {other}"),
        },
    }
}
pub fn collider(v: &Value, d: Dimension) -> ColliderState {
    let f = &v["flags"];
    let m = &v["mprops"];
    let mat = &v["material"];
    let user: u128 = v["user_data"].as_str().unwrap().parse().unwrap();
    ColliderState {
        kind: match v["coll_type"].as_str().unwrap() {
            "Solid" => ColliderKind::Solid,
            "Sensor" => ColliderKind::Sensor,
            other => panic!("unknown collider {other}"),
        },
        position: body_capture::pose(&v["pos"], d),
        mass: if let Some(v) = m.get("Density") {
            ColliderMass::Density(float(v))
        } else if let Some(v) = m.get("Mass") {
            ColliderMass::Mass(float(v))
        } else {
            ColliderMass::Properties(body_capture::local(&m["MassProperties"], d))
        },
        changes: word(&v["changes"]),
        material: ColliderMaterial {
            friction: float(&mat["friction"]),
            restitution: float(&mat["restitution"]),
            friction_rule: rule(&mat["friction_combine_rule"]),
            restitution_rule: rule(&mat["restitution_combine_rule"]),
        },
        active_collision_types: word(&f["active_collision_types"]),
        collision_groups: groups(&f["collision_groups"]),
        solver_groups: groups(&f["solver_groups"]),
        active_hooks: word(&f["active_hooks"]),
        active_events: word(&f["active_events"]),
        enabled: match f["enabled"].as_str().unwrap() {
            "Enabled" => ColliderEnabled::Enabled,
            "Disabled" => ColliderEnabled::Disabled,
            "DisabledByParent" => ColliderEnabled::DisabledByParent,
            other => panic!("unknown enabled {other}"),
        },
        contact_skin: float(&v["contact_skin"]),
        force_event_threshold: float(&v["contact_force_event_threshold"]),
        deformable: !v["deformable_mesh_ref"].is_null(),
        user_data: std::array::from_fn(|i| (user >> (32 * i)) as u32),
    }
}
