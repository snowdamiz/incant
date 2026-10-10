use incant_assets::{TextureUsage, cook_texture, load_texture};
use serde_json::{Value, json};
use std::path::Path;
fn png(path: &Path, w: u32, h: u32, color: [u8; 4]) {
    image::RgbaImage::from_pixel(w, h, image::Rgba(color))
        .save(path)
        .unwrap();
}
fn descriptor(root: &Path) -> Value {
    let value = json!({"version":1,"id":"00000000000000000000000009","maximum_size":[64,64],"padding":2,
    "frames":{
        "00000000000000000000000001":{"name":"First","path":"a.png"},
        "00000000000000000000000002":{"name":"Second","path":"b.png","region":[1,1,2,3]}
    }});
    std::fs::write(
        root.join("test.atlas.json"),
        serde_json::to_vec(&value).unwrap(),
    )
    .unwrap();
    value
}
#[test]
fn atlases_extrude_preserve_ids_track_dependencies_and_load_without_sources() {
    let root = tempfile::tempdir().unwrap();
    let cache = root.path().join("cache");
    png(&root.path().join("a.png"), 3, 2, [255, 0, 0, 255]);
    png(&root.path().join("b.png"), 5, 5, [0, 128, 255, 128]);
    descriptor(root.path());
    let first = cook_texture(
        root.path(),
        Path::new("test.atlas.json"),
        &cache,
        TextureUsage::Color,
    )
    .unwrap();
    assert!(!first.cache_hit);
    assert_eq!(first.metadata.all_dependencies().len(), 3);
    let layout = first.metadata.atlas.as_ref().unwrap();
    layout.validate().unwrap();
    assert_eq!(layout.frames.len(), 2);
    assert_eq!(
        first.texture.format,
        incant_assets::TextureFormat::Rgba8Srgb
    );
    for (id, expected) in [
        ("00000000000000000000000001", [255, 0, 0, 255]),
        ("00000000000000000000000002", [0, 128, 255, 128]),
    ] {
        let [x, y, w, h] = layout.frames[id].region;
        for row in y - 2..y + h + 2 {
            for col in x - 2..x + w + 2 {
                let index = ((row * first.texture.width + col) * 4) as usize;
                assert_eq!(&first.texture.levels[0][index..index + 4], &expected);
            }
        }
    }
    assert!(
        cook_texture(
            root.path(),
            Path::new("test.atlas.json"),
            &cache,
            TextureUsage::Color
        )
        .unwrap()
        .cache_hit
    );
    png(&root.path().join("a.png"), 13, 7, [0, 255, 0, 255]);
    let second = cook_texture(
        root.path(),
        Path::new("test.atlas.json"),
        &cache,
        TextureUsage::Color,
    )
    .unwrap();
    assert_ne!(first.metadata.fingerprint, second.metadata.fingerprint);
    assert_eq!(
        first
            .metadata
            .atlas
            .as_ref()
            .unwrap()
            .frames
            .keys()
            .collect::<Vec<_>>(),
        second
            .metadata
            .atlas
            .as_ref()
            .unwrap()
            .frames
            .keys()
            .collect::<Vec<_>>()
    );
    assert_eq!(
        &second.metadata.atlas.as_ref().unwrap().frames["00000000000000000000000001"].region[2..],
        &[13, 7]
    );
    for file in ["test.atlas.json", "a.png", "b.png"] {
        std::fs::remove_file(root.path().join(file)).unwrap();
    }
    assert_eq!(
        load_texture(&cache, &first.metadata.fingerprint)
            .unwrap()
            .texture,
        first.texture
    );
    assert_eq!(
        load_texture(&cache, &second.metadata.fingerprint)
            .unwrap()
            .texture,
        second.texture
    );
    // A forged frame layout cannot retain the original key.
    let manifest = cache.join(format!("{}.json", second.metadata.fingerprint));
    let mut corrupted: Value = serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
    corrupted["atlas"]["frames"]["00000000000000000000000001"]["name"] = json!("Forged");
    std::fs::write(manifest, serde_json::to_vec(&corrupted).unwrap()).unwrap();
    assert!(load_texture(&cache, &second.metadata.fingerprint).is_err());
}
#[test]
fn invalid_atlas_sources_and_options_never_replace_a_valid_cache() {
    let root = tempfile::tempdir().unwrap();
    let cache = root.path().join("cache");
    png(&root.path().join("a.png"), 3, 2, [255, 0, 0, 255]);
    png(&root.path().join("b.png"), 5, 5, [0, 128, 255, 128]);
    let value = descriptor(root.path());
    let old = cook_texture(
        root.path(),
        Path::new("test.atlas.json"),
        &cache,
        TextureUsage::Color,
    )
    .unwrap();
    for usage in [TextureUsage::Linear, TextureUsage::Normal] {
        assert!(cook_texture(root.path(), Path::new("test.atlas.json"), &cache, usage).is_err());
    }
    for (field, invalid) in [
        ("path", json!("../outside.png")),
        ("region", json!([4, 4, 2, 3])),
    ] {
        let mut bad = value.clone();
        bad["frames"]["00000000000000000000000002"][field] = invalid;
        std::fs::write(
            root.path().join("test.atlas.json"),
            serde_json::to_vec(&bad).unwrap(),
        )
        .unwrap();
        assert!(
            cook_texture(
                root.path(),
                Path::new("test.atlas.json"),
                &cache,
                TextureUsage::Color
            )
            .is_err()
        );
        assert_eq!(
            load_texture(&cache, &old.metadata.fingerprint)
                .unwrap()
                .texture,
            old.texture
        );
    }
}
