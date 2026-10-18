//! Manual, CPU-only qualification of the actual build-linked Core route.
//! No executable bytes are loaded from the fixture. Native windows, GPU and
//! physical audio devices are never constructed. Normal ignored CI cannot
//! select this module without the explicit diagnostic-core-pose feature.
use crate::{bounded, embedded, package, pcm_output::PcmOutput, session, startup};
use incant_assets_runtime::{RuntimeAsset, RuntimeAssetData, animation::AnimationPlayer};
use incant_game_runtime::HostedOfflineGameSession;
use incant_input::InputReplay;
use incant_render_data::{SceneInput, UiInput};
use incant_render_game::{
    NativeRenderHost, NativeRenderSettings, RenderInputConsumer,
    pose::{PoseConsumer, PoseRenderHost, PoseRequest},
};
use incant_render_native::{UpdateKind, interpolation::PoseReceipt};
use incant_runtime::CookedScene;
use serde::Deserialize;
use std::{collections::BTreeMap, fs::OpenOptions, io::Write, path::Path, sync::Arc};
mod compare;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Fixture {
    format: String,
    data: String,
    recording: String,
    observations: String,
    expected_bytecode_sha256: [u8; 32],
    expected_pcm_sha256: String,
    output: String,
}
type Animated = (Arc<RuntimeAsset>, AnimationPlayer);
#[derive(Default)]
struct Consumer {
    animated: BTreeMap<String, Animated>,
    ui: Option<Arc<UiInput>>,
    displays: u64,
    sampled_rigid_poses: u64,
    glyph_displays: u64,
}
impl Consumer {
    fn sample(&mut self, input: &SceneInput, camera: &str) -> session::Result<()> {
        assert!(
            input
                .nodes
                .iter()
                .any(|n| n.id == camera && n.camera.is_some())
        );
        self.animated.retain(|id, _| {
            input
                .nodes
                .iter()
                .any(|n| &n.id == id && n.mesh.as_ref().is_some_and(|m| m.animation.is_some()))
        });
        for node in &input.nodes {
            assert!(node.world.is_finite());
            let Some(mesh) = node.mesh.as_ref().filter(|m| m.animation.is_some()) else {
                continue;
            };
            let RuntimeAssetData::Model(model) = mesh.source.data() else {
                panic!("animated model")
            };
            assert!(
                model.animation.as_ref().unwrap().skins.is_empty(),
                "the frozen Core fixture uses rigid animation"
            );
            if self
                .animated
                .get(&node.id)
                .is_none_or(|(source, _)| !Arc::ptr_eq(source, &mesh.source))
            {
                assert!(
                    self.animated.len() < 128,
                    "bounded Core animation instances"
                );
                self.animated.insert(
                    node.id.clone(),
                    (mesh.source.clone(), AnimationPlayer::new(model)?),
                );
            }
            let (_, player) = self.animated.get_mut(&node.id).unwrap();
            incant_render_data::animation_sample::sample_committed(player, mesh)?;
            assert!(player.globals().iter().all(|m| m.is_finite()));
            self.sampled_rigid_poses += 1;
        }
        Ok(())
    }
}
impl RenderInputConsumer for Consumer {
    const SUPPORTS_UI: bool = true;
    type Settings = ();
    type Error = std::io::Error;
    fn prepare(
        input: &SceneInput,
        camera: &str,
        ui: Option<&Arc<UiInput>>,
        (): (),
    ) -> Result<Self, Self::Error> {
        let mut out = Self {
            ui: Some(ui.expect("Core HUD").clone()),
            ..Self::default()
        };
        out.sample(input, camera)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        assert!(
            !out.animated.is_empty(),
            "actual animated Core actors required"
        );
        Ok(out)
    }
    fn accept(
        &mut self,
        _: &SceneInput,
        _: &str,
        _: UpdateKind,
        _: Option<&Arc<UiInput>>,
    ) -> Result<(), Self::Error> {
        panic!("pose host must use canonical admission")
    }
    fn admit_ui(&mut self, ui: &Arc<UiInput>) -> Result<(), Self::Error> {
        self.ui = Some(ui.clone());
        Ok(())
    }
}
impl PoseConsumer for Consumer {
    fn admit_pose(
        &mut self,
        input: &SceneInput,
        camera: &str,
        _: UpdateKind,
        _: u64,
        ui: Option<&Arc<UiInput>>,
    ) -> Result<(), Self::Error> {
        self.sample(input, camera)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        self.ui = Some(ui.expect("Core HUD").clone());
        Ok(())
    }
    fn present_pose(
        &mut self,
        input: &SceneInput,
        camera: &str,
        _: PoseReceipt,
        ui: Option<&Arc<UiInput>>,
    ) -> Result<(), Self::Error> {
        assert!(Arc::ptr_eq(
            self.ui.as_ref().unwrap(),
            ui.expect("accepted HUD")
        ));
        self.sample(input, camera)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        assert!(
            ui.unwrap().widgets.iter().any(|w| !w.glyphs.is_empty()),
            "actual shaped HUD required"
        );
        self.glyph_displays += 1;
        self.displays += 1;
        Ok(())
    }
}

fn settings(args: &[String]) -> NativeRenderSettings {
    let Some(session::Settings::Cpu(settings)) = session::settings(args).unwrap() else {
        panic!("CPU host only")
    };
    settings
}

#[test]
#[ignore = "manual fresh authored Core producer and bounded INCANT_CORE_POSE_FIXTURE required"]
fn actual_core_2156_route_preserves_gameplay_and_complete_pcm_with_hud_and_rigid_animation()
-> session::Result<()> {
    let fixture: Fixture = serde_json::from_slice(&bounded(
        &std::env::var("INCANT_CORE_POSE_FIXTURE")?,
        16 * 1024,
    )?)?;
    assert_eq!(fixture.format, "incant-core-pose-cpu-v1");
    let output = Path::new(&fixture.output);
    std::fs::create_dir(output)?; // private create-only output, no overwritten prior outcome
    let args = package::expand(vec![
        "--package".into(),
        fixture.data.clone(),
        "--ticks".into(),
        "2156".into(),
        "--output".into(),
        output.join("ordinary.pcm").to_str().unwrap().into(),
        "--ui-viewport".into(),
        "640x480".into(),
    ])?;
    let scene = CookedScene::from_bytes(&bounded(&args[0], 128 * 1024 * 1024)?)?;
    assert_eq!(scene.tick_rate(), 60);
    assert!(scene.has_ui() && scene.has_text());
    let replay = InputReplay::from_text(
        std::str::from_utf8(&bounded(
            &fixture.recording,
            incant_input::MAX_RECORDING_BYTES,
        )?)?,
        0,
        60,
    )?;
    assert_eq!(replay.end_tick(), 2156);
    assert!(!replay.has_ui(), "same frozen physical-input Core route");
    let observations = bounded(&fixture.observations, incant_observe::MAX_PLAN_BYTES)?;
    let mut a_plan = incant_observe::Plan::parse(&observations, 2156)?;
    let mut b_plan = incant_observe::Plan::parse(&observations, 2156)?;
    let actions: incant_input::InputActions =
        serde_json::from_slice(&bounded(&args[4], 256 * 1024)?)?;
    let program = embedded::load()?; // exact trusted cook-generated include, no runtime bytecode loader
    assert_eq!(
        program.metadata().bytecode_sha256,
        fixture.expected_bytecode_sha256
    );
    let mut ordinary = HostedOfflineGameSession::<NativeRenderHost>::new(
        scene.clone(),
        program.clone(),
        actions.clone(),
        startup::clips(&args[1], &args[2])?,
        48000,
        settings(&args),
    )?;
    let mut pose = HostedOfflineGameSession::<PoseRenderHost<Consumer>>::new(
        scene,
        program,
        actions,
        startup::clips(&args[1], &args[2])?,
        48000,
        settings(&args),
    )?;
    assert!(
        pose.world()?.entity_count() >= 10_000,
        "full workload, not a static probe"
    );
    ordinary.enable_restart_requests()?;
    pose.enable_restart_requests()?;
    a_plan.observe(ordinary.world()?, ordinary.input()?)?;
    b_plan.observe(pose.world()?, pose.input()?)?;
    let mut a_pcm = PcmOutput::new(&output.join("ordinary.pcm"), 1_724_800)?;
    let mut b_pcm = PcmOutput::new(&output.join("pose.pcm"), 1_724_800)?;
    let mut a = [0.; 1600];
    let mut b = [0.; 1600];
    for step in 1..=2156 {
        assert_eq!(ordinary.next_output_frames()?, 800);
        assert_eq!(pose.next_output_frames()?, 800);
        let events = replay.events_at(step)?;
        ordinary.tick_with_ui(events, &[], &mut a)?;
        pose.tick_with_ui(events, &[], &mut b)?;
        assert!(
            a.iter().zip(&b).all(|(a, b)| a.to_bits() == b.to_bits()),
            "complete PCM mismatch at {step}"
        );
        assert_eq!(ordinary.input()?, pose.input()?, "input mismatch at {step}");
        assert_eq!(
            ordinary.take_haptics()?,
            pose.take_haptics()?,
            "haptics mismatch at {step}"
        );
        assert!(ordinary.take_session_request()?.is_none());
        assert!(
            pose.take_session_request()?.is_none(),
            "the original win route does not restart"
        );
        a_plan.observe_at(step, ordinary.world()?, ordinary.input()?)?;
        b_plan.observe_at(step, pose.world()?, pose.input()?)?;
        a_pcm.append(&a)?;
        b_pcm.append(&b)?;
        let before = pose.rendered_frames()?;
        let ui = pose.host()?.ui_input().unwrap().clone();
        // Exact endpoints plus an interior pose; all are sampled from the same
        // accepted simulation tick, with no further VM/input/audio operation.
        for fraction in [0., 0.5, 1.] {
            pose.present_host(PoseRequest::Display { fraction })?;
            assert!(Arc::ptr_eq(&ui, pose.host()?.ui_input().unwrap()));
            assert_eq!(pose.world()?.tick(), step);
            assert_eq!(pose.rendered_frames()?, before);
            compare::canonical(ordinary.host()?.render_input(), pose.host()?.committed());
        }
    }
    assert_eq!(ordinary.world()?.snapshot(), pose.world()?.snapshot());
    for node in ordinary.world()?.spatial_view()?.nodes() {
        assert_eq!(
            ordinary.world()?.animation_state(node.id),
            pose.world()?.animation_state(node.id)
        );
    }
    assert_eq!(
        ordinary.world()?.audio_nodes().len(),
        pose.world()?.audio_nodes().len()
    );
    for (node, position) in ordinary.world()?.audio_nodes() {
        let other = pose
            .world()?
            .audio_nodes()
            .find(|(other, _)| other.entity == node.entity)
            .unwrap();
        assert_eq!(position, other.1);
        if node.source.is_some() {
            assert_eq!(
                ordinary.source_state(node.entity)?,
                pose.source_state(node.entity)?
            );
        }
    }
    assert_eq!(
        ordinary.host()?.ui().unwrap().checkpoint_bytes()?,
        pose.host()?.ui().unwrap().checkpoint_bytes()?
    );
    compare::ui(
        ordinary.host()?.ui_input().unwrap(),
        pose.host()?.ui_input().unwrap(),
    );
    let render_input = session::input_report(ordinary.host()?.render_input())?;
    assert_eq!(
        render_input,
        session::input_report(pose.host()?.committed())?
    );
    let a_report = a_plan.finish(ordinary.world()?)?;
    let b_report = b_plan.finish(pose.world()?)?;
    assert!(a_report.passed && b_report.passed);
    assert_eq!(
        serde_json::to_value(&a_report)?,
        serde_json::to_value(&b_report)?
    );
    assert_eq!(
        serde_json::to_value(ordinary.logs()?)?,
        serde_json::to_value(pose.logs()?)?
    );
    assert_eq!(a_pcm.frames(), 1_724_800);
    assert_eq!(b_pcm.frames(), a_pcm.frames());
    assert_eq!(a_pcm.sha256(), fixture.expected_pcm_sha256);
    assert_eq!(b_pcm.sha256(), fixture.expected_pcm_sha256);
    let consumer = pose.host()?.consumer();
    assert_eq!(consumer.displays, 2156 * 3);
    assert_eq!(consumer.glyph_displays, consumer.displays);
    assert!(consumer.sampled_rigid_poses > consumer.displays);
    let report = serde_json::json!({"format":"incant-core-pose-cpu-result-v1","passed":true,"steps":2156,
        "ordinary":a_report,"pose":b_report,"render_input":render_input,"logs":ordinary.logs()?,
        "full_world_snapshot_equal":true,"animation_states_equal":true,"audio_states_equal":true,"ui_checkpoint_and_draw_input_equal":true,
        "complete_pcm_sha256":a_pcm.sha256(),"pcm_frames":a_pcm.frames(),
        "pose_displays":consumer.displays,"glyph_displays":consumer.glyph_displays,"sampled_rigid_poses":consumer.sampled_rigid_poses,
        "scope":"actual CPU composed hosts and sampled poses; no GPU/window/appearance/pacing/performance qualification"});
    a_pcm.publish(&output.join("ordinary.pcm"))?;
    b_pcm.publish(&output.join("pose.pcm"))?;
    OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(output.join("result.json"))?
        .write_all(&serde_json::to_vec_pretty(&report)?)?;
    println!("{}", serde_json::to_string(&report)?);
    Ok(())
}
