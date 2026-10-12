use incant_module::{AudioDeclarations, EntityId};
use incant_wasm_abi::{self as abi, audio};
fn id(n: u128) -> [u8; 16] {
    n.to_be_bytes()
}
fn empty(n: u128) -> abi::AudioRow {
    let mut r = abi::AudioRow::decode(&[0; abi::AudioRow::SIZE]).unwrap();
    r.entity = id(n);
    r
}
fn bus(n: u128, parent: Option<u128>) -> abi::AudioRow {
    let mut r = empty(n);
    r.roles = 1;
    if let Some(parent) = parent {
        r.bus_parent_present = 1;
        r.bus_parent = id(parent);
    }
    r.bus_gain = -3.;
    r
}
fn source(n: u128) -> abi::AudioRow {
    let mut r = empty(n);
    r.roles = 2;
    r.clip = id(900);
    r.source_bus_present = 1;
    r.source_bus = id(1);
    r.source_flags = 7;
    r.transport_epoch = u64::MAX;
    r.gain = -8.;
    r.pan = 0.25;
    r.rate = 1.13;
    r.start_seconds = 0.00037;
    r
}
fn frame(rows: &[abi::AudioRow]) -> Vec<u8> {
    let mut bytes = abi::AudioHeader {
        magic: *abi::AUDIO_MAGIC,
        scene: id(99),
        tick: u64::MAX - 17,
        count: rows.len() as u32,
        stride: abi::AudioRow::SIZE as u32,
    }
    .encode()
    .to_vec();
    for row in rows {
        bytes.extend(row.encode());
    }
    bytes
}
#[test]
fn owned_unaligned_frame_preserves_complete_values_and_exact_transport_identity() {
    let mut rows = vec![bus(1, Some(4)), source(2), bus(4, None)];
    let mut both = source(5);
    both.roles = 6;
    both.source_flags |= 8;
    both.pan = 0.;
    both.world_translation = [0.5, -1., 20.];
    both.min_distance = f64::from(0.01_f32);
    both.max_distance = f64::from(1_000_000_f32);
    both.listener_rotation = [0., 0., 0., 1.];
    rows.push(both);
    let bytes = frame(&rows);
    let mut unaligned = vec![0xa5];
    unaligned.extend(&bytes);
    let s = AudioDeclarations::parse(&unaligned[1..]).unwrap();
    assert_eq!(s.scene(), EntityId(id(99)));
    assert_eq!(s.tick(), u64::MAX - 17);
    assert_eq!(s.len(), 4);
    assert!(!s.is_empty());
    assert_eq!(s.iter().count(), 4);
    let source = s.find(EntityId(id(2))).unwrap().source.unwrap();
    assert_eq!(source.transport_epoch, u64::MAX);
    assert_eq!(source.clip, EntityId(id(900)));
    assert_eq!(source.bus, Some(EntityId(id(1))));
    assert_eq!(
        (
            source.gain_db,
            source.pan,
            source.rate,
            source.start_seconds
        ),
        (-8., 0.25, 1.13, 0.00037)
    );
    assert!(source.playing && source.looping && source.streaming);
    assert!(source.spatial.is_none());
    let node = s.find(EntityId(id(5))).unwrap();
    assert_eq!(node.listener.unwrap().world_position, [0.5, -1., 20.]);
    assert_eq!(
        node.source.unwrap().spatial.unwrap().min_distance,
        f64::from(0.01_f32)
    );
    assert_eq!(s.find(EntityId(id(4))).unwrap().bus.unwrap().parent, None);
    assert!(s.find(EntityId(id(0))).is_none());
    assert!(s.find(EntityId(id(3))).is_none());
    assert!(s.find(EntityId(id(6))).is_none());
}
#[test]
fn frame_admission_rejects_short_trailing_unknown_unsorted_or_inconsistent_rows() {
    let original = frame(&[bus(1, None), source(2)]);
    for len in 0..original.len() {
        assert!(AudioDeclarations::parse(&original[..len]).is_err(), "{len}");
    }
    let mut trailing = original.clone();
    trailing.push(0);
    assert!(AudioDeclarations::parse(&trailing).is_err());
    for change in [
        |r: &mut abi::AudioRow| r.roles = 0,
        |r: &mut abi::AudioRow| r.roles = 3,
        |r: &mut abi::AudioRow| r.roles = 8,
        |r: &mut abi::AudioRow| r.source_flags = 16,
        |r: &mut abi::AudioRow| r.source_bus_present = 2,
        |r: &mut abi::AudioRow| r.source_bus = id(88),
        |r: &mut abi::AudioRow| r.bus_parent = id(88),
        |r: &mut abi::AudioRow| r.bus_gain = -0.,
        |r: &mut abi::AudioRow| r.gain = f64::NAN,
        |r: &mut abi::AudioRow| r.gain = 24.000_000_001,
        |r: &mut abi::AudioRow| r.gain = 0.1,
        |r: &mut abi::AudioRow| r.rate = 4.000_000_001,
        |r: &mut abi::AudioRow| r.start_seconds = 3_600.000_000_001,
        |r: &mut abi::AudioRow| r.min_distance = 1.,
        |r: &mut abi::AudioRow| r.world_translation = [1., 0., 0.],
        |r: &mut abi::AudioRow| r.listener_rotation = [0., 0., 0., 1.],
    ] {
        let mut row = source(2);
        change(&mut row);
        assert!(AudioDeclarations::parse(&frame(&[bus(1, None), row])).is_err());
    }
    for rows in [
        vec![source(2), bus(1, None)],
        vec![bus(1, None), bus(1, None)],
        vec![bus(1, Some(1))],
        vec![bus(1, Some(2)), bus(2, Some(1))],
        vec![bus(1, Some(2)), source(2)],
        vec![bus(1, Some(9))],
    ] {
        assert!(AudioDeclarations::parse(&frame(&rows)).is_err());
    }
}
#[test]
fn full_bounded_bus_chain_and_sources_are_accepted_without_relaxing_limits() {
    let mut rows: Vec<_> = (1..=32)
        .map(|n| bus(n, (n < 32).then_some(n + 1)))
        .collect();
    rows.extend((33..=160).map(source));
    let mut listener = empty(161);
    listener.roles = 4;
    listener.listener_rotation = [0., 0., 0., 1.];
    rows.push(listener);
    let bytes = frame(&rows);
    assert_eq!(bytes.len(), abi::AUDIO_MAX_BYTES);
    assert_eq!(AudioDeclarations::parse(&bytes).unwrap().len(), 161);
    rows.push(source(162));
    assert!(AudioDeclarations::parse(&frame(&rows)).is_err());
    rows.pop();
    rows[159] = bus(160, None); // 33 buses, despite remaining within total row bound.
    assert!(AudioDeclarations::parse(&frame(&rows)).is_err());
}
#[test]
fn spatial_and_listener_constraints_and_empty_scene_are_explicit() {
    assert!(AudioDeclarations::parse(&frame(&[])).unwrap().is_empty());
    let mut spatial = source(2);
    spatial.source_flags = 8;
    spatial.pan = 0.;
    spatial.min_distance = 1.;
    spatial.max_distance = 10.;
    let mut listener = empty(3);
    listener.roles = 4;
    listener.listener_rotation = [0., 0., 0., 1.];
    assert!(AudioDeclarations::parse(&frame(&[bus(1, None), spatial, listener])).is_ok());
    assert!(AudioDeclarations::parse(&frame(&[bus(1, None), spatial])).is_err());
    let mut duplicate = listener;
    duplicate.entity = id(4);
    assert!(
        AudioDeclarations::parse(&frame(&[bus(1, None), spatial, listener, duplicate])).is_err()
    );
    for bad in [[0.; 4], [0., 0., 0., 2.], [f64::INFINITY, 0., 0., 1.]] {
        listener.listener_rotation = bad;
        assert!(AudioDeclarations::parse(&frame(&[bus(1, None), spatial, listener])).is_err());
    }
}
fn batch(row: abi::AudioCommand) -> Vec<u8> {
    let mut out = abi::AudioCommandHeader {
        magic: *abi::AUDIO_COMMAND_MAGIC,
        count: 1,
        stride: abi::AudioCommand::SIZE as u32,
    }
    .encode()
    .to_vec();
    out.extend(row.encode());
    out
}
#[test]
fn commands_validate_original_f64_and_entire_batch_before_exposing_rows() {
    for (kind, good, bad) in [
        (0, 1., 2.),
        (1, 0., 1.),
        (2, 0., -0.),
        (3, 3600., 3_600.000_000_001),
        (4, 24., 24.000_000_001),
        (5, -80., -80.000_000_001),
    ] {
        let mut r = abi::AudioCommand {
            entity: id(2),
            kind,
            reserved: 0,
            value: good,
        };
        assert_eq!(audio::Commands::parse(&batch(r)).unwrap().iter().count(), 1);
        for v in [bad, f64::NAN, f64::INFINITY] {
            r.value = v;
            assert!(audio::Commands::parse(&batch(r)).is_err());
        }
    }
    let r = abi::AudioCommand {
        entity: id(2),
        kind: u32::MAX,
        reserved: 0,
        value: 0.,
    };
    assert!(audio::Commands::parse(&batch(r)).is_err());
    let mut r = r;
    r.kind = 2;
    r.reserved = 1;
    assert!(audio::Commands::parse(&batch(r)).is_err());
    let mut bytes = batch(abi::AudioCommand {
        entity: id(2),
        kind: 2,
        reserved: 0,
        value: 0.,
    });
    for n in 0..bytes.len() {
        assert!(audio::Commands::parse(&bytes[..n]).is_err());
    }
    bytes.push(0);
    assert!(audio::Commands::parse(&bytes).is_err());
}
