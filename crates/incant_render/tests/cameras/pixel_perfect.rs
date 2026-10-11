//! Numerical pixel replication and camera-motion contracts, not appearance approval.
use super::*;

fn pixels(png: &[u8]) -> Vec<u8> {
    let mut reader = png::Decoder::new(std::io::Cursor::new(png))
        .read_info()
        .unwrap();
    let mut data = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut data).unwrap();
    data.truncate(info.buffer_size());
    data
}
fn fixture() -> (Fixture, String) {
    let mut f = Fixture::new(emissive([1., 0., 0.], 1., "OPAQUE"));
    // Uneven coverage catches offsets and transposition, rather than comparing solid frames.
    f.edit(|g| {
        g["nodes"][0]["scale"] = json!([0.37, 0.29, 1.]);
        g["nodes"][0]["translation"] = json!([0.4375, -0.3125, 0.]);
    });
    let id = camera(
        &mut f,
        Transform {
            translation: [0., 0., 8.],
            ..Default::default()
        },
    );
    edit(
        &mut f,
        &id,
        "Camera",
        json!({"fov_degrees":60,"near":0.1,"far":100,
        "projection":{"kind":"pixel_perfect","pixels_per_unit":8,"reference_resolution":[64,48]}}),
    );
    (f, id)
}

#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn fixed_logical_pixels_replicate_exactly_with_integer_scaling_and_letterboxing() {
    let r = Renderer::headless().unwrap();
    let (f, id) = fixture();
    let scene = f.scene(&r).with_camera(&id).unwrap();
    let base_png = r.screenshot_scene_png(&scene, 64, 48).unwrap();
    assert!(red_pixels(&base_png) > 100);
    let base = pixels(&base_png);
    save("pixel-perfect-logical-reference", &base_png);
    for (w, h, scale, x0, y0) in [
        (192, 144, 3, 0, 0),
        (201, 157, 3, 4, 6),
        (385, 300, 6, 0, 6),
    ] {
        let png = r.screenshot_scene_png(&scene, w, h).unwrap();
        let image = pixels(&png);
        for y in 0..h {
            for x in 0..w {
                let expected = if x >= x0 && x < x0 + 64 * scale && y >= y0 && y < y0 + 48 * scale {
                    let index = (((y - y0) / scale * 64 + (x - x0) / scale) * 4) as usize;
                    &base[index..index + 4]
                } else {
                    &[20, 21, 25, 255]
                };
                let index = ((y * w + x) * 4) as usize;
                assert_eq!(&image[index..index + 4], expected, "{w}x{h} pixel {x},{y}");
            }
        }
        save(&format!("pixel-perfect-{w}-{h}"), &png);
    }
    for (w, h) in [(63, 48), (64, 47)] {
        let error = r.screenshot_scene_png(&scene, w, h).unwrap_err();
        assert!(
            error.to_string().contains("64x48 logical resolution"),
            "{error}"
        );
    }
}

#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn subpixel_camera_motion_is_stable_and_crossing_the_grid_moves_by_one_logical_pixel() {
    let r = Renderer::headless().unwrap();
    let (mut f, id) = fixture();
    let retained = f.scene(&r).with_camera(&id).unwrap();
    let original = r.screenshot_scene_png(&retained, 192, 144).unwrap();
    for x in [-0.49 / 8., 0.49 / 8.] {
        edit(
            &mut f,
            &id,
            "Transform",
            json!(Transform {
                translation: [x, 0., 8.],
                ..Default::default()
            }),
        );
        assert_eq!(
            r.screenshot_scene_png(&f.scene(&r).with_camera(&id).unwrap(), 192, 144)
                .unwrap(),
            original
        );
        // Preparing a render never snaps authored state.
        assert_eq!(
            f.project.scenes.values().next().unwrap().entities[&id].components["Transform"]["translation"]
                [0],
            json!(x)
        );
    }
    edit(
        &mut f,
        &id,
        "Transform",
        json!(Transform {
            translation: [0.51 / 8., 0., 8.],
            ..Default::default()
        }),
    );
    let moved = r
        .screenshot_scene_png(&f.scene(&r).with_camera(&id).unwrap(), 192, 144)
        .unwrap();
    assert_ne!(moved, original);
    let a = pixels(&original);
    let b = pixels(&moved);
    for y in 0..144usize {
        for x in 0..189usize {
            assert_eq!(
                &a[(y * 192 + x + 3) * 4..(y * 192 + x + 4) * 4],
                &b[(y * 192 + x) * 4..(y * 192 + x + 1) * 4]
            );
        }
    }
    assert_eq!(
        r.screenshot_scene_png(&retained, 192, 144).unwrap(),
        original
    );
    save("pixel-perfect-one-grid-step", &moved);
}

#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn followed_grid_geometry_is_stable_at_half_pixel_ties_and_odd_resolutions() {
    let r = Renderer::headless().unwrap();
    let (mut f, id) = fixture();
    f.edit(|g| {
        g["nodes"][0]["scale"] = json!([0.25, 0.25, 1.]);
        g["nodes"][0]["translation"] = json!([0., 0., 0.]);
    });
    let model = f
        .project
        .scenes
        .values()
        .next()
        .unwrap()
        .entities
        .values()
        .find(|e| e.components.contains_key("MeshRenderer"))
        .unwrap()
        .id
        .clone();
    for angle in [0., std::f64::consts::FRAC_PI_2, 0.37] {
        let rotation = glam::DQuat::from_rotation_z(angle);
        for resolution in [[64, 48], [65, 49], [64, 49], [65, 48]] {
            edit(
                &mut f,
                &id,
                "Camera",
                json!({"fov_degrees":60,"near":0.1,"far":100,
        "projection":{"kind":"pixel_perfect","pixels_per_unit":8,"reference_resolution":resolution}}),
            );
            edit(
                &mut f,
                &id,
                "Transform",
                json!(Transform {
                    translation: [0., 0., 8.],
                    rotation: rotation.to_array(),
                    ..Default::default()
                }),
            );
            edit(
                &mut f,
                &model,
                "Transform",
                json!(Transform {
                    rotation: rotation.to_array(),
                    ..Default::default()
                }),
            );
            let baseline = r
                .screenshot_scene_png(&f.scene(&r).with_camera(&id).unwrap(), 192, 144)
                .unwrap();
            assert!(red_pixels(&baseline) > 100);
            for axis in [
                [1., 0.],
                [-1., 0.],
                [0., 1.],
                [0., -1.],
                [1., 1.],
                [-1., -1.],
                [1., -1.],
                [-1., 1.],
            ] {
                for step in [0.25, 0.49, 0.5, 0.51, 0.75, 1., 1.5, 2.5] {
                    let [x, y] = axis.map(|v| v * step / 8.);
                    let delta = rotation * glam::DVec3::new(x, y, 0.);
                    edit(
                        &mut f,
                        &id,
                        "Transform",
                        json!(Transform {
                            translation: (delta + glam::DVec3::new(0., 0., 8.)).to_array(),
                            rotation: rotation.to_array(),
                            ..Default::default()
                        }),
                    );
                    edit(
                        &mut f,
                        &model,
                        "Transform",
                        json!(Transform {
                            translation: delta.to_array(),
                            rotation: rotation.to_array(),
                            ..Default::default()
                        }),
                    );
                    let image = r
                        .screenshot_scene_png(&f.scene(&r).with_camera(&id).unwrap(), 192, 144)
                        .unwrap();
                    assert!(
                        image == baseline,
                        "followed quad changed at {axis:?} * {step} logical pixels, resolution {resolution:?}, roll {angle}"
                    );
                }
            }
        }
    }
}
