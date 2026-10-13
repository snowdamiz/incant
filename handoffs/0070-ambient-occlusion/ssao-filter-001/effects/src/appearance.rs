//! Bounded appearance renders through the composed host: 4 views x 4 variants
//! = 16 complete frames, 11 matched pairs, 16 PNGs, at most 960x540. Each
//! render uses a fresh renderer configured once at startup (no per-frame
//! setup call) and one game tick. Every render is charged before it starts.
//! The numeric checks here are invariants (chrome, UI layout and opaque UI
//! interiors, DRS receipts); the appearance verdict is Claude's own review.
use crate::common::*;
use crate::composed::{Fixture, Probe, Targets, View};
use incant_render::Viewport;
use incant_render_data::{UiInput, UiRole};
use incant_render_gpu_host::{GpuRenderSettings, TargetProvider};
use serde_json::{Value, json};
use std::collections::{BTreeMap, btree_map::Entry};
use std::path::Path;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Variant {
    Off,
    Ssao,
    Bloom,
    Both,
}
impl Variant {
    fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Ssao => "ssao",
            Self::Bloom => "bloom",
            Self::Both => "ssao+bloom",
        }
    }
}

struct Render {
    label: &'static str,
    view: View,
    size: [u32; 2],
    /// Offset rounded viewport and screen-space UI together.
    framed: bool,
    variant: Variant,
    scale: Option<u8>,
}
const fn render(
    label: &'static str,
    view: View,
    size: [u32; 2],
    framed: bool,
    variant: Variant,
    scale: Option<u8>,
) -> Render {
    Render {
        label,
        view,
        size,
        framed,
        variant,
        scale,
    }
}
const PLAN: [Render; 16] = [
    render(
        "v1_off",
        View::Perspective,
        [959, 539],
        true,
        Variant::Off,
        None,
    ),
    render(
        "v1_ssao",
        View::Perspective,
        [959, 539],
        true,
        Variant::Ssao,
        None,
    ),
    render(
        "v1_bloom",
        View::Perspective,
        [959, 539],
        true,
        Variant::Bloom,
        None,
    ),
    render(
        "v1_both",
        View::Perspective,
        [959, 539],
        true,
        Variant::Both,
        None,
    ),
    render(
        "v2_off",
        View::Orthographic,
        [960, 540],
        false,
        Variant::Off,
        None,
    ),
    render(
        "v2_ssao",
        View::Orthographic,
        [960, 540],
        false,
        Variant::Ssao,
        None,
    ),
    render(
        "v2_bloom",
        View::Orthographic,
        [960, 540],
        false,
        Variant::Bloom,
        None,
    ),
    render(
        "v2_both",
        View::Orthographic,
        [960, 540],
        false,
        Variant::Both,
        None,
    ),
    render(
        "v3_off_12",
        View::Perspective,
        [959, 539],
        true,
        Variant::Off,
        Some(12),
    ),
    render(
        "v3_both_12",
        View::Perspective,
        [959, 539],
        true,
        Variant::Both,
        Some(12),
    ),
    render(
        "v3_off_8",
        View::Perspective,
        [959, 539],
        true,
        Variant::Off,
        Some(8),
    ),
    render(
        "v3_both_8",
        View::Perspective,
        [959, 539],
        true,
        Variant::Both,
        Some(8),
    ),
    render(
        "v4_off",
        View::CloseUp,
        [640, 480],
        false,
        Variant::Off,
        None,
    ),
    render(
        "v4_ssao",
        View::CloseUp,
        [640, 480],
        false,
        Variant::Ssao,
        None,
    ),
    render(
        "v4_bloom",
        View::CloseUp,
        [640, 480],
        false,
        Variant::Bloom,
        None,
    ),
    render(
        "v4_both",
        View::CloseUp,
        [640, 480],
        false,
        Variant::Both,
        None,
    ),
];
/// Baseline/candidate pairs reviewed by Claude (11 of the 16-pair ceiling).
pub const PAIRS: [(&str, &str); 11] = [
    ("v1_off", "v1_ssao"),
    ("v1_off", "v1_bloom"),
    ("v1_off", "v1_both"),
    ("v2_off", "v2_ssao"),
    ("v2_off", "v2_bloom"),
    ("v2_off", "v2_both"),
    ("v3_off_12", "v3_both_12"),
    ("v3_off_8", "v3_both_8"),
    ("v4_off", "v4_ssao"),
    ("v4_off", "v4_bloom"),
    ("v4_off", "v4_both"),
];

/// Handoff 0067's offset camera viewport with the editor's rounded chrome.
pub fn offset_viewport() -> Viewport {
    Viewport {
        rect: [37.5, 21.25, 880.25, 495.5],
        corner_radii: [8.; 4],
        canvas_srgb: [11, 12, 15],
    }
}

fn gpu_for(variant: Variant) -> Result<Gpu> {
    let gpu = Gpu::new()?;
    Ok(match variant {
        Variant::Off => gpu,
        Variant::Ssao => gpu.with_ao(AO)?,
        Variant::Bloom => gpu.with_bloom(BLOOM)?,
        Variant::Both => gpu.with_ao(AO)?.with_bloom(BLOOM)?,
    })
}

struct Rendered {
    frame: Rgba,
    state: Value,
    ui: Option<Arc<UiInput>>,
}

fn tick<P: TargetProvider>(
    fixture: &Fixture,
    size: [u32; 2],
    settings: GpuRenderSettings<P>,
) -> Result<(Value, Option<Arc<UiInput>>)> {
    let mut game = fixture.game(size, settings)?;
    let mut pcm = vec![0.; game.next_output_frames().map_err(|e| e.to_string())? * 2];
    game.tick(&[], &mut pcm).map_err(|e| e.to_string())?;
    let host = game.host().map_err(|e| e.to_string())?;
    let consumer = host.consumer();
    let receipt = consumer.resolution_frame().map_or(Value::Null, |f| {
        json!({"sequence": f.sequence(), "scale16": (f.scale().fraction() * 16.).round() as u32,
            "output": f.output_size(), "internal": f.internal_size(),
            "pixel_perfect": f.pixel_perfect()})
    });
    let ui = host.ui_input().cloned();
    Ok((
        json!({"accepted_frames": consumer.accepted_frames(), "receipt": receipt,
            "ui": ui_json(ui.as_deref())}),
        ui,
    ))
}

/// UI layout snapshot: widget IDs, roles, border and visible boxes, focus.
fn ui_json(ui: Option<&UiInput>) -> Value {
    ui.map_or(Value::Null, |ui| {
        json!({"viewport": ui.viewport, "widgets": ui.widgets.iter().map(|w| json!({
            "id": crate::composed::ulid_text(w.id), "role": format!("{:?}", w.role),
            "bounds": w.bounds, "visible": w.visible, "focused": w.focused,
            "pressed": w.pressed, "disabled": w.disabled})).collect::<Vec<_>>()})
    })
}
/// Physical rectangle of a logical UI box: the UI viewport maps onto the
/// target's viewport rectangle (`region.xy + logical * region.wh / ui.viewport`).
fn ui_to_physical(b: [f32; 4], region: [f32; 4], viewport: [f32; 2]) -> [f32; 4] {
    let s = [region[2] / viewport[0], region[3] / viewport[1]];
    [
        region[0] + b[0] * s[0],
        region[1] + b[1] * s[1],
        region[0] + b[2] * s[0],
        region[1] + b[3] * s[1],
    ]
}
/// Opaque Button/TextInput interiors (shrunk by one pixel, corners skipped):
/// UI surfaces are opaque, so these pixels cannot depend on the world.
fn opaque_regions(ui: &UiInput, region: [f32; 4]) -> Vec<[u32; 4]> {
    ui.widgets
        .iter()
        .filter(|w| matches!(w.role, UiRole::Button | UiRole::TextInput))
        .map(|w| {
            let v = ui_to_physical(w.visible, region, ui.viewport);
            [
                v[0].ceil() as u32 + 1,
                v[1].ceil() as u32 + 1,
                (v[2].floor() as u32).saturating_sub(1),
                (v[3].floor() as u32).saturating_sub(1),
            ]
        })
        .collect()
}
fn compare_regions(a: &Rgba, b: &Rgba, regions: &[[u32; 4]]) -> Value {
    const CORNER: u32 = 5;
    let (mut compared, mut differing) = (0u64, 0u64);
    for r in regions {
        for y in r[1]..r[3] {
            for x in r[0]..r[2] {
                let near_x = x < r[0] + CORNER || x + CORNER >= r[2];
                let near_y = y < r[1] + CORNER || y + CORNER >= r[3];
                if near_x && near_y {
                    continue;
                }
                compared += 1;
                differing += u64::from(a.px(x, y) != b.px(x, y));
            }
        }
    }
    json!({"regions": regions.len(), "compared": compared, "differing": differing})
}

struct Context {
    frames: Frames,
    store: Store,
    fixtures: BTreeMap<(View, bool), Fixture>,
    rendered: BTreeMap<&'static str, Rendered>,
    adapter: Value,
}

fn render_one(context: &mut Context, plan: &Render) -> Result<()> {
    context.frames.charge(plan.label)?;
    let fixture = match context.fixtures.entry((plan.view, plan.framed)) {
        Entry::Occupied(entry) => entry.into_mut(),
        Entry::Vacant(entry) => entry.insert(Fixture::new(plan.view, plan.framed)?),
    };
    let gpu = gpu_for(plan.variant)?;
    context.adapter = gpu.adapter();
    let setup = gpu.registry();
    let Gpu { instance, renderer } = gpu;
    let renderer = Arc::new(renderer);
    let viewport = plan.framed.then(offset_viewport);
    let probe = Probe::new(&renderer, plan.size, viewport);
    let settings = GpuRenderSettings {
        renderer: Arc::clone(&renderer),
        targets: Targets(Arc::clone(&probe)),
    };
    let (state, ui) = match plan.scale {
        None => tick(fixture, plan.size, settings)?,
        Some(n) => tick(
            fixture,
            plan.size,
            settings.with_dynamic_resolution(crate::numeric::fixed(n)?)?,
        )?,
    };
    let frame = read(&renderer, probe.texture())?;
    context.store.png(
        plan.label,
        &frame,
        json!({"view": plan.view.name(), "variant": plan.variant.name(), "size": plan.size,
            "viewport": viewport.map(|v| json!({"rect": v.rect, "corner_radii": v.corner_radii,
                "canvas_srgb": v.canvas_srgb})),
            "ui": plan.framed, "fixed_scale16": plan.scale, "state": state,
            "scene_sha256": fixture.scene_sha256(), "setup_registry": setup,
            "end_registry": registry_of(&instance),
            "presents": probe.presents.load(std::sync::atomic::Ordering::SeqCst)}),
    )?;
    context
        .rendered
        .insert(plan.label, Rendered { frame, state, ui });
    Ok(())
}

pub fn run(out: &Path) -> Result<bool> {
    let mut context = Context {
        frames: Frames::new("appearance", 16),
        store: Store::create(out)?,
        fixtures: BTreeMap::new(),
        rendered: BTreeMap::new(),
        adapter: Value::Null,
    };
    let mut verdict = Verdict::default();
    let mut failure = None;
    for plan in &PLAN {
        if plan.size[0] > 960 || plan.size[1] > 540 {
            failure = Some(format!("{} exceeds 960x540", plan.label));
            break;
        }
        if let Err(error) = render_one(&mut context, plan) {
            failure = Some(format!("{}: {error}", plan.label));
            break;
        }
    }
    verdict.check(
        failure.is_none(),
        "all 16 planned renders completed",
        json!(failure),
    );
    let rendered = &context.rendered;
    for (label, r) in rendered {
        let opaque = r.frame.bytes.chunks(4).all(|p| p[3] == 255);
        verdict.check(
            opaque,
            &format!("{label}: complete opaque output"),
            Value::Null,
        );
        verdict.check(
            r.state["accepted_frames"] == 1,
            &format!("{label}: one accepted frame"),
            r.state["accepted_frames"].clone(),
        );
    }
    let viewport = offset_viewport();
    if let Some(base) = rendered.get("v1_off") {
        let regions = base
            .ui
            .as_deref()
            .map(|ui| opaque_regions(ui, viewport.rect))
            .unwrap_or_default();
        for label in [
            "v1_ssao",
            "v1_bloom",
            "v1_both",
            "v3_off_12",
            "v3_both_12",
            "v3_off_8",
            "v3_both_8",
        ] {
            let Some(r) = rendered.get(label) else {
                continue;
            };
            let chrome =
                chrome_identical(&r.frame, &base.frame, viewport.rect, viewport.corner_radii);
            verdict.check(
                chrome["differing"] == 0,
                &format!("{label}: chrome/canvas identical to v1_off"),
                chrome,
            );
            verdict.check(
                r.state["ui"] == base.state["ui"],
                &format!("{label}: physical UI layout identical to v1_off"),
                Value::Null,
            );
            let interior = compare_regions(&r.frame, &base.frame, &regions);
            verdict.check(
                interior["differing"] == 0 && interior["compared"].as_u64().unwrap_or(0) > 0,
                &format!("{label}: opaque UI control interiors identical to v1_off"),
                interior,
            );
        }
    }
    for (label, n) in [
        ("v3_off_12", 12u32),
        ("v3_both_12", 12),
        ("v3_off_8", 8),
        ("v3_both_8", 8),
    ] {
        if let Some(r) = rendered.get(label) {
            let expected = [959u32, 539].map(|v| (v * n).div_ceil(16));
            let receipt = &r.state["receipt"];
            verdict.check(
                receipt["scale16"] == json!(n)
                    && receipt["internal"] == json!(expected)
                    && receipt["output"] == json!([959, 539]),
                &format!("{label}: actual DRS receipt"),
                receipt.clone(),
            );
        }
    }
    let mut pairs = Vec::new();
    for (baseline, candidate) in PAIRS {
        if let (Some(a), Some(b)) = (rendered.get(baseline), rendered.get(candidate)) {
            let (darker, brighter) = darker_brighter(&b.frame, &a.frame);
            pairs.push(json!({"baseline": format!("{baseline}.png"),
                "candidate": format!("{candidate}.png"),
                "candidate_darker_pixels": darker, "candidate_brighter_pixels": brighter,
                "identical": identical(&a.frame, &b.frame)}));
        }
    }
    context.frames.receipt();
    emit("appearance", "pairs", json!(pairs));
    context
        .store
        .finish(json!({"adapter": context.adapter, "pairs": pairs,
        "renders": context.rendered.len(), "frames": context.frames.used()}))?;
    Ok(verdict.finish("appearance"))
}
