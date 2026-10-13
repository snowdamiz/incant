//! Real retained history lifecycle. Twelve frames and twelve packed readbacks;
//! delegated GPU execution only. No screenshots or appearance acceptance.
use super::*;
use glam::{DMat4, DVec3};
use incant_render_data::{SceneLight, SceneNode};

const SIZE: u32 = 32;
const STRIDE: u32 = 256;
const PLANE: u64 = STRIDE as u64 * SIZE as u64;

fn surfaces(scene: &RetainedScene) -> [wgpu::TextureView; 3] {
    scene
        .upscaling
        .as_ref()
        .unwrap()
        .accepted_owner_surfaces()
        .unwrap()
}

// Read only after a successful submit. This is the prepared CPU upload, not a
// public accepted-state view; a refused frame may legitimately alter scratch.
fn change_header(scene: &RetainedScene) -> [u32; 4] {
    scene
        .motion
        .as_ref()
        .unwrap()
        .owner_test_change_header()
        .unwrap()
}

fn read(
    renderer: &Renderer,
    views: &[wgpu::TextureView; 3],
    highest: u32,
    interior_coverage: bool,
) -> Vec<u8> {
    let buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Retained owner history numeric readback"),
        size: PLANE * 3,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = renderer.device.create_command_encoder(&Default::default());
    for (i, view) in views.iter().enumerate() {
        let t = view.texture();
        assert_eq!([t.width(), t.height()], [SIZE, SIZE]);
        encoder.copy_texture_to_buffer(
            t.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: i as u64 * PLANE,
                    bytes_per_row: Some(STRIDE),
                    rows_per_image: Some(SIZE),
                },
            },
            t.size(),
        );
    }
    renderer.queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |v| tx.send(v).unwrap());
    renderer
        .device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(10)),
        })
        .unwrap();
    rx.recv_timeout(std::time::Duration::from_secs(10))
        .unwrap()
        .unwrap();
    let bytes = buffer.slice(..).get_mapped_range().to_vec();
    buffer.unmap();
    let mut present = 0;
    for y in 0..SIZE as usize {
        for x in 0..SIZE as usize {
            let offset = y * STRIDE as usize + x * 8;
            for word in bytes[offset..offset + 8].as_chunks::<2>().0 {
                // RGBA16Float: reject NaN/Inf and negative color numerically.
                let bits = u16::from_le_bytes(*word);
                assert_ne!(bits & 0x7c00, 0x7c00);
                assert!(bits & 0x8000 == 0 || bits & 0x7fff == 0);
            }
            let depth_offset = PLANE as usize + y * STRIDE as usize + x * 4;
            let depth =
                f32::from_le_bytes(bytes[depth_offset..depth_offset + 4].try_into().unwrap());
            assert!(depth.is_finite() && (0. ..=50.).contains(&depth));
            let offset = 2 * PLANE as usize + offset;
            let a = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
            let b = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap());
            if a == 0 {
                assert_eq!(b, 0);
            } else {
                let anchor = a & 0x1ffff;
                assert_eq!(a & !0x1ffff, 1 << 31);
                assert!((1..=highest).contains(&anchor));
                if b & (1 << 31) != 0 {
                    assert_ne!(b & (1 << (anchor % 31)), 0);
                } else {
                    assert!(b == 0 || ((1..=highest).contains(&b) && b != anchor));
                }
                assert!(depth > 0.);
                present += 1;
            }
        }
    }
    // At quarter resolution these quads are only 1.2 source pixels wide, so
    // rejecting every mixed-clear footprint is valid. At half resolution and
    // above the opaque 2.4-pixel footprint must retain interior records.
    if interior_coverage {
        assert!(present > 0, "opaque interior must produce reusable records");
    }
    bytes
}

#[test]
#[ignore = "requires delegated GPU; 12 renders +12 packed readbacks, no images"]
fn retained_owner_color_depth_records_commit_and_reset_together() {
    let mut renderer = crate::test_support::checked(Renderer::headless().unwrap());
    println!("RETAINED_OWNER adapter={:?}", renderer.adapter.get_info());
    pollster::block_on(renderer.enable_temporal_upscaling()).unwrap();
    pollster::block_on(renderer.enable_motion_owner()).unwrap();
    let mut input = motion_owner_tests::fixture();
    input.nodes[0].mesh.as_mut().unwrap().cast_shadows = true;
    let light_index = input.nodes.len();
    let camera_index = light_index - 1;
    input.nodes.push(SceneNode {
        id: "light".into(),
        world: DMat4::IDENTITY,
        camera: None,
        light: Some(SceneLight::Directional(incant_types::DirectionalLight {
            color: [1.; 3],
            intensity: 1.,
            shadows: None,
        })),
        mesh: None,
        sprite: None,
        tilemap: None,
        diagnostic: false,
    });
    let config = crate::temporal::TemporalConfig {
        history_weight: 0.5,
        relative_depth: 0.01,
        absolute_depth: 0.001,
        reactive_threshold: 1.,
        reactive_strength: 1.,
    };
    let mut scene = renderer.prepare_retained(&input, "camera").unwrap();
    scene.enable_motion_history(&renderer).unwrap();
    scene.set_motion_owner_attachment(&renderer, true).unwrap();
    scene.enable_temporal_upscaling(&renderer, config).unwrap();
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let texture = crate::hdr_output::texture(&renderer, format, SIZE, SIZE);
    let view = texture.create_view(&Default::default());
    let target = |width| RetainedTarget {
        view: &view,
        format,
        width,
        height: SIZE,
        viewport: None,
    };
    let mut renders = 0;
    let mut reads = 0;
    let submit = |scene: &mut RetainedScene, input: &SceneInput, renders: &mut usize| {
        scene.submit(&renderer, input, target(SIZE)).unwrap();
        *renders += 1;
    };
    let capture = |scene: &RetainedScene, reads: &mut usize| {
        *reads += 1;
        read(
            &renderer,
            &surfaces(scene),
            scene.motion_changes().unwrap().highest_owner(),
            scene.motion_frame().unwrap().internal_size()[0] >= 16,
        )
    };
    submit(&mut scene, &input, &mut renders);
    let first = scene.upscale_frame().unwrap();
    assert!(!first.motion().has_previous());
    assert!(!first.motion().camera_static());
    assert_eq!(change_header(&scene)[1], 2);
    let accepted = surfaces(&scene);
    let initial = capture(&scene, &mut reads);
    let old_instances = scene
        .live
        .batches
        .iter()
        .map(|b| b.instances.clone())
        .collect::<Vec<_>>();
    let old_count = scene.motion_changes().unwrap().changed_count();
    input.nodes[0].world = DMat4::from_translation(DVec3::new(-1.75, 2., 0.));
    input.nodes[light_index].world = DMat4::from_rotation_y(1.);
    assert!(scene.submit(&renderer, &input, target(0)).is_err());
    assert!(
        scene
            .submit(
                &renderer,
                &input,
                RetainedTarget {
                    format: wgpu::TextureFormat::R32Float,
                    ..target(SIZE)
                }
            )
            .is_err()
    );
    assert_eq!(
        scene.upscale_frame().unwrap().motion().sequence(),
        first.motion().sequence()
    );
    assert_eq!(surfaces(&scene), accepted);
    assert_eq!(capture(&scene, &mut reads), initial);
    assert_eq!(scene.motion_changes().unwrap().changed_count(), old_count);
    assert_eq!(
        scene
            .live
            .batches
            .iter()
            .map(|b| b.instances.clone())
            .collect::<Vec<_>>(),
        old_instances
    );
    input.nodes[light_index].world = DMat4::IDENTITY;
    submit(&mut scene, &input, &mut renders);
    assert_eq!(scene.motion_changes().unwrap().changed_count(), 1);
    assert!(scene.motion_frame().unwrap().camera_static());
    assert_eq!(
        change_header(&scene)[1],
        3,
        "moved caster invalidates static persistence"
    );
    let moved = surfaces(&scene);
    assert!(moved.iter().zip(&accepted).all(|(a, b)| a != b));
    capture(&scene, &mut reads);
    submit(&mut scene, &input, &mut renders);
    assert_eq!(scene.motion_changes().unwrap().changed_count(), 0);
    assert_eq!(change_header(&scene)[..2], [0, 1]);
    assert_eq!(surfaces(&scene), accepted);
    let stable = capture(&scene, &mut reads);
    // Authored light configuration is an immutable retained binding. Its native
    // world transform is mutable and changes the packed directional light.
    input.nodes[light_index].world = DMat4::from_rotation_y(1.);
    submit(&mut scene, &input, &mut renders);
    assert!(scene.motion_frame().unwrap().camera_static());
    assert_eq!(scene.motion_changes().unwrap().changed_count(), 0);
    assert_eq!(
        change_header(&scene)[..2],
        [0, 3],
        "light change reaches the actual upload"
    );
    let lit = capture(&scene, &mut reads);
    // Interior of the fixed opaque quad; changing only lighting must reach color.
    let pixel = (8 * STRIDE + 8 * 8) as usize;
    assert_ne!(&stable[pixel..pixel + 6], &lit[pixel..pixel + 6]);
    input.nodes[camera_index].world *= DMat4::from_translation(DVec3::new(0.1, 0., 0.));
    submit(&mut scene, &input, &mut renders);
    assert!(!scene.motion_frame().unwrap().camera_static());
    assert_eq!(change_header(&scene)[..2], [0, 0]);
    capture(&scene, &mut reads);
    for n in [8, 4, 12, 16] {
        let scale = crate::resolution::ResolutionScale::new(n).unwrap();
        scene
            .set_dynamic_resolution(Some(crate::resolution::ResolutionConfig {
                target_ms: 16.,
                deadband: 0.1,
                minimum: scale,
                maximum: scale,
                initial: scale,
                overload_samples: 2,
                recovery_samples: 2,
                cooldown_samples: 1,
                slew: 1,
            }))
            .unwrap();
        let old = scene.upscale_frame().unwrap();
        let before = surfaces(&scene);
        assert!(scene.submit(&renderer, &input, target(0)).is_err());
        assert_eq!(surfaces(&scene), before);
        assert_eq!(
            scene.upscale_frame().unwrap().motion().sequence(),
            old.motion().sequence()
        );
        submit(&mut scene, &input, &mut renders);
        let next = scene.upscale_frame().unwrap();
        assert!(next.motion().same_owner(first.motion()));
        assert_eq!(next.motion().epoch(), first.motion().epoch());
        assert_eq!(next.motion().sequence(), old.motion().sequence() + 1);
        assert!(next.motion().has_previous());
        assert!(!next.motion().camera_static());
        assert_eq!(change_header(&scene)[..2], [0, 0]);
        assert_eq!(next.motion().internal_size(), [SIZE * n as u32 / 16; 2]);
        assert_eq!(next.output_size(), [SIZE; 2]);
        capture(&scene, &mut reads);
    }
    let old = scene.upscale_frame().unwrap();
    scene.invalidate_motion_history();
    assert!(scene.submit(&renderer, &input, target(0)).is_err());
    assert_eq!(
        scene.upscale_frame().unwrap().motion().sequence(),
        old.motion().sequence()
    );
    submit(&mut scene, &input, &mut renders);
    let reset = scene.upscale_frame().unwrap();
    assert!(reset.motion().epoch() > old.motion().epoch());
    assert!(!reset.motion().has_previous());
    assert_eq!(reset.motion().jitter_phase(), Some(0));
    capture(&scene, &mut reads);
    scene.set_motion_owner_attachment(&renderer, false).unwrap();
    assert!(scene.upscale_frame().is_none() && scene.motion_frame().is_none());
    scene.enable_temporal_upscaling(&renderer, config).unwrap();
    submit(&mut scene, &input, &mut renders);
    assert!(
        scene
            .upscaling
            .as_ref()
            .unwrap()
            .accepted_owner_surfaces()
            .is_none()
    );
    assert!(!scene.motion_frame().unwrap().same_owner(reset.motion()));
    scene.set_motion_owner_attachment(&renderer, true).unwrap();
    assert!(scene.upscale_frame().is_none() && scene.motion_frame().is_none());
    scene.enable_temporal_upscaling(&renderer, config).unwrap();
    submit(&mut scene, &input, &mut renders);
    assert!(!scene.motion_frame().unwrap().has_previous());
    assert!(!scene.motion_frame().unwrap().same_owner(reset.motion()));
    capture(&scene, &mut reads);
    assert_eq!((renders, reads), (12, 12));
    println!(
        "RETAINED_OWNER frames=12 readback_submits=12 queue_submits=24 maps=12 max_extent=32x32 no_images=true"
    );
}
