use incant_module::{HapticTarget, HapticTargets};
use incant_wasm_abi as abi;
fn frame(rows: &[abi::HapticTarget]) -> Vec<u8> {
    let mut bytes = abi::HapticHeader {
        magic: *abi::HAPTIC_MAGIC,
        count: rows.len() as u32,
        stride: abi::HapticTarget::SIZE as u32,
    }
    .encode()
    .to_vec();
    for row in rows {
        bytes.extend(row.encode());
    }
    bytes
}
fn row() -> abi::HapticCommand {
    abi::HapticCommand {
        target_kind: 0,
        id: 0,
        generation: 7,
        effect_kind: 0,
        strong: 0.5,
        weak: 1.,
        duration_ms: 125,
        reserved: 0,
    }
}
#[test]
fn complete_unaligned_targets_are_ordered_unique_and_bounded() {
    let rows: Vec<_> = (0..16)
        .map(|id| abi::HapticTarget {
            kind: 0,
            id,
            generation: id + 1,
        })
        .chain([abi::HapticTarget {
            kind: 1,
            id: 0,
            generation: u32::MAX,
        }])
        .collect();
    let bytes = frame(&rows);
    assert_eq!(bytes.len(), abi::HAPTIC_MAX_BYTES);
    let mut unaligned = vec![0xff];
    unaligned.extend(&bytes);
    let targets = HapticTargets::parse(&unaligned[1..]).unwrap();
    assert_eq!(targets.len(), 17);
    assert_eq!(
        targets.iter().last(),
        Some(HapticTarget::Device {
            generation: u32::MAX
        })
    );
    for len in 0..bytes.len() {
        assert!(HapticTargets::parse(&bytes[..len]).is_err());
    }
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert!(HapticTargets::parse(&trailing).is_err());
    assert!(HapticTargets::parse(&frame(&[])).unwrap().is_empty());
    for invalid in [
        vec![rows[1], rows[0]],
        vec![rows[0], rows[0]],
        vec![abi::HapticTarget { id: 16, ..rows[0] }],
        vec![abi::HapticTarget {
            generation: 0,
            ..rows[0]
        }],
        vec![abi::HapticTarget {
            kind: 256,
            ..rows[0]
        }],
        vec![abi::HapticTarget { id: 1, ..rows[16] }],
    ] {
        assert!(HapticTargets::parse(&frame(&invalid)).is_err());
    }
}
#[test]
fn commands_validate_f64_before_narrowing_and_unused_fields_by_bits() {
    assert!(abi::haptics::validate_command(&row()).is_ok());
    for value in [f64::NAN, f64::INFINITY, -f64::EPSILON, 1. + f64::EPSILON] {
        assert!(
            abi::haptics::validate_command(&abi::HapticCommand {
                strong: value,
                ..row()
            })
            .is_err()
        );
        assert!(
            abi::haptics::validate_command(&abi::HapticCommand {
                weak: value,
                ..row()
            })
            .is_err()
        );
    }
    for duration_ms in [0, 5001, u32::MAX] {
        assert!(
            abi::haptics::validate_command(&abi::HapticCommand {
                duration_ms,
                ..row()
            })
            .is_err()
        );
    }
    let stop = abi::HapticCommand {
        effect_kind: 2,
        strong: 0.,
        weak: 0.,
        duration_ms: 0,
        ..row()
    };
    let impact = abi::HapticCommand {
        target_kind: 1,
        effect_kind: 1,
        strong: 0.25,
        ..stop
    };
    assert!(abi::haptics::validate_command(&stop).is_ok());
    assert!(abi::haptics::validate_command(&impact).is_ok());
    for invalid in [
        abi::HapticCommand {
            strong: -0.,
            ..stop
        },
        abi::HapticCommand {
            weak: -0.,
            ..impact
        },
        abi::HapticCommand {
            duration_ms: 1,
            ..impact
        },
        abi::HapticCommand {
            reserved: 1,
            ..row()
        },
        abi::HapticCommand {
            effect_kind: 256,
            ..row()
        },
        abi::HapticCommand {
            target_kind: 1,
            ..row()
        },
        abi::HapticCommand {
            target_kind: 0,
            ..impact
        },
    ] {
        assert!(abi::haptics::validate_command(&invalid).is_err());
    }
}
#[test]
fn batch_framing_refuses_truncation_trailing_bytes_and_invalid_last_row() {
    let mut bytes = abi::HapticCommandHeader {
        magic: *abi::HAPTIC_COMMAND_MAGIC,
        count: 32,
        stride: abi::HapticCommand::SIZE as u32,
    }
    .encode()
    .to_vec();
    for _ in 0..32 {
        bytes.extend(row().encode());
    }
    assert_eq!(bytes.len(), abi::HAPTIC_MAX_COMMAND_BYTES);
    assert_eq!(
        abi::haptics::Commands::parse(&bytes)
            .unwrap()
            .iter()
            .count(),
        32
    );
    for len in 0..bytes.len() {
        assert!(abi::haptics::Commands::parse(&bytes[..len]).is_err());
    }
    let mut extra = bytes.clone();
    extra.push(0);
    assert!(abi::haptics::Commands::parse(&extra).is_err());
    *bytes.last_mut().unwrap() = 1;
    assert!(abi::haptics::Commands::parse(&bytes).is_err());
}
#[test]
fn ignored_local_sdk_validation_error_still_rejects_callback_without_imports() {
    for target in [
        HapticTarget::Gamepad {
            id: 16,
            generation: 1,
        },
        HapticTarget::Device { generation: 0 },
    ] {
        assert!(
            std::panic::catch_unwind(|| incant_module::__private::run(1, 0.25, |tick, _| {
                let _ = tick.stop_haptic(target);
                Ok(())
            }))
            .is_err()
        );
    }
}
