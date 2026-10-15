//! Actual raster and compiler-owner controls. These remain ignored until an
//! explicit native GPU lease. The pause exists only in the test binary and
//! holds a queued job before its compiler closure; it never pauses a driver call.
use crate::{Renderer, SceneInput, SceneMsaa, hdr_output};
use incant_assets_runtime::RuntimeAsset;
use incant_render_data::{MeshInput, SceneNode, material_keys::MaterialSamples, prewarm::*};
use serde_json::json;
use std::{path::Path, sync::Arc, time::Duration};
const SIZE: [u32; 2] = [96, 80];

fn node(id: &str) -> SceneNode {
    SceneNode {
        id: id.into(),
        world: glam::DMat4::IDENTITY,
        camera: None,
        light: None,
        mesh: None,
        sprite: None,
        tilemap: None,
        diagnostic: false,
    }
}
fn fixture(alpha: &str, factor: f32, double: bool, mirror: bool, backface: bool) -> SceneInput {
    let root = tempfile::tempdir().unwrap();
    let positions = [
        -1.6_f32, -1.4, 0., 1.6, -1.4, 0., 1.6, 1.4, 0., -1.6, -1.4, 0., 1.6, 1.4, 0., -1.6, 1.4,
        0.,
    ];
    let uv = [0_f32, 0., 1., 0., 1., 1., 0., 0., 1., 1., 0., 1.];
    std::fs::write(
        root.path().join("mesh.bin"),
        positions
            .into_iter()
            .chain(uv)
            .flat_map(f32::to_le_bytes)
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let mut encoder = png::Encoder::new(
        std::fs::File::create(root.path().join("alpha.png")).unwrap(),
        2,
        2,
    );
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&[
            255, 255, 255, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 0,
        ])
        .unwrap();
    let front = json!({"alphaMode":alpha,"alphaCutoff":0.5,"doubleSided":double,
        "pbrMetallicRoughness":{"baseColorFactor":[0.8,0.06,0.04,factor],"metallicFactor":0,"baseColorTexture":{"index":0}},"emissiveFactor":[0.5,0,0]});
    let doc = json!({"asset":{"version":"2.0"},"buffers":[{"uri":"mesh.bin","byteLength":120}],
        "bufferViews":[{"buffer":0,"byteLength":72},{"buffer":0,"byteOffset":72,"byteLength":48}],
        "accessors":[{"bufferView":0,"componentType":5126,"count":6,"type":"VEC3","min":[-1.6,-1.4,0],"max":[1.6,1.4,0]},{"bufferView":1,"componentType":5126,"count":6,"type":"VEC2"}],
        "images":[{"uri":"alpha.png"}],"samplers":[{"magFilter":9728,"minFilter":9728,"wrapS":33071,"wrapT":33071}],"textures":[{"source":0,"sampler":0}],
        "materials":[front,{"doubleSided":true,"pbrMetallicRoughness":{"baseColorFactor":[0.04,0.06,0.8,1],"metallicFactor":0},"emissiveFactor":[0,0,0.5]}],
        "meshes":[{"primitives":[{"attributes":{"POSITION":0,"TEXCOORD_0":1},"material":0}]},{"primitives":[{"attributes":{"POSITION":0,"TEXCOORD_0":1},"material":1}]}],
        "nodes":[{"mesh":0,"rotation":if backface{[0,1,0,0]}else{[0,0,0,1]}},{"mesh":1,"translation":[0,0,-1],"scale":[1.15,1.15,1]}],"scenes":[{"nodes":[0,1]}],"scene":0});
    std::fs::write(root.path().join("mesh.gltf"), doc.to_string()).unwrap();
    let cache = root.path().join("cache");
    let cooked = incant_assets::cook_gltf(root.path(), Path::new("mesh.gltf"), &cache).unwrap();
    let source = Arc::new(
        RuntimeAsset::load_model("fixture", 1, &cache, &cooked.metadata.fingerprint).unwrap(),
    );
    let mut model = node("model");
    model.world = glam::DMat4::from_scale_rotation_translation(
        glam::DVec3::new(if mirror { -1. } else { 1. }, 1., 1.),
        glam::DQuat::from_rotation_z(0.19),
        glam::DVec3::ZERO,
    );
    model.mesh = Some(MeshInput {
        source,
        cast_shadows: false,
        animation: None,
        animation_blend: None,
        animation_pose: None,
        animation_ik: None,
        animation_root: None,
    });
    let mut camera = node("camera");
    camera.world.w_axis.z = 8.;
    camera.camera = Some(incant_types::Camera {
        near: 0.1,
        far: 30.,
        fov_degrees: 60.,
        projection: incant_types::CameraProjection::Orthographic { vertical_size: 5. },
    });
    root.close().unwrap();
    SceneInput {
        nodes: vec![model, camera],
        ..Default::default()
    }
}
fn manifest(input: &SceneInput, samples: MaterialSamples) -> MaterialManifest {
    MaterialRecorder::new(
        input,
        "camera",
        0,
        RecordingContext {
            scene: [1; 16],
            cooked_scene_sha256: [2; 32],
            profile: MaterialProfile {
                samples,
                program: Renderer::material_program(),
            },
            script_sha256: [3; 32],
            input_sha256: [4; 32],
            tick_rate: 60,
        },
    )
    .unwrap()
    .manifest()
    .unwrap()
}
fn draw(renderer: &Renderer, input: &SceneInput) -> Vec<u8> {
    let scene = renderer
        .prepare(input)
        .unwrap()
        .with_camera("camera")
        .unwrap();
    let output = hdr_output::texture(
        renderer,
        wgpu::TextureFormat::Rgba8UnormSrgb,
        SIZE[0],
        SIZE[1],
    );
    renderer.queue.submit([renderer
        .draw_scene(
            &scene,
            &output.create_view(&Default::default()),
            output.format(),
            SIZE[0],
            SIZE[1],
            None,
        )
        .unwrap()]);
    hdr_output::read(renderer, &output)
}
fn record(name: &str, renderer: &Renderer, pixels: &[u8]) {
    use std::io::Write;
    let Some(root) = std::env::var_os("INCANT_MATERIAL_PREWARM_EVIDENCE") else {
        return;
    };
    let root = Path::new(&root);
    let info = renderer.adapter.get_info();
    for (suffix,bytes) in [("rgba",pixels.to_vec()),("json",serde_json::to_vec_pretty(&json!({"case":name,"size":SIZE,"adapter":format!("{info:?}"),"profile":Renderer::material_program(),"samples":renderer.scene_sample_count(),"prewarm":renderer.material_prewarm_report(),"scope":"numerical source-free raster control; no latency or stall claim"})).unwrap())] {
        std::fs::OpenOptions::new().write(true).create_new(true).open(root.join(format!("{name}.{suffix}"))).unwrap().write_all(&bytes).unwrap();
    }
}
fn partial_alpha(renderer: &Renderer, samples: u32) {
    // Both pending surfaces stay below the output curve's 0.8 shoulder.
    // Decode sRGB and compare actual partial coverage against the two native
    // endpoint draws. This catches opaque substitution and incorrect blend
    // factors without comparing the deliberately different pending/real RGB.
    let zero = draw(renderer, &fixture("BLEND", 0., false, false, false));
    let one = draw(renderer, &fixture("BLEND", 1., false, false, false));
    let linear = |byte: u8| {
        let value = f32::from(byte) / 255.;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    for alpha in [0.25, 0.75] {
        let partial = draw(renderer, &fixture("BLEND", alpha, false, false, false));
        let mut checked = 0;
        for ((zero, one), partial) in zero
            .as_chunks::<4>()
            .0
            .iter()
            .zip(one.as_chunks::<4>().0)
            .zip(partial.as_chunks::<4>().0)
        {
            if one[0].saturating_sub(zero[0]) < 32 {
                continue;
            }
            for channel in 0..3 {
                let expected = linear(zero[channel]) * (1. - alpha) + linear(one[channel]) * alpha;
                assert!(
                    (linear(partial[channel]) - expected).abs() < 0.012,
                    "partial alpha {alpha}, samples {samples}, channel {channel}"
                );
            }
            checked += 1;
        }
        assert!(checked > 100, "partial-alpha fixture has no interior");
        record(
            &format!("pending-{samples}-blend-{alpha}"),
            renderer,
            &partial,
        );
    }
}
#[test]
#[ignore = "requires an explicit native GPU lease and raster review"]
fn pending_materials_preserve_alpha_culling_coverage_and_occlusion_in_one_and_four_samples() {
    for (samples, count) in [(MaterialSamples::One, 1), (MaterialSamples::Four, 4)] {
        let first = fixture("OPAQUE", 1., false, false, false);
        let mut keys = manifest(&first, samples);
        keys.observed.clear();
        let admitted = keys.admit(&keys.identity).unwrap();
        let instance = wgpu::Instance::default();
        let mut pending = crate::test_support::checked(
            pollster::block_on(Renderer::new_with_recorded_materials(
                &instance, None, admitted,
            ))
            .unwrap(),
        );
        pending
            .materials
            .recorded
            .as_ref()
            .unwrap()
            .pause_worker(true);
        let eager = crate::test_support::checked(
            pollster::block_on(Renderer::new_with_scene_msaa(
                &wgpu::Instance::default(),
                None,
                count,
            ))
            .unwrap(),
        );
        let before = instance.generate_report().unwrap();
        for alpha in ["OPAQUE", "MASK", "BLEND"] {
            for double in [false, true] {
                for mirror in [false, true] {
                    let input = fixture(alpha, 1., double, mirror, false);
                    let a = draw(&pending, &input);
                    let b = draw(&eager, &input);
                    // Red foreground versus blue rear geometry. The pending RGB is
                    // deliberately simpler; exact red-dominant sample coverage must
                    // still match for both winding signs and the alpha texture.
                    let coverage = |bytes: &[u8]| {
                        bytes
                            .as_chunks::<4>()
                            .0
                            .iter()
                            .map(|p| p[0] > p[2])
                            .collect::<Vec<_>>()
                    };
                    assert_eq!(
                        coverage(&a),
                        coverage(&b),
                        "{alpha}/{double}/{mirror}/{count}"
                    );
                    assert!(coverage(&a).iter().any(|v| *v));
                    record(
                        &format!("pending-{count}-{alpha}-{double}-{mirror}"),
                        &pending,
                        &a,
                    );
                }
            }
        }
        for (alpha, factor, double, backface, visible) in [
            ("OPAQUE", 0., false, false, true),
            ("MASK", 0., false, false, false),
            ("BLEND", 0., false, false, false),
            ("OPAQUE", 1., false, true, false),
            ("OPAQUE", 1., true, true, true),
        ] {
            let input = fixture(alpha, factor, double, false, backface);
            let a = draw(&pending, &input);
            let b = draw(&eager, &input);
            let red = |bytes: &[u8]| {
                bytes
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .filter(|p| p[0] > p[2])
                    .count()
            };
            assert_eq!(red(&a), red(&b));
            assert_eq!(red(&a) > 0, visible);
        }
        partial_alpha(&pending, count);
        let after = instance.generate_report().unwrap();
        assert_eq!(
            before.hub.render_pipelines, after.hub.render_pipelines,
            "paused frame path created a PSO"
        );
        let report = pending.material_prewarm_report().unwrap();
        assert_eq!(report.prewarmed, 0);
        assert_eq!(report.real_selections, 0);
        assert!(report.fallback_selections > 0);
        assert!(report.keys.iter().all(|k| !k.ready && k.failure.is_none()));
        assert!(
            pollster::block_on(pending.enable_ambient_occlusion(crate::AmbientOcclusion {
                radius: 1.,
                bias: 0.01,
                strength: 1.
            }))
            .is_err()
        );
        pending
            .close_recorded_materials(Duration::from_secs(2))
            .unwrap();
        assert_eq!(
            pending.scene_msaa,
            if count == 1 {
                SceneMsaa::One
            } else {
                SceneMsaa::Four
            }
        );
    }
}
#[cfg(all(feature = "native-compilation", target_os = "macos"))]
#[test]
#[ignore = "requires an explicit native Metal GPU lease"]
fn missing_key_native_calls_are_owned_by_the_worker_and_publication_changes_next_frame() {
    use crate::compilation::{Capture, Kind, Outcome, Phase};
    let mut input = fixture("OPAQUE", 1., false, false, false);
    let keys = manifest(&input, MaterialSamples::One);
    let mut capture = Capture::new(512).unwrap();
    let instance = wgpu::Instance::default();
    let mut renderer = crate::test_support::checked(
        pollster::block_on(Renderer::new_recorded_materials_with_compilation_capture(
            &instance,
            None,
            keys.admit(&keys.identity).unwrap(),
            &mut capture,
        ))
        .unwrap(),
    );
    renderer
        .materials
        .recorded
        .as_ref()
        .unwrap()
        .pause_worker(true);
    let observer = capture.observer().unwrap();
    let startup = observer.snapshot();
    input.nodes[0].world.x_axis = -input.nodes[0].world.x_axis;
    let fallback = draw(&renderer, &input);
    let pending = renderer.material_prewarm_report().unwrap();
    assert!(pending.keys.iter().any(|k| k.requested && !k.ready));
    assert_eq!(
        observer.snapshot().counts,
        startup.counts,
        "paused frame path issued a native creation call"
    );
    record("worker-pending", &renderer, &fallback);
    renderer
        .materials
        .recorded
        .as_ref()
        .unwrap()
        .pause_worker(false);
    let start = std::time::Instant::now();
    loop {
        let report = renderer.material_prewarm_report().unwrap();
        assert!(report.keys.iter().all(|k| k.failure.is_none()));
        if report.keys.iter().filter(|k| k.requested).all(|k| k.ready) {
            break;
        }
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "worker did not finish in this probe"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
    let real = draw(&renderer, &input);
    assert_ne!(fallback, real);
    let observed = observer.snapshot();
    let later: Vec<_> = observed
        .rows
        .iter()
        .filter(|row| row.sequence > startup.rows.last().map_or(0, |r| r.sequence))
        .collect();
    assert!(
        later
            .iter()
            .any(|row| row.kind == Kind::RenderPipeline && row.outcome == Outcome::Success)
    );
    assert!(
        later
            .iter()
            .all(|row| row.context.phase == Phase::Background
                && row.context.segment > 0
                && row.outcome == Outcome::Success)
    );
    assert_eq!(observed.overflow, 0);
    assert!(!observed.saturated);
    let native = json!({"device": observed.device,"startup_last_sequence":startup.rows.last().map_or(0,|r|r.sequence),
        "rows":observed.rows.iter().map(|r|json!({"sequence":r.sequence,"phase":format!("{:?}",r.context.phase),"segment":r.context.segment,"kind":format!("{:?}",r.kind),"stage":format!("{:?}",r.stage),"outcome":format!("{:?}",r.outcome),"elapsed_ns":r.elapsed_ns,"error_domain":format!("{:?}",r.error_domain),"error_code":r.error_code})).collect::<Vec<_>>(),
        "scope":"native creation calls attributed to actual worker; not driver stall/performance evidence"});
    if let Some(root) = std::env::var_os("INCANT_MATERIAL_PREWARM_EVIDENCE") {
        use std::io::Write;
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(Path::new(&root).join("worker-native-compilation.json"))
            .unwrap()
            .write_all(&serde_json::to_vec_pretty(&native).unwrap())
            .unwrap();
    }
    assert!(renderer.material_prewarm_report().unwrap().real_selections > 0);
    record("worker-published", &renderer, &real);
    renderer
        .close_recorded_materials(Duration::from_secs(2))
        .unwrap();
}
