//! Independent typed native oracle for the public weighted-pose package probe.
//! Authoring/cooking happens once; no ABI, guest decoder or guest code is loaded.
use incant_runtime::*;
fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
fn controls(turn: u32) -> Vec<StructuralCommand> {
    let animation = |entity, control| StructuralCommand::ControlAnimation {
        entity: id(entity),
        control,
    };
    let parameter = |key, value| StructuralCommand::ControlAnimationGraph {
        entity: id(1),
        control: AnimationGraphControl::SetParameter {
            parameter: id(key),
            value,
        },
    };
    let select = |key| StructuralCommand::ControlAnimationGraph {
        entity: id(1),
        control: AnimationGraphControl::SelectState(id(key)),
    };
    let first = match turn {
        0 | 4 | 5 | 6 | 16 => Some(parameter(
            10,
            match turn {
                0 => 0.75,
                4 => 0.0,
                5 => 1.0,
                16 => 0.25,
                _ => 0.5,
            },
        )),
        1 | 12 => Some(animation(1, AnimationControl::SetPlaying(false))),
        2 | 14 => Some(select(20)),
        3 | 13 => Some(animation(1, AnimationControl::SetPlaying(true))),
        7 => Some(animation(1, AnimationControl::SetSpeed(-1.0))),
        8 => Some(animation(1, AnimationControl::SetSpeed(1.0))),
        9 => Some(select(21)),
        10 => Some(animation(1, AnimationControl::SetSpeed(1.0))),
        15 => Some(parameter(12, 0.25)),
        _ => None,
    };
    let second = match turn {
        0 => Some(AnimationControl::Crossfade {
            target: AnimationSample {
                clip: Some(1),
                seconds: 0.5,
            },
            duration: 0.2,
        }),
        1 | 10 => Some(AnimationControl::SetPlaying(false)),
        3 | 12 => Some(AnimationControl::SetPlaying(true)),
        4 => Some(AnimationControl::SetSpeed(-1.0)),
        7 => Some(AnimationControl::Seek(0.7)),
        8 => Some(AnimationControl::Crossfade {
            target: AnimationSample::REST,
            duration: 0.1,
        }),
        15 => Some(AnimationControl::Select(AnimationSample {
            clip: Some(0),
            seconds: 0.0,
        })),
        _ => None,
    }
    .map(|control| animation(2, control));
    first.into_iter().chain(second).collect()
}

/// Independent native observations: retain every sample, never hash or decode
/// guest bytes. Scalar clocks, flags and interpolation meaning are separate.
fn observe(world: &mut NativeWorld) -> ([[f64; 3]; 48], [[f64; 3]; 9]) {
    let mut rows = Vec::new();
    world.snapshot_animations(&mut rows).unwrap();
    rows.sort_unstable_by_key(|r| r.id);
    let mut observations = [[-2.0, 0.0, 0.0]; 48];
    let mut metadata = [[0.0; 3]; 9];
    for (index, row) in rows.iter().enumerate() {
        let (kind, samples): (_, Vec<_>) = if let Some(pose) = row.pose {
            (
                3.0,
                pose.samples()
                    .iter()
                    .map(|v| (v.sample, v.weight))
                    .collect(),
            )
        } else if let Some(blend) = row.blend {
            (
                if row.state.transition.is_some() {
                    1.0
                } else {
                    2.0
                },
                vec![
                    (blend.sample, 1.0 - blend.weight),
                    (row.state.sample, blend.weight),
                ],
            )
        } else {
            (0.0, vec![(row.state.sample, 1.0)])
        };
        for (slot, (sample, weight)) in samples.iter().enumerate() {
            observations[index * 16 + slot] =
                [sample.clip.map_or(-1.0, f64::from), sample.seconds, *weight];
        }
        let state = row.state;
        metadata[index * 3] = [samples.len() as f64, kind, state.sample.seconds];
        metadata[index * 3 + 1] = [
            state.speed,
            f64::from(u8::from(state.playing)),
            f64::from(u8::from(state.completed)),
        ];
        metadata[index * 3 + 2] = [
            f64::from(u8::from(state.looping)),
            state.transition.map_or(-1.0, |t| t.elapsed),
            state.transition.map_or(-1.0, |t| t.duration),
        ];
    }
    (observations, metadata)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let ik = args.get(4).is_some_and(|s| s == "--ik");
    if args.len() != 4 && !(args.len() == 5 && ik) {
        return Err("usage: animation_pose_package_oracle PROJECT SCENE OUTPUT_SCENE TICKS".into());
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
    if scene.id() != id(99) || scene.tick_rate() != 60 || ticks != 20 {
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
        let (samples, metadata) = observe(&mut world);
        let mut ik_rows = Vec::new();
        let mut record =
            serde_json::json!({"tick":turn+1,"samples":samples.as_slice(),"metadata":metadata});
        if ik {
            world.snapshot_ik(&mut ik_rows)?;
            if ik_rows.len() != 4 {
                return Err("oracle constraint count".into());
            }
            let mut values = Vec::new();
            for row in &ik_rows {
                values.extend([
                    row.sample.target,
                    row.sample.pole,
                    [
                        row.sample.weight,
                        u128::from_be_bytes(row.sample.id.0) as f64,
                        f64::from(row.sample.chain[1]),
                    ],
                ]);
            }
            record["ik"] = serde_json::json!(values);
        }
        println!("{record}");
        let mut commands = FrameCommands::default();
        for command in controls(turn) {
            commands.push(command)?;
        }
        for (index, row) in ik_rows.iter().enumerate() {
            commands.push(StructuralCommand::ControlIk {
                entity: row.entity,
                control: IkControl {
                    constraint: row.sample.id,
                    target: [(f64::from(turn) + 1.) / 8., index as f64, 0.],
                    pole: [0., 1., (f64::from(turn) + 1.) / 64.],
                    weight: f64::from(turn % 3) / 2.,
                },
            })?;
        }
        world.apply(&mut commands)?;
    }
    Ok(())
}
