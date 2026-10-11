use super::*;
use incant_runtime::{CookedEntity, CookedScene};
use incant_types::{Transform, Velocity};
use serde_json::{Value, json};

const CHILD: &str = "00000000000000000000000002";
const MISSING: &str = "00000000000000000000000003";
fn id(n: u8) -> StableId {
    let mut bytes = [0; 16];
    bytes[15] = n;
    StableId(bytes)
}
fn world() -> NativeWorld {
    NativeWorld::new(
        CookedScene::new(
            id(99),
            2,
            vec![
                CookedEntity {
                    id: id(1),
                    parent: None,
                    transform: Transform {
                        translation: [10., 0., 0.],
                        ..Default::default()
                    },
                    velocity: None,
                },
                CookedEntity {
                    id: id(2),
                    parent: Some(id(1)),
                    transform: Transform::default(),
                    velocity: Some(Velocity {
                        linear: [2., 0., 0.],
                    }),
                },
            ],
        )
        .unwrap(),
    )
}
fn request(checks: Value) -> Value {
    json!({"format":"incant-native-observation","version":1,"checks":checks,"entities":[CHILD,MISSING]})
}
fn parse(request: Value) -> Result<Plan> {
    Plan::parse(&serde_json::to_vec(&request)?, 2)
}
fn check(name: &str, tick: u64, expect: Value) -> Value {
    json!({"name":name,"tick":tick,"expect":expect})
}

#[test]
fn accepted_ticks_observe_local_and_world_motion_without_changing_simulation() {
    let mut plan=parse(request(json!([
        check("final world",2,json!({"type":"vector","entity":CHILD,"property":"world_translation","value":[12,0,0],"tolerance":0})),
        check("startup",0,json!({"type":"entity_count","value":2})),
        check("middle local",1,json!({"type":"vector","entity":CHILD,"property":"translation","value":[1,0,0],"tolerance":0})),
    ]))).unwrap();
    let mut world = world();
    let mut control = self::world();
    for tick in 0..=2 {
        if tick > 0 {
            world.step().unwrap();
            control.step().unwrap();
        }
        plan.observe(&world, &InputFrame::default()).unwrap();
    }
    let report = serde_json::to_value(plan.finish(&world).unwrap()).unwrap();
    assert_eq!(report["passed"], true);
    assert_eq!(report["tick_rate"], 2);
    assert_eq!(report["checks"][0]["name"], "startup");
    assert_eq!(report["checks"][1]["actual"], json!([1., 0., 0.]));
    assert_eq!(
        report["entities"][CHILD]["translation"],
        json!([2., 0., 0.])
    );
    assert_eq!(
        report["entities"][CHILD]["parent"],
        "00000000000000000000000001"
    );
    assert!(report["entities"][MISSING].is_null());
    assert_eq!(world.inspect(id(2)), control.inspect(id(2)));
}

#[test]
fn missing_components_entities_and_actions_are_false_with_null_actuals() {
    let checks = vec![
        check(
            "absent entity",
            0,
            json!({"type":"vector","entity":MISSING,"property":"translation","value":[0,0,0],"tolerance":0}),
        ),
        check(
            "absent physics",
            0,
            json!({"type":"vector","entity":CHILD,"property":"physics_velocity","value":[0,0,0],"tolerance":0}),
        ),
        check(
            "absent velocity",
            0,
            json!({"type":"vector","entity":"00000000000000000000000001","property":"velocity","value":[0,0,0],"tolerance":0}),
        ),
        check(
            "absent action",
            0,
            json!({"type":"action","name":"jump","active":false}),
        ),
        check(
            "count mismatch",
            0,
            json!({"type":"entity_count","value":3}),
        ),
        check(
            "explicit absence",
            0,
            json!({"type":"exists","entity":MISSING,"value":false}),
        ),
    ];
    let world = world();
    let mut plan = parse(request(json!(checks))).unwrap();
    plan.observe(&world, &InputFrame::default()).unwrap();
    let report = serde_json::to_value(plan.finish(&world).unwrap()).unwrap();
    assert_eq!(report["passed"], false);
    let checks = report["checks"].as_array().unwrap();
    for outcome in &checks[..4] {
        assert_eq!(outcome["passed"], false);
        assert!(outcome["actual"].is_null());
    }
    assert_eq!(checks[4]["actual"], 2);
    assert_eq!(checks[5]["passed"], true);
}

#[test]
fn action_edges_and_tolerance_compare_requested_fields_only() {
    let world = world();
    let mut input = InputFrame::default();
    input.actions.insert(
        "jump".into(),
        ActionState {
            kind: incant_input::ActionKind::Button,
            value: [1., 0.],
            active: true,
            pressed: true,
            released: false,
        },
    );
    let mut plan = parse(request(json!([
        check(
            "pass",
            0,
            json!({"type":"action","name":"jump","pressed":true,"value":[0.99,0],"tolerance":0.011})
        ),
        check(
            "edge mismatch",
            0,
            json!({"type":"action","name":"jump","released":true})
        ),
        check(
            "value mismatch",
            0,
            json!({"type":"action","name":"jump","value":[0.99,0],"tolerance":0.001})
        ),
    ])))
    .unwrap();
    plan.observe(&world, &input).unwrap();
    let report = serde_json::to_value(plan.finish(&world).unwrap()).unwrap();
    assert_eq!(report["checks"][0]["passed"], false);
    assert_eq!(report["checks"][1]["passed"], true);
    assert_eq!(report["checks"][2]["passed"], false);
    assert_eq!(report["checks"][1]["actual"]["pressed"], true);
}

#[test]
fn unknown_schema_names_ticks_ulids_and_bounds_fail_before_execution() {
    let base = request(json!([check(
        "check",
        0,
        json!({"type":"entity_count","value":2})
    )]));
    let mut invalid = Vec::new();
    for (key, value) in [
        ("format", json!("future")),
        ("version", json!(2)),
        ("unknown", json!(0)),
        ("entities", json!([CHILD, CHILD])),
        ("entities", json!(["7zzzzzzzzzzzzzzzzzzzzzzzzz"])),
    ] {
        let mut v = base.clone();
        v[key] = value;
        invalid.push(v);
    }
    for (key, value) in [
        ("tick", json!(3)),
        ("name", json!("")),
        ("unknown", json!(0)),
    ] {
        let mut v = base.clone();
        v["checks"][0][key] = value;
        invalid.push(v);
    }
    let mut duplicate = base.clone();
    duplicate["checks"] = json!([base["checks"][0], base["checks"][0]]);
    invalid.push(duplicate);
    let mut excessive = base.clone();
    excessive["checks"] = json!(vec![base["checks"][0].clone(); 257]);
    invalid.push(excessive);
    let mut empty = base.clone();
    empty["checks"] = json!([]);
    empty["entities"] = json!([]);
    invalid.push(empty);
    for v in invalid {
        assert!(parse(v.clone()).is_err(), "accepted {v}");
    }
    assert!(Plan::parse(&vec![b' '; MAX_PLAN_BYTES + 1], 2).is_err());
}

#[test]
fn invalid_expectations_are_rejected() {
    for expect in [
        json!({"type":"vector","entity":CHILD,"property":"translation","value":[0,0,0],"tolerance":-1}),
        json!({"type":"vector","entity":CHILD,"property":"unknown","value":[0,0,0],"tolerance":0}),
        json!({"type":"action","name":"jump"}),
        json!({"type":"action","name":"jump","value":[0,0]}),
        json!({"type":"action","name":"jump","active":true,"tolerance":0}),
        json!({"type":"action","name":"bad action","active":false}),
        json!({"type":"entity_count","value":2,"extra":0}),
    ] {
        assert!(parse(request(json!([check("bad", 0, expect)]))).is_err());
    }
}

#[test]
fn skipped_or_incomplete_requested_ticks_are_errors() {
    let mut world = world();
    world.step().unwrap();
    let value = request(json!([check(
        "startup",
        0,
        json!({"type":"entity_count","value":2})
    )]));
    let mut plan = parse(value.clone()).unwrap();
    assert!(plan.observe(&world, &InputFrame::default()).is_err());
    assert!(parse(value).unwrap().finish(&world).is_err());
}

#[test]
fn observation_options_preserve_strict_legacy_and_spatial_argument_shapes() {
    for count in [8, 12] {
        let plain = vec![String::from("arg"); count];
        assert_eq!(split_args(&plain).1, None);
        let mut with = plain.clone();
        with.extend(["--observe".into(), "plan.json".into()]);
        let (args, path) = split_args(&with);
        assert_eq!(args, plain);
        assert_eq!(path, Some("plan.json"));
        with.extend(["--observe".into(), "other.json".into()]);
        assert!(crate::session::settings(split_args(&with).0).is_err());
    }
}
