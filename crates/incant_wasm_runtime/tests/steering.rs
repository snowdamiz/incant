use incant_runtime::{CookedEntity, CookedScene, NativeWorld, StableId};
use incant_types::Transform;
use incant_wasm_abi::{self as abi, steering as wire};
use incant_wasm_runtime::{Call, Capability};
const DT: f64 = 1. / 60.;
fn world() -> NativeWorld {
    NativeWorld::new(
        CookedScene::new(
            StableId([0; 16]),
            64,
            vec![CookedEntity {
                id: StableId([1; 16]),
                parent: None,
                transform: Transform::default(),
                velocity: None,
            }],
        )
        .unwrap(),
    )
}
fn agents() -> [abi::SteeringAgent; 2] {
    [
        abi::SteeringAgent {
            id: 9u128.to_be_bytes(),
            position: [-1., 0., 0.],
            velocity: [1., 0.],
            preferred_velocity: [1., 0.],
            radius: 0.3,
            height: 1.8,
            max_speed: 1.,
            responsibility: 1.,
        },
        abi::SteeringAgent {
            id: 2u128.to_be_bytes(),
            position: [1., 0., 0.],
            velocity: [-1., 0.],
            preferred_velocity: [-1., 0.],
            radius: 0.3,
            height: 1.8,
            max_speed: 1.,
            responsibility: 1.,
        },
    ]
}
fn request(agents: &[abi::SteeringAgent], memory: &mut [u8]) -> i32 {
    wire::encode_request(
        wire::Request {
            agents,
            obstacles: &[],
            time_horizon: 2.,
            obstacle_time_horizon: 2.,
            margin: 0.05,
        },
        memory,
    )
    .unwrap() as i32
}
fn prefix(cap: &mut Capability, memory: &mut [u8], epoch: i64) {
    cap.invoke(
        memory,
        epoch,
        Call::Column {
            chunk: 0,
            ptr: 0,
            velocity: false,
            write: false,
        },
    )
    .unwrap();
    memory[..8].copy_from_slice(&99f64.to_le_bytes());
    cap.invoke(
        memory,
        epoch,
        Call::Column {
            chunk: 0,
            ptr: 0,
            velocity: false,
            write: true,
        },
    )
    .unwrap();
    cap.invoke(
        memory,
        epoch,
        Call::Log {
            level: 1,
            ptr: 0,
            len: 0,
        },
    )
    .unwrap();
    memory[..16].copy_from_slice(&[1; 16]);
    cap.invoke(memory, epoch, Call::Despawn(0)).unwrap();
}
#[test]
fn proposals_match_native_solver_and_only_explicit_numeric_writes_mutate_world() {
    let mut world = world();
    let before = world.snapshot();
    let mut cap = Capability::default();
    let mut memory = vec![0; abi::STEERING_MAX_REQUEST_BYTES];
    let epoch = cap.begin(&mut world, DT).unwrap();
    let agents = agents();
    let len = request(&agents, &mut memory);
    let output = cap
        .invoke(&mut memory, epoch, Call::Steer { ptr: 0, len })
        .unwrap();
    memory.fill(55);
    cap.invoke(
        &mut memory,
        epoch,
        Call::ReadSteering {
            ptr: 1,
            capacity: output,
        },
    )
    .unwrap();
    assert_eq!(memory[0], 55);
    let rows: Vec<_> = wire::decode_result(&memory[1..output as usize + 1])
        .unwrap()
        .iter()
        .collect();
    let native = incant_nav::steer(
        &incant_runtime::SteeringQuery {
            agents: agents
                .map(|a| incant_runtime::SteeringAgent {
                    id: StableId(a.id),
                    position: a.position.map(|v| v as f32),
                    velocity: a.velocity.map(|v| v as f32),
                    preferred_velocity: a.preferred_velocity.map(|v| v as f32),
                    radius: a.radius as f32,
                    height: a.height as f32,
                    max_speed: a.max_speed as f32,
                    responsibility: a.responsibility as f32,
                })
                .to_vec(),
            obstacles: vec![],
            time_horizon: 2.,
            obstacle_time_horizon: 2.,
            margin: 0.05,
        },
        DT as f32,
    )
    .unwrap();
    for (row, expected) in rows.iter().zip(native) {
        assert_eq!(row.id, expected.id.0);
        assert_eq!(row.velocity, expected.velocity.map(f64::from));
        assert_eq!(row.neighbors, expected.neighbors);
    }
    cap.finish(&mut world).unwrap();
    assert_eq!(world.snapshot(), before);
    let epoch = cap.begin(&mut world, DT).unwrap();
    assert!(
        cap.invoke(
            &mut memory,
            epoch,
            Call::ReadSteering {
                ptr: 0,
                capacity: output
            }
        )
        .is_err()
    );
    assert!(cap.finish(&mut world).is_err());
}
#[test]
fn malformed_ingress_or_output_refuses_complete_transaction_even_if_caller_ignores_error() {
    for case in 0..10 {
        let mut world = world();
        let before = world.save_bytes().unwrap();
        let mut cap = Capability::default();
        let mut memory = vec![0; abi::STEERING_MAX_REQUEST_BYTES];
        let epoch = cap.begin(&mut world, DT).unwrap();
        prefix(&mut cap, &mut memory, epoch);
        let mut agents = agents();
        if case == 5 {
            agents[1].id = agents[0].id;
        }
        if case == 6 {
            agents[0].radius = -1.;
        }
        if case == 7 {
            agents[0].position[0] = f64::MAX;
        }
        let mut len = request(&agents, &mut memory);
        let mut ptr = 0;
        match case {
            0 => len = -1,
            1 => len += 1,
            2 => ptr = -1,
            3 => len = abi::STEERING_MAX_REQUEST_BYTES as i32 + 1,
            4 => memory[0] = 0,
            _ => {}
        }
        let result = if case < 8 {
            cap.invoke(&mut memory, epoch, Call::Steer { ptr, len })
        } else {
            let out = cap
                .invoke(&mut memory, epoch, Call::Steer { ptr: 0, len })
                .unwrap();
            memory.fill(77);
            let r = cap.invoke(
                &mut memory,
                epoch,
                Call::ReadSteering {
                    ptr: if case == 9 { i32::MAX } else { 0 },
                    capacity: if case == 9 { out } else { out - 1 },
                },
            );
            assert!(memory.iter().all(|b| *b == 77));
            r
        };
        assert!(result.is_err(), "case{case}");
        assert!(cap.finish(&mut world).is_err());
        assert_eq!(world.save_bytes().unwrap(), before, "case{case}");
        assert!(cap.logs().is_err());
    }
}
#[test]
fn aggregate_agent_query_limits_and_stale_epoch_checks_cannot_be_bypassed() {
    for dt in [DT, 1. / 240. - f64::EPSILON] {
        let mut world = world();
        let mut cap = Capability::default();
        let mut memory = vec![0; abi::STEERING_MAX_REQUEST_BYTES];
        let epoch = cap.begin(&mut world, dt).unwrap();
        let len = request(&[], &mut memory);
        assert!(
            cap.invoke(&mut memory, epoch - 1, Call::Steer { ptr: 0, len })
                .is_err()
        );
        if dt < 1. / 240. {
            assert!(
                cap.invoke(&mut memory, epoch, Call::Steer { ptr: 0, len })
                    .is_err()
            );
            assert!(cap.finish(&mut world).is_err());
            continue;
        }
        for _ in 0..32 {
            cap.invoke(&mut memory, epoch, Call::Steer { ptr: 0, len })
                .unwrap();
        }
        assert!(
            cap.invoke(&mut memory, epoch, Call::Steer { ptr: 0, len })
                .is_err()
        );
        assert!(cap.finish(&mut world).is_err());
    }
    let mut world = world();
    let mut cap = Capability::default();
    let mut memory = vec![0; abi::STEERING_MAX_REQUEST_BYTES];
    let epoch = cap.begin(&mut world, DT).unwrap();
    let agents: Vec<_> = (0..128u128)
        .map(|n| abi::SteeringAgent {
            id: n.to_be_bytes(),
            ..agents()[0]
        })
        .collect();
    let len = request(&agents, &mut memory);
    cap.invoke(&mut memory, epoch, Call::Steer { ptr: 0, len })
        .unwrap();
    let len = request(&agents[..1], &mut memory);
    assert!(
        cap.invoke(&mut memory, epoch, Call::Steer { ptr: 0, len })
            .is_err()
    );
    assert!(cap.finish(&mut world).is_err());
}

#[test]
fn original_f64_domains_reject_before_solver_rounding_and_roll_back_all_prefixes() {
    let mut wrongly_accepted = Vec::new();
    for case in 0..23 {
        let mut world = world();
        let before = world.save_bytes().unwrap();
        let mut cap = Capability::default();
        let mut memory = vec![0; abi::STEERING_MAX_REQUEST_BYTES];
        let epoch = cap.begin(&mut world, DT).unwrap();
        prefix(&mut cap, &mut memory, epoch);
        let mut agents = agents();
        let mut points = [[0., -2.], [1., -2.]];
        let (mut horizon, mut obstacle_horizon, mut margin) = (2., 2., 0.05);
        let (mut min_y, mut max_y) = (0., 2.);
        match case {
            0 => margin = -1e-50,
            1 => agents[0].radius = 0.0499999999,
            2 => agents[0].position[0] = 10000.00001,
            3 => horizon = DT - 1e-12,
            4 => obstacle_horizon = DT - 1e-12,
            5 => agents[0].velocity = [30.0000001, 0.],
            6 => agents[0].preferred_velocity = [30.0000001, 0.],
            7 => agents[0].radius = 4.00000001,
            8 => agents[0].height = 0.0999999999,
            9 => agents[0].height = 10.00000001,
            10 => agents[0].max_speed = -1e-50,
            11 => agents[0].max_speed = 30.0000001,
            12 => agents[0].responsibility = 0.0999999999,
            13 => agents[0].responsibility = 10.00000001,
            14 => margin = 1.00000001,
            15 => horizon = 10.00000001,
            16 => obstacle_horizon = 10.00000001,
            17 => min_y = -10000.00001,
            18 => {
                min_y = 10000.00001;
                max_y = 10001.;
            }
            19 => max_y = 10010.00001,
            20 => {
                min_y = -10000.;
                max_y = -10000.00001;
            }
            21 => points[0][0] = 10000.00001,
            22 => agents[0].position[1] = -10000.00001,
            _ => unreachable!(),
        }
        let obstacles = [wire::Obstacle {
            id: 7u128.to_be_bytes(),
            vertices: &points,
            closed: false,
            min_y,
            max_y,
        }];
        let len = wire::encode_request(
            wire::Request {
                agents: &agents,
                obstacles: &obstacles,
                time_horizon: horizon,
                obstacle_time_horizon: obstacle_horizon,
                margin,
            },
            &mut memory,
        )
        .unwrap() as i32;
        if cap
            .invoke(&mut memory, epoch, Call::Steer { ptr: 0, len })
            .is_ok()
        {
            wrongly_accepted.push(case);
            continue;
        }
        assert!(cap.finish(&mut world).is_err(), "case {case}");
        assert_eq!(world.save_bytes().unwrap(), before, "case {case}");
        assert!(cap.logs().is_err());
    }
    assert!(
        wrongly_accepted.is_empty(),
        "rounded invalid f64 cases: {wrongly_accepted:?}"
    );
}
