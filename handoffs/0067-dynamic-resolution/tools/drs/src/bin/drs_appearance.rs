//! Bounded appearance helper for handoff 0067. Renders exactly the frozen plan's
//! frames through the canonical composed game (one fresh game and renderer per
//! frame, tick 1, offscreen target), writes unedited RGBA PNGs and a report of
//! actual receipts. Every render counts against the packet's 24-frame budget.
//! No window, desktop capture, audio device or performance claim.
//!
//!   drs_appearance <plan.json> <new-output-directory>
use incant_0067_drs::fixture::{Fixture, Projection};
use incant_0067_drs::pixels;
use incant_0067_drs::probe::{Log, Probe};
use incant_0067_drs::*;
use incant_render_gpu_host::{GpuRenderSettings, TargetProvider};
use serde_json::{Value, json};
use std::collections::{BTreeMap, btree_map::Entry};
use std::{path::Path, sync::Arc};

const MAX_RENDERS: usize = 24;

fn render<P: TargetProvider>(
    fixture: &Fixture,
    size: [u32; 2],
    settings: GpuRenderSettings<P>,
    probe: &Probe,
    log: &Log,
) -> Result<Value> {
    let mut game = fixture.game(size, settings)?;
    let mut pcm = vec![0.; game.next_output_frames().map_err(|e| e.to_string())? * 2];
    log.begin_tick(1);
    game.tick(&[], &mut pcm).map_err(|e| e.to_string())?;
    log.end_tick(1);
    let host = game.host().map_err(|e| e.to_string())?;
    let consumer = host.consumer();
    Ok(json!({"accepted_frames": consumer.accepted_frames(),
        "receipt": receipt_json(consumer.resolution_frame().as_ref()),
        "ui": pixels::ui_json(host.ui_input().map(|u| &**u)),
        "presents": probe.presents.load(std::sync::atomic::Ordering::SeqCst)}))
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [plan_path, out] = &args[..] else {
        return Err("usage: drs_appearance <plan.json> <new-output-directory>".into());
    };
    let out = Path::new(out);
    let plan: Value = serde_json::from_slice(&std::fs::read(plan_path)?)?;
    let renders = plan["renders"].as_array().ok_or("plan renders")?;
    if renders.len() > MAX_RENDERS {
        return Err(format!(
            "{} renders exceed the budget of {MAX_RENDERS}",
            renders.len()
        )
        .into());
    }
    // Validate the whole plan before the first render.
    for r in renders {
        let size = [r["size"][0].as_u64(), r["size"][1].as_u64()];
        if !matches!(size, [Some(1..=960), Some(1..=540)])
            || Projection::parse(r["projection"].as_str().unwrap_or("")).is_none()
            || !matches!(r["viewport"].as_str(), Some("offset" | "none"))
            || !(r["scale16"].is_null() || matches!(r["scale16"].as_u64(), Some(8 | 12 | 16)))
            || r["name"]
                .as_str()
                .is_none_or(|n| n.is_empty() || n.contains('/'))
        {
            return Err(format!("invalid plan entry {r}").into());
        }
    }
    std::fs::create_dir(out)?;
    let mut fixtures: BTreeMap<(&str, bool), Fixture> = BTreeMap::new();
    let mut frames = Vec::new();
    let mut adapter = Value::Null;
    for r in renders {
        let name = r["name"].as_str().unwrap_or_default();
        let projection_name = r["projection"].as_str().unwrap_or_default();
        let projection = Projection::parse(projection_name).ok_or("projection")?;
        let ui = r["ui"].as_bool().unwrap_or(false);
        let size = [
            r["size"][0].as_u64().unwrap_or(0) as u32,
            r["size"][1].as_u64().unwrap_or(0) as u32,
        ];
        let viewport = (r["viewport"] == "offset").then(offset_viewport);
        let fixture = match fixtures.entry((projection_name, ui)) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(Fixture::new(projection, ui)?),
        };
        let (instance, renderer) = renderer()?;
        adapter = adapter_json(&renderer);
        let log = Log::new();
        let probe = Probe::new("R", &renderer, &log, size, viewport);
        let settings = GpuRenderSettings {
            renderer: Arc::clone(&renderer),
            targets: probe.targets(),
        };
        let scale16 = r["scale16"].as_u64().map(|n| n as u8);
        let state = match scale16 {
            None => render(fixture, size, settings, &probe, &log)?,
            Some(n) => render(
                fixture,
                size,
                settings.with_dynamic_resolution(fixed(n))?,
                &probe,
                &log,
            )?,
        };
        let frame = probe.pixels()?;
        let file = out.join(format!("{name}.png"));
        frame.write_png(&file)?;
        frames.push(
            json!({"name": name, "projection": projection.name(), "ui": ui,
            "size": size, "viewport": viewport.map(|v| json!({"rect": v.rect,
                "corner_radii": v.corner_radii, "canvas_srgb": v.canvas_srgb})),
            "requested": scale16.map(|n| config_json(&fixed(n))),
            "state": state, "rgba_sha256": frame.sha256(),
            "png": file.file_name().and_then(|n| n.to_str()),
            "png_sha256": incant_assets_runtime::sha256(&std::fs::read(&file)?),
            "registry": registry_json(&instance),
            "scene_sha256": fixture.scene_sha256()}),
        );
        println!("rendered {name}");
    }
    let report = json!({"format": "incant-0067-appearance", "adapter": adapter,
        "format_target": format!("{FORMAT:?}"), "renders": frames.len(), "frames": frames});
    std::fs::write(
        out.join("report.json"),
        serde_json::to_string_pretty(&report)? + "\n",
    )?;
    Ok(())
}
