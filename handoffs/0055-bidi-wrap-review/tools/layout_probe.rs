//! Handoff 0055 CPU layout probe (public incant_text API only).
//!
//! `tools/probe.sh` copies this file to
//! `crates/incant_text/tests/zz_0055_layout_probe.rs`, runs only this target and
//! deletes the copy; it is never committed under `crates/`. It shapes every
//! fixture text in `cases.json` with the packaged 0054 chain (Noto Sans, Noto
//! Sans Arabic, CJK fixture; no Hebrew face), rasterizes each glyph into one
//! atlas exactly like the native binding, and writes:
//! - `layout.json`: lines, glyphs (cluster, font, id, position, advance, bidi
//!   level, atlas rect/origin) and the regression-style checks per text;
//! - `atlas.pgm`: the referenced atlas prefix, for the independent pixel
//!   reference in `pixels.py`.
use incant_text::{
    AtlasConfig, FontSource, GlyphAtlas, MAX_ATLAS_GLYPHS, ShapedText, TextEngine, TextStyle, Wrap,
};
use serde_json::{Value, json};
use std::{fs, path::PathBuf};

const FONTS: [(&str, &[u8]); 3] = [
    (
        "NotoSans-Regular.ttf",
        include_bytes!("fonts/NotoSans-Regular.ttf"),
    ),
    (
        "NotoSansArabic.ttf",
        include_bytes!("fonts/NotoSansArabic.ttf"),
    ),
    (
        "IncantCJKFixture.ttf",
        include_bytes!("fonts/IncantCJKFixture.ttf"),
    ),
];

fn number(case: &Value, key: &str) -> f32 {
    case[key].as_f64().unwrap() as f32
}

/// Non-whitespace (cluster, font, glyph, level), as in tests/bidi_wrap.rs.
fn content(shaped: &ShapedText) -> Vec<(usize, usize, ulid::Ulid, u16, u8)> {
    let mut result: Vec<_> = shaped
        .glyphs()
        .iter()
        .filter(|g| {
            !shaped.source()[g.source.clone()]
                .chars()
                .all(char::is_whitespace)
        })
        .map(|g| (g.source.start, g.source.end, g.font, g.id, g.bidi_level))
        .collect();
    result.sort();
    result
}

#[test]
fn dump_0055_layouts() {
    let cases: Value =
        serde_json::from_slice(&fs::read(std::env::var("INCANT0055_CASES").unwrap()).unwrap())
            .unwrap();
    let out = PathBuf::from(std::env::var("INCANT0055_OUT").unwrap());
    let chain: Vec<&str> = cases["fonts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(chain, FONTS.map(|(name, _)| name), "fixture chain order");
    let mut engine = TextEngine::new(
        FONTS
            .iter()
            .enumerate()
            .map(|(i, (_, bytes))| FontSource {
                id: ulid::Ulid::from(i as u128 + 1),
                bytes: bytes.to_vec(),
            })
            .collect(),
    )
    .unwrap();
    // Same configuration as incant_render_native::TextSettings::new.
    let mut atlas = GlyphAtlas::new(AtlasConfig {
        width: 2048,
        height: 2048,
        max_glyphs: MAX_ATLAS_GLYPHS,
    })
    .unwrap();
    let mut extent = [0u32; 2];
    let mut texts = Vec::new();
    for case in cases["texts"].as_array().unwrap() {
        let text = case["content"].as_str().unwrap();
        let wrap = match case["wrap"].as_str().unwrap() {
            "none" => Wrap::None,
            "word" => Wrap::Word,
            "word_or_glyph" => Wrap::WordOrGlyph,
            other => panic!("unknown wrap {other}"),
        };
        let style = TextStyle {
            font_size: number(case, "font_size"),
            line_height: number(case, "line_height"),
            width: number(case, "width"),
            wrap,
        };
        let shaped = engine.shape(text, style).unwrap();
        let unwrapped = engine
            .shape(
                text,
                TextStyle {
                    wrap: Wrap::None,
                    ..style
                },
            )
            .unwrap();
        let mut glyphs = Vec::new();
        for (i, g) in shaped.glyphs().iter().enumerate() {
            let handle = engine.cache_glyph(&shaped, i, &mut atlas).unwrap();
            let entry = *atlas.get(&handle).unwrap();
            if let Some([x, y, w, h]) = entry.rect {
                extent = [extent[0].max(x + w + 1), extent[1].max(y + h + 1)];
            }
            glyphs.push(json!({
                "source": [g.source.start, g.source.end],
                "text": &text[g.source.clone()],
                "font": g.font.0 - 1,
                "id": g.id,
                "x": g.position[0],
                "y": g.position[1],
                "advance": g.advance,
                "font_size": g.font_size,
                "level": g.bidi_level,
                "rect": entry.rect,
                "origin": entry.origin,
                "missing": entry.missing,
            }));
        }
        let widest_glyph = unwrapped
            .glyphs()
            .iter()
            .map(|g| g.advance)
            .fold(0_f32, f32::max);
        let mut lines = Vec::new();
        let mut interior_missing = Vec::new();
        for (index, line) in shaped.lines().iter().enumerate() {
            let range = &shaped.glyphs()[line.glyphs.clone()];
            let first = range.iter().map(|g| g.source.start).min().unwrap_or(0);
            let last = range.iter().map(|g| g.source.end).max().unwrap_or(0);
            for (i, ch) in text.char_indices() {
                if ch == ' '
                    && first < i
                    && i < last
                    && !range.iter().any(|g| g.source.contains(&i))
                {
                    interior_missing.push(json!([index, i]));
                }
            }
            let spaces = range
                .iter()
                .filter(|g| text[g.source.clone()].chars().all(char::is_whitespace))
                .count();
            let pen = range
                .iter()
                .fold([f32::INFINITY, f32::NEG_INFINITY], |a, g| {
                    [a[0].min(g.position[0]), a[1].max(g.position[0] + g.advance)]
                });
            lines.push(json!({
                "glyphs": [line.glyphs.start, line.glyphs.end],
                "top": line.top,
                "baseline": line.baseline,
                "height": line.height,
                "width": line.width,
                "rtl": line.rtl,
                "logical": &text[first..last],
                "space_glyphs": spaces,
                "pen": if range.is_empty() { json!(null) } else { json!(pen) },
                "over_width": line.width > style.width + 0.01,
            }));
        }
        let lost: Vec<_> = text
            .char_indices()
            .filter(|(i, c)| {
                !c.is_whitespace() && !shaped.glyphs().iter().any(|g| g.source.contains(i))
            })
            .map(|(i, _)| i)
            .collect();
        let checks = json!({
            "clusters_equal_unwrapped": content(&shaped) == content(&unwrapped),
            "lost_non_whitespace": lost,
            "interior_spaces_missing": interior_missing,
            "source_spaces": text.chars().filter(|c| *c == ' ').count(),
            "space_glyphs": shaped.glyphs().iter()
                .filter(|g| text[g.source.clone()].chars().all(char::is_whitespace)).count(),
            "widest_unwrapped_glyph": widest_glyph,
            "missing_clusters": shaped.missing_clusters().iter()
                .map(|r| json!([r.start, r.end])).collect::<Vec<_>>(),
        });
        println!(
            "{} ({:?} {} px, line {}, width {}): {} lines, size {:?}",
            case["key"].as_str().unwrap(),
            wrap,
            style.font_size,
            style.line_height,
            style.width,
            shaped.lines().len(),
            shaped.size()
        );
        for line in &lines {
            println!(
                "    w {:>8} rtl {:<5} spaces {} over {:<5} | {}",
                line["width"],
                line["rtl"],
                line["space_glyphs"],
                line["over_width"],
                line["logical"]
            );
        }
        println!("    checks {checks}");
        texts.push(json!({
            "key": case["key"],
            "content": text,
            "style": {"font_size": style.font_size, "line_height": style.line_height,
                      "width": style.width, "wrap": case["wrap"]},
            "size": shaped.size(),
            "lines": lines,
            "glyphs": glyphs,
            "checks": checks,
        }));
    }
    fs::create_dir_all(&out).unwrap();
    let [w, h] = extent;
    let stride = atlas.size()[0] as usize;
    let mut pgm = format!("P5\n{w} {h}\n255\n").into_bytes();
    for row in 0..h as usize {
        pgm.extend_from_slice(&atlas.pixels()[row * stride..row * stride + w as usize]);
    }
    fs::write(out.join("atlas.pgm"), pgm).unwrap();
    let record = json!({
        "format": "incant-0055-layout-probe",
        "fonts": chain,
        "atlas": {"config": [2048, 2048], "extent": extent, "file": "atlas.pgm"},
        "texts": texts,
    });
    fs::write(
        out.join("layout.json"),
        serde_json::to_string_pretty(&record).unwrap() + "\n",
    )
    .unwrap();
}
