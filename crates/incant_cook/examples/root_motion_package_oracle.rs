//! Typed native oracle for the exact publicly cooked scene. No guest or ABI decoder.
#[allow(dead_code)]
#[path = "../../incant_wasm_runtime/tests/support/root_motion.rs"]
mod root;
use incant_runtime::*;
use root::compound;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 4 {
        return Err("usage: root_motion_package_oracle PROJECT SCENE OUTPUT TICKS".into());
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
    if scene.id() != root::id(99) || scene.tick_rate() != 60 || ticks != 32 {
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
        let mut roots = Vec::new();
        world.snapshot_root_motion(&mut roots)?;
        let mut poses = Vec::new();
        world.snapshot_animations(&mut poses)?;
        let compound = poses
            .iter()
            .any(|r| r.pose.is_some_and(|p| p.outer_weight().is_some()));
        let max_samples = poses
            .iter()
            .map(|r| {
                r.pose
                    .map_or(if r.blend.is_some() { 2 } else { 1 }, |p| p.samples().len())
            })
            .max()
            .unwrap_or(0);
        let mut values = Vec::from(compound::observations(&mut world));
        values.extend(root::observations(&world));
        let mut body = [0.; 60];
        world.with_numeric_columns(|view| {
            view.for_each_chunk(|chunk| {
                for (i, (id, t)) in chunk
                    .ids
                    .iter()
                    .zip(chunk.transforms.as_chunks::<10>().0)
                    .enumerate()
                {
                    let n = u128::from_be_bytes(id.0);
                    if let Some(row) = [1, 2, 3, 5].iter().position(|v| *v == n) {
                        let at = row * 15;
                        body[at] = n as f64;
                        body[at + 1] = f64::from(chunk.velocities.is_some());
                        body[at + 2..at + 12].copy_from_slice(t);
                        if let Some(v) = chunk.velocities.as_ref() {
                            body[at + 12..at + 15].copy_from_slice(&v[i * 3..i * 3 + 3]);
                        }
                    }
                }
            })
        });
        values.extend(body);
        let reverse = roots.iter().any(|r| r.current.cycle < 0);
        let wrapped = roots.iter().any(|r| r.current.cycle != 0);
        let collision_limited = roots.iter().any(|r| {
            r.entity == root::id(3)
                && r.requested
                    .translation
                    .iter()
                    .zip(r.applied.translation)
                    .any(|(a, b)| (a - b).abs() > 1e-5)
        });
        println!(
            "{}",
            serde_json::json!({"tick":turn+1,"values":values,"compound":compound,"max_samples":max_samples,"reverse":reverse,"wrapped":wrapped,"collision_limited":collision_limited})
        );
        root::oracle(&mut world, turn);
    }
    Ok(())
}
