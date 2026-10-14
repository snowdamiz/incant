//! Actual half-float feedback, not uploaded float32 stand-ins for prior results.
//! Eight jitter phases, six parallel lanes, one encoder/submit/map per phase:
//! ordinary, owner fallback, owner static, each with opaque/fractional coverage.
//! 48 resolves + 8 native-depth clears + 6 initial-history clears; max 32x32.
//! No images, renderer frames, retry, timing or appearance acceptance.
use super::*;

// A supported 7/16 source scale deliberately avoids power-of-two UV division.
// The 16-to-32 fixture hid the real-world roundoff and also passed old shaders.
const SOURCE: u32 = 14;
const OUTPUT: u32 = 32;
const LANES: usize = 6;
const ROW: u64 = 256;
const PLANE: u64 = ROW * OUTPUT as u64;
const PRESENT: u32 = 1 << 31;

struct Slot {
    textures: [wgpu::Texture; 4],
    views: [wgpu::TextureView; 4],
}
impl Slot {
    fn new(d: &wgpu::Device) -> Self {
        let textures = [
            crate::frame::HDR_FORMAT,
            wgpu::TextureFormat::R32Float,
            wgpu::TextureFormat::Rg32Uint,
            wgpu::TextureFormat::Rg16Float,
        ]
        .map(|format| {
            texture(
                d,
                format,
                OUTPUT,
                wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC,
            )
        });
        let views = std::array::from_fn(|i| textures[i].create_view(&Default::default()));
        Self { textures, views }
    }
    fn clear(&self, encoder: &mut wgpu::CommandEncoder) {
        let attachments = self.views.each_ref().map(attachment);
        drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Initialize feedback history without a queue submission"),
            color_attachments: &attachments,
            ..Default::default()
        }));
    }
    fn resolve(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        pipeline: &wgpu::RenderPipeline,
        bindings: &wgpu::BindGroup,
        owner: bool,
    ) {
        let attachments = [
            attachment(&self.views[0]),
            attachment(&self.views[1]),
            attachment(&self.views[if owner { 2 } else { 3 }]),
            attachment(&self.views[3]),
        ];
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Half-float opaque feedback resolve"),
            color_attachments: &attachments[..if owner { 4 } else { 3 }],
            ..Default::default()
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, bindings, &[]);
        pass.draw(0..3, 0..1);
    }
}
fn texture(
    d: &wgpu::Device,
    format: wgpu::TextureFormat,
    n: u32,
    usage: wgpu::TextureUsages,
) -> wgpu::Texture {
    d.create_texture(&wgpu::TextureDescriptor {
        label: Some("Bounded half-float feedback fixture"),
        size: wgpu::Extent3d {
            width: n,
            height: n,
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
    assert_eq!(values.len(), (SOURCE * SOURCE) as usize);
    let t = texture(
        d,
        format,
        SOURCE,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
    );
    q.write_texture(
        t.as_image_copy(),
        bytemuck::cast_slice(values),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(SOURCE * size_of::<T>() as u32),
            rows_per_image: Some(SOURCE),
        },
        wgpu::Extent3d {
            width: SOURCE,
            height: SOURCE,
            depth_or_array_layers: 1,
        },
    );
    t.create_view(&Default::default())
}
fn half(bits: u16) -> f32 {
    let sign = if bits & 0x8000 == 0 { 1. } else { -1. };
    let exponent = i32::from((bits >> 10) & 31);
    let fraction = f32::from(bits & 1023);
    match exponent {
        0 => sign * fraction * 2_f32.powi(-24),
        31 => f32::NAN,
        _ => sign * (1. + fraction / 1024.) * 2_f32.powi(exponent - 15),
    }
}
fn half_at(bytes: &[u8], lane: usize, plane: usize, p: [u32; 2], channel: usize) -> f32 {
    let bpp = if plane == 0 { 8 } else { 4 };
    let at = ((lane * 4 + plane) as u64 * PLANE + u64::from(p[1]) * ROW + u64::from(p[0]) * bpp)
        as usize
        + channel * 2;
    half(u16::from_le_bytes(bytes[at..at + 2].try_into().unwrap()))
}
fn word_at(bytes: &[u8], lane: usize, plane: usize, p: [u32; 2]) -> u32 {
    let bpp = if plane == 2 { 8 } else { 4 };
    let at = ((lane * 4 + plane) as u64 * PLANE + u64::from(p[1]) * ROW + u64::from(p[0]) * bpp)
        as usize;
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}
#[test]
fn feedback_fixture_is_opaque_or_fractional_with_bounded_stationary_motion() {
    assert_eq!(crate::frame::HDR_FORMAT, wgpu::TextureFormat::Rgba16Float);
    assert_eq!(half(0x3c00), 1.);
    assert_eq!(half(0x3bff), 1. - 1. / 2048.);
    assert_eq!(half(0x3800), 0.5);
    let samples = crate::temporal::jitter::UPSCALE_SAMPLES;
    for (phase, current) in samples.into_iter().enumerate() {
        let previous = if phase == 0 {
            current
        } else {
            samples[phase - 1]
        };
        for axis in 0..2 {
            let j = current[axis] / SOURCE as f32;
            let old = previous[axis] / SOURCE as f32;
            let motion = old - j;
            assert!(((j + motion - old) * OUTPUT as f32).abs() < 0.000001);
        }
    }
    // One bit-pattern below one is a real distinct representable value, not
    // opaque coverage. The regression must never weaken this admission boundary.
    assert!(half(0x3bff) < 1.);
}
#[test]
#[ignore = "requires bounded GPU numerical execution; no images or appearance acceptance"]
fn actual_half_float_feedback_preserves_opaque_and_fractional_coverage() {
    let mut renderer = crate::test_support::checked(Renderer::headless().unwrap());
    pollster::block_on(renderer.enable_temporal_upscaling()).unwrap();
    check_owner_format(&renderer.adapter).unwrap();
    println!("OPAQUE_FEEDBACK adapter={:?}", renderer.adapter.get_info());
    let d = &renderer.device;
    let q = &renderer.queue;
    let ordinary = renderer.upscaling.as_ref().unwrap();
    let owner = OwnerSystem::new(d, renderer.adapter.get_info().backend).unwrap();
    let lanes: [[Slot; 2]; LANES] = std::array::from_fn(|_| std::array::from_fn(|_| Slot::new(d)));
    // Exact representable premultiplied constants. No shader-duplicating oracle
    // is needed: any normalized nonnegative weighting preserves a constant field.
    let opaque = upload(
        d,
        q,
        wgpu::TextureFormat::Rgba16Float,
        &[[0x3400u16, 0x3000, 0x2c00, 0x3c00]; (SOURCE * SOURCE) as usize],
    );
    let fractional = upload(
        d,
        q,
        wgpu::TextureFormat::Rgba16Float,
        &[[0x3000u16, 0x2c00, 0x2800, 0x3800]; (SOURCE * SOURCE) as usize],
    );
    let ids = upload(
        d,
        q,
        wgpu::TextureFormat::R32Uint,
        &[5u32; (SOURCE * SOURCE) as usize],
    );
    let current_depth = texture(
        d,
        wgpu::TextureFormat::Depth32Float,
        SOURCE,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
    )
    .create_view(&Default::default());
    let read = d.create_buffer(&wgpu::BufferDescriptor {
        label: Some("All six feedback lanes in one mapped readback"),
        size: LANES as u64 * 4 * PLANE,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let samples = crate::temporal::jitter::UPSCALE_SAMPLES;
    let mut previous_mass = [[0f32; (OUTPUT * OUTPUT) as usize]; LANES];
    let (mut submits, mut maps, mut resolves) = (0, 0, 0);
    for (phase, jitter) in samples.into_iter().enumerate() {
        let previous_jitter = if phase == 0 {
            jitter
        } else {
            samples[phase - 1]
        };
        let j = jitter.map(|v| v / SOURCE as f32);
        let pj = previous_jitter.map(|v| v / SOURCE as f32);
        // Analytic stationary motion includes the difference of raster jitters.
        let motion = upload(
            d,
            q,
            wgpu::TextureFormat::Rgba32Float,
            &[[pj[0] - j[0], pj[1] - j[1], 0.5, 1.]; (SOURCE * SOURCE) as usize],
        );
        let mut e = d.create_command_encoder(&Default::default());
        drop(e.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Real Depth32 feedback fixture"),
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &current_depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(0.5),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        }));
        for (lane, slots) in lanes.iter().enumerate() {
            let destination = &slots[phase % 2];
            let previous = &slots[1 - phase % 2];
            if phase == 0 {
                previous.clear(&mut e);
            }
            let with_owner = lane >= 2;
            let still = lane >= 4;
            let current = if lane % 2 == 0 { &opaque } else { &fractional };
            let frame = Frame {
                source: [0., 0., SOURCE as f32, SOURCE as f32],
                output: [0., 0., OUTPUT as f32, OUTPUT as f32],
                depth: [0.1, 100., 0., f32::from(u8::from(phase != 0))],
                options: [0.9, 0.001, 0.001, 1.],
                reactive: [0., 1., 1., f32::from(u8::from(phase != 0))],
                jitter: [j[0], j[1], pj[0], pj[1]],
            };
            let uniform = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Feedback fixture frame"),
                contents: bytemuck::bytes_of(&frame),
                usage: wgpu::BufferUsages::UNIFORM,
            });
            let mut changed = [0u32; 3132];
            changed[1] = u32::from(still);
            changed[2] = 5;
            let changes = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Unchanged owner; explicit static or fallback branch"),
                contents: bytemuck::cast_slice(&changed),
                usage: wgpu::BufferUsages::UNIFORM,
            });
            let textures = [
                current,
                &motion,
                &current_depth,
                &previous.views[0],
                &previous.views[1],
                &previous.views[3],
                &ids,
                &previous.views[2],
            ];
            let (layout, pipeline) = if with_owner {
                (&owner.layout, &owner.resolve)
            } else {
                (&ordinary.resolve_layout, &ordinary.resolve)
            };
            let bindings = group(
                d,
                layout,
                &uniform,
                &textures[..if with_owner { 8 } else { 6 }],
                with_owner.then_some(&changes),
            );
            destination.resolve(&mut e, pipeline, &bindings, with_owner);
            resolves += 1;
            for (plane, t) in destination.textures.iter().enumerate() {
                if plane == 2 && !with_owner {
                    continue;
                }
                e.copy_texture_to_buffer(
                    t.as_image_copy(),
                    wgpu::TexelCopyBufferInfo {
                        buffer: &read,
                        layout: wgpu::TexelCopyBufferLayout {
                            offset: (lane * 4 + plane) as u64 * PLANE,
                            bytes_per_row: Some(ROW as u32),
                            rows_per_image: Some(OUTPUT),
                        },
                    },
                    wgpu::Extent3d {
                        width: OUTPUT,
                        height: OUTPUT,
                        depth_or_array_layers: 1,
                    },
                );
            }
        }
        q.submit([e.finish()]);
        submits += 1;
        let (tx, rx) = std::sync::mpsc::channel();
        read.slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = tx.send(result);
            });
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
        for (lane, old_masses) in previous_mass.iter_mut().enumerate() {
            for y in 6..26 {
                for x in 6..26 {
                    let p = [x, y];
                    let alpha = half_at(&bytes, lane, 0, p, 3);
                    let mass = half_at(&bytes, lane, 3, p, 0);
                    let age = half_at(&bytes, lane, 3, p, 1);
                    let z = f32::from_bits(word_at(&bytes, lane, 1, p));
                    let at = (y * OUTPUT + x) as usize;
                    let expected_alpha = if lane % 2 == 0 { 1. } else { 0.5 };
                    assert!(
                        alpha.is_finite() && (alpha - expected_alpha).abs() <= 0.001,
                        "lane {lane} phase {phase} {p:?}: alpha={alpha}"
                    );
                    for (channel, value) in [0.25, 0.125, 0.0625].into_iter().enumerate() {
                        let actual = half_at(&bytes, lane, 0, p, channel);
                        assert!(
                            (actual - value * expected_alpha).abs() <= 0.001,
                            "lane {lane} phase {phase} {p:?} channel {channel}: {actual}"
                        );
                    }
                    assert_eq!(age, 0.);
                    if lane % 2 == 0 {
                        assert_eq!(
                            alpha, 1.,
                            "opaque feedback lost canonical alpha: lane {lane} phase {phase} {p:?}"
                        );
                        assert!(
                            mass.is_finite() && mass > old_masses[at] && mass < 36.,
                            "opaque history did not grow below cap: lane {lane} phase {phase} {p:?}: {} -> {mass}",
                            old_masses[at]
                        );
                        assert!((z - 1. / (0.5 / 0.1 + 0.5 / 100.)).abs() < 1e-6);
                        if lane >= 2 {
                            assert_eq!(word_at(&bytes, lane, 2, p), PRESENT | 5);
                        }
                    } else {
                        assert!(alpha < 1., "fractional coverage was made opaque");
                        // Static owner records explicitly permit fractional persistence;
                        // ordinary/owner fallback must not promote it to single-surface history.
                        if lane < 4 || phase == 0 {
                            assert_eq!((mass, z), (0., 0.));
                            if lane >= 2 {
                                assert_eq!(word_at(&bytes, lane, 2, p), 0);
                            }
                        } else {
                            assert!(mass.is_finite() && mass > old_masses[at] && mass < 36.);
                            assert_eq!(word_at(&bytes, lane, 2, p), PRESENT | 5);
                        }
                    }
                    old_masses[at] = mass;
                }
            }
        }
        drop(bytes);
        read.unmap();
    }
    assert_eq!((submits, maps, resolves), (8, 8, 48));
    println!(
        "OPAQUE_FEEDBACK_COMPLETE phases=8 lanes=6 submissions=8 maps=8 resolves=48 max_extent=32"
    );
}
