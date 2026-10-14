//! Test-only camera/owner reconstruction diagnostic. Never a production shader.
use super::*;
use glam::{Mat4, Vec2, Vec3};
use wgpu::util::DeviceExt;
#[path = "reconstruction_owner_camera_controls.rs"]
mod controls;
#[path = "reconstruction_owner_cubic_controls.rs"]
mod cubic_controls;
#[path = "reconstruction_owner_footprint_controls.rs"]
mod footprint_controls;
#[path = "reconstruction_owner_output_controls.rs"]
mod output_controls;
#[path = "reconstruction_owner_sampling.rs"]
mod sampling;
#[path = "reconstruction_owner_camera_support.rs"]
mod support;

const FORMATS: [wgpu::TextureFormat; 4] = [
    wgpu::TextureFormat::Rgba16Float,
    wgpu::TextureFormat::R32Float,
    wgpu::TextureFormat::Rg32Uint,
    wgpu::TextureFormat::Rg16Float,
];
#[derive(Clone, Copy)]
struct Plane {
    rect: [u32; 4],
    bpp: u32,
    stride: u32,
    offset: u64,
}
#[derive(Clone, Copy)]
enum Kernel {
    Point,
    Footprint,
}
#[derive(Clone, Copy)]
enum History {
    Tent,
    Cubic,
}
impl History {
    fn name(self) -> &'static str {
        match self {
            Self::Tent => "tent",
            Self::Cubic => "clamped-cubic-color-positive-mass",
        }
    }
    fn selected() -> Self {
        match std::env::var("INCANT_OWNER_CAMERA_HISTORY").as_deref() {
            Err(std::env::VarError::NotPresent) | Ok("tent") => Self::Tent,
            Ok("cubic") => Self::Cubic,
            Ok(other) => panic!("unknown diagnostic history filter: {other}"),
            Err(error) => panic!("invalid diagnostic history filter: {error}"),
        }
    }
}
impl Kernel {
    fn name(self) -> &'static str {
        match self {
            Self::Point => "point",
            Self::Footprint => "integrated-footprint",
        }
    }
    fn selected() -> Self {
        match std::env::var("INCANT_OWNER_CAMERA_KERNEL").as_deref() {
            Err(std::env::VarError::NotPresent) | Ok("point") => Self::Point,
            Ok("footprint") => Self::Footprint,
            Ok(other) => panic!("unknown diagnostic kernel: {other}"),
            Err(error) => panic!("invalid diagnostic kernel: {error}"),
        }
    }
}
fn shader() -> String {
    shader_with_options(3, Kernel::selected(), History::selected())
}
fn shader_with_radius(radius: i32) -> String {
    shader_with_kernel(radius, Kernel::Point)
}
fn shader_with_kernel(radius: i32, kernel: Kernel) -> String {
    shader_with_options(radius, kernel, History::Tent)
}
fn shader_with_options(radius: i32, kernel: Kernel, history: History) -> String {
    assert!([2, 3].contains(&radius));
    let original = include_str!("upscaling/resolve_owner.wgsl");
    let entry = "@fragment fn resolve(@builtin(position) position:vec4f)";
    assert_eq!(original.matches(entry).count(), 1);
    let mut owner = original.replace(entry, "fn original_resolve(position:vec4f)");
    // Diagnostic correctness successor: full alpha and one depth layer are
    // independent facts. A rounded opaque mixture must not enter the path for
    // genuine fractional coverage. The production source remains untouched.
    for (before, after) in [
        (
            "valid:bool, single:bool, surface:bool, changed:bool,",
            "valid:bool, single:bool, opaque:bool, surface:bool, changed:bool,",
        ),
        (
            "Evidence(vec4f(0.0),0.0,true,true,false,false,",
            "Evidence(vec4f(0.0),0.0,true,true,true,false,false,",
        ),
        (
            "if v.a<1.0 || z<=0.0 {e.single=false;}",
            "if v.a<1.0 || z<=0.0 {e.single=false;e.opaque=false;}",
        ),
        (
            "if !e.surface {e.single=false; return e;}",
            "if !e.surface {e.single=false;e.opaque=false;return e;}",
        ),
        (
            "if e.single {result.color.a=1.0;}",
            "if e.opaque {result.color.a=1.0;}",
        ),
        (
            "if e.single && c.a==1.0 {result.color.a=1.0;}",
            "if e.opaque && c.a==1.0 {result.color.a=1.0;}",
        ),
    ] {
        assert_eq!(owner.matches(before).count(), 1, "opacity splice: {before}");
        owner = owner.replace(before, after);
    }
    let footprint = match kernel {
        Kernel::Point => "",
        Kernel::Footprint => {
            assert_eq!(owner.matches("fn kernel_weight(").count(), 1);
            owner = owner.replace("fn kernel_weight(", "fn point_kernel_weight(");
            include_str!("upscaling/owner_footprint_experiment.wgsl")
        }
    };
    let mut parts = vec![include_str!("upscaling/common.wgsl").to_owned(), owner];
    if !footprint.is_empty() {
        parts.push(footprint.to_owned());
    }
    let mut camera = include_str!("upscaling/owner_camera_experiment.wgsl").replace(
        "const CAMERA_RADIUS:i32=3;",
        &format!("const CAMERA_RADIUS:i32={radius};"),
    );
    let opacity_guard = "if e.single || (changes.header.y&1u)!=0u";
    assert_eq!(camera.matches(opacity_guard).count(), 1);
    camera = camera.replace(opacity_guard, "if e.opaque || (changes.header.y&1u)!=0u");
    if let History::Cubic = history {
        // Keep the baseline helper byte-exact; each splice is a bounded,
        // test-only estimator replacement with an explicit source guard.
        for (before, after) in [
            (
                "var nearest=frame.depth.y+1.0; var selected=0u;",
                "var nearest=frame.depth.y+1.0; var selected=0u; var selected_old=vec2i(0); var selected_projection=vec2f(0.0);",
            ),
            (
                "nearest=v.position.z;selected=v.owner;",
                "nearest=v.position.z;selected=v.owner;selected_old=center+vec2i(x,y);selected_projection=v.position.xy;",
            ),
            (
                "result.color=clean((numerator/mass*weight+e.sum)/total);",
                "var history_color=numerator/mass;\n    let cubic=cubic_history(p,selected_old,selected_projection,selected,nearest,e);\n    if cubic.ok {history_color=cubic.color;age=max(age,cubic.age);}\n    result.color=clean((history_color*weight+e.sum)/total);",
            ),
        ] {
            assert_eq!(camera.matches(before).count(), 1, "cubic splice: {before}");
            camera = camera.replace(before, after);
        }
        parts.push(include_str!("upscaling/owner_history_cubic_experiment.wgsl").to_owned());
    }
    // Canonicalize only the final attachment value, after every blend. Cleaning
    // intermediate f32 values cannot detect alpha that underflows at storage.
    assert_eq!(camera.matches(entry).count(), 1);
    parts.push(camera.replace(entry, "fn experiment_resolve(position:vec4f)"));
    parts.push(include_str!("upscaling/owner_output_half_experiment.wgsl").to_owned());
    parts.join("\n")
}
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraPair {
    previous_inverse: [[f32; 4]; 4],
    current_matrix: [[f32; 4]; 4],
}
const _: () = assert!(size_of::<CameraPair>() == 128);
fn matrix(input: &SceneInput, camera: &str) -> Mat4 {
    let n = &input.nodes[node(input, camera)];
    crate::camera::CameraView::authored(n.camera.as_ref().unwrap(), n.world)
        .unwrap()
        .unjittered()
        .matrix(SIZE[0] as f32 / SIZE[1] as f32)
        .unwrap()
}
fn project(matrix: Mat4, world: Vec3) -> (Vec2, f32) {
    let clip = matrix * world.extend(1.);
    assert!(clip.w > 0.);
    let ndc = clip.truncate() / clip.w;
    (
        Vec2::new((ndc.x + 1.) * 0.5 * 960., (1. - ndc.y) * 0.5 * 540.),
        ndc.z,
    )
}
#[test]
fn owner_camera_shader_and_projection_controls() {
    for radius in [2, 3] {
        for kernel in [Kernel::Point, Kernel::Footprint] {
            for history in [History::Tent, History::Cubic] {
                let module =
                    naga::front::wgsl::parse_str(&shader_with_options(radius, kernel, history))
                        .unwrap();
                naga::valid::Validator::new(
                    naga::valid::ValidationFlags::all(),
                    naga::valid::Capabilities::all(),
                )
                .validate(&module)
                .unwrap();
            }
        }
    }
    let stage = Stage::new(true);
    let mut input = stage.input();
    stage.pose(&mut input, 1);
    let previous = matrix(&input, &stage.camera);
    stage.pose(&mut input, 24);
    let current = matrix(&input, &stage.camera);
    for point in [WIRES, FENCE, SIGN, MOVER_AT] {
        let world = Vec3::from_array(point.map(|v| v as f32));
        let (pixel, device_z) = project(previous, world);
        let near = NEAR as f32;
        let far = FAR as f32;
        let linear_z = 1. / ((1. - device_z) / near + device_z / far);
        let restored_z = (1. / near - 1. / linear_z) / (1. / near - 1. / far);
        let reconstructed = previous.inverse()
            * glam::Vec4::new(
                2. * pixel.x / 960. - 1.,
                1. - 2. * pixel.y / 540.,
                restored_z,
                1.,
            );
        let restored_world = reconstructed.truncate() / reconstructed.w;
        assert!((restored_world - world).length() < 0.001);
        let (expected, _) = project(current, world);
        let (actual, _) = project(current, restored_world);
        assert!((actual - expected).length() < 0.01);
    }
}

struct Slot {
    views: [wgpu::TextureView; 4],
}
fn attachment(view: &wgpu::TextureView) -> Option<wgpu::RenderPassColorAttachment<'_>> {
    Some(wgpu::RenderPassColorAttachment {
        view,
        depth_slice: None,
        resolve_target: None,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
            store: wgpu::StoreOp::Store,
        },
    })
}
impl Slot {
    fn new(d: &wgpu::Device) -> Self {
        Self::sized(d, [960, 540])
    }
    fn sized(d: &wgpu::Device, size: [u32; 2]) -> Self {
        Self {
            views: FORMATS.map(|format| {
                d.create_texture(&wgpu::TextureDescriptor {
                    label: Some("Test-only owner camera history"),
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
                        | wgpu::TextureUsages::COPY_DST
                        | wgpu::TextureUsages::COPY_SRC,
                    view_formats: &[],
                })
                .create_view(&Default::default())
            }),
        }
    }
    fn clear(&self, encoder: &mut wgpu::CommandEncoder) {
        drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Initialize test owner camera history"),
            color_attachments: &self.views.each_ref().map(attachment),
            ..Default::default()
        }));
    }
}
struct Experiment {
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
}
impl Experiment {
    fn new(renderer: &Renderer) -> Self {
        Self::with_options(renderer, 3, Kernel::selected(), History::selected())
    }
    fn with_radius(renderer: &Renderer, radius: i32) -> Self {
        Self::with_kernel(renderer, radius, Kernel::Point)
    }
    fn with_kernel(renderer: &Renderer, radius: i32, kernel: Kernel) -> Self {
        Self::with_options(renderer, radius, kernel, History::Tent)
    }
    fn with_options(renderer: &Renderer, radius: i32, kernel: Kernel, history: History) -> Self {
        let d = &renderer.device;
        let mut entries = crate::upscaling::system::owner_layout_entries();
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: 10,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new(128),
            },
            count: None,
        });
        let layout = d.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Test camera owner bindings"),
            entries: &entries,
        });
        let module = d.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Test camera owner shader"),
            source: wgpu::ShaderSource::Wgsl(shader_with_options(radius, kernel, history).into()),
        });
        let pipeline_layout = d.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = d.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Test-only camera forward gather"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vertex"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("resolve"),
                compilation_options: Default::default(),
                targets: &FORMATS.map(|format| {
                    Some(wgpu::ColorTargetState {
                        format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })
                }),
            }),
            multiview_mask: None,
            cache: None,
        });
        Self { layout, pipeline }
    }
    #[allow(clippy::too_many_arguments)]
    fn resolve(
        &self,
        renderer: &Renderer,
        encoder: &mut wgpu::CommandEncoder,
        previous: &Slot,
        destination: &Slot,
        frame: super::recurrence_fixture::Frame,
        matrices: CameraPair,
        current: [&wgpu::TextureView; 4],
        changes: &wgpu::Buffer,
    ) {
        let d = &renderer.device;
        let uniform = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::bytes_of(&frame),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let cameras = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::bytes_of(&matrices),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let textures = [
            current[0],
            current[1],
            current[2],
            &previous.views[0],
            &previous.views[1],
            &previous.views[3],
            current[3],
            &previous.views[2],
        ];
        let mut entries = vec![wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform.as_entire_binding(),
        }];
        entries.extend(
            textures
                .iter()
                .enumerate()
                .map(|(i, view)| wgpu::BindGroupEntry {
                    binding: i as u32 + 1,
                    resource: wgpu::BindingResource::TextureView(view),
                }),
        );
        entries.push(wgpu::BindGroupEntry {
            binding: 9,
            resource: changes.as_entire_binding(),
        });
        entries.push(wgpu::BindGroupEntry {
            binding: 10,
            resource: cameras.as_entire_binding(),
        });
        let group = d.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.layout,
            entries: &entries,
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Test camera gather"),
            color_attachments: &destination.views.each_ref().map(attachment),
            ..Default::default()
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.draw(0..3, 0..1);
    }
}

#[test]
#[ignore = "bounded test-only owner camera experiment; no production or visual acceptance"]
fn actual_scene_owner_camera_gather() {
    use super::recurrence_fixture::{Frame, color_range, evict_stage, png};
    let out = PathBuf::from(std::env::var("INCANT_OWNER_CAMERA_OUT").unwrap());
    std::fs::create_dir(&out).expect("new diagnostic output");
    let motion = sampling::Motion::selected();
    let sampling = sampling::Sampling::selected();
    let _sampling = sampling.begin();
    let phases = crate::temporal::jitter::phase_count(true);
    let renderer = owner_renderer();
    assert_eq!(renderer.adapter.get_info().backend, wgpu::Backend::Metal);
    let _footprint = crate::temporal::jitter::DiagnosticInternalFootprint::begin();
    let experiment = Experiment::new(&renderer);
    let (_target, view) = review_target(&renderer);
    let mut total = 0;
    let planes: [Plane; 8] = std::array::from_fn(|i| {
        let rect: [u32; 4] = match i {
            0 | 1 => [0, 0, 960, 540],
            2..=4 => [0, 0, 440, 240],
            _ => [0, 0, 480, 270],
        };
        let bpp = [8, 8, 4, 8, 4, 8, 4, 4][i];
        let stride = (rect[2] * bpp).div_ceil(256) * 256;
        let p = Plane {
            rect,
            bpp,
            stride,
            offset: total,
        };
        total += u64::from(stride) * u64::from(rect[3]);
        p
    });
    let read = renderer.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Test camera packed readback"),
        size: total,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut rows = Vec::new();
    let mut images = Vec::new();
    let mut submits = 0;
    let mut maps = 0;
    for eviction in [false, true] {
        let label = if eviction { "eviction" } else { "camera" };
        let stage = Stage::new(true);
        let mut input = stage.input();
        stage.pose(&mut input, 1);
        let mut scene = variant(&renderer, &input, &stage.camera, Some(HALF), Variant::Owner);
        let slots: [Slot; 2] = std::array::from_fn(|_| Slot::new(&renderer.device));
        let mut old_matrix = matrix(&input, &stage.camera);
        let mut old_jitter = [0.; 2];
        for frame in 1..=64 {
            motion.pose(&stage, &mut input, frame);
            if eviction {
                evict_stage(&stage, &mut input, frame);
            }
            let current_matrix = matrix(&input, &stage.camera);
            scene
                .submit(
                    &renderer,
                    &input,
                    RetainedTarget {
                        view: &view,
                        format: FORMAT,
                        width: 960,
                        height: 540,
                        viewport: None,
                    },
                )
                .unwrap();
            submits += 1;
            let state = scene.upscale_frame().unwrap();
            let jitter = state.motion().jitter_offset().unwrap();
            assert_eq!(jitter, crate::temporal::jitter::offset(frame - 1, true));
            assert_eq!(state.motion().has_previous(), frame > 1);
            let motion_state = scene.motion.as_ref().unwrap();
            let (owner, motion, depth) = motion_state.owner_test_views().unwrap();
            let changes = motion_state.owner_test_changes().unwrap();
            let header = motion_state.owner_test_change_header().unwrap();
            let current = renderer.frames.diagnostic_color();
            let history = scene
                .upscaling
                .as_ref()
                .unwrap()
                .diagnostic_surfaces()
                .unwrap();
            let previous = &slots[1 - frame as usize % 2];
            let destination = &slots[frame as usize % 2];
            let uniform = Frame {
                source: [0., 0., 480., 270.],
                output: [0., 0., 960., 540.],
                depth: [NEAR as f32, FAR as f32, 0., f32::from(u8::from(frame > 1))],
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
                    f32::from(u8::from(frame > 1)),
                ],
                jitter: [
                    jitter[0] / 480.,
                    jitter[1] / 270.,
                    old_jitter[0] / 480.,
                    old_jitter[1] / 270.,
                ],
            };
            let mut encoder = renderer.device.create_command_encoder(&Default::default());
            if frame == 1 {
                previous.clear(&mut encoder);
            }
            experiment.resolve(
                &renderer,
                &mut encoder,
                previous,
                destination,
                uniform,
                CameraPair {
                    previous_inverse: old_matrix.inverse().to_cols_array_2d(),
                    current_matrix: current_matrix.to_cols_array_2d(),
                },
                [&current, &motion, &depth, &owner],
                &changes,
            );
            let views = [
                &destination.views[0],
                &history[0],
                &destination.views[1],
                &destination.views[2],
                &destination.views[3],
                &current,
                &owner,
                &depth,
            ];
            for (i, (v, p)) in views.iter().zip(planes).enumerate() {
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
            submits += 1;
            let (tx, rx) = std::sync::mpsc::channel();
            read.slice(..).map_async(wgpu::MapMode::Read, move |r| {
                let _ = tx.send(r);
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
                let p = planes[i];
                &bytes[p.offset as usize..p.offset as usize + (p.stride * p.rect[3]) as usize]
            };
            let mut files = Vec::new();
            for (i, name) in [
                "candidate",
                "baseline",
                "depth",
                "record",
                "evidence",
                "current",
                "owner",
                "native-depth",
            ]
            .into_iter()
            .enumerate()
            {
                let name = format!("{label}-{frame:02}-{name}.bin");
                std::fs::write(out.join(&name), data(i)).unwrap();
                let p = planes[i];
                files.push(json!({"name":name,"sha256":hex(data(i)),"rect":p.rect,"stride":p.stride,"bytes_per_pixel":p.bpp,"format":format!("{:?}",views[i].texture().format())}));
            }
            if [1, 8, 14, 16, 24, 32, 33, 40, 64].contains(&frame) {
                for (i, name) in ["candidate", "baseline"].into_iter().enumerate() {
                    images.push(png(
                        &out,
                        &format!("{label}-{frame:02}-{name}.png"),
                        data(i),
                        planes[i].stride as usize,
                    ));
                }
            }
            let poses: Vec<_> = input
                .nodes
                .iter()
                .map(|n| n.world.to_cols_array())
                .collect();
            rows.push(json!({"case":label,"frame":frame,"jitter":jitter,"jitter_phase":(frame-1)%phases,"node_world_matrices":poses,"changes":header,"previous_matrix":old_matrix.to_cols_array(),"current_matrix":current_matrix.to_cols_array(),"candidate_range":color_range(data(0)),"planes":files}));
            std::fs::write(
                out.join("progress.json"),
                serde_json::to_vec_pretty(
                    &json!({"submissions":submits,"maps":maps,"frames":rows,"images":images}),
                )
                .unwrap(),
            )
            .unwrap();
            drop(bytes);
            read.unmap();
            old_jitter = jitter;
            old_matrix = current_matrix;
        }
    }
    assert_eq!((submits, maps), (256, 128));
    let report = json!({"schema":"incant-owner-camera-diagnostic-v1","adapter":describe(&renderer),"output":SIZE,"source":[480,270],"submissions":submits,"maps":maps,"frames":rows,"images":images,"kernel":Kernel::selected().name(),"history_filter":History::selected().name(),"motion":motion.name(),"sampling":sampling.name(),"jitter_phase_count":phases,"shader_sha256":hex(shader().as_bytes()),"raw_bytes":total*128,"scope":"Test-only radius3 (7x7) forward gather of static single-owner fractional coverage; selected stationary or small camera motion, internal LOD footprint, unchanged history age8. Halton32 changes sequence and count, not a sample-count theorem. No production, arbitrary motion, timing or visual acceptance."});
    std::fs::write(
        out.join("report.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
}
