use incant_text::{AtlasConfig, FontSource, GlyphAtlas, TextEngine, TextError, TextStyle, Wrap};
use ulid::Ulid;
const LATIN: &[u8] = include_bytes!("fonts/NotoSans-Regular.ttf");
fn engine() -> TextEngine {
    TextEngine::new(
        [
            LATIN,
            include_bytes!("fonts/NotoSansArabic.ttf"),
            include_bytes!("fonts/NotoSansHebrew.ttf"),
            include_bytes!("fonts/IncantCJKFixture.ttf"),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, b)| FontSource {
            id: Ulid::from(i as u128 + 1),
            bytes: b.to_vec(),
        })
        .collect(),
    )
    .unwrap()
}
fn style(size: f32) -> TextStyle {
    TextStyle {
        font_size: size,
        line_height: size * 2.,
        width: 1000.,
        wrap: Wrap::None,
    }
}
fn atlas(width: u32, height: u32, max_glyphs: usize) -> GlyphAtlas {
    GlyphAtlas::new(AtlasConfig {
        width,
        height,
        max_glyphs,
    })
    .unwrap()
}
fn coverage(atlas: &GlyphAtlas, rect: [u32; 4]) -> Vec<u8> {
    let [x, y, w, h] = rect;
    let stride = atlas.size()[0] as usize;
    (y..y + h)
        .flat_map(|row| {
            atlas.pixels()
                [row as usize * stride + x as usize..row as usize * stride + (x + w) as usize]
                .iter()
                .copied()
        })
        .collect()
}
#[test]
fn real_latin_rtl_and_cjk_outlines_have_nonempty_antialiased_coverage() {
    let mut engine = engine();
    let mut atlas = atlas(512, 512, 256);
    for (text, font) in [("Aé", 1), ("مرحبا", 2), ("שלום", 3), ("中文日本語一", 4)]
    {
        let shaped = engine.shape(text, style(32.)).unwrap();
        assert!(shaped.missing_clusters().is_empty());
        assert!(shaped.glyphs().iter().all(|g| g.font == Ulid::from(font)));
        if font == 2 || font == 3 {
            assert!(shaped.lines()[0].rtl);
        }
        for i in 0..shaped.glyphs().len() {
            let handle = engine.cache_glyph(&shaped, i, &mut atlas).unwrap();
            let entry = atlas.get(&handle).unwrap();
            assert!(!entry.missing);
            let r = entry.rect.unwrap();
            assert!(r[2] > 0 && r[3] > 0 && r[2] < 80 && r[3] < 80);
            let pixels = coverage(&atlas, r);
            assert!(pixels.iter().any(|&x| x > 0));
            assert!(pixels.iter().any(|&x| x > 0 && x < 255));
            // One-pixel guards remain zero and cannot sample neighboring glyphs.
            let [x, y, w, h] = r;
            let stride = atlas.size()[0] as usize;
            for row in y - 1..=y + h {
                assert_eq!(atlas.pixels()[row as usize * stride + (x - 1) as usize], 0);
                assert_eq!(atlas.pixels()[row as usize * stride + (x + w) as usize], 0);
            }
            for col in x - 1..=x + w {
                assert_eq!(atlas.pixels()[(y - 1) as usize * stride + col as usize], 0);
                assert_eq!(atlas.pixels()[(y + h) as usize * stride + col as usize], 0);
            }
        }
    }
}
#[test]
fn unchanged_reuse_size_identity_and_reset_are_explicit() {
    let mut e = engine();
    let mut a = atlas(512, 512, 128);
    let s = e.shape("AA", style(24.)).unwrap();
    let h = e.cache_glyph(&s, 0, &mut a).unwrap();
    let first = *a.get(&h).unwrap();
    let revision = a.revision();
    let pointer = a.pixels().as_ptr();
    for _ in 0..100 {
        let h = e.cache_glyph(&s, 1, &mut a).unwrap();
        assert_eq!(*a.get(&h).unwrap(), first);
    }
    assert_eq!(a.len(), 1);
    assert_eq!(a.revision(), revision);
    assert_eq!(a.pixels().as_ptr(), pointer);
    let large = e.shape("A", style(48.)).unwrap();
    let h2 = e.cache_glyph(&large, 0, &mut a).unwrap();
    let second = *a.get(&h2).unwrap();
    assert_ne!(first.key, second.key);
    assert!(second.rect.unwrap()[3] >= first.rect.unwrap()[3] * 2 - 1);
    assert_eq!(*a.get(&h).unwrap(), first); // appending never invalidates placement
    let other = atlas(512, 512, 128);
    assert!(other.get(&h).is_none());
    a.reset().unwrap();
    assert!(a.get(&h).is_none());
    assert!(a.get(&h2).is_none());
    assert!(a.pixels().iter().all(|&n| n == 0));
    assert_eq!(a.pixels().as_ptr(), pointer);
    let h3 = e.cache_glyph(&s, 0, &mut a).unwrap();
    assert_eq!(*a.get(&h3).unwrap(), first);
    assert!(a.get(&h).is_none());
}
#[test]
fn missing_and_space_are_distinct_and_exhaustion_is_atomic() {
    let mut e = engine();
    let mut a = atlas(64, 64, 2);
    let s = e.shape(" \u{10ffff}A", style(20.)).unwrap();
    let space = e.cache_glyph(&s, 0, &mut a).unwrap();
    assert!(a.get(&space).unwrap().rect.is_none());
    assert!(!a.get(&space).unwrap().missing);
    let missing = e.cache_glyph(&s, 1, &mut a).unwrap();
    assert!(a.get(&missing).unwrap().missing);
    assert!(!s.missing_clusters().is_empty());
    let pixels = a.pixels().to_vec();
    let revision = a.revision();
    assert!(matches!(
        e.cache_glyph(&s, 2, &mut a),
        Err(TextError::Limit("atlas glyph count"))
    ));
    assert_eq!(a.revision(), revision);
    assert_eq!(a.pixels(), pixels);
    assert!(a.get(&missing).is_some());
    let mut tiny = atlas(8, 8, 64);
    assert!(e.cache_glyph(&s, 2, &mut tiny).is_err());
    assert_eq!(tiny.revision(), 0);
    assert!(tiny.is_empty());
    assert!(tiny.pixels().iter().all(|&v| v == 0));
    let mut full = atlas(32, 32, 64);
    let text = e.shape("ABCDEFGHIJKLMNOPQRSTUVWXYZ", style(20.)).unwrap();
    let mut refused = false;
    for i in 0..text.glyphs().len() {
        let old = full.pixels().to_vec();
        let len = full.len();
        let rev = full.revision();
        if e.cache_glyph(&text, i, &mut full).is_err() {
            assert_eq!(full.pixels(), old);
            assert_eq!(full.len(), len);
            assert_eq!(full.revision(), rev);
            refused = true;
            break;
        }
    }
    assert!(refused);
}
#[test]
fn reloaded_font_identity_cannot_reinterpret_old_shaped_glyph_ids() {
    let mut old = engine();
    let shape = old.shape("A", style(24.)).unwrap();
    let mut a = atlas(256, 256, 16);
    let h = old.cache_glyph(&shape, 0, &mut a).unwrap();
    let entry = *a.get(&h).unwrap();
    // Same asset id, changed immutable bytes. The glyph ID may mean another face.
    let mut new = TextEngine::new(vec![FontSource {
        id: Ulid::from(1),
        bytes: include_bytes!("fonts/IncantCJKFixture.ttf").to_vec(),
    }])
    .unwrap();
    assert!(new.cache_glyph(&shape, 0, &mut a).is_err());
    assert_eq!(a.len(), 1);
    let next = new.shape("中", style(24.)).unwrap();
    let nh = new.cache_glyph(&next, 0, &mut a).unwrap();
    assert_ne!(
        a.get(&nh).unwrap().key.content_sha256,
        entry.key.content_sha256
    );
    assert_eq!(*a.get(&h).unwrap(), entry);
    assert!(old.cache_glyph(&shape, usize::MAX, &mut a).is_err());

    // A reimport can keep the exact same glyph ID and size while changing its
    // physical outline scale. Such a key must not alias the old atlas coverage.
    let mut changed = LATIN.to_vec();
    let count = u16::from_be_bytes(changed[4..6].try_into().unwrap()) as usize;
    let row = changed[12..12 + count * 16]
        .as_chunks::<16>()
        .0
        .iter()
        .find(|r| &r[..4] == b"head")
        .unwrap();
    let head = u32::from_be_bytes(row[8..12].try_into().unwrap()) as usize;
    let units = u16::from_be_bytes(changed[head + 18..head + 20].try_into().unwrap());
    changed[head + 18..head + 20].copy_from_slice(&(units * 2).to_be_bytes());
    let mut same_glyph = TextEngine::new(vec![FontSource {
        id: Ulid::from(1),
        bytes: changed,
    }])
    .unwrap();
    let scaled = same_glyph.shape("A", style(24.)).unwrap();
    let scaled_handle = same_glyph.cache_glyph(&scaled, 0, &mut a).unwrap();
    let scaled_entry = *a.get(&scaled_handle).unwrap();
    assert_eq!(scaled_entry.key.font, entry.key.font);
    assert_eq!(scaled_entry.key.glyph, entry.key.glyph);
    assert_eq!(scaled_entry.key.font_size_bits, entry.key.font_size_bits);
    assert_ne!(scaled_entry.key.content_sha256, entry.key.content_sha256);
    assert!(scaled_entry.rect.unwrap()[3] < entry.rect.unwrap()[3]);
    assert_eq!(*a.get(&h).unwrap(), entry);
    assert_eq!(a.len(), 3);
}
#[test]
fn atlas_configuration_limits_reject_before_pixels() {
    for config in [
        AtlasConfig {
            width: u32::MAX,
            height: u32::MAX,
            max_glyphs: 1,
        },
        AtlasConfig {
            width: 0,
            height: 1,
            max_glyphs: 1,
        },
        AtlasConfig {
            width: 1,
            height: 1,
            max_glyphs: 8193,
        },
        AtlasConfig {
            width: 2049,
            height: 1,
            max_glyphs: 1,
        },
    ] {
        assert!(GlyphAtlas::new(config).is_err());
    }
}
