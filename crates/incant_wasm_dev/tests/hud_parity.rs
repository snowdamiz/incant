#![cfg(feature = "static-conformance")]
#[path = "support/hud.rs"]
mod fixture;
use fixture::{composed_fixture, id};
use incant_game_runtime::HostedOfflineGameSession;
use incant_input::{InputEvent, KeyCode};
use incant_render_game::NativeRenderHost;
use incant_script::SessionProgram;
use incant_ui::UiEvent;
use std::sync::Arc;
fn program(development: bool) -> SessionProgram {
    if development {
        SessionProgram::DevelopmentWasm(
            std::fs::read(
                std::path::Path::new(env!("INCANT_RUST_SDK_GUESTS")).join("rust-hud.wasm"),
            )
            .unwrap(),
        )
    } else {
        SessionProgram::Static(incant_wasm_runtime::embedded::linked("rust-hud").unwrap())
    }
}
fn game(
    development: bool,
) -> (
    HostedOfflineGameSession<NativeRenderHost>,
    tempfile::TempDir,
) {
    let (temp, scene, settings) = composed_fixture();
    let g = HostedOfflineGameSession::new(
        scene,
        program(development),
        Default::default(),
        vec![],
        44101,
        settings,
    )
    .unwrap();
    (g, temp)
}
fn tick(g: &mut HostedOfflineGameSession<NativeRenderHost>, events: &[InputEvent], ui: &[UiEvent]) {
    let mut pcm = vec![9.; g.next_output_frames().unwrap() * 2];
    g.tick_with_ui(events, ui, &mut pcm).unwrap();
    assert!(pcm.iter().all(|&x| x == 0.));
}
#[test]
fn actual_composed_hosts_deliver_focus_edits_submit_once_filter_keys_and_shape_hud_after_source_deletion()
 {
    let mut results = Vec::new();
    for dev in [false, true] {
        let (mut g, temp) = game(dev);
        temp.close().unwrap();
        tick(&mut g, &[], &[UiEvent::Focus(id(3))]);
        let focus = g.host().unwrap().ui().unwrap().focus().unwrap();
        tick(
            &mut g,
            &[],
            &[UiEvent::Commit {
                target: focus,
                text: "中文".into(),
            }],
        );
        assert_eq!(
            g.host().unwrap().ui().unwrap().text(id(3)).unwrap().text(),
            "Name中文"
        );
        assert_eq!(
            g.host()
                .unwrap()
                .ui_input()
                .unwrap()
                .widgets
                .iter()
                .find(|r| r.id == id(2))
                .unwrap()
                .layout
                .source(),
            "Edited 中文"
        );
        tick(
            &mut g,
            &[InputEvent::Key {
                code: KeyCode::Enter,
                down: true,
            }],
            &[],
        );
        assert_eq!(g.input().unwrap().ui.events.len(), 1);
        assert!(!g.input().unwrap().keyboard.held.contains(&KeyCode::Enter));
        assert_eq!(
            g.host()
                .unwrap()
                .ui_input()
                .unwrap()
                .widgets
                .iter()
                .find(|r| r.id == id(2))
                .unwrap()
                .layout
                .source(),
            "Submitted"
        );
        tick(
            &mut g,
            &[InputEvent::Key {
                code: KeyCode::Enter,
                down: false,
            }],
            &[],
        );
        assert!(g.input().unwrap().ui.events.is_empty());
        tick(&mut g, &[], &[]);
        let logs = g.logs().unwrap().entries();
        assert_eq!(logs.len(), 5);
        assert!(logs[0].message.contains("focus=3|text=true"));
        assert!(
            logs[1].message.contains("w3:false:Name") && logs[1].message.contains("e3:1:Name中文")
        );
        assert!(logs[2].message.contains("e3:2:Name中文"));
        assert!(!logs[3].message.contains("|e"));
        assert!(
            g.checkpoint_bytes().is_err(),
            "Guest heap/module continuation remains unsupported"
        );
        results.push((logs.to_vec(), g.world().unwrap().snapshot()));
    }
    assert_eq!(results[0], results[1]);
}
#[test]
fn caught_and_late_failures_hide_world_host_logs_and_do_not_publish_pending_ui_or_pcm() {
    for dev in [false, true] {
        for key in [
            KeyCode::F1,
            KeyCode::F2,
            KeyCode::F3,
            KeyCode::F4,
            KeyCode::F5,
        ] {
            let (mut g, temp) = game(dev);
            temp.close().unwrap();
            tick(&mut g, &[], &[UiEvent::Focus(id(3))]);
            let accepted = Arc::clone(g.host().unwrap().ui_input().unwrap());
            let focus = g.host().unwrap().ui().unwrap().focus().unwrap();
            let mut pcm = vec![9.; g.next_output_frames().unwrap() * 2];
            assert!(
                g.tick_with_ui(
                    &[InputEvent::Key {
                        code: key,
                        down: true
                    }],
                    &[
                        UiEvent::Commit {
                            target: focus,
                            text: "X".into()
                        },
                        UiEvent::Focus(id(2))
                    ],
                    &mut pcm
                )
                .is_err(),
                "{dev} {key:?}"
            );
            assert!(g.is_failed() && g.world().is_err() && g.host().is_err() && g.logs().is_err());
            assert!(pcm.iter().all(|&x| x == 0.));
            assert_eq!(
                accepted
                    .widgets
                    .iter()
                    .find(|r| r.id == id(3))
                    .unwrap()
                    .layout
                    .source(),
                "Name"
            );
            assert_eq!(
                accepted
                    .widgets
                    .iter()
                    .find(|r| r.id == id(2))
                    .unwrap()
                    .layout
                    .source(),
                "Health 8"
            );
        }
    }
}

#[allow(dead_code)]
#[path = "../../incant_runtime/tests/support/hud.rs"]
mod native_fixture;
#[test]
fn raw_caught_bounds_utf8_flags_and_quota_errors_never_commit_prefix_in_either_host() {
    use incant_runtime::NativeWorld;
    use incant_wasm_dev::DevModuleHost;
    use incant_wasm_runtime::{StaticModule, embedded};
    let bytes = wat::parse_str(include_str!("../../incant_wasm_runtime/fixtures/hud.wat")).unwrap();
    for mode in 0..11 {
        let mut a = NativeWorld::new(native_fixture::scene());
        let mut b = NativeWorld::new(native_fixture::scene());
        let before = native_fixture::declarations(&a);
        let numerical = a.snapshot();
        let mut static_host = StaticModule::new(embedded::linked("hud").unwrap()).unwrap();
        let mut dev = DevModuleHost::new(&bytes).unwrap();
        let dt = f64::from(mode + 1) / 64.;
        let x = static_host.tick(&mut a, dt);
        let y = dev.tick(&mut b, dt);
        assert_eq!(x.is_ok(), mode == 0, "static mode{mode}: {x:?}");
        assert_eq!(y.is_ok(), mode == 0, "dev mode{mode}: {y:?}");
        assert_eq!(
            native_fixture::declarations(&a),
            native_fixture::declarations(&b)
        );
        if mode != 0 {
            assert_eq!(native_fixture::declarations(&a), before);
            assert!(static_host.is_failed() && dev.is_failed());
        } else {
            assert!(native_fixture::declarations(&a).0[1].1.disabled);
        }
        assert_eq!(a.snapshot(), numerical);
        assert_eq!(b.snapshot(), numerical);
    }
}
