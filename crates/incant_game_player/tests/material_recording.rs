//! Actual source-free CLI processes. No GPU is requested by these CPU controls;
//! the stale-manifest route must refuse before renderer/device construction.
#![cfg(feature = "render-input")]
use incant_render_data::prewarm::MaterialManifest;
use incant_runtime::{
    CookedEntity, CookedScene, CookedSpatial, NativeMeshRenderer, StableId, VisualAssetKind,
    VisualAssetRef,
};
use incant_types::Camera;
use serde_json::{Value, json};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
fn fixture(root: &Path) {
    let source = root.join("source");
    fs::create_dir(&source).unwrap();
    let positions = [-1_f32, -1., 0., 1., -1., 0., 0., 1., 0.];
    fs::write(
        source.join("tri.bin"),
        positions
            .into_iter()
            .flat_map(f32::to_le_bytes)
            .collect::<Vec<_>>(),
    )
    .unwrap();
    fs::write(source.join("tri.gltf"), json!({"asset":{"version":"2.0"},"buffers":[{"uri":"tri.bin","byteLength":36}],"bufferViews":[{"buffer":0,"byteLength":36}],"accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[-1,-1,0],"max":[1,1,0]}],"meshes":[{"primitives":[{"attributes":{"POSITION":0}}]}],"nodes":[{"mesh":0}],"scenes":[{"nodes":[0]}],"scene":0}).to_string()).unwrap();
    let model =
        incant_assets::cook_gltf(&source, Path::new("tri.gltf"), &root.join("models")).unwrap();
    let asset = VisualAssetRef {
        id: id(91),
        kind: VisualAssetKind::Model,
        fingerprint: std::array::from_fn(|i| {
            u8::from_str_radix(&model.metadata.fingerprint[i * 2..i * 2 + 2], 16).unwrap()
        }),
    };
    let mut mesh = CookedSpatial::empty(id(1));
    mesh.mesh = Some(NativeMeshRenderer {
        mesh: id(91),
        cast_shadows: false,
    });
    let mut camera = CookedSpatial::empty(id(2));
    camera.camera = Some(Camera::default());
    let scene = CookedScene::new(
        id(99),
        60,
        (1..=2)
            .map(|n| CookedEntity {
                id: id(n),
                parent: None,
                velocity: None,
                transform: incant_types::Transform {
                    translation: if n == 2 { [0., 0., 4.] } else { [0.; 3] },
                    ..Default::default()
                },
            })
            .collect(),
    )
    .unwrap()
    .with_spatial(vec![asset], vec![mesh, camera])
    .unwrap();
    fs::write(root.join("scene"), scene.to_bytes()).unwrap();
    fs::write(
        root.join("clips"),
        [b"NCLIPS01".as_slice(), &[0; 4]].concat(),
    )
    .unwrap();
    fs::write(root.join("actions"), b"{}").unwrap();
    fs::write(root.join("input"),json!({"format":"incant-input","version":1,"start_tick":0,"tick_rate":60,"ticks":3,"frames":[]}).to_string()).unwrap();
    fs::remove_dir_all(source).unwrap();
}
fn run(root: &Path, stem: &str, option: &str, manifest: &str) -> Output {
    let bin = std::env::var_os("INCANT_TEST_PLAYER")
        .unwrap_or_else(|| env!("CARGO_BIN_EXE_incant_game_player").into());
    let mut command = Command::new(bin);
    for path in [
        root.join("scene"),
        root.to_path_buf(),
        root.join("clips"),
        root.join("input"),
        root.join("actions"),
    ] {
        command.arg(path);
    }
    command
        .args(["3", "48000"])
        .arg(root.join(format!("{stem}.pcm")))
        .arg("--cpu-render-input")
        .arg(root.join("models"))
        .arg(root.join("textures"))
        .arg("00000000000000000000000000000002")
        .arg(option)
        .arg(root.join(manifest));
    command.output().unwrap()
}
fn success(out: Output) -> Value {
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}
#[test]
fn separate_source_free_processes_record_same_content_and_never_overwrite_manifest() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fixture(root);
    let first = success(run(root, "first", "--record-materials", "first.materials"));
    let second = success(run(
        root,
        "second",
        "--record-materials",
        "second.materials",
    ));
    assert_eq!(first["material_recording"], second["material_recording"]);
    assert_eq!(first["render_input"]["mode"], "cpu-material-recording");
    assert_eq!(first["render_input"]["gpu_submitted"], false);
    let bytes = fs::read(root.join("first.materials")).unwrap();
    assert_eq!(bytes, fs::read(root.join("second.materials")).unwrap());
    let manifest = MaterialManifest::from_bytes(&bytes).unwrap();
    assert_eq!(manifest.identity.initial_models.len(), 1);
    assert_eq!(manifest.observed.len(), 1);
    assert_eq!(
        (
            manifest.observed[0].first_tick,
            manifest.observed[0].last_tick,
            manifest.observed[0].ticks
        ),
        (0, 3, 4)
    );
    assert!(!manifest.observed[0].key.mirrored());
    let duplicate = run(root, "duplicate", "--record-materials", "first.materials");
    assert!(!duplicate.status.success());
    assert!(duplicate.stdout.is_empty());
    assert!(!root.join("duplicate.pcm").exists());
    assert_eq!(fs::read(root.join("first.materials")).unwrap(), bytes);
}
#[cfg(feature = "render-gpu")]
#[test]
fn stale_source_free_manifest_refuses_before_any_gpu_frame_or_output() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fixture(root);
    success(run(
        root,
        "recorded",
        "--record-materials",
        "good.materials",
    ));
    let mut manifest =
        MaterialManifest::from_bytes(&fs::read(root.join("good.materials")).unwrap()).unwrap();
    manifest.identity.cooked_scene_sha256[0] ^= 1;
    fs::write(root.join("stale.materials"), manifest.to_bytes().unwrap()).unwrap();
    let out = run(root, "stale", "--prewarm-materials", "stale.materials");
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
    assert!(!root.join("stale.pcm").exists());
    let message = String::from_utf8(out.stderr).unwrap();
    assert!(
        message.contains("material manifest does not match"),
        "{message}"
    );
}
#[cfg(feature = "render-gpu")]
#[test]
#[ignore = "requires an explicit native GPU lease; actual separate player processes"]
fn recorded_level_load_and_unrecorded_mirror_input_use_the_real_async_fallback_route() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fixture(root);
    success(run(
        root,
        "recorded",
        "--record-materials",
        "level.materials",
    ));
    let manifest =
        MaterialManifest::from_bytes(&fs::read(root.join("level.materials")).unwrap()).unwrap();
    assert_eq!(manifest.observed.len(), 1);
    assert!(!manifest.observed[0].key.mirrored());
    fs::write(
        root.join("actions"),
        json!({"mirror":{"kind":"button","bindings":[{"type":"key","code":"KeyM"}]}}).to_string(),
    )
    .unwrap();
    fs::write(root.join("input"), json!({"format":"incant-input","version":1,"start_tick":0,"tick_rate":60,"ticks":3,"frames":[{"tick":1,"events":[{"type":"key","code":"KeyM","down":true}]}]}).to_string()).unwrap();
    let report = success(run(
        root,
        "rendered",
        "--prewarm-materials",
        "level.materials",
    ));
    let input = &report["render_input"];
    assert_eq!(input["mode"], "gpu-offscreen-recorded-materials");
    assert_eq!(input["gpu"]["accepted_frames"], 3);
    assert_eq!(input["material_prewarm"]["prewarmed"], 1);
    assert!(
        input["material_prewarm"]["fallback_selections"]
            .as_u64()
            .unwrap()
            > 0
    );
    assert!(
        input["material_prewarm"]["keys"]
            .as_array()
            .unwrap()
            .iter()
            .any(|k| k["key"] == 1 && k["requested"] == true)
    );
    assert!(
        input["gpu"]["captures"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["non_background_pixels"].as_u64().unwrap() > 0)
    );
    assert_eq!(report["material_prewarm_close"]["closed"], true);
    if let Some(expected) = std::env::var_os("INCANT_EXPECT_GPU_BACKEND") {
        assert_eq!(
            input["gpu"]["adapter"]["backend"],
            expected.to_str().unwrap()
        );
    }
    if let Some(evidence) = std::env::var_os("INCANT_MATERIAL_PREWARM_EVIDENCE") {
        use std::io::Write;
        let path = Path::new(&evidence).join("source-free-player.json");
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap()
            .write_all(&serde_json::to_vec_pretty(&report).unwrap())
            .unwrap();
    }
}
