//! Small actual-shader refusals independent of the moving scene's caster flag.
use super::*;
const N: u32 = 16;
const PRESENT: u32 = 1 << 31;
const PLANE: u64 = 256 * N as u64;
fn upload<T: bytemuck::Pod>(q: &wgpu::Queue, view: &wgpu::TextureView, values: &[T]) {
    assert_eq!(values.len(), (N * N) as usize);
    q.write_texture(
        view.texture().as_image_copy(),
        bytemuck::cast_slice(values),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(N * size_of::<T>() as u32),
            rows_per_image: Some(N),
        },
        wgpu::Extent3d {
            width: N,
            height: N,
            depth_or_array_layers: 1,
        },
    );
}
fn texture(d: &wgpu::Device, format: wgpu::TextureFormat) -> wgpu::TextureView {
    d.create_texture(&wgpu::TextureDescriptor {
        label: Some("Camera gather small refusal input"),
        size: wgpu::Extent3d {
            width: N,
            height: N,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING
            | if format == wgpu::TextureFormat::Depth32Float {
                wgpu::TextureUsages::RENDER_ATTACHMENT
            } else {
                wgpu::TextureUsages::COPY_DST
            },
        view_formats: &[],
    })
    .create_view(&Default::default())
}
#[test]
#[ignore = "bounded actual Metal ownership and occlusion controls"]
fn actual_camera_owner_refusals() {
    let out = PathBuf::from(std::env::var("INCANT_OWNER_CAMERA_CONTROLS_OUT").unwrap());
    std::fs::create_dir(&out).expect("fresh control output");
    let renderer = owner_renderer();
    assert_eq!(renderer.adapter.get_info().backend, wgpu::Backend::Metal);
    let d = &renderer.device;
    let q = &renderer.queue;
    let experiment = Experiment::new(&renderer);
    let cases = [
        "unchanged",
        "changed_owner",
        "caster_change",
        "age_eighth_absence",
        "zero_mass",
        "cold",
        "opaque_history",
        "mixed_record",
        "invalid_owner",
        "invalid_depth",
        "invalid_matrix",
        "out_of_window",
        "foreground_same_owner",
        "foreground_other_owner",
    ];
    let read = d.create_buffer(&wgpu::BufferDescriptor {
        label: Some("All camera refusal colors"),
        size: PLANE * cases.len() as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = d.create_command_encoder(&Default::default());
    let near = NEAR as f32;
    let far = FAR as f32;
    let projection = Mat4::perspective_rh(50_f32.to_radians(), 1., near, far);
    for (index, case) in cases.iter().copied().enumerate() {
        let previous = Slot::sized(d, [N, N]);
        let destination = Slot::sized(d, [N, N]);
        let alpha = if case == "opaque_history" {
            0x3c00
        } else {
            0x3800
        };
        upload(
            q,
            &previous.views[0],
            &vec![[0x3400u16, 0, 0, alpha]; (N * N) as usize],
        );
        upload(
            q,
            &previous.views[1],
            &vec![
                if case == "invalid_depth" {
                    f32::NAN
                } else {
                    10.
                };
                (N * N) as usize
            ],
        );
        upload(
            q,
            &previous.views[2],
            &vec![[PRESENT | 1, if case == "mixed_record" { 2 } else { 0 }]; (N * N) as usize],
        );
        upload(
            q,
            &previous.views[3],
            &vec![
                [
                    if case == "zero_mass" { 0u16 } else { 0x4400 },
                    if case == "age_eighth_absence" {
                        0x4700
                    } else {
                        0
                    }
                ];
                (N * N) as usize
            ],
        );
        let color = texture(d, wgpu::TextureFormat::Rgba16Float);
        let motion = texture(d, wgpu::TextureFormat::Rgba32Float);
        let depth = texture(d, wgpu::TextureFormat::Depth32Float);
        let owner = texture(d, wgpu::TextureFormat::R32Uint);
        let foreground = case.starts_with("foreground");
        let mut pixels = vec![[0u16; 4]; (N * N) as usize];
        if foreground {
            pixels[(8 * N + 8) as usize] = [0, 0x3400, 0, 0x3c00];
        }
        upload(q, &color, &pixels);
        upload(q, &motion, &vec![[0f32; 4]; (N * N) as usize]);
        upload(
            q,
            &owner,
            &vec![
                if case == "foreground_other_owner" {
                    2u32
                } else {
                    1
                };
                (N * N) as usize
            ],
        );
        let z = if foreground {
            (1. / near - 1.) / (1. / near - 1. / far)
        } else {
            1.
        };
        drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Actual native-depth refusal input"),
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(z),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        }));
        let mut changes = vec![0u32; 12528 / 4];
        changes[2] = if case == "invalid_owner" { 0 } else { 2 };
        if case == "changed_owner" {
            changes[0] = 2;
            changes[4] = 2;
        }
        if case == "caster_change" {
            changes[1] = 2;
        }
        let changes = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&changes),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let current_matrix = if case == "invalid_matrix" {
            Mat4::ZERO
        } else {
            projection
                * Mat4::from_translation(Vec3::new(
                    if case == "out_of_window" { 100. } else { 0.1 },
                    0.,
                    0.,
                ))
        };
        let frame = super::super::recurrence_fixture::Frame {
            source: [0., 0., N as f32, N as f32],
            output: [0., 0., N as f32, N as f32],
            depth: [near, far, 0., f32::from(u8::from(case != "cold"))],
            options: [0.9, 0.05, 0.02, 4.],
            reactive: [0.5, 0., 1., 1.],
            jitter: [0.; 4],
        };
        experiment.resolve(
            &renderer,
            &mut encoder,
            &previous,
            &destination,
            frame,
            CameraPair {
                previous_inverse: projection.inverse().to_cols_array_2d(),
                current_matrix: current_matrix.to_cols_array_2d(),
            },
            [&color, &motion, &depth, &owner],
            &changes,
        );
        encoder.copy_texture_to_buffer(
            destination.views[0].texture().as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &read,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: PLANE * index as u64,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(N),
                },
            },
            wgpu::Extent3d {
                width: N,
                height: N,
                depth_or_array_layers: 1,
            },
        );
    }
    q.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    read.slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
    d.poll(wgpu::PollType::Wait {
        submission_index: None,
        timeout: Some(Duration::from_secs(10)),
    })
    .unwrap();
    rx.recv_timeout(Duration::from_secs(10)).unwrap().unwrap();
    let bytes = read.slice(..).get_mapped_range();
    std::fs::write(out.join("colors.bin"), &bytes).unwrap();
    let mut results = Vec::new();
    for (index, case) in cases.iter().copied().enumerate() {
        let at = index * PLANE as usize + 8 * 256 + 8 * 8;
        let rgba: [f32; 4] = std::array::from_fn(|i| {
            half::f16::from_bits(u16::from_le_bytes(
                bytes[at + 2 * i..at + 2 * i + 2].try_into().unwrap(),
            ))
            .to_f32()
        });
        let passed = if case == "unchanged" {
            rgba[0] > 0. && rgba[3] > 0. && rgba[3] < 1.
        } else if case.starts_with("foreground") {
            rgba[0] == 0. && rgba[1] > 0.
        } else {
            rgba == [0.; 4]
        };
        results.push(json!({"case":case,"rgba":rgba,"passed":passed}));
    }
    let report = json!({"adapter":describe(&renderer),"shader_sha256":hex(shader().as_bytes()),"submissions":1,"maps":1,"size":[N,N],"colors_sha256":hex(&bytes),"cases":results,"scope":"Actual shader refusals, no appearance or performance acceptance"});
    std::fs::write(
        out.join("report.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    drop(bytes);
    read.unmap();
    assert!(results.iter().all(|r| r["passed"] == true), "{results:?}");
}
