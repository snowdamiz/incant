//! Numerical filter contract, not appearance acceptance. The ignored case uses
//! two submissions (one per projection), six tiny render passes and no files.
//! Actual Depth32Float is rasterized from known R32Float values. Both actual R8
//! stages are checked independently, so a wrong H pass cannot cancel a wrong V.
use super::math::{AmbientOcclusion, Uniform};
use glam::{DVec3, DVec4, Mat4, Vec3};
use sha2::{Digest, Sha256};
use wgpu::util::DeviceExt as _;

const W: u32 = 19;
const H: u32 = 15;
const RECT: [f32; 4] = [1.25, 1.5, 16.5, 12.25];
const ROW: u32 = 256;
const COPY_BYTES: u64 = ROW as u64 * H as u64;

// Outward f32 arithmetic bounds around an independent f64 scalar oracle. This
// encloses fused/unfused arithmetic rather than adding a tolerance to readback.
// Each R8 store rounds the resulting interval to integer UNORM codes; the next
// stage consumes its *actual* integer readback. All fixture divisors are positive.
#[derive(Clone, Copy, Debug)]
struct I(f64, f64);
impl I {
    fn exact(x: f64) -> Self {
        Self(x, x)
    }
    fn rounded(lo: f64, hi: f64) -> Self {
        assert!(lo.is_finite() && hi.is_finite() && lo <= hi);
        Self((lo as f32).next_down() as f64, (hi as f32).next_up() as f64)
    }
    fn add(self, b: Self) -> Self {
        Self::rounded(self.0 + b.0, self.1 + b.1)
    }
    fn sub(self, b: Self) -> Self {
        Self::rounded(self.0 - b.1, self.1 - b.0)
    }
    fn mul(self, b: Self) -> Self {
        let v = [self.0 * b.0, self.0 * b.1, self.1 * b.0, self.1 * b.1];
        Self::rounded(
            v.into_iter().fold(f64::INFINITY, f64::min),
            v.into_iter().fold(f64::NEG_INFINITY, f64::max),
        )
    }
    fn div(self, b: Self) -> Self {
        assert!(b.0 > 0.);
        self.mul(Self::rounded(1. / b.1, 1. / b.0))
    }
    fn abs(self) -> Self {
        Self(
            if self.0 <= 0. && self.1 >= 0. {
                0.
            } else {
                self.0.abs().min(self.1.abs())
            },
            self.0.abs().max(self.1.abs()),
        )
    }
    fn clamp(self) -> Self {
        Self(self.0.clamp(0., 1.), self.1.clamp(0., 1.))
    }
    fn codes(self) -> [u8; 2] {
        let q = self.clamp();
        [
            (q.0 * 255. + 0.5).floor() as u8,
            (q.1 * 255. + 0.5).floor() as u8,
        ]
    }
}
fn dot(a: [I; 3], b: [I; 3]) -> I {
    a.into_iter()
        .zip(b)
        .fold(I::exact(0.), |sum, (x, y)| sum.add(x.mul(y)))
}
fn transform(m: [[f32; 4]; 4], p: [I; 4]) -> [I; 4] {
    std::array::from_fn(|row| {
        (0..4).fold(I::exact(0.), |s, col| {
            s.add(p[col].mul(I::exact(m[col][row] as f64)))
        })
    })
}
fn index(x: u32, y: u32) -> usize {
    (y * W + x) as usize
}
fn inside(x: u32, y: u32) -> bool {
    let p = [x as f32 + 0.5, y as f32 + 0.5];
    (0..2).all(|a| p[a] >= RECT[a] && p[a] < RECT[a] + RECT[a + 2])
}
#[derive(Clone, Copy)]
enum Metric {
    Plane,
    EyeZ,
    Unweighted,
}
struct Fixture {
    uniform: Uniform,
    depths: Vec<f32>,
    normals: Vec<[u16; 4]>,
    raw: Vec<u8>,
}
impl Fixture {
    fn new(ortho: bool) -> Self {
        let view = Mat4::from_rotation_z(0.37);
        let uniform = Uniform::new(
            crate::camera::CameraView {
                view,
                eye: Vec3::ZERO,
                forward: -Vec3::Z,
                near: 0.1,
                far: 20.,
                fov: 60_f32.to_radians(),
                orthographic_half_height: ortho.then_some(2.5),
                pixel_resolution: None,
                jitter: [0.; 2],
                jitter_guard: [0.; 2],
            },
            RECT,
            [W, H],
            AmbientOcclusion {
                radius: 1.,
                bias: 0.025,
                strength: 1.,
            },
        )
        .unwrap();
        let projection = Mat4::from_cols_array_2d(&uniform.projection).as_dmat4();
        let inverse = projection.inverse();
        let encoded = [0., 0.6, 0.8, 0.].map(|v| half::f16::from_f32(v).to_bits());
        let n_world = DVec3::new(
            0.,
            half::f16::from_bits(encoded[1]).to_f64(),
            half::f16::from_bits(encoded[2]).to_f64(),
        );
        let n = view.as_dmat4().transform_vector3(n_world).normalize();
        let mut f = Self {
            uniform,
            depths: vec![1.; (W * H) as usize],
            normals: vec![[0; 4]; (W * H) as usize],
            raw: vec![0; (W * H) as usize],
        };
        for y in 0..H {
            for x in 0..W {
                let i = index(x, y);
                // Nonseparable, nonconstant input; independent of the SSAO sampler.
                f.raw[i] = ((x * 73 + y * 47 + x * y * 19 + 17) % 251) as u8;
                if !inside(x, y) || (x == 5 && (4..=8).contains(&y)) || (x == 13 && y == 10) {
                    continue;
                }
                let ndc = [
                    2. * (x as f64 + 0.5 - RECT[0] as f64) / RECT[2] as f64 - 1.,
                    1. - 2. * (y as f64 + 0.5 - RECT[1] as f64) / RECT[3] as f64,
                ];
                let a = inverse * DVec4::new(ndc[0], ndc[1], 0., 1.);
                let b = inverse * DVec4::new(ndc[0], ndc[1], 1., 1.);
                let a = a.truncate() / a.w;
                let b = b.truncate() / b.w;
                let ray = b - a;
                // A sloping plane (including large eye-z differences) and a second
                // displaced parallel slab with an irregular boundary.
                let offset = if x >= 10 && y >= 4 { 0.8 } else { 0. };
                let plane = n.dot(DVec3::new(0., 0., -4.)) + offset;
                let p = a + ray * ((plane - n.dot(a)) / n.dot(ray));
                let clip = projection * p.extend(1.);
                let depth = (clip.z / clip.w) as f32;
                assert!(depth > 0. && depth < 1. && p.z < 0.);
                f.depths[i] = depth;
                f.normals[i] = if (x + y) % 4 == 0 {
                    encoded.map(|bits| {
                        half::f16::from_f32(-half::f16::from_bits(bits).to_f32()).to_bits()
                    })
                } else {
                    encoded
                };
                // Invalid zero and subthreshold normals must return accessibility1.
                if (x, y) == (7, 6) {
                    f.normals[i] = [0; 4];
                }
                if (x, y) == (8, 9) {
                    f.normals[i] = [half::f16::from_f32(0.25).to_bits(), 0, 0, 0];
                }
            }
        }
        f
    }
    fn position(&self, x: u32, y: u32) -> [I; 3] {
        let uv = std::array::from_fn::<_, 2, _>(|a| {
            I::exact([x, y][a] as f64 + 0.5)
                .sub(I::exact(RECT[a] as f64))
                .div(I::exact(RECT[a + 2] as f64))
        });
        let p = transform(
            self.uniform.inverse,
            [
                uv[0].mul(I::exact(2.)).sub(I::exact(1.)),
                I::exact(1.).sub(uv[1].mul(I::exact(2.))),
                I::exact(self.depths[index(x, y)] as f64),
                I::exact(1.),
            ],
        );
        std::array::from_fn(|a| p[a].div(p[3]))
    }
    fn normal(&self, i: usize) -> Option<[I; 3]> {
        let stored = self.normals[i].map(|b| half::f16::from_bits(b).to_f64());
        if stored[..3].iter().map(|v| v * v).sum::<f64>() < 0.5 {
            return None;
        }
        let v = transform(self.uniform.view, stored.map(I::exact));
        let n = [v[0], v[1], v[2]];
        let length = dot(n, n);
        let length = I::rounded(length.0.sqrt(), length.1.sqrt());
        Some(n.map(|c| c.div(length)))
    }
    fn stage(&self, src: &[u8], axis: usize, metric: Metric) -> Vec<[u8; 2]> {
        (0..W * H)
            .map(|i| {
                let (x, y) = (i % W, i / W);
                let i = i as usize;
                if !inside(x, y) || self.depths[i] >= 1. {
                    return [255; 2];
                }
                let Some(n) = self.normal(i) else {
                    return [255; 2];
                };
                let p = self.position(x, y);
                let t = p[2].mul(I::exact(-0.01));
                let bias = self.uniform.parameters[1] as f64;
                let tau = I(t.0.max(bias), t.1.max(bias));
                let sample = |j: usize| I::rounded(src[j] as f64 / 255., src[j] as f64 / 255.);
                let mut total = sample(i);
                let mut mass = I::exact(1.);
                for offset in [-2_i32, -1, 1, 2] {
                    let mut xy = [x as i32, y as i32];
                    xy[axis] += offset;
                    if xy[0] < 0 || xy[1] < 0 || xy[0] >= W as i32 || xy[1] >= H as i32 {
                        continue;
                    }
                    let (qx, qy) = (xy[0] as u32, xy[1] as u32);
                    let j = index(qx, qy);
                    if !inside(qx, qy) || self.depths[j] >= 1. {
                        continue;
                    }
                    let q = self.position(qx, qy);
                    let d = std::array::from_fn(|a| q[a].sub(p[a]));
                    let distance = match metric {
                        Metric::Plane => dot(d, n),
                        Metric::EyeZ => d[2],
                        Metric::Unweighted => I::exact(0.),
                    }
                    .abs();
                    let w = I::exact(1.)
                        .sub(distance.div(tau))
                        .clamp()
                        .mul(I::exact(if offset.abs() == 2 { 0.5 } else { 1. }));
                    total = total.add(w.mul(sample(j)));
                    mass = mass.add(w);
                }
                total.div(mass).codes()
            })
            .collect()
    }
}
fn middle(bounds: &[[u8; 2]]) -> Vec<u8> {
    bounds
        .iter()
        .map(|p| ((u16::from(p[0]) + u16::from(p[1])) / 2) as u8)
        .collect()
}
fn difference(a: &[u8], b: &[u8]) -> u8 {
    a.iter().zip(b).map(|(a, b)| a.abs_diff(*b)).max().unwrap()
}
fn assert_codes(stage: &str, actual: &[u8], bounds: &[[u8; 2]]) {
    assert_eq!(actual.len(), bounds.len());
    for (i, (&v, &[lo, hi])) in actual.iter().zip(bounds).enumerate() {
        assert!(
            hi - lo <= 2,
            "fixture precision budget too broad at {i}: {lo}..{hi}"
        );
        assert!(
            (lo..=hi).contains(&v),
            "{stage} ({},{}) actual{v} outside {lo}..={hi}",
            i % W as usize,
            i / W as usize
        );
    }
}
#[test]
fn filter_fixture_detects_wrong_axis_order_depth_metric_and_invalid_centers() {
    for ortho in [false, true] {
        let f = Fixture::new(ortho);
        let h = middle(&f.stage(&f.raw, 0, Metric::Plane));
        let hv = middle(&f.stage(&h, 1, Metric::Plane));
        let v = middle(&f.stage(&f.raw, 1, Metric::Plane));
        let vh = middle(&f.stage(&v, 0, Metric::Plane));
        let wrong = middle(&f.stage(&f.raw, 0, Metric::EyeZ));
        let blurred = middle(&f.stage(&f.raw, 0, Metric::Unweighted));
        assert!(
            difference(&h, &blurred) > 10,
            "slab fixture must reject non-geometric averaging"
        );
        assert!(difference(&h, &f.raw) > 20);
        assert!(difference(&hv, &h) > 20);
        assert!(
            difference(&hv, &vh) > 5,
            "fixture must reject swapped passes"
        );
        assert!(
            difference(&h, &wrong) > 20,
            "grazing plane must reject eye-z bilateral substitute"
        );
        for (x, y) in [(0, 0), (5, 5), (7, 6), (8, 9)] {
            assert_eq!(h[index(x, y)], 255);
            assert_eq!(hv[index(x, y)], 255);
        }
        let mut reversed = Fixture::new(ortho);
        for n in &mut reversed.normals {
            for v in &mut n[..3] {
                *v = half::f16::from_f32(-half::f16::from_bits(*v).to_f32()).to_bits();
            }
        }
        assert_eq!(
            f.stage(&f.raw, 0, Metric::Plane),
            reversed.stage(&f.raw, 0, Metric::Plane)
        );
        let flat = f.stage(&vec![153; (W * H) as usize], 0, Metric::Plane);
        for y in 0..H {
            for x in 0..W {
                let i = index(x, y);
                if inside(x, y) && f.depths[i] < 1. && f.normal(i).is_some() {
                    // Independent numerator/denominator intervals discard common
                    // weight correlation. The exact constant-field answer must be
                    // enclosed; the oracle need not reduce to a singleton.
                    let [lo, hi] = flat[i];
                    assert!((lo..=hi).contains(&153));
                    assert!(hi - lo <= 2);
                }
            }
        }
        // Clamped edge duplication and foreground/background averaging must be
        // observable: center-only controls above cannot conceal a no-op filter.
        assert_codes("oracle H", &h, &f.stage(&f.raw, 0, Metric::Plane));
        assert_codes("oracle V", &hv, &f.stage(&h, 1, Metric::Plane));
    }
}
const DEPTH_SOURCE: &str = r#"
@group(0) @binding(0) var source:texture_2d<f32>;
@vertex fn vertex(@builtin(vertex_index) i:u32)->@builtin(position) vec4f {
 let p=array<vec2f,3>(vec2f(-1,-1),vec2f(3,-1),vec2f(-1,3));return vec4f(p[i],0,1);
}
@fragment fn depth(@builtin(position) p:vec4f)->@builtin(frag_depth) f32 {
 return textureLoad(source,vec2i(p.xy),0).r;
}
"#;
fn texture(renderer: &crate::Renderer, format: wgpu::TextureFormat) -> wgpu::Texture {
    crate::hdr_output::texture(renderer, format, W, H)
}
fn texture_entry(binding: u32, depth: bool) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: if depth {
                wgpu::TextureSampleType::Depth
            } else {
                wgpu::TextureSampleType::Float { filterable: false }
            },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}
fn pipeline(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    source: &str,
    entry: &str,
    depth: bool,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("numeric plane filter"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(layout)],
        immediate_size: 0,
    });
    let targets = [Some(wgpu::ColorTargetState {
        format: wgpu::TextureFormat::R8Unorm,
        blend: None,
        write_mask: wgpu::ColorWrites::RED,
    })];
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(entry),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vertex"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        primitive: Default::default(),
        depth_stencil: depth.then_some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Always),
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some(entry),
            compilation_options: Default::default(),
            targets: if depth { &[] } else { &targets },
        }),
        multiview_mask: None,
        cache: None,
    })
}
fn copy(
    encoder: &mut wgpu::CommandEncoder,
    source: &wgpu::Texture,
    buffer: &wgpu::Buffer,
    offset: u64,
    depth: bool,
) {
    let mut info = source.as_image_copy();
    if depth {
        info.aspect = wgpu::TextureAspect::DepthOnly;
    }
    encoder.copy_texture_to_buffer(
        info,
        wgpu::TexelCopyBufferInfo {
            buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset,
                bytes_per_row: Some(ROW),
                rows_per_image: Some(H),
            },
        },
        source.size(),
    );
}
#[test]
#[ignore = "requires delegated GPU: exactly two 19x15 submissions, six render passes, no images"]
fn actual_plane_filter_horizontal_vertical_depth_and_r8_contract() {
    let renderer = crate::test_support::checked(crate::Renderer::headless().unwrap());
    eprintln!("SSAO_FILTER_ADAPTER {:?}", renderer.adapter.get_info());
    let device = &renderer.device;
    let depth_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[texture_entry(0, false)],
    });
    let depth_pipeline = pipeline(device, &depth_layout, DEPTH_SOURCE, "depth", true);
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(size_of::<Uniform>() as u64),
                },
                count: None,
            },
            texture_entry(1, true),
            texture_entry(2, false),
            texture_entry(3, false),
        ],
    });
    let filters = ["horizontal", "vertical"]
        .map(|s| pipeline(device, &layout, include_str!("filter.wgsl"), s, false));
    for ortho in [false, true] {
        let f = Fixture::new(ortho);
        let depth_source = texture(&renderer, wgpu::TextureFormat::R32Float);
        let depth = texture(&renderer, wgpu::TextureFormat::Depth32Float);
        let normal = texture(&renderer, wgpu::TextureFormat::Rgba16Float);
        let ao = [
            texture(&renderer, wgpu::TextureFormat::R8Unorm),
            texture(&renderer, wgpu::TextureFormat::R8Unorm),
        ];
        for (t, bytes, bpp) in [
            (&depth_source, bytemuck::cast_slice::<_, u8>(&f.depths), 4),
            (&normal, bytemuck::cast_slice(&f.normals), 8),
            (&ao[0], f.raw.as_slice(), 1),
        ] {
            renderer.queue.write_texture(
                t.as_image_copy(),
                bytes,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(W * bpp),
                    rows_per_image: Some(H),
                },
                t.size(),
            );
        }
        let u = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("736-byte actual filter ABI"),
            contents: bytemuck::bytes_of(&f.uniform),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let depth_view = depth.create_view(&Default::default());
        let normal_view = normal.create_view(&Default::default());
        let views = ao.each_ref().map(|t| t.create_view(&Default::default()));
        let groups = views.each_ref().map(|v| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: u.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&depth_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&normal_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(v),
                    },
                ],
            })
        });
        let source_view = depth_source.create_view(&Default::default());
        let depth_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &depth_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&source_view),
            }],
        });
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("H,V,depth numeric readback"),
            size: COPY_BYTES * 3,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("known depth attachment"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&depth_pipeline);
            pass.set_bind_group(0, &depth_group, &[]);
            pass.draw(0..3, 0..1);
        }
        for axis in 0..2 {
            {
                let attachment = [Some(wgpu::RenderPassColorAttachment {
                    view: &views[1 - axis],
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                })];
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("actual plane filter"),
                    color_attachments: &attachment,
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                pass.set_pipeline(&filters[axis]);
                pass.set_bind_group(0, &groups[axis], &[]);
                pass.set_viewport(RECT[0], RECT[1], RECT[2], RECT[3], 0., 1.);
                pass.draw(0..3, 0..1);
            }
            copy(
                &mut encoder,
                &ao[1 - axis],
                &readback,
                COPY_BYTES * axis as u64,
                false,
            );
        }
        copy(&mut encoder, &depth, &readback, COPY_BYTES * 2, true);
        renderer.queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(10)),
            })
            .unwrap();
        rx.recv_timeout(std::time::Duration::from_secs(10))
            .unwrap()
            .unwrap();
        let data = readback.slice(..).get_mapped_range();
        let stages: [Vec<u8>; 2] = std::array::from_fn(|stage| {
            (0..H as usize)
                .flat_map(|y| {
                    data[stage * COPY_BYTES as usize + y * ROW as usize..][..W as usize]
                        .iter()
                        .copied()
                })
                .collect()
        });
        for y in 0..H {
            for x in 0..W {
                let start = (COPY_BYTES * 2) as usize + (y * ROW + x * 4) as usize;
                let observed = f32::from_le_bytes(data[start..start + 4].try_into().unwrap());
                assert_eq!(
                    observed.to_bits(),
                    f.depths[index(x, y)].to_bits(),
                    "actual Depth32 attachment"
                );
            }
        }
        drop(data);
        readback.unmap();
        assert_codes("horizontal", &stages[0], &f.stage(&f.raw, 0, Metric::Plane));
        assert_codes(
            "vertical",
            &stages[1],
            &f.stage(&stages[0], 1, Metric::Plane),
        );
        assert!(difference(&stages[0], &f.raw) > 20);
        assert!(difference(&stages[1], &stages[0]) > 20);
        eprintln!(
            "SSAO_FILTER projection={} size={W}x{H} submits=1 render_passes=3 checked_depth={} checked_r8={} h_sha256={:x} v_sha256={:x}",
            if ortho { "orthographic" } else { "perspective" },
            W * H,
            2 * W * H,
            Sha256::digest(&stages[0]),
            Sha256::digest(&stages[1])
        );
    }
}
