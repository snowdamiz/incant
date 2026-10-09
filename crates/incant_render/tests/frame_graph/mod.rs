//! Exercise the actual scheduled GPU path after authoring and CPU versions die.
use super::*;
use crate::{
    hdr_output::{read, save, texture},
    test_support,
};

#[test]
#[ignore = "requires a native GPU; run by all desktop workflows"]
fn queued_graph_frames_outlive_scene_versions_and_failed_preparation() {
    let renderer = crate::Renderer::headless().unwrap();
    let directory = tempfile::tempdir().unwrap();
    let (mut project, mut assets, asset_id) = test_support::fixture(directory.path());
    let mut jobs = Vec::new();
    let mut references = Vec::new();
    for (index, (offset, width, height)) in [
        (0., 321, 193),
        (2., 480, 270),
        (-2., 321, 193),
        (-2., 321, 193),
    ]
    .into_iter()
    .enumerate()
    {
        if index > 0 {
            let mut replacement = test_support::model(directory.path(), offset);
            replacement.id = asset_id.clone();
            project.assets.insert(asset_id.clone(), replacement);
            assets.sync_project(&project, directory.path()).unwrap();
        }
        // Last two frames reuse the exact grid shape/word count but alternate
        // which local-light bit can reach geometry. Stale masks cannot pass.
        for light_index in 0..2 {
            let mut light = incant_doc::Entity::new("Retained local light");
            light.id = format!("{:026}", light_index + 1);
            light.components.insert(
                "PointLight".into(),
                serde_json::json!({
                    "color": if light_index == 0 { [1.,0.05,0.05] } else { [0.05,1.,0.05] },
                    "intensity":80,"range":20
                }),
            );
            light.components.insert(
                "Transform".into(),
                serde_json::json!(incant_doc::Transform {
                    translation: if light_index == index % 2 {
                        [1., 3., 6.]
                    } else {
                        [1000.; 3]
                    },
                    ..Default::default()
                }),
            );
            project
                .scenes
                .values_mut()
                .next()
                .unwrap()
                .entities
                .insert(light.id.clone(), light);
        }
        let scene = renderer.prepare_scene(&project, &assets).unwrap();
        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let reference = texture(&renderer, format, width, height);
        let reference_view = reference.create_view(&Default::default());
        renderer.queue.submit([renderer
            .draw_scene(&scene, &reference_view, format, width, height, None)
            .unwrap()]);
        let expected = read(&renderer, &reference);
        // Positive coverage: neither a skipped model node nor a blank output can pass.
        assert!(
            expected
                .as_chunks::<4>()
                .0
                .iter()
                .filter(|p| p[..3] != [20, 21, 25])
                .count()
                > 100
        );
        save(
            &format!("graph-retained-model-{index}"),
            &expected,
            width,
            height,
        );
        references.push(expected.clone());
        let target = texture(&renderer, format, width, height);
        let command = renderer
            .draw_scene(
                &scene,
                &target.create_view(&Default::default()),
                format,
                width,
                height,
                None,
            )
            .unwrap();
        jobs.push((target, command, expected));
        let bad_viewport = crate::Viewport {
            rect: [0., 0., width as f32 + 1., height as f32],
            corner_radii: [0.; 4],
            canvas_srgb: [0; 3],
        };
        assert!(
            renderer
                .draw_scene(
                    &scene,
                    &reference_view,
                    format,
                    width,
                    height,
                    Some(bad_viewport)
                )
                .is_err()
        );
        let state = renderer.graph.0.lock().unwrap();
        assert!(!state.world.contains_resource::<PreparedFrame>());
        assert!(!state.world.contains_resource::<Encoder>());
    }
    assert_ne!(
        references[0], references[2],
        "different retained geometry must produce different pixels"
    );
    assert_ne!(
        references[2], references[3],
        "same geometry/grid must show the different active local light"
    );
    drop(assets);
    drop(project);
    directory.close().unwrap();
    // The scheduler must not keep CPU scene/decoded asset versions alive. Encoded
    // wgpu handles are sufficient, including after asset files disappear.
    assert!(
        renderer
            .models
            .lock()
            .unwrap()
            .values()
            .all(|model| model.upgrade().is_none())
    );
    // Submit out of encoding order: no frame may depend on the next frame's state.
    for (target, command, expected) in jobs.into_iter().rev() {
        renderer.queue.submit([command]);
        assert_eq!(read(&renderer, &target), expected);
    }
    // A model-free frame still clears stale HDR/depth and executes display output.
    let empty = renderer
        .screenshot_png(&incant_doc::Project::empty("Empty after models"), 321, 193)
        .unwrap();
    let decoder = png::Decoder::new(std::io::Cursor::new(empty));
    let mut reader = decoder.read_info().unwrap();
    let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
    reader.next_frame(&mut pixels).unwrap();
    assert!(
        pixels
            .as_chunks::<4>()
            .0
            .iter()
            .all(|p| *p == [20, 21, 25, 255])
    );
}
