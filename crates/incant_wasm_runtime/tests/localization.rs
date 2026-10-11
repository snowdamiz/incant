#[path = "../../incant_runtime/tests/support/localization.rs"]
mod fixture;
use fixture::*;
use incant_runtime::*;
use incant_wasm_abi::{self as abi, localization as wire};
use incant_wasm_runtime::{BoundaryError, Call, Capability, limits};
const DT: f64 = 1. / 60.;
fn setting(cap: &mut Capability, memory: &mut [u8], epoch: i64, locale: &str) {
    let len = wire::encode_settings(
        wire::LocaleSettings {
            locale,
            fallbacks: &[],
            pseudo: false,
        },
        memory,
    )
    .unwrap();
    cap.invoke(
        memory,
        epoch,
        Call::SetLocale {
            ptr: 0,
            len: len as i32,
        },
    )
    .unwrap();
}
fn query(cap: &mut Capability, memory: &mut [u8], epoch: i64, name: &str) -> i32 {
    let len = wire::encode_request(
        wire::LocalizeRequest {
            table_id: TABLE,
            key: "hello",
            arguments: &[wire::Argument {
                name: "name",
                value: wire::MessageArgument::Text(name),
            }],
        },
        memory,
    )
    .unwrap();
    cap.invoke(
        memory,
        epoch,
        Call::Localize {
            ptr: 0,
            len: len as i32,
        },
    )
    .unwrap()
}
fn prefix(cap: &mut Capability, memory: &mut [u8], epoch: i64) {
    let call = Call::Column {
        chunk: 0,
        ptr: 0,
        velocity: false,
        write: false,
    };
    cap.invoke(memory, epoch, call).unwrap();
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
    setting(cap, memory, epoch, "fr");
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
}
#[test]
fn snapshots_are_copied_replace_latest_and_observe_locale_only_after_acceptance() {
    let mut world = NativeWorld::new(scene());
    let mut cap = Capability::default();
    let mut memory = vec![0; abi::LOCALIZATION_MAX_RESULT_BYTES];
    let epoch = cap.begin(&mut world, DT).unwrap();
    setting(&mut cap, &mut memory, epoch, "fr");
    query(&mut cap, &mut memory, epoch, "雪🙂");
    memory.fill(42);
    let len = cap
        .invoke(
            &mut memory,
            epoch,
            Call::ReadLocalized {
                ptr: 0,
                capacity: abi::LOCALIZATION_MAX_RESULT_BYTES as i32,
            },
        )
        .unwrap();
    assert_eq!(
        wire::decode_localized(&memory[..len as usize])
            .unwrap()
            .text,
        "Hello 雪🙂"
    );
    let len = cap.invoke(&mut memory, epoch, Call::Locale).unwrap();
    cap.invoke(
        &mut memory,
        epoch,
        Call::ReadLocalized {
            ptr: 0,
            capacity: len,
        },
    )
    .unwrap();
    assert_eq!(
        wire::decode_locale(&memory[..len as usize]).unwrap().locale,
        "en"
    );
    cap.finish(&mut world).unwrap();
    assert_eq!(world.localization().settings().locale, "fr");
    assert!(
        cap.invoke(
            &mut memory,
            epoch,
            Call::ReadLocalized {
                ptr: 0,
                capacity: len
            }
        )
        .is_err()
    );
    let epoch = cap.begin(&mut world, DT).unwrap();
    let len = query(&mut cap, &mut memory, epoch, "雪🙂");
    cap.invoke(
        &mut memory,
        epoch,
        Call::ReadLocalized {
            ptr: 0,
            capacity: len,
        },
    )
    .unwrap();
    assert_eq!(
        wire::decode_localized(&memory[..len as usize])
            .unwrap()
            .text,
        "Bonjour 雪🙂"
    );
    cap.finish(&mut world).unwrap();
    let epoch = cap.begin(&mut world, DT).unwrap();
    assert!(
        cap.invoke(
            &mut memory,
            epoch,
            Call::ReadLocalized {
                ptr: 0,
                capacity: len
            }
        )
        .is_err()
    );
    assert!(cap.finish(&mut world).is_err());
}
#[test]
fn short_destination_and_ignored_active_failure_reject_all_prefixes_without_partial_copy() {
    for mode in [
        "short",
        "malformed",
        "invalid-locale",
        "missing-table",
        "invalid-date",
    ] {
        let mut world = NativeWorld::new(scene());
        let before = world.snapshot();
        let mut cap = Capability::default();
        let mut memory = vec![0; abi::LOCALIZATION_MAX_RESULT_BYTES];
        let epoch = cap.begin(&mut world, DT).unwrap();
        prefix(&mut cap, &mut memory, epoch);
        let (ptr, call) = match mode {
            "short" => {
                let len = query(&mut cap, &mut memory, epoch, "Hi");
                memory.fill(77);
                (
                    0,
                    Call::ReadLocalized {
                        ptr: 0,
                        capacity: len - 1,
                    },
                )
            }
            "malformed" => (0, Call::Localize { ptr: 0, len: 0 }),
            "invalid-locale" => {
                let len = wire::encode_settings(
                    wire::LocaleSettings {
                        locale: "EN_us",
                        fallbacks: &[],
                        pseudo: false,
                    },
                    &mut memory,
                )
                .unwrap();
                (
                    0,
                    Call::SetLocale {
                        ptr: 0,
                        len: len as i32,
                    },
                )
            }
            _ => {
                let len = wire::encode_request(
                    wire::LocalizeRequest {
                        table_id: if mode == "missing-table" {
                            "00000000000000000000000003"
                        } else {
                            TABLE
                        },
                        key: "hello",
                        arguments: &[wire::Argument {
                            name: "name",
                            value: wire::MessageArgument::Date(wire::CalendarDate {
                                year: 2024,
                                month: 2,
                                day: 30,
                            }),
                        }],
                    },
                    &mut memory,
                )
                .unwrap();
                (
                    0,
                    Call::Localize {
                        ptr: 0,
                        len: len as i32,
                    },
                )
            }
        };
        let memory_before = memory.clone();
        assert!(
            cap.invoke(&mut memory, epoch, call).is_err(),
            "{mode}:{ptr}"
        );
        assert_eq!(memory, memory_before); // even a short result never publishes a prefix
        assert!(cap.finish(&mut world).is_err());
        assert_eq!(world.snapshot(), before);
        assert_eq!(world.localization().settings().locale, "en");
        assert!(cap.logs().is_err());
    }
}
#[test]
fn stale_and_unrelated_failures_keep_prior_semantics_but_current_quota_failure_is_sticky() {
    let mut world = NativeWorld::new(scene());
    let mut cap = Capability::default();
    let mut memory = vec![0; abi::LOCALIZATION_MAX_RESULT_BYTES];
    let epoch = cap.begin(&mut world, DT).unwrap();
    assert!(cap.invoke(&mut memory, epoch - 1, Call::Locale).is_err());
    assert!(cap.invoke(&mut memory, epoch, Call::ChunkLen(-1)).is_err());
    setting(&mut cap, &mut memory, epoch, "fr");
    cap.finish(&mut world).unwrap();
    assert_eq!(world.localization().settings().locale, "fr");
    let epoch = cap.begin(&mut world, DT).unwrap();
    for _ in 0..abi::LOCALIZATION_MAX_QUERIES {
        cap.invoke(&mut memory, epoch, Call::Locale).unwrap();
    }
    assert!(matches!(
        cap.invoke(&mut memory, epoch, Call::Locale),
        Err(BoundaryError::Limit("localization query quota"))
    ));
    assert!(cap.finish(&mut world).is_err());
}
#[test]
fn actual_shared_calls_memory_and_transfer_boundaries_poison_authorized_locale_calls() {
    for mode in ["calls", "memory", "transfers"] {
        let mut world = NativeWorld::new(scene());
        let before = world.snapshot();
        let mut cap = Capability::default();
        let mut memory = vec![0; abi::LOCALIZATION_MAX_RESULT_BYTES];
        let epoch = cap.begin(&mut world, DT).unwrap();
        prefix(&mut cap, &mut memory, epoch);
        let error = match mode {
            "calls" => {
                // Four prefix calls have already consumed the common quota.
                for _ in 4..limits::CALLS {
                    cap.invoke(&mut memory, epoch, Call::ChunkCount).unwrap();
                }
                cap.invoke(&mut memory, epoch, Call::Locale).unwrap_err()
            }
            "memory" => cap
                .invoke(&mut vec![0; limits::MEMORY + 1], epoch, Call::Locale)
                .unwrap_err(),
            _ => {
                let len = query(&mut cap, &mut memory, epoch, &"x".repeat(32_768));
                // Repeated copies consume the real common transfer quota while
                // using only one localization query and fewer than CALLS imports.
                loop {
                    if let Err(error) = cap.invoke(
                        &mut memory,
                        epoch,
                        Call::ReadLocalized {
                            ptr: 0,
                            capacity: len,
                        },
                    ) {
                        break error;
                    }
                }
            }
        };
        assert!(matches!(error, BoundaryError::Limit(_)), "{mode}: {error}");
        assert!(cap.finish(&mut world).is_err());
        assert_eq!(world.snapshot(), before);
        assert_eq!(world.localization().settings().locale, "en");
        assert!(cap.logs().is_err());
    }
}
#[test]
fn locale_command_and_shared_structural_quotas_remain_atomic() {
    for mixed in [false, true] {
        let mut world = NativeWorld::new(scene());
        let before = world.snapshot();
        let mut cap = Capability::default();
        let mut memory = vec![0; abi::LOCALIZATION_MAX_RESULT_BYTES];
        let epoch = cap.begin(&mut world, DT).unwrap();
        if mixed {
            memory[..16].copy_from_slice(&[1; 16]);
            for _ in 0..limits::COMMANDS {
                cap.invoke(&mut memory, epoch, Call::SetParent(0, -1))
                    .unwrap();
            }
        } else {
            for _ in 0..abi::LOCALIZATION_MAX_COMMANDS {
                setting(&mut cap, &mut memory, epoch, "fr");
            }
        }
        let len = wire::encode_settings(
            wire::LocaleSettings {
                locale: "en",
                fallbacks: &[],
                pseudo: false,
            },
            &mut memory,
        )
        .unwrap();
        assert!(
            cap.invoke(
                &mut memory,
                epoch,
                Call::SetLocale {
                    ptr: 0,
                    len: len as i32
                }
            )
            .is_err()
        );
        assert!(cap.finish(&mut world).is_err());
        assert_eq!(world.snapshot(), before);
        assert_eq!(world.localization().settings().locale, "en");
    }
}
#[test]
fn summed_logical_input_and_output_limits_reject_at_their_actual_boundaries() {
    for output in [false, true] {
        let mut tables = tables();
        tables
            .get_mut(TABLE)
            .unwrap()
            .messages
            .insert("loud".into(), [("en".into(), "x".repeat(8192))].into());
        let mut world = NativeWorld::new(
            scene()
                .with_localization(NativeLocalization::new(tables, settings("en")).unwrap())
                .unwrap(),
        );
        let before = world.snapshot();
        let mut cap = Capability::default();
        let mut memory = vec![0; abi::LOCALIZATION_MAX_RESULT_BYTES];
        let epoch = cap.begin(&mut world, DT).unwrap();
        prefix(&mut cap, &mut memory, epoch);
        let text = "x".repeat(1100);
        let args = [wire::Argument {
            name: "name",
            value: wire::MessageArgument::Text(&text),
        }];
        let mut accepted = 0;
        loop {
            let len = wire::encode_request(
                wire::LocalizeRequest {
                    table_id: TABLE,
                    key: if output { "loud" } else { "hello" },
                    arguments: if output { &[] } else { &args },
                },
                &mut memory,
            )
            .unwrap();
            if let Err(error) = cap.invoke(
                &mut memory,
                epoch,
                Call::Localize {
                    ptr: 0,
                    len: len as i32,
                },
            ) {
                assert!(
                    matches!(error, BoundaryError::Limit(name) if name == if output { "localization output quota" } else { "localization input quota" })
                );
                break;
            }
            accepted += 1;
        }
        assert_eq!(
            accepted,
            if output {
                abi::LOCALIZATION_MAX_OUTPUT_BYTES / (8192 + 4)
            } else {
                abi::LOCALIZATION_MAX_INPUT_BYTES / (TABLE.len() + 5 + 4 + text.len())
            }
        );
        assert!(cap.finish(&mut world).is_err());
        assert_eq!(world.snapshot(), before);
        assert_eq!(world.localization().settings().locale, "en");
    }
}
#[test]
fn valid_localization_work_cannot_escape_later_numeric_or_structural_acceptance_failure() {
    for numeric in [false, true] {
        let mut world = NativeWorld::new(scene());
        let before = world.snapshot();
        let mut cap = Capability::default();
        let mut memory = vec![0; abi::LOCALIZATION_MAX_RESULT_BYTES];
        let epoch = cap.begin(&mut world, DT).unwrap();
        prefix(&mut cap, &mut memory, epoch);
        query(&mut cap, &mut memory, epoch, "accepted query");
        if numeric {
            cap.invoke(
                &mut memory,
                epoch,
                Call::Column {
                    chunk: 0,
                    ptr: 0,
                    velocity: false,
                    write: false,
                },
            )
            .unwrap();
            memory[..8].copy_from_slice(&f64::NAN.to_le_bytes());
            cap.invoke(
                &mut memory,
                epoch,
                Call::Column {
                    chunk: 0,
                    ptr: 0,
                    velocity: false,
                    write: true,
                },
            )
            .unwrap();
        } else {
            memory[..16].copy_from_slice(&[1; 16]);
            // Valid copied IDs, but the native structural graph refuses this cycle.
            cap.invoke(&mut memory, epoch, Call::SetParent(0, 0))
                .unwrap();
        }
        assert!(cap.finish(&mut world).is_err());
        assert_eq!(world.snapshot(), before);
        assert_eq!(world.localization().settings().locale, "en");
        assert!(cap.logs().is_err());
    }
}
