#[path = "../../incant_runtime/tests/support/localization.rs"]
mod fixture;
use fixture::*;
use incant_runtime::*;
use incant_wasm_abi::{self as abi, localization as wire};
use incant_wasm_runtime::{BoundaryError, Call, Capability};
const DT: f64 = 1. / 60.;
fn query(cap: &mut Capability, memory: &mut [u8], epoch: i64, offset: i32, limit: i32) -> Vec<u8> {
    let n = cap
        .invoke(memory, epoch, Call::LocalizationReport { offset, limit })
        .unwrap();
    cap.invoke(
        memory,
        epoch,
        Call::ReadLocalized {
            ptr: 1,
            capacity: n,
        },
    )
    .unwrap();
    memory[1..1 + n as usize].to_vec()
}
fn set(cap: &mut Capability, memory: &mut [u8], epoch: i64, locale: &str) {
    let n = wire::encode_settings(
        wire::LocaleSettings {
            locale,
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
    set(cap, memory, epoch, "fr");
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
fn reject(cap: &mut Capability, world: &mut NativeWorld, before: &[u8]) {
    assert!(cap.finish(world).is_err());
    assert_eq!(world.save_bytes().unwrap(), before);
    assert!(cap.logs().is_err());
}

#[test]
fn pages_match_prepared_native_coverage_and_change_only_after_commit() {
    let mut world = NativeWorld::new(scene());
    let mut cap = Capability::default();
    let mut memory = vec![0; abi::LOCALIZATION_MAX_RESULT_BYTES + 1];
    let epoch = cap.begin(&mut world, DT).unwrap();
    set(&mut cap, &mut memory, epoch, "fr");
    let before = query(&mut cap, &mut memory, epoch, 0, 128);
    assert_eq!(wire::report::decode(&before).unwrap().total, 0);
    cap.finish(&mut world).unwrap();
    let epoch = cap.begin(&mut world, DT).unwrap();
    assert!(
        cap.invoke(
            &mut memory,
            epoch - 1,
            Call::LocalizationReport {
                offset: -1,
                limit: 0
            }
        )
        .is_err()
    );
    for (offset, limit) in [(0, 128), (0, 1), (1, 1), (2, 128), (3, 1), (4096, 128)] {
        let native = world
            .localization()
            .report(incant_localization::LocalizationReportRequest { offset, limit })
            .unwrap();
        let bytes = query(&mut cap, &mut memory, epoch, offset as i32, limit as i32);
        let page = wire::report::decode(&bytes).unwrap();
        assert_eq!(page.total, 3);
        assert_eq!(page.total, native.total);
        assert_eq!(page.len(), native.entries.len());
        for (actual, expected) in page.entries().zip(native.entries) {
            assert_eq!(
                (
                    actual.table_id,
                    actual.key,
                    actual.requested_locale,
                    actual.resolved_locale
                ),
                (
                    expected.table_id.as_str(),
                    expected.key.as_str(),
                    expected.requested_locale.as_str(),
                    expected.resolved_locale.as_deref()
                )
            );
            assert_eq!(actual.kind, wire::MissingKind::Translation);
        }
    }
    assert_eq!(wire::report::decode(&before).unwrap().total, 0);
    cap.finish(&mut world).unwrap();
    let epoch = cap.begin(&mut world, DT).unwrap();
    assert!(
        cap.invoke(
            &mut memory,
            epoch,
            Call::ReadLocalized {
                ptr: 0,
                capacity: 128
            }
        )
        .is_err()
    );
    assert!(cap.finish(&mut world).is_err());
}

#[test]
fn bad_requests_and_short_or_out_of_bounds_copies_reject_entire_prefix_untouched() {
    for mode in 0..9 {
        let mut world = NativeWorld::new(scene());
        let before = world.save_bytes().unwrap();
        let mut cap = Capability::default();
        let mut memory = vec![0; 4096];
        let epoch = cap.begin(&mut world, DT).unwrap();
        prefix(&mut cap, &mut memory, epoch);
        let call = match mode {
            0 => Call::LocalizationReport {
                offset: -1,
                limit: 1,
            },
            1 => Call::LocalizationReport {
                offset: 4097,
                limit: 1,
            },
            2 => Call::LocalizationReport {
                offset: 0,
                limit: 0,
            },
            3 => Call::LocalizationReport {
                offset: 0,
                limit: 129,
            },
            4 => Call::LocalizationReport {
                offset: 0,
                limit: -1,
            },
            _ => {
                let n = cap
                    .invoke(
                        &mut memory,
                        epoch,
                        Call::LocalizationReport {
                            offset: 0,
                            limit: 128,
                        },
                    )
                    .unwrap();
                match mode {
                    5 => Call::ReadLocalized {
                        ptr: 1,
                        capacity: n - 1,
                    },
                    6 => Call::ReadLocalized {
                        ptr: -1,
                        capacity: n,
                    },
                    7 => Call::ReadLocalized {
                        ptr: 4090,
                        capacity: n,
                    },
                    _ => Call::ReadLocalized {
                        ptr: 1,
                        capacity: -1,
                    },
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
fn coverage_queries_share_localization_query_and_logical_output_limits() {
    for output in [false, true] {
        let mut tables = tables();
        for i in 0..128 {
            tables.get_mut(TABLE).unwrap().messages.insert(
                format!("{i:03}{}", "x".repeat(abi::LOCALIZATION_MAX_KEY_BYTES - 3)),
                [("en".into(), "X".into())].into(),
            );
        }
        let scene = scene()
            .with_localization(NativeLocalization::new(tables, settings("fr")).unwrap())
            .unwrap();
        let mut world = NativeWorld::new(scene);
        let before = world.save_bytes().unwrap();
        let mut cap = Capability::default();
        let mut memory = vec![0; abi::LOCALIZATION_MAX_RESULT_BYTES + 1];
        let epoch = cap.begin(&mut world, DT).unwrap();
        prefix(&mut cap, &mut memory, epoch);
        let limit = if output { 128 } else { 1 };
        let native = world
            .localization()
            .report(incant_localization::LocalizationReportRequest { offset: 0, limit })
            .unwrap();
        let logical = 8 + native
            .entries
            .iter()
            .map(|r| {
                r.table_id.len()
                    + r.key.len()
                    + r.requested_locale.len()
                    + r.resolved_locale.as_ref().map_or(0, String::len)
                    + 16
            })
            .sum::<usize>();
        let count = if output {
            abi::LOCALIZATION_MAX_OUTPUT_BYTES / logical
        } else {
            abi::LOCALIZATION_MAX_QUERIES
        };
        for _ in 0..count {
            query(&mut cap, &mut memory, epoch, 0, limit as i32);
        }
        let error = cap
            .invoke(
                &mut memory,
                epoch,
                Call::LocalizationReport {
                    offset: 0,
                    limit: limit as i32,
                },
            )
            .unwrap_err();
        assert!(
            matches!(error, BoundaryError::Limit(name) if name == if output {"localization output quota"} else {"localization query quota"})
        );
        reject(&mut cap, &mut world, &before);
    }
}
