use super::{GpuLights, scene::LightPlan};
use crate::{
    Renderer, Viewport,
    hdr_output::{read, texture},
};
use incant_doc::{Entity, Transform};
use serde_json::json;
use std::{collections::BTreeMap, time::Duration};

fn grid_readback(r: &Renderer, plan: LightPlan) -> (Vec<u32>, Vec<u32>) {
    let lights = GpuLights::upload(&r.device, plan);
    let mut encoder = r.device.create_command_encoder(&Default::default());
    let _frame = r
        .lighting
        .prepare(&r.device, &mut encoder, &lights, [0., 0., 320., 192.])
        .unwrap();
    let grid = r.lighting.buffers(&r.device, [5, 3]).unwrap();
    let size = grid.counts.size() + grid.indices.size();
    let readback = r.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Cluster assignment readback"),
        size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    encoder.copy_buffer_to_buffer(&grid.counts, 0, &readback, 0, grid.counts.size());
    encoder.copy_buffer_to_buffer(
        &grid.indices,
        0,
        &readback,
        grid.counts.size(),
        grid.indices.size(),
    );
    r.queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
    r.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(10)),
        })
        .unwrap();
    rx.recv_timeout(Duration::from_secs(10)).unwrap().unwrap();
    let view = readback.slice(..).get_mapped_range();
    let mut words: Vec<_> = view
        .as_chunks::<4>()
        .0
        .iter()
        .map(|v| u32::from_le_bytes(*v))
        .collect();
    let indices = words.split_off(grid.counts.size() as usize / 4);
    drop(view);
    readback.unmap();
    (words, indices)
}

#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn every_depth_slice_selects_its_light_and_reused_clusters_clear_old_membership() {
    let r = Renderer::headless().unwrap();
    let mut plan = LightPlan::default();
    plan.add(
        &BTreeMap::from([(
            "DirectionalLight".into(),
            json!({"color":[1,1,1],"intensity":1}),
        )]),
        glam::DMat4::IDENTITY,
    )
    .unwrap();
    let inverse_view = crate::camera_view().inverse();
    for z in 0..24 {
        // One tiny light at the geometric center of each logarithmic depth
        // interval, on the camera axis. None reaches another tile or Z slice.
        let depth = crate::CAMERA_NEAR
            * (crate::CAMERA_FAR / crate::CAMERA_NEAR).powf((z as f32 + 0.5) / 24.);
        let position = inverse_view.transform_point3(glam::Vec3::new(0., 0., -depth));
        plan.add(
            &BTreeMap::from([(
                "PointLight".into(),
                json!({
                    "color":[1,1,1], "intensity":1, "range":(depth * 0.001).max(0.001)
                }),
            )]),
            glam::DMat4::from_translation(position.as_dvec3()),
        )
        .unwrap();
    }
    let (counts, indices) = grid_readback(&r, plan);
    assert_eq!(counts.len(), 5 * 3 * 24);
    assert_eq!(
        counts.iter().sum::<u32>(),
        24,
        "sparse lights must actually be culled"
    );
    for z in 0..24 {
        let cluster = (z * 3 + 1) * 5 + 2;
        assert_eq!(counts[cluster], 1, "slice {z}");
        assert_eq!(
            indices[cluster * 64],
            z as u32 + 1,
            "directional lights are outside local lists"
        );
    }
    let mut plan = LightPlan::default();
    plan.add(
        &BTreeMap::from([(
            "PointLight".into(),
            json!({"color":[1,1,1],"intensity":1,"range":1}),
        )]),
        glam::DMat4::from_translation(glam::DVec3::new(1e6, 0., 0.)),
    )
    .unwrap();
    let (counts, _) = grid_readback(&r, plan);
    assert!(
        counts.iter().all(|n| *n == 0),
        "stale cluster counts survived the next dispatch"
    );
}

fn crop(bytes: &[u8], width: usize, rect: [usize; 4]) -> Vec<u8> {
    (rect[1]..rect[1] + rect[3])
        .flat_map(|y| {
            bytes[(y * width + rect[0]) * 4..(y * width + rect[0] + rect[2]) * 4]
                .iter()
                .copied()
        })
        .collect()
}
#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn clustered_viewport_offsets_and_queued_resize_commands_keep_their_own_light_data() {
    let root = tempfile::tempdir().unwrap();
    let (mut p, assets, _) = crate::test_support::fixture(root.path());
    for x in [-3., 0., 3.] {
        let mut light = Entity::new("Localized light");
        light.components.insert(
            "PointLight".into(),
            json!({"color":[0.8,0.2,0.1],"intensity":5,"range":3}),
        );
        light.components.insert(
            "Transform".into(),
            json!(Transform {
                translation: [x, 1., 1.],
                ..Default::default()
            }),
        );
        p.scenes
            .values_mut()
            .next()
            .unwrap()
            .entities
            .insert(light.id.clone(), light);
    }
    let r = Renderer::headless().unwrap();
    let scene = r.prepare_scene(&p, &assets).unwrap();
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let a = texture(&r, format, 320, 180);
    let b = texture(&r, format, 800, 500);
    let c = texture(&r, format, 512, 320);
    let first = r
        .draw_scene(
            &scene,
            &a.create_view(&Default::default()),
            format,
            320,
            180,
            None,
        )
        .unwrap();
    let second = r
        .draw_scene(
            &scene,
            &b.create_view(&Default::default()),
            format,
            800,
            500,
            Some(Viewport {
                rect: [80., 40., 640., 360.],
                corner_radii: [0.; 4],
                canvas_srgb: [20, 21, 25],
            }),
        )
        .unwrap();
    let third = r
        .draw_scene(
            &scene,
            &c.create_view(&Default::default()),
            format,
            512,
            320,
            Some(Viewport {
                rect: [37., 41., 320., 180.],
                corner_radii: [0.; 4],
                canvas_srgb: [20, 21, 25],
            }),
        )
        .unwrap();
    // Submit in a different order from encoding; retained size-specific buffers
    // and per-command uniforms must be valid until the queue finishes each pass.
    r.queue.submit([second, first, third]);
    let base = read(&r, &a);
    let offset = crop(&read(&r, &c), 512, [37, 41, 320, 180]);
    assert_eq!(base.len(), offset.len());
    assert!(base.iter().zip(offset).all(|(a, b)| a.abs_diff(b) <= 1));
    let small = r.lighting.buffers(&r.device, [5, 3]).unwrap();
    let reused = r.lighting.buffers(&r.device, [5, 3]).unwrap();
    assert!(std::sync::Arc::ptr_eq(&small, &reused));
    assert!(r.lighting.buffers(&r.device, [u32::MAX, 2]).is_err());
}
