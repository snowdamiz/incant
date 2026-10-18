//! Static carry falsification. Actual GPU output is retained before assertions.
use super::{
    raster_geometry::{self as geometry, EDGE, STRIP},
    raster_readback::{OUTPUT, Pixels, Reader},
    raster_tests::{owner, render, write},
};
use crate::{Renderer, RetainedTarget};
use glam::{DMat4, DVec3};
use serde_json::json;
use std::path::Path;

fn save(root: &Path, name: &str, p: &Pixels) {
    let raw: Vec<_> = p.0.iter().flatten().flat_map(|v| v.to_le_bytes()).collect();
    write(root, &format!("{name}.f32le"), &raw);
}
fn strip_is_clear(p: &Pixels) {
    for x in 6..55 {
        assert_eq!(p.history()[44 * OUTPUT + x], [0.; 4]);
    }
}

#[test]
#[ignore = "coordinated GPU only: static continuity/vacated geometry; no full images or timing"]
fn static_carry_retires_on_scene_change_and_rejected_preparation_never_publishes() {
    let root = std::path::PathBuf::from(std::env::var_os("INCANT_TAAU_STATIC_OUTPUT").unwrap());
    std::fs::create_dir(&root).unwrap();
    let mut r = crate::test_support::checked(Renderer::headless().unwrap());
    pollster::block_on(r.enable_temporal_upscale()).unwrap();
    let output = crate::hdr_output::texture(&r, wgpu::TextureFormat::Rgba8UnormSrgb, 64, 64);
    let view = output.create_view(&Default::default());
    let reader = Reader::new(&r);
    let mut records = Vec::new();
    for case in [
        "strip", "camera", "light", "occluder", "removal", "geometry", "material", "cut",
        "rejected",
    ] {
        let mut input = geometry::fixture();
        if case == "light" || case == "rejected" {
            let mut light = geometry::fixture().nodes.remove(0);
            light.id = "light".into();
            light.mesh = None;
            light.world = DMat4::from_translation(DVec3::new(0., 0., 2.));
            light.light = Some(incant_render_data::SceneLight::Point(
                incant_types::PointLight {
                    color: [1.; 3],
                    intensity: 1.,
                    range: 10.,
                },
            ));
            input.nodes.push(light);
        }
        if case == "occluder" {
            let rectangle = [[4., 42.], [57., 42.], [57., 48.], [4., 48.]];
            let mut node = geometry::fixture_with(&[rectangle], [0.; 3], 1)
                .nodes
                .remove(0);
            node.id = "occluder".into();
            node.world = DMat4::from_translation(DVec3::new(0., 20., 1.));
            input.nodes.push(node);
        }
        let mut retained = owner(&r, &input);
        let first = render(&r, &mut retained, &input, &view, &reader);
        save(&root, &format!("{case}-before"), &first);
        assert!(first.history()[44 * OUTPUT + 24][3] > 0.);
        let accepted = retained.motion_frame().unwrap().sequence();
        match case {
            "strip" => input.nodes[0].world = DMat4::from_translation(DVec3::Y),
            "camera" => input.nodes[1].world = DMat4::from_translation(DVec3::new(0., -1., 10.)),
            "light" => input.nodes[2].world = DMat4::from_translation(DVec3::new(1., 0., 2.)),
            "occluder" => input.nodes[2].world = DMat4::from_translation(DVec3::Z),
            "removal" => input.nodes[0].mesh = None,
            "geometry" => {
                input.nodes[0].mesh = geometry::fixture_with(&[EDGE], [1.; 3], 2)
                    .nodes
                    .remove(0)
                    .mesh
            }
            "material" => {
                input.nodes[0].mesh = geometry::fixture_with(&[EDGE, STRIP], [0.1, 0.3, 0.7], 2)
                    .nodes
                    .remove(0)
                    .mesh
            }
            "cut" => retained.invalidate_motion_history(),
            "rejected" => {
                input.nodes[0].world = DMat4::from_translation(DVec3::Y);
                input.nodes[2].world = DMat4::from_translation(DVec3::new(1., 0., 2.));
                // Output admission is later than motion/static preparation.
                let error = retained
                    .submit(
                        &r,
                        &input,
                        RetainedTarget {
                            view: &view,
                            format: wgpu::TextureFormat::R32Float,
                            width: 64,
                            height: 64,
                            viewport: None,
                        },
                    )
                    .unwrap_err();
                write(&root, "rejected-error.txt", error.to_string().as_bytes());
                assert!(matches!(
                    error.downcast_ref::<crate::ResourceError>(),
                    Some(crate::ResourceError::OutputFormat(
                        wgpu::TextureFormat::R32Float
                    ))
                ));
                assert_eq!(retained.motion_frame().unwrap().sequence(), accepted);
                let [motion, depth, history] = retained.temporal_upscale_test_views();
                let current = r
                    .frames
                    .get(&r.device, 48, 48, crate::SceneMsaa::One)
                    .unwrap();
                let rejected = reader.read(&r, &current.color, &motion, &depth, &history);
                save(&root, "rejected-unpublished", &rejected);
                assert_eq!(rejected.0, first.0);
                input.nodes[0].world = DMat4::IDENTITY;
                input.nodes[2].world = DMat4::from_translation(DVec3::new(0., 0., 2.));
            }
            _ => unreachable!(),
        }
        if matches!(case, "removal" | "geometry" | "material") {
            assert!(
                retained
                    .submit(
                        &r,
                        &input,
                        RetainedTarget {
                            view: &view,
                            format: wgpu::TextureFormat::Rgba8UnormSrgb,
                            width: 64,
                            height: 64,
                            viewport: None
                        }
                    )
                    .is_err()
            );
            assert_eq!(retained.motion_frame().unwrap().sequence(), accepted);
            retained = owner(&r, &input);
        }
        let actual = render(&r, &mut retained, &input, &view, &reader);
        save(&root, &format!("{case}-after"), &actual);
        let phase = retained.motion_frame().unwrap().jitter_phase().unwrap();
        assert_eq!(
            phase,
            if matches!(case, "removal" | "geometry" | "material" | "cut") {
                0
            } else {
                1
            }
        );
        match case {
            "strip" | "camera" | "light" | "removal" | "geometry" => strip_is_clear(&actual),
            "occluder" | "cut" | "material" => {
                let mut fresh = owner(&r, &input);
                let reference = render(&r, &mut fresh, &input, &view, &reader);
                save(&root, &format!("{case}-fresh"), &reference);
                if case == "occluder" {
                    for x in 10..51 {
                        assert_eq!(
                            actual.history()[44 * OUTPUT + x],
                            reference.history()[44 * OUTPUT + x]
                        );
                        // Opaque occluder lies at z1, closer than the old z0 strip.
                        assert!(actual.depth()[33 * 48 + x * 3 / 4][0] < (10. - 0.1) / (50. - 0.1));
                    }
                } else {
                    assert_eq!(actual.history(), reference.history());
                }
            }
            "rejected" => {
                let mut fresh = owner(&r, &input);
                render(&r, &mut fresh, &input, &view, &reader);
                let reference = render(&r, &mut fresh, &input, &view, &reader);
                save(&root, "rejected-reference", &reference);
                assert_eq!(actual.0, reference.0);
                for x in 6..55 {
                    assert!(actual.history()[44 * OUTPUT + x][3] > 0.);
                }
            }
            _ => unreachable!(),
        }
        records.push(json!({"case":case,"phase":phase,"accepted_before":accepted,
            "accepted_after":retained.motion_frame().unwrap().sequence(),"passed":true}));
    }
    write(
        &root,
        "result.json",
        &serde_json::to_vec_pretty(&json!({"cases":records,"appearance_accepted":false})).unwrap(),
    );
}
