//! Bounded translation support, independently checked against a tent partition.
//! One submission/map; no scene, appearance, or arbitrary-camera support claim.
use super::*;
use controls::{N, PLANE, PRESENT, texture, upload};

#[derive(Clone, Copy)]
struct Case {
    name: &'static str,
    delta: [f32; 2],
    split: bool,
}
const CASES: [Case; 19] = [
    Case {
        name: "zero",
        delta: [0., 0.],
        split: false,
    },
    Case {
        name: "right_2_5",
        delta: [2.5, 0.],
        split: false,
    },
    Case {
        name: "left_2_5",
        delta: [-2.5, 0.],
        split: false,
    },
    Case {
        name: "down_2_5",
        delta: [0., 2.5],
        split: false,
    },
    Case {
        name: "up_2_5",
        delta: [0., -2.5],
        split: false,
    },
    Case {
        name: "left_down_2_5",
        delta: [-2.5, 2.5],
        split: false,
    },
    Case {
        name: "right_3",
        delta: [3., 0.],
        split: false,
    },
    Case {
        name: "left_3",
        delta: [-3., 0.],
        split: false,
    },
    Case {
        name: "down_3",
        delta: [0., 3.],
        split: false,
    },
    Case {
        name: "up_3",
        delta: [0., -3.],
        split: false,
    },
    Case {
        name: "right_3_minus_1_16",
        delta: [47. / 16., 0.],
        split: false,
    },
    Case {
        name: "left_3_minus_1_16",
        delta: [-47. / 16., 0.],
        split: false,
    },
    Case {
        name: "right_3_plus_1_16",
        delta: [49. / 16., 0.],
        split: false,
    },
    Case {
        name: "left_3_plus_1_16",
        delta: [-49. / 16., 0.],
        split: false,
    },
    Case {
        name: "outside_right_4",
        delta: [4., 0.],
        split: false,
    },
    Case {
        name: "outside_left_4",
        delta: [-4., 0.],
        split: false,
    },
    Case {
        name: "outside_down_4",
        delta: [0., 4.],
        split: false,
    },
    Case {
        name: "outside_up_4",
        delta: [0., -4.],
        split: false,
    },
    Case {
        name: "split_plus_2_plus_3",
        delta: [-2.25, 0.],
        split: true,
    },
];

// Sum of the unit lattice tent over integer offsets [-R,R]. At the interior
// observation pixel no texture/viewport boundary clips these contributing taps.
fn partition(radius: i32, displacement: f64) -> f64 {
    (f64::from(radius) + 1. - displacement.abs()).clamp(0., 1.)
}
fn current_mass() -> f64 {
    // Equal source/output extents, zero jitter, pixel-center observation: the
    // declared 3x3 Gaussian separates into two identical [a,1,a] sums. Clear
    // samples contribute their full mass, even though their color is zero.
    let variance = 0.35_f64.powi(2);
    (1. + 2. * (-0.5 / variance).exp()).powi(2) / (2. * std::f64::consts::PI * variance)
}
fn expected(case: Case, radius: i32) -> ([f64; 4], [f64; 2], f64) {
    let mass =
        partition(radius, f64::from(case.delta[0])) * partition(radius, f64::from(case.delta[1]));
    if mass == 0. {
        // No accepted old tap: the original clear-current resolver has no owner
        // and emits zero evidence, rather than seeding the clear sample mass.
        return ([0.; 4], [0.; 2], 0.);
    }
    let total = current_mass() + mass;
    let numerator = if case.split {
        // F(q)=q+(-2.25,0): old offsets +2 and +3 have weights .75 and .25.
        // Radius 2 omits green at +3; it must not renormalize this missing mass.
        [
            0.25 * 0.75,
            if radius == 3 { 0.25 * 0.25 } else { 0. },
            0.,
            0.5 * mass,
        ]
    } else {
        [0.25 * mass, 0., 0., 0.5 * mass]
    };
    (numerator.map(|n| n / total), [total, 1.], mass)
}
fn cameras(delta: [f32; 2]) -> CameraPair {
    // With an orthographic depth interval [1,17] and old linear depth 9,
    // device depth is exactly .5. Identity inverse plus this clip translation
    // gives F(q)=q+delta; top-left output Y reverses the clip-space Y sign.
    CameraPair {
        previous_inverse: Mat4::IDENTITY.to_cols_array_2d(),
        current_matrix: Mat4::from_translation(Vec3::new(delta[0] / 8., -delta[1] / 8., 0.))
            .to_cols_array_2d(),
    }
}
fn half_close(actual: f32, reference: f64) -> bool {
    if reference == 0. {
        return actual == 0.;
    }
    // Predeclared numerical allowance: at most one adjacent binary16 value
    // from the independent f64 prediction, covering final half storage and
    // small f32 arithmetic differences. This is a regression allowance, not a
    // proof of an arbitrary backend's transcendental implementation accuracy.
    actual.is_finite()
        && actual >= 0.
        && half::f16::from_f32(actual)
            .to_bits()
            .abs_diff(half::f16::from_f64(reference).to_bits())
            <= 1
}
fn pixel<const C: usize>(bytes: &[u8], plane: usize) -> [f32; C] {
    let at = plane * PLANE as usize + 8 * 256 + 8 * C * 2;
    std::array::from_fn(|channel| {
        let start = at + channel * 2;
        half::f16::from_bits(u16::from_le_bytes([bytes[start], bytes[start + 1]])).to_f32()
    })
}

#[test]
fn support_partition_and_projection_reference() {
    assert_eq!(partition(2, 2.5), 0.5);
    assert_eq!(partition(2, -2.5).powi(2), 0.25);
    assert_eq!(partition(3, 3.), 1.);
    assert_eq!(partition(3, 47. / 16.), 1.);
    assert_eq!(partition(3, -49. / 16.), 15. / 16.);
    assert_eq!(partition(3, 4.), 0.);
    for case in CASES {
        let pair = cameras(case.delta);
        let old = glam::Vec4::new(2. * 8.5 / 16. - 1., 1. - 2. * 8.5 / 16., 0.5, 1.);
        let clip = Mat4::from_cols_array_2d(&pair.current_matrix) * old;
        assert_eq!(clip.w, 1.);
        assert_eq!(clip.z, 0.5);
        assert_eq!(
            [(clip.x * 0.5 + 0.5) * 16., (0.5 - clip.y * 0.5) * 16.],
            [8.5 + case.delta[0], 8.5 + case.delta[1]]
        );
    }
    let (_, full, _) = expected(CASES[1], 3);
    let (_, truncated, _) = expected(CASES[1], 2);
    assert!(!half_close(
        half::f16::from_f64(truncated[0]).to_f32(),
        full[0]
    ));
    let (full, _, _) = expected(CASES[18], 3);
    let (truncated, _, _) = expected(CASES[18], 2);
    assert!(full[1] > 0. && truncated[1] == 0.);
    assert!(!half_close(truncated[1] as f32, full[1]));
}

#[test]
#[ignore = "bounded actual Metal camera-support partition, one submission and map"]
fn actual_camera_owner_translation_partition() {
    let out = PathBuf::from(std::env::var("INCANT_OWNER_CAMERA_SUPPORT_OUT").unwrap());
    std::fs::create_dir(&out).expect("fresh support output");
    let renderer = owner_renderer();
    assert_eq!(renderer.adapter.get_info().backend, wgpu::Backend::Metal);
    let d = &renderer.device;
    let q = &renderer.queue;
    let shaders = [shader_with_radius(2), shader_with_radius(3)];
    for (index, source) in shaders.iter().enumerate() {
        std::fs::write(out.join(format!("radius-{}.wgsl", index + 2)), source).unwrap();
    }
    let experiments = [
        Experiment::with_radius(&renderer, 2),
        Experiment::with_radius(&renderer, 3),
    ];
    let read = d.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Both support radii, color and evidence"),
        size: PLANE * (CASES.len() * 2 * 2) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = d.create_command_encoder(&Default::default());
    let color = texture(d, wgpu::TextureFormat::Rgba16Float);
    let motion = texture(d, wgpu::TextureFormat::Rgba32Float);
    let depth = texture(d, wgpu::TextureFormat::Depth32Float);
    let owner = texture(d, wgpu::TextureFormat::R32Uint);
    upload(q, &color, &vec![[0u16; 4]; (N * N) as usize]);
    upload(q, &motion, &vec![[0f32; 4]; (N * N) as usize]);
    upload(q, &owner, &vec![0u32; (N * N) as usize]);
    drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Actual clear depth attachment for support controls"),
        color_attachments: &[],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: &depth,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        ..Default::default()
    }));
    let mut changes = vec![0u32; 12528 / 4];
    changes[2] = 1;
    let changes = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Unchanged owner 1, no static-camera shortcut"),
        contents: bytemuck::cast_slice(&changes),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    for (index, case) in CASES.iter().copied().enumerate() {
        let previous = Slot::sized(d, [N, N]);
        let mut colors = vec![[0x3400u16, 0, 0, 0x3800]; (N * N) as usize];
        if case.split {
            colors[(8 * N + 11) as usize] = [0, 0x3400, 0, 0x3800];
        }
        upload(q, &previous.views[0], &colors);
        upload(q, &previous.views[1], &vec![9f32; (N * N) as usize]);
        upload(
            q,
            &previous.views[2],
            &vec![[PRESENT | 1, 0u32]; (N * N) as usize],
        );
        upload(
            q,
            &previous.views[3],
            &vec![[0x3c00u16, 0]; (N * N) as usize],
        );
        for (variant, experiment) in experiments.iter().enumerate() {
            let destination = Slot::sized(d, [N, N]);
            let frame = super::super::recurrence_fixture::Frame {
                source: [0., 0., N as f32, N as f32],
                output: [0., 0., N as f32, N as f32],
                depth: [1., 17., 1., 1.],
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
                cameras(case.delta),
                [&color, &motion, &depth, &owner],
                &changes,
            );
            for (plane, attachment) in [0, 3].into_iter().enumerate() {
                encoder.copy_texture_to_buffer(
                    destination.views[attachment].texture().as_image_copy(),
                    wgpu::TexelCopyBufferInfo {
                        buffer: &read,
                        layout: wgpu::TexelCopyBufferLayout {
                            offset: PLANE * ((index * 2 + variant) * 2 + plane) as u64,
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
        }
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
    std::fs::write(out.join("color-evidence.bin"), &bytes).unwrap();
    let mut results = Vec::new();
    for (index, case) in CASES.iter().copied().enumerate() {
        for (variant, radius) in [2, 3].into_iter().enumerate() {
            let first = (index * 2 + variant) * 2;
            let color = pixel::<4>(&bytes, first);
            let evidence = pixel::<2>(&bytes, first + 1);
            let (expected_color, expected_evidence, mass) = expected(case, radius);
            let passed = color
                .iter()
                .zip(expected_color)
                .all(|(&v, e)| half_close(v, e))
                && half_close(evidence[0], expected_evidence[0])
                && f64::from(evidence[1]) == expected_evidence[1];
            results.push(json!({
                "case":case.name,"radius":radius,"forward_output_displacement":case.delta,
                "current_matrix":cameras(case.delta).current_matrix,
                "history_partition":mass,"expected_color":expected_color,"color":color,
                "expected_evidence":expected_evidence,"evidence":evidence,"passed":passed,
                "planes":[
                    {"format":"rgba16float","offset":first*PLANE as usize,"sha256":hex(&bytes[first*PLANE as usize..(first+1)*PLANE as usize])},
                    {"format":"rg16float","offset":(first+1)*PLANE as usize,"sha256":hex(&bytes[(first+1)*PLANE as usize..(first+2)*PLANE as usize])}
                ]
            }));
        }
    }
    let report = json!({
        "adapter":describe(&renderer),"submissions":1,"maps":1,"resolves":CASES.len()*2,
        "size":[N,N],"observation_pixel":[8,8],"row_stride_bytes":256,"plane_bytes":PLANE,
        "current_gaussian_mass":current_mass(),"history_color":[0.25,0.,0.,0.5],"history_mass":1,
        "previous_linear_depth":9,"near_far":[1,17],"projection":"orthographic, exact dyadic forward pixel displacement",
        "shader_sha256":{"radius2":hex(shaders[0].as_bytes()),"radius3":hex(shaders[1].as_bytes())},
        "raw_file":"color-evidence.bin","raw_bytes":bytes.len(),"raw_sha256":hex(&bytes),
        "comparison":"Independent f64 partition and Gaussian mass; positive outputs within one adjacent binary16 value, zeros and age exact",
        "scope":"Bounded pure-translation support only; no scene, appearance, performance, or arbitrary-camera support acceptance",
        "cases":results
    });
    std::fs::write(
        out.join("report.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    drop(bytes);
    read.unmap();
    assert!(results.iter().all(|r| r["passed"] == true), "{results:?}");
}
