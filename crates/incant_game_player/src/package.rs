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

pub fn expand(args: Vec<String>) -> Result<Vec<String>> {
    if args.first().is_none_or(|arg| arg != "--package") {
        return Ok(args);
    }
    let root = Path::new(args.get(1).ok_or("--package needs a data directory")?);
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
            "--ticks" | "--output" | "--observe" | "--input-recording" | "--stop-after"
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
        if options.keys().any(|key| *key != "--stop-after") {
            return Err("window package options conflict with offline options".into());
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
        if let Some(stop) = options.remove("--stop-after") {
            result.extend(["--stop-after".into(), stop]);
        }
        return Ok(result);
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
    if let Some(plan) = options.remove("--observe") {
        result.extend(["--observe".into(), plan]);
    }
    Ok(result)
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
