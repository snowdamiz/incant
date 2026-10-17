//! Same authored/cooked material scene and complete motion schedule as MetalFX.
//! Capture and cost are separate invocations; neither silently approves quality.
use crate::{
    Renderer, RetainedTarget,
    metalfx_image::{capture, scene},
};
use serde_json::json;
const WIDTH: u32 = 960;
const HEIGHT: u32 = 540;
fn target(view: &wgpu::TextureView) -> RetainedTarget<'_> {
    RetainedTarget {
        view,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        width: WIDTH,
        height: HEIGHT,
        viewport: None,
    }
}
#[test]
#[ignore = "coordinated GPU only: four arms, 32 frames each, 128 bounded final-image readbacks"]
fn full_motion_color_taau_comparison() {
    run(false);
}
#[test]
#[ignore = "separate quiet GPU grant: same four arms/32 frames, timestamps only, no image readbacks"]
fn full_motion_color_taau_gpu_cost() {
    run(true);
}
fn run(cost: bool) {
    let root = std::env::var_os("INCANT_TAAU_OUTPUT").expect("explicit create-only output");
    let mut out = capture::Output::comparison(root.into());
    let stage = scene::Stage::new();
    out.assets(stage.fixture.root.path());
    out.write(
        "fixture/project.json",
        &serde_json::to_vec_pretty(&stage.fixture.project).unwrap(),
    );
    let mut input = stage.input();
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::METAL,
        ..Default::default()
    });
    let renderer = if cost {
        pollster::block_on(Renderer::new_with_scene_msaa_and_gpu_timing(
            &instance, None, 4,
        ))
    } else {
        pollster::block_on(Renderer::new_with_scene_msaa(&instance, None, 4))
    };
    let mut renderer = crate::test_support::checked(renderer.unwrap());
    assert_eq!(renderer.adapter.get_info().backend, wgpu::Backend::Metal);
    pollster::block_on(renderer.enable_temporal_upscale()).unwrap();
    pollster::block_on(renderer.enable_metalfx()).unwrap();
    assert!(
        renderer.metalfx_admission.is_some_and(|v| v.is_ok()),
        "comparison requires actual MetalFX"
    );
    let texture = capture::texture(&renderer);
    let view = texture.create_view(&Default::default());
    let mut bindings = Vec::new();
    let mut rows = Vec::new();
    for name in ["full4x", "spatial75", "taau75", "metalfx75"] {
        stage.pose(&mut input, 1);
        let mut retained = renderer.prepare_retained(&input, &stage.camera).unwrap();
        if name != "full4x" {
            let scale = crate::resolution::ResolutionScale::new(12).unwrap();
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
        }
        if name == "taau75" {
            retained.enable_temporal_upscale(&renderer).unwrap();
            retained
                .prepare_temporal_upscale_extent(&renderer, [WIDTH, HEIGHT])
                .unwrap();
        }
        if name == "metalfx75" {
            retained
                .enable_metalfx(
                    &renderer,
                    crate::metalfx::Preference::RequireMetalFx,
                    crate::metalfx::Config {
                        relative_depth: 0.01,
                        absolute_depth: 0.001,
                    },
                )
                .unwrap();
            retained
                .prepare_metalfx_extent(&renderer, [WIDTH, HEIGHT])
                .unwrap();
        }
        for frame in 1..=32 {
            stage.pose(&mut input, frame);
            let identity = stage.bindings(&input);
            if name == "full4x" {
                bindings.push(identity.clone());
            } else {
                assert_eq!(identity, bindings[(frame - 1) as usize]);
            }
            if frame == 25 {
                retained.invalidate_motion_history();
            }
            if name == "full4x" {
                // Ordinary production draw_scene actually selects Four. Retained
                // native submits deliberately select One, even on a Four renderer.
                let scene = renderer
                    .prepare(&input)
                    .unwrap()
                    .with_camera(&stage.camera)
                    .unwrap();
                let commands = renderer
                    .draw_scene(
                        &scene,
                        &view,
                        wgpu::TextureFormat::Rgba8UnormSrgb,
                        WIDTH,
                        HEIGHT,
                        None,
                    )
                    .unwrap();
                renderer.queue.submit([commands]);
            } else {
                retained.submit(&renderer, &input, target(&view)).unwrap();
            }
            capture::wait(&renderer);
            let temporal = name == "taau75" || name == "metalfx75";
            let motion = if temporal {
                let m = retained.motion_frame().unwrap();
                assert_eq!(m.sequence(), u64::from(frame));
                assert_eq!(m.internal_size(), [720, 405]);
                assert_eq!(m.has_previous(), frame != 1 && frame != 25);
                assert_eq!(m.epoch(), if frame < 25 { 1 } else { 2 });
                assert_eq!(m.jitter_phase(), Some((frame - 1) % 8));
                json!({"sequence":m.sequence(),"epoch":m.epoch(),"previous":m.has_previous(),"jitter_phase":m.jitter_phase()})
            } else {
                assert!(retained.motion_frame().is_none());
                json!(null)
            };
            let timing = if cost {
                let t = renderer
                    .take_gpu_timing()
                    .unwrap()
                    .expect("one completed timing per scene");
                assert!(t.frame.nanoseconds.is_finite() && t.frame.nanoseconds > 0.);
                if name == "taau75" {
                    assert!(t.passes[crate::timing::GpuPass::SceneResolve.index()].is_some());
                }
                json!(t)
            } else {
                json!(null)
            };
            if !cost {
                let pixels = capture::read(&renderer, &texture);
                if [1, 16, 24, 25, 32].contains(&frame) {
                    out.image(name, frame, &pixels);
                } else {
                    out.png(name, frame, &pixels);
                }
            }
            rows.push(
                json!({"arm":name,"frame":frame,"input":identity,"motion":motion,"timing":timing}),
            );
            if name == "taau75" && frame == 16 {
                let before = retained.motion_frame().unwrap();
                let mut refused = target(&view);
                refused.width = WIDTH - 1;
                assert!(retained.submit(&renderer, &input, refused).is_err());
                assert_eq!(
                    retained.motion_frame().unwrap().sequence(),
                    before.sequence()
                );
                // Rejected preparation neither writes nor submits. The actual
                // next accepted frame must keep its owner/sequence/jitter above.
            }
        }
    }
    let report = json!({"schema":1,"purpose":"fixed candidate comparison; appearance and cost acceptance pending",
        "adapter":format!("{:?}",renderer.adapter.get_info()),"cost":cost,"output":[WIDTH,HEIGHT],
        "frames_per_arm":32,"arms":["full4x","spatial75","taau75","metalfx75"],
        "full4x_route":"ordinary draw_scene, rebuilt CPU scene preparation; GPU timings only",
        "input_contract":"single-sample center opaque/MASK; motion previous-current jittered UV; output-sized unjittered history/current depth",
        "excluded":["transparent models","sprites","text","SSAO","bloom","cooked TAAU","performance gate"],
        "first_frames_include_cold_resources":true,"scene_submits":128,"image_readbacks":if cost{0}else{128},
        "candidate_history_bytes":super::owner::bytes([WIDTH,HEIGHT],8192).unwrap(),"rows":rows,"files":out.files});
    out.write("report.json", &serde_json::to_vec_pretty(&report).unwrap());
    println!("TAAU fixed comparison: four arms x 32 frames; cost={cost}; appearance unreviewed");
}
