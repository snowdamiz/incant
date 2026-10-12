//! Actual diagnostic vertex draws; numerical evidence is separate from PNG review.
use crate::{Renderer, SceneInput, hdr_output};
use glam::{DMat4, DQuat, DVec3};
use incant_render_data::SceneNode;
use std::io::Write as _;

fn node(id: &str, world: DMat4, diagnostic: bool) -> SceneNode {
    SceneNode {
        id: id.into(),
        world,
        camera: None,
        light: None,
        mesh: None,
        sprite: None,
        tilemap: None,
        diagnostic,
    }
}
fn scene(geometry: bool, raised: f64) -> SceneInput {
    let mut camera = node(
        "camera",
        DMat4::from_translation(DVec3::new(0., 0., 5.)),
        false,
    );
    camera.camera = Some(incant_types::Camera {
        fov_degrees: 60.,
        near: 0.1,
        far: 30.,
        projection: Default::default(),
    });
    let mut input = SceneInput {
        nodes: vec![camera],
        environment: None,
    };
    if geometry {
        for (id, x, reflection) in [("left", -0.9, 1.), ("right", 0.9, -1.)] {
            input.nodes.push(node(
                id,
                DMat4::from_scale_rotation_translation(
                    DVec3::new(reflection * 1.1, 0.8, 0.9),
                    DQuat::from_rotation_y(0.45),
                    DVec3::new(x, raised, 0.),
                ),
                true,
            ));
        }
    }
    input
}
fn record(case: &str, renderer: &Renderer, target: &wgpu::Texture, pixels: &[u8]) {
    use sha2::{Digest, Sha256};
    let Some(directory) = std::env::var_os("INCANT_GEOMETRY_NUMERICAL_EVIDENCE") else {
        return;
    };
    assert_eq!(
        pixels.len(),
        (target.width() * target.height() * 4) as usize
    );
    let info = renderer.adapter.get_info();
    let adapter = serde_json::json!({"name": info.name, "vendor": info.vendor,
        "device": info.device, "device_type": format!("{:?}",info.device_type),
        "backend": format!("{:?}",info.backend), "driver": info.driver,
        "driver_info": info.driver_info});
    let name = format!("diagnostic-{case}-{:?}", target.format());
    let source = concat!(
        include_str!("../../src/fixture.wgsl"),
        "\n",
        include_str!("../../src/output.wgsl"),
        "\n",
        include_str!("../../src/tone_map.wgsl")
    );
    let receipt = serde_json::json!({"case":case,"format":format!("{:?}",target.format()),
        "width":target.width(),"height":target.height(),
        "normalization":"row-major RGBA8, padding removed, BGRA channels swapped",
        "shader_path":if cfg!(feature="cooked-geometry") {"cooked-geometry"} else {"wgsl"},
        "source_sha256":format!("{:x}",Sha256::digest(source.as_bytes())),
        "adapter_sha256":format!("{:x}",Sha256::digest(serde_json::to_vec(&adapter).unwrap())),
        "adapter":adapter,"readback_sha256":format!("{:x}",Sha256::digest(pixels)),
        "readback_file":format!("{name}.rgba"),"readback_bytes":pixels.len()});
    std::fs::create_dir_all(&directory).unwrap();
    for (suffix, bytes) in [
        ("rgba", pixels.to_vec()),
        ("json", serde_json::to_vec_pretty(&receipt).unwrap()),
    ] {
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(std::path::Path::new(&directory).join(format!("{name}.{suffix}")))
            .unwrap()
            .write_all(&bytes)
            .unwrap();
    }
}
#[test]
#[ignore = "requires a native GPU; actual default/cooked comparison runs separately"]
fn diagnostic_vertices_draw_reflected_geometry_and_preserve_display_formats() {
    let renderer = Renderer::headless().unwrap();
    let (width, height) = (160, 96);
    for format in [
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureFormat::Rgba8UnormSrgb,
        wgpu::TextureFormat::Bgra8Unorm,
        wgpu::TextureFormat::Bgra8UnormSrgb,
    ] {
        let mut background: Option<Vec<u8>> = None;
        let mut previous: Option<(Vec<u8>, f64)> = None;
        for (case, geometry, raised) in [
            ("empty", false, 0.),
            ("base", true, 0.),
            ("raised", true, 0.6),
        ] {
            let input = scene(geometry, raised);
            let prepared = renderer
                .prepare(&input)
                .unwrap()
                .with_camera("camera")
                .unwrap();
            assert_eq!(
                prepared.stats().diagnostic_entities,
                if geometry { 2 } else { 0 }
            );
            assert_eq!(prepared.stats().model_entities, 0);
            let target = hdr_output::texture(&renderer, format, width, height);
            renderer.queue.submit([renderer
                .draw_scene(
                    &prepared,
                    &target.create_view(&Default::default()),
                    format,
                    width,
                    height,
                    None,
                )
                .unwrap()]);
            let pixels = hdr_output::read(&renderer, &target);
            assert!(pixels.as_chunks::<4>().0.iter().all(|p| p[3] == 255));
            if let Some(empty) = &background {
                let changed: Vec<_> = pixels
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .zip(empty.as_chunks::<4>().0.iter())
                    .enumerate()
                    .filter_map(|(i, (a, b))| (a != b).then_some(i))
                    .collect();
                assert!(
                    changed.len() > 100 && changed.len() < (width * height / 2) as usize,
                    "fixture must draw substantial bounded geometry: {} pixels",
                    changed.len()
                );
                assert!(
                    changed.iter().all(|&i| {
                        let x = i as u32 % width;
                        let y = i as u32 / width;
                        x > 8 && x < width - 8 && y > 8 && y < height - 8
                    }),
                    "geometry escaped fixture bounds"
                );
                // Both reflected placements must draw actual vertices, not only a pipeline.
                assert!(changed.iter().any(|&i| i as u32 % width < width / 2 - 8));
                assert!(changed.iter().any(|&i| i as u32 % width > width / 2 + 8));
                let average_y = changed.iter().map(|&i| i / width as usize).sum::<usize>() as f64
                    / changed.len() as f64;
                if let Some((old_pixels, old_y)) = &previous {
                    assert_ne!(
                        &pixels, old_pixels,
                        "changed accepted pose must move geometry"
                    );
                    assert!(
                        average_y < old_y - 3.,
                        "raising world Y must move the projected geometry upward"
                    );
                }
                previous = Some((pixels.clone(), average_y));
                if format == wgpu::TextureFormat::Rgba8UnormSrgb {
                    hdr_output::save(&format!("diagnostic-{case}"), &pixels, width, height);
                }
            } else {
                assert!(
                    pixels.as_chunks::<4>().0.iter().all(|p| p == &pixels[..4]),
                    "empty scene must be uniform"
                );
                background = Some(pixels.clone());
            }
            record(case, &renderer, &target, &pixels);
        }
    }
    renderer.check_device().unwrap();
}
