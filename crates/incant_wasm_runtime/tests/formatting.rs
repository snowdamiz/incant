#[path = "../../incant_runtime/tests/support/localization.rs"]
mod fixture;
use fixture::*;
use incant_runtime::*;
use incant_wasm_abi::{self as abi, localization as wire};
use incant_wasm_runtime::{BoundaryError, Call, Capability, limits};
const DT: f64 = 1. / 60.;
fn set(cap: &mut Capability, memory: &mut [u8], epoch: i64) {
    let n = wire::encode_settings(
        wire::LocaleSettings {
            locale: "fr",
            fallbacks: &[],
            pseudo: true,
        },
        memory,
    )
    .unwrap();
    cap.invoke(
        memory,
        epoch,
        Call::SetLocale {
            ptr: 0,
            len: n as i32,
        },
    )
    .unwrap();
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
    set(cap, memory, epoch);
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
fn query(cap: &mut Capability, memory: &mut [u8], epoch: i64, request: wire::FormatRequest) -> i32 {
    wire::encode_format(request, &mut memory[1..]).unwrap();
    cap.invoke(memory, epoch, Call::Format { ptr: 1, len: 24 })
        .unwrap()
}
fn read(cap: &mut Capability, memory: &mut [u8], epoch: i64, len: i32) -> Vec<u8> {
    cap.invoke(
        memory,
        epoch,
        Call::ReadLocalized {
            ptr: 101,
            capacity: len,
        },
    )
    .unwrap();
    memory[101..101 + len as usize].to_vec()
}
fn reject(cap: &mut Capability, world: &mut NativeWorld, before: &[u8]) {
    assert!(cap.finish(world).is_err());
    assert_eq!(world.save_bytes().unwrap(), before);
    assert!(cap.logs().is_err());
}
#[test]
fn committed_locale_all_lengths_and_signed_boundaries_match_native_formatter_without_pseudo() {
    let mut world = NativeWorld::new(scene());
    let mut cap = Capability::default();
    let mut memory = vec![0; 2048];
    let epoch = cap.begin(&mut world, DT).unwrap();
    set(&mut cap, &mut memory, epoch);
    let mut retained = None;
    for value in [
        -0.,
        -1234.5,
        9_007_199_254_740_991.,
        -9_007_199_254_740_991.,
    ] {
        let n = query(
            &mut cap,
            &mut memory,
            epoch,
            wire::FormatRequest::Number(value),
        );
        let copied = read(&mut cap, &mut memory, epoch, n);
        assert_eq!(
            wire::decode_formatted(&copied).unwrap(),
            incant_localization::format_number("en", value).unwrap()
        );
        retained = Some(copied);
    }
    for year in [-9999, 0, 1900, 2000, 2024, 9999] {
        for (length, native) in [
            (wire::DateLength::Short, DateLength::Short),
            (wire::DateLength::Medium, DateLength::Medium),
            (wire::DateLength::Long, DateLength::Long),
        ] {
            let date = wire::CalendarDate {
                year,
                month: 2,
                day: 28,
            };
            let n = query(
                &mut cap,
                &mut memory,
                epoch,
                wire::FormatRequest::Date(date, length),
            );
            let copied = read(&mut cap, &mut memory, epoch, n);
            assert_eq!(
                wire::decode_formatted(&copied).unwrap(),
                incant_localization::format_date(
                    "en",
                    &CalendarDate {
                        year,
                        month: 2,
                        day: 28
                    },
                    native
                )
                .unwrap()
            );
        }
    }
    cap.finish(&mut world).unwrap();
    assert_eq!(world.localization().settings().locale, "fr");
    assert!(world.localization().settings().pseudo);
    let epoch = cap.begin(&mut world, DT).unwrap();
    assert!(
        cap.invoke(&mut memory, epoch - 1, Call::Format { ptr: 1, len: 24 })
            .is_err()
    );
    let n = query(
        &mut cap,
        &mut memory,
        epoch,
        wire::FormatRequest::Number(-1234.5),
    );
    let copied = read(&mut cap, &mut memory, epoch, n);
    assert_eq!(
        wire::decode_formatted(&copied).unwrap(),
        incant_localization::format_number("fr", -1234.5).unwrap()
    );
    assert!(wire::decode_formatted(&retained.unwrap()).is_ok());
    cap.finish(&mut world).unwrap();
    let epoch = cap.begin(&mut world, DT).unwrap();
    assert!(
        cap.invoke(
            &mut memory,
            epoch,
            Call::ReadLocalized {
                ptr: 101,
                capacity: n
            }
        )
        .is_err()
    );
    assert!(cap.finish(&mut world).is_err());
}
#[test]
fn malformed_or_invalid_calendar_and_short_copies_reject_mixed_prefix_without_partial_writes() {
    for mode in 0..16 {
        let mut world = NativeWorld::new(scene());
        let before = world.save_bytes().unwrap();
        let mut cap = Capability::default();
        let mut memory = vec![0; 2048];
        let epoch = cap.begin(&mut world, DT).unwrap();
        prefix(&mut cap, &mut memory, epoch);
        wire::encode_format(
            wire::FormatRequest::Date(
                wire::CalendarDate {
                    year: 2024,
                    month: 2,
                    day: 29,
                },
                wire::DateLength::Medium,
            ),
            &mut memory[1..],
        )
        .unwrap();
        let call = match mode {
            0 => {
                memory[13] = 99;
                Call::Format { ptr: 1, len: 24 }
            }
            1 => {
                memory[23] = 3;
                Call::Format { ptr: 1, len: 24 }
            }
            2 => {
                memory[24] = 1;
                Call::Format { ptr: 1, len: 24 }
            }
            3 => {
                memory[17..21].copy_from_slice(&2023i32.to_le_bytes());
                Call::Format { ptr: 1, len: 24 }
            }
            4 => {
                memory[17..21].copy_from_slice(&1900i32.to_le_bytes());
                Call::Format { ptr: 1, len: 24 }
            }
            5 => {
                memory[21] = 4;
                memory[22] = 31;
                Call::Format { ptr: 1, len: 24 }
            }
            6 => {
                memory[17..21].copy_from_slice(&10000i32.to_le_bytes());
                Call::Format { ptr: 1, len: 24 }
            }
            7..=9 => {
                memory[13..17].copy_from_slice(&0u32.to_le_bytes());
                memory[17..25].copy_from_slice(
                    &[f64::NAN, f64::INFINITY, 9_007_199_254_740_992.][mode - 7].to_le_bytes(),
                );
                Call::Format { ptr: 1, len: 24 }
            }
            10 => Call::Format { ptr: -1, len: 24 },
            11 => Call::Format { ptr: 2040, len: 24 },
            12 => Call::Format { ptr: 1, len: 23 },
            13 => Call::Format { ptr: 1, len: 25 },
            14 => {
                memory[9] = 2;
                Call::Format { ptr: 1, len: 24 }
            }
            _ => {
                let n = query(
                    &mut cap,
                    &mut memory,
                    epoch,
                    wire::FormatRequest::Number(1234.5),
                );
                memory.fill(77);
                Call::ReadLocalized {
                    ptr: 101,
                    capacity: n - 1,
                }
            }
        };
        let untouched = memory.clone();
        assert!(cap.invoke(&mut memory, epoch, call).is_err(), "mode {mode}");
        assert_eq!(memory, untouched);
        reject(&mut cap, &mut world, &before);
    }
}
#[test]
fn formatting_shares_message_query_and_output_quotas_without_charging_logical_input() {
    for mode in ["input", "queries", "output"] {
        let mut tables = tables();
        tables
            .get_mut(TABLE)
            .unwrap()
            .messages
            .insert("loud".into(), [("en".into(), "x".repeat(8192))].into());
        tables
            .get_mut(TABLE)
            .unwrap()
            .messages
            .insert("tail".into(), [("en".into(), "x".repeat(8063))].into());
        let mut world = NativeWorld::new(
            scene()
                .with_localization(NativeLocalization::new(tables, settings("en")).unwrap())
                .unwrap(),
        );
        let before = world.save_bytes().unwrap();
        let mut cap = Capability::default();
        let mut memory = vec![0; abi::LOCALIZATION_MAX_RESULT_BYTES];
        let epoch = cap.begin(&mut world, DT).unwrap();
        prefix(&mut cap, &mut memory, epoch);
        if mode == "input" {
            let text = "x".repeat(abi::LOCALIZATION_MAX_INPUT_BYTES - TABLE.len() - 6 - 1);
            let n = wire::encode_request(
                wire::LocalizeRequest {
                    table_id: TABLE,
                    key: "absent",
                    arguments: &[wire::Argument {
                        name: "a",
                        value: wire::MessageArgument::Text(&text),
                    }],
                },
                &mut memory,
            )
            .unwrap();
            cap.invoke(
                &mut memory,
                epoch,
                Call::Localize {
                    ptr: 0,
                    len: n as i32,
                },
            )
            .unwrap();
            query(
                &mut cap,
                &mut memory,
                epoch,
                wire::FormatRequest::Number(1234.5),
            );
            cap.finish(&mut world).unwrap();
            assert_eq!(world.localization().settings().locale, "fr");
            continue;
        }
        if mode == "queries" {
            for _ in 0..abi::LOCALIZATION_MAX_QUERIES {
                cap.invoke(&mut memory, epoch, Call::Locale).unwrap();
            }
        } else {
            for key in std::iter::repeat_n("loud", 31).chain(["tail"]) {
                let n = wire::encode_request(
                    wire::LocalizeRequest {
                        table_id: TABLE,
                        key,
                        arguments: &[],
                    },
                    &mut memory,
                )
                .unwrap();
                cap.invoke(
                    &mut memory,
                    epoch,
                    Call::Localize {
                        ptr: 0,
                        len: n as i32,
                    },
                )
                .unwrap();
            }
        }
        wire::encode_format(wire::FormatRequest::Number(1234.5), &mut memory).unwrap();
        assert!(
            matches!(cap.invoke(&mut memory, epoch, Call::Format { ptr: 0, len: 24 }), Err(BoundaryError::Limit(name)) if name == if mode == "queries" { "localization query quota" } else { "localization output quota" })
        );
        reject(&mut cap, &mut world, &before);
    }
}
#[test]
fn authorized_format_common_ingress_limits_are_sticky_but_old_epochs_are_not() {
    for mode in ["calls", "memory", "transfers"] {
        let mut world = NativeWorld::new(scene());
        let before = world.save_bytes().unwrap();
        let mut cap = Capability::default();
        let mut memory = vec![0; abi::LOCALIZATION_MAX_RESULT_BYTES];
        let epoch = cap.begin(&mut world, DT).unwrap();
        prefix(&mut cap, &mut memory, epoch);
        if mode == "calls" {
            for _ in 4..limits::CALLS {
                cap.invoke(&mut memory, epoch, Call::ChunkCount).unwrap();
            }
        } else if mode == "memory" {
            memory.resize(limits::MEMORY + 1, 0);
        } else {
            // Choose a legal large message whose copied reply leaves <24 transfer
            // bytes after repeated full reads, without exhausting the call quota.
            let n = (16384..32768)
                .find(|n| (limits::TRANSFERS - (253 + n)) % (46 + n) < 24)
                .unwrap();
            let text = "x".repeat(n);
            let len = wire::encode_request(
                wire::LocalizeRequest {
                    table_id: TABLE,
                    key: "hello",
                    arguments: &[wire::Argument {
                        name: "name",
                        value: wire::MessageArgument::Text(&text),
                    }],
                },
                &mut memory,
            )
            .unwrap();
            assert_eq!(len, 67 + n);
            let reply = cap
                .invoke(
                    &mut memory,
                    epoch,
                    Call::Localize {
                        ptr: 0,
                        len: len as i32,
                    },
                )
                .unwrap();
            assert_eq!(reply as usize, 46 + n);
            for _ in 0..(limits::TRANSFERS - (186 + len)) / reply as usize {
                cap.invoke(
                    &mut memory,
                    epoch,
                    Call::ReadLocalized {
                        ptr: 0,
                        capacity: reply,
                    },
                )
                .unwrap();
            }
        }
        wire::encode_format(wire::FormatRequest::Number(1.), &mut memory).unwrap();
        let error = cap
            .invoke(&mut memory, epoch, Call::Format { ptr: 0, len: 24 })
            .unwrap_err();
        assert!(matches!(error, BoundaryError::Limit(_)), "{mode}: {error}");
        reject(&mut cap, &mut world, &before);
    }
}
