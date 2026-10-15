//! Actual Metal readback, separate grant required. The small font cache below is
//! test data in the existing lossless runtime format, not export/cooker evidence.
use super::*;
use incant_runtime::{CookedEntity, CookedScene, CookedSpatial, CookedUi, FontAssetRef, StableId};
use incant_types::{Camera, TextWrap, UiKind, UiLength, UiNode, UiStyle, UiText, Velocity};
use std::{fs, path::Path, sync::Arc};
fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
fn fixture(root: &Path) -> CookedScene {
    use incant_assets_runtime::font::{
        FONT_HEADER_BYTES, FONT_MAGIC, FontMetadata, font_key, font_path,
    };
    const FONT: &[u8] = include_bytes!("../../../incant_text/tests/fonts/NotoSans-Regular.ttf");
    let content = incant_assets_runtime::sha256(FONT);
    let dependency = incant_assets_runtime::Dependency {
        path: "font.ttf".into(),
        sha256: content.clone(),
    };
    let fingerprint = font_key(&dependency).unwrap();
    let fontdir = root.join("fonts");
    fs::create_dir(&fontdir).unwrap();
    let mut header = [0; FONT_HEADER_BYTES];
    header[..8].copy_from_slice(FONT_MAGIC);
    header[8..12].copy_from_slice(&1_u32.to_le_bytes());
    header[12..16].copy_from_slice(&(FONT.len() as u32).to_le_bytes());
    for (n, p) in header[16..].iter_mut().enumerate() {
        *p = u8::from_str_radix(&content[n * 2..n * 2 + 2], 16).unwrap();
    }
    fs::write(
        font_path(&fontdir, &content, "ifont").unwrap(),
        [header.as_slice(), FONT].concat(),
    )
    .unwrap();
    let meta = FontMetadata {
        version: 1,
        fingerprint: fingerprint.clone(),
        dependency,
        bytes: FONT.len() as u32,
        content_sha256: content,
    };
    fs::write(
        font_path(&fontdir, &fingerprint, "json").unwrap(),
        serde_json::to_vec(&meta).unwrap(),
    )
    .unwrap();
    let mut camera = CookedSpatial::empty(id(8));
    camera.camera = Some(Camera {
        fov_degrees: 60.,
        near: 0.1,
        far: 100.,
        projection: Default::default(),
    });
    let scene = CookedScene::new(
        id(99),
        144,
        [1, 2, 3, 8]
            .map(|n| CookedEntity {
                id: id(n),
                parent: (n == 3).then_some(id(2)),
                transform: Default::default(),
                velocity: (n == 1).then_some(Velocity { linear: [0.; 3] }),
            })
            .to_vec(),
    )
    .unwrap()
    .with_spatial(vec![], vec![camera])
    .unwrap()
    .with_text(
        vec![FontAssetRef {
            id: id(9),
            fingerprint: std::array::from_fn(|n| {
                u8::from_str_radix(&fingerprint[n * 2..n * 2 + 2], 16).unwrap()
            }),
        }],
        vec![],
    )
    .unwrap()
    .with_ui(vec![
        CookedUi {
            entity: id(2),
            node: Arc::new(UiNode {
                kind: UiKind::Root,
                style: UiStyle::default(),
                order: 0,
                disabled: false,
            }),
        },
        CookedUi {
            entity: id(3),
            node: Arc::new(UiNode {
                kind: UiKind::Button {
                    text: UiText {
                        content: "Capture HUD".into(),
                        fonts: vec![id(9)],
                        font_size: 16.,
                        line_height: 22.,
                        wrap: TextWrap::WordOrGlyph,
                    },
                },
                style: UiStyle {
                    size: [UiLength::Pixels(140.), UiLength::Pixels(40.)],
                    ..Default::default()
                },
                order: 0,
                disabled: false,
            }),
        },
    ])
    .unwrap();
    fs::write(root.join("scene"), scene.to_bytes()).unwrap();
    fs::write(
        root.join("clips"),
        [b"NCLIPS01".as_slice(), &[0; 4]].concat(),
    )
    .unwrap();
    fs::write(
        root.join("actions"),
        br#"{"right":{"kind":"button","bindings":[{"type":"key","code":"KeyD"}]}}"#,
    )
    .unwrap();
    fs::write(root.join("input"),serde_json::to_vec(&serde_json::json!({"format":"incant-input","version":1,"start_tick":0,"tick_rate":144,"ticks":4,
        "frames":[{"tick":1,"events":[{"type":"key","code":"KeyD","down":true}]},
                  {"tick":2,"events":[],"viewport":[80.,96.]}]})).unwrap()).unwrap();
    scene
}
fn settings(root: &Path) -> incant_render_game::NativeRenderSettings {
    incant_render_game::NativeRenderSettings {
        camera: id(8),
        model_cache: root.join("models"),
        texture_cache: root.join("textures"),
        text: Some(incant_render_native::TextSettings::new(root.join("fonts"))),
        ui: Some(incant_render_game::UiSettings::new([160., 96.])),
        consumer: (),
    }
}
fn args(root: &Path, stem: &str) -> Vec<String> {
    let mut args = [
        root.join("scene"),
        root.to_path_buf(),
        root.join("clips"),
        root.join("input"),
        root.join("actions"),
    ]
    .map(|p| p.to_str().unwrap().to_owned())
    .to_vec();
    args.extend([
        "4".into(),
        "44117".into(),
        root.join(stem).to_str().unwrap().into(),
        "--cpu-render-input".into(),
        root.join("models").to_str().unwrap().into(),
        root.join("textures").to_str().unwrap().into(),
        format!("{:032x}", 8),
        "--font-cache".into(),
        root.join("fonts").to_str().unwrap().into(),
        "--ui-viewport".into(),
        "160x96".into(),
    ]);
    args
}
fn run(args: Vec<String>) -> serde_json::Value {
    let mut reports = vec![];
    crate::run(args, |s| reports.push(s.to_owned())).unwrap();
    assert_eq!(reports.len(), 1);
    serde_json::from_str(&reports[0]).unwrap()
}

#[test]
#[ignore = "requires a separately granted actual Metal GPU; sampled UI/PCM/restart proof, no window"]
fn selected_frames_include_ui_and_preserve_actual_player_pcm_world_and_restart_epochs() {
    let root = tempfile::tempdir().unwrap();
    let root = root.path();
    let scene = fixture(root);
    // The ordinary public runner still owns its only simulation and PCM stream.
    let baseline = run(args(root, "baseline.pcm"));
    let mut selected = args(root, "captured.pcm");
    selected.extend([
        "--capture-frames".into(),
        root.join("cli-frames").to_str().unwrap().into(),
        "--capture-at".into(),
        "0,1,4".into(),
        "--capture-size".into(),
        "320x192".into(),
    ]);
    let captured = run(selected);
    assert_eq!(
        fs::read(root.join("baseline.pcm")).unwrap(),
        fs::read(root.join("captured.pcm")).unwrap()
    );
    for key in [
        "tick",
        "frames",
        "pcm_sha256",
        "sources",
        "logs",
        "render_input",
        "flipbooks",
    ] {
        assert_eq!(baseline[key], captured[key], "{key}");
    }
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("cli-frames/manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["complete"], true);
    assert_eq!(manifest["frames"].as_array().unwrap().len(), 3);
    assert_eq!(
        manifest["frames"][0]["logical_viewport"],
        serde_json::json!([160., 96.])
    );
    assert_eq!(
        manifest["frames"][2]["logical_viewport"],
        serde_json::json!([80., 96.])
    );
    for (index, step) in [0, 1, 4].into_iter().enumerate() {
        let r = &manifest["frames"][index];
        assert_eq!(r["host_step"], step);
        assert_eq!(r["simulation_tick"], step);
        assert_eq!(r["generation"], 1);
        assert!(r["ui_glyph_quads"].as_u64().unwrap() > 0);
        assert_eq!(r["physical_size"], serde_json::json!([320, 192]));
        let bytes = fs::read(root.join("cli-frames").join(r["png"].as_str().unwrap())).unwrap();
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(u32::from_be_bytes(bytes[16..20].try_into().unwrap()), 320);
        assert_eq!(u32::from_be_bytes(bytes[20..24].try_into().unwrap()), 192);
        assert_eq!(
            incant_assets_runtime::sha256(&bytes),
            r["png_sha256"].as_str().unwrap()
        );
    }

    // A restart-capable trusted script proves pre-restart generation/tick
    // receipts. Its persistent counter catches a reset/re-execution per sample.
    let script=incant_script::CompiledScript::compile("let calls=0;exports.default={update(t){calls++;if(calls!==t.clock.tick)throw Error('VM continuity');for(const c of t.chunks)if(c.velocities)c.velocities[0]=calls;if(calls===2)t.commands.requestRestart();}};").unwrap();
    let program = incant_script::SessionProgram::from(script);
    let s = Some(crate::session::Settings::Cpu(settings(root)));
    let mut restart =
        crate::restart::Restart::prepare(&scene, &program, &Default::default(), &[], &s).unwrap();
    let mut game = Player::new(scene, program, Default::default(), vec![], 44117, s, 4).unwrap();
    restart.enable(&mut game).unwrap();
    let mut capture = Capture::new(
        Options {
            directory: root.join("restart-frames"),
            steps: vec![0, 2, 3, 4],
            size: [320, 192],
        },
        &game,
    )
    .unwrap();
    assert_eq!(
        capture.renderer.adapter.get_info().backend,
        incant_render::wgpu::Backend::Metal
    );
    let mut pcm = PcmOutput::new(&root.join("restart.pcm"), 2000).unwrap();
    capture.sample(0, 1, &game, &pcm).unwrap();
    // Same accepted world/target, omitting UI strictly for the negative oracle.
    let Player::Spatial(current) = &game else {
        panic!()
    };
    let host = current.host().unwrap();
    let retained = capture.scene.as_mut().unwrap();
    let without = capture
        .target
        .render(
            &capture.renderer,
            &mut retained.gpu,
            host.render_input().input(),
            None,
        )
        .unwrap();
    assert_ne!(
        incant_assets_runtime::sha256(without),
        capture.receipts[0].rgba_sha256,
        "screen UI must change real readback bytes"
    );
    for step in 1..=4 {
        let mut output = vec![1.; game.next_output_frames().unwrap() * 2];
        game.tick_recorded(&[], &[], None, &mut output).unwrap();
        pcm.append(&output).unwrap();
        capture
            .sample(step, restart.generation, &game, &pcm)
            .unwrap();
        restart
            .accept(&mut game, step, u128::from(pcm.frames()))
            .unwrap();
    }
    assert_eq!(
        capture
            .receipts
            .iter()
            .map(|r| (r.host_step, r.generation, r.simulation_tick))
            .collect::<Vec<_>>(),
        [(0, 1, 0), (2, 1, 2), (3, 2, 1), (4, 2, 2)]
    );
    assert_eq!(restart.generation, 3);
    assert_eq!(game.world().unwrap().tick(), 0);
    let expected = 2 * (2 * 44117u64 / 144);
    assert_eq!(pcm.frames(), expected);
    let report = serde_json::json!({"tick":0,"pcm_sha256":pcm.sha256(),"frames":pcm.frames()});
    pcm.publish(&root.join("restart.pcm")).unwrap();
    capture.finish(&report).unwrap();
    assert_eq!(
        fs::metadata(root.join("restart.pcm")).unwrap().len(),
        expected * 8
    );
}
