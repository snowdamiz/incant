//! R4 owner-history controls with independent f64 support/evidence expectations. Delegated numeric execution only.
//! Exactly 12 encoders/submissions/maps: each includes one real Depth32 fixture,
//! one owner resolve, a current-only reference in case 3, ordinary R4
//! baseline resolves in cases 7/8, and all readbacks.
//! No renderer frame, image, extra readback submission, retry, or appearance gate.
use super::*;

const N: u32 = 16;
const ROW: u64 = 256;
const PLANE: u64 = ROW * N as u64;
const PRESENT: u32 = 1 << 31;
const CENTER: [u32; 2] = [8, 8];
const _: () = assert!(N <= 32);
fn at(p: [u32; 2]) -> usize {
    (p[1] * N + p[0]) as usize
}
fn rgba(v: f32) -> [f32; 4] {
    [v, v, v, 1.]
}
fn classes(ids: &[u32]) -> u32 {
    ids.iter().fold(0, |mask, id| mask | (1 << (id % 31)))
}
fn canonical(record: [u32; 2]) -> bool {
    let [x, y] = record;
    if x == 0 {
        return y == 0;
    }
    let anchor = x & 0x1ffff;
    if x & PRESENT == 0 || x & !(PRESENT | 0x1ffff) != 0 || !(1..=100_000).contains(&anchor) {
        return false;
    }
    if y & PRESENT != 0 {
        y & (1 << (anchor % 31)) != 0
    } else {
        y == 0 || ((1..=100_000).contains(&y) && y != anchor)
    }
}
fn half(bits: u16) -> f32 {
    let sign = if bits & 0x8000 != 0 { -1. } else { 1. };
    let exp = ((bits >> 10) & 31) as i32;
    let mantissa = f32::from(bits & 1023);
    match exp {
        0 => sign * mantissa * 2_f32.powi(-24),
        31 => f32::NAN, // All fixture outputs must be finite, including unused texels.
        _ => sign * (1. + mantissa / 1024.) * 2_f32.powi(exp - 15),
    }
}
#[derive(Debug)]
struct Probe {
    pixel: [u32; 2],
    expected: evidence_reference::Result,
}
struct Case {
    frame: Frame,
    current: Vec<[f32; 4]>,
    motion: Vec<[f32; 4]>,
    old: Vec<[f32; 4]>,
    old_depth: Vec<f32>,
    old_evidence: Vec<[f32; 4]>,
    owners: Vec<u32>,
    records: Vec<[u32; 2]>,
    changes: [u32; 3132],
    depth: f32,
    probes: Vec<Probe>,
    baseline: bool,
    current_only_reference: bool,
}
impl Case {
    fn reference(&self) -> evidence_reference::Input<'_> {
        evidence_reference::Input {
            frame: &self.frame,
            source_size: N as usize,
            output_size: N as usize,
            current: &self.current,
            motion: &self.motion,
            native_depth: self.depth,
            old: &self.old,
            old_depth: &self.old_depth,
            old_evidence: &self.old_evidence,
            owner: Some(evidence_reference::Owners {
                ids: &self.owners,
                records: &self.records,
                changes: &self.changes,
            }),
        }
    }
}
fn fixture(case: u32) -> Case {
    assert!((1..=12).contains(&case));
    let orthographic = case == 3 || case == 5;
    let z = if orthographic {
        50.05
    } else {
        1. / (0.5 / 0.1 + 0.5 / 100.)
    };
    let mut c = Case {
        frame: Frame {
            source: [1., 1., 14., 14.],
            output: [1., 1., 14., 14.],
            depth: [0.1, 100., f32::from(u8::from(orthographic)), 1.],
            options: [0.9, 0.001, 0.001, 1.],
            // Same area, accepted evidence origin. Mass is per texel, not a
            // prescribed fixed blend fraction.
            reactive: [0., 9., 1., 1.],
            jitter: [0.; 4],
        },
        current: vec![rgba(0.25); (N * N) as usize],
        motion: vec![[0., 0., 0.5, 1.]; (N * N) as usize],
        old: vec![rgba(0.75); (N * N) as usize],
        old_depth: vec![z; (N * N) as usize],
        old_evidence: vec![[9., 0., 0., 0.]; (N * N) as usize],
        owners: vec![5; (N * N) as usize],
        records: vec![[PRESENT | 5, 0]; (N * N) as usize],
        changes: [0; 3132],
        depth: 0.5,
        probes: vec![],
        baseline: case == 7 || case == 8,
        current_only_reference: case == 3,
    };
    c.changes[1] = 1;
    c.changes[2] = 100_000;
    let mut pixels = vec![CENTER];
    match case {
        1 => c.frame.depth[3] = 0.,
        2 => c.records.fill([PRESENT | 7, 5]), // Carried primary 7, absent age advances.
        3 => {
            c.current.fill(rgba(0.));
            c.old.fill(rgba(0.45));
            c.records.fill([PRESENT | 5, 7]);
            // The archived bilinear exact-zero fixture is preserved in fd5. In
            // R4 this Gaussian tap is positive. Compare tight fresh reconstruction
            // and the entire history-disabled reference; stale .45 is forbidden.
            c.owners[at([9, 8])] = 7;
            c.current[at([9, 8])] = rgba(0.5);
            c.changes[0] = classes(&[7]);
            c.changes[4] = 1 << 7;
            pixels.push([3, 3]);
        }
        4 | 5 => {
            c.records.fill([PRESENT | 5, PRESENT | classes(&[5, 7, 9])]);
            if case == 4 {
                c.changes[0] = classes(&[7]);
                c.changes[4] = 1 << 7;
            }
        }
        6 => {
            c.changes[1] = 0;
            c.records.fill([PRESENT | 9, 0]);
            c.old_depth.fill(z + 0.00025);
        }
        7 | 8 => {
            c.changes[1] = if case == 7 { 3 } else { 0 };
            for y in 0..N {
                for x in 0..N {
                    c.current[at([x, y])] = rgba(if (x + y) % 2 == 0 { 0.25 } else { 64. });
                    c.old[at([x, y])] = rgba(if (x + y) % 2 == 0 { 0.25 } else { 32. });
                }
            }
            let delta = if case == 7 {
                [3. / 1024., 0.]
            } else {
                [0.25, 0.5]
            };
            c.motion.fill([delta[0] / 14., delta[1] / 14., 0.5, 1.]);
        }
        9 => {
            c.changes[1] = 0;
            c.current.fill(rgba(0.));
            c.old.fill(rgba(1.));
            c.current[at([7, 8])] = rgba(0.5);
            c.owners[at([7, 8])] = 7;
        }
        10 => {
            c.changes[1] = 0;
            c.changes[0] = classes(&[7]);
            c.changes[4] = 1 << 7;
            c.motion.fill([0.25 / 14., 0.5 / 14., 0.5, 1.]);
            c.old.fill(rgba(0.5));
            c.old[at([9, 8])] = rgba(0.9);
            c.records[at([9, 8])] = [PRESENT | 5, 7];
            c.records[at([8, 9])] = [PRESENT | 5, 9];
            c.current[at([7, 8])] = rgba(0.);
            c.current[at([8, 7])] = rgba(1.);
        }
        11 => {
            for (p, word) in [([3, 3], 0), ([11, 3], 100_001), ([6, 3], 5 | (1 << 20))] {
                c.owners[at(p)] = word;
                pixels.push(p);
            }
            c.current[at([3, 11])] = [0.125, 0.125, 0.125, 0.5];
            pixels.push([3, 11]);
            c.records[at([11, 11])] = [PRESENT, 7];
            pixels.push([11, 11]);
            c.records[at([8, 11])] = [PRESENT | 5 | (1 << 20), 0];
            pixels.push([8, 11]);
            // Three existing unused tiles add retirement, carried identity and
            // malformed-age observation without another pass, submit or map.
            for (p, age) in [([3, 7], 7.), ([7, 7], 6.), ([11, 7], 1.5)] {
                c.records[at(p)] = [PRESENT | 7, 5];
                c.old_evidence[at(p)][1] = age;
                pixels.push(p);
            }
        }
        12 => {
            c.depth = 1.;
            c.current.fill([0.; 4]);
        }
        _ => unreachable!(),
    }
    c.probes = pixels
        .into_iter()
        .map(|pixel| Probe {
            pixel,
            expected: c.reference().resolve(pixel),
        })
        .collect();
    c
}

fn texture(
    d: &wgpu::Device,
    format: wgpu::TextureFormat,
    usage: wgpu::TextureUsages,
) -> wgpu::Texture {
    d.create_texture(&wgpu::TextureDescriptor {
        label: Some("Owner history numeric fixture"),
        size: wgpu::Extent3d {
            width: N,
            height: N,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage,
        view_formats: &[],
    })
}
fn upload<T: bytemuck::Pod>(
    d: &wgpu::Device,
    q: &wgpu::Queue,
    format: wgpu::TextureFormat,
    values: &[T],
) -> wgpu::TextureView {
    assert_eq!(values.len(), (N * N) as usize);
    let t = texture(
        d,
        format,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
    );
    q.write_texture(
        t.as_image_copy(),
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
    t.create_view(&Default::default())
}
fn draw(
    e: &mut wgpu::CommandEncoder,
    pipeline: &wgpu::RenderPipeline,
    bindings: &wgpu::BindGroup,
    color: &wgpu::TextureView,
    depth: &wgpu::TextureView,
    record: Option<&wgpu::TextureView>,
    evidence: &wgpu::TextureView,
) {
    let mut attachments = vec![attachment(color), attachment(depth)];
    if let Some(record) = record {
        attachments.push(attachment(record));
    }
    attachments.push(attachment(evidence));
    let mut pass = e.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Actual owner history numerical resolve"),
        color_attachments: &attachments,
        ..Default::default()
    });
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, bindings, &[]);
    pass.draw(0..3, 0..1);
}
fn read_pixel(bytes: &[u8], p: [u32; 2]) -> ([f32; 4], f32, [u32; 2], [f32; 2]) {
    let offset = (p[1] as u64 * ROW + p[0] as u64 * 8) as usize;
    let color = std::array::from_fn(|c| {
        half(u16::from_le_bytes(
            bytes[offset + 2 * c..offset + 2 * c + 2]
                .try_into()
                .unwrap(),
        ))
    });
    let dz = (PLANE + p[1] as u64 * ROW + p[0] as u64 * 4) as usize;
    let depth = f32::from_le_bytes(bytes[dz..dz + 4].try_into().unwrap());
    let record = std::array::from_fn(|c| {
        let at = 2 * PLANE as usize + offset + 4 * c;
        u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
    });
    let evidence = std::array::from_fn(|c| {
        let at = (3 * PLANE + p[1] as u64 * ROW + p[0] as u64 * 4) as usize + 2 * c;
        half(u16::from_le_bytes(bytes[at..at + 2].try_into().unwrap()))
    });
    (color, depth, record, evidence)
}
#[test]
fn owner_history_fixture_has_exact_budget_and_independent_counterexamples() {
    let cases: Vec<_> = (1..=12).map(fixture).collect();
    assert_eq!(cases.iter().filter(|c| c.baseline).count(), 2);
    assert_eq!(cases.iter().filter(|c| c.current_only_reference).count(), 1);
    assert_eq!(size_of::<[u32; 3132]>(), 12_528);
    for bad in [
        [PRESENT, 0],
        [0, 5],
        [PRESENT | 5, 5],
        [PRESENT | 5 | (1 << 20), 0],
        [PRESENT | 5, PRESENT | classes(&[7])],
    ] {
        assert!(!canonical(bad));
    }
    for good in [
        [0, 0],
        [PRESENT | 5, 0],
        [PRESENT | 5, 7],
        [PRESENT | 5, PRESENT | classes(&[5, 7, 9])],
    ] {
        assert!(canonical(good));
    }
    assert_eq!(cases[2].owners[at([9, 8])], 7);
    assert_eq!(cases[2].probes[1].expected.color, [0., 0., 0., 1.]);
    assert_eq!(cases[2].probes[1].pixel, [3, 3]);
    assert!((cases[6].motion[0][0] * 14.) < 1. / 256.);
    assert!(cases[6].motion[0][0] > 0.);
    assert_eq!(cases[8].probes[0].expected.record, [PRESENT | 5, 7]);
    let renormalized = (0.375 * 0.5 + 0.375 * 0.5 + 0.125 * 0.5) / 0.875;
    assert_eq!(renormalized, 0.5);
    assert!((0.375 * 0.5 + 0.125 * 0.9 + 0.375 * 0.5 + 0.125 * 0.5) > renormalized);
    assert_eq!(cases[9].probes[0].expected.record, [PRESENT | 5, 9]);
    assert_eq!(cases[10].probes.len(), 10);
}
#[test]
#[ignore = "requires delegated GPU numerical execution; no images or appearance acceptance"]
fn actual_owner_history_obeys_provenance_and_preserves_ordinary_fallback() {
    let mut renderer = crate::test_support::checked(Renderer::headless().unwrap());
    // Unsupported features are a failed numerical gate, not an early successful test.
    pollster::block_on(renderer.enable_temporal_upscaling()).unwrap();
    check_owner_format(&renderer.adapter).unwrap();
    println!("OWNER_HISTORY adapter={:?}", renderer.adapter.get_info());
    let d = &renderer.device;
    let q = &renderer.queue;
    let original = renderer.upscaling.as_ref().unwrap();
    let owner = OwnerSystem::new(d, renderer.adapter.get_info().backend).unwrap();
    let usage = wgpu::TextureUsages::RENDER_ATTACHMENT
        | wgpu::TextureUsages::TEXTURE_BINDING
        | wgpu::TextureUsages::COPY_SRC;
    let color = texture(d, crate::frame::HDR_FORMAT, usage);
    let depth = texture(d, wgpu::TextureFormat::R32Float, usage);
    let record = texture(d, wgpu::TextureFormat::Rg32Uint, usage);
    // Both resolve programs write RG16Float (mass, age) last.
    let evidence_texture = texture(d, wgpu::TextureFormat::Rg16Float, usage);
    let evidence = evidence_texture.create_view(&Default::default());
    let baseline_evidence_texture = texture(d, wgpu::TextureFormat::Rg16Float, usage);
    let baseline_evidence = baseline_evidence_texture.create_view(&Default::default());
    let baseline_color = texture(d, crate::frame::HDR_FORMAT, usage);
    let baseline_depth = texture(d, wgpu::TextureFormat::R32Float, usage);
    let baseline_record = texture(d, wgpu::TextureFormat::Rg32Uint, usage);
    let current_depth =
        texture(d, wgpu::TextureFormat::Depth32Float, usage).create_view(&Default::default());
    let color_view = color.create_view(&Default::default());
    let depth_view = depth.create_view(&Default::default());
    let record_view = record.create_view(&Default::default());
    let baseline_color_view = baseline_color.create_view(&Default::default());
    let baseline_depth_view = baseline_depth.create_view(&Default::default());
    let baseline_record_view = baseline_record.create_view(&Default::default());
    let read = d.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Batched owner history scalar readback"),
        size: PLANE * 8,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let (mut submissions, mut maps, mut owner_renders, mut baseline_renders) = (0, 0, 0, 0);
    let mut current_only_renders = 0;
    for case in 1..=12 {
        let c = fixture(case);
        let current = upload(d, q, wgpu::TextureFormat::Rgba32Float, &c.current);
        let motion = upload(d, q, wgpu::TextureFormat::Rgba32Float, &c.motion);
        let previous = upload(d, q, wgpu::TextureFormat::Rgba32Float, &c.old);
        let previous_depth = upload(d, q, wgpu::TextureFormat::R32Float, &c.old_depth);
        // Previous mass 9; case 11 additionally exercises valid/retired/malformed ages.
        let previous_evidence = upload(d, q, wgpu::TextureFormat::Rgba32Float, &c.old_evidence);
        let owners = upload(d, q, wgpu::TextureFormat::R32Uint, &c.owners);
        let records = upload(d, q, wgpu::TextureFormat::Rg32Uint, &c.records);
        let frame = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::bytes_of(&c.frame),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let changes = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&c.changes),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let textures = [
            &current,
            &motion,
            &current_depth,
            &previous,
            &previous_depth,
            &previous_evidence,
        ];
        let mut owner_textures = textures.to_vec();
        owner_textures.extend([&owners, &records]);
        let bindings = group(d, &owner.layout, &frame, &owner_textures, Some(&changes));
        let baseline_bindings = c
            .baseline
            .then(|| group(d, &original.resolve_layout, &frame, &textures, None));
        let current_only_bindings = c.current_only_reference.then(|| {
            let mut fresh = c.frame;
            fresh.depth[3] = 0.;
            let frame = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("History-disabled owner reference"),
                contents: bytemuck::bytes_of(&fresh),
                usage: wgpu::BufferUsages::UNIFORM,
            });
            group(d, &owner.layout, &frame, &owner_textures, Some(&changes))
        });
        let mut e = d.create_command_encoder(&Default::default());
        // This is an actual native depth attachment, not a float texture bound as depth.
        drop(e.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Owner fixture native depth"),
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &current_depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(c.depth),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        }));
        draw(
            &mut e,
            &owner.resolve,
            &bindings,
            &color_view,
            &depth_view,
            Some(&record_view),
            &evidence,
        );
        owner_renders += 1;
        if let Some(fresh) = &current_only_bindings {
            draw(
                &mut e,
                &owner.resolve,
                fresh,
                &baseline_color_view,
                &baseline_depth_view,
                Some(&baseline_record_view),
                &baseline_evidence,
            );
            current_only_renders += 1;
        }
        if let Some(baseline) = &baseline_bindings {
            draw(
                &mut e,
                &original.resolve,
                baseline,
                &baseline_color_view,
                &baseline_depth_view,
                None,
                &baseline_evidence,
            );
            baseline_renders += 1;
        }
        let mut copies = vec![
            (0, &color),
            (1, &depth),
            (2, &record),
            (3, &evidence_texture),
        ];
        if c.baseline || c.current_only_reference {
            copies.extend([
                (4, &baseline_color),
                (5, &baseline_depth),
                (7, &baseline_evidence_texture),
            ]);
        }
        if c.current_only_reference {
            copies.push((6, &baseline_record));
        }
        for (i, texture) in copies {
            e.copy_texture_to_buffer(
                texture.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &read,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: i as u64 * PLANE,
                        bytes_per_row: Some(ROW as u32),
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
        q.submit([e.finish()]);
        submissions += 1;
        let (tx, rx) = std::sync::mpsc::channel();
        read.slice(..)
            .map_async(wgpu::MapMode::Read, move |result| tx.send(result).unwrap());
        maps += 1;
        d.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(10)),
        })
        .unwrap();
        rx.recv_timeout(std::time::Duration::from_secs(10))
            .unwrap()
            .unwrap();
        let bytes = read.slice(..).get_mapped_range();
        for y in 0..N {
            for x in 0..N {
                let (color, depth, record, evidence) = read_pixel(&bytes, [x, y]);
                assert!(
                    color.iter().all(|v| v.is_finite() && *v >= 0.) && color[3] <= 1.,
                    "case {case} ({x},{y}) color {color:?}"
                );
                assert!(evidence[0].is_finite() && (0. ..=65504.).contains(&evidence[0]));
                assert!(
                    evidence[1].is_finite()
                        && (0. ..8.).contains(&evidence[1])
                        && evidence[1] == evidence[1].floor()
                );
                assert!(depth.is_finite() && (0. ..=100.).contains(&depth));
                assert!(
                    canonical(record),
                    "case {case} ({x},{y}) noncanonical {record:?}"
                );
                if x == 0 || y == 0 || x == N - 1 || y == N - 1 || case == 12 {
                    assert_eq!(
                        (color, depth, record, evidence),
                        ([0.; 4], 0., [0; 2], [0.; 2]),
                        "case {case} clear/outside ({x},{y})"
                    );
                }
            }
        }
        for probe in &c.probes {
            let (color, depth, record, evidence) = read_pixel(&bytes, probe.pixel);
            for (actual, expected) in color.into_iter().zip(probe.expected.color) {
                assert!(
                    (f64::from(actual) - expected).abs()
                        <= evidence_reference::half_bound(expected),
                    "case {case} probe {probe:?}: {color:?}"
                );
            }
            assert!(
                (f64::from(depth) - probe.expected.depth).abs()
                    <= probe.expected.depth.abs() * 2_f64.powi(-20),
                "case {case} probe {probe:?}: {depth}"
            );
            assert_eq!(record, probe.expected.record, "case {case} probe {probe:?}");
            for (actual, expected) in evidence.into_iter().zip(probe.expected.evidence) {
                assert!(
                    (f64::from(actual) - expected).abs()
                        <= evidence_reference::half_bound(expected),
                    "case {case} evidence {evidence:?} expected {:?}",
                    probe.expected.evidence
                );
            }
        }
        if c.current_only_reference {
            let (color, _, record, _) = read_pixel(&bytes, CENTER);
            // Independent Gaussian support gives ~0.0079; the archived bilinear
            // 2^-20 bound does not describe R4. The .405 stale contribution still
            // fails both this bound and the exact history-disabled comparison.
            let fresh = 0.5 * (-1_f64 / (2. * 0.35_f64.powi(2))).exp()
                / (1. + 2. * (-1_f64 / (2. * 0.35_f64.powi(2))).exp()).powi(2);
            assert!(color[..3].iter().all(|&v| (f64::from(v)-fresh).abs() <= evidence_reference::half_bound(fresh)));
            assert_eq!(color[3], 1.);
            assert_eq!(record, [PRESENT | 5, 7]);
        }
        if c.baseline || c.current_only_reference {
            for (plane, bpp) in [(0, 8), (1, 4), (2, 8), (3, 4)] {
                if plane == 2 && !c.current_only_reference {
                    continue;
                }
                for y in 0..N as usize {
                    let begin = plane * PLANE as usize + y * ROW as usize;
                    let original = (plane + 4) * PLANE as usize + y * ROW as usize;
                    assert_eq!(
                        &bytes[begin..begin + N as usize * bpp],
                        &bytes[original..original + N as usize * bpp],
                        "case {case} plane {plane} row {y} differs from reference"
                    );
                }
            }
        }
        println!(
            "OWNER_HISTORY case={case} center={:?} probes={} baseline={} current_only={}",
            read_pixel(&bytes, CENTER),
            c.probes.len(),
            c.baseline,
            c.current_only_reference
        );
        drop(bytes);
        read.unmap();
    }
    assert_eq!(
        (owner_renders, baseline_renders, submissions, maps),
        (12, 2, 12, 12)
    );
    assert_eq!(current_only_renders, 1);
    println!(
        "OWNER_HISTORY cases=12 owner_renders=12 current_only_renders=1 baseline_renders=2 queue_submits=12 readback_maps=12 max_extent=16x16 no_images=true"
    );
}
