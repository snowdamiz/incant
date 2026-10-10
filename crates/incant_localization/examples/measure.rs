//! Reproducible catalog/format timing, not a renderer or reference-device gate.
use incant_localization::*;
use std::{collections::BTreeMap, hint::black_box, time::Instant};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let id = "00000000000000000000000001";
    let pattern = "{name}: {n, plural, one {# coin} other {# coins}}, {date, date, long}";
    let table = StringTable {
        id: id.into(),
        name: "Measured catalog".into(),
        source_locale: "en".into(),
        messages: (0..4096)
            .map(|i| {
                (
                    format!("message.{i}"),
                    BTreeMap::from([("en".into(), pattern.into()), ("de".into(), pattern.into())]),
                )
            })
            .collect(),
    };
    let input = BTreeMap::from([(id.into(), table)]);
    let start = Instant::now();
    let catalog = Catalog::compile(&input)?;
    let compile_ms = start.elapsed().as_secs_f64() * 1000.;
    let settings = LocaleSettings {
        locale: "de".into(),
        ..Default::default()
    };
    let request = LocalizeRequest {
        table_id: id.into(),
        key: "message.0".into(),
        arguments: BTreeMap::from([
            ("name".into(), MessageArgument::Text("Ada".into())),
            ("n".into(), MessageArgument::Number(1234.)),
            (
                "date".into(),
                MessageArgument::Date(CalendarDate {
                    year: 2024,
                    month: 5,
                    day: 8,
                }),
            ),
        ]),
    };
    let start = Instant::now();
    let iterations = 10_000;
    for _ in 0..iterations {
        black_box(catalog.localize(&settings, &request)?);
    }
    let format_ms = start.elapsed().as_secs_f64() * 1000.;
    println!(
        "{}",
        serde_json::json!({"keys":4096,"locales":2,"catalog_json_bytes":serde_json::to_vec(&input)?.len(),"compile_ms":compile_ms,
        "iterations":iterations,"format_total_ms":format_ms,"format_mean_us":format_ms*1000./f64::from(iterations),"sample":catalog.localize(&settings,&request)?.text,
        "reference_device_gate":false})
    );
    Ok(())
}
