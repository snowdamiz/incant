//! Delegated numeric-only control: 12 renders + 12 batched readback submissions.
//! No image files. This does not establish appearance or physical memory bounds.
use super::*;
use glam::{DMat4, DVec3};
use incant_assets_runtime::RuntimeAsset;
use incant_render_data::{MeshInput, SceneNode};
use serde_json::json;
fn quad(root: &std::path::Path, mode: &str) -> Arc<RuntimeAsset> {
    std::fs::create_dir_all(root).unwrap();
    let pos = [
        -0.6f32, -0.6, 0., 0.6, -0.6, 0., 0.6, 0.6, 0., -0.6, -0.6, 0., 0.6, 0.6, 0., -0.6, 0.6, 0.,
    ];
    let uv = [0f32, 1., 1., 1., 1., 0., 0., 1., 1., 0., 0., 0.];
    std::fs::write(
        root.join("q.bin"),
        pos.into_iter()
            .chain(uv)
            .flat_map(f32::to_le_bytes)
            .collect::<Vec<_>>(),
    )
    .unwrap();
    // Fixed 2x1 RGBA8 PNG: white transparent, white opaque; nearest MASK fixture.
    const MASK_PNG: &[u8] = &[
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 2, 0, 0, 0, 1, 8, 6,
        0, 0, 0, 244, 34, 127, 138, 0, 0, 0, 16, 73, 68, 65, 84, 120, 156, 99, 248, 255, 255, 63,
        3, 16, 255, 7, 0, 30, 234, 6, 250, 57, 43, 101, 106, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66,
        96, 130,
    ];
    std::fs::write(root.join("mask.png"), MASK_PNG).unwrap();
    let mut mat = json!({"alphaMode":mode,"doubleSided":false,"pbrMetallicRoughness":{"baseColorFactor":[0.6,0.3,0.2,1.0],"metallicFactor":0}});
    if mode == "MASK" {
        mat["pbrMetallicRoughness"]["baseColorTexture"] = json!({"index":0});
        mat["alphaCutoff"] = json!(0.5);
    }
    if mode == "BLEND" {
        mat["pbrMetallicRoughness"]["baseColorFactor"][3] = json!(0.5);
    }
    let doc = json!({"asset":{"version":"2.0"},"buffers":[{"uri":"q.bin","byteLength":120}],"bufferViews":[{"buffer":0,"byteOffset":0,"byteLength":72},{"buffer":0,"byteOffset":72,"byteLength":48}],"accessors":[{"bufferView":0,"componentType":5126,"count":6,"type":"VEC3","min":[-0.6,-0.6,0],"max":[0.6,0.6,0]},{"bufferView":1,"componentType":5126,"count":6,"type":"VEC2"}],"images":[{"uri":"mask.png"}],"textures":[{"source":0,"sampler":0}],"samplers":[{"magFilter":9728,"minFilter":9728,"wrapS":33071,"wrapT":33071}],"materials":[mat],"meshes":[{"primitives":[{"attributes":{"POSITION":0,"TEXCOORD_0":1},"material":0}]}],"nodes":[{"mesh":0}],"scenes":[{"nodes":[0]}],"scene":0});
    std::fs::write(root.join("q.gltf"), doc.to_string()).unwrap();
    let cache = root.join("cache");
    let cooked = incant_assets::cook_gltf(root, std::path::Path::new("q.gltf"), &cache).unwrap();
    Arc::new(RuntimeAsset::load_model(mode, 1, &cache, &cooked.metadata.fingerprint).unwrap())
}
fn fixture() -> SceneInput {
    let dir = tempfile::tempdir().unwrap();
    let opaque = quad(&dir.path().join("opaque"), "OPAQUE");
    let mask = quad(&dir.path().join("mask"), "MASK");
    let blend = quad(&dir.path().join("blend"), "BLEND");
    let skin_dir = dir.path().join("skin");
    std::fs::create_dir_all(&skin_dir).unwrap();
    let mut d = crate::models::skeletal_fixture::fixture(&skin_dir);
    d["scenes"][0]["nodes"] = json!([0, 3]);
    crate::models::skeletal_fixture::write(&skin_dir, &d);
    let cache = skin_dir.join("cache");
    let cooked =
        incant_assets::cook_gltf(&skin_dir, std::path::Path::new("model.gltf"), &cache).unwrap();
    let skin = Arc::new(
        RuntimeAsset::load_model("skin", 1, &cache, &cooked.metadata.fingerprint).unwrap(),
    );
    let mut input = SceneInput::default();
    for (i, (source, x, y, mirror)) in [
        (opaque.clone(), -2., 2., false),
        (mask, 2., 2., false),
        (opaque.clone(), -2., -2., true),
        (opaque, -2., 0., false),
        (skin, -11.07, -2.03, false),
        (blend, 2., -2., false),
    ]
    .into_iter()
    .enumerate()
    {
        input.nodes.push(SceneNode {
            id: format!("mesh{i}"),
            world: DMat4::from_translation(DVec3::new(x, y, 0.))
                * DMat4::from_scale(DVec3::new(if mirror { -1. } else { 1. }, 1., 1.)),
            camera: None,
            mesh: Some(MeshInput {
                source,
                cast_shadows: false,
                animation: (i == 4).then_some(incant_types::AnimationSample {
                    clip: Some(0),
                    seconds: 0.,
                }),
                animation_blend: None,
                animation_pose: None,
                animation_ik: None,
                animation_root: None,
            }),
            light: None,
            sprite: None,
            tilemap: None,
            diagnostic: false,
        });
    }
    input.nodes.push(SceneNode {
        id: "camera".into(),
        world: DMat4::from_translation(DVec3::new(0., 0., 10.)),
        camera: Some(incant_types::Camera {
            fov_degrees: 60.,
            near: 0.1,
            far: 50.,
            projection: incant_types::CameraProjection::Orthographic { vertical_size: 8. },
        }),
        mesh: None,
        light: None,
        sprite: None,
        tilemap: None,
        diagnostic: false,
    });
    dir.close().unwrap();
    input
}

#[test]
fn source_deleted_owner_fixture_matches_analytic_skin_and_mask() {
    use incant_assets_runtime::{RuntimeAssetData, animation::AnimationPlayer};
    let input = fixture();
    let source = |i: usize| match input.nodes[i].mesh.as_ref().unwrap().source.data() {
        RuntimeAssetData::Model(model) => model,
        _ => unreachable!(),
    };
    let mask = &source(1).images[0];
    assert_eq!((mask.width, mask.height), (2, 1));
    assert_eq!(
        mask.levels[0].as_ref(),
        &[255, 255, 255, 0, 255, 255, 255, 255]
    );
    let model = source(4);
    assert_eq!(model.metadata.scenes[0], [0, 3]);
    let mut player = AnimationPlayer::new(model).unwrap();
    for seconds in [0., 1.] {
        player.sample(Some(0), seconds).unwrap();
        for (i, vertex) in model.meshes[0].vertices.iter().enumerate() {
            let p = [vertex[0], vertex[1], vertex[2]];
            let actual = player.skin_position(3, 0, i, p).unwrap();
            assert!(DVec3::from_array(actual).abs_diff_eq(
                DVec3::new(f64::from(p[0]) + 10. + seconds, f64::from(p[1]), 0.),
                1e-10
            ));
        }
    }
}

fn read(
    renderer: &Renderer,
    views: &[(&wgpu::TextureView, u32, wgpu::TextureAspect)],
) -> Vec<Vec<u8>> {
    let mut encoder = renderer.device.create_command_encoder(&Default::default());
    let mut rows = Vec::new();
    for &(view, bpp, aspect) in views {
        let t = view.texture();
        let stride = (t.width() * bpp).div_ceil(256) * 256;
        let b = renderer.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Owner numeric readback"),
            size: u64::from(stride) * u64::from(t.height()),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                aspect,
                ..t.as_image_copy()
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &b,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(stride),
                    rows_per_image: Some(t.height()),
                },
            },
            t.size(),
        );
        rows.push((b, stride, t.width() * bpp));
    }
    renderer.queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    for (b, _, _) in &rows {
        let tx = tx.clone();
        b.slice(..).map_async(wgpu::MapMode::Read, move |v| {
            let _ = tx.send(v);
        });
    }
    renderer
        .device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(10)),
        })
        .unwrap();
    for _ in &rows {
        rx.recv_timeout(std::time::Duration::from_secs(10))
            .unwrap()
            .unwrap();
    }
    rows.into_iter()
        .map(|(b, stride, width)| {
            let map = b.slice(..).get_mapped_range();
            let bytes = map
                .chunks(stride as usize)
                .flat_map(|row| row[..width as usize].iter().copied())
                .collect();
            drop(map);
            b.unmap();
            bytes
        })
        .collect()
}
#[test]
#[ignore = "requires separately granted native GPU; 12 renders +12 readback submissions, no images"]
fn exact_integer_owner_matches_visibility_and_leaves_motion_depth_color_unchanged() {
    let mut renderer = crate::test_support::checked(Renderer::headless().unwrap());
    eprintln!("MOTION_OWNER adapter={:?}", renderer.adapter.get_info());
    if crate::motion::admission().is_err() {
        assert!(pollster::block_on(renderer.enable_motion_vectors()).is_err());
        return;
    }
    assert!(pollster::block_on(renderer.enable_motion_owner()).is_err());
    pollster::block_on(renderer.enable_motion_vectors()).unwrap();
    pollster::block_on(renderer.enable_motion_owner()).unwrap();
    for scale in [16, 8] {
        let mut input = fixture();
        let mut off = renderer.prepare_retained(&input, "camera").unwrap();
        let mut on = renderer.prepare_retained(&input, "camera").unwrap();
        off.enable_motion_history(&renderer).unwrap();
        on.enable_motion_history(&renderer).unwrap();
        on.set_motion_owner_attachment(&renderer, true).unwrap();
        for owner in [&mut off, &mut on] {
            let s = crate::resolution::ResolutionScale::new(scale).unwrap();
            owner
                .set_dynamic_resolution(Some(crate::resolution::ResolutionConfig {
                    target_ms: 16.,
                    deadband: 0.1,
                    minimum: s,
                    maximum: s,
                    initial: s,
                    overload_samples: 1,
                    recovery_samples: 1,
                    cooldown_samples: 0,
                    slew: 1,
                }))
                .unwrap();
        }
        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let outputs = std::array::from_fn::<_, 2, _>(|_| {
            crate::hdr_output::texture(&renderer, format, 32, 32)
        });
        let views = outputs
            .each_ref()
            .map(|t| t.create_view(&Default::default()));
        let target = |i, width| RetainedTarget {
            view: &views[i],
            format,
            width,
            height: 32,
            viewport: None,
        };
        let mut last_owner = None;
        for frame in 0..3 {
            if frame == 2 {
                input.nodes[2].world = DMat4::from_translation(DVec3::new(-1.75, -2., 0.));
                input.nodes[4]
                    .mesh
                    .as_mut()
                    .unwrap()
                    .animation
                    .as_mut()
                    .unwrap()
                    .seconds = 1.;
            }
            if frame == 2 {
                let accepted = on.motion_frame().unwrap();
                assert!(on.submit(&renderer, &input, target(1, 0)).is_err());
                assert_eq!(on.motion_frame().unwrap().sequence(), accepted.sequence());
            }
            off.submit(&renderer, &input, target(0, 32)).unwrap();
            on.submit(&renderer, &input, target(1, 32)).unwrap();
            let m = on.motion.as_ref().unwrap();
            let (owner, motion, depth) = m.owner_test_views().unwrap();
            if let Some(old) = last_owner.replace(owner.clone()) {
                assert_eq!(old, owner, "unchanged extent must reuse allocation");
            }
            let raw = read(
                &renderer,
                &[
                    (&owner, 4, wgpu::TextureAspect::All),
                    (&motion, 16, wgpu::TextureAspect::All),
                    (&depth, 4, wgpu::TextureAspect::DepthOnly),
                    (&views[1], 4, wgpu::TextureAspect::All),
                ],
            );
            let offm = off.motion.as_ref().unwrap();
            let offraw = read(
                &renderer,
                &[
                    (&offm.readback_view().unwrap(), 16, wgpu::TextureAspect::All),
                    (
                        &offm.depth_test_view().unwrap(),
                        4,
                        wgpu::TextureAspect::DepthOnly,
                    ),
                    (&views[0], 4, wgpu::TextureAspect::All),
                ],
            );
            assert_eq!(raw[1..], offraw);
            let size = owner.texture().width();
            let changes = on.motion_changes().unwrap();
            assert_eq!(changes.highest_owner(), 5);
            assert_eq!(
                changes.changed_count(),
                if frame == 0 {
                    5
                } else if frame == 2 {
                    2
                } else {
                    0
                }
            );
            let ids: Vec<_> = (0..5).map(|node| m.owner_test_id(node).unwrap()).collect();
            for (i, b) in raw[0].as_chunks::<4>().0.iter().enumerate() {
                let x = (i as u32 % size) as f64;
                let y = (i as u32 / size) as f64;
                let wx = (x + 0.5) * 8. / f64::from(size) - 4.;
                let wy = 4. - (y + 0.5) * 8. / f64::from(size);
                let mut expected = 0;
                for (node, cx, cy) in [
                    (0, -2., 2.),
                    (1, 2., 2.),
                    (2, if frame == 2 { -1.75 } else { -2. }, -2.),
                    (3, -2., 0.),
                ] {
                    if (wx - cx).abs() < 0.6 && (wy - cy).abs() < 0.6 && (node != 1 || wx > cx) {
                        expected = ids[node];
                    }
                }
                let sx = if frame == 2 { 0.93 } else { -0.07 };
                if wx > sx && wy > -2.03 && wx - sx + (wy + 2.03) < 1. {
                    expected = ids[4];
                }
                if expected != 0 && changes.changed(expected) == Some(true) {
                    expected |= 1 << 31;
                }
                assert_eq!(
                    u32::from_le_bytes(*b),
                    expected,
                    "scale={scale} frame={frame} sample={i} world=({wx},{wy})"
                );
            }
        }
        let old = on.motion_frame().unwrap();
        on.set_motion_owner_attachment(&renderer, true).unwrap();
        assert_eq!(on.motion_frame().unwrap().sequence(), old.sequence());
        let refused = on
            .enable_temporal_history(
                &renderer,
                crate::temporal::TemporalConfig {
                    history_weight: 0.5,
                    relative_depth: 0.01,
                    absolute_depth: 0.001,
                    reactive_threshold: 1.,
                    reactive_strength: 1.,
                },
            )
            .unwrap_err();
        assert_eq!(
            refused.downcast_ref::<crate::motion::MotionError>(),
            Some(&crate::motion::MotionError::OwnerCombination)
        );
        assert_eq!(on.motion_frame().unwrap().sequence(), old.sequence());
        on.set_motion_owner_attachment(&renderer, false).unwrap();
        assert!(on.motion_frame().is_none());
        assert!(on.motion_changes().is_none());
        assert!(!on.motion_owner_attachment_enabled());
        // No extra render: next history preparation is required to create a new owner.
        let _ = old;
    }
    eprintln!(
        "MOTION_OWNER renders=12 readback_submissions=12 max_extent=32x32 source_removed=true no_images=true"
    );
}
