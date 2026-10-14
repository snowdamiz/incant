//! Actual material image/motion observations, never a numerical quality gate.
//! Explicitly outside the ordinary/default test and shipping execution routes.
mod capture;
mod geometry;
#[path = "../../tests/materials/mod.rs"]
#[allow(dead_code)] // Reuse the existing whole fixture; its other public helpers are unrelated.
mod materials;
mod scene;
use crate::{
    Renderer, RetainedTarget,
    metalfx::{Config, Preference, Selection},
};
use serde_json::json;
const WIDTH: u32 = 960;
const HEIGHT: u32 = 540;
const CAPTURES: [u32; 5] = [1, 16, 24, 25, 32];

#[test]
fn shared_pose_schedule_has_static_motion_and_explicit_cut() {
    let (first, mover) = scene::poses(1);
    for f in 1..=16 {
        assert_eq!(scene::poses(f), (first, mover));
    }
    for f in [17, 18, 24, 26, 32] {
        let (before, old_mover) = scene::poses(f - 1);
        let (now, new_mover) = scene::poses(f);
        assert!(((now.w_axis.x - before.w_axis.x) - 0.05).abs() < 1e-12);
        assert!(((new_mover.w_axis.x - old_mover.w_axis.x) - 0.12).abs() < 1e-12);
    }
    let (before, _) = scene::poses(24);
    let (after, _) = scene::poses(25);
    assert!(((after.w_axis.x - before.w_axis.x) - 1.05).abs() < 1e-12);
    assert_eq!(CAPTURES.len() * 3, 15);
}
#[test]
fn shared_stage_uses_opaque_cooked_assets_and_stable_semantic_bindings() {
    let stage = scene::Stage::new();
    let mut input = stage.input();
    assert_eq!(input.nodes.len(), 6);
    assert!(input.text.is_none());
    assert!(
        input
            .nodes
            .iter()
            .all(|n| !n.diagnostic && n.sprite.is_none() && n.tilemap.is_none())
    );
    let meshes: Vec<_> = input.nodes.iter().filter_map(|n| n.mesh.as_ref()).collect();
    assert_eq!(meshes.len(), 3);
    for mesh in &meshes {
        let incant_assets_runtime::RuntimeAssetData::Model(model) = mesh.source.data() else {
            panic!("model binding");
        };
        assert!(
            model
                .materials
                .iter()
                .all(|m| m.alpha_mode == incant_assets_runtime::AlphaMode::Opaque)
        );
        assert!(!model.meshes.is_empty());
    }
    let versions: Vec<_> = meshes.iter().map(|m| m.source.version_id()).collect();
    drop(meshes);
    stage.pose(&mut input, 1);
    let first = stage.bindings(&input);
    stage.pose(&mut input, 32);
    assert_ne!(first, stage.bindings(&input));
    stage.pose(&mut input, 1);
    assert_eq!(first, stage.bindings(&input));
    assert_eq!(
        versions,
        input
            .nodes
            .iter()
            .filter_map(|n| n.mesh.as_ref())
            .map(|m| m.source.version_id())
            .collect::<Vec<_>>()
    );
}
#[test]
#[ignore = "coordinated Mac GPU only: 96 scene submits, 15 readbacks/maps, 15 images"]
fn actual_full_half_three_quarter_material_images_and_motion() {
    let root =
        std::env::var_os("INCANT_METALFX_IMAGE_OUTPUT").expect("explicit create-only output");
    let mut output = capture::Output::new(root.into());
    let stage = scene::Stage::new();
    output.assets(stage.fixture.root.path());
    output.write(
        "fixture/project.json",
        &serde_json::to_vec_pretty(&stage.fixture.project).unwrap(),
    );
    let mut input = stage.input();
    assert!(input.text.is_none());
    assert!(
        input
            .nodes
            .iter()
            .all(|n| !n.diagnostic && n.sprite.is_none() && n.tilemap.is_none())
    );
    let mut renderer = crate::test_support::checked(Renderer::headless().unwrap());
    assert_eq!(renderer.adapter.get_info().backend, wgpu::Backend::Metal);
    pollster::block_on(renderer.enable_metalfx()).unwrap();
    assert!(
        renderer.metalfx_admission.is_some_and(|r| r.is_ok()),
        "native support required"
    );
    let texture = capture::texture(&renderer);
    let view = texture.create_view(&Default::default());
    let mut arms = Vec::new();
    for (name, sixteenths) in [("full", 16), ("mfx50", 8), ("mfx75", 12)] {
        let mut retained = renderer.prepare_retained(&input, &stage.camera).unwrap();
        if sixteenths != 16 {
            retained
                .enable_metalfx(
                    &renderer,
                    Preference::RequireMetalFx,
                    Config {
                        relative_depth: 0.01,
                        absolute_depth: 0.001,
                    },
                )
                .unwrap();
            let scale = crate::resolution::ResolutionScale::new(sixteenths).unwrap();
            retained
                .set_dynamic_resolution(Some(crate::resolution::ResolutionConfig {
                    initial: scale,
                    minimum: scale,
                    maximum: scale,
                    target_ms: 16.,
                    deadband: 0.1,
                    overload_samples: 1,
                    recovery_samples: 1,
                    cooldown_samples: 0,
                    slew: 1,
                }))
                .unwrap();
            // Every vendor scaler is initialized before the measured chronology.
            // This is an image experiment; no initialization or frame speed claim.
            retained
                .prepare_metalfx_extent(&renderer, [WIDTH, HEIGHT])
                .unwrap();
        }
        arms.push((name, sixteenths, retained));
    }
    let mut bindings = Vec::new();
    let mut rows = Vec::new();
    let (mut submits, mut maps, mut native, mut buffers) = (0, 0, 0, 0);
    for (name, scale, mut retained) in arms {
        for frame in 1..=32 {
            stage.pose(&mut input, frame);
            let input_binding = stage.bindings(&input);
            if name == "full" {
                bindings.push(input_binding.clone());
            } else {
                assert_eq!(
                    input_binding,
                    bindings[(frame - 1) as usize],
                    "shared exact inputs"
                );
            }
            if frame == 25 && scale != 16 {
                retained.invalidate_motion_history();
            }
            retained
                .submit(
                    &renderer,
                    &input,
                    RetainedTarget {
                        view: &view,
                        format: wgpu::TextureFormat::Rgba8UnormSrgb,
                        width: WIDTH,
                        height: HEIGHT,
                        viewport: None,
                    },
                )
                .unwrap();
            submits += 1;
            buffers += if scale == 16 { 1 } else { 3 };
            capture::wait(&renderer);
            let motion = if scale == 16 {
                assert_eq!(retained.metalfx_selection(), None);
                assert!(retained.motion_frame().is_none());
                json!(null)
            } else {
                assert_eq!(retained.metalfx_selection(), Some(Selection::MetalFx));
                native += 1;
                let m = retained.motion_frame().unwrap();
                let extent = [
                    WIDTH * u32::from(scale) / 16,
                    HEIGHT * u32::from(scale) / 16,
                ];
                assert_eq!(m.internal_size(), extent);
                assert_eq!(m.sequence(), u64::from(frame));
                assert_eq!(m.has_previous(), frame != 1 && frame != 25);
                assert!(!m.bypassed());
                let phase = (frame - 1) % 8;
                assert_eq!(m.jitter_phase(), Some(phase));
                assert_eq!(m.epoch(), if frame < 25 { 1 } else { 2 });
                json!({"sequence":m.sequence(),"epoch":m.epoch(),"previous":m.has_previous(),
                    "content":extent,"jitter_phase":phase,"jitter_internal_pixels":crate::temporal::jitter::SAMPLES[phase as usize]})
            };
            rows.push(json!({"arm":name,"frame":frame,"input":input_binding,"motion":motion,
                "selection":format!("{:?}",retained.metalfx_selection()),"output":[WIDTH,HEIGHT],"cut":frame==25}));
            if CAPTURES.contains(&frame) {
                let pixels = capture::read(&renderer, &texture);
                submits += 1;
                maps += 1;
                buffers += 1;
                output.image(name, frame, &pixels);
            }
        }
    }
    assert_eq!((submits, maps, native, buffers), (111, 15, 64, 239));
    let report = json!({"schema":1,"purpose":"observations only, no visual or performance acceptance",
        "adapter":format!("{:?}",renderer.adapter.get_info()),"native_admission":format!("{:?}",renderer.metalfx_admission),
        "scene_source":"6cf4660a71a30767e2f4e09749e44ab1678d2a53 opaque Stage subset",
        "excluded":["BLEND card","sprites","world text","bloom","portable upscaling shaders"],
        "output_format":"RGBA8 sRGB final display; alpha is final composition, not vendor coverage",
        "budget":{"queue_submits":submits,"maps":maps,"native_raw_encodes":native,"explicit_command_buffers":buffers,
            "vendor_internal_work_counted":false,"frames_per_arm":32,"captures":15,"width":WIDTH,"height":HEIGHT},
        "frames":rows,"files":output.files});
    output.write("report.json", &serde_json::to_vec_pretty(&report).unwrap());
    println!(
        "MetalFX image fixture: 96 scene frames; 111 submits; 15 maps; 64 native encodes; 239 command buffers; 15 images; appearance unreviewed"
    );
}
