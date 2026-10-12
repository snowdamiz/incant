use incant_wasm_abi::{self as abi, steering::*};
fn agent(id: u128) -> abi::SteeringAgent {
    abi::SteeringAgent {
        id: id.to_be_bytes(),
        position: [1., 2., 3.],
        velocity: [0.; 2],
        preferred_velocity: [1., 0.],
        radius: 0.3,
        height: 1.8,
        max_speed: 2.,
        responsibility: 1.,
    }
}
#[test]
fn unaligned_bounded_frames_roundtrip_and_every_truncated_prefix_refuses() {
    let agents = [agent(9), agent(2)];
    let obstacles = [Obstacle {
        id: 3u128.to_be_bytes(),
        vertices: &[[0., 0.], [3., 0.], [3., 3.]],
        closed: true,
        min_y: 0.,
        max_y: 3.,
    }];
    let request = Request {
        agents: &agents,
        obstacles: &obstacles,
        time_horizon: 2.,
        obstacle_time_horizon: 1.,
        margin: 0.05,
    };
    let mut bytes = vec![0xcc; abi::STEERING_MAX_REQUEST_BYTES + 1];
    let len = encode_request(request, &mut bytes[1..]).unwrap();
    assert_eq!(bytes[0], 0xcc);
    let raw = &bytes[1..len + 1];
    let decoded = decode_request(raw).unwrap();
    assert_eq!(decoded.agents().collect::<Vec<_>>(), agents);
    assert_eq!(decoded.edges, 3);
    let obstacle = decoded.obstacles().next().unwrap();
    assert_eq!(obstacle.header.id, 3u128.to_be_bytes());
    assert_eq!(
        obstacle.vertices().collect::<Vec<_>>(),
        obstacles[0].vertices
    );
    for end in 0..len {
        assert!(decode_request(&raw[..end]).is_err(), "prefix {end}");
    }
    let mut extended = raw.to_vec();
    extended.push(0);
    assert!(decode_request(&extended).is_err());
    for (offset, value) in [
        (8, 255),
        (12, 255),
        (40 + 2 * abi::SteeringAgent::SIZE + 36, 2),
        (40 + 2 * abi::SteeringAgent::SIZE + 37, 1),
    ] {
        let mut bad = raw.to_vec();
        bad[offset] = value;
        assert!(decode_request(&bad).is_err(), "offset {offset}");
    }
    for offset in [16, 40 + 16, 40 + 2 * abi::SteeringAgent::SIZE + 40] {
        let mut bad = raw.to_vec();
        bad[offset..offset + 8].copy_from_slice(&f64::NAN.to_le_bytes());
        assert!(decode_request(&bad).is_err());
    }
}
#[test]
fn encoder_limits_are_atomic_and_maximum_supported_work_fits_advertised_scratch() {
    let agents = [agent(1); 128];
    let vertices = [[0., 0.], [1., 0.], [0., 1.], [-1., 0.]];
    let obstacles = [Obstacle {
        id: [2; 16],
        vertices: &vertices,
        closed: true,
        min_y: 0.,
        max_y: 1.,
    }; 32];
    let request = Request {
        agents: &agents,
        obstacles: &obstacles,
        time_horizon: 1.,
        obstacle_time_horizon: 1.,
        margin: 0.,
    };
    let mut bytes = vec![55; abi::STEERING_MAX_REQUEST_BYTES];
    let len = encode_request(request, &mut bytes).unwrap();
    assert_eq!(decode_request(&bytes[..len]).unwrap().edges, 128);
    let mut short = vec![55; len - 1];
    assert!(encode_request(request, &mut short).is_err());
    assert!(short.iter().all(|b| *b == 55));
    let mut invalid = agents;
    invalid[127].radius = f64::INFINITY;
    assert!(
        encode_request(
            Request {
                agents: &invalid,
                ..request
            },
            &mut short
        )
        .is_err()
    );
    assert!(short.iter().all(|b| *b == 55));
    let over = [agent(1); 129];
    assert!(
        encode_request(
            Request {
                agents: &over,
                ..request
            },
            &mut bytes
        )
        .is_err()
    );
}
#[test]
fn results_are_exact_sorted_finite_and_copied_without_native_storage() {
    let rows = [
        abi::SteeringVelocity {
            id: 2u128.to_be_bytes(),
            velocity: [0.25, -1.],
            neighbors: 1,
            reserved: 0,
        },
        abi::SteeringVelocity {
            id: 9u128.to_be_bytes(),
            velocity: [-0.25, 1.],
            neighbors: 1,
            reserved: 0,
        },
    ];
    let mut bytes = vec![0; abi::STEERING_MAX_RESULT_BYTES];
    let len = encode_result(&rows, &mut bytes).unwrap();
    assert_eq!(
        decode_result(&bytes[..len])
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        rows
    );
    for end in 0..len {
        assert!(decode_result(&bytes[..end]).is_err());
    }
    let mut duplicate = rows;
    duplicate[1].id = duplicate[0].id;
    assert!(encode_result(&duplicate, &mut bytes).is_err());
    bytes[12] = 1;
    assert!(decode_result(&bytes[..len]).is_err());
    bytes[12] = 0;
    bytes[16 + abi::SteeringVelocity::RESERVED_OFFSET] = 1;
    assert!(decode_result(&bytes[..len]).is_err());
    assert!(encode_result(&[], &mut bytes).is_ok());
    assert!(decode_result(&bytes[..16]).unwrap().is_empty());
}
