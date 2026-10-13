//! Numerical grid observations only: no pixel readback or appearance assertion.
use super::{GpuLights, LightPlan, LocalLightSelection, readback};
use crate::{Renderer, RetainedTarget, Viewport, camera::CameraView, hdr_output::texture};
use glam::{DMat4, DVec3};
use incant_render_data::{SceneInput, SceneLight, SceneNode};
use incant_types::{Camera, CameraProjection, DirectionalLight, PointLight, SpotLight};
use std::sync::Arc;

const VIEWPORTS: [[f32; 4]; 5] = [
    [0., 0., 320., 192.],
    [0., 0., 640., 360.],
    [0., 0., 960., 540.],
    [0., 0., 333., 197.],
    [37., 41., 640., 360.],
];
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

fn node(id: String) -> SceneNode {
    SceneNode {
        id,
        world: DMat4::IDENTITY,
        camera: None,
        light: None,
        mesh: None,
        sprite: None,
        tilemap: None,
        diagnostic: false,
    }
}

fn fixture(local: u32, ortho: bool) -> SceneInput {
    let mut camera = node("camera".into());
    camera.world = DMat4::look_at_rh(DVec3::new(0., 3., 12.), DVec3::ZERO, DVec3::Y).inverse();
    camera.camera = Some(Camera {
        fov_degrees: 55.,
        near: 0.1,
        far: 80.,
        projection: if ortho {
            CameraProjection::Orthographic { vertical_size: 16. }
        } else {
            CameraProjection::Perspective {}
        },
    });
    let mut input = SceneInput {
        nodes: vec![camera],
        ..Default::default()
    };
    // Prefix lights must never consume local membership bits.
    for index in 0..2 {
        let mut light = node(format!("directional-{index}"));
        light.light = Some(SceneLight::Directional(DirectionalLight {
            color: [1.; 3],
            intensity: 1.,
            shadows: None,
        }));
        input.nodes.push(light);
    }
    let mut seed = 0x12ab_3456u32;
    for index in 0..local {
        let mut next = || {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            f64::from(seed >> 8) / f64::from(1u32 << 24)
        };
        let position = DVec3::new(next() * 24. - 12., next() * 16. - 8., next() * 12. - 8.);
        let range = 0.25 + next() * 3.;
        let mut light = node(format!("local-{index}"));
        // A guaranteed in-frustum point makes an all-zero false read fail even
        // for the one-light case. Other lights vary spatially and by range.
        light.world = DMat4::from_translation(if index == 0 { DVec3::ZERO } else { position });
        light.light = Some(if index % 3 == 1 {
            SceneLight::Spot(SpotLight {
                color: [0.3, 0.7, 1.],
                intensity: 2.,
                range,
                inner_degrees: 15.,
                outer_degrees: 35.,
            })
        } else {
            SceneLight::Point(PointLight {
                color: [1., 0.5, 0.2],
                intensity: 2.,
                range,
            })
        });
        input.nodes.push(light);
    }
    input
}

fn plan(input: &SceneInput) -> LightPlan {
    let mut plan = LightPlan::default();
    for node in &input.nodes {
        if let Some(light) = &node.light {
            plan.add_typed(light, node.world).unwrap();
        }
    }
    plan
}

fn configurations(r: &Renderer) {
    for (camera_name, ortho) in [
        ("preview", false),
        ("perspective", false),
        ("orthographic", true),
    ] {
        for local in [1, 31, 32, 33, 4096] {
            let input = fixture(local, ortho);
            let camera = if camera_name == "preview" {
                CameraView::preview()
            } else {
                CameraView::authored(
                    input.nodes[0].camera.as_ref().unwrap(),
                    input.nodes[0].world,
                )
                .unwrap()
            };
            let lights = GpuLights::upload(&r.device, plan(&input));
            assert_eq!((lights.directional, lights.local), (2, local));
            let mut unshifted = None;
            for (index, viewport) in VIEWPORTS.into_iter().enumerate() {
                let grid = readback::grid_readback_at(r, &lights, viewport, camera);
                assert_eq!(
                    grid.dimensions,
                    [
                        (viewport[2] as u32).div_ceil(64),
                        (viewport[3] as u32).div_ceil(64)
                    ]
                );
                grid.report(&format!("{camera_name}-n{local}-v{index}"), local);
                assert!(
                    grid.counts.iter().any(|n| *n > 0),
                    "in-frustum light was not observed"
                );
                if index == 4 {
                    let previous: &readback::GridReadback = unshifted.as_ref().unwrap();
                    assert_eq!(
                        grid.counts, previous.counts,
                        "viewport origin changes no camera-relative clusters"
                    );
                    assert_eq!(grid.masks, previous.masks);
                }
                if index == 1 {
                    unshifted = Some(grid);
                }
            }
        }
    }
}

fn retained_input(ortho: bool) -> SceneInput {
    let mut input = fixture(33, ortho);
    // The canonical renderer schedules clustered lighting from model geometry.
    // Lights alone produce no model draw, so observing that frame would read the
    // previous cache without exercising retained updates at all.
    let directory = tempfile::tempdir().unwrap();
    let (project, assets, _) = crate::test_support::fixture(directory.path());
    let models = incant_render_authoring::project_input(&project, Some(&assets)).unwrap();
    assert!(models.nodes.iter().any(|node| node.mesh.is_some()));
    input.nodes.extend(models.nodes);
    input
}

#[test]
fn retained_cluster_fixture_resolves_real_model_geometry_without_source_files() {
    for ortho in [false, true] {
        // retained_input has already dropped its temporary source/cache directory.
        let input = retained_input(ortho);
        let resolved = crate::scene::resolve(&input).unwrap();
        assert_eq!(resolved.stats.model_entities, 2);
        assert!(resolved.stats.primitive_instances > 0);
        let (lights, directional, _) = plan(&input).finish();
        assert_eq!(directional, 2);
        assert_eq!(lights.len(), 35);
    }
}

fn retained(r: &Renderer, ortho: bool) {
    let mut input = retained_input(ortho);
    let mut scene = r.prepare_retained(&input, "camera").unwrap();
    assert!(scene.stats().primitive_instances > 0);
    let original: Vec<_> = input.nodes.iter().map(|node| node.world).collect();
    let mut previous_grid = None;
    let mut initial_masks = None;
    for step in 0..4 {
        for (node, world) in input.nodes.iter_mut().zip(&original).filter(|(node, _)| {
            matches!(node.light, Some(SceneLight::Point(_) | SceneLight::Spot(_)))
        }) {
            node.world = DMat4::from_translation(match step {
                1 | 3 => DVec3::new(4., 2., -1.),
                2 => DVec3::new(100_000., 0., 0.),
                _ => DVec3::ZERO,
            }) * *world;
        }
        let (width, height, viewport) = if step == 3 {
            (
                720,
                440,
                Some(Viewport {
                    rect: VIEWPORTS[4],
                    corner_radii: [0.; 4],
                    canvas_srgb: [20, 21, 25],
                }),
            )
        } else {
            (320, 192, None)
        };
        let output = texture(r, FORMAT, width, height);
        let view = output.create_view(&Default::default());
        scene
            .submit(
                r,
                &input,
                RetainedTarget {
                    view: &view,
                    format: FORMAT,
                    width,
                    height,
                    viewport,
                },
            )
            .unwrap();
        let cached = readback::last_grid(r);
        if let Some(previous) = &previous_grid {
            assert_eq!(
                Arc::ptr_eq(previous, &cached),
                step != 3,
                "same-size moves reuse; resize replaces"
            );
        }
        let grid = readback::read_last_grid(r);
        assert!(
            Arc::ptr_eq(&cached, &readback::last_grid(r)),
            "observer must not replace cache"
        );
        assert_eq!(grid.dimensions, if step == 3 { [10, 6] } else { [5, 3] });
        grid.report(
            &format!(
                "retained-{}-step{step}",
                if ortho { "orthographic" } else { "perspective" }
            ),
            33,
        );
        if step == 2 {
            assert!(
                grid.counts.iter().chain(&grid.masks).all(|n| *n == 0),
                "reused grid retained old membership"
            );
        } else {
            assert!(grid.counts.iter().any(|n| *n > 0));
        }
        if step == 1 {
            assert_ne!(
                initial_masks.as_ref().unwrap(),
                &grid.masks,
                "motion must change raw membership"
            );
        }
        if step == 0 {
            initial_masks = Some(grid.masks);
        }
        previous_grid = Some(cached);
    }
}

fn directional_only(r: &Renderer) {
    let lights = GpuLights::upload(&r.device, plan(&fixture(0, false)));
    assert_eq!((lights.directional, lights.local), (2, 0));
    let camera = CameraView::preview();
    let shadows = r.shadows.prepare(&r.device, camera, 1., &[], &[]).unwrap();
    let frame = r
        .lighting
        .prepare(
            &r.device,
            &lights,
            VIEWPORTS[0],
            LocalLightSelection::Clustered,
            camera,
            &shadows,
        )
        .unwrap();
    assert!(frame.compute.is_none());
    let grid = readback::read_last_grid(r);
    assert_eq!(grid.dimensions, [1, 1]);
    grid.report("directional-only", 0);
}

#[test]
#[ignore = "requires a native GPU; coordinated baseline/cooked run with --nocapture"]
fn extended_cluster_readback_cameras_viewports_and_retained_motion() {
    let renderer = Renderer::headless().unwrap();
    println!("cluster-readback-adapter {:?}", renderer.adapter.get_info());
    configurations(&renderer);
    retained(&renderer, false);
    retained(&renderer, true);
    directional_only(&renderer);
}
