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
