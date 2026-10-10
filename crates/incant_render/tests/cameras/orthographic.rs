//! Projection, lighting and clipping contracts; pixel appearance is Claude-reviewed.
use super::*;

fn projection(size: f64, near: f64, far: f64) -> serde_json::Value {
    json!({"fov_degrees":60,"near":near,"far":far,
        "projection":{"kind":"orthographic","vertical_size":size}})
}

#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn orthographic_size_clipping_and_retained_projection_match_authored_world_units() {
    let r = Renderer::headless().unwrap();
    let mut f = Fixture::new(emissive([1., 0., 0.], 1., "OPAQUE"));
    let id = camera(
        &mut f,
        Transform {
            translation: [0., 0., 8.],
            ..Default::default()
        },
    );
    edit(&mut f, &id, "Camera", projection(8., 0.1, 100.));
    let retained = f.scene(&r).with_camera(&id).unwrap();
    let first = capture(&f, &r, &id, "orthographic-size-8-depth-8");
    // The six-metre square occupies 6/8 of the 241-pixel vertical extent.
    assert!((red_pixels(&first) as i64 - 181 * 181).abs() <= 362);
    edit(
        &mut f,
        &id,
        "Transform",
        json!(Transform {
            translation: [0., 0., 30.],
            ..Default::default()
        }),
    );
    assert_eq!(capture(&f, &r, &id, "orthographic-size-8-depth-30"), first);
    let mut parameters = projection(8., 0.1, 100.);
    parameters["fov_degrees"] = json!(150.);
    edit(&mut f, &id, "Camera", parameters);
    assert_eq!(capture(&f, &r, &id, "orthographic-retained-fov"), first);
    edit(&mut f, &id, "Camera", projection(16., 0.1, 100.));
    let wide = capture(&f, &r, &id, "orthographic-size-16");
    assert!((red_pixels(&wide) as i64 - 91 * 91).abs() <= 182);
    for (name, near, far) in [("near", 31., 100.), ("far", 0.1, 29.)] {
        edit(&mut f, &id, "Camera", projection(8., near, far));
        assert_eq!(
            red_pixels(&capture(
                &f,
                &r,
                &id,
                &format!("orthographic-{name}-clipped")
            )),
            0
        );
    }
    edit(&mut f, &id, "Camera", projection(8., 29.9, 30.1));
    assert_eq!(capture(&f, &r, &id, "orthographic-tight-clips"), first);
    assert_eq!(r.screenshot_scene_png(&retained, 321, 241).unwrap(), first);
}

#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn orthographic_parallel_view_rays_preserve_specular_lighting_when_camera_moves_back() {
    let r = Renderer::headless().unwrap();
    let mut f =
        Fixture::new(json!({"pbrMetallicRoughness":{"metallicFactor":1,"roughnessFactor":0.25}}));
    f.environment(
        &[0, 0, 0, 255].repeat(2),
        2,
        1,
        incant_doc::TextureUsage::Linear,
    );
    let mut sun = Entity::new("Parallel specular illumination");
    sun.components.insert(
        "DirectionalLight".into(),
        json!({"color":[1,0.5,0.25],"intensity":1}),
    );
    sun.components
        .insert("Transform".into(), json!(Transform::default()));
    insert(&mut f, sun);
    let id = camera(
        &mut f,
        Transform {
            translation: [0., 0., 8.],
            ..Default::default()
        },
    );
    edit(&mut f, &id, "Camera", projection(8., 0.1, 100.));
    let first = capture(&f, &r, &id, "orthographic-specular-near");
    assert!(super::super::materials::center(&first)[0] > 100);
    edit(
        &mut f,
        &id,
        "Transform",
        json!(Transform {
            translation: [0., 0., 30.],
            ..Default::default()
        }),
    );
    assert_eq!(capture(&f, &r, &id, "orthographic-specular-far"), first);
}

#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn orthographic_light_clusters_match_all_lights_at_multiple_sizes_poses_and_aspects() {
    let r = Renderer::headless().unwrap();
    let mut f =
        Fixture::new(json!({"pbrMetallicRoughness":{"metallicFactor":0,"roughnessFactor":0.6}}));
    f.environment(
        &[0, 0, 0, 255].repeat(2),
        2,
        1,
        incant_doc::TextureUsage::Linear,
    );
    let id = camera(&mut f, Transform::default());
    for y in -3..=3 {
        for x in -3..=3 {
            let mut light = Entity::new("Orthographic cluster light");
            light.id = format!("{:026}", 1000 + (y + 3) * 7 + (x + 3));
            light.components.insert("PointLight".into(),json!({"color":[0.5+f64::from(x)*0.1,0.5+f64::from(y)*0.1,0.4],"intensity":0.4,"range":1.5}));
            light.components.insert(
                "Transform".into(),
                json!(Transform {
                    translation: [f64::from(x), f64::from(y), 0.6],
                    ..Default::default()
                }),
            );
            insert(&mut f, light);
        }
    }
    for (index, (eye, size, near, far)) in [
        ([0., 0., 8.], 8., 0.1, 100.),
        ([0., 0., 30.], 8., 1., 60.),
        ([3., 2., 7.], 6., 0.4, 15.),
        ([-5., 4., 10.], 12., 2., 50.),
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
        edit(&mut f, &id, "Camera", projection(size, near, far));
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
            save(&format!("orthographic-clusters-{index}-{w}x{h}"), &a);
        }
    }
}
