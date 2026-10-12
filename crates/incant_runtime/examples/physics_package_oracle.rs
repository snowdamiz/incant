//! Independent native oracle for the public Rust/Wasm physics package probe.
//! Reads the real cooked scene and calls canonical solvers without any Wasm host.
use incant_runtime::*;
fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
fn hit<const N: usize>(entity: StableId, distance: f64, point: [f64; N], normal: [f64; N]) -> f64 {
    let mut value = distance + f64::from(entity.0[15]);
    for (i, (&p, &n)) in point.iter().zip(&normal).enumerate() {
        value += p * (i + 1) as f64 + n * (i + 5) as f64;
    }
    value
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err("usage: physics_package_oracle COOKED_SCENE TICKS".into());
    }
    let scene = CookedScene::from_bytes(&std::fs::read(&args[1])?)?;
    let count: u64 = args[2].parse()?;
    if count == 0 || count > 60 || scene.id() != id(99) {
        return Err("oracle scene/count bounds".into());
    }
    let dt = 1. / f64::from(scene.tick_rate());
    let mut world = NativeWorld::new(scene);
    for clock in 1..=count {
        world.step()?;
        let (checksum, motion, event_score, entered2, exited2, entered3, exited3) = world
            .with_numeric_physics(|_, q| -> Result<_, QueryError> {
                let ray = RayQuery {
                    scene_id: id(99),
                    origin: [-8., 1., 0.],
                    direction: [2., 0., 0.],
                    max_distance: 20.,
                    include_sensors: false,
                    exclude_entity: Some(id(11)),
                    memberships: u32::MAX,
                    filter: u32::MAX,
                };
                let mut checksum = 0.;
                for (i, ray) in [
                    ray.clone(),
                    RayQuery {
                        include_sensors: true,
                        ..ray.clone()
                    },
                    RayQuery { filter: 0, ..ray },
                ]
                .iter()
                .enumerate()
                {
                    checksum += (i + 1) as f64
                        * q.raycast(ray)?
                            .map_or(-1., |h| hit(h.entity_id, h.distance, h.point, h.normal));
                }
                let ray = RayQuery2D {
                    origin: [0.5, 3.],
                    direction: [0., -2.],
                    max_distance: 10.,
                    include_sensors: false,
                    exclude_entity: Some(id(2)),
                    memberships: u32::MAX,
                    filter: u32::MAX,
                };
                for (i, ray) in [
                    ray.clone(),
                    RayQuery2D {
                        include_sensors: true,
                        ..ray.clone()
                    },
                    RayQuery2D {
                        origin: [1.5, 3.],
                        ..ray
                    },
                ]
                .iter()
                .enumerate()
                {
                    checksum += (i + 4) as f64
                        * q.raycast_2d(ray)?
                            .map_or(-1., |h| hit(h.entity, h.distance, h.point, h.normal));
                }
                let c = q.compute_character_motion(
                    &CharacterQuery {
                        scene_id: id(99),
                        entity_id: id(11),
                        translation: [5., -1., 0.],
                        options: CharacterOptions::default(),
                    },
                    dt,
                )?;
                let mut score = f64::from(u8::from(c.grounded))
                    + 2. * f64::from(u8::from(c.sliding_down_slope));
                for (i, v) in c.translation.iter().enumerate() {
                    score += v * (i + 1) as f64;
                }
                for id in &c.collisions {
                    score += f64::from(id.0[15]);
                }
                checksum += 7. * score;
                let events2 = q.trigger_events_2d()?;
                let events3 = q.trigger_events()?;
                assert!(events3.iter().all(|event| event.scene_id == id(99)));
                let score2: f64 = events2
                    .iter()
                    .map(|e| {
                        f64::from(e.first.0[15])
                            + f64::from(e.second.0[15]) * 7.
                            + f64::from(u8::from(e.entered)) * 101.
                    })
                    .sum();
                let score3: f64 = events3
                    .iter()
                    .map(|e| {
                        f64::from(e.first.0[15])
                            + f64::from(e.second.0[15]) * 7.
                            + f64::from(u8::from(e.entered)) * 101.
                    })
                    .sum();
                Ok((
                    checksum,
                    c.translation,
                    score2 + score3 * 1000.,
                    events2.iter().filter(|e| e.entered).count(),
                    events2.iter().filter(|e| !e.entered).count(),
                    events3.iter().filter(|e| e.entered).count(),
                    events3.iter().filter(|e| !e.entered).count(),
                ))
            })??;
        world.with_numeric_columns(|columns| {
            columns.for_each_chunk(|chunk| {
                for (entity, t) in chunk
                    .ids
                    .iter()
                    .zip(chunk.transforms.as_chunks_mut::<10>().0)
                {
                    if *entity == id(20) {
                        t[0] = checksum;
                        t[1] = event_score;
                        t[2] += 1.;
                    }
                    if *entity == id(11) {
                        for i in 0..3 {
                            t[i] += motion[i];
                        }
                        if clock == 3 {
                            t[2] += 10.;
                        }
                    }
                    if *entity == id(2) {
                        t[0] += 10.;
                    }
                }
            })
        });
        world.apply(&mut FrameCommands::default())?;
        println!(
            "{{\"tick\":{clock},\"marker\":{:?},\"character\":{:?},\"planar\":{:?},\"events\":[{entered2},{exited2},{entered3},{exited3}]}}",
            world.inspect(id(20)).unwrap().transform.translation,
            world.inspect(id(11)).unwrap().transform.translation,
            world.inspect(id(2)).unwrap().transform.translation
        );
    }
    Ok(())
}
