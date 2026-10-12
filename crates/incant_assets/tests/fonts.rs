use incant_assets::{MAX_FONT_BYTES, SourceSet, cook_font};
use std::{fs, path::Path};
const FONT: &[u8] = include_bytes!("../../incant_text/tests/fonts/NotoSans-Regular.ttf");

#[test]
fn font_cook_is_lossless_cache_checked_and_rebuilds_corrupt_entries() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let cache = root.join("cache");
    fs::write(root.join("font.ttf"), FONT).unwrap();
    let first = cook_font(root, Path::new("font.ttf"), &cache).unwrap();
    assert!(!first.cache_hit);
    assert_eq!(first.bytes(), FONT);
    let second = cook_font(root, Path::new("font.ttf"), &cache).unwrap();
    assert!(second.cache_hit);
    assert_eq!(first.metadata().fingerprint, second.metadata().fingerprint);
    let blob = cache.join(format!("{}.ifont", first.metadata().content_sha256));
    fs::write(&blob, b"broken").unwrap();
    let repaired = cook_font(root, Path::new("font.ttf"), &cache).unwrap();
    assert!(!repaired.cache_hit);
    assert_eq!(repaired.bytes(), FONT);
    assert_eq!(first.bytes(), FONT);
    // Both accepted source extensions share the single-sfnt policy, not invented
    // assumptions about the outlines or user-facing font family name.
    fs::write(root.join("FONT.OTF"), FONT).unwrap();
    assert_eq!(
        cook_font(root, Path::new("FONT.OTF"), &cache)
            .unwrap()
            .bytes(),
        FONT
    );
}
#[test]
fn font_source_bounds_malformed_faces_and_paths_fail_before_publication() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let cache = root.join("cache");
    for bytes in [
        &b"bad"[..],
        &b"ttcf\0\x01\0\0\xff\xff\xff\xff"[..],
        &b"\0\x01\0\0"[..],
    ] {
        fs::write(root.join("font.ttf"), bytes).unwrap();
        assert!(cook_font(root, Path::new("font.ttf"), &cache).is_err());
        assert!(!cache.exists());
    }
    fs::File::create(root.join("font.ttf"))
        .unwrap()
        .set_len(MAX_FONT_BYTES as u64 + 1)
        .unwrap();
    assert!(cook_font(root, Path::new("font.ttf"), &cache).is_err());
    assert!(!cache.exists());
    fs::write(root.join("font.ttf"), FONT).unwrap();
    assert!(cook_font(root, Path::new("../font.ttf"), &cache).is_err());
    assert!(cook_font(root, &root.join("font.ttf"), &cache).is_err());
    let mut sources = SourceSet::new(root).unwrap();
    sources.read(Path::new("font.ttf")).unwrap();
    assert!(sources.read_limited(Path::new("font.ttf"), 1).is_err());
}
