use super::*;
use std::time::Duration;
fn decode(png: &[u8]) -> Vec<u8> {
    let mut reader = png::Decoder::new(std::io::Cursor::new(png))
        .read_info()
        .unwrap();
    let mut bytes = vec![0; reader.output_buffer_size().unwrap()];
    reader.next_frame(&mut bytes).unwrap();
    bytes
}
fn read(r: &Renderer, target: &wgpu::Texture) -> Vec<u8> {
    let width = target.width();
    let height = target.height();
    let stride = (width * 4).div_ceil(256) * 256;
    let buffer = r.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Queued shadow readback"),
        size: u64::from(stride) * u64::from(height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = r.device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(stride),
                rows_per_image: Some(height),
            },
        },
        target.size(),
    );
    r.queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer
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
    let mapped = buffer.slice(..).get_mapped_range();
    let bytes = mapped
        .chunks(stride as usize)
        .flat_map(|row| row[..width as usize * 4].iter().copied())
        .collect();
    drop(mapped);
    buffer.unmap();
    bytes
}
#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn queued_shadow_maps_retain_camera_light_data_and_resized_atlas_versions() {
    let r = Renderer::headless().unwrap();
    let (mut f, camera, caster, light) = fixture();
    let mut extra = Entity::new("Second queued sun");
    extra.components.insert(
        "DirectionalLight".into(),
        json!({"color":[1,1,1],"intensity":0,"shadows":{"distance":40}}),
    );
    extra.components.insert(
        "Transform".into(),
        json!(Transform {
            rotation: glam::DQuat::from_rotation_y(std::f64::consts::FRAC_PI_4).to_array(),
            ..Default::default()
        }),
    );
    let extra = insert(&mut f, extra);
    let mut jobs = Vec::new();
    // Same-count frames share atlas storage; the fourth changes layer count,
    // and the fifth changes it back. All commands execute after scene disposal.
    for (i, (x, width, height, distance, suns)) in [
        (0., 320, 240, 40., 1),
        (1., 512, 320, 40., 1),
        (-1., 320, 240, 20., 1),
        (0., 320, 240, 40., 2),
        (0.5, 320, 240, 40., 1),
    ]
    .into_iter()
    .enumerate()
    {
        set(
            &mut f,
            &camera,
            "Transform",
            json!(Transform {
                translation: [x, 0., 8.],
                ..Default::default()
            }),
        );
        set(
            &mut f,
            &caster,
            "Transform",
            json!(Transform {
                translation: [x, 0., 2.],
                scale: [0.25; 3],
                ..Default::default()
            }),
        );
        set(
            &mut f,
            &light,
            "DirectionalLight",
            json!({"color":[1,1,1],"intensity":2,"shadows":{"distance":distance}}),
        );
        set(
            &mut f,
            &extra,
            "DirectionalLight",
            json!({"color":[1,1,1],"intensity":if suns==2 {1} else {0},"shadows":{"distance":40}}),
        );
        let scene = f.scene(&r).with_camera(&camera).unwrap();
        let reference = r.screenshot_scene_png(&scene, width, height).unwrap();
        let expected = decode(&reference);
        let shadow_x = ((0.5
            - 2. / (2. * 8. * 30_f32.to_radians().tan() * width as f32 / height as f32))
            * width as f32)
            .floor() as usize;
        assert!(
            expected[((height / 2) as usize * width as usize + shadow_x) * 4] < 30,
            "frame must contain a real shadow"
        );
        if let Some(dir) = std::env::var_os("INCANT_SHADOW_EVIDENCE") {
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                std::path::Path::new(&dir).join(format!("shadow-queued-{i}.png")),
                &reference,
            )
            .unwrap();
        }
        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let target = r.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Queued shadow target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let command = r
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
    }
    drop(f);
    for (target, command, expected) in jobs.into_iter().rev() {
        r.queue.submit([command]);
        assert_eq!(read(&r, &target), expected);
    }
}
