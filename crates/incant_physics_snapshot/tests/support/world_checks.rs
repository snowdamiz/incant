use incant_physics_snapshot::*;
fn budget() -> Budget {
    Budget::new(MAX_ALLOCATION).unwrap()
}
fn section(bytes: &[u8], tag: usize) -> std::ops::Range<usize> {
    let len = |i| u32::from_le_bytes(bytes[52 + i * 8..56 + i * 8].try_into().unwrap()) as usize;
    let start = 224 + (0..tag).map(len).sum::<usize>();
    start..start + len(tag)
}
pub fn hostile(bytes: &[u8], id: Identity) {
    let decode =
        |b: &[u8], budget: &mut Budget| with_decoded_world_boundary(b, id, budget, |_| Ok(()));
    for n in 0..bytes.len() {
        assert!(decode(&bytes[..n], &mut budget()).is_err());
    }
    let mut b = bytes.to_vec();
    b.push(0);
    assert!(decode(&b, &mut budget()).is_err());
    for tag in 0..22 {
        for offset in [48 + tag * 8, 52 + tag * 8] {
            let mut b = bytes.to_vec();
            b[offset..offset + 4].copy_from_slice(&u32::MAX.to_le_bytes());
            let mut q = budget();
            let before = q.remaining();
            assert!(decode(&b, &mut q).is_err());
            assert_eq!(q.remaining(), before);
        }
    }
    for tag in [
        1, 2, 3, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 18, 19, 20, 21,
    ] {
        let mut b = bytes.to_vec();
        b[section(bytes, tag).start + 12] ^= 1;
        assert!(decode(&b, &mut budget()).is_err());
    }
    // Metadata refuses pending journals and unsupported owner families explicitly.
    for word in 0..6 {
        let mut b = bytes.to_vec();
        b[224 + word * 4..228 + word * 4].copy_from_slice(&1u32.to_le_bytes());
        assert_eq!(
            decode(&b, &mut budget()).err(),
            Some(if word < 3 {
                Error::PendingMutation
            } else {
                Error::UnsupportedIsland
            })
        );
    }
    let p = section(bytes, 21).start;
    for (off, word, error) in [
        (60, 0, Error::PipelinePhase),
        (60, 2, Error::PipelinePhase),
        (64, 1, Error::PipelineDeferred),
        (72, 1, Error::PipelineQuarantine),
    ] {
        let mut b = bytes.to_vec();
        b[p + off..p + off + 4].copy_from_slice(&u32::to_le_bytes(word));
        assert_eq!(decode(&b, &mut budget()).err(), Some(error));
    }
    // A callback is never invoked on failed validation, including exhausted budget.
    let mut called = false;
    assert!(
        with_decoded_world_boundary(bytes, id, &mut Budget::new(0).unwrap(), |_| {
            called = true;
            Ok(())
        })
        .is_err()
    );
    assert!(!called);
    let mut q = budget();
    decode(bytes, &mut q).unwrap();
    let used = MAX_ALLOCATION - q.remaining();
    assert!(used > 0);
    decode(bytes, &mut Budget::new(used).unwrap()).unwrap();
    assert!(decode(bytes, &mut Budget::new(used - 1).unwrap()).is_err());
    // Aggregate charge is retained across calls, not reset per section/frame.
    let mut q = Budget::new(used).unwrap();
    decode(bytes, &mut q).unwrap();
    assert!(decode(bytes, &mut q).is_err());
    let other = Identity {
        build: [1; 32],
        ..id
    };
    assert!(with_decoded_world_boundary(bytes, other, &mut budget(), |_| Ok(())).is_err());
    eprintln!(
        "WORLD_FRAME dimension={} bytes={} aggregate_decode_payload={} canonical_sections=22 hostile=true",
        id.dimension as u8,
        bytes.len(),
        used
    );
}
