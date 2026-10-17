use incant_wasm_abi::{self as abi, coroutines::*};

#[test]
fn catalog_round_trip_and_every_truncation_refuses() {
    let routines = [
        Routine::new("walk", 8, [1; 32]).unwrap(),
        Routine::new("jump", 16, [2; 32]).unwrap(),
    ];
    let bytes = encode_catalog::<224>(&routines);
    assert_eq!(
        Catalog::parse(&bytes)
            .unwrap()
            .routines()
            .collect::<Vec<_>>(),
        routines
    );
    for len in 0..bytes.len() {
        assert!(Catalog::parse(&bytes[..len]).is_err());
    }
    for at in [0, 8, 12, 16 + 64, 16 + 69] {
        let mut bad = bytes;
        bad[at] = 255;
        assert!(Catalog::parse(&bad).is_err(), "offset {at}");
    }
    let mut duplicate = bytes;
    duplicate[120..].copy_from_slice(&bytes[16..120]);
    assert!(Catalog::parse(&duplicate).is_err());
    for (name, size, schema) in [
        ("", 8, [1; 32]),
        ("with space", 8, [1; 32]),
        ("ok", 0, [1; 32]),
        ("ok", 16385, [1; 32]),
        ("ok", 1, [0; 32]),
    ] {
        assert!(Routine::new(name, size, schema).is_none());
    }
}

#[test]
fn wire_frames_reject_reserved_values_extra_bytes_and_invalid_commands() {
    let (id, id_len) = abi::timers::encode_id("task").unwrap();
    let (routine, routine_len) = abi::timers::encode_id("walk").unwrap();
    let header = abi::CoroutineHeader {
        magic: *abi::COROUTINE_MAGIC,
        kind: 0,
        delay: 0,
        id_len,
        routine_len,
        flags: 0,
        state_len: 8,
        id,
        routine,
        entity: [0; 16],
    };
    let bytes = [header.encode().as_slice(), &[0; 8]].concat();
    assert!(frame(&bytes).is_ok());
    for len in 0..bytes.len() {
        assert!(frame(&bytes[..len]).is_err());
    }
    for change in [
        |h: &mut abi::CoroutineHeader| h.kind = 5,
        |h: &mut abi::CoroutineHeader| h.delay = 1,
        |h: &mut abi::CoroutineHeader| h.flags = 2,
        |h: &mut abi::CoroutineHeader| h.entity[0] = 1,
        |h: &mut abi::CoroutineHeader| h.id[63] = 1,
        |h: &mut abi::CoroutineHeader| h.state_len = 9,
    ] {
        let mut bad = header;
        change(&mut bad);
        assert!(frame(&[bad.encode().as_slice(), &[0; 8]].concat()).is_err());
    }
    let mut extra = bytes;
    extra.push(0);
    assert!(frame(&extra).is_err());
}
