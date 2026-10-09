use serde_json::json;
use std::time::Instant;
const STEPS: usize = 600;
fn positions() -> impl Iterator<Item = [f32; 3]> {
    (0..8).flat_map(|y| {
        (0..8).flat_map(move |z| {
            (0..8).map(move |x| {
                [
                    x as f32 * 1.2 - 4.2,
                    y as f32 * 1.2 + 1.,
                    z as f32 * 1.2 - 4.2,
                ]
            })
        })
    })
}
fn summarize(name: &str, mut times: Vec<f64>, poses: Vec<u32>) -> (serde_json::Value, Vec<u32>) {
    times.sort_by(f64::total_cmp);
    let hash = poses.iter().fold(0xcbf29ce484222325u64, |h, v| {
        (h ^ u64::from(*v)).wrapping_mul(0x100000001b3)
    });
    (
        json!({"backend":name,"bodies":512,"steps":STEPS,"dt":1./60.,"warmup":30,"median_ms":times[times.len()/2],"p95_ms":times[times.len()*95/100],"position_bits_fnv64":format!("{hash:016x}"),"sorted_samples_ms":times}),
        poses,
    )
}
#[cfg(feature = "jolt")]
fn jolt(workers: u32) -> (serde_json::Value, Vec<u32>) {
    use oxijolt::prelude::math::*;
    use oxijolt::prelude::*;
    let mut world = PhysicsWorld::new(WorldSettings::default().worker_threads(workers)).unwrap();
    let floor = Shape::new_box(Vec3::new(50., 1., 50.)).unwrap();
    world
        .create_body(
            &floor,
            &BodySettings::new_static()
                .position(RVec3::new(0., -1., 0.))
                .friction(0.5)
                .restitution(0.),
        )
        .unwrap();
    let cube = Shape::new_box(Vec3::new(0.45, 0.45, 0.45)).unwrap();
    let bodies: Vec<_> = positions()
        .map(|[x, y, z]| {
            world
                .create_body(
                    &cube,
                    &BodySettings::new_dynamic()
                        .position(RVec3::new(x, y, z))
                        .friction(0.5)
                        .restitution(0.)
                        .linear_damping(0.)
                        .angular_damping(0.)
                        .allow_sleeping(false),
                )
                .unwrap()
        })
        .collect();
    let mut times = Vec::new();
    let mut poses = Vec::new();
    for i in 0..STEPS {
        let t = Instant::now();
        assert!(world.step(1. / 60.).unwrap().is_complete());
        let ms = t.elapsed().as_secs_f64() * 1000.;
        if i >= 30 {
            times.push(ms);
        }
        for id in &bodies {
            let p = world.body(*id).unwrap().position();
            for v in [p.x, p.y, p.z] {
                assert!(v.is_finite() && v.abs() < 100.);
                poses.push(v.to_bits());
            }
        }
    }
    summarize(
        &format!("oxijolt1.0.1-jolt5.6-workers{workers}"),
        times,
        poses,
    )
}
#[cfg(feature = "rapier")]
fn rapier() -> (serde_json::Value, Vec<u32>) {
    use rapier3d::prelude::*;
    let mut world = PhysicsWorld::new();
    world.gravity = Vector::new(0., -9.81, 0.);
    world.integration_parameters.dt = 1. / 60.;
    world.insert_collider(
        ColliderBuilder::cuboid(50., 1., 50.)
            .translation(Vector::new(0., -1., 0.))
            .friction(0.5)
            .restitution(0.),
        None,
    );
    let bodies: Vec<_> = positions()
        .map(|[x, y, z]| {
            world
                .insert(
                    RigidBodyBuilder::dynamic()
                        .translation(Vector::new(x, y, z))
                        .linear_damping(0.)
                        .angular_damping(0.)
                        .can_sleep(false),
                    ColliderBuilder::cuboid(0.45, 0.45, 0.45)
                        .friction(0.5)
                        .restitution(0.),
                )
                .0
        })
        .collect();
    let mut times = Vec::new();
    let mut poses = Vec::new();
    for i in 0..STEPS {
        let t = Instant::now();
        world.step();
        let ms = t.elapsed().as_secs_f64() * 1000.;
        if i >= 30 {
            times.push(ms);
        }
        for id in &bodies {
            let p = world.bodies[*id].translation();
            for v in [p.x, p.y, p.z] {
                assert!(v.is_finite() && v.abs() < 100.);
                poses.push(v.to_bits());
            }
        }
    }
    summarize("rapier0.36-serial", times, poses)
}
fn main() {
    let backend = std::env::args().nth(1).expect("jolt1|jolt4|rapier");
    let (record, poses) = match backend.as_str() {
        #[cfg(feature = "jolt")]
        "jolt1" => jolt(1),
        #[cfg(feature = "jolt")]
        "jolt4" => jolt(4),
        #[cfg(feature = "rapier")]
        "rapier" => rapier(),
        _ => panic!("unknown or unbuilt backend"),
    };
    if let Some(path) = std::env::args().nth(2) {
        let bytes: Vec<u8> = poses.into_iter().flat_map(u32::to_le_bytes).collect();
        std::fs::write(path, bytes).unwrap();
    }
    println!("{record}");
}

#[cfg(all(test, feature = "jolt"))]
mod checks;
