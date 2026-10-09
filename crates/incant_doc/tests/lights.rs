use incant_doc::*;
use serde_json::{Value, json};
fn project(kind: &str, value: Value) -> Project {
    let mut p = Project::empty("Lights");
    let mut s = Scene::new("World");
    let mut e = Entity::new("Light");
    e.components.insert(kind.into(), value);
    s.entities.insert(e.id.clone(), e);
    p.scenes.insert(s.id.clone(), s);
    p
}
#[test]
fn punctual_lights_roundtrip_and_reject_invalid_units_angles_and_types() {
    for (kind, value) in [
        (
            "DirectionalLight",
            json!(DirectionalLight {
                color: [1.; 3],
                intensity: 2.
            }),
        ),
        (
            "PointLight",
            json!(PointLight {
                color: [1., 0.5, 0.],
                intensity: 10.,
                range: 8.
            }),
        ),
        (
            "SpotLight",
            json!(SpotLight {
                color: [1.; 3],
                intensity: 20.,
                range: 10.,
                inner_degrees: 15.,
                outer_degrees: 30.
            }),
        ),
    ] {
        let p = project(kind, value.clone());
        assert_eq!(Project::from_text(&p.canonical_text().unwrap()).unwrap(), p);
        for (field, bad) in [
            ("color", json!([1.1, 0., 0.])),
            ("color", json!([-0.1, 0., 0.])),
            ("intensity", json!(-1.)),
            ("intensity", json!(1_000_001.)),
            ("unknown", json!(true)),
        ] {
            let mut v = value.clone();
            v[field] = bad;
            assert!(project(kind, v).validate().is_err());
        }
    }
    let valid =
        json!({"color":[1,1,1],"intensity":1,"range":1,"inner_degrees":0,"outer_degrees":45});
    for (field, bad) in [
        ("range", 0.),
        ("range", 10001.),
        ("inner_degrees", 45.),
        ("inner_degrees", 46.),
        ("outer_degrees", 0.),
        ("outer_degrees", 90.),
    ] {
        let mut v = valid.clone();
        v[field] = json!(bad);
        assert!(project("SpotLight", v).validate().is_err());
    }
    let mut p = project(
        "PointLight",
        json!({"color":[1,1,1],"intensity":1,"range":1}),
    );
    p.scenes
        .values_mut()
        .next()
        .unwrap()
        .entities
        .values_mut()
        .next()
        .unwrap()
        .components
        .insert(
            "DirectionalLight".into(),
            json!({"color":[1,1,1],"intensity":0}),
        );
    assert!(p.validate().is_err());
}
