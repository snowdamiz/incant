//! Test-only recurrence experiment fed by the existing real retained scene.
//! No production targets, pipeline, cooker ABI or admission cap changes.
use super::*;
use wgpu::util::DeviceExt;

pub(super) const SHADER: &str = concat!(
    include_str!("upscaling/common.wgsl"),
    "\n",
    include_str!("upscaling/recurrence_experiment.wgsl"),
    "\n",
    include_str!("upscaling/resolve_recurrence_experiment.wgsl"),
);
const OUTPUT: [u32; 2] = [960, 540];
const SOURCE: [u32; 2] = [480, 270];
const FORMATS: [wgpu::TextureFormat; 4] = [
    wgpu::TextureFormat::Rgba16Float,
    wgpu::TextureFormat::R32Float,
    wgpu::TextureFormat::Rg16Float,
    wgpu::TextureFormat::Rgba16Float,
];
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct Frame {
    pub source: [f32; 4],
    pub output: [f32; 4],
    pub depth: [f32; 4],
    pub options: [f32; 4],
    pub reactive: [f32; 4],
    pub jitter: [f32; 4],
}
const _: () = assert!(size_of::<Frame>() == 96);
pub(super) struct Slot {
    pub views: [wgpu::TextureView; 4],
}
impl Slot {
    pub(super) fn new(d: &wgpu::Device, size: [u32; 2]) -> Self {
        assert!(size[0] > 0 && size[1] > 0 && size[0] <= 960 && size[1] <= 540);
        Self {
            views: FORMATS.map(|format| {
                d.create_texture(&wgpu::TextureDescriptor {
                    label: Some("Experimental recurrence history only"),
                    size: wgpu::Extent3d {
                        width: size[0],
                        height: size[1],
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING
                        | wgpu::TextureUsages::COPY_SRC,
                    view_formats: &[],
                })
                .create_view(&Default::default())
            }),
        }
    }
    pub(super) fn clear(&self, encoder: &mut wgpu::CommandEncoder) {
        let attachments = std::array::from_fn::<_, 4, _>(|i| attachment(&self.views[i], i == 3));
        drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Initialize experimental history; recurrence invalid sentinel"),
            color_attachments: &attachments,
            ..Default::default()
        }));
    }
}
fn attachment(
    view: &wgpu::TextureView,
    invalid: bool,
) -> Option<wgpu::RenderPassColorAttachment<'_>> {
    let v = if invalid { -1. } else { 0. };
    Some(wgpu::RenderPassColorAttachment {
        view,
        depth_slice: None,
        resolve_target: None,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color {
                r: v,
                g: v,
                b: v,
                a: v,
            }),
            store: wgpu::StoreOp::Store,
        },
    })
}
pub(super) struct Experiment {
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
}
impl Experiment {
    pub(super) fn new(
        renderer: &Renderer,
        unclipped_ablation: bool,
        cubic_ablation: bool,
        lock: bool,
        nearest: bool,
        coarse: bool,
    ) -> Self {
        let d = &renderer.device;
        assert!(d.limits().max_color_attachments >= 4);
        assert!(d.limits().max_color_attachment_bytes_per_sample >= 24);
        let usage = wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC;
        for format in FORMATS {
            assert!(
                renderer
                    .adapter
                    .get_texture_format_features(format)
                    .allowed_usages
                    .contains(usage)
            );
        }
        let entries: [wgpu::BindGroupLayoutEntry; 8] =
            std::array::from_fn(|i| wgpu::BindGroupLayoutEntry {
                binding: i as u32,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: if i == 0 {
                    wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(96),
                    }
                } else {
                    wgpu::BindingType::Texture {
                        sample_type: if i == 3 {
                            wgpu::TextureSampleType::Depth
                        } else {
                            wgpu::TextureSampleType::Float { filterable: false }
                        },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    }
                },
                count: None,
            });
        let layout = d.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Test recurrence bindings"),
            entries: &entries,
        });
        let shader = d.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Test-only recurrence WGSL"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline_layout = d.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let targets = FORMATS.map(|format| {
            Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })
        });
        let pipeline = d.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Standalone recurrence diagnostic"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("resolve"),
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants: &[
                        (
                            "diagnostic_disable_color_rectification",
                            f64::from(u8::from(unclipped_ablation)),
                        ),
                        (
                            "diagnostic_cubic_history",
                            f64::from(u8::from(cubic_ablation)),
                        ),
                        ("diagnostic_bounded_lock", f64::from(u8::from(lock))),
                        ("diagnostic_nearest_surface", f64::from(u8::from(nearest))),
                        ("diagnostic_coarse_shading", f64::from(u8::from(coarse))),
                    ],
                    ..Default::default()
                },
                targets: &targets,
            }),
            multiview_mask: None,
            cache: None,
        });
        Self { layout, pipeline }
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn resolve(
        &self,
        renderer: &Renderer,
        encoder: &mut wgpu::CommandEncoder,
        destination: &Slot,
        previous: &Slot,
        frame: Frame,
        current: [&wgpu::TextureView; 3],
    ) {
        let uniform = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Experimental frame96"),
                contents: bytemuck::bytes_of(&frame),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let textures = [
            current[0],
            current[1],
            current[2],
            &previous.views[0],
            &previous.views[1],
            &previous.views[2],
            &previous.views[3],
        ];
        let entries: [wgpu::BindGroupEntry<'_>; 8] =
            std::array::from_fn(|i| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: if i == 0 {
                    uniform.as_entire_binding()
                } else {
                    wgpu::BindingResource::TextureView(textures[i - 1])
                },
            });
        let bindings = renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &self.layout,
                entries: &entries,
            });
        let attachments =
            std::array::from_fn::<_, 4, _>(|i| attachment(&destination.views[i], i == 3));
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("One experimental recurrence resolve"),
            color_attachments: &attachments,
            ..Default::default()
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bindings, &[]);
        pass.draw(0..3, 0..1);
    }
}

pub(super) fn half(bits: u16) -> f32 {
    let sign = if bits & 0x8000 == 0 { 1. } else { -1. };
    let exponent = i32::from((bits >> 10) & 31);
    let fraction = f32::from(bits & 1023);
    match exponent {
        0 => sign * fraction * 2_f32.powi(-24),
        31 => f32::NAN,
        _ => sign * (1. + fraction / 1024.) * 2_f32.powi(exponent - 15),
    }
}
fn pixel_half(bytes: &[u8]) -> [f32; 4] {
    std::array::from_fn(|i| {
        half(u16::from_le_bytes(
            bytes[2 * i..2 * i + 2].try_into().unwrap(),
        ))
    })
}
fn display(hdr: [f32; 4]) -> [u8; 4] {
    // f64 CPU transcription of tone_map.wgsl + output.wgsl. It is diagnostic,
    // not bit-exact production display output; physical overlays are excluded.
    // The external reference can contain small signed RGB after inverse YCoCg.
    // The unchanged display conversion below clamps RGB before tone mapping;
    // raw signed values and their counts remain in the diagnostic evidence.
    assert!(hdr.iter().all(|v| v.is_finite()));
    let coverage = f64::from(hdr[3]).clamp(0., 1.);
    let c: [f64; 3] =
        std::array::from_fn(|i| (f64::from(hdr[i]) / coverage.max(1e-8)).clamp(0., 65504.));
    let peak = c.iter().copied().fold(0_f64, f64::max);
    let mapped: [f64; 3] = if peak < 0.8 {
        [c[0], c[1], c[2]]
    } else {
        let new_peak = 1. - 0.2 * 0.2 / (peak + 0.2 - 0.8);
        let g = 1. - 1. / (0.15 * (peak - new_peak) + 1.);
        std::array::from_fn(|i| c[i] * (new_peak / peak) * (1. - g) + new_peak * g)
    };
    let mut result = [0_u8; 4];
    for i in 0..3 {
        let s = f64::from(BACKDROP[i]) / 255.;
        let backdrop = if s <= 0.04045 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        };
        let linear = backdrop * (1. - coverage) + mapped[i] * coverage;
        let encoded = if linear <= 0.0031308 {
            12.92 * linear
        } else {
            1.055 * linear.powf(1. / 2.4) - 0.055
        };
        result[i] = (encoded.clamp(0., 1.) * 255.).round() as u8;
    }
    result[3] = 255;
    result
}
fn png(out: &Path, name: &str, raw: &[u8], stride: usize) -> Value {
    let mut pixels = Vec::with_capacity((OUTPUT[0] * OUTPUT[1] * 4) as usize);
    for row in raw.chunks_exact(stride) {
        for p in row[..OUTPUT[0] as usize * 8].as_chunks::<8>().0 {
            pixels.extend_from_slice(&display(pixel_half(p)));
        }
    }
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(out.join(name))
        .unwrap();
    let mut e = png::Encoder::new(file, OUTPUT[0], OUTPUT[1]);
    e.set_color(png::ColorType::Rgba);
    e.set_depth(png::BitDepth::Eight);
    let mut writer = e.write_header().unwrap();
    writer.write_image_data(&pixels).unwrap();
    writer.finish().unwrap();
    json!({"name":name,"sha256":hex(&std::fs::read(out.join(name)).unwrap()),"rgba_sha256":hex(&pixels),"size":OUTPUT})
}
#[derive(Clone, Copy)]
struct Plane {
    rect: [u32; 4],
    bpp: u32,
    stride: u32,
    offset: u64,
}
fn planes() -> [Plane; 8] {
    capture_planes(false)
}
fn pose(stage: &Stage, input: &mut SceneInput, step: u32, camera_only: bool) {
    stage.pose(input, if camera_only { 1 } else { step });
    if camera_only {
        let camera = node(input, &stage.camera);
        input.nodes[camera].world = camera_world(step);
    }
}
#[test]
fn recurrence_camera_only_pose_keeps_every_other_node_fixed() {
    let stage = Stage::new(true);
    let mut input = stage.input();
    stage.pose(&mut input, 1);
    let original: Vec<_> = input.nodes.iter().map(|n| n.world).collect();
    let camera = node(&input, &stage.camera);
    let mover = node(&input, stage.mover.as_ref().unwrap());
    for step in [1, 8, 15, 32, 64] {
        pose(&stage, &mut input, step, true);
        assert_eq!(input.nodes[camera].world, camera_world(step));
        assert_eq!(input.nodes.len(), original.len());
        for (index, before) in original.iter().enumerate() {
            if index != camera {
                assert_eq!(input.nodes[index].world, *before);
            }
        }
    }
    assert_eq!(input.nodes[mover].world, mover_world(1));
    pose(&stage, &mut input, 64, false);
    assert_eq!(input.nodes[mover].world, mover_world(64));
    assert_ne!(input.nodes[mover].world, mover_world(1));
}
fn capture_planes(reference_state: bool) -> [Plane; 8] {
    let mut offset = 0;
    std::array::from_fn(|i| {
        let rect: [u32; 4] = match (reference_state, i) {
            (true, 2) => [294, 62, 108, 108],
            (true, 4) => [0, 48, 800, 300],
            (_, i) => match i {
                0 | 1 => [0, 0, 960, 540],
                2..=4 => [592, 128, 208, 208],
                5 | 6 => [294, 62, 108, 108],
                _ => [0, 0, 480, 270],
            },
        };
        let bpp = if reference_state && i == 3 {
            8
        } else {
            [8, 8, 4, 4, 8, 8, 16, 4][i]
        };
        let stride = (rect[2] * bpp).div_ceil(256) * 256;
        let p = Plane {
            rect,
            bpp,
            stride,
            offset,
        };
        offset += u64::from(stride) * u64::from(rect[3]);
        p
    })
}
fn cold_delta(candidate: &[u8], baseline: &[u8]) -> u16 {
    let mut maximum = 0;
    for (a, b) in candidate
        .as_chunks::<2>()
        .0
        .iter()
        .zip(baseline.as_chunks::<2>().0)
    {
        let a = u16::from_le_bytes(*a);
        let b = u16::from_le_bytes(*b);
        assert!(half(a).is_finite() && half(b).is_finite() && half(a) >= 0. && half(b) >= 0.);
        let a = if half(a) == 0. { 0 } else { a };
        let b = if half(b) == 0. { 0 } else { b };
        maximum = maximum.max(a.abs_diff(b));
    }
    maximum
}
fn color_range(raw: &[u8]) -> Value {
    let mut low = [f32::INFINITY; 4];
    let mut high = [f32::NEG_INFINITY; 4];
    let mut negative_rgb = 0;
    let mut nonzero_rgb_at_zero_alpha = 0;
    for pixel in raw.as_chunks::<8>().0 {
        let values: [f32; 4] =
            std::array::from_fn(|i| half(u16::from_le_bytes([pixel[2 * i], pixel[2 * i + 1]])));
        assert!(values.iter().all(|v| v.is_finite()), "nonfinite output");
        for i in 0..4 {
            low[i] = low[i].min(values[i]);
            high[i] = high[i].max(values[i]);
        }
        negative_rgb += usize::from(values[..3].iter().any(|v| *v < 0.));
        nonzero_rgb_at_zero_alpha +=
            usize::from(values[3] == 0. && values[..3].iter().any(|v| *v != 0.));
    }
    json!({"minimum":low,"maximum":high,"negative_rgb_pixels":negative_rgb,
        "nonzero_rgb_at_zero_alpha_pixels":nonzero_rgb_at_zero_alpha})
}
#[test]
fn recurrence_fixture_layout_and_display_controls() {
    let ordinary = crate::temporal::jitter::offset(0, true);
    {
        let _guard = crate::temporal::jitter::DiagnosticHalton32::begin();
        assert_eq!(crate::temporal::jitter::phase_count(true), 32);
        assert_eq!(crate::temporal::jitter::phase_count(false), 8);
        let first = crate::temporal::jitter::offset(0, true);
        assert_eq!(first[0], 0.);
        assert!((first[1] - 1. / 6.).abs() < 1e-7);
        assert_ne!(crate::temporal::jitter::offset(8, true), first);
        assert_eq!(crate::temporal::jitter::offset(32, true), first);
        // A different host thread must never inherit this test's override.
        assert_eq!(
            std::thread::spawn(|| crate::temporal::jitter::phase_count(true))
                .join()
                .unwrap(),
            8
        );
    }
    assert_eq!(crate::temporal::jitter::phase_count(true), 8);
    assert_eq!(crate::temporal::jitter::offset(0, true), ordinary);
    let p = planes();
    assert_eq!(p[0].stride, 7680);
    assert_eq!(p[1].offset, 4_147_200);
    let total = p[7].offset + u64::from(p[7].stride * p[7].rect[3]);
    assert!(total < 11 * 1024 * 1024);
    let reference = capture_planes(true);
    assert_eq!(reference[2].rect, [294, 62, 108, 108]);
    assert_eq!(reference[3].bpp, 8);
    assert_eq!(reference[4].rect, [0, 48, 800, 300]);
    for pair in reference.windows(2) {
        assert_eq!(
            pair[0].offset + u64::from(pair[0].stride * pair[0].rect[3]),
            pair[1].offset
        );
        assert_eq!(pair[0].stride % 256, 0);
    }
    assert!(
        reference[7].offset + u64::from(reference[7].stride * reference[7].rect[3])
            < 13 * 1024 * 1024
    );
    assert_eq!(2 * 960 * 540 * (8 + 4 + 4 + 8), 24_883_200);
    assert_eq!(display([0.; 4]), [20, 21, 25, 255]);
    assert_eq!(display([0., 0., 0., 1.]), [0, 0, 0, 255]);
    assert_eq!(half(0x3c00), 1.);
    assert_eq!(half(0xbc00), -1.);
}
#[test]
#[ignore = "bounded standalone recurrence experiment; explicit GPU grant required"]
fn actual_scene_recurrence_comparison() {
    let out = PathBuf::from(std::env::var("INCANT_RECURRENCE_EXPERIMENT_OUT").unwrap());
    std::fs::create_dir(&out).expect("fresh experimental output directory");
    let renderer = owner_renderer();
    assert_eq!(renderer.adapter.get_info().backend, wgpu::Backend::Metal);
    let external =
        std::env::var("INCANT_RECURRENCE_ABLATION").as_deref() == Ok("external-reference");
    let camera_only = match std::env::var("INCANT_REFERENCE_MOTION").as_deref() {
        Err(std::env::VarError::NotPresent) | Ok("combined") => false,
        Ok("camera-only") if external => true,
        other => panic!("unknown diagnostic motion: {other:?}"),
    };
    let reference_state = match std::env::var("INCANT_REFERENCE_STATE").as_deref() {
        Err(std::env::VarError::NotPresent) => false,
        Ok("capture") if external => true,
        other => panic!("unknown diagnostic state capture: {other:?}"),
    };
    let halton32 = match std::env::var("INCANT_REFERENCE_JITTER").as_deref() {
        Err(std::env::VarError::NotPresent) | Ok("incant8") => false,
        Ok("halton32") if external => true,
        other => panic!("unknown diagnostic jitter: {other:?}"),
    };
    let _jitter_guard = halton32.then(crate::temporal::jitter::DiagnosticHalton32::begin);
    let frame_count = match std::env::var("INCANT_REFERENCE_FRAMES").as_deref() {
        Err(std::env::VarError::NotPresent) | Ok("24") => 24,
        Ok("64") if external => 64,
        other => panic!("unknown diagnostic frame count: {other:?}"),
    };
    let (ablation, cubic, lock, nearest, coarse) =
        match std::env::var("INCANT_RECURRENCE_ABLATION").as_deref() {
            Err(std::env::VarError::NotPresent) | Ok("recurrence") => {
                (false, false, false, false, false)
            }
            Ok("unclipped") => (true, false, false, false, false),
            Ok("unclipped-cubic") => (true, true, false, false, false),
            Ok("bounded-lock") => (false, false, true, false, false),
            Ok("nearest-surface") => (false, false, false, true, false),
            Ok("external-reference") => (false, false, false, false, false),
            Ok("coarse-shading") => (false, false, false, false, true),
            other => panic!("unknown recurrence experiment mode: {other:?}"),
        };
    let experiment = Experiment::new(&renderer, ablation, cubic, lock, nearest, coarse);
    let (_target, view) = review_target(&renderer);
    let layout = capture_planes(reference_state);
    let total = layout[7].offset + u64::from(layout[7].stride * layout[7].rect[3]);
    let read = renderer.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Bounded experiment packed raw planes"),
        size: total,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut rows = Vec::new();
    let mut images = Vec::new();
    let mut submissions = 0;
    let mut maps = 0;
    let mut nonzero_lock_samples = 0usize;
    for moving in [false, true] {
        let reference = external.then(|| super::reference_fixture::Reference::new(&renderer));
        let stage = Stage::new(moving);
        let mut input = stage.input();
        stage.pose(&mut input, 1);
        let mut scene = variant(&renderer, &input, &stage.camera, Some(HALF), Variant::Up);
        let slots: [Slot; 2] = std::array::from_fn(|_| Slot::new(&renderer.device, OUTPUT));
        let mut previous_jitter = [0_f32; 2];
        for frame in 1..=frame_count {
            pose(
                &stage,
                &mut input,
                if moving { frame } else { 1 },
                camera_only,
            );
            scene
                .submit(
                    &renderer,
                    &input,
                    RetainedTarget {
                        view: &view,
                        format: FORMAT,
                        width: OUTPUT[0],
                        height: OUTPUT[1],
                        viewport: None,
                    },
                )
                .unwrap();
            submissions += 1;
            let state = scene.upscale_frame().unwrap();
            let jitter = state.motion().jitter_offset().unwrap();
            assert_eq!(
                state.motion().jitter_phase(),
                Some((frame - 1) % if halton32 { 32 } else { 8 })
            );
            assert_eq!(state.motion().has_previous(), frame > 1);
            let history = scene
                .upscaling
                .as_ref()
                .unwrap()
                .diagnostic_surfaces()
                .unwrap();
            let current = renderer.frames.diagnostic_color();
            let motion = scene.motion.as_ref().unwrap().readback_view().unwrap();
            let depth = scene.motion.as_ref().unwrap().depth_test_view().unwrap();
            for v in [&current, &motion, &depth] {
                assert_eq!([v.texture().width(), v.texture().height()], SOURCE);
            }
            let destination = &slots[frame as usize % 2];
            let previous = &slots[1 - frame as usize % 2];
            let uniform = Frame {
                source: [0., 0., 480., 270.],
                output: [0., 0., 960., 540.],
                depth: [NEAR as f32, FAR as f32, 0., if frame > 1 { 1. } else { 0. }],
                options: [
                    REVIEW.history_weight,
                    REVIEW.relative_depth,
                    REVIEW.absolute_depth,
                    REVIEW.reactive_threshold,
                ],
                reactive: [
                    REVIEW.reactive_strength,
                    0.,
                    1.,
                    if frame > 1 { 1. } else { 0. },
                ],
                jitter: [
                    jitter[0] / 480.,
                    jitter[1] / 270.,
                    previous_jitter[0] / 480.,
                    previous_jitter[1] / 270.,
                ],
            };
            let mut encoder = renderer.device.create_command_encoder(&Default::default());
            if frame == 1 {
                previous.clear(&mut encoder);
            }
            experiment.resolve(
                &renderer,
                &mut encoder,
                destination,
                previous,
                uniform,
                [&current, &motion, &depth],
            );
            if let Some(reference) = &reference {
                reference.dispatch(
                    &renderer,
                    &mut encoder,
                    frame,
                    jitter,
                    previous_jitter,
                    [&current, &motion, &depth],
                );
            }
            let candidate_view = reference
                .as_ref()
                .map_or(&destination.views[0], |v| &v.output);
            let mut views = [
                candidate_view,
                &history[0],
                &destination.views[1],
                &destination.views[2],
                &destination.views[3],
                &current,
                &motion,
                &depth,
            ];
            if reference_state {
                let internal = reference.as_ref().unwrap().diagnostic_state(frame);
                views[2..5].copy_from_slice(&internal);
            }
            for (i, (v, p)) in views.iter().zip(layout).enumerate() {
                let mut copy = v.texture().as_image_copy();
                copy.origin = wgpu::Origin3d {
                    x: p.rect[0],
                    y: p.rect[1],
                    z: 0,
                };
                if i == 7 {
                    copy.aspect = wgpu::TextureAspect::DepthOnly;
                }
                encoder.copy_texture_to_buffer(
                    copy,
                    wgpu::TexelCopyBufferInfo {
                        buffer: &read,
                        layout: wgpu::TexelCopyBufferLayout {
                            offset: p.offset,
                            bytes_per_row: Some(p.stride),
                            rows_per_image: Some(p.rect[3]),
                        },
                    },
                    wgpu::Extent3d {
                        width: p.rect[2],
                        height: p.rect[3],
                        depth_or_array_layers: 1,
                    },
                );
            }
            renderer.queue.submit([encoder.finish()]);
            submissions += 1;
            let (tx, rx) = std::sync::mpsc::channel();
            read.slice(..).map_async(wgpu::MapMode::Read, move |v| {
                let _ = tx.send(v);
            });
            renderer
                .device
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: Some(Duration::from_secs(10)),
                })
                .unwrap();
            rx.recv_timeout(Duration::from_secs(10)).unwrap().unwrap();
            maps += 1;
            let bytes = read.slice(..).get_mapped_range();
            let data = |i: usize| {
                let p = layout[i];
                &bytes[p.offset as usize..p.offset as usize + (p.stride * p.rect[3]) as usize]
            };
            if !reference_state {
                for sample in data(3).as_chunks::<4>().0 {
                    let remaining = half(u16::from_le_bytes([sample[2], sample[3]]));
                    assert!((0.0..=7.0).contains(&remaining) && remaining == remaining.floor());
                    nonzero_lock_samples += usize::from(remaining > 0.0);
                }
            }
            let label = if !moving {
                "stationary"
            } else if camera_only {
                "camera-only"
            } else {
                "moving"
            };
            let mut files = Vec::new();
            let names = if reference_state {
                [
                    "candidate",
                    "baseline",
                    "reference-masks",
                    "reference-history",
                    "reference-locks",
                    "current",
                    "motion",
                    "native-depth",
                ]
            } else {
                [
                    "candidate",
                    "baseline",
                    "depth",
                    "evidence",
                    "recurrence",
                    "current",
                    "motion",
                    "native-depth",
                ]
            };
            for (i, name) in names.into_iter().enumerate() {
                let p = layout[i];
                let name = format!("{label}-{frame:02}-{name}.bin");
                std::fs::write(out.join(&name), data(i)).unwrap();
                files.push(json!({"name":name,"sha256":hex(data(i)),"rect":p.rect,"stride":p.stride,"bytes_per_pixel":p.bpp,"format":format!("{:?}",views[i].texture().format())}));
            }
            // This equality compares two Incant kernels. The external kernel
            // differs even on reset and can emit signed RGB after YCoCg inverse.
            // Preserve its full raw range instead of interpreting signed half
            // bit patterns as a nonnegative half-code distance.
            let delta = if frame == 1 && !external {
                Some(cold_delta(data(0), data(1)))
            } else {
                None
            };
            let capture =
                frame == 8 || frame == 24 || frame == 32 || frame == 64 || (!moving && frame == 1);
            if capture {
                for (i, name) in ["candidate", "baseline"].into_iter().enumerate() {
                    images.push(png(
                        &out,
                        &format!("{label}-{frame:02}-{name}.png"),
                        data(i),
                        layout[i].stride as usize,
                    ));
                }
            }
            rows.push(json!({"case":label,"frame":frame,"phase":state.motion().jitter_phase(),"jitter":jitter,"previous_jitter":previous_jitter,"cold_max_half_code_delta":delta,"candidate_range":color_range(data(0)),"planes":files}));
            // Preserve raw bytes and comparisons before refusing a cold mismatch.
            std::fs::write(
                out.join("progress.json"),
                serde_json::to_vec_pretty(
                    &json!({"submissions":submissions,"maps":maps,"frames":rows,"images":images}),
                )
                .unwrap(),
            )
            .unwrap();
            drop(bytes);
            read.unmap();
            previous_jitter = jitter;
            if let Some(delta) = delta.filter(|_| !external) {
                assert!(
                    delta <= 1,
                    "cold reconstruction differs by {delta} half codes"
                );
            }
        }
    }
    assert_eq!(
        (submissions, maps, images.len()),
        (
            frame_count * 4,
            frame_count * 2,
            if frame_count == 24 { 10 } else { 18 }
        )
    );
    let report = json!({"schema":"incant-recurrence-experiment-v1","adapter":describe(&renderer),"output":OUTPUT,"source":SOURCE,"configuration":policy(),"submissions":submissions,"maps":maps,"experimental_resolves":48,"history_initialization_passes":2,"extra_history_bytes":24_883_200,"readback_bytes":total,"raw_bytes":total*48,"frames":rows,"images":images,"shader_sha256":hex(SHADER.as_bytes()),"tone_map_sha256":hex(include_bytes!("tone_map.wgsl")),"output_shader_sha256":hex(include_bytes!("output.wgsl")),"scope":"Test-only ordinary recurrence over actual UP scene inputs; CPU tone conversion excludes current-only overlays and is not exact production display. No performance or appearance acceptance."});
    let mut report = report;
    report["experimental_resolves"] = json!(frame_count * 2);
    report["raw_bytes"] = json!(total * u64::from(frame_count) * 2);
    report["diagnostic_jitter"] = json!(if halton32 {
        "negative_halton_2_3_32"
    } else {
        "incant_stratified_8"
    });
    report["rectification"] = json!(if ablation {
        "disabled_for_diagnosis_only"
    } else {
        "recurrence_candidate"
    });
    if external {
        report["external_reference"] = json!({
            "upstream": "https://github.com/pmndrs/upscaler",
            "commit": "96184a6812bc5fd5e5fd7898257788a1663105a9",
            "algorithm_modified": false,
            "dispatches": frame_count * 2 * 7,
            "input_adapter": "Incant jittered previous-current UV to unjittered current-previous; opposite jitter convention; invalid vectors zeroed",
            "settings": "fixed exposure 1, max accumulation 24, thin locks and shading detector enabled, no sharpening; sequence recorded in diagnostic_jitter",
            "cold_equality_required": false,
            "auxiliary_planes": if reference_state { "reference masks RGBA8, history RGB+age RGBA16, locks life+luma+shading+alpha RGBA16; exact crop/stride per plane" } else { "Incant recurrence path; NOT external reference state" },
            "limits": "Test-only external reference. No production adoption, invalid-motion/alpha conformance, performance or memory admission claim."
        });
        report["scope"] = json!(
            "Same actual UP scene inputs; external pinned compute reference compared with Incant baseline. CPU tone transform, no display/performance/adoption claim."
        );
    }
    report["bounded_lock"] = json!(lock);
    report["nearest_surface_guidance"] = json!(nearest);
    report["coarse_shading_transfer"] = json!(coarse);
    report["four_channel_history_semantics"] = json!(if coarse {
        "coarse_rgb_mean_and_luma_sigma"
    } else {
        "four_recent_fresh_luminances"
    });
    report["nonzero_lock_samples"] = json!(nonzero_lock_samples);
    report["reference_state_captured"] = json!(reference_state);
    report["diagnostic_motion"] = json!(if camera_only {
        "camera pans after frame 14; mover and every other node held at frame 1"
    } else {
        "camera pans after frame 14; mover begins after frame 8"
    });
    report["recurrence_state_captured"] = json!(!reference_state);
    if reference_state {
        report["four_channel_history_semantics"] =
            json!("reference lock life, tracked luma, shading change, resolved alpha");
        report["nonzero_lock_samples"] = Value::Null;
    }
    report["history_filter"] = json!(if cubic {
        "bounded_signed_cubic_diagnostic"
    } else {
        "positive_bilinear_mass"
    });
    std::fs::write(
        out.join("report.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    if lock {
        assert!(
            nonzero_lock_samples > 0,
            "scene must actually exercise retention state"
        );
    } else {
        assert_eq!(nonzero_lock_samples, 0);
    }
    println!(
        "RECURRENCE_EXPERIMENT frames={} submissions={submissions} maps={maps} resolves={} pngs={} adapter={:?}",
        frame_count * 2,
        frame_count * 2,
        images.len(),
        renderer.adapter.get_info()
    );
}
