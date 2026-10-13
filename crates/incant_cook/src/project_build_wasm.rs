//! Admit the complete engine-generated static bundle, never native project code.
//! The external trusted exporter creates it privately from validated Wasm.
use incant_wasm_cook::CapturedModule;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, io::Read, path::Path};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub(super) fn destination(target: &str) -> Result<()> {
    if !matches!(target, "aarch64-apple-ios" | "aarch64-apple-ios-sim") {
        return Err("authored mobile static Wasm requires an explicit iOS destination".into());
    }
    Ok(())
}

pub(super) fn validate(bundle: &Path, captured: &CapturedModule) -> Result<()> {
    if !fs::symlink_metadata(bundle)?.file_type().is_dir() {
        return Err("mobile static bundle must be a trusted directory".into());
    }
    let manifest: serde_json::Value =
        serde_json::from_slice(&read(&bundle.join("manifest.json"), 1024 * 1024)?)?;
    let expected = serde_json::json!([{
        "name":"scene-program", "prefix":"m0",
        "input_sha256":captured.input_sha256(),
        "prepared_sha256":captured.prepared_sha256(),
    }]);
    let object = manifest
        .as_object()
        .ok_or("static bundle manifest object")?;
    if object.len() != 3 || manifest["wabt"] != "1.0.42" || manifest["modules"] != expected {
        return Err("mobile static bundle captured identity mismatch".into());
    }
    let files = manifest["files"]
        .as_object()
        .ok_or("static bundle manifest files")?;
    let names = [
        "m0.original.wasm",
        "m0.wasm",
        "m0.c",
        "m0.h",
        "m0-facade.c",
        "linked.rs",
    ];
    let expected_names: BTreeSet<_> = names.into_iter().collect();
    if files.keys().map(String::as_str).collect::<BTreeSet<_>>() != expected_names {
        return Err("mobile static bundle file set mismatch".into());
    }
    let actual_names: BTreeSet<_> = fs::read_dir(bundle)?
        .map(|entry| entry.map(|entry| entry.file_name()))
        .collect::<std::io::Result<_>>()?;
    let complete: BTreeSet<_> = names
        .into_iter()
        .chain(["manifest.json"])
        .map(std::ffi::OsString::from)
        .collect();
    if actual_names != complete {
        return Err("mobile static bundle contains unexpected build inputs".into());
    }
    for name in names {
        let bytes = read(&bundle.join(name), 128 * 1024 * 1024)?;
        if files[name].as_str() != Some(format!("{:x}", Sha256::digest(&bytes)).as_str()) {
            return Err("mobile static bundle checksum mismatch".into());
        }
        if (name == "m0.original.wasm" && bytes != captured.original())
            || (name == "m0.wasm" && bytes != captured.prepared())
        {
            return Err("mobile static bundle input differs from captured module".into());
        }
    }
    Ok(())
}

fn read(path: &Path, cap: u64) -> Result<Vec<u8>> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() || metadata.len() > cap {
        return Err("mobile static bundle requires bounded regular files".into());
    }
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(cap + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > cap {
        return Err("mobile static bundle file exceeds limit".into());
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn other_mobile_bytecode_routes_cannot_admit_static_wasm() {
        destination("aarch64-apple-ios").unwrap();
        destination("aarch64-apple-ios-sim").unwrap();
        for target in [
            "aarch64-linux-android",
            "aarch64-apple-darwin",
            "x86_64-apple-ios",
            "wasm32-unknown-unknown",
        ] {
            assert_eq!(
                destination(target).unwrap_err().to_string(),
                "authored mobile static Wasm requires an explicit iOS destination"
            );
        }
    }

    fn fixture() -> (tempfile::TempDir, CapturedModule) {
        // (module (memory (export "memory") 1)
        //   (func (export "incant_abi") (result i32) i32.const 1)
        //   (func (export "update") (param i64 f64)))
        let bytes = b"\0asm\x01\0\0\0\x01\x0a\x02\x60\0\x01\x7f\x60\x02\x7e\x7c\0\x03\x03\x02\0\x01\x05\x03\x01\0\x01\x07\x20\x03\x06memory\x02\0\x0aincant_abi\0\0\x06update\0\x01\x0a\x09\x02\x04\0\x41\x01\x0b\x02\0\x0b";
        let captured = CapturedModule::capture(bytes).unwrap();
        let root = tempfile::tempdir().unwrap();
        let mut files = serde_json::Map::new();
        for (name, data) in [
            ("m0.original.wasm", captured.original()),
            ("m0.wasm", captured.prepared()),
            ("m0.c", b"trusted test C".as_slice()),
            ("m0.h", b"trusted test header".as_slice()),
            ("m0-facade.c", b"trusted test facade".as_slice()),
            ("linked.rs", b"trusted test binding".as_slice()),
        ] {
            fs::write(root.path().join(name), data).unwrap();
            files.insert(name.into(), format!("{:x}", Sha256::digest(data)).into());
        }
        let manifest = serde_json::json!({"wabt":"1.0.42","modules":[{
            "name":"scene-program","prefix":"m0","input_sha256":captured.input_sha256(),
            "prepared_sha256":captured.prepared_sha256()}],"files":files});
        fs::write(
            root.path().join("manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        (root, captured)
    }

    #[test]
    fn complete_manifest_is_required_and_changed_generated_input_is_refused() {
        let (root, captured) = fixture();
        validate(root.path(), &captured).unwrap();
        fs::write(root.path().join("m0.c"), b"replaced native code").unwrap();
        assert_eq!(
            validate(root.path(), &captured).unwrap_err().to_string(),
            "mobile static bundle checksum mismatch"
        );
        fs::remove_file(root.path().join("manifest.json")).unwrap();
        assert!(validate(root.path(), &captured).is_err());
    }

    #[test]
    fn valid_hashes_cannot_admit_a_different_module_or_extra_compiler_input() {
        let (root, captured) = fixture();
        let path = root.path().join("manifest.json");
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        manifest["modules"][0]["prepared_sha256"] = "0".repeat(64).into();
        fs::write(&path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        assert_eq!(
            validate(root.path(), &captured).unwrap_err().to_string(),
            "mobile static bundle captured identity mismatch"
        );
        let (root, captured) = fixture();
        fs::write(root.path().join("extra.c"), b"unexpected").unwrap();
        assert_eq!(
            validate(root.path(), &captured).unwrap_err().to_string(),
            "mobile static bundle contains unexpected build inputs"
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_generated_code_is_refused_even_with_matching_bytes() {
        let (root, captured) = fixture();
        let external = tempfile::tempdir().unwrap();
        let path = root.path().join("m0.c");
        fs::rename(&path, external.path().join("m0.c")).unwrap();
        std::os::unix::fs::symlink(external.path().join("m0.c"), &path).unwrap();
        assert_eq!(
            validate(root.path(), &captured).unwrap_err().to_string(),
            "mobile static bundle requires bounded regular files"
        );
    }
}
