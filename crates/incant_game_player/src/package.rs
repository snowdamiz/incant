//! Package convenience arguments. Only bounded data is loaded here; executable
//! behavior remains the trusted program linked into this player.
use crate::{bounded, session::Result};
use serde::Deserialize;
use std::{collections::BTreeMap, path::Path};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    format: String,
    version: u32,
    camera: Option<String>,
    files: BTreeMap<String, String>,
}

fn verify(root: &Path) -> Result<Manifest> {
    let manifest: Manifest = serde_json::from_slice(&bounded(&path(root, "package.json")?, 8192)?)?;
    if manifest.format != "incant-native-package" || manifest.version != 1 {
        return Err("unsupported native package manifest".into());
    }
    // These are the only indirect startup files; no manifest path becomes code,
    // a destination, or an arbitrary filesystem read.
    let names = [
        "world.scene",
        "input-actions.json",
        "clips.bin",
        "empty-input.json",
    ];
    if manifest.files.len() != names.len()
        || names.iter().any(|name| !manifest.files.contains_key(*name))
    {
        return Err("native package startup file set mismatch".into());
    }
    for (name, cap) in names
        .iter()
        .zip([128 * 1024 * 1024, 256 * 1024, 12 + 128 * 48, 8192])
    {
        if incant_assets_runtime::sha256(&bounded(&path(root, name)?, cap)?)
            != manifest.files[*name]
        {
            return Err("native package startup checksum mismatch".into());
        }
    }
    Ok(manifest)
}

pub fn expand(args: Vec<String>) -> Result<Vec<String>> {
    if args.first().is_none_or(|arg| arg != "--package") {
        return Ok(args);
    }
    let root = Path::new(args.get(1).ok_or("--package needs a data directory")?);
    let manifest = verify(root)?;
    let mut options = BTreeMap::new();
    let mut window = false;
    let mut index = 2;
    while index < args.len() {
        let key = args[index].as_str();
        if key == "--window" {
            if window {
                return Err("duplicate --window".into());
            }
            window = true;
            index += 1;
            continue;
        }
        if !matches!(
            key,
            "--ticks"
                | "--output"
                | "--observe"
                | "--input-recording"
                | "--stop-after"
                | "--ui-viewport"
                | "--presentation-output"
                | "--presentation-capacity"
        ) || options.contains_key(key)
        {
            return Err("unknown or duplicate native package option".into());
        }
        options.insert(
            key,
            args.get(index + 1)
                .ok_or("native package option needs a value")?
                .clone(),
        );
        index += 2;
    }
    let scene = path(root, "world.scene")?;
    let audio = path(root, "audio")?;
    let clips = path(root, "clips.bin")?;
    let actions = path(root, "input-actions.json")?;
    if window {
        if options.keys().any(|key| {
            !matches!(
                *key,
                "--stop-after"
                    | "--input-recording"
                    | "--observe"
                    | "--presentation-output"
                    | "--presentation-capacity"
            )
        }) {
            return Err("window package options conflict with offline options".into());
        }
        if (options.contains_key("--input-recording") || options.contains_key("--observe"))
            && !options.contains_key("--stop-after")
        {
            return Err("window recording/observation requires --stop-after".into());
        }
        let mut result = vec![
            "--window".into(),
            scene,
            audio,
            clips,
            actions,
            path(root, "models")?,
            path(root, "textures")?,
            camera(&manifest)?,
            "--font-cache".into(),
            path(root, "fonts")?,
        ];
        for key in [
            "--stop-after",
            "--input-recording",
            "--observe",
            "--presentation-output",
            "--presentation-capacity",
        ] {
            if let Some(value) = options.remove(key) {
                result.extend([key.into(), value]);
            }
        }
        return Ok(result);
    }
    if options.contains_key("--presentation-output")
        || options.contains_key("--presentation-capacity")
    {
        return Err("presentation capture requires --window".into());
    }
    if options.contains_key("--stop-after") {
        return Err("--stop-after requires --window".into());
    }
    let ticks = options
        .remove("--ticks")
        .ok_or("offline package requires --ticks")?;
    let output = options
        .remove("--output")
        .ok_or("offline package requires --output PCM_FILE")?;
    let recording = options
        .remove("--input-recording")
        .unwrap_or(path(root, "empty-input.json")?);
    let mut result = vec![
        scene,
        audio,
        clips,
        recording,
        actions,
        ticks,
        "48000".into(),
        output,
    ];
    if manifest.camera.is_some() {
        result.extend([
            "--cpu-render-input".into(),
            path(root, "models")?,
            path(root, "textures")?,
            camera(&manifest)?,
            "--font-cache".into(),
            path(root, "fonts")?,
        ]);
    }
    if let Some(viewport) = options.remove("--ui-viewport") {
        if manifest.camera.is_none() {
            return Err("current composed UI package requires an explicit camera host".into());
        }
        result.extend(["--ui-viewport".into(), viewport]);
    }
    if let Some(plan) = options.remove("--observe") {
        result.extend(["--observe".into(), plan]);
    }
    Ok(result)
}

#[cfg(feature = "mobile-host")]
pub(crate) struct HostedPackage {
    pub scene: incant_runtime::CookedScene,
    pub actions: incant_input::InputActions,
    pub clips: Vec<incant_audio_runtime::ClipBinding>,
    pub settings: incant_render_game::NativeRenderSettings,
}

#[cfg(feature = "mobile-host")]
pub(crate) fn hosted(root: &Path, viewport: Option<[f32; 2]>) -> Result<HostedPackage> {
    let manifest = verify(root)?;
    let camera = camera(&manifest)?;
    let mut id = [0; 16];
    for (i, byte) in id.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&camera[i * 2..i * 2 + 2], 16)?;
    }
    Ok(HostedPackage {
        scene: incant_runtime::CookedScene::from_bytes(&bounded(
            &path(root, "world.scene")?,
            128 * 1024 * 1024,
        )?)?,
        actions: serde_json::from_slice(&bounded(&path(root, "input-actions.json")?, 256 * 1024)?)?,
        clips: crate::startup::clips(&path(root, "audio")?, &path(root, "clips.bin")?)?,
        settings: incant_render_game::NativeRenderSettings {
            model_cache: root.join("models"),
            texture_cache: root.join("textures"),
            camera: incant_runtime::StableId(id),
            text: Some(incant_render_game::TextSettings::new(root.join("fonts"))),
            ui: viewport.map(incant_render_game::UiSettings::new),
            consumer: (),
        },
    })
}

fn camera(manifest: &Manifest) -> Result<String> {
    let camera = manifest
        .camera
        .as_deref()
        .ok_or("native window package requires a camera")?;
    if camera.len() != 32 || !camera.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("native package camera must be 32 hexadecimal digits".into());
    }
    Ok(camera.to_owned())
}
fn path(root: &Path, name: &str) -> Result<String> {
    let path = root.join(name);
    let text = path.to_str().ok_or("native package path must be UTF-8")?;
    if text.chars().any(char::is_control) {
        return Err("control character in native package path".into());
    }
    Ok(text.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let mut files = BTreeMap::new();
        for name in [
            "world.scene",
            "input-actions.json",
            "clips.bin",
            "empty-input.json",
        ] {
            std::fs::write(dir.path().join(name), name).unwrap();
            files.insert(name, incant_assets_runtime::sha256(name.as_bytes()));
        }
        std::fs::write(dir.path().join("package.json"), serde_json::to_vec(&serde_json::json!({
            "format": "incant-native-package", "version": 1, "camera": "00000000000000000000000000000001", "files": files,
        })).unwrap()).unwrap();
        dir
    }
    fn args(dir: &Path, tail: &[&str]) -> Vec<String> {
        [
            vec!["--package".to_owned(), dir.to_str().unwrap().to_owned()],
            tail.iter().map(|s| (*s).into()).collect(),
        ]
        .concat()
    }
    #[test]
    fn presentation_destination_is_only_an_explicit_window_argument() {
        let dir = fixture();
        let expanded = expand(args(
            dir.path(),
            &[
                "--window",
                "--stop-after",
                "6",
                "--presentation-output",
                "trusted.json",
                "--presentation-capacity",
                "8",
            ],
        ))
        .unwrap();
        assert_eq!(
            &expanded[10..],
            [
                "--stop-after",
                "6",
                "--presentation-output",
                "trusted.json",
                "--presentation-capacity",
                "8"
            ]
        );
        assert!(
            expand(args(
                dir.path(),
                &[
                    "--ticks",
                    "6",
                    "--output",
                    "sound.pcm",
                    "--presentation-output",
                    "not-allowed.json"
                ]
            ))
            .is_err()
        );
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&std::fs::read(dir.path().join("package.json")).unwrap())
                .unwrap();
        manifest["presentation_output"] = "project-controlled.json".into();
        std::fs::write(
            dir.path().join("package.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        assert!(expand(args(dir.path(), &["--window", "--stop-after", "6"])).is_err());
    }
    #[test]
    fn offline_and_window_expand_to_existing_hosts() {
        let dir = fixture();
        let offline = expand(args(
            dir.path(),
            &[
                "--ticks",
                "4",
                "--output",
                "audio.pcm",
                "--observe",
                "assertions.json",
            ],
        ))
        .unwrap();
        assert_eq!(
            &offline[5..9],
            ["4", "48000", "audio.pcm", "--cpu-render-input"]
        );
        assert_eq!(&offline[14..], ["--observe", "assertions.json"]);
        let (settings, plan) = crate::observe::split_args(&offline);
        assert_eq!(plan, Some("assertions.json"));
        assert_eq!(settings.len(), 14);
        #[cfg(feature = "render-input")]
        assert!(matches!(
            crate::session::settings(settings).unwrap(),
            Some(crate::session::Settings::Cpu(_))
        ));
        let window = expand(args(dir.path(), &["--window", "--stop-after", "3"])).unwrap();
        assert_eq!(window[0], "--window");
        assert_eq!(window[8], "--font-cache");
        assert!(window[9].ends_with("fonts"));
        assert_eq!(&window[10..], ["--stop-after", "3"]);
        assert!(expand(args(dir.path(), &["--window", "--output", "audio.pcm"])).is_err());
        assert!(
            expand(args(
                dir.path(),
                &["--ticks", "4", "--ticks", "5", "--output", "audio.pcm"]
            ))
            .is_err()
        );
        assert!(
            expand(args(
                dir.path(),
                &["--ticks", "4", "--output", "audio.pcm", "--stop-after", "1"]
            ))
            .is_err()
        );
    }
    #[cfg(feature = "render-input")]
    #[test]
    fn ui_font_and_observation_options_reach_the_same_composed_host() {
        let dir = fixture();
        let expanded = expand(args(
            dir.path(),
            &[
                "--ticks",
                "12",
                "--output",
                "out.pcm",
                "--ui-viewport",
                "320x200",
                "--observe",
                "plan.json",
            ],
        ))
        .unwrap();
        let (args, plan) = crate::observe::split_args(&expanded);
        assert_eq!(plan, Some("plan.json"));
        let Some(crate::session::Settings::Cpu(settings)) = crate::session::settings(args).unwrap()
        else {
            panic!("CPU host")
        };
        assert_eq!(settings.ui.unwrap().viewport, [320., 200.]);
        assert!(settings.text.is_some());
    }
    #[test]
    fn window_replay_expands_only_explicit_bounded_inputs() {
        let dir = fixture();
        let expanded = expand(args(
            dir.path(),
            &[
                "--window",
                "--input-recording",
                "route.json",
                "--stop-after",
                "12",
                "--observe",
                "checks.json",
            ],
        ))
        .unwrap();
        assert_eq!(
            &expanded[10..],
            [
                "--stop-after",
                "12",
                "--input-recording",
                "route.json",
                "--observe",
                "checks.json"
            ]
        );
        for tail in [
            vec!["--window", "--input-recording", "route.json"],
            vec!["--window", "--observe", "checks.json"],
            vec!["--window", "--stop-after", "12", "--ui-viewport", "640x480"],
            vec!["--window", "--stop-after", "12", "--ticks", "12"],
        ] {
            assert!(expand(args(dir.path(), &tail)).is_err(), "{tail:?}");
        }
    }
    #[test]
    fn tampered_or_indirected_startup_files_are_refused() {
        let dir = fixture();
        std::fs::write(dir.path().join("world.scene"), "changed").unwrap();
        let error = expand(args(dir.path(), &["--window"])).unwrap_err();
        assert!(error.to_string().contains("checksum"));
        let file = dir.path().join("package.json");
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
        manifest["files"]["../../outside"] = "a".into();
        std::fs::write(&file, serde_json::to_vec(&manifest).unwrap()).unwrap();
        assert!(
            expand(args(dir.path(), &["--window"]))
                .unwrap_err()
                .to_string()
                .contains("file set")
        );
    }
}
