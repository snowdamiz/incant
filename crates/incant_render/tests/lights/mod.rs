use super::materials::{Fixture, center, near};
use incant_doc::{Entity, TextureUsage, Transform};
use incant_render::Renderer;
use serde_json::{Value, json};

fn fixture() -> Fixture {
    let mut f =
        Fixture::new(json!({"pbrMetallicRoughness":{"metallicFactor":0,"roughnessFactor":1}}));
    f.environment(&[0, 0, 0, 255].repeat(2), 2, 1, TextureUsage::Linear);
    f
}
fn light(f: &mut Fixture, kind: &str, value: Value, position: [f64; 3]) -> String {
    let mut e = Entity::new("Test light");
    e.components.insert(kind.into(), value);
    e.components.insert(
        "Transform".into(),
        json!(Transform {
            translation: position,
            ..Default::default()
        }),
    );
    let id = e.id.clone();
    f.project
        .scenes
        .values_mut()
        .next()
        .unwrap()
        .entities
        .insert(id.clone(), e);
    id
}
fn component(f: &mut Fixture, id: &str, kind: &str, value: Value) {
    f.project
        .scenes
        .values_mut()
        .next()
        .unwrap()
        .entities
        .get_mut(id)
        .unwrap()
        .components
        .insert(kind.into(), value);
}
fn shot(f: &Fixture, r: &Renderer, name: &str) -> [u8; 4] {
    let scene = f.scene(r);
    assert_eq!(scene.stats().diagnostic_entities, 0);
    let png = r.screenshot_scene_png(&scene, 320, 180).unwrap();
    if let Some(directory) = std::env::var_os("INCANT_LIGHT_EVIDENCE") {
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            std::path::Path::new(&directory).join(format!("{name}.png")),
            &png,
        )
        .unwrap();
    }
    center(&png)
}
fn linear(v: u8) -> f64 {
    let x = v as f64 / 255.;
    if x <= 0.04045 {
        x / 12.92
    } else {
        ((x + 0.055) / 1.055).powf(2.4)
    }
}
#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn directional_point_and_spot_lights_obey_authored_physical_parameters() {
    let r = Renderer::headless().unwrap();
    let mut f = fixture();
    let id = light(
        &mut f,
        "PointLight",
        json!({"color":[1,1,1],"intensity":8,"range":1000}),
        [0., 0., 4.],
    );
    let near_point = shot(&f, &r, "point-near");
    component(
        &mut f,
        &id,
        "Transform",
        json!(Transform {
            translation: [0., 0., 8.],
            ..Default::default()
        }),
    );
    let far_point = shot(&f, &r, "point-far");
    let ratio = linear(near_point[0]) / linear(far_point[0]);
    assert!(
        (3.8..4.2).contains(&ratio),
        "{near_point:?}/{far_point:?}: {ratio}"
    );
    component(
        &mut f,
        &id,
        "PointLight",
        json!({"color":[1,1,1],"intensity":8,"range":1}),
    );
    near(shot(&f, &r, "point-outside-range"), [0, 0, 0, 255], 0);
    let e = f
        .project
        .scenes
        .values_mut()
        .next()
        .unwrap()
        .entities
        .get_mut(&id)
        .unwrap();
    e.components.remove("PointLight");
    component(
        &mut f,
        &id,
        "Transform",
        json!(Transform {
            translation: [0., 0., 4.],
            ..Default::default()
        }),
    );
    component(
        &mut f,
        &id,
        "SpotLight",
        json!({"color":[1,1,1],"intensity":8,"range":1000,"inner_degrees":10,"outer_degrees":20}),
    );
    near(shot(&f, &r, "spot-center"), near_point, 1);
    component(
        &mut f,
        &id,
        "Transform",
        json!(Transform {
            translation: [0., 0., 4.],
            rotation: [0., 1., 0., 0.],
            ..Default::default()
        }),
    );
    near(shot(&f, &r, "spot-away"), [0, 0, 0, 255], 0);
    let e = f
        .project
        .scenes
        .values_mut()
        .next()
        .unwrap()
        .entities
        .get_mut(&id)
        .unwrap();
    e.components.remove("SpotLight");
    component(&mut f, &id, "Transform", json!(Transform::default()));
    component(
        &mut f,
        &id,
        "DirectionalLight",
        json!({"color":[1,0,0],"intensity":0.5}),
    );
    let directional = shot(&f, &r, "directional-red");
    assert!(
        directional[0] > 80 && directional[1] == 0 && directional[2] == 0,
        "{directional:?}"
    );
    component(
        &mut f,
        &id,
        "Transform",
        json!(Transform {
            translation: [99., 77., -12.],
            scale: [3., 4., 5.],
            ..Default::default()
        }),
    );
    near(f.pixel(&r), directional, 0);
    component(
        &mut f,
        &id,
        "DirectionalLight",
        json!({"color":[1,1,1],"intensity":0}),
    );
    near(shot(&f, &r, "explicit-light-disabled"), [0, 0, 0, 255], 0);
}
#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn clustered_lists_and_overflow_match_total_light_energy_and_retain_old_scenes() {
    let r = Renderer::headless().unwrap();
    let mut f = fixture();
    let id = light(
        &mut f,
        "PointLight",
        json!({"color":[1,1,1],"intensity":8,"range":1000}),
        [0., 0., 4.],
    );
    let reference = f.scene(&r);
    let first = r.screenshot_scene_png(&reference, 320, 180).unwrap();
    f.project
        .scenes
        .values_mut()
        .next()
        .unwrap()
        .entities
        .remove(&id);
    for count in [64, 65, 128] {
        let mut candidate = f.project.clone();
        for _ in 0..count {
            light(
                &mut f,
                "PointLight",
                json!({"color":[1,1,1],"intensity":8./count as f64,"range":1000}),
                [0., 0., 4.],
            );
        }
        near(
            shot(&f, &r, &format!("cluster-{count}-lights")),
            center(&first),
            1,
        );
        let current = f.scene(&r);
        let a = r.screenshot_scene_png(&current, 320, 180).unwrap();
        let b = r.screenshot_scene_png(&current, 320, 180).unwrap();
        assert_eq!(a, b, "repeated clustered sums must be stable");
        std::mem::swap(&mut f.project, &mut candidate);
    }
    assert_eq!(first, r.screenshot_scene_png(&reference, 320, 180).unwrap());
    // A light-only entity never becomes diagnostic cube geometry.
    assert_eq!(reference.stats().diagnostic_entities, 0);
}

#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn spatial_clusters_match_brute_force_lighting_across_tile_edges_and_resize() {
    let r = Renderer::headless().unwrap();
    let mut f = fixture();
    // Localized colored lights straddle screen tile edges and depth slices.
    // The all-light oracle below uses the exact same BRDF but bypasses spatial
    // selection by forcing list overflow with zero-energy, global-range lights.
    for z in [0.4, 1.2, 2.8] {
        for y in [-2.5, -0.4, 1.5] {
            for x in [-2.5, -1., 0.5, 2.] {
                light(
                    &mut f,
                    "PointLight",
                    json!({"color":[0.3+0.05*x,0.5+0.08*y,0.4],"intensity":0.15,"range":1.8}),
                    [x, y, z],
                );
            }
        }
    }
    let clustered = f.scene(&r);
    for _ in 0..65 {
        light(
            &mut f,
            "PointLight",
            json!({"color":[0,0,0],"intensity":0,"range":10000}),
            [0., 0., 0.],
        );
    }
    let brute_force = f.scene(&r);
    for (w, h) in [(320, 180), (257, 191), (640, 360), (320, 180)] {
        let a = r.screenshot_scene_png(&clustered, w, h).unwrap();
        let b = r.screenshot_scene_png(&brute_force, w, h).unwrap();
        assert_eq!(
            a, b,
            "spatial clusters differ from brute-force lighting at {w}x{h}"
        );
        if let Some(directory) = std::env::var_os("INCANT_LIGHT_EVIDENCE") {
            std::fs::create_dir_all(&directory).unwrap();
            std::fs::write(
                std::path::Path::new(&directory).join(format!("spatial-clusters-{w}x{h}.png")),
                &a,
            )
            .unwrap();
        }
    }
}
