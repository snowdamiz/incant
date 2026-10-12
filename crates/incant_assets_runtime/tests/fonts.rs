#![cfg(feature = "font")]
use incant_assets_runtime::font::{
    FONT_HEADER_BYTES, FONT_MAGIC, MAX_FONT_BYTES, font_key, font_path,
};
use incant_assets_runtime::{Dependency, FontMetadata, load_font};
use sha2::{Digest, Sha256};
use std::{fs, io::Write};
const FONT: &[u8] = include_bytes!("../../incant_text/tests/fonts/NotoSans-Regular.ttf");

fn cooked(cache: &std::path::Path, bytes: &[u8]) -> String {
    let dependency = Dependency {
        path: "deleted/font.ttf".into(),
        sha256: incant_assets_runtime::sha256(bytes),
    };
    let key = font_key(&dependency).unwrap();
    let metadata = FontMetadata {
        version: 1,
        fingerprint: key.clone(),
        dependency,
        bytes: bytes.len() as u32,
        content_sha256: incant_assets_runtime::sha256(bytes),
    };
    let mut header = [0_u8; FONT_HEADER_BYTES];
    header[..8].copy_from_slice(FONT_MAGIC);
    header[8..12].copy_from_slice(&1_u32.to_le_bytes());
    header[12..16].copy_from_slice(&metadata.bytes.to_le_bytes());
    header[16..].copy_from_slice(&Sha256::digest(bytes));
    let mut file =
        fs::File::create(font_path(cache, &metadata.content_sha256, "ifont").unwrap()).unwrap();
    file.write_all(&header).unwrap();
    file.write_all(bytes).unwrap();
    fs::write(
        font_path(cache, &key, "json").unwrap(),
        serde_json::to_vec(&metadata).unwrap(),
    )
    .unwrap();
    key
}
#[test]
fn cooked_font_shapes_without_any_source_and_retains_immutable_bytes() {
    let temp = tempfile::tempdir().unwrap();
    let key = cooked(temp.path(), FONT);
    let retained = load_font(temp.path(), &key).unwrap();
    assert_eq!(retained.bytes(), FONT);
    assert_eq!(retained.metadata().bytes as usize, FONT.len());
    assert!(!temp.path().join("deleted/font.ttf").exists());
    let font = load_font(temp.path(), &key).unwrap();
    let mut engine = incant_text::TextEngine::new(vec![font.into_source(1.into())]).unwrap();
    let shaped = engine
        .shape(
            "ffi e\u{301}",
            incant_text::TextStyle {
                font_size: 20.,
                line_height: 30.,
                width: 800.,
                wrap: incant_text::Wrap::Word,
            },
        )
        .unwrap();
    assert!(shaped.missing_clusters().is_empty());
    assert!(shaped.glyphs().iter().any(|g| g.source == (0..3)));
    let blob = font_path(temp.path(), &retained.metadata().content_sha256, "ifont").unwrap();
    fs::write(blob, b"corrupt").unwrap();
    assert!(load_font(temp.path(), &key).is_err());
    assert_eq!(retained.bytes(), FONT);
}
#[test]
fn untrusted_font_manifest_header_hash_and_shaping_face_are_checked() {
    let temp = tempfile::tempdir().unwrap();
    let key = cooked(temp.path(), FONT);
    let manifest = font_path(temp.path(), &key, "json").unwrap();
    let good = fs::read(&manifest).unwrap();
    let json: serde_json::Value = serde_json::from_slice(&good).unwrap();
    for (field, value) in [
        ("version", serde_json::json!(2)),
        ("bytes", serde_json::json!(0)),
        ("bytes", serde_json::json!(u32::MAX)),
        ("fingerprint", serde_json::json!("../escape")),
        ("content_sha256", serde_json::json!("f".repeat(64))),
        ("extra", serde_json::json!(1)),
    ] {
        let mut bad = json.clone();
        bad[field] = value;
        fs::write(&manifest, serde_json::to_vec(&bad).unwrap()).unwrap();
        assert!(load_font(temp.path(), &key).is_err(), "{field}");
    }
    for path in ["../escape.ttf", "a//b.ttf", "a\\b.ttf", "/font.ttf"] {
        let mut bad = json.clone();
        bad["dependency"]["path"] = serde_json::json!(path);
        fs::write(&manifest, serde_json::to_vec(&bad).unwrap()).unwrap();
        assert!(load_font(temp.path(), &key).is_err());
    }
    fs::write(&manifest, &good).unwrap();
    let blob = font_path(
        temp.path(),
        json["content_sha256"].as_str().unwrap(),
        "ifont",
    )
    .unwrap();
    let original = fs::read(&blob).unwrap();
    for index in [0, 8, 12, 16, FONT_HEADER_BYTES] {
        let mut bad = original.clone();
        bad[index] ^= 1;
        fs::write(&blob, bad).unwrap();
        assert!(load_font(temp.path(), &key).is_err(), "byte{index}");
    }
    let mut trailing = original.clone();
    trailing.push(0);
    fs::write(&blob, trailing).unwrap();
    assert!(load_font(temp.path(), &key).is_err());
    fs::write(&blob, &original[..original.len() - 1]).unwrap();
    assert!(load_font(temp.path(), &key).is_err());
    let malformed = cooked(temp.path(), b"\0\x01\0\0");
    assert!(load_font(temp.path(), &malformed).is_err());
    let collection = cooked(temp.path(), b"ttcf\0\x01\0\0\xff\xff\xff\xff");
    assert!(load_font(temp.path(), &collection).is_err());
    assert!(load_font(temp.path(), "../escape").is_err());
    fs::File::create(&manifest).unwrap().set_len(8193).unwrap();
    assert!(load_font(temp.path(), &key).is_err());
    fs::write(&manifest, good).unwrap();
    fs::File::create(&blob)
        .unwrap()
        .set_len((MAX_FONT_BYTES + FONT_HEADER_BYTES + 1) as u64)
        .unwrap();
    assert!(load_font(temp.path(), &key).is_err());
}
#[test]
#[cfg(unix)]
fn cooked_font_reader_refuses_symlink_files() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let key = cooked(temp.path(), FONT);
    let manifest = font_path(temp.path(), &key, "json").unwrap();
    let font = load_font(temp.path(), &key).unwrap();
    let blob = font_path(temp.path(), &font.metadata().content_sha256, "ifont").unwrap();
    for path in [manifest, blob] {
        let target = outside.path().join(path.file_name().unwrap());
        fs::rename(&path, &target).unwrap();
        symlink(&target, &path).unwrap();
        assert!(load_font(temp.path(), &key).is_err());
        fs::remove_file(&path).unwrap();
        fs::rename(target, path).unwrap();
    }
}
