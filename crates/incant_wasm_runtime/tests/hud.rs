#[path = "../../incant_runtime/tests/support/hud.rs"]
mod fixture;
use fixture::*;
use incant_input::{InputRuntime, UiNotification, UiSnapshot};
use incant_module::{HudControl, HudSnapshot};
use incant_runtime::*;
use incant_wasm_abi as abi;
use incant_wasm_runtime::{Call, Capability};
fn read(c: &mut Capability, m: &mut [u8], e: i64) -> Vec<u8> {
    let n = c.invoke(m, e, Call::HudLen).unwrap();
    let before = m.to_vec();
    assert_eq!(
        c.invoke(
            m,
            e,
            Call::ReadHud {
                ptr: 7,
                capacity: n
            }
        )
        .unwrap(),
        n
    );
    assert_eq!(&m[..7], &before[..7]);
    assert_eq!(&m[7 + n as usize..], &before[7 + n as usize..]);
    m[7..7 + n as usize].to_vec()
}
fn control(
    c: &mut Capability,
    m: &mut [u8],
    e: i64,
    owner: StableId,
    v: HudControl<'_>,
) -> Result<i32, incant_wasm_runtime::BoundaryError> {
    let n = abi::hud::encode_command(owner.0, v, &mut m[4096..]).unwrap();
    c.invoke(
        m,
        e,
        Call::ControlHud {
            ptr: 4096,
            len: n as i32,
        },
    )
}
#[test]
fn snapshot_is_precommand_and_routed_value_is_distinct_then_updates_commit_together() {
    let mut w = NativeWorld::new(scene());
    let mut c = Capability::default();
    let mut m = vec![0xa5; 32768];
    let mut input = InputRuntime::default();
    input
        .advance_validated_with_ui(
            &[],
            1. / 60.,
            &incant_input::ValidatedActions::new(Default::default()).unwrap(),
            UiSnapshot {
                focused: Some("00000000000000000000000003".into()),
                text_input: true,
                discarded_events: 2,
                events: vec![UiNotification::ValueChanged {
                    entity_id: "00000000000000000000000003".into(),
                    value: "edited 中文".into(),
                }],
            },
        )
        .unwrap();
    w.step().unwrap();
    let e = c.begin_with_input(&mut w, 1. / 60., &input).unwrap();
    let b = read(&mut c, &mut m, e);
    let s = HudSnapshot::parse(&b).unwrap();
    assert_eq!(s.widgets().nth(2).unwrap().text, Some("Health 10"));
    assert_eq!(s.events().next().unwrap().value, Some("edited 中文"));
    assert!(s.text_input());
    assert_eq!(s.discarded_events(), 2);
    control(&mut c, &mut m, e, id(2), HudControl::UiText("生命 8")).unwrap();
    control(&mut c, &mut m, e, id(2), HudControl::UiDisabled(true)).unwrap();
    control(&mut c, &mut m, e, id(4), HudControl::Text("Pickup 2")).unwrap();
    assert_eq!(read(&mut c, &mut m, e), b);
    c.finish(&mut w).unwrap();
    assert_eq!(
        declarations(&w).0[1].1.kind.text().unwrap().content,
        "生命 8"
    );
    assert!(declarations(&w).0[1].1.disabled);
    assert_eq!(declarations(&w).1[0].1.content.as_ref(), "Pickup 2");
    assert_eq!(s.widgets().nth(1).unwrap().text, Some("Health 10"));
}
#[test]
fn raw_memory_and_quota_failures_are_sticky_but_stale_epoch_does_not_poison() {
    for mode in 0..7 {
        let mut w = NativeWorld::new(scene());
        let before = declarations(&w);
        let mut c = Capability::default();
        let mut m = vec![0xa5; 32768];
        let e = c.begin(&mut w, 1. / 60.).unwrap();
        control(&mut c, &mut m, e, id(2), HudControl::UiText("prefix")).unwrap();
        let result = match mode {
            0 => c.invoke(
                &mut m,
                e,
                Call::ReadHud {
                    ptr: -1,
                    capacity: 32768,
                },
            ),
            1 => c.invoke(
                &mut m,
                e,
                Call::ReadHud {
                    ptr: 0,
                    capacity: 1,
                },
            ),
            2 => c.invoke(
                &mut m,
                e,
                Call::ControlHud {
                    ptr: 0,
                    len: i32::MAX,
                },
            ),
            3 => {
                let n = abi::hud::encode_command(id(2).0, HudControl::UiText("x"), &mut m).unwrap();
                m[n - 1] = 255;
                c.invoke(
                    &mut m,
                    e,
                    Call::ControlHud {
                        ptr: 0,
                        len: n as i32,
                    },
                )
            }
            4 => {
                for _ in 1..abi::HUD_MAX_COMMANDS {
                    control(&mut c, &mut m, e, id(2), HudControl::UiDisabled(false)).unwrap();
                }
                control(&mut c, &mut m, e, id(2), HudControl::UiDisabled(false))
            }
            5 => {
                let text = "a".repeat(abi::HUD_MAX_TEXT_BYTES);
                for _ in 0..63 {
                    control(&mut c, &mut m, e, id(2), HudControl::UiText(&text)).unwrap();
                }
                control(&mut c, &mut m, e, id(2), HudControl::UiText(&text))
            }
            _ => c.invoke(
                &mut m,
                e - 1,
                Call::ReadHud {
                    ptr: -1,
                    capacity: 0,
                },
            ),
        };
        assert!(result.is_err(), "mode {mode}");
        let accepted = c.finish(&mut w);
        if mode == 6 {
            accepted.unwrap();
            assert_ne!(declarations(&w), before);
        } else {
            assert!(accepted.is_err());
            assert_eq!(declarations(&w), before);
            assert!(c.is_failed());
        }
    }
}
#[test]
fn late_role_and_despawn_failure_roll_back_numeric_structural_and_hud_prefixes() {
    for despawn in [false, true] {
        let mut w = NativeWorld::new(scene());
        let before = declarations(&w);
        let snap = w.snapshot();
        let mut c = Capability::default();
        let mut m = vec![0; 32768];
        let e = c.begin(&mut w, 1. / 60.).unwrap();
        control(&mut c, &mut m, e, id(2), HudControl::UiText("prefix")).unwrap();
        if despawn {
            m[..16].copy_from_slice(&id(2).0);
            c.invoke(&mut m, e, Call::Despawn(0)).unwrap();
        }
        control(
            &mut c,
            &mut m,
            e,
            if despawn { id(2) } else { id(1) },
            HudControl::UiText("bad"),
        )
        .unwrap();
        assert!(c.finish(&mut w).is_err());
        assert_eq!(declarations(&w), before);
        assert_eq!(w.snapshot(), snap);
    }
}
