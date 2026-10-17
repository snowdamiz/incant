use super::*;
#[test]
fn bytecode_v1_through_v6_keep_exact_published_encoding() {
    // Fixed data sections and independently assembled header/digest fixtures.
    let expected = [
        "cb9b2fa8d2105681b0dea0604505123ae3494b7cd7bab05f52f2e67ba58107c4",
        "36f185ab3c2cfe43a2db1d906f4638db556ab7c047c86ad579058e679fe14561",
        "784c631ffb4d4bda18ed7ddf605740a92317043312dfd4f4c35cce0d432c267e",
        "2d43b5180c7760a994df2b4629cfcc9c786c1d69784c962cad95f868796d40ad",
        "a0d4faf2f07a0f0af6fc0b34aaf14f4754dd737002c019a1354c1eb327d832af",
        "dba4da339c7c90e6fd2f40d5afbc75b8edc346dfa72b7e0e52dc7755812e7917",
    ];
    for version in 1..=6 {
        #[cfg(not(feature = "native-checkpoints"))]
        if matches!(version, 4 | 6) {
            continue;
        }
        let mut schedule = schedule::Schedule::default();
        if version >= 5 {
            let mut tasks = crate::coroutines::Store::default();
            tasks
                .insert(
                    schedule::Key {
                        scope: Scope::Global,
                        id: "task".into(),
                    },
                    Arc::new(crate::coroutines::Task {
                        routine: "routine".into(),
                        entity: None,
                        due: 1,
                        value: ScriptValue::Null,
                        cost: Cost { bytes: 1, nodes: 1 },
                    }),
                )
                .unwrap();
            schedule.coroutines = Some(tasks);
        }
        let ui = (version >= 2).then_some(&b"ui"[..]);
        let audio = (version >= 3).then_some(&b"audio"[..]);
        let format = match version {
            #[cfg(feature = "native-checkpoints")]
            4 | 6 => Format::Persisted,
            _ => Format::Ordinary,
        };
        let bytes = encode_for(
            [7; 32],
            [8; 32],
            b"world",
            &[],
            &Store::default(),
            &schedule,
            ui,
            audio,
            format,
        )
        .unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(&bytes)),
            expected[version - 1]
        );
        let decoded = sections_for(&bytes, [7; 32], format).unwrap();
        assert_eq!(decoded.world, b"world");
    }
}

#[cfg(any(feature = "wasmtime-host", feature = "static-module"))]
#[test]
fn wasm_sections_reject_wrong_kind_identity_counts_and_total_before_allocation() {
    let declaration = incant_wasm_runtime::persistent::Declaration::new(16, [1; 32]).unwrap();
    let mut state = declaration.encode().to_vec();
    state.extend([0u8; 32]);
    let bytes = wasm_encode(
        [7; 32],
        [8; 32],
        b"world",
        &state,
        Some(b"ui"),
        Some(b"audio"),
        false,
    )
    .unwrap();
    assert_eq!(
        wasm_sections(&bytes, [7; 32], false).unwrap().wasm,
        Some(state.as_slice())
    );
    assert!(sections(&bytes, [7; 32]).is_err());
    assert!(wasm_sections(&bytes, [8; 32], false).is_err());
    assert!(wasm_sections(&bytes, [7; 32], true).is_err());
    for (offset, value) in [
        (8, 1u32),
        (12, 0),
        (80, u32::MAX),
        (84, 72),
        (88, u32::MAX),
        (92, u32::MAX),
    ] {
        let mut bad = bytes.clone();
        bad[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        let checksum = digest(&bad, HEADER);
        bad[96..128].copy_from_slice(&checksum);
        assert!(wasm_sections(&bad, [7; 32], false).is_err());
    }
    for n in 0..bytes.len() {
        assert!(wasm_sections(&bytes[..n], [7; 32], false).is_err());
    }
}
