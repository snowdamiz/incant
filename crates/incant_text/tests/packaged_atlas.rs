//! Real public cooked-font reader -> TextEngine -> atlas after cache deletion.
use incant_assets_runtime::{
    Dependency,
    font::{FONT_MAGIC, FontMetadata, font_key, load_font},
};
use incant_text::{AtlasConfig, GlyphAtlas, TextEngine, TextStyle, Wrap};
use sha2::{Digest, Sha256};
#[test]
fn checked_packaged_font_rasterizes_after_source_and_cache_are_gone() {
    let cache = tempfile::tempdir().unwrap();
    let data = include_bytes!("fonts/IncantCJKFixture.ttf");
    let digest = Sha256::digest(data);
    let dependency = Dependency {
        path: "fonts/cjk.ttf".into(),
        sha256: format!("{digest:x}"),
    };
    let key = font_key(&dependency).unwrap();
    let metadata = FontMetadata {
        version: 1,
        fingerprint: key.clone(),
        dependency,
        bytes: data.len() as u32,
        content_sha256: format!("{digest:x}"),
    };
    let mut bytes = Vec::from(*FONT_MAGIC);
    bytes.extend(1_u32.to_le_bytes());
    bytes.extend((data.len() as u32).to_le_bytes());
    bytes.extend(digest);
    bytes.extend(data);
    std::fs::write(
        cache
            .path()
            .join(format!("{}.ifont", metadata.content_sha256)),
        bytes,
    )
    .unwrap();
    std::fs::write(
        cache.path().join(format!("{key}.json")),
        serde_json::to_vec(&metadata).unwrap(),
    )
    .unwrap();
    let source = load_font(cache.path(), &key)
        .unwrap()
        .into_source(ulid::Ulid::from(9));
    cache.close().unwrap();
    let mut engine = TextEngine::new(vec![source]).unwrap();
    let shape = engine
        .shape(
            "中文",
            TextStyle {
                font_size: 32.,
                line_height: 40.,
                width: 200.,
                wrap: Wrap::None,
            },
        )
        .unwrap();
    assert!(shape.missing_clusters().is_empty());
    let mut atlas = GlyphAtlas::new(AtlasConfig {
        width: 128,
        height: 128,
        max_glyphs: 16,
    })
    .unwrap();
    for i in 0..shape.glyphs().len() {
        let handle = engine.cache_glyph(&shape, i, &mut atlas).unwrap();
        assert!(atlas.get(&handle).unwrap().rect.is_some());
    }
    assert!(atlas.pixels().iter().any(|&p| p != 0));
}
