//! Numerical GPU checks for Claude/CI execution. Never part of CPU acceptance.
use super::*;
use incant_render_authoring::RendererAuthoringExt as _;

#[test]
#[ignore = "requires GPU; SSAO pending independent Claude/backend acceptance"]
fn explicit_opt_in_zero_strength_resize_queued_frames_and_failure_preserve_default_output() {
    let mut renderer = crate::test_support::checked(crate::Renderer::headless().unwrap());
    let settings = AmbientOcclusion {
        radius: 1.,
        bias: 0.01,
        strength: 0.,
    };
    if cfg!(all(
        feature = "cooked-materials",
        not(target_arch = "wasm32")
    )) {
        assert!(matches!(
            pollster::block_on(renderer.enable_ambient_occlusion(settings))
                .unwrap_err()
                .downcast_ref::<AmbientOcclusionError>(),
            Some(AmbientOcclusionError::CookedMaterials)
        ));
        return;
    }
    let directory = tempfile::tempdir().unwrap();
    let (project, assets, _) = crate::test_support::fixture(directory.path());
    let scene = renderer.prepare_scene(&project, &assets).unwrap();
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let mut jobs = Vec::new();
    for size in [[321, 193], [481, 271], [321, 193]] {
        let viewport = Some(crate::Viewport {
            rect: [13.25, 17.5, 280.5, 160.5],
            corner_radii: [0.; 4],
            canvas_srgb: [31, 63, 95],
        });
        renderer.disable_ambient_occlusion();
        let reference = crate::hdr_output::texture(&renderer, format, size[0], size[1]);
        renderer.queue.submit([renderer
            .draw_scene(
                &scene,
                &reference.create_view(&Default::default()),
                format,
                size[0],
                size[1],
                viewport,
            )
            .unwrap()]);
        let expected = crate::hdr_output::read(&renderer, &reference);
        assert!(
            expected
                .as_chunks::<4>()
                .0
                .iter()
                .any(|p| p[..3] != [31, 63, 95] && p[..3] != [20, 21, 25])
        );
        pollster::block_on(renderer.enable_ambient_occlusion(settings)).unwrap();
        let target = crate::hdr_output::texture(&renderer, format, size[0], size[1]);
        let command = renderer
            .draw_scene(
                &scene,
                &target.create_view(&Default::default()),
                format,
                size[0],
                size[1],
                viewport,
            )
            .unwrap();
        assert!(
            pollster::block_on(renderer.enable_ambient_occlusion(AmbientOcclusion {
                radius: f32::NAN,
                ..settings
            }))
            .is_err()
        );
        assert_eq!(
            renderer.ambient_occlusion.as_ref().unwrap().settings,
            settings
        );
        jobs.push((target, command, expected));
    }
    drop(scene);
    drop(project);
    drop(assets);
    directory.close().unwrap();
    renderer.disable_ambient_occlusion();
    for (target, command, expected) in jobs.into_iter().rev() {
        renderer.queue.submit([command]);
        assert_eq!(crate::hdr_output::read(&renderer, &target), expected);
    }
    // Model-free scene never consumes stale AO or forces an ambient contribution.
    pollster::block_on(renderer.enable_ambient_occlusion(AmbientOcclusion {
        strength: 1.,
        ..settings
    }))
    .unwrap();
    let empty = renderer.prepare(&crate::SceneInput::default()).unwrap();
    let target = crate::hdr_output::texture(&renderer, format, 64, 64);
    renderer.queue.submit([renderer
        .draw_scene(
            &empty,
            &target.create_view(&Default::default()),
            format,
            64,
            64,
            None,
        )
        .unwrap()]);
    assert!(
        crate::hdr_output::read(&renderer, &target)
            .as_chunks::<4>()
            .0
            .iter()
            .all(|p| *p == [20, 21, 25, 255])
    );
}
