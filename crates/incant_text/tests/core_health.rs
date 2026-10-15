//! Exercise the sample's real message table and packaged font chain together.
//! Glyph positions are CPU evidence; this does not replace native-window review.
use incant_localization::{Catalog, LocaleSettings, LocalizeRequest, MessageArgument, StringTable};
use incant_text::{FontSource, ShapedText, TextEngine, TextStyle, Wrap};
use std::collections::BTreeMap;

fn engine() -> TextEngine {
    TextEngine::new(
        [
            include_bytes!("../../../third_party/cosmic-text/fonts/Inter-Regular.ttf").as_slice(),
            include_bytes!("../../../third_party/cosmic-text/fonts/NotoSansArabic.ttf").as_slice(),
            include_bytes!("../../../samples/core/art/fonts/CoreSampleSC-Regular.ttf").as_slice(),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, bytes)| FontSource {
            id: (i as u128 + 1).into(),
            bytes: bytes.to_vec(),
        })
        .collect(),
    )
    .unwrap()
}

fn visual_pips(shaped: &ShapedText) -> Vec<char> {
    assert!(shaped.missing_clusters().is_empty());
    assert_eq!(shaped.lines().len(), 1);
    let mut pips: Vec<_> = shaped
        .glyphs()
        .iter()
        .filter_map(|glyph| {
            shaped.source()[glyph.source.clone()]
                .chars()
                .find(|ch| matches!(ch, '●' | '○'))
                .map(|ch| {
                    assert!(glyph.id != 0 && glyph.advance > 0.0);
                    (glyph.position[0], ch)
                })
        })
        .collect();
    assert_eq!(pips.len(), 3);
    pips.sort_by(|a, b| a.0.total_cmp(&b.0));
    assert!(pips.windows(2).all(|pair| pair[0].0 < pair[1].0));
    pips.into_iter().map(|(_, ch)| ch).collect()
}

#[test]
fn localized_core_health_depletes_from_the_end_of_each_reading_direction() {
    let table: StringTable =
        serde_json::from_str(include_str!("../../../samples/core/ui/strings.json")).unwrap();
    let id = table.id.clone();
    let catalog = Catalog::compile(&BTreeMap::from([(id.clone(), table)])).unwrap();
    let mut engine = engine();
    // Reuse the same text engine across damage, reset and locale switches.
    for locale in ["en", "ar", "zh-Hans", "ar", "en"] {
        for health in [3, 2, 1, 0, 3] {
            let output = catalog
                .localize(
                    &LocaleSettings {
                        locale: locale.into(),
                        ..Default::default()
                    },
                    &LocalizeRequest {
                        table_id: id.clone(),
                        key: "hud.health".into(),
                        arguments: BTreeMap::from([(
                            "health".into(),
                            MessageArgument::Number(f64::from(health)),
                        )]),
                    },
                )
                .unwrap();
            assert!(output.missing.is_none());
            assert_eq!(output.resolved_locale.as_deref(), Some(locale));
            let shaped = engine
                .shape(
                    &output.text,
                    TextStyle {
                        font_size: 22.0,
                        line_height: 28.0,
                        width: 160.0,
                        wrap: Wrap::Word,
                    },
                )
                .unwrap();
            let mut expected: Vec<_> = (0..3)
                .map(|index| if index < health { '●' } else { '○' })
                .collect();
            if locale == "ar" {
                expected.reverse();
            }
            assert_eq!(visual_pips(&shaped), expected, "{locale}, health={health}");
            assert_eq!(shaped.lines()[0].rtl, locale == "ar");
        }
    }
}
