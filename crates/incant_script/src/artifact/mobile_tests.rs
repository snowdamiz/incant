use super::*;
use crate::{CompiledScript, NativeSession};
use incant_input::{InputEvent, KeyCode};
use incant_runtime::{CookedEntity, CookedScene, StableId};
use incant_types::{Transform, Velocity};

#[test]
fn closed_cross_target_contract_rejects_every_abi_and_unverified_target_change() {
    let abi = BytecodeAbi {
        version: 28,
        pointer_bits: 64,
        little_endian: true,
        value_bytes: 16,
        value_alignment: 8,
        atom_bytes: 4,
        float_mantissa: 53,
        float_max_exponent: 1024,
        write_flags: rquickjs::qjs::JS_WRITE_OBJ_BYTECODE
            | rquickjs::qjs::JS_WRITE_OBJ_STRIP_SOURCE,
        default_c_flags: true,
    };
    let mut targets = vec!["aarch64-apple-ios", "aarch64-apple-ios-sim"];
    if cfg!(feature = "probe-android-bytecode") {
        targets.push("aarch64-linux-android");
    }
    for target in targets {
        assert!(compatibility::admits("aarch64-apple-darwin", target, abi));
        let actual = BytecodeAbi::current();
        assert_eq!(
            compatibility::admits("aarch64-apple-darwin", target, actual),
            actual.reviewed_arm64()
        );
        for mutate in 0..10 {
            let mut wrong = abi;
            match mutate {
                0 => wrong.version += 1,
                1 => wrong.pointer_bits = 32,
                2 => wrong.little_endian = false,
                3 => wrong.value_bytes = 8,
                4 => wrong.value_alignment = 4,
                5 => wrong.atom_bytes = 8,
                6 => wrong.float_mantissa = 24,
                7 => wrong.float_max_exponent = 128,
                8 => wrong.write_flags |= rquickjs::qjs::JS_WRITE_OBJ_SAB,
                _ => wrong.default_c_flags = false,
            }
            assert!(!compatibility::admits(
                "aarch64-apple-darwin",
                target,
                wrong
            ));
        }
        assert!(!compatibility::admits("x86_64-apple-darwin", target, abi));
        assert!(!compatibility::admits(
            "aarch64-unknown-linux-gnu",
            target,
            abi
        ));
    }
    assert_eq!(
        compatibility::admits("aarch64-apple-darwin", "aarch64-linux-android", abi),
        cfg!(feature = "probe-android-bytecode")
    );
    for target in ["x86_64-linux-android", "armv7-linux-androideabi"] {
        assert!(!compatibility::admits("aarch64-apple-darwin", target, abi));
    }
    assert!(!compatibility::admits(
        "aarch64-apple-ios-sim",
        "aarch64-apple-ios",
        abi
    ));
}

fn session() -> NativeSession {
    let code = super::tests::embedded(
        CompiledScript::compile_named(
            "mobile-compat.js",
            include_str!("../../tests/fixtures/mobile_compat.js"),
        )
        .unwrap(),
    );
    let scene = CookedScene::new(
        StableId(99_u128.to_be_bytes()),
        60,
        vec![CookedEntity {
            id: StableId(1_u128.to_be_bytes()),
            parent: None,
            transform: Transform::default(),
            velocity: Some(Velocity { linear: [0.; 3] }),
        }],
    )
    .unwrap();
    let actions = serde_json::from_str(
        r#"{
      "right":{"kind":"button","bindings":[{"type":"key","code":"KeyD"}]},
      "fail":{"kind":"button","bindings":[{"type":"key","code":"KeyF"}]}}
    "#,
    )
    .unwrap();
    NativeSession::new(scene, code, actions).unwrap()
}

#[test]
fn embedded_mobile_fixture_matches_independent_ieee_integer_and_language_records() {
    let expected: serde_json::Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/mobile_compat.expected.json"
    ))
    .unwrap();
    for name in ["held", "released"] {
        let mut game = session();
        for tick in 1..=3 {
            let events = if tick == 1 {
                vec![InputEvent::Key {
                    code: KeyCode::KeyD,
                    down: true,
                }]
            } else if tick == 2 && name == "released" {
                vec![InputEvent::Key {
                    code: KeyCode::KeyD,
                    down: false,
                }]
            } else {
                vec![]
            };
            game.tick(&events).unwrap();
        }
        let actual: Vec<_> = game
            .logs()
            .unwrap()
            .entries()
            .iter()
            .map(|entry| entry.message.as_str())
            .collect();
        let messages: Vec<_> = expected["scenarios"][name]["messages"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert_eq!(actual, messages);
        let position = game
            .world()
            .unwrap()
            .inspect(StableId(1_u128.to_be_bytes()))
            .unwrap()
            .transform
            .translation;
        for (axis, actual) in position.into_iter().enumerate() {
            assert!(
                (actual
                    - expected["scenarios"][name]["translation"][axis]
                        .as_f64()
                        .unwrap())
                .abs()
                    < 1e-12
            );
        }
    }
    let mut game = session();
    game.tick(&[InputEvent::Key {
        code: KeyCode::KeyD,
        down: true,
    }])
    .unwrap();
    let failure = game
        .tick(&[InputEvent::Key {
            code: KeyCode::KeyF,
            down: true,
        }])
        .unwrap_err();
    assert!(failure.to_string().contains("accepted game input failure"));
    assert!(game.world().is_err() && game.logs().is_err());
}
