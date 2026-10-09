use super::materials::{Fixture, emissive};
use incant_doc::{Camera, Entity, Transform};
use incant_render::{LocalLightSelection, Renderer, SceneError};
use serde_json::json;

fn insert(f: &mut Fixture, entity: Entity) -> String {
    let id = entity.id.clone();
    f.project
        .scenes
        .values_mut()
        .next()
        .unwrap()
        .entities
        .insert(id.clone(), entity);
    id
}
fn camera(f: &mut Fixture, pose: Transform) -> String {
    let mut e = Entity::new("Capture camera");
    e.components.insert("Transform".into(), json!(pose));
    e.components.insert(
        "Camera".into(),
        json!(Camera {
            fov_degrees: 60.,
            near: 0.1,
            far: 100.
        }),
    );
    insert(f, e)
}
fn edit(f: &mut Fixture, id: &str, kind: &str, value: serde_json::Value) {
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
fn capture(f: &Fixture, r: &Renderer, id: &str, name: &str) -> Vec<u8> {
    let png = r
        .screenshot_scene_png(&f.scene(r).with_camera(id).unwrap(), 321, 241)
        .unwrap();
    save(name, &png);
    png
}
fn save(name: &str, png: &[u8]) {
    if let Some(dir) = std::env::var_os("INCANT_CAMERA_EVIDENCE") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(std::path::Path::new(&dir).join(format!("{name}.png")), png).unwrap();
    }
}
fn red_pixels(png: &[u8]) -> usize {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(png))
        .read_info()
        .unwrap();
    let mut pixels = vec![0; decoder.output_buffer_size().unwrap()];
    let info = decoder.next_frame(&mut pixels).unwrap();
    pixels[..info.buffer_size()]
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| p[0] > p[1].saturating_add(50))
        .count()
}

#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn authored_camera_changes_projection_clips_geometry_and_retains_its_selected_pose() {
    let r = Renderer::headless().unwrap();
    let mut f = Fixture::new(emissive([1., 0., 0.], 1., "OPAQUE"));
    let preview = r.screenshot_scene_png(&f.scene(&r), 321, 241).unwrap();
    let id = camera(
        &mut f,
        Transform {
            translation: [0., 0., 8.],
            ..Default::default()
        },
    );
    let prepared = f.scene(&r);
    assert_eq!(
        prepared.stats().diagnostic_entities,
        0,
        "camera-only entities are not cubes"
    );
    assert_eq!(
        r.screenshot_scene_png(&prepared, 321, 241).unwrap(),
        preview
    );
    assert!(matches!(
        f.scene(&r).with_camera("unknown"),
        Err(SceneError::MissingCamera(_))
    ));
    let retained = prepared.with_camera(&id).unwrap();
    let first = r.screenshot_scene_png(&retained, 321, 241).unwrap();
    assert_ne!(first, preview);
    edit(
        &mut f,
        &id,
        "Camera",
        json!({"fov_degrees":30,"near":0.1,"far":100}),
    );
    let narrow = capture(&f, &r, &id, "camera-fov-30");
    edit(
        &mut f,
        &id,
        "Camera",
        json!({"fov_degrees":50,"near":0.1,"far":100}),
    );
    let visible_edges = capture(&f, &r, &id, "camera-fov-50-visible-edges");
    assert!(red_pixels(&visible_edges) < 321 * 241);
    assert!(red_pixels(&visible_edges) > 190 * 190);
    edit(
        &mut f,
        &id,
        "Camera",
        json!({"fov_degrees":80,"near":0.1,"far":100}),
    );
    let wide = capture(&f, &r, &id, "camera-fov-80");
    assert!(red_pixels(&narrow) > red_pixels(&wide) * 2);
    for (name, near, far) in [("near-clipped", 9., 100.), ("far-clipped", 0.1, 7.)] {
        edit(
            &mut f,
            &id,
            "Camera",
            json!({"fov_degrees":60,"near":near,"far":far}),
        );
        assert_eq!(red_pixels(&capture(&f, &r, &id, name)), 0);
    }
    edit(
        &mut f,
        &id,
        "Camera",
        json!({"fov_degrees":60,"near":0.1,"far":100}),
    );
    edit(
        &mut f,
        &id,
        "Camera",
        json!({"fov_degrees":60,"near":7.9,"far":8.1}),
    );
    assert_eq!(capture(&f, &r, &id, "camera-tight-clip-positive"), first);
    f.edit(|g| {
        g["nodes"][0]["rotation"] =
            json!([15f64.to_radians().sin(), 0, 0, 15f64.to_radians().cos()])
    });
    edit(
        &mut f,
        &id,
        "Camera",
        json!({"fov_degrees":60,"near":7.5,"far":8.5}),
    );
    let partial = capture(&f, &r, &id, "camera-tilted-partial-clip");
    let (top, bottom) = red_rows(&partial, 321);
    let half_height = 241f64 / 2.;
    let y = 30f64.to_radians().cos();
    let tangent = 30f64.to_radians().tan();
    let expected_top = half_height - half_height * y / (7.5 * tangent);
    let expected_bottom = half_height + half_height * y / (8.5 * tangent);
    assert!((top as f64 - expected_top).abs() <= 1.);
    assert!((bottom as f64 - expected_bottom).abs() <= 1.);
    edit(
        &mut f,
        &id,
        "Transform",
        json!(Transform {
            translation: [0., 0., 8.],
            rotation: [0., 1., 0., 0.],
            ..Default::default()
        }),
    );
    assert_eq!(red_pixels(&capture(&f, &r, &id, "camera-facing-away")), 0);
    assert_eq!(r.screenshot_scene_png(&retained, 321, 241).unwrap(), first);
}

#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn inherited_cameras_and_custom_light_clusters_match_the_all_light_reference() {
    let r = Renderer::headless().unwrap();
    let mut f =
        Fixture::new(json!({"pbrMetallicRoughness":{"metallicFactor":0,"roughnessFactor":0.6}}));
    f.environment(
        &[0, 0, 0, 255].repeat(2),
        2,
        1,
        incant_doc::TextureUsage::Linear,
    );
    let id = camera(
        &mut f,
        Transform {
            translation: [0., 0., 8.],
            ..Default::default()
        },
    );
    for y in -3..=3 {
        for x in -3..=3 {
            let mut e = Entity::new("Local light");
            e.components.insert("PointLight".into(),json!({"color":[0.5+f64::from(x)*0.1,0.5+f64::from(y)*0.1,0.4],"intensity":0.4,"range":1.5}));
            e.components.insert(
                "Transform".into(),
                json!(Transform {
                    translation: [f64::from(x), f64::from(y), 0.6],
                    ..Default::default()
                }),
            );
            insert(&mut f, e);
        }
    }
    for (index, (eye, fov, near, far)) in [
        ([0., 0., 8.], 35., 1., 40.),
        ([3., 2., 7.], 75., 0.4, 15.),
        ([-5., 4., 10.], 90., 2., 50.),
    ]
    .into_iter()
    .enumerate()
    {
        let eye = glam::DVec3::from_array(eye);
        let rotation = glam::DQuat::from_mat4(
            &glam::DMat4::look_at_rh(eye, glam::DVec3::ZERO, glam::DVec3::Y).inverse(),
        );
        edit(
            &mut f,
            &id,
            "Transform",
            json!(Transform {
                translation: eye.to_array(),
                rotation: rotation.to_array(),
                ..Default::default()
            }),
        );
        edit(
            &mut f,
            &id,
            "Camera",
            json!({"fov_degrees":fov,"near":near,"far":far}),
        );
        let scene = f.scene(&r).with_camera(&id).unwrap();
        let all = f
            .scene(&r)
            .with_camera(&id)
            .unwrap()
            .with_local_light_selection(LocalLightSelection::All);
        for (w, h) in [(321, 241), (480, 270)] {
            let a = r.screenshot_scene_png(&scene, w, h).unwrap();
            let b = r.screenshot_scene_png(&all, w, h).unwrap();
            assert_eq!(a, b, "pose {index}, {w}x{h}");
            save(&format!("camera-clusters-{index}-{w}x{h}"), &a);
            save(&format!("camera-clusters-{index}-{w}x{h}-oracle"), &b);
        }
    }
    // The same final pose is represented through a translated parent.
    let flat = capture(&f, &r, &id, "camera-flat-parent-reference");
    let entities = &mut f.project.scenes.values_mut().next().unwrap().entities;
    let mut parent = Entity::new("Camera parent");
    parent.components.insert(
        "Transform".into(),
        json!(Transform {
            translation: [2., 0., 0.],
            ..Default::default()
        }),
    );

    let child = entities.get_mut(&id).unwrap();
    child.parent = Some(parent.id.clone());
    child.components.get_mut("Transform").unwrap()["translation"][0] = json!(-7.);
    entities.insert(parent.id.clone(), parent);
    assert_eq!(capture(&f, &r, &id, "camera-inherited-pose"), flat);
    assert_eq!(f.scene(&r).stats().diagnostic_entities, 0);
    let entities = &mut f.project.scenes.values_mut().next().unwrap().entities;
    let parent_id = entities[&id].parent.clone().unwrap();
    let roll = glam::DQuat::from_rotation_z(30f64.to_radians());
    entities.get_mut(&parent_id).unwrap().components.insert(
        "Transform".into(),
        json!(Transform {
            translation: [2., 0., 0.],
            rotation: roll.to_array(),
            scale: [2., 3., 4.]
        }),
    );
    entities.get_mut(&id).unwrap().components.insert(
        "Transform".into(),
        json!(Transform {
            translation: [-1., 0., 2.],
            ..Default::default()
        }),
    );
    edit(
        &mut f,
        &id,
        "Camera",
        json!({"fov_degrees":60,"near":7.9,"far":8.1}),
    );
    let inherited = capture(&f, &r, &id, "camera-scaled-roll-parent");
    let world = glam::DMat4::from_scale_rotation_translation(
        glam::DVec3::new(2., 3., 4.),
        roll,
        glam::DVec3::new(2., 0., 0.),
    );
    let eye = world.transform_point3(glam::DVec3::new(-1., 0., 2.));
    let entities = &mut f.project.scenes.values_mut().next().unwrap().entities;
    entities.get_mut(&id).unwrap().parent = None;
    entities.remove(&parent_id);
    edit(
        &mut f,
        &id,
        "Transform",
        json!(Transform {
            translation: eye.to_array(),
            rotation: roll.to_array(),
            ..Default::default()
        }),
    );
    assert_eq!(capture(&f, &r, &id, "camera-scaled-roll-flat"), inherited);
}

#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn transparency_depth_order_follows_the_selected_camera() {
    let r = Renderer::headless().unwrap();
    let mut f = Fixture::new(emissive([1., 0., 0.], 0.5, "BLEND"));
    f.edit(|g| {
        let mut red = emissive([1., 0., 0.], 0.5, "BLEND");
        red["doubleSided"] = json!(true);
        let mut blue = emissive([0., 0., 1.], 0.5, "BLEND");
        blue["doubleSided"] = json!(true);
        g["materials"] = json!([red, blue]);
        let mut far = g["meshes"][0].clone();
        far["primitives"][0]["material"] = json!(1);
        g["meshes"].as_array_mut().unwrap().push(far);
        g["nodes"] =
            json!([{"mesh":0,"translation":[0,0,0.5]},{"mesh":1,"translation":[0,0,-0.5]}]);
        g["scenes"][0]["nodes"] = json!([0, 1]);
    });
    let id = camera(
        &mut f,
        Transform {
            translation: [0., 0., 8.],
            ..Default::default()
        },
    );
    let front = super::materials::center(&capture(&f, &r, &id, "camera-transparent-front"));
    edit(
        &mut f,
        &id,
        "Transform",
        json!(Transform {
            translation: [0., 0., -8.],
            rotation: [0., 1., 0., 0.],
            ..Default::default()
        }),
    );
    let back = super::materials::center(&capture(&f, &r, &id, "camera-transparent-back"));
    assert!(front[0] > front[2] + 30, "{front:?}");
    assert!(back[2] > back[0] + 30, "{back:?}");
}

fn red_rows(bytes: &[u8], width: usize) -> (usize, usize) {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .unwrap();
    let mut pixels = vec![0; decoder.output_buffer_size().unwrap()];
    let info = decoder.next_frame(&mut pixels).unwrap();
    let rows: Vec<_> = pixels[..info.buffer_size()]
        .as_chunks::<4>()
        .0
        .iter()
        .enumerate()
        .filter(|(_, p)| p[0] > p[1].saturating_add(50))
        .map(|(i, _)| i / width)
        .collect();
    (*rows.iter().min().unwrap(), *rows.iter().max().unwrap())
}
