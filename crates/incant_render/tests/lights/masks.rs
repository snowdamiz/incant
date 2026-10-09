//! Full-capacity rendering compares mask selection with an unculled reference.
use super::*;

#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn all_4096_local_bits_match_unculled_point_and_spot_lighting() {
    let r = Renderer::headless().unwrap();
    let mut f = fixture();
    light(
        &mut f,
        "DirectionalLight",
        json!({"color":[0.2,0.1,0.3],"intensity":0.04}),
        [0.; 3],
    );
    for i in 0..4096 {
        let position = [
            (i % 32) as f64 * 0.5 - 7.75,
            ((i / 32) % 32) as f64 * 0.5 - 7.75,
            (i / 1024) as f64 * 0.5 + 0.2,
        ];
        let mut value = json!({"color":[0.2+(i%5) as f64*0.15,0.3+(i%3) as f64*0.2,0.7],
            "intensity":0.08,"range":1.2});
        let kind = if i % 3 == 0 {
            value["inner_degrees"] = json!(20);
            value["outer_degrees"] = json!(50);
            "SpotLight"
        } else {
            "PointLight"
        };
        light(&mut f, kind, value, position);
    }
    let clustered = f.scene(&r);
    let all = f
        .scene(&r)
        .with_local_light_selection(LocalLightSelection::All);
    for (w, h) in [(257, 193), (320, 240)] {
        let a = r.screenshot_scene_png(&clustered, w, h).unwrap();
        let b = r.screenshot_scene_png(&all, w, h).unwrap();
        assert_eq!(a, b, "4096 mixed point/spot lights at {w}x{h}");
        assert!(
            center(&a)[0] > 20,
            "the fixture must illuminate visible geometry"
        );
        assert_eq!(a, r.screenshot_scene_png(&clustered, w, h).unwrap());
        coverage::save(&format!("full-mask-{w}x{h}"), &a);
        coverage::save(&format!("full-mask-{w}x{h}-oracle"), &b);
    }
}

#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn last_mask_bit_alone_lights_visible_geometry_after_a_directional_prefix() {
    let r = Renderer::headless().unwrap();
    let mut f = fixture();
    light(
        &mut f,
        "DirectionalLight",
        json!({"color":[0.2,0.1,0.3],"intensity":0.04}),
        [0.; 3],
    );
    // Fixed valid ULIDs guarantee that this is local index 4095 regardless of
    // creation time/randomness. Scene resolution walks the ordered entity map.
    let id = light(
        &mut f,
        "PointLight",
        json!({"color":[0.4,0.8,0.2],"intensity":8,"range":1000}),
        [0., 0., 4.],
    );
    let entities = &mut f.project.scenes.values_mut().next().unwrap().entities;
    let mut last = entities.remove(&id).unwrap();
    last.id = format!("{:026}", 4096);
    entities.insert(last.id.clone(), last);
    let expected = r.screenshot_scene_png(&f.scene(&r), 321, 181).unwrap();
    for i in 1..4096 {
        let mut e = Entity::new("Off-frustum prefix light");
        e.id = format!("{i:026}");
        e.components.insert(
            "PointLight".into(),
            json!({"color":[1,0,0],"intensity":1,"range":1}),
        );
        e.components.insert(
            "Transform".into(),
            json!(Transform {
                translation: [1e6, 0., 0.],
                ..Default::default()
            }),
        );
        f.project
            .scenes
            .values_mut()
            .next()
            .unwrap()
            .entities
            .insert(e.id.clone(), e);
    }
    let clustered = r.screenshot_scene_png(&f.scene(&r), 321, 181).unwrap();
    let all = r
        .screenshot_scene_png(
            &f.scene(&r)
                .with_local_light_selection(LocalLightSelection::All),
            321,
            181,
        )
        .unwrap();
    assert_eq!(
        clustered, expected,
        "the last bit must shade the same as one visible local light"
    );
    assert_eq!(clustered, all);
    assert!(center(&clustered)[1] > 60);
    coverage::save("last-bit-only-321x181", &clustered);
    coverage::save("last-bit-only-321x181-oracle", &all);
    coverage::save("last-bit-only-321x181-single-light", &expected);
}
