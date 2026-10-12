//! Public cooked native oracle. Does not load guest bytes or an ABI decoder.
#[allow(dead_code)]
#[path = "../../incant_wasm_runtime/tests/support/animation_compound.rs"]
mod fixture;
use incant_runtime::*;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 4 {
        return Err(
            "usage: animation_compound_package_oracle PROJECT SCENE OUTPUT_SCENE TICKS".into(),
        );
    }
    let path = std::path::Path::new(&args[0]);
    let project = incant_doc::Project::from_text(&std::fs::read_to_string(path)?)?;
    let resources = incant_cook::load_project_animation_resources(
        &project,
        &args[1],
        path.parent().ok_or("project parent")?,
    )?;
    let bytes = incant_cook::cook_scene_with_resources(
        &project,
        &args[1],
        &Default::default(),
        &resources,
    )?;
    let scene = CookedScene::from_bytes(&bytes)?;
    let ticks: u32 = args[3].parse()?;
    if scene.id() != fixture::id(99) || scene.tick_rate() != 60 || ticks != 56 {
        return Err("oracle scene/clock bounds".into());
    }
    let mut output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[2])?;
    std::io::Write::write_all(&mut output, &bytes)?;
    let mut world = NativeWorld::new(scene);
    for turn in 0..ticks {
        world.step()?;
        let mut rows = Vec::new();
        world.snapshot_animations(&mut rows)?;
        let compound = rows
            .iter()
            .any(|r| r.pose.is_some_and(|p| p.outer_weight().is_some()));
        let max_samples = rows
            .iter()
            .map(|r| {
                r.pose
                    .map_or(if r.blend.is_some() { 2 } else { 1 }, |p| p.samples().len())
            })
            .max()
            .unwrap_or(0);
        let values = fixture::observations(&mut world);
        println!(
            "{}",
            serde_json::json!({"tick":turn+1,"values":values.as_slice(),"compound":compound,"max_samples":max_samples})
        );
        fixture::controls(&mut world, turn);
    }
    Ok(())
}
