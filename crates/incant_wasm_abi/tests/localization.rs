use incant_wasm_abi::{self as abi, localization::*};
const TABLE: &str = "00000000000000000000000002";
#[test]
fn typed_frames_round_trip_unicode_dates_numbers_and_exact_metadata() {
    let args = [
        Argument {
            name: "name",
            value: MessageArgument::Text("雪🙂\0"),
        },
        Argument {
            name: "n",
            value: MessageArgument::Number(21.),
        },
        Argument {
            name: "date",
            value: MessageArgument::Date(CalendarDate {
                year: -1,
                month: 2,
                day: 28,
            }),
        },
    ];
    let mut buffer = vec![0; abi::LOCALIZATION_MAX_REQUEST_BYTES];
    let len = encode_request(
        LocalizeRequest {
            table_id: TABLE,
            key: "hello",
            arguments: &args,
        },
        &mut buffer,
    )
    .unwrap();
    let decoded = decode_request(&buffer[..len]).unwrap();
    assert_eq!(decoded.arguments().collect::<Vec<_>>(), args);
    assert_eq!(
        decoded.input_bytes,
        TABLE.len() + 5 + 4 + "雪🙂\0".len() + 1 + 16 + 4 + 16
    );
    let settings = LocaleSettings {
        locale: "zh-Hant",
        fallbacks: &["en", "fr"],
        pseudo: true,
    };
    let len = encode_settings(settings, &mut buffer).unwrap();
    let decoded = decode_settings(&buffer[..len]).unwrap();
    assert_eq!(decoded.locale, "zh-Hant");
    assert!(decoded.pseudo);
    assert_eq!(decoded.fallbacks().collect::<Vec<_>>(), ["en", "fr"]);
    for kind in [MissingKind::Translation, MissingKind::Key] {
        let value = LocalizedText {
            text: "你好 🙂",
            requested_locale: "zh-Hant",
            resolved_locale: if kind == MissingKind::Key {
                None
            } else {
                Some("en")
            },
            missing: Some(MissingString {
                table_id: TABLE,
                key: "hello",
                kind,
            }),
        };
        let len = encode_localized(value, &mut buffer).unwrap();
        assert_eq!(decode_localized(&buffer[..len]).unwrap(), value);
    }
}
#[test]
fn malformed_versions_lengths_utf8_tags_and_trailing_bytes_are_refused() {
    let mut buffer = vec![0; abi::LOCALIZATION_MAX_REQUEST_BYTES];
    let len = encode_request(
        LocalizeRequest {
            table_id: TABLE,
            key: "hello",
            arguments: &[],
        },
        &mut buffer,
    )
    .unwrap();
    for offset in [0, 8, 12, 38, 42, 47] {
        let mut altered = buffer[..len].to_vec();
        altered[offset] = 255;
        assert!(decode_request(&altered).is_err(), "offset {offset}");
    }
    for end in 0..len {
        assert!(decode_request(&buffer[..end]).is_err());
    }
    assert!(decode_request(&buffer[..len + 1]).is_err());
    assert!(decode_request(&vec![0; abi::LOCALIZATION_MAX_REQUEST_BYTES + 1]).is_err());
    let args = [Argument {
        name: "a",
        value: MessageArgument::Number(1.),
    }];
    let len = encode_request(
        LocalizeRequest {
            table_id: TABLE,
            key: "k",
            arguments: &args,
        },
        &mut buffer,
    )
    .unwrap();
    // Last argument's tag precedes its 8-byte number.
    buffer[len - 12] = 3;
    assert!(decode_request(&buffer[..len]).is_err());
}
#[test]
fn primitive_domain_duplicates_and_logical_limit_are_checked_before_host_allocation() {
    let mut buffer = vec![0; abi::LOCALIZATION_MAX_REQUEST_BYTES];
    for value in [
        MessageArgument::Number(f64::NAN),
        MessageArgument::Number(9_007_199_254_740_992.),
        MessageArgument::Date(CalendarDate {
            year: 2024,
            month: 0,
            day: 1,
        }),
    ] {
        assert!(
            encode_request(
                LocalizeRequest {
                    table_id: TABLE,
                    key: "k",
                    arguments: &[Argument { name: "a", value }]
                },
                &mut buffer
            )
            .is_err()
        );
    }
    let duplicate = Argument {
        name: "a",
        value: MessageArgument::Number(1.),
    };
    assert!(
        encode_request(
            LocalizeRequest {
                table_id: TABLE,
                key: "k",
                arguments: &[duplicate, duplicate]
            },
            &mut buffer
        )
        .is_err()
    );
    assert!(
        encode_settings(
            LocaleSettings {
                locale: "en",
                fallbacks: &["fr", "fr"],
                pseudo: false
            },
            &mut buffer
        )
        .is_err()
    );
    let text = "x".repeat(abi::LOCALIZATION_MAX_INPUT_BYTES - TABLE.len() - 1 - 1);
    let request = LocalizeRequest {
        table_id: TABLE,
        key: "k",
        arguments: &[Argument {
            name: "a",
            value: MessageArgument::Text(&text),
        }],
    };
    let len = encode_request(request, &mut buffer).unwrap();
    assert_eq!(
        decode_request(&buffer[..len]).unwrap().input_bytes,
        abi::LOCALIZATION_MAX_INPUT_BYTES
    );
    let text = text + "x";
    assert!(
        encode_request(
            LocalizeRequest {
                table_id: TABLE,
                key: "k",
                arguments: &[Argument {
                    name: "a",
                    value: MessageArgument::Text(&text)
                }]
            },
            &mut buffer
        )
        .is_err()
    );
}
#[test]
fn settings_and_both_result_variants_require_complete_exact_versioned_frames() {
    let mut bytes = vec![0; abi::LOCALIZATION_MAX_RESULT_BYTES];
    let settings = LocaleSettings {
        locale: "en",
        fallbacks: &["fr"],
        pseudo: true,
    };
    let len = encode_settings(settings, &mut bytes).unwrap();
    for end in 0..len {
        assert!(decode_settings(&bytes[..end]).is_err());
    }
    assert!(decode_settings(&bytes[..len + 1]).is_err());
    // Boolean payloads accept exactly 0 or 1.
    bytes[len - 4] = 2;
    assert!(decode_settings(&bytes[..len]).is_err());
    let len = encode_locale(settings, &mut bytes).unwrap();
    for end in 0..len {
        assert!(decode_locale(&bytes[..end]).is_err());
    }
    assert!(decode_locale(&bytes[..len + 1]).is_err());
    let len = encode_localized(
        LocalizedText {
            text: "Hello",
            requested_locale: "en",
            resolved_locale: Some("en"),
            missing: None,
        },
        &mut bytes,
    )
    .unwrap();
    for end in 0..len {
        assert!(decode_localized(&bytes[..end]).is_err());
    }
    assert!(decode_localized(&bytes[..len + 1]).is_err());
    bytes[len - 4] = 3;
    assert!(decode_localized(&bytes[..len]).is_err());
}
