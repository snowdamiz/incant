use incant_wasm_abi::{
    self as abi,
    localization::{self as wire, MissingKind, report::*},
};
const TABLE: &str = "00000000000000000000000002";
fn row(key: &str) -> Entry<'_> {
    Entry {
        table_id: TABLE,
        key,
        requested_locale: "zh-Hant",
        resolved_locale: Some("en"),
        kind: MissingKind::Translation,
    }
}

#[test]
fn canonical_pages_are_borrowed_complete_and_distinct_from_older_reply_kinds() {
    let mut bytes = vec![0; abi::LOCALIZATION_MAX_RESULT_BYTES];
    let rows = [
        row("a"),
        Entry {
            resolved_locale: None,
            kind: MissingKind::Key,
            ..row("b")
        },
    ];
    let n = encode(4096, &rows, &mut bytes).unwrap();
    let page = decode(&bytes[..n]).unwrap();
    assert_eq!(page.total, 4096);
    assert_eq!(page.len(), 2);
    assert!(!page.is_empty());
    assert_eq!(page.entries().collect::<Vec<_>>(), rows);
    assert_eq!(page.entries().len(), 2);
    assert!(wire::decode_locale(&bytes[..n]).is_err());
    assert!(wire::decode_localized(&bytes[..n]).is_err());
    assert!(wire::decode_formatted(&bytes[..n]).is_err());
    for end in 0..n {
        assert!(decode(&bytes[..end]).is_err(), "truncated {end}");
    }
    assert!(decode(&bytes[..n + 1]).is_err());
    for at in [0, 8, 12, 16, 20, 24, 50, 54, 55, 59, 66, 70, 74, 76] {
        let mut bad = bytes[..n].to_vec();
        bad[at] = 255;
        assert!(decode(&bad).is_err(), "offset {at}");
    }
    for kind in [0u32, 1, 2, 4] {
        let mut bad = bytes[..n].to_vec();
        bad[12..16].copy_from_slice(&kind.to_le_bytes());
        assert!(decode(&bad).is_err());
    }
    let n = encode(4096, &[], &mut bytes).unwrap();
    assert!(decode(&bytes[..n]).unwrap().is_empty());
}

#[test]
fn bounds_order_duplicates_and_invalid_missing_metadata_are_refused() {
    let mut bytes = vec![0; abi::LOCALIZATION_MAX_RESULT_BYTES];
    for (total, rows) in [
        (1, vec![row("b"), row("a")]),
        (2, vec![row("b"), row("a")]),
        (2, vec![row("a"); 2]),
        (4097, vec![]),
        (129, vec![row("a"); 129]),
    ] {
        assert!(encode(total, &rows, &mut bytes).is_err());
    }
    for entry in [
        Entry {
            table_id: "80000000000000000000000000",
            ..row("a")
        },
        row(""),
        row("not a key"),
        Entry {
            kind: MissingKind::Key,
            ..row("a")
        },
    ] {
        assert!(encode(1, &[entry], &mut bytes).is_err());
    }
    for request in [
        Request {
            offset: 4097,
            limit: 1,
        },
        Request {
            offset: 0,
            limit: 0,
        },
        Request {
            offset: 0,
            limit: 129,
        },
        Request {
            offset: u32::MAX,
            limit: 1,
        },
    ] {
        assert!(request.validate().is_err());
    }
    Request::default().validate().unwrap();
    Request {
        offset: 4096,
        limit: 128,
    }
    .validate()
    .unwrap();
    assert!(decode(&vec![0; abi::LOCALIZATION_MAX_RESULT_BYTES + 1]).is_err());
}

#[test]
fn maximum_page_fits_existing_copy_bound() {
    let keys = (0..128)
        .map(|i| format!("{i:03}{}", "x".repeat(abi::LOCALIZATION_MAX_KEY_BYTES - 3)))
        .collect::<Vec<_>>();
    let locale = "x".repeat(abi::LOCALIZATION_MAX_LOCALE_BYTES);
    let rows = keys
        .iter()
        .map(|key| Entry {
            requested_locale: &locale,
            resolved_locale: Some(&locale),
            ..row(key)
        })
        .collect::<Vec<_>>();
    let mut bytes = vec![0; abi::LOCALIZATION_MAX_RESULT_BYTES];
    let n = encode(4096, &rows, &mut bytes).unwrap();
    assert!(n <= abi::LOCALIZATION_MAX_RESULT_BYTES);
    assert_eq!(
        decode(&bytes[..n]).unwrap().entries().collect::<Vec<_>>(),
        rows
    );
    assert!(encode(4096, &rows, &mut bytes[..n - 1]).is_err());
}
