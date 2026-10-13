//! Handoff 0067 GPU acceptance cases for opt-in retained dynamic resolution.
//!
//! Ignored by default. The packet's guarded runner executes each case once, in
//! its own bounded process group, under the root GPU grant. Every case's frame
//! attempts are frozen in tools/cases.json and re-checked by `Budget`. Raw
//! receipts are printed as `DRS-RECEIPT` lines before any verdict.
use incant_0067_drs::fixture::{self, Fixture, Game, Projection};
use incant_0067_drs::pixels::{self, Rgba};
use incant_0067_drs::probe::{self, Event, Log, Probe, Targets};
use incant_0067_drs::*;
use incant_input::{InputEvent, KeyCode};
use incant_render_data::{SceneInput, UiInput};
use incant_render_game::RenderInputConsumer;
use incant_render_gpu_host::{
    GpuRenderConsumer, GpuRenderSettings, ResolutionTargets, TargetProvider,
};
use incant_render_native::UpdateKind;
use incant_ui::UiEvent;
use serde_json::{Value, json};
use std::sync::{Arc, atomic::Ordering};
use std::time::Instant;

const OUT: [u32; 2] = [959, 539];
type Scaled = ResolutionTargets<Targets>;

fn viewport_json(v: &Viewport) -> Value {
    json!({"rect": v.rect, "corner_radii": v.corner_radii, "canvas_srgb": v.canvas_srgb})
}

struct Run<P: TargetProvider> {
    instance: wgpu::Instance,
    renderer: Arc<Renderer>,
    probe: Arc<Probe>,
    game: Game<P>,
}
fn disabled(
    fx: &Fixture,
    log: &Arc<Log>,
    name: &'static str,
    size: [u32; 2],
    viewport: Option<Viewport>,
) -> Run<Targets> {
    let (instance, renderer) = renderer().unwrap();
    let probe = Probe::new(name, &renderer, log, size, viewport);
    let settings = GpuRenderSettings {
        renderer: Arc::clone(&renderer),
        targets: probe.targets(),
    };
    let game = fx.game(size, settings).unwrap();
    Run {
        instance,
        renderer,
        probe,
        game,
    }
}
fn scaled(
    fx: &Fixture,
    log: &Arc<Log>,
    name: &'static str,
    size: [u32; 2],
    viewport: Option<Viewport>,
    config: ResolutionConfig,
) -> Run<Scaled> {
    let (instance, renderer) = renderer().unwrap();
    let probe = Probe::new(name, &renderer, log, size, viewport);
    let settings = GpuRenderSettings {
        renderer: Arc::clone(&renderer),
        targets: probe.targets(),
    }
    .with_dynamic_resolution(config)
    .unwrap();
    let game = fx.game(size, settings).unwrap();
    Run {
        instance,
        renderer,
        probe,
        game,
    }
}
impl<P: TargetProvider> Run<P> {
    fn tick(
        &mut self,
        log: &Log,
        j: u64,
        keys: &[InputEvent],
        ui: &[UiEvent],
        budget: &mut Budget,
        label: &str,
    ) -> std::result::Result<(), String> {
        budget.attempt(label, true);
        let frames = self.game.next_output_frames().map_err(|e| e.to_string())?;
        let mut pcm = vec![0.; frames * 2];
        log.begin_tick(j);
        let result = if ui.is_empty() {
            self.game.tick(keys, &mut pcm)
        } else {
            self.game.tick_with_ui(keys, ui, &mut pcm)
        };
        log.end_tick(j);
        result.map_err(|e| e.to_string())
    }
    fn receipt(&self) -> Option<ResolutionFrame> {
        self.game
            .host()
            .ok()
            .and_then(|h| h.consumer().resolution_frame())
    }
    fn discarded(&self) -> u64 {
        self.game
            .host()
            .map_or(u64::MAX, |h| h.consumer().discarded_resolution_samples())
    }
    fn ui(&self) -> Option<Arc<UiInput>> {
        self.game.host().ok().and_then(|h| h.ui_input().cloned())
    }
    fn observe(&self, tick: u64) -> Value {
        let registry = registry_json(&self.instance);
        match self.game.host() {
            Ok(host) => {
                let c = host.consumer();
                json!({"probe": self.probe.name, "tick": tick, "game_failed": false,
                    "consumer_failed": c.is_failed(), "accepted_frames": c.accepted_frames(),
                    "receipt": receipt_json(c.resolution_frame().as_ref()),
                    "discarded": c.discarded_resolution_samples(),
                    "acquires": self.probe.acquires.load(Ordering::SeqCst),
                    "presents": self.probe.presents.load(Ordering::SeqCst),
                    "registry": registry})
            }
            Err(e) => json!({"probe": self.probe.name, "tick": tick, "game_failed": true,
                "host_error": e.to_string(), "registry": registry}),
        }
    }
}
fn receipt_is(
    receipt: Option<&ResolutionFrame>,
    sequence: u64,
    n: u8,
    output: [u32; 2],
    internal_size: [u32; 2],
    pixel: bool,
) -> bool {
    receipt.is_some_and(|r| {
        r.sequence() == sequence
            && sixteenths(r.scale()) == n
            && r.output_size() == output
            && r.internal_size() == internal_size
            && r.pixel_perfect() == pixel
    })
}
fn fire_phases(events: &[Event], probe: &str, frame: u64) -> Vec<u32> {
    events
        .iter()
        .filter(|e| e.kind == "fire" && e.probe == probe && e.frame == frame)
        .map(|e| e.phase)
        .collect()
}
fn widget_center(ui: Option<&UiInput>, n: u128) -> [f32; 2] {
    let w = ui
        .unwrap()
        .widgets
        .iter()
        .find(|w| w.id == fixture::id(n))
        .unwrap();
    [
        (w.visible[0] + w.visible[2]) / 2.,
        (w.visible[1] + w.visible[3]) / 2.,
    ]
}
fn pointer<P: TargetProvider>(game: &Game<P>, position: [f32; 2]) -> Vec<UiEvent> {
    let token = game.host().unwrap().ui().unwrap().pointer_token().unwrap();
    vec![
        UiEvent::Pointer {
            position,
            pressed: true,
            target: token.clone(),
        },
        UiEvent::Pointer {
            position,
            pressed: false,
            target: token,
        },
    ]
}
fn guest_logs<P: TargetProvider>(game: &Game<P>) -> Vec<(u64, String)> {
    game.logs()
        .unwrap()
        .entries()
        .iter()
        .map(|e| (e.tick, e.message.clone()))
        .collect()
}
fn focus<P: TargetProvider>(game: &Game<P>) -> Option<String> {
    game.host()
        .unwrap()
        .ui()
        .unwrap()
        .focus()
        .map(|f| fixture::ulid_text(f.widget()))
}
fn key(down: bool) -> InputEvent {
    InputEvent::Key {
        code: KeyCode::Space,
        down,
    }
}
fn u(v: &Value) -> u64 {
    v.as_u64().unwrap_or(u64::MAX)
}

#[test]
#[ignore = "requires a native GPU; handoff 0067 bounded execution"]
fn fixed_scales_ui_offset_viewport() {
    const CASE: &str = "fixed_scales_ui_offset_viewport";
    let mut budget = Budget::new(CASE, 8);
    let mut v = Verdict::default();
    let log = Log::new();
    let fx = Fixture::new(Projection::Perspective, true).unwrap();
    let vp = offset_viewport();
    emit(
        CASE,
        "fixture",
        json!({"scene_sha256": fx.scene_sha256(), "projection": fx.projection.name(), "ui": fx.ui,
            "output": OUT, "viewport": viewport_json(&vp),
            "configs": {"16": config_json(&fixed(16)), "12": config_json(&fixed(12)), "8": config_json(&fixed(8))}}),
    );
    // Disabled control.
    let mut d = disabled(&fx, &log, "D", OUT, Some(vp));
    emit(CASE, "adapter", adapter_json(&d.renderer));
    d.tick(&log, 1, &[], &[], &mut budget, "D tick 1").unwrap();
    let d1 = d.probe.pixels().unwrap();
    let d1_ui = d.ui();
    let d1_registry = registry_json(&d.instance);
    emit(
        CASE,
        "observation",
        json!({"run": "D", "obs": d.observe(1), "frame_sha256": d1.sha256(),
            "ui": pixels::ui_json(d1_ui.as_deref())}),
    );
    v.check(
        d.receipt().is_none(),
        "disabled control publishes no receipt",
    );
    // The UI contract maps the logical UI viewport onto the camera viewport rect.
    let boxes = pixels::ui_boxes(d1_ui.as_deref(), vp.rect);
    let opaque = pixels::opaque_regions(d1_ui.as_deref(), vp.rect);
    // Fixed 100%, 75% and 50% controls: identical policy fields, own renderers.
    let mut runs: Vec<(u8, Run<Scaled>, Rgba, Value)> = Vec::new();
    for (n, name) in [(16u8, "F16"), (12, "F12"), (8, "F8")] {
        let mut run = scaled(&fx, &log, name, OUT, Some(vp), fixed(n));
        run.tick(&log, 1, &[], &[], &mut budget, name).unwrap();
        let frame = run.probe.pixels().unwrap();
        let ui = run.ui();
        let registry = registry_json(&run.instance);
        let receipt = run.receipt();
        v.check(
            receipt_is(receipt.as_ref(), 1, n, OUT, internal(OUT, n), false),
            format!(
                "{name} receipt: sequence 1, scale {n}/16, physical output, ceil internal size"
            ),
        );
        v.check(
            pixels::ui_json(ui.as_deref()) == pixels::ui_json(d1_ui.as_deref()),
            format!("{name} UI layout boxes equal the disabled control"),
        );
        let outside = pixels::compare_outside(&d1, &frame, vp.rect, vp.corner_radii);
        v.check(
            u(&outside["mismatched"]) == 0 && u(&outside["compared"]) > 10_000,
            format!("{name} chrome outside the physical viewport equals the control"),
        );
        let controls = pixels::compare_regions(&d1, &frame, &opaque);
        v.check(
            u(&controls["mismatched"]) == 0 && u(&controls["compared"]) > 10_000,
            format!("{name} opaque UI control interiors equal the control"),
        );
        emit(
            CASE,
            "observation",
            json!({"run": name, "obs": run.observe(1), "frame_sha256": frame.sha256(),
                "ui": pixels::ui_json(ui.as_deref()), "outside_viewport": outside,
                "opaque_ui_interiors": controls}),
        );
        runs.push((n, run, frame, registry));
    }
    let exact = pixels::equal_bytes(&d1, &runs[0].2);
    v.check(
        exact["identical"] == true,
        "fixed 100% equals the disabled control byte for byte",
    );
    emit(CASE, "disabled_vs_fixed_100", exact);
    // Disabled references at the internal sizes from the same tick-1 world.
    let mut cpu = fx.cpu_game(OUT).unwrap();
    let mut pcm = vec![0.; cpu.next_output_frames().unwrap() * 2];
    cpu.tick(&[], &mut pcm).unwrap();
    let (_reference_instance, reference_renderer) = renderer().unwrap();
    let host = cpu.host().unwrap();
    let input = host.render_input().input();
    let label = host.render_input().camera_label();
    let mut reference_scene = reference_renderer.prepare_retained(input, label).unwrap();
    for (n, _, frame, _) in runs.iter().skip(1) {
        let size = internal(OUT, *n);
        let texture = probe::target_texture(&reference_renderer, size);
        let view = texture.create_view(&Default::default());
        budget.attempt(&format!("reference {n}"), true);
        reference_scene
            .submit(
                &reference_renderer,
                input,
                RetainedTarget {
                    view: &view,
                    format: FORMAT,
                    width: size[0],
                    height: size[1],
                    viewport: Some(scaled_viewport(*n)),
                },
            )
            .unwrap();
        let reference = pixels::read(&reference_renderer, &texture).unwrap();
        let mapping = pixels::compare_mapping(
            frame,
            &reference,
            u32::from(*n),
            vp.rect,
            vp.corner_radii,
            &boxes,
        );
        v.check(
            u(&mapping["mismatched"]) == 0 && u(&mapping["compared"]) > 100_000,
            format!("{n}/16 frame equals nearest-texel expansion of its internal-size reference"),
        );
        emit(
            CASE,
            "mapping",
            json!({"scale16": n, "reference_viewport": viewport_json(&scaled_viewport(*n)),
                "reference_sha256": reference.sha256(), "comparison": mapping}),
        );
    }
    // Physical focus and hit: one pointer press/release at the Resume centre.
    let center = widget_center(d1_ui.as_deref(), fixture::RESUME);
    let events = pointer(&d.game, center);
    d.tick(&log, 2, &[], &events, &mut budget, "D pointer")
        .unwrap();
    let d2 = d.probe.pixels().unwrap();
    let d2_ui = d.ui();
    let d2_registry = registry_json(&d.instance);
    let (_, f8, _, f8_registry_1) = &mut runs[2];
    let f8_registry_1 = f8_registry_1.clone();
    let events = pointer(&f8.game, center);
    f8.tick(&log, 2, &[], &events, &mut budget, "F8 pointer")
        .unwrap();
    let f8_2 = f8.probe.pixels().unwrap();
    let f8_2_ui = f8.ui();
    let f8_registry_2 = registry_json(&f8.instance);
    let activated = vec![(2, "activated".to_string())];
    let resume = Some(fixture::ulid_text(fixture::id(fixture::RESUME)));
    v.check(
        guest_logs(&d.game) == activated && guest_logs(&f8.game) == activated,
        "the physical pointer hit activates Resume once in both",
    );
    v.check(
        focus(&d.game) == resume && focus(&f8.game) == resume,
        "the pointer press focuses Resume in both",
    );
    v.check(
        pixels::ui_json(d2_ui.as_deref()) == pixels::ui_json(f8_2_ui.as_deref()),
        "focused UI layout equals the control",
    );
    let focused = pixels::compare_regions(
        &d2,
        &f8_2,
        &pixels::opaque_regions(d2_ui.as_deref(), vp.rect),
    );
    v.check(
        u(&focused["mismatched"]) == 0,
        "focused opaque controls (with the focus ring) equal the control",
    );
    v.check(
        receipt_is(f8.receipt().as_ref(), 2, 8, OUT, internal(OUT, 8), false),
        "F8 second receipt keeps scale and target size",
    );
    for (name, a, b) in [
        ("D", &d1_registry, &d2_registry),
        ("F8", &f8_registry_1, &f8_registry_2),
    ] {
        v.check(
            a["render_pipelines"] == b["render_pipelines"] && a["textures"] == b["textures"],
            format!("{name}: an unchanged-size frame creates no pipelines or textures"),
        );
    }
    emit(
        CASE,
        "pointer",
        json!({"center": center, "D": {"obs": d.observe(2), "logs": guest_logs(&d.game),
            "focus": focus(&d.game), "registry_t1": d1_registry, "registry_t2": d2_registry,
            "frame_sha256": d2.sha256()},
            "F8": {"obs": f8.observe(2), "logs": guest_logs(&f8.game), "focus": focus(&f8.game),
            "registry_t1": f8_registry_1, "registry_t2": f8_registry_2, "frame_sha256": f8_2.sha256()},
            "focused_opaque_interiors": focused, "ui_t2": pixels::ui_json(d2_ui.as_deref())}),
    );
    emit(CASE, "events", log.json());
    budget.finish();
    v.finish(CASE);
}

/// Corrective re-check of the first case's 75% and 50% world mapping with UI
/// exclusion in physical pixels (the UI contract maps the logical UI viewport onto
/// the camera viewport rect). The composed-host frames are not rendered again:
/// the appearance run retained them as PNGs whose RGBA hashes equal the first
/// case's disabled, 12/16 and 8/16 receipts. Only the two disabled internal-size
/// references are new submissions.
const CASE1_D_SHA: &str = "999e429206f78a2cd2b736daa1cf1443540b041738b3dbfd3c22ee44ed9a0b49";
const CASE1_F12_SHA: &str = "e5d21bc18690100c949a23002a0e95b3d30399307ab59077c13b9f52d1cbd47b";
const CASE1_F8_SHA: &str = "243acab0e116d7332f15f94eb94ae5e386139b9ddda127f75421e9eae701ee75";
#[test]
#[ignore = "requires a native GPU; handoff 0067 bounded execution"]
fn fixed_scales_reference_recheck() {
    const CASE: &str = "fixed_scales_reference_recheck";
    let mut budget = Budget::new(CASE, 2);
    let mut v = Verdict::default();
    let frames_dir =
        std::path::PathBuf::from(std::env::var_os("INCANT_0067_FRAMES").expect("frames dir"));
    let out_dir =
        std::path::PathBuf::from(std::env::var_os("INCANT_0067_OUT").expect("output dir"));
    std::fs::create_dir(&out_dir).unwrap();
    let fx = Fixture::new(Projection::Perspective, true).unwrap();
    let vp = offset_viewport();
    let load = |name: &str, sha: &str| {
        let frame = pixels::read_png(&frames_dir.join(name)).unwrap();
        (frame.sha256() == sha, frame)
    };
    let (d_ok, d) = load("a1-offset-ui-disabled.png", CASE1_D_SHA);
    let (f12_ok, f12) = load("a3-offset-ui-75.png", CASE1_F12_SHA);
    let (f8_ok, f8) = load("a4-offset-ui-50.png", CASE1_F8_SHA);
    v.check(
        d_ok && f12_ok && f8_ok,
        "retained frames are byte-identical to the first case's receipts",
    );
    // UI layout from the CPU observer composition of the same scene (no GPU).
    let mut cpu = fx.cpu_game(OUT).unwrap();
    let mut pcm = vec![0.; cpu.next_output_frames().unwrap() * 2];
    cpu.tick(&[], &mut pcm).unwrap();
    let host = cpu.host().unwrap();
    let ui = host.ui_input().cloned();
    let boxes = pixels::ui_boxes(ui.as_deref(), vp.rect);
    let opaque = pixels::opaque_regions(ui.as_deref(), vp.rect);
    let interiors: Vec<Value> = [(&f12, 12), (&f8, 8)]
        .iter()
        .map(|(frame, n)| {
            let c = pixels::compare_regions(&d, frame, &opaque);
            v.check(
                u(&c["mismatched"]) == 0 && u(&c["compared"]) > 10_000,
                format!("{n}/16 opaque UI control interiors equal the disabled control"),
            );
            json!({"scale16": n, "comparison": c})
        })
        .collect();
    emit(
        CASE,
        "ui",
        json!({"scene_sha256": fx.scene_sha256(), "viewport": viewport_json(&vp),
            "ui": pixels::ui_json(ui.as_deref()), "physical_ui_boxes": boxes,
            "opaque_interiors": interiors, "retained": {"D": d.sha256(), "F12": f12.sha256(), "F8": f8.sha256()}}),
    );
    let (_reference_instance, reference_renderer) = renderer().unwrap();
    emit(CASE, "adapter", adapter_json(&reference_renderer));
    let input = host.render_input().input();
    let label = host.render_input().camera_label();
    let mut reference_scene = reference_renderer.prepare_retained(input, label).unwrap();
    for (n, frame) in [(12u8, &f12), (8, &f8)] {
        let size = internal(OUT, n);
        let texture = probe::target_texture(&reference_renderer, size);
        let view = texture.create_view(&Default::default());
        budget.attempt(&format!("reference {n}"), true);
        reference_scene
            .submit(
                &reference_renderer,
                input,
                RetainedTarget {
                    view: &view,
                    format: FORMAT,
                    width: size[0],
                    height: size[1],
                    viewport: Some(scaled_viewport(n)),
                },
            )
            .unwrap();
        let reference = pixels::read(&reference_renderer, &texture).unwrap();
        reference
            .write_png(&out_dir.join(format!("reference-{n}.png")))
            .unwrap();
        let mapping = pixels::compare_mapping(
            frame,
            &reference,
            u32::from(n),
            vp.rect,
            vp.corner_radii,
            &boxes,
        );
        v.check(
            u(&mapping["mismatched"]) == 0 && u(&mapping["compared"]) > 100_000,
            format!("{n}/16 frame equals nearest-texel expansion of its internal-size reference"),
        );
        emit(
            CASE,
            "mapping",
            json!({"scale16": n, "reference_viewport": viewport_json(&scaled_viewport(n)),
                "reference_sha256": reference.sha256(), "comparison": mapping}),
        );
    }
    budget.finish();
    v.finish(CASE);
}

/// The separate adaptive run with real delivered measurements: the realistic
/// policy, one explicit test-driven dispatch (blocking poll) after every frame,
/// no sleeps. The same witnessed replay predicts every receipt scale.
#[test]
#[ignore = "requires a native GPU; handoff 0067 bounded execution"]
fn adaptive_paced_explicit_dispatch() {
    const CASE: &str = "adaptive_paced_explicit_dispatch";
    const TICKS: u64 = 4;
    let mut budget = Budget::new(CASE, 4);
    let mut v = Verdict::default();
    let log = Log::new();
    let fx = Fixture::new(Projection::Perspective, true).unwrap();
    let config = realistic();
    emit(
        CASE,
        "fixture",
        json!({"scene_sha256": fx.scene_sha256(), "projection": fx.projection.name(), "ui": fx.ui,
            "output": OUT, "viewport": null, "config": config_json(&config),
            "cadence": "one tick, then one explicit blocking poll; no sleeps"}),
    );
    let mut run = scaled(&fx, &log, "P", OUT, None, config);
    emit(CASE, "adapter", adapter_json(&run.renderer));
    let ui0 = pixels::ui_json(run.ui().as_deref());
    let mut observations = Vec::new();
    for j in 1..=TICKS {
        run.tick(&log, j, &[], &[], &mut budget, "paced").unwrap();
        log.poll(&run.renderer, probe::PHASE_TEST_POLL).unwrap();
        observations.push(run.observe(j));
    }
    let events = log.events();
    let (predicted, samples, ambiguous) = replay(&events, "P", &config);
    let observed: Vec<u8> = observations
        .iter()
        .map(|o| o["receipt"]["scale16"].as_u64().unwrap_or(0) as u8)
        .collect();
    v.check(
        samples.len() as u64 == TICKS - 1,
        "every frame's completion was delivered and consumed by the next frame",
    );
    v.check(
        predicted.contains(&observed),
        "receipt scales follow the delivered measurements, single slot and policy",
    );
    v.check(
        observations.iter().all(|o| o["discarded"] == 0),
        "no sample of one owner at one extent is discarded",
    );
    v.check(
        observations.iter().all(|o| {
            let n = o["receipt"]["scale16"].as_u64().unwrap_or(0) as u8;
            o["receipt"]["internal"] == json!(internal(OUT, n))
                && o["receipt"]["output"] == json!(OUT)
        }),
        "every receipt's internal size is ceil(output x scale) at physical output",
    );
    v.check(
        pixels::ui_json(run.ui().as_deref()) == ui0,
        "UI layout stays physical through scale changes",
    );
    emit(
        CASE,
        "replay",
        json!({"observations": observations, "predicted_scale16": predicted,
            "observed_scale16": observed, "samples": samples, "ambiguous_samples": ambiguous}),
    );
    emit(CASE, "events", log.json());
    budget.finish();
    v.finish(CASE);
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Class {
    Under,
    InBand,
    Over,
    Ambiguous,
}
fn classify(lo: f64, hi: f64, c: &ResolutionConfig) -> Class {
    let (low, high) = (
        c.target_ms * (1. - c.deadband),
        c.target_ms * (1. + c.deadband),
    );
    if hi < low {
        Class::Under
    } else if lo > high {
        Class::Over
    } else if lo >= low && hi <= high {
        Class::InBand
    } else {
        Class::Ambiguous
    }
}
/// Every class a value in [lo, hi] can take (controller: over iff ms > high,
/// under iff ms < low, otherwise in band).
fn admitted(lo: f64, hi: f64, c: &ResolutionConfig) -> Vec<Class> {
    let (low, high) = (
        c.target_ms * (1. - c.deadband),
        c.target_ms * (1. + c.deadband),
    );
    let mut out = Vec::new();
    if lo < low {
        out.push(Class::Under);
    }
    if hi >= low && lo <= high {
        out.push(Class::InBand);
    }
    if hi > high {
        out.push(Class::Over);
    }
    out
}
/// The documented policy, written out: cooldown, sustained overload/recovery
/// counts, deadband reset, bounded slew and clamping.
#[derive(Clone, PartialEq)]
struct Model {
    c: ResolutionConfig,
    scale: u8,
    overload: u16,
    recovery: u16,
    cooldown: u16,
}
impl Model {
    fn new(c: ResolutionConfig) -> Self {
        Self {
            scale: sixteenths(c.initial),
            c,
            overload: 0,
            recovery: 0,
            cooldown: 0,
        }
    }
    fn observe(&mut self, class: Class) {
        if self.cooldown > 0 {
            self.cooldown -= 1;
            return;
        }
        match class {
            Class::Over => {
                self.recovery = 0;
                self.overload += 1;
                if self.overload >= self.c.overload_samples {
                    self.overload = 0;
                    self.change(self.scale.saturating_sub(self.c.slew));
                }
            }
            Class::Under => {
                self.overload = 0;
                self.recovery += 1;
                if self.recovery >= self.c.recovery_samples {
                    self.recovery = 0;
                    self.change(self.scale.saturating_add(self.c.slew));
                }
            }
            Class::InBand | Class::Ambiguous => {
                self.overload = 0;
                self.recovery = 0;
            }
        }
    }
    fn change(&mut self, candidate: u8) {
        let next = candidate.clamp(sixteenths(self.c.minimum), sixteenths(self.c.maximum));
        if next != self.scale {
            self.scale = next;
            self.cooldown = self.c.cooldown_samples;
        }
    }
}
/// Replays one owner at one extent from the event log: witnessed dispatch
/// moves Pending(k) to Ready(k); the take before each acquire consumes Ready;
/// the receipt uses the model's scale; a frame registers only into an Idle slot.
/// A sample whose latency interval admits several classes branches the model.
/// Returns every admitted predicted scale sequence and the consumed samples.
fn replay(
    events: &[Event],
    probe: &str,
    c: &ResolutionConfig,
) -> (Vec<Vec<u8>>, Vec<Value>, usize) {
    #[derive(Clone, Copy, PartialEq)]
    enum Slot {
        Idle,
        Pending(u64),
        Ready(u64),
    }
    let at = |kind: &str, frame: u64| {
        events
            .iter()
            .find(|e| e.kind == kind && e.probe == probe && e.frame == frame)
            .map(|e| e.at_ns)
    };
    let tick_of = |frame: u64| {
        events
            .iter()
            .find(|e| e.kind == "acquire" && e.probe == probe && e.frame == frame)
            .map(|e| u64::from(e.phase / 10))
    };
    let tick_call = |tick: u64| {
        events
            .iter()
            .find(|e| e.kind == "tick-call" && e.frame == tick)
            .map(|e| e.at_ns)
    };
    let mut branches = vec![(Model::new(*c), Vec::new())];
    let mut slot = Slot::Idle;
    let mut samples = Vec::new();
    let mut ambiguous = 0;
    for e in events {
        if e.probe != probe {
            continue;
        }
        match e.kind {
            "fire" if slot == Slot::Pending(e.frame) => slot = Slot::Ready(e.frame),
            "acquire" => {
                if let Slot::Ready(k) = slot {
                    let fire = events
                        .iter()
                        .position(|f| f.kind == "fire" && f.probe == probe && f.frame == k)
                        .unwrap();
                    let next = events.get(fire + 1).map_or(u64::MAX, |n| n.at_ns);
                    let lo = (events[fire].at_ns - at("acquire", k).unwrap()) as f64 / 1e6;
                    let hi = (next - tick_call(tick_of(k).unwrap()).unwrap()) as f64 / 1e6;
                    let classes = admitted(lo, hi, c);
                    ambiguous += usize::from(classes.len() > 1);
                    let mut next_branches: Vec<(Model, Vec<u8>)> = Vec::new();
                    for (model, predicted) in &branches {
                        for class in &classes {
                            let mut m = model.clone();
                            m.observe(*class);
                            let branch = (m, predicted.clone());
                            if !next_branches.contains(&branch) {
                                next_branches.push(branch);
                            }
                        }
                    }
                    branches = next_branches;
                    samples.push(json!({"frame": k, "consumed_before_frame": e.frame,
                        "lo_ms": lo, "hi_ms": hi, "class": format!("{:?}", classify(lo, hi, c)),
                        "admitted": classes.iter().map(|x| format!("{x:?}")).collect::<Vec<_>>()}));
                    slot = Slot::Idle;
                }
                for (model, predicted) in &mut branches {
                    predicted.push(model.scale);
                }
            }
            "witness-registered" if slot == Slot::Idle => slot = Slot::Pending(e.frame),
            _ => {}
        }
    }
    (
        branches.into_iter().map(|(_, p)| p).collect(),
        samples,
        ambiguous,
    )
}

#[test]
#[ignore = "requires a native GPU; handoff 0067 bounded execution"]
fn adaptive_natural_cadence() {
    const CASE: &str = "adaptive_natural_cadence";
    const TICKS: u64 = 7;
    let mut budget = Budget::new(CASE, 7);
    let mut v = Verdict::default();
    let log = Log::new();
    let fx = Fixture::new(Projection::Perspective, true).unwrap();
    let config = realistic();
    emit(
        CASE,
        "fixture",
        json!({"scene_sha256": fx.scene_sha256(), "projection": fx.projection.name(), "ui": fx.ui,
            "output": OUT, "viewport": null, "config": config_json(&config),
            "cadence": "back-to-back ticks; dispatch only by the host's own polls"}),
    );
    let mut run = scaled(&fx, &log, "A", OUT, None, config);
    emit(CASE, "adapter", adapter_json(&run.renderer));
    let ui0 = pixels::ui_json(run.ui().as_deref());
    let mut observations = Vec::new();
    for j in 1..=TICKS {
        run.tick(&log, j, &[], &[], &mut budget, "adaptive")
            .unwrap();
        observations.push(run.observe(j));
    }
    let events = log.events();
    let (predicted, samples, ambiguous) = replay(&events, "A", &config);
    let observed: Vec<u8> = observations
        .iter()
        .map(|o| o["receipt"]["scale16"].as_u64().unwrap_or(0) as u8)
        .collect();
    v.check(
        predicted.contains(&observed),
        "receipt scales follow the witnessed deliveries, single slot and policy",
    );
    v.check(
        observations.iter().all(|o| o["discarded"] == 0),
        "no sample of one owner at one extent is discarded",
    );
    v.check(
        observations
            .iter()
            .enumerate()
            .all(|(i, o)| o["receipt"]["sequence"] == i as u64 + 1),
        "one receipt per accepted frame",
    );
    v.check(
        observations.iter().all(|o| {
            let n = o["receipt"]["scale16"].as_u64().unwrap_or(0) as u8;
            o["receipt"]["internal"] == json!(internal(OUT, n))
                && o["receipt"]["output"] == json!(OUT)
        }),
        "every receipt's internal size is ceil(output x scale) at physical output",
    );
    v.check(
        pixels::ui_json(run.ui().as_deref()) == ui0,
        "UI layout stays physical through scale changes",
    );
    emit(
        CASE,
        "replay",
        json!({"observations": observations, "predicted_scale16": predicted,
            "observed_scale16": observed, "samples": samples,
            "ambiguous_samples": ambiguous}),
    );
    emit(CASE, "events", log.json());
    budget.finish();
    v.finish(CASE);
}

#[test]
#[ignore = "requires a native GPU; handoff 0067 bounded execution"]
fn delayed_delivery_resize_and_replacement() {
    const CASE: &str = "delayed_delivery_resize_and_replacement";
    let mut budget = Budget::new(CASE, 7);
    let mut v = Verdict::default();
    let log = Log::new();
    let fx = Fixture::new(Projection::Perspective, false).unwrap();
    let config = stepping(4);
    let (e1, e2) = ([960, 540], [800, 450]);
    emit(
        CASE,
        "fixture",
        json!({"scene_sha256": fx.scene_sha256(), "projection": fx.projection.name(), "ui": fx.ui,
            "config": config_json(&config), "extents": [e1, e2],
            "injections": {"backlog_bytes": probe::BACKLOG_BYTES, "backlog_copies": probe::BACKLOG_COPIES}}),
    );
    let mut run = scaled(&fx, &log, "L", e1, None, config);
    emit(CASE, "adapter", adapter_json(&run.renderer));
    let mut obs = Vec::new();
    // Frame 1: its callback waits behind injected queued work.
    run.probe.arm_backlog();
    run.tick(&log, 1, &[], &[], &mut budget, "backlog").unwrap();
    obs.push(run.observe(1));
    v.check(
        receipt_is(run.receipt().as_ref(), 1, 4, e1, internal(e1, 4), false),
        "frame 1 receipt",
    );
    // Frame 2: viewport resize; dispatch is forced only after the take.
    run.probe.resize(e2);
    run.probe.arm_acquire_poll();
    run.tick(&log, 2, &[], &[], &mut budget, "resize").unwrap();
    obs.push(run.observe(2));
    let delayed_resize = fire_phases(&log.events(), "L", 1) == vec![22];
    v.check(
        delayed_resize,
        "frame 1 completion dispatched after tick 2's take",
    );
    v.check(
        receipt_is(run.receipt().as_ref(), 2, 4, e2, internal(e2, 4), false)
            && run.discarded() == 0,
        "resized frame keeps scale; nothing consumed yet",
    );
    // Frame 3: the old-extent result is consumed and discarded.
    run.tick(&log, 3, &[], &[], &mut budget, "stale").unwrap();
    obs.push(run.observe(3));
    v.check(
        receipt_is(run.receipt().as_ref(), 3, 4, e2, internal(e2, 4), false)
            && run.discarded() == 1,
        "stale-epoch delivery discarded; scale unchanged",
    );
    log.poll(&run.renderer, probe::PHASE_TEST_POLL).unwrap();
    // Frame 4: the new epoch's own sample is accepted; arm backlog again.
    run.probe.arm_backlog();
    run.tick(&log, 4, &[], &[], &mut budget, "accept+backlog")
        .unwrap();
    obs.push(run.observe(4));
    v.check(
        receipt_is(run.receipt().as_ref(), 4, 5, e2, internal(e2, 5), false)
            && run.discarded() == 1,
        "current-epoch delivery accepted: 5/16",
    );
    // Frame 5: Space despawns the pillar, so the consumer prepares a replacement
    // scene (fresh telemetry owner); old dispatch forced after the take.
    run.probe.arm_acquire_poll();
    run.tick(&log, 5, &[key(true)], &[], &mut budget, "replace")
        .unwrap();
    obs.push(run.observe(5));
    let delayed_replacement = fire_phases(&log.events(), "L", 4) == vec![52];
    v.check(
        delayed_replacement,
        "frame 4 completion dispatched after tick 5's take",
    );
    v.check(
        receipt_is(run.receipt().as_ref(), 1, 4, e2, internal(e2, 4), false)
            && run.discarded() == 1,
        "replacement owner starts at sequence 1 and its initial scale",
    );
    // Frame 6: the previous owner's result is foreign to the replacement.
    run.tick(&log, 6, &[key(false)], &[], &mut budget, "foreign")
        .unwrap();
    obs.push(run.observe(6));
    v.check(
        receipt_is(run.receipt().as_ref(), 2, 4, e2, internal(e2, 4), false)
            && run.discarded() == 2,
        "previous-owner delivery discarded; replacement scale unchanged",
    );
    log.poll(&run.renderer, probe::PHASE_TEST_POLL).unwrap();
    // Frame 7: the replacement's own sample is accepted.
    run.tick(&log, 7, &[], &[], &mut budget, "accept").unwrap();
    obs.push(run.observe(7));
    v.check(
        receipt_is(run.receipt().as_ref(), 3, 5, e2, internal(e2, 5), false)
            && run.discarded() == 2,
        "replacement delivery accepted: 5/16",
    );
    let events = log.events();
    v.check(
        fire_phases(&events, "L", 2).len() == 1 && fire_phases(&events, "L", 5).len() == 1,
        "frames without a consumer callback still completed (witnessed)",
    );
    emit(
        CASE,
        "observations",
        json!({"obs": obs, "delayed_resize": delayed_resize, "delayed_replacement": delayed_replacement}),
    );
    emit(CASE, "events", log.json());
    budget.finish();
    v.finish(CASE);
}

#[test]
#[ignore = "requires a native GPU; handoff 0067 bounded execution"]
fn drop_and_recreate_host() {
    const CASE: &str = "drop_and_recreate_host";
    let mut budget = Budget::new(CASE, 3);
    let mut v = Verdict::default();
    let log = Log::new();
    let fx = Fixture::new(Projection::Perspective, false).unwrap();
    let config = stepping(4);
    let size = [960, 540];
    emit(
        CASE,
        "fixture",
        json!({"scene_sha256": fx.scene_sha256(), "config": config_json(&config), "output": size}),
    );
    let (instance, renderer) = renderer().unwrap();
    emit(CASE, "adapter", adapter_json(&renderer));
    let settings = |probe: &Arc<Probe>| {
        GpuRenderSettings {
            renderer: Arc::clone(&renderer),
            targets: probe.targets(),
        }
        .with_dynamic_resolution(config)
        .unwrap()
    };
    let a_probe = Probe::new("A", &renderer, &log, size, None);
    let mut a = Run {
        instance,
        renderer: Arc::clone(&renderer),
        probe: Arc::clone(&a_probe),
        game: fx.game(size, settings(&a_probe)).unwrap(),
    };
    a.probe.arm_backlog();
    a.tick(&log, 1, &[], &[], &mut budget, "A backlog").unwrap();
    let a_obs = a.observe(1);
    // Drop the whole composed host while its callback is outstanding.
    log.set_phase(probe::PHASE_DROP);
    let Run { instance, game, .. } = a;
    drop(game);
    log.record("host-dropped", "A", 1);
    log.set_phase(probe::PHASE_PREPARE);
    let b_probe = Probe::new("B", &renderer, &log, size, None);
    let mut b = Run {
        instance,
        renderer: Arc::clone(&renderer),
        probe: Arc::clone(&b_probe),
        game: fx.game(size, settings(&b_probe)).unwrap(),
    };
    log.record("host-prepared", "B", 0);
    b.probe.arm_acquire_poll();
    b.tick(&log, 2, &[], &[], &mut budget, "B first").unwrap();
    let b1 = b.observe(2);
    v.check(
        receipt_is(b.receipt().as_ref(), 1, 4, size, internal(size, 4), false)
            && b.discarded() == 0,
        "recreated host starts fresh: sequence 1, initial scale, no discards",
    );
    let events = log.events();
    let dropped_at = events
        .iter()
        .position(|e| e.kind == "host-dropped")
        .unwrap();
    let a_fire = events
        .iter()
        .position(|e| e.kind == "fire" && e.probe == "A" && e.frame == 1);
    v.check(
        a_fire.is_some_and(|i| i > dropped_at),
        "the dropped host's callback was outstanding and dispatched afterwards",
    );
    log.poll(&renderer, probe::PHASE_TEST_POLL).unwrap();
    b.tick(&log, 3, &[], &[], &mut budget, "B second").unwrap();
    let b2 = b.observe(3);
    v.check(
        receipt_is(b.receipt().as_ref(), 2, 5, size, internal(size, 5), false)
            && b.discarded() == 0,
        "only the new host's own delivery reached it: 5/16, nothing discarded",
    );
    emit(
        CASE,
        "observations",
        json!({"A": a_obs, "B": [b1, b2], "a_fire_event_index": a_fire, "drop_event_index": dropped_at}),
    );
    emit(CASE, "events", log.json());
    budget.finish();
    v.finish(CASE);
}

#[test]
#[ignore = "requires a native GPU; handoff 0067 bounded execution"]
fn terminal_failures_and_late_callbacks() {
    const CASE: &str = "terminal_failures_and_late_callbacks";
    let mut budget = Budget::new(CASE, 4);
    let mut v = Verdict::default();
    let log = Log::new();
    let fx = Fixture::new(Projection::Perspective, false).unwrap();
    let size = [960, 540];
    let config = stepping(4);
    // SceneInput from the CPU observer composition: no GPU work.
    let mut cpu = fx.cpu_game(size).unwrap();
    let mut pcm = vec![0.; cpu.next_output_frames().unwrap() * 2];
    cpu.tick(&[], &mut pcm).unwrap();
    let host = cpu.host().unwrap();
    let input = host.render_input().input();
    let label = host.render_input().camera_label();
    let (instance, renderer) = renderer().unwrap();
    emit(CASE, "adapter", adapter_json(&renderer));
    let mut results = Vec::new();
    for (mode, name, j) in [(1u8, "Q1", 1u64), (2, "Q2", 4)] {
        let probe = Probe::new(name, &renderer, &log, size, None);
        let mut consumer = <GpuRenderConsumer<Scaled> as RenderInputConsumer>::prepare(
            input,
            label,
            None,
            GpuRenderSettings {
                renderer: Arc::clone(&renderer),
                targets: probe.targets(),
            }
            .with_dynamic_resolution(config)
            .unwrap(),
        )
        .unwrap();
        let state = |c: &GpuRenderConsumer<Scaled>| {
            json!({"failed": c.is_failed(), "accepted_frames": c.accepted_frames(),
                "receipt": receipt_json(c.resolution_frame().as_ref()),
                "discarded": c.discarded_resolution_samples(),
                "acquires": probe.acquires.load(Ordering::SeqCst),
                "presents": probe.presents.load(Ordering::SeqCst)})
        };
        probe.arm_backlog();
        budget.attempt(&format!("{name} accepted"), true);
        log.begin_tick(j);
        consumer
            .accept(input, label, UpdateKind::Motion, None)
            .unwrap();
        log.end_tick(j);
        let first = state(&consumer);
        v.check(
            receipt_is(
                consumer.resolution_frame().as_ref(),
                1,
                4,
                size,
                internal(size, 4),
                false,
            ),
            format!("{name}: accepted first frame"),
        );
        if mode == 2 {
            probe.fill([255, 0, 255, 255]);
        }
        probe.fault.store(mode, Ordering::SeqCst);
        budget.attempt(&format!("{name} refused"), mode == 2);
        log.begin_tick(j + 1);
        let error = consumer
            .accept(input, label, UpdateKind::Motion, None)
            .unwrap_err()
            .to_string();
        log.end_tick(j + 1);
        let failed = state(&consumer);
        // Dispatch everything outstanding only after the terminal failure.
        log.poll(&renderer, probe::PHASE_AFTER_FAILURE).unwrap();
        let after = state(&consumer);
        let events = log.events();
        let late = fire_phases(&events, name, 1);
        v.check(
            late == vec![probe::PHASE_AFTER_FAILURE],
            format!("{name}: the accepted frame's completion dispatched after the failure"),
        );
        v.check(
            consumer.is_failed()
                && consumer.accepted_frames() == 1
                && consumer.resolution_frame().is_none()
                && consumer.discarded_resolution_samples() == 0
                && failed == after,
            format!("{name}: refusal is terminal and the late callback changes nothing"),
        );
        let mut extra = json!({});
        if mode == 2 {
            let frame = probe.pixels().unwrap();
            let magenta = frame
                .bytes
                .chunks(4)
                .filter(|p| *p == [255, 0, 255, 255])
                .count();
            v.check(
                magenta == 0 && fire_phases(&events, name, 2).len() == 1,
                "Q2: the refused presentation's frame was submitted and completed",
            );
            v.check(
                probe.presents.load(Ordering::SeqCst) == 1,
                "Q2: only the first frame was presented",
            );
            extra = json!({"magenta_pixels_left": magenta, "target_sha256": frame.sha256()});
        }
        let refusal = if mode == 1 {
            "acquisition refusal"
        } else {
            "presentation refusal"
        };
        results.push(json!({"probe": name, "mode": refusal,
            "after_first": first, "refusal_error": error, "after_refusal": failed,
            "after_late_dispatch": after, "late_fire_phases": late,
            "extra": extra}));
    }
    emit(CASE, "results", Value::Array(results));
    emit(CASE, "registry", registry_json(&instance));
    emit(CASE, "events", log.json());
    budget.finish();
    v.finish(CASE);
}

type Owned = (wgpu::Texture, wgpu::TextureView, [u32; 2]);
fn target(owned: &Owned, viewport: Option<Viewport>) -> RetainedTarget<'_> {
    RetainedTarget {
        view: &owned.1,
        format: FORMAT,
        width: owned.2[0],
        height: owned.2[1],
        viewport,
    }
}

/// Lower-level owner contract with actual callbacks: start before the retained
/// submit, end in this test's own `on_submitted_work_done`, dispatched by an
/// explicit poll. Returns the accepted receipt and the measured milliseconds.
fn submit_measured(
    scene: &mut incant_render::RetainedScene,
    renderer: &Renderer,
    input: &SceneInput,
    target: RetainedTarget<'_>,
    log: &Log,
) -> (Option<ResolutionFrame>, f64) {
    let start = Instant::now();
    scene.submit(renderer, input, target).unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    renderer.queue.on_submitted_work_done(move || {
        let _ = tx.send(start.elapsed().as_secs_f64() * 1000.);
    });
    log.poll(renderer, probe::PHASE_TEST_POLL).unwrap();
    let ms = rx
        .try_recv()
        .expect("callback dispatched by the explicit poll");
    (scene.resolution_frame(), ms)
}

#[test]
#[ignore = "requires a native GPU; handoff 0067 bounded execution"]
fn owner_contract_orthographic_lower_level() {
    const CASE: &str = "owner_contract_orthographic_lower_level";
    let mut budget = Budget::new(CASE, 10);
    let mut v = Verdict::default();
    let log = Log::new();
    let fx = Fixture::new(Projection::Orthographic, false).unwrap();
    let vp = offset_viewport();
    let mut cpu = fx.cpu_game(OUT).unwrap();
    let mut pcm = vec![0.; cpu.next_output_frames().unwrap() * 2];
    cpu.tick(&[], &mut pcm).unwrap();
    let host = cpu.host().unwrap();
    let input = host.render_input().input();
    let label = host.render_input().camera_label();
    let (instance, renderer) = renderer().unwrap();
    emit(CASE, "adapter", adapter_json(&renderer));
    emit(
        CASE,
        "fixture",
        json!({"scene_sha256": fx.scene_sha256(), "projection": fx.projection.name(),
            "viewport": viewport_json(&vp), "fixed": config_json(&fixed(12)), "stepping": config_json(&stepping(8))}),
    );
    let mut scene = renderer.prepare_retained(input, label).unwrap();
    let make = |size: [u32; 2]| {
        let texture = probe::target_texture(&renderer, size);
        let view = texture.create_view(&Default::default());
        (texture, view, size)
    };
    let (out, small, reference) = (make(OUT), make([800, 450]), make(internal(OUT, 12)));
    // Orthographic fixed 75% under the offset viewport, and its reference.
    scene.set_dynamic_resolution(Some(fixed(12))).unwrap();
    budget.attempt("fixed 12", true);
    let (fixed_receipt, fixed_ms) =
        submit_measured(&mut scene, &renderer, input, target(&out, Some(vp)), &log);
    let fixed_receipt = fixed_receipt.unwrap();
    v.check(
        receipt_is(Some(&fixed_receipt), 1, 12, OUT, internal(OUT, 12), false),
        "orthographic fixed 12/16 receipt",
    );
    let frame = pixels::read(&renderer, &out.0).unwrap();
    scene.set_dynamic_resolution(None).unwrap();
    v.check(
        scene.resolution_frame().is_none(),
        "None restores output-sized rendering and drops the receipt owner",
    );
    budget.attempt("reference 12", true);
    scene
        .submit(
            &renderer,
            input,
            target(&reference, Some(scaled_viewport(12))),
        )
        .unwrap();
    let reference_frame = pixels::read(&renderer, &reference.0).unwrap();
    let mapping =
        pixels::compare_mapping(&frame, &reference_frame, 12, vp.rect, vp.corner_radii, &[]);
    v.check(
        u(&mapping["mismatched"]) == 0 && u(&mapping["compared"]) > 100_000,
        "orthographic 12/16 frame equals nearest expansion of its reference",
    );
    emit(
        CASE,
        "mapping",
        json!({"receipt": receipt_json(Some(&fixed_receipt)), "measured_ms": fixed_ms,
            "frame_sha256": frame.sha256(), "reference_sha256": reference_frame.sha256(), "comparison": mapping}),
    );
    // Fresh telemetry owner: every rejection below has exactly one reason.
    scene.set_dynamic_resolution(Some(stepping(8))).unwrap();
    let mut checks = Vec::new();
    let mut record = |name: &str,
                      frame: &ResolutionFrame,
                      ms: f64,
                      expected: std::result::Result<bool, ResolutionError>,
                      scene: &mut incant_render::RetainedScene| {
        let actual = scene.record_frame_time(frame, ms);
        checks.push(
            json!({"check": name, "sequence": frame.sequence(), "ms": ms,
            "expected": format!("{expected:?}"), "actual": format!("{actual:?}")}),
        );
        actual == expected
    };
    budget.attempt("g1", true);
    let (g1, ms1) = submit_measured(&mut scene, &renderer, input, target(&out, None), &log);
    let g1 = g1.unwrap();
    v.check(
        receipt_is(Some(&g1), 1, 8, OUT, internal(OUT, 8), false),
        "g1 receipt",
    );
    let ok = [
        record(
            "foreign: replaced owner's receipt",
            &fixed_receipt,
            ms1,
            Ok(false),
            &mut scene,
        ),
        record(
            "invalid: NaN",
            &g1,
            f64::NAN,
            Err(ResolutionError::Telemetry),
            &mut scene,
        ),
        record(
            "invalid: zero",
            &g1,
            0.,
            Err(ResolutionError::Telemetry),
            &mut scene,
        ),
        record(
            "invalid: negative",
            &g1,
            -1.,
            Err(ResolutionError::Telemetry),
            &mut scene,
        ),
        record(
            "invalid: above 10000 ms",
            &g1,
            10_000.1,
            Err(ResolutionError::Telemetry),
            &mut scene,
        ),
        record("valid current receipt", &g1, ms1, Ok(true), &mut scene),
        record("duplicate", &g1, ms1, Ok(false), &mut scene),
    ];
    v.check(
        ok.iter().all(|x| *x),
        "foreign, invalid, valid and duplicate telemetry",
    );
    budget.attempt("g2", true);
    let (g2, ms2) = submit_measured(&mut scene, &renderer, input, target(&out, None), &log);
    let g2 = g2.unwrap();
    v.check(
        receipt_is(Some(&g2), 2, 9, OUT, internal(OUT, 9), false),
        "only the one valid sample changed the scale (9/16)",
    );
    budget.attempt("g3", true);
    let (g3, ms3) = submit_measured(&mut scene, &renderer, input, target(&small, None), &log);
    let g3 = g3.unwrap();
    budget.attempt("g4", true);
    let (g4, ms4) = submit_measured(&mut scene, &renderer, input, target(&small, None), &log);
    let g4 = g4.unwrap();
    v.check(
        receipt_is(Some(&g3), 3, 9, [800, 450], internal([800, 450], 9), false)
            && receipt_is(Some(&g4), 4, 9, [800, 450], internal([800, 450], 9), false),
        "resized receipts g3 and g4",
    );
    let ok = [
        record(
            "stale: previous extent epoch",
            &g2,
            ms2,
            Ok(false),
            &mut scene,
        ),
        record("valid: current epoch", &g3, ms3, Ok(true), &mut scene),
        record("stale: previous scale", &g4, ms4, Ok(false), &mut scene),
    ];
    v.check(
        ok.iter().all(|x| *x),
        "stale epoch, valid and previous-scale telemetry",
    );
    // Refused staging, planning and encoding never publish a receipt.
    let empty = SceneInput::default();
    let refusals = [
        ("staging: changed bindings", {
            budget.attempt("refused staging", false);
            scene.submit(&renderer, &empty, target(&small, None))
        }),
        ("planning: viewport outside the target", {
            budget.attempt("refused planning", false);
            scene.submit(
                &renderer,
                input,
                target(
                    &small,
                    Some(Viewport {
                        rect: [700., 400., 200., 100.],
                        corner_radii: [0.; 4],
                        canvas_srgb: CANVAS_SRGB,
                    }),
                ),
            )
        }),
        ("encoding: unsupported output format", {
            budget.attempt("refused encoding", false);
            scene.submit(
                &renderer,
                input,
                RetainedTarget {
                    format: wgpu::TextureFormat::Rgba16Float,
                    ..target(&small, None)
                },
            )
        }),
    ]
    .map(|(name, result)| {
        let kept = scene.resolution_frame().map(|f| f.sequence());
        json!({"refusal": name, "error": result.err().map(|e| e.to_string()), "receipt_sequence_after": kept})
    });
    v.check(
        refusals
            .iter()
            .all(|r| !r["error"].is_null() && r["receipt_sequence_after"] == 4),
        "every refusal returned an error and left receipt 4 current",
    );
    budget.attempt("g5", true);
    let (g5, ms5) = submit_measured(&mut scene, &renderer, input, target(&small, None), &log);
    v.check(
        receipt_is(
            g5.as_ref(),
            5,
            10,
            [800, 450],
            internal([800, 450], 10),
            false,
        ),
        "next accepted frame is sequence 5 at the accepted 10/16",
    );
    emit(
        CASE,
        "owner",
        json!({"checks": checks, "refusals": refusals,
            "receipts": [receipt_json(Some(&g1)), receipt_json(Some(&g2)), receipt_json(Some(&g3)),
                receipt_json(Some(&g4)), receipt_json(g5.as_ref())],
            "measured_ms": [ms1, ms2, ms3, ms4, ms5]}),
    );
    emit(CASE, "registry", registry_json(&instance));
    emit(CASE, "events", log.json());
    budget.finish();
    v.finish(CASE);
}

#[test]
#[ignore = "requires a native GPU; handoff 0067 bounded execution"]
fn pixel_camera_bypass() {
    const CASE: &str = "pixel_camera_bypass";
    let mut budget = Budget::new(CASE, 3);
    let mut v = Verdict::default();
    let log = Log::new();
    let fx = Fixture::new(Projection::PixelPerfect, false).unwrap();
    let vp = offset_viewport();
    let config = stepping(4);
    // Integer presentation inside the physical viewport, from the PixelGrid
    // contract: origin ceil(xy), available floor(xy+wh)-origin, whole scale,
    // centred letterbox.
    let logical = [240u32, 135];
    let origin = [vp.rect[0].ceil() as u32, vp.rect[1].ceil() as u32];
    let available = [
        (vp.rect[0] + vp.rect[2]).floor() as u32 - origin[0],
        (vp.rect[1] + vp.rect[3]).floor() as u32 - origin[1],
    ];
    let grid = (available[0] / logical[0]).min(available[1] / logical[1]);
    let at = [
        origin[0] + (available[0] - logical[0] * grid) / 2,
        origin[1] + (available[1] - logical[1] * grid) / 2,
    ];
    emit(
        CASE,
        "fixture",
        json!({"scene_sha256": fx.scene_sha256(), "projection": fx.projection.name(),
            "viewport": viewport_json(&vp), "config": config_json(&config),
            "expected_grid": {"logical": logical, "scale": grid, "origin": at}}),
    );
    let mut d = disabled(&fx, &log, "D", OUT, Some(vp));
    emit(CASE, "adapter", adapter_json(&d.renderer));
    d.tick(&log, 1, &[], &[], &mut budget, "D").unwrap();
    let d1 = d.probe.pixels().unwrap();
    let mut p = scaled(&fx, &log, "P", OUT, Some(vp), config);
    p.tick(&log, 1, &[], &[], &mut budget, "P first").unwrap();
    let p1 = p.probe.pixels().unwrap();
    let exact = pixels::equal_bytes(&d1, &p1);
    v.check(
        exact["identical"] == true,
        "adaptive policy at 4/16 leaves the pixel camera frame byte-identical",
    );
    v.check(
        receipt_is(p.receipt().as_ref(), 1, 16, OUT, logical, true),
        "pixel receipt: full scale, the logical target",
    );
    // Whole-pixel blocks: every grid cell of the letterboxed image is uniform.
    let mut uniform = true;
    for cy in 0..logical[1] {
        for cx in 0..logical[0] {
            let first = p1.px(at[0] + cx * grid, at[1] + cy * grid);
            for dy in 0..grid {
                for dx in 0..grid {
                    uniform &= p1.px(at[0] + cx * grid + dx, at[1] + cy * grid + dy) == first;
                }
            }
        }
    }
    v.check(uniform, "every logical pixel maps to a uniform 3x3 block");
    log.poll(&p.renderer, probe::PHASE_TEST_POLL).unwrap();
    p.tick(&log, 2, &[], &[], &mut budget, "P second").unwrap();
    let events = log.events();
    let fired = events
        .iter()
        .position(|e| e.kind == "fire" && e.probe == "P" && e.frame == 1);
    let second = events
        .iter()
        .position(|e| e.kind == "tick-call" && e.frame == 2);
    v.check(
        fired.zip(second).is_some_and(|(f, t)| f < t),
        "the pixel frame's work completed before the second frame",
    );
    v.check(
        receipt_is(p.receipt().as_ref(), 2, 16, OUT, logical, true) && p.discarded() == 0,
        "no pixel-camera completion drove feedback or was delivered",
    );
    emit(
        CASE,
        "observations",
        json!({"D": d.observe(1), "P": p.observe(2), "exact": exact, "uniform_blocks": uniform,
            "frame_sha256": p1.sha256()}),
    );
    emit(CASE, "events", log.json());
    budget.finish();
    v.finish(CASE);
}
